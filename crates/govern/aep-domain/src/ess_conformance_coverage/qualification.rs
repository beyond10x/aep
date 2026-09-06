//! One coverage decision over one record and independent task context.
use super::{
    EssConformanceCoverageReading, EssConformanceCoverageSources, Knowledge, RefusalScope,
};
use crate::artifact::ArtifactKind;
use crate::ess_conformance_v2::{CountStatus, EssAdmissionIssue};
use crate::evidence::{Evidence, EvidenceRecord, Producer};
use crate::requirement::{EvidenceRequirement, RecordQualification, RequirementContext};
use crate::verification::Verifier;

fn unknown(reason: &'static str, path: &str, detail: &str) -> RecordQualification {
    RecordQualification::Unknown(EssAdmissionIssue {
        reason,
        path: path.into(),
        detail: detail.into(),
    })
}
fn contradiction(reason: &'static str, path: &str, detail: &str) -> RecordQualification {
    RecordQualification::Contradiction(EssAdmissionIssue {
        reason,
        path: path.into(),
        detail: detail.into(),
    })
}

pub(crate) fn qualify(
    required: &EvidenceRequirement,
    record: &EvidenceRecord,
    context: &dyn RequirementContext,
) -> RecordQualification {
    if required.at_least == 0 {
        return contradiction(
            "InvalidRequirement",
            "$requirement.at_least",
            "coverage requires at least one qualified record",
        );
    }
    let Evidence::EssConformanceCoverageV1(sources) = &record.value else {
        return unknown("WrongKind", "$.kind", "not coverage evidence");
    };
    let Some(reading) = sources.reading() else {
        return unknown(
            "MissingAdmission",
            "$",
            "raw sources have no admitted reading",
        );
    };
    let Some(subject) = context.task_subject() else {
        return unknown(
            "MissingTaskSubject",
            "$task.subject",
            "the task must independently name its subject",
        );
    };
    if record.subject.as_ref() != Some(subject)
        || required
            .subject
            .as_ref()
            .is_some_and(|expected| record.subject.as_ref() != Some(expected))
    {
        return contradiction(
            "SubjectMismatch",
            "$.subject",
            "record does not name the expected task/requirement subject",
        );
    }
    if !matches!(
        record.producer,
        Producer::Verifier {
            verifier: Verifier::ConformanceRunner
        }
    ) || required
        .verifier
        .as_ref()
        .is_some_and(|verifier| *verifier != Verifier::ConformanceRunner)
    {
        return contradiction(
            "ProducerMismatch",
            "$.producer",
            "an independent conformance-runner is required; provenance.tool supplies no authority",
        );
    }
    if let Some(refusal) = identity(reading, context) {
        return refusal;
    }
    observation(required, sources, record, context)
}

fn identity(
    reading: &EssConformanceCoverageReading,
    context: &dyn RequirementContext,
) -> Option<RecordQualification> {
    let Some(expected) = context.ess_conformance_coverage_expectation() else {
        return Some(unknown(
            "MissingExpectation",
            "$task.constraints.ess_conformance_coverage_v1",
            "author the independent expectation before consuming evidence",
        ));
    };
    let Some(model) = context.artifacts().resolve(expected.model()) else {
        return Some(unknown(
            "MissingModel",
            "$task.constraints.ess_conformance_coverage_v1.model",
            "the named model is absent from the current graph",
        ));
    };
    if model.kind != ArtifactKind::ExecutableSystemSpecification {
        return Some(contradiction(
            "ModelKindMismatch",
            "$task.constraints.ess_conformance_coverage_v1.model",
            "the named artifact must be an executable-system specification",
        ));
    }
    let Some(digest) = &model.model_digest else {
        return Some(unknown(
            "MissingModelDigest",
            "$model.model_digest",
            "the named current model declares no digest",
        ));
    };
    let data = reading.data();
    if digest != &data.spec_digest {
        return Some(contradiction(
            "ModelDigestMismatch",
            "$.spec_digest",
            "report differs from the specifically named current model",
        ));
    }
    if expected.suite() != &data.suite {
        return Some(contradiction(
            "SuiteReferenceMismatch",
            "$.suite",
            "exact suite reference differs from the task expectation",
        ));
    }
    if expected.selection() != &data.coverage.selection
        || expected.selected_ids() != reading.selected_ids()
    {
        return Some(contradiction("SelectionMismatch", "$.coverage.selection", "full scope, origins, filter, parent and selected IDs must match the independent expectation"));
    }
    None
}

fn observation(
    required: &EvidenceRequirement,
    sources: &EssConformanceCoverageSources,
    record: &EvidenceRecord,
    context: &dyn RequirementContext,
) -> RecordQualification {
    if let Err(error) = sources.check_observation(record.observed_at) {
        return contradiction("ObservationMismatch", "$.observed_at", &error.to_string());
    }
    let Some(now) = context.now() else {
        return unknown(
            "MissingTime",
            "$context.now",
            "current evaluation time is required even without a horizon",
        );
    };
    let data = sources.reading().expect("admission checked").data();
    let Some(age) = now
        .epoch_millis()
        .checked_sub(data.completed_at.epoch_millis())
    else {
        return contradiction(
            "FutureObservation",
            "$.observed_at",
            "observation is later than evaluation time",
        );
    };
    if required
        .horizon
        .is_some_and(|horizon| age > horizon.as_millis())
    {
        return unknown(
            "StaleObservation",
            "$.observed_at",
            "observation exceeds the requirement horizon",
        );
    }
    if data.execution_status == CountStatus::Failed {
        return contradiction(
            "ExecutionFailed",
            "$.execution_status",
            "producer observed a failing final scenario",
        );
    }
    if data.counts.total == 0 {
        return unknown(
            "EmptySelection",
            "$.counts.total",
            "an empty selection establishes no conformance",
        );
    }
    if data.coverage.knowledge == Knowledge::Unknown {
        return unknown(
            "UnknownCoverage",
            "$.coverage.knowledge",
            "builder did not establish complete inventory",
        );
    }
    if data
        .coverage
        .refused
        .iter()
        .any(|refusal| refusal.scope == RefusalScope::InScope)
    {
        return unknown(
            "InScopeRefusal",
            "$.coverage.refused",
            "an in-scope obligation remains unobserved",
        );
    }
    if data.execution_status == CountStatus::Inconclusive {
        return unknown(
            "ExecutionInconclusive",
            "$.execution_status",
            "terminal outcomes did not establish all-pass execution",
        );
    }
    RecordQualification::Qualified
}
