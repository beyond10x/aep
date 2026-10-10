//! Ordinary suites from /6 on: no `coverage` block, read for their identity and scenario keys.
//!
//! An ordinary major is the even neighbour of a coverage major, and carries the same provenance
//! and scenario-id vocabulary that coverage major does. So this reader gates each member and id
//! form by major exactly as `coverage_suite` does, and, as for every coverage major after /5,
//! leaves scenario bodies to ESS, which admitted the suite before reporting on it: the report
//! binds every original byte by digest, which `counts::admit_suite` checks.
use aep_domain::ess_conformance_v2::is_ordinary_suite_version;
use aep_domain::evidence::SpecDigest;

use crate::count_json::{Json, Result};
use crate::count_suite::AdmittedSuite;
use crate::coverage_suite::{
    admit_id_forms, admit_provenance_vocabulary, admit_seed_record, major, scenario_keys,
};

/// Admits one ordinary suite whose `provenance.suite_version` is `version`.
pub(crate) fn admit(value: &Json, version: &str) -> Result<AdmittedSuite> {
    debug_assert!(is_ordinary_suite_version(version));
    let fields = value.closed(&["provenance", "scenarios"], &[])?;
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
    let major = major(version);
    admit_provenance_vocabulary(major, &fields["provenance"], provenance)?;
    provenance["system"].text()?;
    provenance["specification_version"].text()?;
    if let Some(component) = provenance.get("component") {
        if !component.null() {
            component.text()?;
        }
    }
    let digest = |value: &Json| {
        SpecDigest::new(value.text()?)
            .map_err(|error| value.error("MalformedModelDigest", error.to_string()))
    };
    let spec_digest = digest(&provenance["spec_digest"])?;
    digest(&provenance["contract_digest"])?;
    let ids = scenario_keys(&fields["scenarios"])?;
    admit_id_forms(version, &ids, &fields["scenarios"])?;
    if let Some(seeds) = provenance.get("synthesis_seeds") {
        admit_seed_record(major, seeds, &ids, None)?;
    }
    Ok(AdmittedSuite {
        version: version.into(),
        spec_digest,
        ids,
    })
}
