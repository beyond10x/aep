//! Original report/2 and input/1 admission for complete ESS selections.
use crate::count_json::{Json, Result};
use aep_domain::ess_conformance_coverage::{
    EssConformanceCoverageReader, EssConformanceCoverageReading, EssConformanceCoverageSources,
    ReadingInput, SuiteReference,
};
use aep_domain::ess_conformance_v2::{
    CountStatus, EssAdmissionError, ProducerProfile, ScenarioCounts, ScenarioId,
};
use aep_domain::time::Timestamp;
use aep_domain::SpecDigest;

/// Optional pure reader for the original suite/5 input chain and its standalone report.
#[derive(Debug, Clone, Copy, Default)]
pub struct CoverageReader;

impl EssConformanceCoverageReader for CoverageReader {
    fn read(
        &self,
        sources: &EssConformanceCoverageSources,
    ) -> Result<EssConformanceCoverageReading> {
        let suite = crate::coverage_suite::admit_input(sources.suite_input_json())?;
        let reading = read_report(sources.report_json())?;
        let data = reading.data();
        if data.suite != suite.reference {
            return Err(EssAdmissionError::new(
                "SuiteDigestMismatch",
                "$.suite",
                "report reference differs from exact selected suite bytes",
            ));
        }
        if data.spec_digest != suite.spec_digest {
            return Err(EssAdmissionError::new(
                "ModelDigestMismatch",
                "$.spec_digest",
                "suite and report name different models",
            ));
        }
        if data.coverage != suite.inventory.summary() {
            return Err(EssAdmissionError::new(
                "CoverageMismatch",
                "$.coverage",
                "report must copy the complete suite coverage summary including all occurrences",
            ));
        }
        if reading.selected_ids() != suite.ids {
            return Err(EssAdmissionError::new(
                "SelectedIdsMismatch",
                "$.outcomes",
                "outcome union must equal the exact selected suite scenario keys",
            ));
        }
        Ok(reading)
    }
}

/// Admits original standalone report/2 and complete input/1 as typed coverage evidence.
///
/// # Errors
/// Refuses malformed or contradictory sources and missing original lineage.
pub fn adapt_json_coverage(
    report_json: &str,
    suite_input_json: &str,
) -> Result<crate::AdaptedEvidence> {
    let mut sources =
        EssConformanceCoverageSources::new(report_json.into(), suite_input_json.into());
    sources.admit(&CoverageReader)?;
    let completed_at = sources
        .reading()
        .expect("successful admission")
        .data()
        .completed_at;
    Ok(crate::AdaptedEvidence {
        evidence: aep_domain::Evidence::EssConformanceCoverageV1(sources),
        observed_at: aep_domain::time::ObservedAt::new(completed_at),
        producer: aep_domain::evidence::Producer::Verifier {
            verifier: aep_domain::verification::Verifier::ConformanceRunner,
        },
        provenance: aep_domain::evidence::Provenance::default(),
    })
}

/// Issues an input/1 carrier around an admitted unfiltered suite/5's exact original bytes.
///
/// # Errors
/// Refuses malformed suites and explicit selections that require original parent documents.
pub fn wrap_coverage_suite(suite_json: &str) -> Result<String> {
    crate::coverage_suite::wrap_suite(suite_json)
}

fn status(value: &Json) -> Result<CountStatus> {
    match value.text()? {
        "passed" => Ok(CountStatus::Passed),
        "failed" => Ok(CountStatus::Failed),
        "inconclusive" => Ok(CountStatus::Inconclusive),
        other => Err(value.error("UnsupportedStatus", other)),
    }
}
fn exact(value: &Json, expected: &str, reason: &'static str) -> Result<()> {
    if value.text()? == expected {
        Ok(())
    } else {
        Err(value.error(reason, format!("expected {expected}")))
    }
}
fn read_report(original: &str) -> Result<EssConformanceCoverageReading> {
    let report = Json::parse(original, "$")?;
    let marker = report
        .object()?
        .get("format")
        .ok_or_else(|| report.error("MissingField", "format"))?;
    exact(
        marker,
        "ess-conformance-report/2",
        "UnsupportedReportVersion",
    )?;
    let r = report.closed(
        &[
            "format",
            "specification",
            "spec_digest",
            "implementation",
            "producer_profile",
            "suite",
            "execution_status",
            "counts",
            "outcomes",
            "coverage",
            "conformance_status",
            "policy",
            "completed_at",
        ],
        &[],
    )?;
    exact(&r["policy"], "complete-selection/1", "UnsupportedPolicy")?;
    let producer_profile = match r["producer_profile"].text()? {
        "rust-scenario-status/1" => ProducerProfile::Rust,
        "go-scenario-status/1" => ProducerProfile::Go,
        other => return Err(r["producer_profile"].error("UnsupportedProducerProfile", other)),
    };
    let s = r["suite"].closed(&["version", "digest_profile", "digest"], &[])?;
    let suite = SuiteReference::new(
        s["version"].text()?.into(),
        s["digest_profile"].text()?.into(),
        s["digest"].text()?.into(),
    )?;
    let c = r["counts"].closed(
        &[
            "total",
            "passed",
            "failed",
            "error",
            "unsupported",
            "skipped",
        ],
        &[],
    )?;
    let o = r["outcomes"].closed(
        &["passed", "failed", "error", "unsupported", "skipped"],
        &[],
    )?;
    let ids = |category: &str| {
        o[category]
            .array()?
            .iter()
            .map(|id| {
                ScenarioId::new(id.text()?.to_owned())
                    .map_err(|error| id.error("MalformedScenarioId", error.to_string()))
            })
            .collect::<Result<Vec<_>>>()
    };
    EssConformanceCoverageReading::new(ReadingInput {
        specification: r["specification"].text()?.into(),
        implementation: r["implementation"].text()?.into(),
        spec_digest: SpecDigest::new(r["spec_digest"].text()?)
            .map_err(|error| r["spec_digest"].error("MalformedModelDigest", error.to_string()))?,
        suite,
        coverage: crate::coverage_wire::summary(&r["coverage"])?,
        counts: ScenarioCounts {
            total: c["total"].unsigned()?,
            passed: c["passed"].unsigned()?,
            failed: c["failed"].unsigned()?,
            error: c["error"].unsigned()?,
            unsupported: c["unsupported"].unsigned()?,
            skipped: c["skipped"].unsigned()?,
        },
        outcomes: [
            ids("passed")?,
            ids("failed")?,
            ids("error")?,
            ids("unsupported")?,
            ids("skipped")?,
        ],
        producer_profile,
        execution_status: status(&r["execution_status"])?,
        conformance_status: status(&r["conformance_status"])?,
        completed_at: Timestamp::from_epoch_millis(r["completed_at"].unsigned()?),
    })
}
