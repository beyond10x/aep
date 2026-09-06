//! Closed transcription of ESS ba43fda scenario.rs and its referenced value vocabulary.
//!
//! This checks the complete legacy suite, including unused metadata/dependencies. It does not
//! execute steps, infer inventory, or reinterpret a major according to the fields it happens to use.
use aep_domain::ess_conformance_v2::{lower_kebab, qualified_name, ScenarioId};
use aep_domain::evidence::SpecDigest;
use aep_domain::{FactPath, Node, Predicate};

use crate::count_json::{Json, Result};

pub(crate) struct AdmittedSuite {
    pub version: String,
    pub spec_digest: SpecDigest,
    pub ids: Vec<ScenarioId>,
}

pub(crate) fn admit(value: &Json) -> Result<AdmittedSuite> {
    let provenance = value
        .object()?
        .get("provenance")
        .ok_or_else(|| value.error("MissingField", "provenance"))?;
    let marker = provenance
        .object()?
        .get("suite_version")
        .ok_or_else(|| provenance.error("MissingField", "suite_version"))?;
    let version = marker.text()?;
    if !matches!(
        version,
        "ess-conformance/1" | "ess-conformance/2" | "ess-conformance/3" | "ess-conformance/4"
    ) {
        return Err(marker.error("UnsupportedSuiteVersion", version));
    }
    let root = value.closed(&["provenance", "scenarios"], &[])?;
    let p = root["provenance"].closed(
        &[
            "suite_version",
            "system",
            "specification_version",
            "spec_digest",
            "contract_digest",
        ],
        &["component"],
    )?;
    p["system"].text()?;
    p["specification_version"].text()?;
    if let Some(component) = p.get("component") {
        if !component.null() {
            component.text()?;
        }
    }
    let digest = |value: &Json| {
        SpecDigest::new(value.text()?)
            .map_err(|error| value.error("MalformedModelDigest", error.to_string()))
    };
    let spec_digest = digest(&p["spec_digest"])?;
    digest(&p["contract_digest"])?;
    let mut ids = Vec::new();
    for (id, scenario) in root["scenarios"].object()? {
        ids.push(
            ScenarioId::new(id.clone())
                .map_err(|error| scenario.error("MalformedScenarioId", error.to_string()))?,
        );
        let s = scenario.closed(&["purpose", "steps", "source"], &[])?;
        let purpose = s["purpose"].text()?.trim();
        if purpose.is_empty()
            || purpose.chars().any(char::is_control)
            || purpose.chars().count() > 200
        {
            return Err(s["purpose"].error(
                "InvalidPurpose",
                "purpose must be one nonempty line of at most 200 characters",
            ));
        }
        for step in s["steps"].array()? {
            step_value(step)?;
        }
        for reference in s["source"].array()? {
            semantic_reference(reference)?;
        }
    }
    Ok(AdmittedSuite {
        version: version.into(),
        spec_digest,
        ids,
    })
}

fn name(value: &Json, kebab: bool) -> Result<()> {
    let text = value.text()?;
    if if kebab {
        lower_kebab(text)
    } else {
        qualified_name(text)
    } {
        Ok(())
    } else {
        Err(value.error("MalformedName", text))
    }
}
fn local_name(value: &Json) -> Result<()> {
    name(value, false)?;
    if value.text()?.contains('.') {
        return Err(value.error("MalformedName", "expected one name segment"));
    }
    Ok(())
}
fn outcome(value: &Json) -> Result<()> {
    let fields = value.closed(&["command", "outcome"], &[])?;
    name(&fields["command"], false)?;
    name(&fields["outcome"], true)
}
fn semantic_reference(value: &Json) -> Result<()> {
    let fields = value.closed(&["kind", "name"], &[])?;
    let target = &fields["name"];
    match fields["kind"].text()? {
        "domain" | "type" | "entity" | "command" | "event" | "error" | "view" | "actor" => {
            name(target, false)
        }
        "binding" | "component" => name(target, true),
        "outcome" => outcome(target),
        "transition" => {
            let transition = target.closed(&["entity", "transition"], &[])?;
            name(&transition["entity"], false)?;
            local_name(&transition["transition"])
        }
        other => Err(fields["kind"].error("UnsupportedSemanticReference", other)),
    }
}
fn scenario_value(value: &Json) -> Result<()> {
    let fields = value.object()?;
    let kind = fields
        .get("kind")
        .ok_or_else(|| value.error("MissingField", "kind"))?;
    match kind.text()? {
        "literal" => {
            let v = value.closed(&["kind", "value"], &[])?;
            v["value"].payload()
        }
        "instance" => {
            let v = value.closed(&["kind", "instance"], &[])?;
            name(&v["instance"], true)
        }
        "observed" => {
            let v = value.closed(&["kind", "event", "field"], &[])?;
            name(&v["event"], false)?;
            v["field"].text()?;
            Ok(())
        }
        other => Err(kind.error("UnsupportedScenarioValue", other)),
    }
}
fn values(value: &Json) -> Result<()> {
    for v in value.object()?.values() {
        scenario_value(v)?;
    }
    Ok(())
}
fn payload_map(value: &Json) -> Result<()> {
    value.object()?;
    value.payload()
}
fn shape(value: &Json) -> Result<()> {
    for leaf in value.object()?.values() {
        let fields = leaf.object()?;
        let holds = fields
            .get("holds")
            .ok_or_else(|| leaf.error("MissingField", "holds"))?;
        let required: &[&str] = match holds.text()? {
            "primitive" => &["holds", "kind"],
            "enum" => &["holds", "variants"],
            "list" | "map" | "union" => &["holds"],
            other => return Err(holds.error("UnsupportedHolds", other)),
        };
        let fields = leaf.closed(required, &["optional"])?;
        if let Some(optional) = fields.get("optional") {
            optional.boolean()?;
        }
        if let Some(kind) = fields.get("kind") {
            if !matches!(
                kind.text()?,
                "string"
                    | "boolean"
                    | "integer"
                    | "decimal"
                    | "timestamp"
                    | "duration"
                    | "uuid"
                    | "bytes"
            ) {
                return Err(kind.error("UnsupportedPrimitive", kind.text()?));
            }
        }
        if let Some(variants) = fields.get("variants") {
            for variant in variants.array()? {
                variant.text()?;
            }
        }
    }
    Ok(())
}
fn rankings(value: &Json) -> Result<()> {
    for rank in value.array()? {
        let words: Vec<_> = rank.text()?.split_whitespace().collect();
        if !matches!(
            words.as_slice(),
            [_] | [_, "asc" | "ascending" | "desc" | "descending"]
        ) {
            return Err(rank.error(
                "InvalidRanking",
                "expected <field> [asc|ascending|desc|descending]",
            ));
        }
    }
    Ok(())
}
fn position(value: &Json) -> Result<()> {
    let fields = value.object()?;
    let row = fields
        .get("row")
        .ok_or_else(|| value.error("MissingField", "row"))?;
    match row.text()? {
        "first" | "last" => {
            value.closed(&["row"], &[])?;
        }
        "nth" => {
            let fields = value.closed(&["row", "index"], &[])?;
            fields["index"].unsigned()?;
        }
        other => return Err(row.error("UnsupportedPosition", other)),
    }
    Ok(())
}

// ESS's scalar constraints/expressions share AEP's original parser. Walk the frozen structural
// grammar explicitly: quantified mappings are the ESS extension, and depth belongs to the
// original tree. Rewrapping nodes would consume extra depth and refuse valid legacy documents.
fn predicate(value: &Json) -> Result<()> {
    fn frozen(node: &Node, value: &Json, depth: u16) -> Result<()> {
        if depth > 32 {
            return Err(value.error("InvalidPredicate", "predicate nesting exceeds 32"));
        }
        match node {
            Node::Map(fields) => {
                for (key, child) in fields {
                    match key.as_str() {
                        "forall" | "exists" => {
                            let Node::Map(q) = child else {
                                return Err(value
                                    .error("InvalidPredicate", "quantifier requires in/as/that"));
                            };
                            if q.len() != 3
                                || !["in", "as", "that"].iter().all(|key| q.contains_key(*key))
                            {
                                return Err(value.error(
                                    "InvalidPredicate",
                                    "quantifier requires exactly in/as/that",
                                ));
                            }
                            let collection = q["in"].as_text().ok_or_else(|| {
                                value.error("InvalidPredicate", "in must be a fact path")
                            })?;
                            FactPath::new(collection).map_err(|error| {
                                value.error("InvalidPredicate", error.to_string())
                            })?;
                            let bind = q["as"].as_text().ok_or_else(|| {
                                value.error("InvalidPredicate", "as must be one fact-path segment")
                            })?;
                            let path = FactPath::new(bind).map_err(|error| {
                                value.error("InvalidPredicate", error.to_string())
                            })?;
                            if path.segments().len() != 1 {
                                return Err(
                                    value.error("InvalidPredicate", "as must be one segment")
                                );
                            }
                            frozen(&q["that"], value, depth + 1)?;
                        }
                        "all" | "and" | "all_of" | "any" | "or" | "none" | "none_of_these" => {
                            for nested in child.as_seq_or_single() {
                                frozen(nested, value, depth + 1)?;
                            }
                        }
                        "not" => frozen(child, value, depth + 1)?,
                        _ => {
                            Predicate::from_node(&Node::Map(
                                [(key.clone(), child.clone())].into_iter().collect(),
                            ))
                            .map_err(|error| value.error("InvalidPredicate", error.to_string()))?;
                        }
                    }
                }
            }
            Node::Seq(items) => {
                for child in items {
                    frozen(child, value, depth + 1)?;
                }
            }
            Node::Text(expression) => {
                let mut rest = expression.trim();
                let mut current_depth = depth;
                while let Some(nested) = rest.strip_prefix("not ") {
                    current_depth += 1;
                    rest = nested.trim();
                }
                if current_depth > 32 {
                    return Err(value.error("InvalidPredicate", "predicate nesting exceeds 32"));
                }
                Predicate::from_node(node)
                    .map_err(|error| value.error("InvalidPredicate", error.to_string()))?;
            }
            other => {
                Predicate::from_node(other)
                    .map_err(|error| value.error("InvalidPredicate", error.to_string()))?;
            }
        }
        Ok(())
    }
    value.payload()?;
    let node: Node = serde_json::from_str(&value.raw)
        .map_err(|error| value.error("InvalidPredicate", error.to_string()))?;
    frozen(&node, value, 0)
}
fn expectation(value: &Json) -> Result<()> {
    let fields = value.object()?;
    let tag = fields
        .get("expect")
        .ok_or_else(|| value.error("MissingField", "expect"))?;
    match tag.text()? {
        "contains" | "excludes" => {
            let v = value.closed(&["expect", "fields"], &[])?;
            values(&v["fields"])?;
        }
        "satisfies" => {
            let v = value.closed(&["expect", "predicate"], &[])?;
            predicate(&v["predicate"])?;
        }
        "counts" => {
            let v = value.closed(&["expect"], &["at_least", "at_most"])?;
            for key in ["at_least", "at_most"] {
                if let Some(count) = v.get(key) {
                    if !count.null() {
                        count.unsigned()?;
                    }
                }
            }
        }
        "ranked" => {
            let v = value.closed(&["expect", "order_by"], &[])?;
            rankings(&v["order_by"])?;
        }
        "at" => {
            let v = value.closed(&["expect", "order_by", "position"], &["fields"])?;
            rankings(&v["order_by"])?;
            position(&v["position"])?;
            if let Some(fields) = v.get("fields") {
                values(fields)?;
            }
        }
        other => return Err(tag.error("UnsupportedViewExpectation", other)),
    }
    Ok(())
}
fn step_value(value: &Json) -> Result<()> {
    let object = value.object()?;
    let tag = object
        .get("step")
        .ok_or_else(|| value.error("MissingField", "step"))?;
    let (required, optional): (&[&str], &[&str]) = match tag.text()? {
        "configure_external_outcome" => (&["step", "force"], &[]),
        "execute_command" => (&["step", "command"], &["actor", "input"]),
        "expect_outcome" => (&["step", "outcome"], &[]),
        "expect_error" => (&["step", "error"], &["fields"]),
        "expect_event" | "eventually_event" => (&["step", "event"], &["payload", "shape"]),
        "expect_no_event" | "redeliver_event" => (&["step", "event"], &[]),
        "capture_instance" => (&["step", "instance", "entity", "event", "field"], &[]),
        "expect_invocation" => (&["step", "binding", "command"], &["input"]),
        "query_view" => (&["step", "view"], &["params"]),
        "expect_view" => (&["step", "view", "expectation"], &[]),
        "eventually_view" => (&["step", "view", "expectation"], &["params"]),
        "mark_instant" => (&["step", "instant"], &[]),
        "expect_not_before" | "expect_within" => (&["step", "instant", "elapsed"], &[]),
        "expect_quiet" => (&["step", "event", "instant", "elapsed"], &[]),
        "expect_halt" | "eventually_halt" => (&["step", "view", "after"], &["params"]),
        other => return Err(tag.error("UnsupportedStep", other)),
    };
    let fields = value.closed(required, optional)?;
    for (key, field) in fields {
        match key.as_str() {
            "step" => {}
            "force" | "outcome" => outcome(field)?,
            "command" | "entity" | "event" | "view" | "error" => name(field, false)?,
            "actor" => {
                if !field.null() {
                    name(field, false)?;
                }
            }
            "instance" | "instant" | "binding" => name(field, true)?,
            "field" => {
                field.text()?;
            }
            "input" | "params" => values(field)?,
            "fields" | "payload" => payload_map(field)?,
            "shape" => shape(field)?,
            "expectation" => expectation(field)?,
            "elapsed" => {
                if field.unsigned()? > u64::from(u32::MAX) {
                    return Err(field.error("InvalidElapsed", "seconds exceed u32"));
                }
            }
            "after" => {
                field.unsigned()?;
            }
            _ => return Err(field.error("UnsupportedStepField", key)),
        }
    }
    Ok(())
}
