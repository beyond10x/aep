//! The composite, held to the same sixteen suites as every other backend — the second acceptance
//! line of `story:hybrid-backend`.

use std::path::{Path, PathBuf};

use aep_backend_hybrid::HybridBackend;
use aep_domain::entity::ActorRef;
use aep_domain::time::Timestamp;
use entity_remote::{Authority, OnDivergence, Policy, ReadPath, WhenUnreachable};
use entity_sqlite::SqliteStore;

/// A scratch plan of this name, emptied first so a rerun is a fresh read.
fn scratch(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("conformance")
        .join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch directory");
    root
}

/// The plan in markdown as the authority, an in-memory SQLite replica, divergences recorded.
fn backend(name: &str) -> HybridBackend<SqliteStore> {
    HybridBackend::open(
        scratch(name),
        SqliteStore::in_memory().expect("a database"),
        Policy::new(
            Authority::Local,
            ReadPath::LocalFirst,
            WhenUnreachable::Refuse,
            OnDivergence::RecordDivergence,
        ),
        std::iter::empty(),
        Timestamp::from_epoch_millis(1_700_000_000_000),
        ActorRef::parse("human:conformance").expect("a well-formed actor"),
        aep_domain::artifact::LifecycleRegistry::default(),
    )
    .expect("an empty plan opens")
}

#[test]
fn the_composite_conforms() {
    let store = backend("suites");
    let report = aep_conformance::run(&store, aep_conformance::Level::Full);
    assert!(
        report.passed(),
        "HybridBackend failed {} of {} checks:\n{}",
        report.failures(),
        report.checks(),
        report
            .failing_suites()
            .flat_map(
                |suite| suite
                    .checks
                    .iter()
                    .filter(|check| !check.passed)
                    .map(|check| format!(
                        "  {}: {}",
                        check.name,
                        check.detail.as_deref().unwrap_or("")
                    ))
            )
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(report.suites.len(), 16, "sixteen suites, not a subset");
    assert!(
        store.divergences().is_empty(),
        "an in-memory replica that took every write left no divergence"
    );
}

#[test]
fn the_suites_that_pass_here_catch_a_backend_that_is_wrong() {
    // The guard: a suite that passes proves nothing until something proves it can fail.
    for fault in [
        aep_conformance::Fault::ReplayApplies,
        aep_conformance::Fault::IgnoreExpectedRevision,
        aep_conformance::Fault::DropRejectionAudit,
    ] {
        let faulty = aep_conformance::FaultyBackend::new(backend("faulty"), fault);
        let report = aep_conformance::run(&faulty, aep_conformance::Level::Full);
        assert!(
            !report.passed(),
            "the suites passed a composite injected with {fault:?}, so they are not evidence"
        );
    }
}

#[test]
fn the_replica_is_the_authority_too() {
    // The other declared shape: SQLite decides, the markdown follows a write it accepted. Same
    // sixteen suites.
    let store = HybridBackend::open(
        scratch("replica-authority"),
        SqliteStore::in_memory().expect("a database"),
        Policy::new(
            Authority::Remote,
            ReadPath::RemoteFirst,
            WhenUnreachable::Refuse,
            OnDivergence::Refuse,
        ),
        std::iter::empty(),
        Timestamp::from_epoch_millis(1_700_000_000_000),
        ActorRef::parse("human:conformance").expect("a well-formed actor"),
        aep_domain::artifact::LifecycleRegistry::default(),
    )
    .expect("an empty plan opens");
    let report = aep_conformance::run(&store, aep_conformance::Level::Full);
    assert!(
        report.passed(),
        "with the replica as authority, {} of {} checks failed",
        report.failures(),
        report.checks()
    );
}
