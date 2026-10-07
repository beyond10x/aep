//! Complete coverage-suite inventory and offline original parent-chain admission.
use crate::count_json::{Json, Result};
use aep_domain::ess_conformance_coverage::{
    is_coverage_suite_version, ordered_ids, original_digest, validate_needs, Filter, Inventory,
    Origin, Outside, OutsideReason, RefusalEffect, RefusalScope, Retained, Scope,
    SourceDisposition, SourceIdentity, SuiteReference,
};
use aep_domain::ess_conformance_v2::{
    field_name, lower_kebab, qualified_name, EssAdmissionError, ScenarioId,
};
use aep_domain::SpecDigest;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub(crate) struct AdmittedSuite {
    definitions: crate::coverage_definition::Definitions,
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
        &["component", "scenario_initial_state", "synthesis_seeds"],
    )?;
    let version = provenance["suite_version"].text()?;
    if !is_coverage_suite_version(version) {
        return Err(provenance["suite_version"].error(
            "UnsupportedSuiteVersion",
            format!("{version} is not a coverage suite version this build admits"),
        ));
    }
    admit_provenance_vocabulary(major(version), &fields["provenance"], provenance)?;
    // The reference binds every original byte; the report must name exactly this digest.
    let reference = SuiteReference::new(
        version.into(),
        "sha256-json-bytes/1".into(),
        digest(original),
    )?;
    // Suite/5 is the one major whose scenario grammar AEP transcribed, and it keeps that closed
    // check. Later majors grow ESS's step vocabulary; ESS admitted the suite before reporting on
    // it and owns that grammar, so AEP reads only their scenario keys and compares bodies exactly.
    let transcribed = version == TRANSCRIBED_SUITE_VERSION;
    provenance["system"].text()?;
    provenance["specification_version"].text()?;
    let parse_digest = |value: &Json| {
        SpecDigest::new(value.text()?)
            .map_err(|error| value.error("MalformedModelDigest", error.to_string()))
    };
    let spec_digest = parse_digest(&provenance["spec_digest"])?;
    parse_digest(&provenance["contract_digest"])?;
    // Every key is read in the grammar current ESS parses and gated by the major that carries its
    // form, suite/5 included: a later form there is a later major's vocabulary, not a malformed id.
    let keys = scenario_keys(&fields["scenarios"])?;
    admit_id_forms(version, &keys, &fields["scenarios"])?;
    let ids = if transcribed {
        crate::count_suite::admit_scenarios(&fields["scenarios"])?
    } else {
        keys
    };
    let inventory = crate::coverage_wire::inventory(&fields["coverage"])?;
    admit_refusal_codes(version, &inventory, &fields["coverage"])?;
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
    if let Some(seeds) = provenance.get("synthesis_seeds") {
        admit_seed_record(major(version), seeds, &ids, &inventory)?;
    }
    let definitions = crate::coverage_definition::Definitions::read(&document, transcribed)?;
    Ok(AdmittedSuite {
        definitions,
        inventory,
        reference,
        spec_digest,
        ids,
    })
}

/// The coverage major whose complete scenario vocabulary `count_suite` transcribes.
const TRANSCRIBED_SUITE_VERSION: &str = "ess-conformance/5";

/// The first ordinary major whose provenance says every scenario starts from an empty state.
///
/// ESS 0.55.0 `admission.rs` `validate_suite`: `scenario_initial_state` is `empty` from suite/34
/// (the `one_time_response::ORDINARY` pair) and absent below it.
const INITIAL_STATE_MAJOR: u32 = 34;

/// The majors whose provenance may carry a seed record: ESS 0.55.0
/// `synthesis_seeds::admitted_in`, the seed pair (`synthesis_seeds::ORDINARY` 42 and `COVERAGE`
/// 43) and the counted event-claim pair over it (`event_multiplicity::ADMITTED`, 44 and 45).
const SEED_MAJORS: [u32; 4] = [42, 43, 44, 45];

/// The majors whose provenance must carry one (`synthesis_seeds::seed_major`).
const SEED_REQUIRED_MAJORS: [u32; 2] = [42, 43];

/// The provenance members ESS added after suite/5, each present exactly where ESS admits it.
///
/// Here AEP reads a seed record's structure, as ESS's `synthesis_seeds::admit_json` does, and the
/// scenario id each application names; `admit_seed_record` checks its names and relations once
/// the inventory is read. A selected child must carry its parent's record unchanged.
fn admit_provenance_vocabulary(
    major: u32,
    provenance: &Json,
    fields: &BTreeMap<String, Json>,
) -> Result<()> {
    let missing = |member: &str, detail: &str| {
        EssAdmissionError::new(
            "MissingField",
            format!("{}.{member}", provenance.path),
            detail,
        )
    };
    match fields.get("scenario_initial_state") {
        Some(state) if major < INITIAL_STATE_MAJOR => {
            return Err(state.error(
                "UnsupportedVocabulary",
                "scenario_initial_state requires suite/34 or later",
            ));
        }
        Some(state) if state.text()? != "empty" => {
            return Err(state.error(
                "UnsupportedVocabulary",
                "scenario_initial_state must be empty from suite/34",
            ));
        }
        None if major >= INITIAL_STATE_MAJOR => {
            return Err(missing(
                "scenario_initial_state",
                "suite/34 and later require scenario_initial_state",
            ));
        }
        Some(_) | None => {}
    }
    match fields.get("synthesis_seeds") {
        Some(seeds) if !SEED_MAJORS.contains(&major) => Err(seeds.error(
            "UnsupportedVocabulary",
            "synthesis seeds require suite/42 or /43, or the counted event-claim pair /44 or /45",
        )),
        Some(seeds) => admit_seeds(seeds),
        None if SEED_REQUIRED_MAJORS.contains(&major) => Err(missing(
            "synthesis_seeds",
            "suite/42 and /43 require synthesis_seeds",
        )),
        None => Ok(()),
    }
}

/// A seed record's closed structure (ESS 0.55.0 `synthesis_seeds::admit_json`), with each
/// application's scenario read as a scenario id.
fn admit_seeds(seeds: &Json) -> Result<()> {
    let fields = seeds.closed(&["sources", "selections", "applications"], &[])?;
    for digest in fields["sources"].object()?.values() {
        digest.text()?;
    }
    for selection in fields["selections"].array()? {
        let record = selection.closed(
            &[
                "source", "instance", "entity", "identity", "fields", "state",
            ],
            &[],
        )?;
        record["identity"].payload()?;
        record["fields"].object()?;
        record["fields"].payload()?;
    }
    for application in fields["applications"].array()? {
        let record = application.closed(
            &[
                "source",
                "instance",
                "scenario",
                "establish_step",
                "command_step",
            ],
            &[],
        )?;
        let scenario = &record["scenario"];
        ScenarioId::new(scenario.text()?)
            .map_err(|error| scenario.error("MalformedScenarioId", error.to_string()))?;
        record["establish_step"].unsigned()?;
        record["command_step"].unsigned()?;
    }
    Ok(())
}

/// The coverage major that excuses an application naming a scenario its selection filter moved
/// outside (ESS 0.55.0 `synthesis_seeds::COVERAGE`).
const SEED_COVERAGE_MAJOR: u32 = 43;

/// The most selections one seed record holds (ESS 0.55.0 `synthesis_seeds::MAX_SELECTIONS`).
const MAX_SEED_SELECTIONS: usize = 64;

/// The deepest a seed row's literal nests (ESS 0.55.0 `admission::setup_literal`).
const MAX_SETUP_NESTING: usize = 120;

/// One admitted seed row, by the names it is identified and ordered by.
struct SeedSelection<'a> {
    source: &'a str,
    instance: &'a str,
    entity: &'a str,
    record: &'a BTreeMap<String, Json>,
}

/// One use of a seed row in a generated scenario.
struct SeedApplication<'a> {
    source: &'a str,
    instance: &'a str,
    scenario: &'a str,
    establish_step: u64,
}

/// A seed record's names and relations, checked as ESS 0.55.0 checks them without reading a
/// scenario body, in ESS's order: the field types of `SynthesisSeeds`, `SeedRecord` and
/// `SeedApplication`, then `synthesis_seeds::admit_selections` (`synthesis_seeds.rs:220-262`,
/// with `admission::setup_row`) and `admit_applications` (`synthesis_seeds.rs:266-334`). Every
/// refusal names ESS's own cause.
///
/// Two of ESS's checks read what AEP leaves to ESS: whether an application's steps establish and
/// address its row, and whether every generated row is bound, read scenario steps. Applications
/// are ordered by `(scenario, establish_step)` (`synthesis_seeds.rs:275`), and ESS orders a
/// scenario id by its rendered name (`impl Ord for ScenarioId`, `scenario.rs:1045`), which for an
/// admitted id is its spelling.
fn admit_seed_record(
    major: u32,
    seeds: &Json,
    ids: &[ScenarioId],
    inventory: &Inventory,
) -> Result<()> {
    let fields = seeds.object()?;
    let mut sources = BTreeMap::new();
    for (identity, digest) in fields["sources"].object()? {
        source_identity(identity, digest)?;
        sources.insert(identity.as_str(), digest);
    }
    let mut selections = Vec::new();
    for selection in fields["selections"].array()? {
        let record = selection.object()?;
        selections.push(SeedSelection {
            source: source_identity(record["source"].text()?, &record["source"])?,
            instance: seed_name(&record["instance"], "instance name", lower_kebab)?,
            entity: seed_name(&record["entity"], "qualified name", qualified_name)?,
            record,
        });
        seed_name(&record["state"], "state name", state_name)?;
    }
    let mut applications = Vec::new();
    for application in fields["applications"].array()? {
        let record = application.object()?;
        applications.push(SeedApplication {
            source: source_identity(record["source"].text()?, &record["source"])?,
            instance: seed_name(&record["instance"], "instance name", lower_kebab)?,
            scenario: record["scenario"].text()?,
            establish_step: record["establish_step"].unsigned()?,
        });
    }
    admit_seed_selections(seeds, &sources, &selections)?;
    admit_seed_applications(seeds, major, &selections, &applications, ids, inventory)
}

/// ESS 0.55.0 `synthesis_seeds::admit_selections`, with `admission::setup_row`.
fn admit_seed_selections(
    seeds: &Json,
    sources: &BTreeMap<&str, &Json>,
    selections: &[SeedSelection<'_>],
) -> Result<()> {
    let refuse = |detail: &str| seeds.error("InvalidShape", detail);
    if sources.is_empty() || selections.is_empty() {
        return Err(refuse("sources and selections must be nonempty"));
    }
    if selections.len() > MAX_SEED_SELECTIONS {
        return Err(refuse("more than 64 selections"));
    }
    for digest in sources.values() {
        if !original_digest(digest.text()?) {
            return Err(digest.error("MalformedSourceDigest", "invalid source digest"));
        }
    }
    if !selections
        .windows(2)
        .all(|pair| (pair[0].source, pair[0].instance) < (pair[1].source, pair[1].instance))
    {
        return Err(refuse("selections must be sorted and distinct"));
    }
    let mut used = BTreeSet::new();
    let mut identities = Vec::new();
    for selection in selections {
        if !sources.contains_key(selection.source) {
            return Err(refuse("a selection names an unknown source"));
        }
        used.insert(selection.source);
        let identity = Rendered::of(&selection.record["identity"])?;
        if identities.contains(&(selection.entity, identity.clone())) {
            return Err(refuse("two selections share one qualified identity"));
        }
        identities.push((selection.entity, identity));
        setup_row(selection.record)?;
    }
    if used.len() != sources.len() {
        return Err(refuse("a source is selected by no selection"));
    }
    Ok(())
}

/// ESS 0.55.0 `synthesis_seeds::admit_applications`, without the step checks that read scenario
/// bodies.
fn admit_seed_applications(
    seeds: &Json,
    major: u32,
    selections: &[SeedSelection<'_>],
    applications: &[SeedApplication<'_>],
    ids: &[ScenarioId],
    inventory: &Inventory,
) -> Result<()> {
    let refuse = |detail: &str| seeds.error("InvalidShape", detail);
    if !applications.windows(2).all(|pair| {
        (pair[0].scenario, pair[0].establish_step) < (pair[1].scenario, pair[1].establish_step)
    }) {
        return Err(refuse("applications must be sorted and distinct"));
    }
    for application in applications {
        if !selections.iter().any(|selection| {
            (selection.source, selection.instance) == (application.source, application.instance)
        }) {
            return Err(refuse("an application names an unknown selection"));
        }
        if application.scenario.split('/').nth(1) == Some("authored") {
            return Err(refuse("an application names an authored scenario"));
        }
        let held = ids.iter().any(|id| id.as_str() == application.scenario);
        let filtered = major == SEED_COVERAGE_MAJOR
            && inventory.outside.iter().any(|outside| {
                outside.scenario.as_str() == application.scenario
                    && outside.reason == OutsideReason::SelectionFilter
            });
        if !held && !filtered {
            return Err(refuse(
                "an application names a scenario the suite does not hold",
            ));
        }
    }
    Ok(())
}

/// A source identity in ESS's checked root-relative grammar.
fn source_identity<'a>(identity: &'a str, at: &Json) -> Result<&'a str> {
    SourceIdentity::new(identity.to_owned()).map_err(|_| {
        at.error(
            "MalformedSourceIdentity",
            format!("authored source identity must be a checked root-relative path: {identity}"),
        )
    })?;
    Ok(identity)
}

/// A name a seed record carries, in the grammar its ESS type parses.
fn seed_name<'a>(value: &'a Json, kind: &str, valid: fn(&str) -> bool) -> Result<&'a str> {
    let name = value.text()?;
    if valid(name) {
        Ok(name)
    } else {
        Err(value.error(
            "MalformedName",
            format!("invalid {kind} identifier {name:?}"),
        ))
    }
}

/// ESS's state-name grammar: one `UpperCamelCase` ASCII word (`ess_domain::entity::StateName`).
fn state_name(value: &str) -> bool {
    value.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
        && value.bytes().all(|c| c.is_ascii_alphanumeric())
}

/// A seed row as ESS 0.55.0 `admission::setup_row` admits it; its numbers are already finite.
fn setup_row(record: &BTreeMap<String, Json>) -> Result<()> {
    let values = record["fields"].object()?;
    for literal in std::iter::once(&record["identity"]).chain(values.values()) {
        if nesting(literal) > MAX_SETUP_NESTING {
            return Err(literal.error("InvalidDocument", "setup literal nesting exceeds 120"));
        }
    }
    if record["identity"].null() {
        return Err(record["identity"].error("InvalidShape", "identity cannot be null"));
    }
    if let Some((name, value)) = values.iter().find(|(name, _)| !field_name(name)) {
        return Err(value.error(
            "MalformedName",
            format!("field names must be local identifiers: {name:?}"),
        ));
    }
    Ok(())
}

/// The magnitude up to which ESS carries an integral binary64 as the integer it is (ESS 0.55.0
/// `ess_primitives::facts::INTEGER_CARRIER`, `facts.rs:73`).
const INTEGER_CARRIER: f64 = 9_223_372_036_854_775_808.0;

/// A seed identity as ESS 0.55.0 renders it to tell two rows apart: `admit_selections` compares
/// `serde_json::to_string(&record.identity)` (`synthesis_seeds.rs:250`), so two identities are one
/// exactly when their rendered texts are equal.
///
/// ESS reads an integer token that fits 64 bits exactly and every other number through binary64
/// (`Number`'s `visit_i64`, `visit_u64` and `visit_f64`, `facts.rs:113-142`). It writes a number
/// as its binary64 where that carries the exact value and as the integer where it does not
/// (`impl Serialize for Number`, `facts.rs:173`, deciding with `canonical_decimal`,
/// `facts.rs:409`). So `9007199254740992`, `9007199254740992.0` and `9.007199254740992e15` are one
/// identity and `9007199254740993` is another. The key is built from the original tokens, so no
/// `serde_json` number feature unified into a build decides it.
#[derive(Clone, PartialEq)]
enum Rendered {
    Null,
    Bool(bool),
    /// An integer its binary64 does not carry, written exactly.
    Integer(i128),
    /// Every other number, by its binary64's bits.
    Binary64(u64),
    Text(String),
    Seq(Vec<Rendered>),
    /// Members by name, in order, as ESS's `Node::Map` holds them.
    Map(Vec<(String, Rendered)>),
}

impl Rendered {
    fn of(value: &Json) -> Result<Self> {
        if let Ok(members) = value.object() {
            return members
                .iter()
                .map(|(name, member)| Ok((name.clone(), Self::of(member)?)))
                .collect::<Result<_>>()
                .map(Self::Map);
        }
        if let Ok(items) = value.array() {
            return items
                .iter()
                .map(Self::of)
                .collect::<Result<_>>()
                .map(Self::Seq);
        }
        if let Ok(text) = value.text() {
            return Ok(Self::Text(text.to_owned()));
        }
        if value.null() {
            return Ok(Self::Null);
        }
        if let Ok(flag) = value.boolean() {
            return Ok(Self::Bool(flag));
        }
        Self::number(value)
    }

    fn number(value: &Json) -> Result<Self> {
        let raw = value.raw.as_str();
        // `serde_json` reads `-0` as the binary64 `-0.0`, whose sign survives in ESS's writing.
        let exact = if raw.contains(['.', 'e', 'E']) || raw.starts_with("-0") {
            None
        } else {
            raw.parse::<i64>()
                .map(i128::from)
                .or_else(|_| raw.parse::<u64>().map(i128::from))
                .ok()
        };
        let Some(integer) = exact else {
            return raw
                .parse::<f64>()
                .map(|binary| Self::Binary64(binary.to_bits()))
                .map_err(|error| value.error("InvalidShape", error.to_string()));
        };
        // `canonical_decimal`: an integral binary64 up to 2^63 is that integer; past it, the
        // integer its shortest decimal spells.
        #[allow(clippy::cast_precision_loss)]
        let binary = integer as f64;
        #[allow(clippy::cast_possible_truncation)]
        let carried = if binary.abs() <= INTEGER_CARRIER {
            binary as i128 == integer
        } else {
            format!("{binary}").parse::<i128>().ok() == Some(integer)
        };
        Ok(if carried {
            Self::Binary64(binary.to_bits())
        } else {
            Self::Integer(integer)
        })
    }
}

/// How deep the deepest member of a literal sits below it.
fn nesting(value: &Json) -> usize {
    let children: Vec<&Json> = if let Ok(members) = value.object() {
        members.values().collect()
    } else if let Ok(items) = value.array() {
        items.iter().collect()
    } else {
        Vec::new()
    };
    children
        .into_iter()
        .map(|child| 1 + nesting(child))
        .max()
        .unwrap_or(0)
}

/// Scenario-id forms ESS added after suite/5, each with the first ordinary major that carries it.
///
/// A form is the id after its subject. A segment in angle brackets stands for any one name, and
/// a final `...` for every remaining segment; `ScenarioId::new` has already checked the names.
/// ESS 0.55.0 refuses a suite whose scenario keys use a form below its major, each in the module
/// named here, with the constant the major is read from; the coverage counterpart is one above.
///
/// | form | ESS module | constant |
/// |---|---|---|
/// | `aggregate` | `aggregate::admit_suite` | `aggregate::ORDINARY` (16) |
/// | `binding/final-failure` | `bounded_retry::admit_format` | `bounded_retry::ORDINARY` (26) |
/// | `grant/denied`, `grant/admitted/<actor>` | `grant::admit_format` | `grant::ORDINARY` (26) |
/// | `grant/read/denied`, `grant/read/admitted/<actor>` | `view_grant::admit_format` | `view_grant::ORDINARY` (34) |
/// | `disclosure/...` | `one_time_response::admit` | `one_time_response::ORDINARY` (34) |
/// | `binding/refusal/<outcome>` | `refusal_policy::admit` | `refusal_policy::ORDINARY` (36) |
/// | `binding/condition-false`, `binding/condition-absent` | `no_invocation::admit` | `no_invocation::ORDINARY` (36) |
const LATER_ID_FORMS: [(&[&str], u32); 10] = [
    (&["aggregate"], 16),
    (&["binding", "final-failure"], 26),
    (&["grant", "denied"], 26),
    (&["grant", "admitted", "<actor>"], 26),
    (&["grant", "read", "denied"], 34),
    (&["grant", "read", "admitted", "<actor>"], 34),
    (&["disclosure", "..."], 34),
    (&["binding", "refusal", "<outcome>"], 36),
    (&["binding", "condition-false"], 36),
    (&["binding", "condition-absent"], 36),
];

/// Whether the segments after an id's subject spell `form`.
fn spells(tail: &[&str], form: &[&str]) -> bool {
    match (form, tail) {
        (["..."], _) => true,
        ([word, form @ ..], [part, tail @ ..]) => {
            (word.starts_with('<') || word == part) && spells(tail, form)
        }
        (form, tail) => form.is_empty() && tail.is_empty(),
    }
}

/// Which suite majors may carry an admitted scenario id.
enum IdForm {
    /// A frozen suite/1–4 form, carried by every coverage major.
    Frozen,
    /// A later form, carried from its first ordinary major on.
    Later { form: String, ordinary: u32 },
    /// A form `ScenarioId::new` admits that `LATER_ID_FORMS` does not gate: one added to the
    /// grammar without its major. It is refused rather than admitted at every major.
    Ungated,
}

fn id_form(id: &ScenarioId) -> IdForm {
    let parts: Vec<&str> = id.as_str().split('/').collect();
    if let Some((form, ordinary)) = LATER_ID_FORMS
        .iter()
        .find(|(form, _)| spells(&parts[1..], form))
    {
        IdForm::Later {
            form: form.join("/"),
            ordinary: *ordinary,
        }
    } else if ScenarioId::frozen(id.as_str()).is_ok() {
        IdForm::Frozen
    } else {
        IdForm::Ungated
    }
}

fn major(version: &str) -> u32 {
    version
        .strip_prefix("ess-conformance/")
        .and_then(|digits| digits.parse().ok())
        .expect("an admitted coverage version")
}

/// Refusal codes ESS added after suite/5, each with the first coverage major that admits it.
///
/// ESS `Inventory::validate` (`coverage.rs`): `ESS-SYNTH-015` needs suite/7, and
/// `aggregate::is_aggregate_refusal` (`ESS-SYNTH-016`, `-017`) suite/17 (`aggregate::COVERAGE`).
const LATER_REFUSAL_CODES: [(&str, u32); 3] = [
    ("ESS-SYNTH-015", 7),
    ("ESS-SYNTH-016", 17),
    ("ESS-SYNTH-017", 17),
];

fn admit_refusal_codes(version: &str, inventory: &Inventory, coverage: &Json) -> Result<()> {
    let major = major(version);
    for refusal in &inventory.refused {
        if let Some((code, coverage_major)) = LATER_REFUSAL_CODES
            .iter()
            .find(|(code, first)| refusal.code == *code && major < *first)
        {
            return Err(coverage.error(
                "UnsupportedVocabulary",
                format!("refusal {code} requires suite/{coverage_major}"),
            ));
        }
    }
    Ok(())
}

fn admit_id_forms(version: &str, ids: &[ScenarioId], scenarios: &Json) -> Result<()> {
    let major = major(version);
    for id in ids {
        match id_form(id) {
            IdForm::Later { form, ordinary } if major < ordinary => {
                return Err(scenarios.error(
                    "UnsupportedVocabulary",
                    format!(
                        "{id} uses the `{form}` form, which requires suite/{ordinary} or /{}",
                        ordinary + 1,
                        id = id.as_str()
                    ),
                ));
            }
            IdForm::Frozen | IdForm::Later { .. } => {}
            IdForm::Ungated => {
                return Err(scenarios.error(
                    "UnsupportedVocabulary",
                    format!(
                        "{} uses a form whose first suite major this build does not know",
                        id.as_str()
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// Scenario identities of a suite whose bodies ESS alone interprets.
fn scenario_keys(scenarios: &Json) -> Result<Vec<ScenarioId>> {
    scenarios
        .object()?
        .iter()
        .map(|(id, scenario)| {
            ScenarioId::new(id.clone())
                .map_err(|error| scenario.error("MalformedScenarioId", error.to_string()))
        })
        .collect()
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
    if child.definitions.provenance != parent.definitions.provenance {
        return Err(invalid(
            "ParentProvenanceMismatch",
            "filter cannot change provenance",
        ));
    }
    for (id, scenario) in &child.definitions.scenarios {
        if parent.definitions.scenarios.get(id) != Some(scenario) {
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

#[cfg(test)]
mod tests {
    use super::{admit_id_forms, id_form, IdForm, Rendered, LATER_ID_FORMS};
    use crate::count_json::Json;
    use aep_domain::ess_conformance_v2::ScenarioId;

    /// One id of each form ESS added after suite/5, with the first ordinary major ESS 0.55.0
    /// carries it in, for checking the table against. The reader's gate is tested through the
    /// reader in `tests/current_suites.rs`, which admits and refuses each form at its majors.
    const LATER: [(&str, u32); 11] = [
        ("metrics.session.ByAgent/aggregate", 16),
        ("notify-ledger/binding/final-failure", 26),
        ("billing.invoice.IssueInvoice/grant/denied", 26),
        ("desk.ops.Tally/grant/admitted/desk.ops.Watcher", 26),
        ("desk.tickets.Board/grant/read/denied", 34),
        ("desk.tickets.Board/grant/read/admitted/desk.tickets.Clerk", 34),
        ("auth.keys.Issue/disclosure/issued/secret/origin/as/anonymous", 34),
        ("auth.keys.Issue/disclosure/issued/secret/denied/auth.keys.Revoke/as/actor/auth.keys.Other", 34),
        ("notify-ledger/binding/refusal/at-limit", 36),
        ("notify-ledger/binding/condition-false", 36),
        ("notify-ledger/binding/condition-absent", 36),
    ];

    #[test]
    fn every_gated_form_is_the_form_of_an_example_at_its_own_major() {
        for (form, ordinary) in LATER_ID_FORMS {
            let form = form.join("/");
            let example = LATER.iter().any(|(id, major)| {
                matches!(
                    id_form(&ScenarioId::new(*id).unwrap()),
                    IdForm::Later { form: spelled, ordinary: first }
                        if spelled == form && first == ordinary && *major == ordinary
                )
            });
            assert!(example, "no example of `{form}` at /{ordinary}");
        }
    }

    #[test]
    fn a_frozen_form_is_admitted_at_every_major() {
        let scenarios = Json::parse("{}", "$suite.scenarios").unwrap();
        let ids = [ScenarioId::new("notify-ledger/binding/flow").unwrap()];
        for major in [5, 7, 33, 45] {
            admit_id_forms(&format!("ess-conformance/{major}"), &ids, &scenarios)
                .unwrap_or_else(|error| panic!("/{major}: {error}"));
        }
    }

    #[test]
    fn seed_identities_compare_as_ess_renders_them() {
        // Read off `ess_primitives::facts` at ESS 0.55.0 (`visit_*`, `Serialize`,
        // `canonical_decimal`); the first two pairs were also measured on `ess 0.55.0`.
        let key = |text: &str| Rendered::of(&Json::parse(text, "$").unwrap()).unwrap();
        for (left, right, same) in [
            ("9007199254740992", "9007199254740992.0", true),
            ("9007199254740993", "9.007199254740992e15", false),
            ("9007199254740992.0", "9.007199254740992e15", true),
            ("1", "1.0", true),
            ("-0", "0", false),
            ("-0", "-0.0", true),
            ("9223372036854775807", "9223372036854775806", false),
            ("9223372036854775808", "9.223372036854776e18", true),
            ("18446744073709551615", "1.8446744073709552e19", false),
            ("10000000000000000000", "1e19", true),
            ("\"1\"", "1", false),
            ("null", "\"null\"", false),
            ("{\"b\": 1, \"a\": [2]}", "{\"a\": [2.0], \"b\": 1.0}", true),
            ("[1, 2]", "[2, 1]", false),
        ] {
            assert_eq!(key(left) == key(right), same, "{left} vs {right}");
        }
    }
}
