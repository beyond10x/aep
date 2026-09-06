//! Count-stage ESS evidence: immutable source bytes and explicitly admitted diagnostics.
//!
//! The reader port is a trusted Rust integration boundary. Deserialization is deliberately raw;
//! it cannot restore an admission result. Legacy suites provide no complete-coverage claim.

use std::collections::BTreeSet;
use std::fmt;

use crate::artifact::ArtifactRef;
use crate::evidence::SpecDigest;
use crate::facts::{FactPath, FactValue};
use crate::time::{Granularity, ObservedAt, Timestamp};

/// A stable admission diagnostic, with the original input location and cause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EssAdmissionIssue {
    /// Stable machine-readable reason.
    pub reason: &'static str,
    /// JSON-style location within the paired sources or envelope.
    pub path: String,
    /// Actionable original cause.
    pub detail: String,
}

/// Independent defects found while admitting a source pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EssAdmissionError {
    /// Nonempty diagnostics in deterministic encounter order.
    pub issues: Vec<EssAdmissionIssue>,
}

impl EssAdmissionError {
    /// One refusal. Readers may accumulate independent issues before returning.
    pub fn new(reason: &'static str, path: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            issues: vec![EssAdmissionIssue {
                reason,
                path: path.into(),
                detail: detail.into(),
            }],
        }
    }
}

impl fmt::Display for EssAdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, issue) in self.issues.iter().enumerate() {
            if index != 0 {
                f.write_str("; ")?;
            }
            write!(f, "{} at {}: {}", issue.reason, issue.path, issue.detail)?;
        }
        Ok(())
    }
}
impl std::error::Error for EssAdmissionError {}

/// The frozen suite/1–4 scenario identity grammar; it is not an inventory claim.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct ScenarioId(String);

/// Whether a name follows ESS's dotted ASCII identifier grammar.
pub fn qualified_name(value: &str) -> bool {
    value.split('.').all(|part| {
        part.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
            && part
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    })
}

/// Whether a name follows the frozen lower-kebab slot/binding grammar.
pub fn lower_kebab(value: &str) -> bool {
    value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && value
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        && !value.ends_with('-')
        && !value.contains("--")
}

impl ScenarioId {
    /// Parses a scenario identity without inventing selected coverage.
    pub fn new(value: impl Into<String>) -> Result<Self, EssAdmissionError> {
        let value = value.into();
        let parts: Vec<_> = value.split('/').collect();
        let local = |name: &str| !name.contains('.') && qualified_name(name);
        let valid = match parts.as_slice() {
            [command, "outcome", outcome] => qualified_name(command) && lower_kebab(outcome),
            [entity, "transition", transition, "by", command, outcome] => {
                qualified_name(entity)
                    && local(transition)
                    && qualified_name(command)
                    && lower_kebab(outcome)
            }
            [entity, "state", state, "refuses" | "accepts", command] => {
                qualified_name(entity)
                    && state.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
                    && state.bytes().all(|c| c.is_ascii_alphanumeric())
                    && qualified_name(command)
            }
            [entity, "invariant", "after", command, outcome] => {
                qualified_name(entity) && qualified_name(command) && lower_kebab(outcome)
            }
            [value, "invariant", "at", view, field] => {
                qualified_name(value) && qualified_name(view) && !field.is_empty()
            }
            [binding, "binding", "flow" | "mapping" | "delivery" | "on-failure"] => {
                lower_kebab(binding)
            }
            [domain, "authored", name] => qualified_name(domain) && lower_kebab(name),
            _ => false,
        };
        if valid {
            Ok(Self(value))
        } else {
            Err(EssAdmissionError::new("MalformedScenarioId", "$", value))
        }
    }
    /// Original canonical identity spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<'de> serde::Deserialize<'de> for ScenarioId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::new(<String as serde::Deserialize>::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawSuiteReference {
    version: String,
    digest_profile: String,
    digest: String,
}

/// Exact original-suite identity; only the frozen count-stage profile is admitted.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SuiteReference {
    version: String,
    digest_profile: String,
    digest: String,
}

impl SuiteReference {
    /// Checks the version/profile and canonical byte-digest spelling.
    pub fn new(
        version: String,
        digest_profile: String,
        digest: String,
    ) -> Result<Self, EssAdmissionError> {
        if !matches!(
            version.as_str(),
            "ess-conformance/1" | "ess-conformance/2" | "ess-conformance/3" | "ess-conformance/4"
        ) {
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
        if !digest.strip_prefix("sha256:").is_some_and(|hex| {
            hex.len() == 64
                && hex
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        }) {
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
    /// Suite wire marker.
    pub fn version(&self) -> &str {
        &self.version
    }
    /// Original-byte digest profile.
    pub fn digest_profile(&self) -> &str {
        &self.digest_profile
    }
    /// Prefixed lowercase SHA-256.
    pub fn digest(&self) -> &str {
        &self.digest
    }
}
impl<'de> serde::Deserialize<'de> for SuiteReference {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = RawSuiteReference::deserialize(d)?;
        Self::new(raw.version, raw.digest_profile, raw.digest).map_err(serde::de::Error::custom)
    }
}

/// Exact final scenario categories. These values never pass through floating point facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioCounts {
    /// Total selected scenarios.
    pub total: u64,
    /// Passed final scenarios.
    pub passed: u64,
    /// Failed final scenarios.
    pub failed: u64,
    /// Rust error final scenarios.
    pub error: u64,
    /// Rust unsupported final scenarios.
    pub unsupported: u64,
    /// Go skipped final scenarios.
    pub skipped: u64,
}
impl ScenarioCounts {
    /// Stable category order for counts and outcome lists.
    pub fn categories(&self) -> [(&'static str, u64); 5] {
        [
            ("passed", self.passed),
            ("failed", self.failed),
            ("error", self.error),
            ("unsupported", self.unsupported),
            ("skipped", self.skipped),
        ]
    }
}

/// Closed whole-run execution/conformance status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountStatus {
    /// Every final scenario passed, or the selection was empty.
    Passed,
    /// Observed failure according to the declared producer profile.
    Failed,
    /// The run or coverage could not decide conformance.
    Inconclusive,
}
impl CountStatus {
    /// Stable wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Inconclusive => "inconclusive",
        }
    }
}

/// Producer aggregation policy, independent of complete conformance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProducerProfile {
    /// Rust passed/failed/error/unsupported final categories.
    Rust,
    /// Go passed/failed/skipped final categories.
    Go,
}
impl ProducerProfile {
    /// Exact versioned wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust-scenario-status/1",
            Self::Go => "go-scenario-status/1",
        }
    }
}

/// Inputs to the checked domain reading constructor. Wire/hash checks belong to the reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingInput {
    /// Specification source label.
    pub specification: String,
    /// Original model digest.
    pub spec_digest: SpecDigest,
    /// Implementation source label.
    pub implementation: String,
    /// Admitted original-suite identity.
    pub suite: SuiteReference,
    /// Exact terminal categories.
    pub counts: ScenarioCounts,
    /// Ordered lists in passed, failed, error, unsupported, skipped order.
    pub outcomes: [Vec<ScenarioId>; 5],
    /// Bound profile.
    pub producer_profile: ProducerProfile,
    /// Claimed whole-run status, recomputed by construction.
    pub execution_status: CountStatus,
    /// Claimed conformance status, recomputed with unknown coverage.
    pub conformance_status: CountStatus,
    /// Original completion instant.
    pub completed_at: Timestamp,
}

/// Checked count diagnostics. Deliberately neither deserializable nor a complete-coverage token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EssConformanceV2Reading {
    input: ReadingInput,
    selected_ids: Vec<ScenarioId>,
}
impl EssConformanceV2Reading {
    /// Checks exact arithmetic, category membership, producer aggregation and unknown coverage.
    pub fn new(input: ReadingInput) -> Result<Self, EssAdmissionError> {
        if input.spec_digest.as_str().len() != 64 {
            return Err(EssAdmissionError::new(
                "MalformedModelDigest",
                "$.spec_digest",
                "report/2 requires the full 64-hex model digest",
            ));
        }
        let mut issues = Vec::new();
        let mut issue = |reason, path: String, detail: String| {
            issues.push(EssAdmissionIssue {
                reason,
                path,
                detail,
            });
        };
        let total = input
            .counts
            .categories()
            .into_iter()
            .try_fold(0_u64, |sum, (_, count)| sum.checked_add(count));
        match total {
            None => issue(
                "CountOverflow",
                "$.counts".into(),
                "category sum overflows u64".into(),
            ),
            Some(total) if total != input.counts.total => issue(
                "TotalMismatch",
                "$.counts.total".into(),
                format!("declared {}, categories {total}", input.counts.total),
            ),
            _ => {}
        }
        let mut selected = BTreeSet::new();
        for ((name, count), ids) in input.counts.categories().into_iter().zip(&input.outcomes) {
            if u64::try_from(ids.len()).ok() != Some(count) {
                issue(
                    "OutcomeLengthMismatch",
                    format!("$.outcomes.{name}"),
                    format!("{count} claimed, {} identities", ids.len()),
                );
            }
            if ids.windows(2).any(|pair| pair[0] >= pair[1]) {
                issue(
                    "OutcomeOrder",
                    format!("$.outcomes.{name}"),
                    "IDs must be sorted and distinct".into(),
                );
            }
            for id in ids {
                if !selected.insert(id.clone()) {
                    issue(
                        "DuplicateOutcome",
                        format!("$.outcomes.{name}"),
                        id.as_str().into(),
                    );
                }
            }
        }
        let expected = Self::profile_status(&input, &mut issue);
        if input.execution_status != expected {
            issue(
                "ExecutionStatusMismatch",
                "$.execution_status".into(),
                format!("expected {}", expected.as_str()),
            );
        }
        let conformance = if expected == CountStatus::Failed {
            CountStatus::Failed
        } else {
            CountStatus::Inconclusive
        };
        if input.conformance_status != conformance {
            issue(
                "ConformanceStatusMismatch",
                "$.conformance_status".into(),
                format!("unknown coverage requires {}", conformance.as_str()),
            );
        }
        if !issues.is_empty() {
            return Err(EssAdmissionError { issues });
        }
        Ok(Self {
            input,
            selected_ids: selected.into_iter().collect(),
        })
    }
    fn profile_status(
        input: &ReadingInput,
        issue: &mut impl FnMut(&'static str, String, String),
    ) -> CountStatus {
        let c = &input.counts;
        match input.producer_profile {
            ProducerProfile::Rust => {
                if c.skipped != 0 {
                    issue(
                        "ProfileOutcomeMismatch",
                        "$.counts.skipped".into(),
                        "Rust cannot emit skipped".into(),
                    );
                }
                if c.failed > 0 || c.unsupported > 0 {
                    CountStatus::Failed
                } else if c.error > 0 {
                    CountStatus::Inconclusive
                } else {
                    CountStatus::Passed
                }
            }
            ProducerProfile::Go => {
                if c.error != 0 || c.unsupported != 0 {
                    issue(
                        "ProfileOutcomeMismatch",
                        "$.counts".into(),
                        "Go cannot emit error or unsupported".into(),
                    );
                }
                if c.failed > 0 {
                    CountStatus::Failed
                } else if c.skipped > 0 {
                    CountStatus::Inconclusive
                } else {
                    CountStatus::Passed
                }
            }
        }
    }
    /// Checked diagnostics, exposed immutably.
    pub fn data(&self) -> &ReadingInput {
        &self.input
    }
    /// Exact sorted union of final outcome identities.
    pub fn selected_ids(&self) -> &[ScenarioId] {
        &self.selected_ids
    }
    /// Truthful diagnostics in the new namespace; decimal counts/times remain Text.
    pub fn facts(&self) -> Vec<(FactPath, FactValue)> {
        let base = FactPath::from_segments(["ess_conformance_v2"]);
        let d = &self.input;
        let mut facts = Vec::new();
        for (name, count) in [("total", d.counts.total)]
            .into_iter()
            .chain(d.counts.categories())
        {
            facts.push((
                base.child("counts").child(name),
                FactValue::text(count.to_string()),
            ));
        }
        for (name, value) in [
            ("completed_at", d.completed_at.epoch_millis().to_string()),
            ("execution_status", d.execution_status.as_str().into()),
            ("conformance_status", d.conformance_status.as_str().into()),
            ("producer_profile", d.producer_profile.as_str().into()),
            ("policy", "complete-selection/1".into()),
            ("spec_digest", d.spec_digest.as_str().into()),
        ] {
            facts.push((base.child(name), FactValue::text(value)));
        }
        for (name, value) in [
            ("version", d.suite.version()),
            ("digest_profile", d.suite.digest_profile()),
            ("digest", d.suite.digest()),
        ] {
            facts.push((base.child("suite").child(name), FactValue::text(value)));
        }
        for (name, value) in [
            ("nonempty", d.counts.total > 0),
            ("failed_zero", d.counts.failed == 0),
            ("coverage_known", false),
            ("passed", false),
        ] {
            facts.push((base.child(name), FactValue::bool(value)));
        }
        facts
    }
}

/// Pure optional adapter port. Installed Rust implementations are trusted integration code.
pub trait EssConformanceV2Reader: fmt::Debug + Send + Sync {
    /// Re-read the exact original bytes. Prior cached readings never authorize admission.
    fn read(
        &self,
        sources: &EssConformanceV2Sources,
    ) -> Result<EssConformanceV2Reading, EssAdmissionError>;
}

/// Original UTF-8 documents. Serde retains bytes and never supplies an admitted reading.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct EssConformanceV2Sources {
    report_json: String,
    suite_json: String,
    #[serde(skip)]
    #[schemars(skip)]
    reading: Option<EssConformanceV2Reading>,
}
impl EssConformanceV2Sources {
    /// Constructs raw archival input with no admission/facts claim.
    pub fn new(report_json: String, suite_json: String) -> Self {
        Self {
            report_json,
            suite_json,
            reading: None,
        }
    }
    /// Exact report bytes as UTF-8.
    pub fn report_json(&self) -> &str {
        &self.report_json
    }
    /// Exact suite bytes as UTF-8.
    pub fn suite_json(&self) -> &str {
        &self.suite_json
    }
    /// An in-process reading, absent after every deserialize/source replacement.
    pub fn reading(&self) -> Option<&EssConformanceV2Reading> {
        self.reading.as_ref()
    }
    /// Re-admits the immutable pair, clearing any previous reading before a fallible call.
    pub fn admit(&mut self, reader: &dyn EssConformanceV2Reader) -> Result<(), EssAdmissionError> {
        self.reading = None;
        self.reading = Some(reader.read(self)?);
        Ok(())
    }
    /// Checks the record's envelope without substituting submission time for observation time.
    pub fn check_observation(&self, observed_at: ObservedAt) -> Result<(), EssAdmissionError> {
        let reading = self.reading.as_ref().ok_or_else(|| {
            EssAdmissionError::new("MissingAdmission", "$", "install a source reader")
        })?;
        if observed_at.written_as() != Granularity::Instant {
            return Err(EssAdmissionError::new(
                "InstantRequired",
                "$.observed_at",
                "a calendar day is not the original completion instant",
            ));
        }
        if observed_at.timestamp() != reading.data().completed_at {
            return Err(EssAdmissionError::new(
                "ObservationMismatch",
                "$.observed_at",
                format!("expected {}", reading.data().completed_at.epoch_millis()),
            ));
        }
        Ok(())
    }
}

/// Independently authored task expectation. A report must never populate this value itself.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EssConformanceV2Expectation {
    model: ArtifactRef,
    suite: SuiteReference,
    selected_ids: Vec<ScenarioId>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawExpectation {
    model: ArtifactRef,
    suite: SuiteReference,
    selected_ids: Vec<ScenarioId>,
}
impl EssConformanceV2Expectation {
    /// Checks deterministic exact selection order. Named-model resolution belongs to qualification.
    pub fn new(
        model: ArtifactRef,
        suite: SuiteReference,
        selected_ids: Vec<ScenarioId>,
    ) -> Result<Self, EssAdmissionError> {
        if selected_ids.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(EssAdmissionError::new(
                "SelectionOrder",
                "$.selected_ids",
                "IDs must be sorted and distinct",
            ));
        }
        Ok(Self {
            model,
            suite,
            selected_ids,
        })
    }
    /// Exactly the artifact the caller chose.
    pub fn model(&self) -> &ArtifactRef {
        &self.model
    }
    /// Exact independently selected suite.
    pub fn suite(&self) -> &SuiteReference {
        &self.suite
    }
    /// Exact independently selected IDs.
    pub fn selected_ids(&self) -> &[ScenarioId] {
        &self.selected_ids
    }
}
impl<'de> serde::Deserialize<'de> for EssConformanceV2Expectation {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = RawExpectation::deserialize(d)?;
        Self::new(raw.model, raw.suite, raw.selected_ids).map_err(serde::de::Error::custom)
    }
}
