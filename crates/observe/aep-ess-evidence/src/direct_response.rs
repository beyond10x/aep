//! Closed ER-required subset of ESS `direct_response::Observation` (ordinary /28, inventory /29).
//! This validates declaration authority, never executes or reinterprets the claimed return.
use crate::count_json::{Json, Result};
use aep_domain::ess_conformance_v2::{lower_kebab, qualified_name};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
enum LiteralType {
    String,
    Boolean,
    Integer,
    OptionalString,
    OptionalStrings,
}

pub(crate) fn admit(value: &Json, preceding: Option<&str>) -> Result<()> {
    let response = value.closed(
        &["command", "outcome", "fields", "declarations", "expected"],
        &[],
    )?;
    let command = response["command"].text()?;
    if !qualified_name(command) || preceding != Some(command) {
        return Err(response["command"].error(
            "ResponseCommandMismatch",
            "direct response must name the preceding invocation",
        ));
    }
    if !response["outcome"].null() {
        let outcome = response["outcome"].closed(&["command", "outcome"], &[])?;
        if outcome["command"].text()? != command {
            return Err(outcome["command"].error(
                "ResponseCommandMismatch",
                "response outcome belongs to another command",
            ));
        }
        if !lower_kebab(outcome["outcome"].text()?) {
            return Err(outcome["outcome"].error("MalformedName", "outcome name"));
        }
    }
    let fields = response["fields"].array()?;
    let declarations = response["declarations"].object()?;
    if fields.is_empty()
        || fields.len() > 256
        || declarations.len() > 4096
        || value.raw.len() > 1_048_576
    {
        return Err(value.error("ResponseResourceLimit", "bounded direct response authority"));
    }
    let mut aliases = BTreeMap::new();
    for (name, declaration) in declarations {
        if !qualified_name(name) || !name.contains('.') {
            return Err(declaration.error("MalformedName", "qualified nominal declaration"));
        }
        let fields = declaration.object()?;
        let kind = fields
            .get("kind")
            .ok_or_else(|| declaration.error("MissingField", "kind"))?;
        if kind.text()? != "newtype" {
            return Err(kind.error(
                "UnsupportedResponseDeclaration",
                "this reader admits transparent nominal declarations in the ER value profile only",
            ));
        }
        let fields = declaration.closed(&["kind", "of"], &[])?;
        aliases.insert(name.as_str(), &fields["of"]);
    }
    let mut used = BTreeSet::new();
    let mut admitted = BTreeMap::new();
    for field in fields {
        let fields = field.closed(&["name", "type"], &[])?;
        let name = fields["name"].text()?;
        if !field_name(name) {
            return Err(fields["name"].error("MalformedName", "response field name"));
        }
        let kind = literal_type(&fields["type"], &aliases, &mut used, &mut BTreeSet::new())?;
        if admitted.insert(name, kind).is_some() {
            return Err(field.error("DuplicateResponseField", name));
        }
    }
    if used.len() != aliases.len() {
        return Err(response["declarations"].error(
            "UnusedResponseDeclaration",
            "response authority must contain exactly its reachable declarations",
        ));
    }
    for (name, literal) in response["expected"].object()? {
        let kind = admitted.get(name.as_str()).ok_or_else(|| {
            literal.error("UnknownResponseField", "literal names an undeclared field")
        })?;
        admit_literal(*kind, literal)?;
    }
    Ok(())
}

fn admit_literal(kind: LiteralType, literal: &Json) -> Result<()> {
    let valid = match kind {
        LiteralType::String => literal.text().is_ok(),
        LiteralType::Boolean => literal.boolean().is_ok(),
        LiteralType::Integer => integer(literal),
        LiteralType::OptionalString => literal.null() || literal.text().is_ok(),
        LiteralType::OptionalStrings => {
            if literal.null() {
                true
            } else if let Ok(values) = literal.array() {
                if values.len() > 65_536 {
                    return Err(
                        literal.error("ResponseResourceLimit", "text list exceeds 65536 entries")
                    );
                }
                values.iter().all(|value| value.text().is_ok())
            } else {
                false
            }
        }
    };
    if valid {
        Ok(())
    } else {
        Err(literal.error(
            "InvalidResponseValue",
            "literal must match its admitted type; integers must be exact integral i64/u64 values",
        ))
    }
}

fn field_name(name: &str) -> bool {
    let unprefixed = name.trim_start_matches('_');
    unprefixed
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphabetic)
        && unprefixed
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn integer(value: &Json) -> bool {
    integer_magnitude(&value.raw).is_some_and(|(negative, magnitude)| {
        magnitude
            <= if negative {
                1_u128 << 63
            } else {
                u128::from(u64::MAX)
            }
    })
}

// ESS may serialize an integral value as 1.0. Decimal shifting proves integrality and range
// without losing a digit, even when a neighboring fractional value rounds to an f64 integer.
fn integer_magnitude(raw: &str) -> Option<(bool, u128)> {
    let negative = raw.starts_with('-');
    let unsigned = raw.strip_prefix('-').unwrap_or(raw);
    let (mantissa, exponent) = unsigned.split_once(['e', 'E']).unwrap_or((unsigned, "0"));
    let exponent = exponent.parse::<i64>().ok()?;
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if whole.is_empty()
        || !whole
            .bytes()
            .chain(fraction.bytes())
            .all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let coefficient = format!("{whole}{fraction}");
    let digits = coefficient.trim_start_matches('0');
    if digits.is_empty() {
        return Some((negative, 0));
    }
    let scale = exponent.checked_sub(i64::try_from(fraction.len()).ok()?)?;
    let integer = if scale < 0 {
        let removed = usize::try_from(scale.unsigned_abs()).ok()?;
        let split = digits.len().checked_sub(removed)?;
        if !digits[split..].bytes().all(|byte| byte == b'0') {
            return None;
        }
        digits[..split].to_owned()
    } else {
        let added = usize::try_from(scale).ok()?;
        if digits.len().checked_add(added)? > 20 {
            return None;
        }
        format!("{digits}{}", "0".repeat(added))
    };
    Some((negative, integer.parse().ok()?))
}

fn literal_type<'a>(
    value: &'a Json,
    aliases: &BTreeMap<&'a str, &'a Json>,
    used: &mut BTreeSet<&'a str>,
    stack: &mut BTreeSet<&'a str>,
) -> Result<LiteralType> {
    let name = value.text()?;
    match name {
        "String" => return Ok(LiteralType::String),
        "Boolean" => return Ok(LiteralType::Boolean),
        "Integer" => return Ok(LiteralType::Integer),
        "Optional<String>" => return Ok(LiteralType::OptionalString),
        "Optional<List<String>>" => return Ok(LiteralType::OptionalStrings),
        _ => {}
    }
    if !qualified_name(name) || !name.contains('.') {
        return Err(value.error(
            "UnsupportedResponseType",
            "only String, Boolean, Integer, Optional<String>, Optional<List<String>> and transparent nominal aliases are admitted",
        ));
    }
    if stack.len() >= 128 {
        return Err(value.error("ResponseResourceLimit", "nominal type depth exceeds 128"));
    }
    if !stack.insert(name) {
        return Err(value.error("ResponseTypeCycle", "recursive nominal declaration"));
    }
    let underlying = aliases.get(name).ok_or_else(|| {
        value.error(
            "MissingResponseDeclaration",
            "response nominal type is undeclared",
        )
    })?;
    used.insert(name);
    let result = literal_type(underlying, aliases, used, stack);
    stack.remove(name);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_spellings_are_checked_exactly_without_binary64_rounding() {
        for literal in [
            "1.0",
            "1e3",
            "100e-2",
            "-0.0",
            "9007199254740993.0",
            "18446744073709551615.0",
            "-9223372036854775808.0",
        ] {
            admit_literal(LiteralType::Integer, &Json::parse(literal, "$").unwrap()).unwrap();
        }
        for literal in [
            "0.1",
            "1e-3",
            "9007199254740993.1",
            "18446744073709551616.0",
            "-9223372036854775809.0",
            "1e1000",
        ] {
            let error = admit_literal(LiteralType::Integer, &Json::parse(literal, "$").unwrap())
                .unwrap_err();
            assert_eq!(error.issues[0].reason, "InvalidResponseValue", "{literal}");
        }
    }
}
