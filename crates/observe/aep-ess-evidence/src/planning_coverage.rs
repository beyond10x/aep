//! Re-admitted coverage originals for the narrow specification lifecycle bridge.
use crate::count_json::{Json, Result};
use aep_domain::ess_conformance_coverage::{
    EssConformanceCoverageSources, Filter, Knowledge, Origins, Scope,
};
use aep_domain::ess_conformance_v2::{CountStatus, EssAdmissionError};
use aep_domain::time::Timestamp;
use aep_domain::SpecDigest;

/// Retains exact originals beside the descriptive planning source fields.
///
/// # Errors
/// Refuses sources without an in-process admission and unknown input transports.
pub fn planning_coverage_source(
    sources: &EssConformanceCoverageSources,
    report_input: &str,
    suite_input: &str,
    input_transport: &str,
) -> Result<String> {
    if !matches!(input_transport, "original_input" | "wrapped_raw_suite") {
        return Err(refusal(
            "UnknownInputTransport",
            "unsupported source transport",
        ));
    }
    let reading = sources.reading().ok_or_else(|| {
        refusal(
            "MissingAdmission",
            "original sources have not been admitted",
        )
    })?;
    let data = reading.data();
    Ok(serde_json::json!({
        "format":"ess-conformance-report/2", "report_input":report_input, "suite_input":suite_input, "input_transport":input_transport,
        "specification":data.specification, "implementation":data.implementation, "spec_digest":data.spec_digest,
        "producer_profile":data.producer_profile.as_str(), "suite":data.suite, "selection":data.coverage.selection,
        "execution_status":data.execution_status.as_str(), "conformance_status":data.conformance_status.as_str(),
        "coverage":data.coverage, "policy":"complete-selection/1", "selected_ids":reading.selected_ids(),
        "counts":{"total":data.counts.total,"passed":data.counts.passed,"failed":data.counts.failed,"error":data.counts.error,"unsupported":data.counts.unsupported,"skipped":data.counts.skipped},
        "completed_at":data.completed_at.epoch_millis().to_string(), "originals":sources
    }).to_string())
}

fn refusal(reason: &'static str, detail: &str) -> EssAdmissionError {
    EssAdmissionError::new(reason, "$planning.coverage", detail)
}

/// Re-admits exact originals from a closed planning source and checks its descriptive fields.
///
/// # Errors
/// Refuses summary-only sources, malformed originals and altered descriptive fields.
pub fn read_planning_coverage_source(source: &str) -> Result<EssConformanceCoverageSources> {
    let value = Json::parse(source, "$planning.coverage")?;
    let fields = value.closed(
        &[
            "format",
            "report_input",
            "suite_input",
            "input_transport",
            "specification",
            "implementation",
            "spec_digest",
            "producer_profile",
            "suite",
            "selection",
            "execution_status",
            "conformance_status",
            "coverage",
            "policy",
            "selected_ids",
            "counts",
            "completed_at",
            "originals",
        ],
        &[],
    )?;
    let originals = fields["originals"].closed(&["report_json", "suite_input_json"], &[])?;
    let mut sources = EssConformanceCoverageSources::new(
        originals["report_json"].text()?.into(),
        originals["suite_input_json"].text()?.into(),
    );
    sources.admit(&crate::CoverageReader)?;
    let canonical = planning_coverage_source(
        &sources,
        fields["report_input"].text()?,
        fields["suite_input"].text()?,
        fields["input_transport"].text()?,
    )?;
    if value.exact() != Json::parse(&canonical, "$planning.coverage")?.exact() {
        return Err(refusal(
            "CoverageSourceMismatch",
            "descriptive fields differ from the admitted originals",
        ));
    }
    Ok(sources)
}

/// Qualifies retained originals against the current specification and observation instant.
///
/// This is a planning lifecycle eligibility check, not task-engine principle evaluation. It
/// requires the entire system's complete generated-and-authored selection. It returns no evidence
/// record and confers no authority on descriptive source text or a cached admission flag.
///
/// # Errors
/// Refuses malformed or summary-only sources, stale models, incomplete selections, nonpassing
/// outcomes, and observation times different from the report's planning projection or in the future.
pub fn qualify_planning_coverage(
    source: &str,
    current_model: Option<&SpecDigest>,
    recorded_at: &str,
    now: Timestamp,
) -> Result<()> {
    let sources = read_planning_coverage_source(source)?;
    let data = sources.reading().expect("admitted originals").data();
    let model = current_model.ok_or_else(|| {
        refusal(
            "MissingModelDigest",
            "the specification has no current model digest",
        )
    })?;
    if model != &data.spec_digest {
        return Err(refusal(
            "ModelDigestMismatch",
            "the current specification differs from the admitted run",
        ));
    }
    if recorded_at != data.completed_at.iso_8601() {
        return Err(refusal(
            "ObservationMismatch",
            "recorded instant differs from the report's existing planning ISO projection",
        ));
    }
    if data.completed_at > now {
        return Err(refusal(
            "FutureObservation",
            "the report completes after the move's evaluation time",
        ));
    }
    if data.coverage.knowledge != Knowledge::CompleteInventory {
        return Err(refusal(
            "UnknownCoverage",
            "complete declared inventory is required",
        ));
    }
    if data.coverage.selection.scope != Scope::System
        || data.coverage.selection.origins != Origins::GeneratedAndAuthored
        || data.coverage.selection.filter != Filter::All
        || data.coverage.counts.outside != 0
    {
        return Err(refusal(
            "PartialSelection",
            "conforming requires the entire generated-and-authored system selection",
        ));
    }
    if data.coverage.counts.refused != 0 || !data.coverage.refused.is_empty() {
        return Err(refusal("SynthesisRefusal", "an obligation was not emitted"));
    }
    if data.counts.total == 0 {
        return Err(refusal(
            "EmptySelection",
            "an empty suite cannot establish conformance",
        ));
    }
    if data.execution_status != CountStatus::Passed
        || data.conformance_status != CountStatus::Passed
        || data.counts.passed != data.counts.total
        || data.counts.failed != 0
        || data.counts.error != 0
        || data.counts.unsupported != 0
        || data.counts.skipped != 0
    {
        return Err(refusal(
            "NonPassingCoverage",
            "every selected scenario must have passed",
        ));
    }
    Ok(())
}
