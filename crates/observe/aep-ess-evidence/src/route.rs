//! Which reader an original suite goes to, by what the suite is rather than by its number.
use aep_domain::ess_conformance_coverage::is_coverage_suite_version;
use aep_domain::ess_conformance_v2::{is_count_suite_version, EssAdmissionError};

/// The reader an original suite is admitted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuiteRoute {
    /// A coverage major (`COVERAGE_SUITE_MAJORS`): the coverage reader, through an input/1
    /// carrier ([`crate::wrap_coverage_suite`]).
    Coverage,
    /// A frozen suite/1–4 or an ordinary major (`ORDINARY_SUITE_MAJORS`), or a suite that does not
    /// state a version: the count reader ([`crate::adapt_json_v2`]), which names whatever is
    /// missing or malformed.
    Count,
}

/// Chooses the reader for `suite_json` from its `provenance.suite_version`.
///
/// This probe chooses a reader only; admission re-reads the complete original bytes.
///
/// # Errors
/// Refuses, by name, a version that neither the coverage nor the count reader knows, before
/// either interprets the suite: a suite from an ESS newer than this build is
/// `UnsupportedSuiteVersion`, never a missing field of whichever reader it would have reached.
pub fn suite_route(suite_json: &str) -> Result<SuiteRoute, EssAdmissionError> {
    let probe = serde_json::from_str::<serde_json::Value>(suite_json).ok();
    let Some(version) = probe
        .as_ref()
        .and_then(|value| value.get("provenance"))
        .and_then(|provenance| provenance.get("suite_version"))
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(SuiteRoute::Count);
    };
    if is_coverage_suite_version(version) {
        Ok(SuiteRoute::Coverage)
    } else if is_count_suite_version(version) {
        Ok(SuiteRoute::Count)
    } else {
        Err(EssAdmissionError::new(
            "UnsupportedSuiteVersion",
            "$suite.provenance.suite_version",
            format!("{version} is not a suite version this build admits"),
        ))
    }
}
