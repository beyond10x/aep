//! Strict JSON structure with original scalar spellings and duplicate detection before maps.
use std::collections::BTreeMap;
use std::fmt;

use aep_domain::ess_conformance_v2::EssAdmissionError;
use serde::de::{MapAccess, Visitor};
use serde::Deserialize;
use serde_json::value::RawValue;

pub(crate) type Result<T> = std::result::Result<T, EssAdmissionError>;

#[derive(Debug)]
pub(crate) struct Json {
    pub raw: String,
    pub path: String,
    kind: Kind,
}
#[derive(Debug)]
enum Kind {
    Object(BTreeMap<String, Json>),
    Array(Vec<Json>),
    String(String),
    Scalar,
}

struct Fields(Vec<(String, Box<RawValue>)>);
impl<'de> Deserialize<'de> for Fields {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = Fields;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an object without duplicate keys")
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Fields, A::Error> {
                let mut fields = Vec::new();
                let mut keys = std::collections::BTreeSet::new();
                while let Some((key, value)) = map.next_entry::<String, Box<RawValue>>()? {
                    if !keys.insert(key.clone()) {
                        return Err(serde::de::Error::custom(format!("duplicate key {key:?}")));
                    }
                    fields.push((key, value));
                }
                Ok(Fields(fields))
            }
        }
        d.deserialize_map(ObjectVisitor)
    }
}

impl Json {
    pub fn parse(raw: &str, path: &str) -> Result<Self> {
        Self::parse_at(raw, path, 0)
    }
    fn parse_at(raw: &str, path: &str, depth: u16) -> Result<Self> {
        if depth > 128 {
            return Err(EssAdmissionError::new(
                "InvalidDocument",
                path,
                "JSON nesting exceeds 128",
            ));
        }
        let syntax = |error: serde_json::Error| {
            let reason = if error.to_string().contains("duplicate key") {
                "DuplicateKey"
            } else {
                "InvalidDocument"
            };
            EssAdmissionError::new(reason, path, error.to_string())
        };
        let checked: Box<RawValue> = serde_json::from_str(raw).map_err(syntax)?;
        let raw = checked.get();
        let kind = match raw.as_bytes()[0] {
            b'{' => {
                let fields: Fields = serde_json::from_str(raw).map_err(syntax)?;
                let mut object = BTreeMap::new();
                for (key, value) in fields.0 {
                    object.insert(
                        key.clone(),
                        Self::parse_at(value.get(), &format!("{path}.{key}"), depth + 1)?,
                    );
                }
                Kind::Object(object)
            }
            b'[' => {
                let values: Vec<Box<RawValue>> = serde_json::from_str(raw).map_err(syntax)?;
                Kind::Array(
                    values
                        .iter()
                        .enumerate()
                        .map(|(index, value)| {
                            Self::parse_at(value.get(), &format!("{path}[{index}]"), depth + 1)
                        })
                        .collect::<Result<_>>()?,
                )
            }
            b'"' => Kind::String(serde_json::from_str(raw).map_err(syntax)?),
            _ => Kind::Scalar,
        };
        Ok(Self {
            raw: raw.to_owned(),
            path: path.to_owned(),
            kind,
        })
    }
    pub fn error(&self, reason: &'static str, detail: impl Into<String>) -> EssAdmissionError {
        EssAdmissionError::new(reason, &self.path, detail)
    }
    pub fn object(&self) -> Result<&BTreeMap<String, Self>> {
        if let Kind::Object(object) = &self.kind {
            Ok(object)
        } else {
            Err(self.error("InvalidShape", "expected an object"))
        }
    }
    pub fn closed(&self, required: &[&str], optional: &[&str]) -> Result<&BTreeMap<String, Self>> {
        let object = self.object()?;
        let mut issues = Vec::new();
        for key in required {
            if !object.contains_key(*key) {
                issues.extend(
                    EssAdmissionError::new(
                        "MissingField",
                        format!("{}.{key}", self.path),
                        "required field absent",
                    )
                    .issues,
                );
            }
        }
        for key in object.keys() {
            if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
                issues.extend(
                    object[key]
                        .error("UnknownField", "closed count-stage vocabulary")
                        .issues,
                );
            }
        }
        if issues.is_empty() {
            Ok(object)
        } else {
            Err(EssAdmissionError { issues })
        }
    }
    pub fn array(&self) -> Result<&[Self]> {
        if let Kind::Array(array) = &self.kind {
            Ok(array)
        } else {
            Err(self.error("InvalidShape", "expected an array"))
        }
    }
    pub fn text(&self) -> Result<&str> {
        if let Kind::String(text) = &self.kind {
            Ok(text)
        } else {
            Err(self.error("InvalidShape", "expected a string"))
        }
    }
    pub fn boolean(&self) -> Result<bool> {
        match self.raw.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(self.error("InvalidShape", "expected a boolean")),
        }
    }
    pub fn unsigned(&self) -> Result<u64> {
        let raw = self.raw.as_str();
        if raw != "0"
            && !(raw
                .as_bytes()
                .first()
                .is_some_and(|c| (b'1'..=b'9').contains(c))
                && raw.bytes().all(|c| c.is_ascii_digit()))
        {
            return Err(self.error(
                "UnsignedIntegerRequired",
                format!("expected an unsigned decimal integer token, found {raw}"),
            ));
        }
        raw.parse()
            .map_err(|_| self.error("UnsignedIntegerOverflow", raw))
    }
    pub fn null(&self) -> bool {
        self.raw == "null"
    }
    // Compare admitted survivor definitions, retaining semantic sequence order and exact integers.
    // Float spellings occur only in validated payload positions; their domain is finite binary64.
    pub fn equivalent(&self, other: &Self) -> bool {
        match (&self.kind, &other.kind) {
            (Kind::Object(a), Kind::Object(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .all(|(key, value)| b.get(key).is_some_and(|other| value.equivalent(other)))
            }
            (Kind::Array(a), Kind::Array(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.equivalent(b))
            }
            (Kind::String(a), Kind::String(b)) => a == b,
            (Kind::Scalar, Kind::Scalar) => {
                if let (Ok(a), Ok(b)) = (self.raw.parse::<u64>(), other.raw.parse::<u64>()) {
                    return a == b;
                }
                if let (Ok(a), Ok(b)) = (self.raw.parse::<f64>(), other.raw.parse::<f64>()) {
                    return aep_domain::facts::Number::new(a).ok()
                        == aep_domain::facts::Number::new(b).ok();
                }
                self.raw == other.raw
            }
            _ => false,
        }
    }
    // Legacy payload numbers deliberately use ESS's finite-number domain, not report scalar rules.
    pub fn payload(&self) -> Result<()> {
        match &self.kind {
            Kind::Object(values) => {
                for value in values.values() {
                    value.payload()?;
                }
            }
            Kind::Array(values) => {
                for value in values {
                    value.payload()?;
                }
            }
            Kind::String(_) => {}
            Kind::Scalar if self.null() || self.raw == "true" || self.raw == "false" => {}
            Kind::Scalar => {
                if !self.raw.parse::<f64>().is_ok_and(f64::is_finite) {
                    return Err(self.error(
                        "InvalidPayloadNumber",
                        "legacy payload number must be finite",
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Json;

    #[test]
    fn unsigned_scalars_cover_full_u64_without_large_outcome_allocations() {
        for expected in [
            0,
            9_007_199_254_740_993,
            i64::MAX as u64,
            i64::MAX as u64 + 1,
            u64::MAX,
        ] {
            let scalar = Json::parse(&expected.to_string(), "$.counts.total").unwrap();
            assert_eq!(scalar.unsigned().unwrap(), expected);
        }
        for token in ["-0", "-1", "1.5", "1.0", "1e0", "\"1\""] {
            let error = Json::parse(token, "$.counts.total")
                .unwrap()
                .unsigned()
                .unwrap_err();
            assert_eq!(error.issues[0].reason, "UnsignedIntegerRequired");
            assert_eq!(error.issues[0].path, "$.counts.total");
        }
        let error = Json::parse("18446744073709551616", "$.counts.total")
            .unwrap()
            .unsigned()
            .unwrap_err();
        assert_eq!(error.issues[0].reason, "UnsignedIntegerOverflow");
    }
}
