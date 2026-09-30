//! A report ESS wrote from an external runner's per-scenario results (`ess verify conform
//! report`, ESS 0.47.0), with the ESS-executed report of the same suite beside it.
#![allow(dead_code)]

/// ESS's own report/2 for results `acme-runner@1.4.0` supplied; ESS executed nothing.
pub const REPORT: &str = include_str!("../fixtures/external-scenario-status/report.json");
/// The report ESS wrote when it executed the same suite itself.
pub const EXECUTED: &str = include_str!("../fixtures/external-scenario-status/own.json");
/// The exact suite/5 bytes both reports reference.
pub const SUITE: &str = include_str!("../fixtures/external-scenario-status/suite.json");
/// The profile the fixture report carries.
pub const RUNNER_PROFILE: &str = "external-scenario-status/1;runner=acme-runner@1.4.0";

/// Every external spelling ESS writes; each must be recorded exactly as written.
pub const ACCEPTED: [&str; 3] = [
    "external-scenario-status/1",
    RUNNER_PROFILE,
    // ESS splits the runner at its last `@`, so a name may carry one.
    "external-scenario-status/1;runner=@scope/runner@0.1.0-rc.1",
];

/// Spellings near the external profile that ESS never writes.
pub const REFUSED: [&str; 14] = [
    "external-scenario-status/2",
    "external-scenario-status",
    "external-scenario-status/1;",
    "external-scenario-status/1;runner=",
    "external-scenario-status/1;runner=acme-runner",
    "external-scenario-status/1;runner=acme-runner@",
    "external-scenario-status/1;runner=@1.4.0",
    "external-scenario-status/1;runner=acme runner@1.4.0",
    "external-scenario-status/1;runner=acme-runner@1.4.0\n",
    "external-scenario-status/1;runner=acme-runner@1.4.0;extra",
    "external-scenario-status/1;name=acme-runner@1.4.0",
    "external-scenario-status/1 ;runner=acme-runner@1.4.0",
    "rust-scenario-status/1;runner=acme-runner@1.4.0",
    "External-Scenario-Status/1",
];
