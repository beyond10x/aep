//! Optional adapter from a standalone ESS conformance report to AEP evidence.
//!
//! The report wire is transcribed here rather than imported from ESS. That keeps the dependency
//! arrow one-way: ESS describes what its runner observed, while this adapter chooses how AEP
//! represents that observation. Core AEP and its evidence model compile without ESS modeling
//! types.

use std::fmt;

use aep_domain::evidence::{EssConformanceResult, Evidence, Producer, Provenance, SpecDigest};
use aep_domain::time::{ObservedAt, Timestamp};
use aep_domain::verification::{VerificationStatus, Verifier};

/// Persisted format this adapter understands.
pub const STANDALONE_REPORT_FORMAT: &str = "ess-conformance-report/1";

/// A standalone ESS conformance report as it crosses into AEP.
///
/// This private transcription intentionally contains no ESS library type. Unknown fields are
/// refused so a future report revision cannot be silently interpreted with older semantics.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StandaloneReport {
    format: String,
    specification: String,
    spec_digest: SpecDigest,
    implementation: String,
    status: VerificationStatus,
    scenarios_total: usize,
    scenarios_failed: usize,
    suite_version: String,
    failed_scenarios: Vec<String>,
    completed_at: Timestamp,
}

/// One AEP evidence entry produced from a standalone ESS report.
///
/// Serializing this value produces the entry shape accepted inside an AEP evidence-document list.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AdaptedEvidence {
    #[serde(flatten)]
    evidence: Evidence,
    observed_at: ObservedAt,
    producer: Producer,
    #[serde(skip_serializing_if = "is_empty_provenance")]
    provenance: Provenance,
}

fn is_empty_provenance(provenance: &Provenance) -> bool {
    provenance == &Provenance::default()
}

impl AdaptedEvidence {
    /// Returns the AEP observation payload.
    pub fn evidence(&self) -> &Evidence {
        &self.evidence
    }

    /// Returns when ESS completed the conformance run.
    pub fn observed_at(&self) -> ObservedAt {
        self.observed_at
    }

    /// Returns the fixed conformance-runner producer.
    pub fn producer(&self) -> &Producer {
        &self.producer
    }

    /// Returns how the standalone report was obtained.
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    /// Records the report source as an AEP provenance input.
    #[must_use]
    pub fn reading(mut self, input: impl Into<String>) -> Self {
        self.provenance.inputs.push(input.into());
        self
    }
}

/// Why a standalone report could not be adapted.
#[derive(Debug)]
pub enum AdapterError {
    /// The JSON does not have the closed standalone-report shape.
    InvalidDocument(serde_json::Error),
    /// The document declares a format this adapter does not implement.
    UnsupportedFormat(String),
    /// Scenario totals contradict one another or the overall verdict.
    InconsistentCounts {
        /// Total scenarios the report says ran.
        total: usize,
        /// Scenarios the report says did not pass.
        failed: usize,
        /// Failure descriptions the report carries.
        described: usize,
        /// Overall status the report claims.
        status: VerificationStatus,
    },
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDocument(error) => write!(formatter, "invalid standalone ESS report: {error}"),
            Self::UnsupportedFormat(format) => write!(
                formatter,
                "unsupported ESS conformance report format `{format}`; expected `{STANDALONE_REPORT_FORMAT}`"
            ),
            Self::InconsistentCounts {
                total,
                failed,
                described,
                status,
            } => write!(
                formatter,
                "ESS conformance report counts disagree: status {status}, {failed} of {total} failed, {described} failure descriptions"
            ),
        }
    }
}

impl std::error::Error for AdapterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidDocument(error) => Some(error),
            Self::UnsupportedFormat(_) | Self::InconsistentCounts { .. } => None,
        }
    }
}

/// Converts one standalone ESS conformance report into AEP `ess_conformance` evidence.
///
/// # Errors
///
/// Refuses malformed JSON, an unknown report format, and reports whose counts contradict the
/// failure list or a passing verdict.
pub fn adapt_json(document: &str) -> Result<AdaptedEvidence, AdapterError> {
    let report: StandaloneReport =
        serde_json::from_str(document).map_err(AdapterError::InvalidDocument)?;
    if report.format != STANDALONE_REPORT_FORMAT {
        return Err(AdapterError::UnsupportedFormat(report.format));
    }
    let counts_disagree = report.scenarios_failed > report.scenarios_total
        || report.scenarios_failed != report.failed_scenarios.len()
        || (report.status == VerificationStatus::Passed && report.scenarios_failed != 0);
    if counts_disagree {
        return Err(AdapterError::InconsistentCounts {
            total: report.scenarios_total,
            failed: report.scenarios_failed,
            described: report.failed_scenarios.len(),
            status: report.status,
        });
    }

    let evidence = Evidence::EssConformance(EssConformanceResult {
        specification: report.specification,
        spec_digest: report.spec_digest,
        implementation: report.implementation,
        status: report.status,
        scenarios_total: report.scenarios_total,
        scenarios_failed: report.scenarios_failed,
        suite_version: Some(report.suite_version),
        compiler_version: None,
        generator_version: None,
        failed_scenarios: report.failed_scenarios,
    });
    Ok(AdaptedEvidence {
        evidence,
        observed_at: ObservedAt::new(report.completed_at),
        producer: Producer::Verifier {
            verifier: Verifier::ConformanceRunner,
        },
        provenance: Provenance::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPORT: &str = r#"{
  "format": "ess-conformance-report/1",
  "specification": "billing/v3",
  "spec_digest": "13577b3ce695932e980d418d5863bcde07f4c362516d53147870d31eaf2ed861",
  "implementation": "billing-reference",
  "status": "failed",
  "scenarios_total": 2,
  "scenarios_failed": 1,
  "suite_version": "ess-conformance/1",
  "failed_scenarios": ["failed billing.invoice.CreateInvoice/outcome/rejected"],
  "completed_at": 1700000000000
}"#;

    #[test]
    fn a_standalone_report_becomes_the_existing_aep_evidence_shape() {
        let adapted = adapt_json(REPORT).expect("report adapts");
        let Evidence::EssConformance(result) = adapted.evidence() else {
            panic!("the adapter produced a different evidence kind");
        };
        assert_eq!(result.specification, "billing/v3");
        assert_eq!(result.scenarios_failed, 1);
        assert_eq!(result.status, VerificationStatus::Failed);
        assert_eq!(
            adapted.observed_at().timestamp(),
            Timestamp::from_epoch_millis(1_700_000_000_000)
        );
        assert_eq!(
            adapted.producer(),
            &Producer::Verifier {
                verifier: Verifier::ConformanceRunner
            }
        );
    }

    #[test]
    fn an_unknown_report_version_is_refused_before_fields_are_interpreted() {
        let report = REPORT.replace("ess-conformance-report/1", "ess-conformance-report/2");
        let error = adapt_json(&report).expect_err("unknown format must be refused");
        assert!(
            matches!(error, AdapterError::UnsupportedFormat(format) if format == "ess-conformance-report/2")
        );
    }

    #[test]
    fn a_passing_report_with_a_failed_scenario_is_refused() {
        let report = REPORT.replace("\"status\": \"failed\"", "\"status\": \"passed\"");
        let error = adapt_json(&report).expect_err("contradictory report must be refused");
        assert!(matches!(
            error,
            AdapterError::InconsistentCounts {
                total: 2,
                failed: 1,
                described: 1,
                status: VerificationStatus::Passed
            }
        ));
    }

    #[test]
    fn a_report_field_added_without_a_format_change_is_refused() {
        let report = REPORT.replace("\n}", ",\n  \"unrecognized_semantics\": true\n}");
        let error = adapt_json(&report).expect_err("unknown field must be refused");
        assert!(matches!(error, AdapterError::InvalidDocument(_)));
    }
}
