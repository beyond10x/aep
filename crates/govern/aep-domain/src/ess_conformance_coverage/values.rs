//! Closed values for the ESS suite/5 selection and inventory contract.
use std::collections::BTreeMap;

use crate::ess_conformance_v2::{lower_kebab, qualified_name, EssAdmissionError, ScenarioId};

macro_rules! vocabulary {
    ($name:ident, $doc:literal, {$($variant:ident => $wire:literal),+ $(,)?}) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
        pub enum $name { $(#[doc = concat!("`", $wire, "`")] #[serde(rename = $wire)] $variant),+ }
        impl $name {
            /// Exact wire spelling.
            pub const fn as_str(self) -> &'static str { match self { $(Self::$variant => $wire),+ } }
        }
    };
}

vocabulary!(Origin, "The source of an executable candidate.", { Authored => "authored", Generated => "generated" });
vocabulary!(Origins, "The origins selected by the independently declared boundary.", { Authored => "authored", Generated => "generated", GeneratedAndAuthored => "generated_and_authored" });
vocabulary!(Knowledge, "Whether the builder declares a complete inventory.", { CompleteInventory => "complete_inventory", Unknown => "unknown" });
vocabulary!(OutsideReason, "Why a known executable candidate is outside the selection.", { OtherComponent => "other_component", OriginSelection => "origin_selection", SelectionFilter => "selection_filter" });
vocabulary!(RefusalEffect, "What the producer could not emit, independently of surviving IDs.", { CandidateNotEmitted => "candidate_not_emitted", CheckNotEmitted => "check_not_emitted" });
vocabulary!(RefusalScope, "Applicability assigned before explicit filtering.", { InScope => "in_scope", OutsideComponent => "outside_component", OutsideOrigin => "outside_origin" });
vocabulary!(SourceDisposition, "Final authored ownership after merging candidates.", { Accepted => "accepted", Refused => "refused" });

impl Origins {
    /// Whether this declared selection includes an origin.
    pub const fn includes(self, origin: Origin) -> bool {
        matches!(
            (self, origin),
            (Self::GeneratedAndAuthored, _)
                | (Self::Generated, Origin::Generated)
                | (Self::Authored, Origin::Authored)
        )
    }
}

/// Whether a digest spells the original-byte SHA-256 profile exactly.
pub fn original_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    })
}

/// Checked exact suite/5 reference. It does not broaden the frozen count reference.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SuiteReference {
    version: String,
    digest_profile: String,
    digest: String,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawReference {
    version: String,
    digest_profile: String,
    digest: String,
}
impl SuiteReference {
    /// Checks the separately supported suite version and original-byte digest grammar.
    pub fn new(
        version: String,
        digest_profile: String,
        digest: String,
    ) -> Result<Self, EssAdmissionError> {
        if version != "ess-conformance/5" {
            return Err(EssAdmissionError::new(
                "UnsupportedSuiteVersion",
                "$.suite.version",
                version,
            ));
        }
        if digest_profile != "sha256-json-bytes/1" {
            return Err(EssAdmissionError::new(
                "UnsupportedDigestProfile",
                "$.suite.digest_profile",
                digest_profile,
            ));
        }
        if !original_digest(&digest) {
            return Err(EssAdmissionError::new(
                "MalformedSuiteDigest",
                "$.suite.digest",
                digest,
            ));
        }
        Ok(Self {
            version,
            digest_profile,
            digest,
        })
    }
    /// Exactly ess-conformance/5.
    pub fn version(&self) -> &str {
        &self.version
    }
    /// Exactly sha256-json-bytes/1.
    pub fn digest_profile(&self) -> &str {
        &self.digest_profile
    }
    /// Original selected suite SHA-256.
    pub fn digest(&self) -> &str {
        &self.digest
    }
}
impl<'de> serde::Deserialize<'de> for SuiteReference {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = RawReference::deserialize(d)?;
        Self::new(raw.version, raw.digest_profile, raw.digest).map_err(serde::de::Error::custom)
    }
}

/// Whole-system or named-component coverage boundary.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Scope {
    /// The system boundary.
    System,
    /// One explicitly named component.
    Component {
        /// Original component name.
        component: String,
    },
}
/// An unfiltered selection or a precise narrowing of an admitted parent.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Filter {
    /// No parent or selected-ID filter.
    All,
    /// Exact IDs selected from the immediately preceding suite.
    Explicit {
        /// Sorted distinct requested identities, possibly empty.
        ids: Vec<ScenarioId>,
        /// Exact immediate parent.
        parent: SuiteReference,
    },
}
/// Complete declared selection, never inferred from an execution count.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    /// System or component boundary.
    pub scope: Scope,
    /// Selected source families.
    pub origins: Origins,
    /// Full parent-bound filter.
    pub filter: Filter,
}
impl Selection {
    /// Checks names and filter order for typed callers as well as raw parsers.
    pub fn validate(&self) -> Result<(), EssAdmissionError> {
        if let Scope::Component { component } = &self.scope {
            if !lower_kebab(component) {
                return Err(EssAdmissionError::new(
                    "MalformedComponent",
                    "$.selection.scope.component",
                    component,
                ));
            }
        }
        if let Filter::Explicit { ids, .. } = &self.filter {
            ordered_ids(ids, "$.selection.filter.ids")?;
        }
        Ok(())
    }
}

/// Checks sorted distinct scenario identities without changing their order.
pub fn ordered_ids(ids: &[ScenarioId], path: &str) -> Result<(), EssAdmissionError> {
    if ids.windows(2).any(|pair| pair[0] >= pair[1]) {
        Err(EssAdmissionError::new(
            "SelectionOrder",
            path,
            "IDs must be sorted and distinct",
        ))
    } else {
        Ok(())
    }
}

/// Named command outcome in the inherited ESS semantic vocabulary.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct OutcomeReference {
    /// Qualified command name.
    pub command: String,
    /// Lower-kebab outcome slot.
    pub outcome: String,
}
/// Named entity transition in the inherited ESS semantic vocabulary.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct TransitionReference {
    /// Qualified entity name.
    pub entity: String,
    /// Local transition name.
    pub transition: String,
}
/// ESS semantic references in their established ordering, retained without a modeling dependency.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(
    tag = "kind",
    content = "name",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SemanticReference {
    /// Domain.
    Domain(String),
    /// Declared type.
    Type(String),
    /// Entity.
    Entity(String),
    /// Command.
    Command(String),
    /// Outcome.
    Outcome(OutcomeReference),
    /// Event.
    Event(String),
    /// Error.
    Error(String),
    /// View.
    View(String),
    /// Actor.
    Actor(String),
    /// Transition.
    Transition(TransitionReference),
    /// Binding.
    Binding(String),
    /// Component.
    Component(String),
}
impl SemanticReference {
    /// Checks the inherited name grammar, independently of model resolution by the builder.
    pub fn validate(&self) -> Result<(), EssAdmissionError> {
        let valid = match self {
            Self::Domain(name)
            | Self::Type(name)
            | Self::Entity(name)
            | Self::Command(name)
            | Self::Event(name)
            | Self::Error(name)
            | Self::View(name)
            | Self::Actor(name) => qualified_name(name),
            Self::Binding(name) | Self::Component(name) => lower_kebab(name),
            Self::Outcome(name) => qualified_name(&name.command) && lower_kebab(&name.outcome),
            Self::Transition(name) => {
                qualified_name(&name.entity)
                    && qualified_name(&name.transition)
                    && !name.transition.contains('.')
            }
        };
        if valid {
            Ok(())
        } else {
            Err(EssAdmissionError::new(
                "MalformedSemanticReference",
                "$.coverage",
                format!("{self:?}"),
            ))
        }
    }
}

/// Checked root-relative authored input identity. This value supplies no filesystem authority.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct SourceIdentity(String);
impl SourceIdentity {
    /// Checks the transport grammar without normalizing Unicode, case or separators.
    pub fn new(value: String) -> Result<Self, EssAdmissionError> {
        if value
            .chars()
            .any(|c| c.is_control() || c == '\\' || c == ':')
            || value
                .split('/')
                .any(|s| s.is_empty() || s == "." || s == "..")
        {
            return Err(EssAdmissionError::new(
                "MalformedSourceIdentity",
                "$.coverage.authored_sources",
                value,
            ));
        }
        Ok(Self(value))
    }
    /// Original identity spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<'de> serde::Deserialize<'de> for SourceIdentity {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

/// Final source ownership after compilation and candidate merging.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct AuthoredSource {
    /// SHA-256 of exact original source bytes.
    pub digest: String,
    /// Known scenario identity, including for a refused candidate.
    pub scenario: Option<ScenarioId>,
    /// Accepted owner or refused input.
    pub disposition: SourceDisposition,
}
/// A known executable candidate outside the declared selected IDs.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct Outside {
    /// Candidate identity.
    pub scenario: ScenarioId,
    /// Candidate source family.
    pub origin: Origin,
    /// Why it is outside.
    pub reason: OutsideReason,
    /// Sorted distinct dependency witnesses; nonempty only for another component.
    pub needs: Vec<SemanticReference>,
}
/// The surviving scenario with a refusal's same identity.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct Retained {
    /// Surviving origin, which can differ from the refused origin after merging.
    pub origin: Origin,
    /// Accepted source for authored survivors, otherwise null.
    pub source: Option<SourceIdentity>,
}
/// One original refusal occurrence. Identical occurrences remain separate array elements.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct Refusal {
    /// Refused candidate's source family.
    pub origin: Origin,
    /// Known candidate identity.
    pub scenario: Option<ScenarioId>,
    /// Known refused semantic subject.
    pub subject: Option<SemanticReference>,
    /// Rejected authored source, otherwise null.
    pub source: Option<SourceIdentity>,
    /// Exact closed baseline diagnostic code.
    pub code: String,
    /// Original nonempty cause text.
    pub message: String,
    /// Missing candidate or omitted check.
    pub effect: RefusalEffect,
    /// Surviving identity, if one exists.
    pub retained: Option<Retained>,
    /// Applicability fixed before explicit filtering.
    pub scope: RefusalScope,
    /// Sorted distinct other-component witnesses, otherwise empty.
    pub needs: Vec<SemanticReference>,
}
impl Refusal {
    /// Wire-defined occurrence ordering; equality deliberately preserves multiplicity.
    pub fn compare(&self, other: &Self) -> std::cmp::Ordering {
        (
            &self.origin,
            &self.source,
            &self.subject,
            &self.scenario,
            &self.code,
            &self.effect,
            &self.retained,
            &self.scope,
            &self.needs,
            &self.message,
        )
            .cmp(&(
                &other.origin,
                &other.source,
                &other.subject,
                &other.scenario,
                &other.code,
                &other.effect,
                &other.retained,
                &other.scope,
                &other.needs,
                &other.message,
            ))
    }
    /// Checks the frozen code-to-effect mapping and local refusal structure.
    pub fn validate(&self) -> Result<(), EssAdmissionError> {
        let expected = match self.code.as_str() {
            "ESS-SYNTH-005" | "ESS-SYNTH-011" | "ESS-SYNTH-012" | "ESS-SYNTH-014"
                if self.origin == Origin::Generated =>
            {
                Some(RefusalEffect::CheckNotEmitted)
            }
            "ESS-SYNTH-001" | "ESS-SYNTH-002" | "ESS-SYNTH-003" | "ESS-SYNTH-004"
            | "ESS-SYNTH-006" | "ESS-SYNTH-007" | "ESS-SYNTH-008" | "ESS-SYNTH-009"
            | "ESS-SYNTH-010" | "ESS-SYNTH-013"
                if self.origin == Origin::Generated =>
            {
                Some(RefusalEffect::CandidateNotEmitted)
            }
            code if self.origin == Origin::Authored
                && (1..=35).any(|number| code == format!("ESS-AUTHOR-{number:03}")) =>
            {
                Some(RefusalEffect::CandidateNotEmitted)
            }
            _ => None,
        };
        let Some(effect) = expected else {
            return Err(EssAdmissionError::new(
                "UnsupportedRefusalCode",
                "$.coverage.refused",
                &self.code,
            ));
        };
        if effect != self.effect {
            return Err(EssAdmissionError::new(
                "RefusalEffectMismatch",
                "$.coverage.refused",
                &self.code,
            ));
        }
        if self.message.trim().is_empty() {
            return Err(EssAdmissionError::new(
                "EmptyRefusalMessage",
                "$.coverage.refused",
                &self.code,
            ));
        }
        if (self.origin == Origin::Generated && (self.subject.is_none() || self.source.is_some()))
            || (self.origin == Origin::Authored && self.source.is_none())
        {
            return Err(EssAdmissionError::new(
                "RefusalIdentityMismatch",
                "$.coverage.refused",
                "generated requires subject only; authored requires source",
            ));
        }
        if self.scenario.is_none() && self.retained.is_some() {
            return Err(EssAdmissionError::new(
                "RetainedIdentityMismatch",
                "$.coverage.refused",
                "unknown scenario has no survivor",
            ));
        }
        if let Some(retained) = &self.retained {
            if (retained.origin == Origin::Authored) != retained.source.is_some() {
                return Err(EssAdmissionError::new(
                    "RetainedIdentityMismatch",
                    "$.coverage.refused",
                    "only authored survivor has a source",
                ));
            }
        }
        if let Some(subject) = &self.subject {
            subject.validate()?;
        }
        validate_needs(&self.needs, self.scope == RefusalScope::OutsideComponent)
    }
}

/// Checks dependency witness order and the required empty/nonempty shape.
pub fn validate_needs(
    needs: &[SemanticReference],
    required: bool,
) -> Result<(), EssAdmissionError> {
    if required == needs.is_empty() {
        return Err(EssAdmissionError::new(
            "NeedsMismatch",
            "$.coverage",
            "only other-component scope carries nonempty witnesses",
        ));
    }
    if needs.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(EssAdmissionError::new(
            "NeedsOrder",
            "$.coverage",
            "witnesses must be sorted and distinct",
        ));
    }
    for reference in needs {
        reference.validate()?;
    }
    Ok(())
}

/// Exact inventory counts; refused counts occurrences, not distinct scenarios.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct CoverageCounts {
    /// Selected generated scenarios.
    pub generated: u64,
    /// Selected authored scenarios.
    pub authored: u64,
    /// Known outside scenarios.
    pub outside: u64,
    /// Original refusal occurrences.
    pub refused: u64,
}
/// Report-visible summary of the exact paired suite's coverage.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct CoverageSummary {
    /// Complete independent selection boundary.
    pub selection: Selection,
    /// Builder inventory claim.
    pub knowledge: Knowledge,
    /// Exact category counts.
    pub counts: CoverageCounts,
    /// Every original refusal occurrence.
    pub refused: Vec<Refusal>,
}
impl CoverageSummary {
    /// Completeness is independent of nonempty selection and execution success.
    pub fn complete(&self) -> bool {
        self.knowledge == Knowledge::CompleteInventory
            && !self
                .refused
                .iter()
                .any(|r| r.scope == RefusalScope::InScope)
    }
    /// Checks summary arithmetic, origin selection, occurrence order and applicability.
    pub fn validate(&self, selected: &[ScenarioId]) -> Result<(), EssAdmissionError> {
        self.selection.validate()?;
        if self.counts.generated.checked_add(self.counts.authored)
            != u64::try_from(selected.len()).ok()
        {
            return Err(EssAdmissionError::new(
                "CoverageCountMismatch",
                "$.coverage.counts",
                "selected inventory sum must match outcomes without overflow",
            ));
        }
        if u64::try_from(self.refused.len()).ok() != Some(self.counts.refused) {
            return Err(EssAdmissionError::new(
                "RefusalCountMismatch",
                "$.coverage.counts.refused",
                "count each occurrence",
            ));
        }
        if (self.selection.origins == Origins::Generated && self.counts.authored != 0)
            || (self.selection.origins == Origins::Authored && self.counts.generated != 0)
        {
            return Err(EssAdmissionError::new(
                "OriginSelectionMismatch",
                "$.coverage.counts",
                "selected origin is excluded",
            ));
        }
        if let Filter::Explicit { ids, .. } = &self.selection.filter {
            if ids != selected {
                return Err(EssAdmissionError::new(
                    "SelectedIdsMismatch",
                    "$.coverage.selection.filter.ids",
                    "explicit IDs must equal selected scenarios",
                ));
            }
        }
        if self
            .refused
            .windows(2)
            .any(|pair| pair[0].compare(&pair[1]).is_gt())
        {
            return Err(EssAdmissionError::new(
                "RefusalOrder",
                "$.coverage.refused",
                "occurrences must be sorted, retaining equals",
            ));
        }
        for refusal in &self.refused {
            refusal.validate()?;
            if self.selection.origins.includes(refusal.origin)
                == (refusal.scope == RefusalScope::OutsideOrigin)
                || (self.selection.scope == Scope::System
                    && refusal.scope == RefusalScope::OutsideComponent)
            {
                return Err(EssAdmissionError::new(
                    "RefusalScopeMismatch",
                    "$.coverage.refused",
                    "scope contradicts declared origins or system boundary",
                ));
            }
        }
        Ok(())
    }
}

/// Full suite inventory, whose structural ownership is checked by the optional reader.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    /// Complete declared selection.
    pub selection: Selection,
    /// Inventory knowledge.
    pub knowledge: Knowledge,
    /// Sorted selected generated IDs.
    pub generated: Vec<ScenarioId>,
    /// Sorted selected authored IDs.
    pub authored: Vec<ScenarioId>,
    /// Every known outside executable scenario.
    pub outside: Vec<Outside>,
    /// Full original refusal occurrences.
    pub refused: Vec<Refusal>,
    /// Final source ownership and original source digests.
    pub authored_sources: BTreeMap<SourceIdentity, AuthoredSource>,
    /// Exact inventory list lengths.
    pub counts: CoverageCounts,
}
impl Inventory {
    /// The report-visible summary, without discarding any refusal occurrence.
    pub fn summary(&self) -> CoverageSummary {
        CoverageSummary {
            selection: self.selection.clone(),
            knowledge: self.knowledge,
            counts: self.counts.clone(),
            refused: self.refused.clone(),
        }
    }
}
