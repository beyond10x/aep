//! Complete suite/5 inventory and offline original parent-chain admission.
use crate::count_json::{Json, Result};
use aep_domain::ess_conformance_coverage::{
    ordered_ids, original_digest, validate_needs, Filter, Inventory, Origin, Outside,
    OutsideReason, RefusalEffect, RefusalScope, Retained, Scope, SourceDisposition, SourceIdentity,
    SuiteReference,
};
use aep_domain::ess_conformance_v2::{EssAdmissionError, ScenarioId};
use aep_domain::SpecDigest;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub(crate) struct AdmittedSuite {
    pub document: Json,
    pub inventory: Inventory,
    pub reference: SuiteReference,
    pub spec_digest: SpecDigest,
    pub ids: Vec<ScenarioId>,
}

fn digest(original: &str) -> String {
    Sha256::digest(original.as_bytes())
        .iter()
        .fold("sha256:".to_owned(), |mut text, byte| {
            write!(text, "{byte:02x}").expect("writing to String");
            text
        })
}

pub(crate) fn admit_input(original: &str) -> Result<AdmittedSuite> {
    let input = Json::parse(original, "$input")?;
    let fields = input.closed(&["format", "suite_json", "parent_suites"], &[])?;
    if fields["format"].text()? != "ess-conformance-input/1" {
        return Err(fields["format"].error(
            "UnsupportedInputVersion",
            "expected ess-conformance-input/1",
        ));
    }
    let parents = fields["parent_suites"].array()?;
    let mut seen = BTreeSet::new();
    let mut ancestor = None;
    // This flat chain has no semantic depth cap; JSON nesting is checked independently.
    for document in parents
        .iter()
        .rev()
        .chain(std::iter::once(&fields["suite_json"]))
    {
        let suite = admit_suite(document.text()?)?;
        if !seen.insert(suite.reference.digest().to_owned()) {
            return Err(document.error(
                "RepeatedParent",
                "original suite repeats in the input chain",
            ));
        }
        match (&suite.inventory.selection.filter, &ancestor) {
            (Filter::All, None) => {}
            (Filter::All, Some(_)) => {
                return Err(document.error("SurplusParent", "all selection has no parent"))
            }
            (Filter::Explicit { .. }, None) => {
                return Err(document.error(
                    "MissingParent",
                    "explicit selection requires its complete original lineage",
                ))
            }
            (Filter::Explicit { .. }, Some(parent)) => admit_child(&suite, parent)?,
        }
        ancestor = Some(suite);
    }
    Ok(ancestor.expect("selected suite is always processed"))
}

pub(crate) fn wrap_suite(original: &str) -> Result<String> {
    let suite = admit_suite(original)?;
    if !matches!(suite.inventory.selection.filter, Filter::All) {
        return Err(EssAdmissionError::new(
            "MissingParent",
            "$suite.coverage.selection.filter",
            "raw explicit suite requires --suite-input lineage",
        ));
    }
    Ok(serde_json::json!({"format":"ess-conformance-input/1", "suite_json":original, "parent_suites":[]}).to_string())
}

fn admit_suite(original: &str) -> Result<AdmittedSuite> {
    let reference = SuiteReference::new(
        "ess-conformance/5".into(),
        "sha256-json-bytes/1".into(),
        digest(original),
    )?;
    let document = Json::parse(original, "$suite")?;
    let fields = document.closed(&["provenance", "scenarios", "coverage"], &[])?;
    let provenance = fields["provenance"].closed(
        &[
            "suite_version",
            "system",
            "specification_version",
            "spec_digest",
            "contract_digest",
        ],
        &["component"],
    )?;
    if provenance["suite_version"].text()? != "ess-conformance/5" {
        return Err(provenance["suite_version"]
            .error("UnsupportedSuiteVersion", "coverage requires suite/5"));
    }
    provenance["system"].text()?;
    provenance["specification_version"].text()?;
    let parse_digest = |value: &Json| {
        SpecDigest::new(value.text()?)
            .map_err(|error| value.error("MalformedModelDigest", error.to_string()))
    };
    let spec_digest = parse_digest(&provenance["spec_digest"])?;
    parse_digest(&provenance["contract_digest"])?;
    let ids = crate::count_suite::admit_scenarios(&fields["scenarios"])?;
    let inventory = crate::coverage_wire::inventory(&fields["coverage"])?;
    let component = provenance
        .get("component")
        .filter(|value| !value.null())
        .map(Json::text)
        .transpose()?;
    let expected_component = match &inventory.selection.scope {
        Scope::System => None,
        Scope::Component { component } => Some(component.as_str()),
    };
    if component != expected_component {
        return Err(fields["coverage"].error(
            "ComponentSelectionMismatch",
            "selection scope must agree with provenance.component",
        ));
    }
    validate_inventory(&inventory, &ids)?;
    Ok(AdmittedSuite {
        document,
        inventory,
        reference,
        spec_digest,
        ids,
    })
}

fn invalid(reason: &'static str, detail: impl Into<String>) -> EssAdmissionError {
    EssAdmissionError::new(reason, "$suite.coverage", detail)
}

fn validate_inventory(inventory: &Inventory, ids: &[ScenarioId]) -> Result<()> {
    inventory.summary().validate(ids)?;
    ordered_ids(&inventory.generated, "$suite.coverage.generated")?;
    ordered_ids(&inventory.authored, "$suite.coverage.authored")?;
    for (claimed, length) in [
        (inventory.counts.generated, inventory.generated.len()),
        (inventory.counts.authored, inventory.authored.len()),
        (inventory.counts.outside, inventory.outside.len()),
    ] {
        if u64::try_from(length).ok() != Some(claimed) {
            return Err(invalid(
                "CoverageCountMismatch",
                "inventory counts must equal actual list lengths",
            ));
        }
    }
    let mut survivors = BTreeMap::new();
    for (origin, selected) in [
        (Origin::Generated, &inventory.generated),
        (Origin::Authored, &inventory.authored),
    ] {
        for id in selected {
            if survivors.insert(id.clone(), origin).is_some() {
                return Err(invalid("DuplicateOrigin", id.as_str()));
            }
        }
    }
    if survivors.keys().ne(ids.iter()) {
        return Err(invalid(
            "SelectedIdsMismatch",
            "generated/authored union must equal scenario-map keys",
        ));
    }
    validate_outside(inventory, &mut survivors)?;
    let owners = source_owners(inventory, &survivors)?;
    for refusal in &inventory.refused {
        let expected = refusal.scenario.as_ref().and_then(|id| {
            survivors.get(id).map(|origin| Retained {
                origin: *origin,
                source: owners.get(id).cloned(),
            })
        });
        if refusal.retained != expected {
            return Err(invalid(
                "RetainedIdentityMismatch",
                "retained must name the final survivor and accepted source",
            ));
        }
        if let Some(source) = &refusal.source {
            let entry = inventory
                .authored_sources
                .get(source)
                .ok_or_else(|| invalid("MissingAuthoredSource", source.as_str()))?;
            if entry.disposition != SourceDisposition::Refused || entry.scenario != refusal.scenario
            {
                return Err(invalid(
                    "AuthoredSourceMismatch",
                    "refusal must retain its rejected source and known ID",
                ));
            }
        }
        if refusal.scope == RefusalScope::OutsideComponent
            && refusal.effect == RefusalEffect::CheckNotEmitted
            && refusal.retained.is_some()
        {
            let witness = inventory
                .outside
                .iter()
                .find(|outside| Some(&outside.scenario) == refusal.scenario.as_ref());
            if !witness.is_some_and(|outside| {
                outside.reason == OutsideReason::OtherComponent && outside.needs == refusal.needs
            }) {
                return Err(invalid(
                    "RefusalScopeMismatch",
                    "omitted check on a survivor needs its matching outside-component witness",
                ));
            }
        }
    }
    Ok(())
}

fn validate_outside(
    inventory: &Inventory,
    survivors: &mut BTreeMap<ScenarioId, Origin>,
) -> Result<()> {
    if inventory
        .outside
        .windows(2)
        .any(|pair| pair[0].scenario >= pair[1].scenario)
    {
        return Err(invalid(
            "OutsideOrder",
            "outside IDs must be sorted and distinct",
        ));
    }
    for outside in &inventory.outside {
        if survivors
            .insert(outside.scenario.clone(), outside.origin)
            .is_some()
        {
            return Err(invalid("OutsideSelectedOverlap", outside.scenario.as_str()));
        }
        validate_needs(
            &outside.needs,
            outside.reason == OutsideReason::OtherComponent,
        )?;
        let included = inventory.selection.origins.includes(outside.origin);
        let valid = match outside.reason {
            OutsideReason::OriginSelection => !included,
            OutsideReason::OtherComponent => {
                included && matches!(inventory.selection.scope, Scope::Component { .. })
            }
            OutsideReason::SelectionFilter => {
                included && matches!(inventory.selection.filter, Filter::Explicit { .. })
            }
        };
        if !valid {
            return Err(invalid(
                "OutsideReasonMismatch",
                "outside reason contradicts scope, origins or filter",
            ));
        }
    }
    Ok(())
}

fn source_owners(
    inventory: &Inventory,
    survivors: &BTreeMap<ScenarioId, Origin>,
) -> Result<BTreeMap<ScenarioId, SourceIdentity>> {
    let mut owners = BTreeMap::new();
    for (identity, source) in &inventory.authored_sources {
        if !original_digest(&source.digest) {
            return Err(invalid("MalformedSourceDigest", identity.as_str()));
        }
        match source.disposition {
            SourceDisposition::Accepted => {
                let id = source.scenario.as_ref().ok_or_else(|| {
                    invalid(
                        "AuthoredSourceMismatch",
                        "accepted source requires a known ID",
                    )
                })?;
                if survivors.get(id) != Some(&Origin::Authored)
                    || owners.insert(id.clone(), identity.clone()).is_some()
                {
                    return Err(invalid(
                        "AuthoredOwnershipMismatch",
                        "exactly one accepted source owns each authored survivor",
                    ));
                }
            }
            SourceDisposition::Refused => {
                if !inventory
                    .refused
                    .iter()
                    .any(|refusal| refusal.source.as_ref() == Some(identity))
                {
                    return Err(invalid("MissingSourceRefusal", identity.as_str()));
                }
            }
        }
    }
    if survivors
        .iter()
        .any(|(id, origin)| *origin == Origin::Authored && !owners.contains_key(id))
    {
        return Err(invalid(
            "MissingAuthoredOwner",
            "authored survivor requires its accepted source",
        ));
    }
    Ok(owners)
}

fn admit_child(child: &AdmittedSuite, parent: &AdmittedSuite) -> Result<()> {
    let Filter::Explicit {
        parent: expected, ..
    } = &child.inventory.selection.filter
    else {
        unreachable!("caller selected explicit child");
    };
    if expected != &parent.reference {
        return Err(invalid(
            "ParentReferenceMismatch",
            "next parent must match its exact original-byte reference",
        ));
    }
    let c = &child.inventory;
    let p = &parent.inventory;
    if c.selection.scope != p.selection.scope
        || c.selection.origins != p.selection.origins
        || c.knowledge != p.knowledge
    {
        return Err(invalid(
            "ParentSelectionMismatch",
            "filter cannot change scope, origins or knowledge",
        ));
    }
    if c.authored_sources != p.authored_sources || c.refused != p.refused {
        return Err(invalid(
            "ParentInventoryMismatch",
            "filter must retain sources and every refusal occurrence unchanged",
        ));
    }
    let child_root = child.document.object()?;
    let parent_root = parent.document.object()?;
    if !child_root["provenance"].equivalent(&parent_root["provenance"]) {
        return Err(invalid(
            "ParentProvenanceMismatch",
            "filter cannot change provenance",
        ));
    }
    let child_scenarios = child_root["scenarios"].object()?;
    let parent_scenarios = parent_root["scenarios"].object()?;
    for (id, scenario) in child_scenarios {
        if !parent_scenarios
            .get(id)
            .is_some_and(|original| scenario.equivalent(original))
        {
            return Err(invalid(
                "ParentScenarioMismatch",
                format!("changed or absent surviving scenario {id}"),
            ));
        }
    }
    let selected: BTreeSet<_> = child.ids.iter().collect();
    for (actual, original) in [(&c.generated, &p.generated), (&c.authored, &p.authored)] {
        if actual
            .iter()
            .ne(original.iter().filter(|id| selected.contains(id)))
        {
            return Err(invalid(
                "ParentOriginMismatch",
                "filter cannot change surviving origins",
            ));
        }
    }
    let mut outside = p.outside.clone();
    for (origin, ids) in [
        (Origin::Generated, &p.generated),
        (Origin::Authored, &p.authored),
    ] {
        for id in ids.iter().filter(|id| !selected.contains(id)) {
            outside.push(Outside {
                scenario: id.clone(),
                origin,
                reason: OutsideReason::SelectionFilter,
                needs: Vec::new(),
            });
        }
    }
    outside.sort_by(|a, b| a.scenario.cmp(&b.scenario));
    if c.outside != outside {
        return Err(invalid(
            "ParentOutsideMismatch",
            "retain prior outside entries and add precisely omitted selected IDs",
        ));
    }
    Ok(())
}
