//! Original ESS coverage sources, separate from the frozen count-stage carrier.

pub(crate) mod qualification;
mod values;
pub use values::{
    ordered_ids, original_digest, validate_needs, AuthoredSource, CoverageCounts, CoverageSummary,
    Filter, Inventory, Knowledge, Origin, Origins, OutcomeReference, Outside, OutsideReason,
    Refusal, RefusalEffect, RefusalScope, Retained, Scope, Selection, SemanticReference,
    SourceDisposition, SourceIdentity, SuiteReference, TransitionReference,
};

use crate::artifact::ArtifactRef;
use crate::ess_conformance_v2::{
    CountStatus, EssAdmissionError, EssAdmissionIssue, EssConformanceV2Reader, ProducerProfile,
    ScenarioCounts, ScenarioId,
};
use crate::evidence::SpecDigest;
use crate::facts::{FactPath, FactValue};
use crate::time::{Granularity, ObservedAt, Timestamp};
use std::collections::BTreeSet;
use std::sync::Arc;

/// Concrete optional ports. Configuring either leaves the other independently available.
#[derive(Debug, Clone, Default)]
pub struct EssEvidenceReaders {
    /// Frozen count-stage reader for report/2 paired with suite/1–4.
    pub count: Option<Arc<dyn EssConformanceV2Reader>>,
    /// Separate complete-selection reader for report/2 paired with input/1 and suite/5.
    pub coverage: Option<Arc<dyn EssConformanceCoverageReader>>,
}

/// Inputs to checked coverage construction. Byte and inventory transcription belong to the reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingInput {
    /// Original specification label.
    pub specification: String,
    /// Exact current model digest named by the producer.
    pub spec_digest: SpecDigest,
    /// Original implementation label.
    pub implementation: String,
    /// Exact original selected suite.
    pub suite: SuiteReference,
    /// Full admitted coverage summary.
    pub coverage: CoverageSummary,
    /// Exact terminal counts.
    pub counts: ScenarioCounts,
    /// Sorted distinct IDs in passed, failed, error, unsupported, skipped order.
    pub outcomes: [Vec<ScenarioId>; 5],
    /// Explicit frozen producer profile.
    pub producer_profile: ProducerProfile,
    /// Claimed status, recomputed by construction.
    pub execution_status: CountStatus,
    /// Claimed complete-selection/1 status, recomputed by construction.
    pub conformance_status: CountStatus,
    /// Exact unsigned original report completion time.
    pub completed_at: Timestamp,
}

/// Checked coverage diagnostics; documents cannot deserialize or mutate this capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EssConformanceCoverageReading {
    input: ReadingInput,
    selected_ids: Vec<ScenarioId>,
}
impl EssConformanceCoverageReading {
    /// Checks exact counts, terminal membership, selection and complete-selection/1 aggregation.
    pub fn new(input: ReadingInput) -> Result<Self, EssAdmissionError> {
        if input.spec_digest.as_str().len() != 64 {
            return Err(EssAdmissionError::new(
                "MalformedModelDigest",
                "$.spec_digest",
                "report/2 requires the full model digest",
            ));
        }
        let selected_ids = checked_outcomes(&input)?;
        input.coverage.validate(&selected_ids)?;
        let expected = execution_status(&input.counts, input.producer_profile)?;
        if input.execution_status != expected {
            return Err(EssAdmissionError::new(
                "ExecutionStatusMismatch",
                "$.execution_status",
                format!("expected {}", expected.as_str()),
            ));
        }
        let conformance = match expected {
            CountStatus::Failed => CountStatus::Failed,
            CountStatus::Inconclusive => CountStatus::Inconclusive,
            CountStatus::Passed if input.counts.total == 0 || !input.coverage.complete() => {
                CountStatus::Inconclusive
            }
            CountStatus::Passed => CountStatus::Passed,
        };
        if input.conformance_status != conformance {
            return Err(EssAdmissionError::new(
                "ConformanceStatusMismatch",
                "$.conformance_status",
                format!("expected {}", conformance.as_str()),
            ));
        }
        Ok(Self {
            input,
            selected_ids,
        })
    }
    /// Full checked immutable reading.
    pub fn data(&self) -> &ReadingInput {
        &self.input
    }
    /// Exact sorted terminal outcome union.
    pub fn selected_ids(&self) -> &[ScenarioId] {
        &self.selected_ids
    }
    /// Exhaustive task-independent diagnostics; counts and time remain exact decimal Text.
    pub fn facts(&self) -> Vec<(FactPath, FactValue)> {
        let base = FactPath::from_segments(["ess_conformance_coverage_v1"]);
        let d = &self.input;
        let coverage = &d.coverage;
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
        for (name, count) in [
            ("generated", coverage.counts.generated),
            ("authored", coverage.counts.authored),
            ("outside", coverage.counts.outside),
            ("refused", coverage.counts.refused),
        ] {
            facts.push((
                base.child("coverage").child("counts").child(name),
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
        reference_facts(&mut facts, &base.child("suite"), &d.suite);
        facts.push((
            base.child("coverage").child("knowledge"),
            FactValue::text(coverage.knowledge.as_str()),
        ));
        let selection = base.child("selection");
        let scope = selection.child("scope");
        let kind = match &coverage.selection.scope {
            Scope::System => "system",
            Scope::Component { component } => {
                facts.push((scope.child("component"), FactValue::text(component)));
                "component"
            }
        };
        facts.push((scope.child("kind"), FactValue::text(kind)));
        facts.push((
            selection.child("origins"),
            FactValue::text(coverage.selection.origins.as_str()),
        ));
        let filter = selection.child("filter");
        let kind = match &coverage.selection.filter {
            Filter::All => "all",
            Filter::Explicit { parent, .. } => {
                reference_facts(&mut facts, &filter.child("parent"), parent);
                "explicit"
            }
        };
        facts.push((filter.child("kind"), FactValue::text(kind)));
        for (name, value) in [
            ("nonempty", d.counts.total > 0),
            ("failed_zero", d.counts.failed == 0),
            (
                "coverage_known",
                coverage.knowledge == Knowledge::CompleteInventory,
            ),
            ("coverage_complete", coverage.complete()),
        ] {
            facts.push((base.child(name), FactValue::bool(value)));
        }
        facts
    }
}

fn reference_facts(
    facts: &mut Vec<(FactPath, FactValue)>,
    path: &FactPath,
    reference: &SuiteReference,
) {
    for (name, value) in [
        ("version", reference.version()),
        ("digest_profile", reference.digest_profile()),
        ("digest", reference.digest()),
    ] {
        facts.push((path.child(name), FactValue::text(value)));
    }
}
fn checked_outcomes(input: &ReadingInput) -> Result<Vec<ScenarioId>, EssAdmissionError> {
    let mut issues = Vec::new();
    let mut issue = |reason, path: String, detail: String| {
        issues.push(EssAdmissionIssue {
            reason,
            path,
            detail,
        });
    };
    match input
        .counts
        .categories()
        .into_iter()
        .try_fold(0_u64, |sum, (_, count)| sum.checked_add(count))
    {
        None => issue(
            "CountOverflow",
            "$.counts".into(),
            "category sum overflows u64".into(),
        ),
        Some(total) if total != input.counts.total => issue(
            "TotalMismatch",
            "$.counts.total".into(),
            format!("expected {total}"),
        ),
        _ => {}
    }
    let mut selected = BTreeSet::new();
    for ((name, count), ids) in input.counts.categories().into_iter().zip(&input.outcomes) {
        if u64::try_from(ids.len()).ok() != Some(count) {
            issue(
                "OutcomeLengthMismatch",
                format!("$.outcomes.{name}"),
                format!("{count} claimed, {} IDs", ids.len()),
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
    if issues.is_empty() {
        Ok(selected.into_iter().collect())
    } else {
        Err(EssAdmissionError { issues })
    }
}
fn execution_status(
    counts: &ScenarioCounts,
    profile: ProducerProfile,
) -> Result<CountStatus, EssAdmissionError> {
    match profile {
        ProducerProfile::Rust => {
            if counts.skipped != 0 {
                return Err(EssAdmissionError::new(
                    "ProfileOutcomeMismatch",
                    "$.counts.skipped",
                    "Rust cannot emit skipped",
                ));
            }
            Ok(if counts.failed > 0 || counts.unsupported > 0 {
                CountStatus::Failed
            } else if counts.error > 0 {
                CountStatus::Inconclusive
            } else {
                CountStatus::Passed
            })
        }
        ProducerProfile::Go => {
            if counts.error != 0 || counts.unsupported != 0 {
                return Err(EssAdmissionError::new(
                    "ProfileOutcomeMismatch",
                    "$.counts",
                    "Go cannot emit error or unsupported",
                ));
            }
            Ok(if counts.failed > 0 {
                CountStatus::Failed
            } else if counts.skipped > 0 {
                CountStatus::Inconclusive
            } else {
                CountStatus::Passed
            })
        }
    }
}

/// Optional pure reader; installed Rust implementations are explicitly trusted integration code.
pub trait EssConformanceCoverageReader: std::fmt::Debug + Send + Sync {
    /// Re-admits the exact report, selected suite and complete original parent chain.
    fn read(
        &self,
        sources: &EssConformanceCoverageSources,
    ) -> Result<EssConformanceCoverageReading, EssAdmissionError>;
}

/// Original UTF-8 documents; raw construction and deserialization confer no admission.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct EssConformanceCoverageSources {
    report_json: String,
    suite_input_json: String,
    #[serde(skip)]
    #[schemars(skip)]
    reading: Option<EssConformanceCoverageReading>,
}

impl EssConformanceCoverageSources {
    /// Retains archival source bytes without interpreting their claims.
    pub fn new(report_json: String, suite_input_json: String) -> Self {
        Self {
            report_json,
            suite_input_json,
            reading: None,
        }
    }

    /// Exact original report JSON.
    pub fn report_json(&self) -> &str {
        &self.report_json
    }

    /// Exact original input carrier JSON, including its inner suite and parent strings.
    pub fn suite_input_json(&self) -> &str {
        &self.suite_input_json
    }

    /// In-process reading; raw construction and serde have none.
    pub fn reading(&self) -> Option<&EssConformanceCoverageReading> {
        self.reading.as_ref()
    }

    /// Clears stale admission before calling the fallible reader on the original bytes.
    pub fn admit(
        &mut self,
        reader: &dyn EssConformanceCoverageReader,
    ) -> Result<(), EssAdmissionError> {
        self.reading = None;
        self.reading = Some(reader.read(self)?);
        Ok(())
    }

    /// Checks exact observation equality without replacing the original completion time.
    pub fn check_observation(&self, observed_at: ObservedAt) -> Result<(), EssAdmissionError> {
        let reading = self.reading.as_ref().ok_or_else(|| {
            EssAdmissionError::new("MissingAdmission", "$", "install a coverage reader")
        })?;
        if observed_at.written_as() != Granularity::Instant {
            return Err(EssAdmissionError::new(
                "InstantRequired",
                "$.observed_at",
                "a day is not the original completion instant",
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

/// Independently chosen model, original suite and complete declared selection.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EssConformanceCoverageExpectation {
    model: ArtifactRef,
    suite: SuiteReference,
    selection: Selection,
    selected_ids: Vec<ScenarioId>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawExpectation {
    model: ArtifactRef,
    suite: SuiteReference,
    selection: Selection,
    selected_ids: Vec<ScenarioId>,
}
impl EssConformanceCoverageExpectation {
    /// Checks typed selection/order; named-model resolution belongs to qualification.
    pub fn new(
        model: ArtifactRef,
        suite: SuiteReference,
        selection: Selection,
        selected_ids: Vec<ScenarioId>,
    ) -> Result<Self, EssAdmissionError> {
        selection.validate()?;
        ordered_ids(&selected_ids, "$.selected_ids")?;
        if let Filter::Explicit { ids, .. } = &selection.filter {
            if ids != &selected_ids {
                return Err(EssAdmissionError::new(
                    "SelectedIdsMismatch",
                    "$.selection.filter.ids",
                    "expected selected IDs must equal explicit filter IDs",
                ));
            }
        }
        Ok(Self {
            model,
            suite,
            selection,
            selected_ids,
        })
    }
    /// Exactly the caller's chosen model artifact.
    pub fn model(&self) -> &ArtifactRef {
        &self.model
    }
    /// Exact independently approved suite bytes.
    pub fn suite(&self) -> &SuiteReference {
        &self.suite
    }
    /// Full independently expected selection.
    pub fn selection(&self) -> &Selection {
        &self.selection
    }
    /// Exact independently expected scenario identities.
    pub fn selected_ids(&self) -> &[ScenarioId] {
        &self.selected_ids
    }
}
impl<'de> serde::Deserialize<'de> for EssConformanceCoverageExpectation {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = RawExpectation::deserialize(d)?;
        Self::new(raw.model, raw.suite, raw.selection, raw.selected_ids)
            .map_err(serde::de::Error::custom)
    }
}
