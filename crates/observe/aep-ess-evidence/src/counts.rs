//! Frozen standalone report/2 reader. Full suite vocabulary is checked before byte pairing.
use aep_domain::ess_conformance_v2::{
    CountStatus, EssAdmissionError, EssConformanceV2Reader, EssConformanceV2Reading,
    EssConformanceV2Sources, ProducerProfile, ReadingInput, ScenarioCounts, ScenarioId,
    SuiteReference,
};
use aep_domain::evidence::{Evidence, Producer, Provenance, SpecDigest};
use aep_domain::time::{ObservedAt, Timestamp};
use aep_domain::verification::Verifier;
use sha2::{Digest, Sha256};
use std::fmt::Write;

use crate::count_json::{Json, Result};
use crate::AdaptedEvidence;

/// The optional pure original-byte reader for report/2 and the frozen suite/1–4 vocabulary.
#[derive(Debug, Clone, Copy, Default)]
pub struct CountStageReader;

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

impl EssConformanceV2Reader for CountStageReader {
    fn read(&self, sources: &EssConformanceV2Sources) -> Result<EssConformanceV2Reading> {
        let reading = read_report(sources.report_json())?;
        admit_suite(sources.suite_json(), &reading)?;
        Ok(reading)
    }
}

fn read_report(report_json: &str) -> Result<EssConformanceV2Reading> {
    let report = Json::parse(report_json, "$")?;
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
    let producer_profile = match r["producer_profile"].text()? {
        "rust-scenario-status/1" => ProducerProfile::Rust,
        "go-scenario-status/1" => ProducerProfile::Go,
        other => return Err(r["producer_profile"].error("UnsupportedProducerProfile", other)),
    };
    exact(&r["policy"], "complete-selection/1", "UnsupportedPolicy")?;
    let s = r["suite"].closed(&["version", "digest_profile", "digest"], &[])?;
    let suite = SuiteReference::new(
        s["version"].text()?.into(),
        s["digest_profile"].text()?.into(),
        s["digest"].text()?.into(),
    )?;
    let coverage = r["coverage"].closed(&["knowledge"], &[])?;
    exact(&coverage["knowledge"], "unknown", "UnsupportedCoverage")?;
    let counts = r["counts"].closed(
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
    let outcomes = r["outcomes"].closed(
        &["passed", "failed", "error", "unsupported", "skipped"],
        &[],
    )?;
    let ids = |category: &str| {
        outcomes[category]
            .array()?
            .iter()
            .map(|id| {
                ScenarioId::new(id.text()?.to_owned()).map_err(|mut error| {
                    for issue in &mut error.issues {
                        issue.path.clone_from(&id.path);
                    }
                    error
                })
            })
            .collect::<Result<Vec<_>>>()
    };
    EssConformanceV2Reading::new(ReadingInput {
        specification: r["specification"].text()?.into(),
        spec_digest: SpecDigest::new(r["spec_digest"].text()?)
            .map_err(|error| r["spec_digest"].error("MalformedModelDigest", error.to_string()))?,
        implementation: r["implementation"].text()?.into(),
        suite,
        counts: ScenarioCounts {
            total: counts["total"].unsigned()?,
            passed: counts["passed"].unsigned()?,
            failed: counts["failed"].unsigned()?,
            error: counts["error"].unsigned()?,
            unsupported: counts["unsupported"].unsigned()?,
            skipped: counts["skipped"].unsigned()?,
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

fn admit_suite(suite_json: &str, reading: &EssConformanceV2Reading) -> Result<()> {
    if suite_json.is_empty() {
        return Err(EssAdmissionError::new(
            "MissingSuite",
            "$suite",
            "original suite JSON is required",
        ));
    }
    let parsed_suite = Json::parse(suite_json, "$suite")?;
    let admitted = crate::count_suite::admit(&parsed_suite)?;
    if admitted.version != reading.data().suite.version() {
        return Err(EssAdmissionError::new(
            "SuiteVersionMismatch",
            "$suite.provenance.suite_version",
            format!("expected {}", reading.data().suite.version()),
        ));
    }
    if admitted.spec_digest != reading.data().spec_digest {
        return Err(EssAdmissionError::new(
            "ModelDigestMismatch",
            "$suite.provenance.spec_digest",
            "suite and report name different models",
        ));
    }
    let digest = Sha256::digest(suite_json.as_bytes()).iter().fold(
        "sha256:".to_owned(),
        |mut text, byte| {
            write!(text, "{byte:02x}").expect("writing to String");
            text
        },
    );
    if digest != reading.data().suite.digest() {
        return Err(EssAdmissionError::new(
            "SuiteDigestMismatch",
            "$.suite.digest",
            format!("original suite bytes hash to {digest}"),
        ));
    }
    if admitted.ids != reading.selected_ids() {
        return Err(EssAdmissionError::new(
            "SelectedIdsMismatch",
            "$.outcomes",
            "outcome union must equal every selected suite scenario",
        ));
    }
    Ok(())
}

/// Admits one original report/2 and suite/1–4 pair as exact typed AEP diagnostics.
///
/// # Errors
/// Refuses malformed/unsupported source vocabulary, incoherent counts or mismatching original bytes.
pub fn adapt_json_v2(report_json: &str, suite_json: &str) -> Result<AdaptedEvidence> {
    let mut sources = EssConformanceV2Sources::new(report_json.into(), suite_json.into());
    sources.admit(&CountStageReader)?;
    let completed_at = sources
        .reading()
        .expect("admission installed a reading")
        .data()
        .completed_at;
    Ok(AdaptedEvidence {
        evidence: Evidence::EssConformanceV2(sources),
        observed_at: ObservedAt::new(completed_at),
        producer: Producer::Verifier {
            verifier: Verifier::ConformanceRunner,
        },
        provenance: Provenance::default(),
    })
}
