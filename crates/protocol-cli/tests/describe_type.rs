//! Wave H, story 2: `describe_type` reports the ladder the kernel decides with — D-P5.
//!
//! Three backends over one set of ladders answer the same descriptor for every planning kind, and
//! the descriptor's edges are exactly what `protocol artifact lifecycle <kind>` prints, because both
//! come from the same `EntityDefinition` the kernel executes.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use aep_backend_entity::{EntityBackend, Identity};
use aep_backend_markdown::backend::MarkdownBackend;
use aep_contract::query::QueryService;
use aep_contract::registry::LifecycleDescriptor;
use aep_contract::testing::block_on;
use aep_domain::artifact::{ArtifactKind, ArtifactStatus, LifecycleRegistry};
use aep_domain::entity::ActorRef;
use aep_domain::evidence::EvidenceKind;
use aep_domain::time::Timestamp;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// The ladders this repository's document tree declares, loaded as `protocol` loads them.
fn ladders() -> LifecycleRegistry {
    let outcome = aep_project::load_tree_report(&root());
    assert!(outcome.failures.is_empty(), "{:?}", outcome.failures);
    outcome.registry.lifecycles().clone()
}

/// Every named kind the ladders declare a lifecycle for.
fn described_kinds(ladders: &LifecycleRegistry) -> Vec<&'static ArtifactKind> {
    ArtifactKind::NAMED
        .iter()
        .filter(|kind| ladders.for_kind(kind).is_some())
        .collect()
}

fn describe<B: QueryService>(backend: &B, kind: &ArtifactKind) -> LifecycleDescriptor {
    block_on(backend.describe_type(&kind.entity_type()))
        .unwrap_or_else(|error| panic!("{kind}: {error}"))
        .lifecycle
        .unwrap_or_else(|| panic!("{kind}: the descriptor reports no lifecycle"))
}

#[test]
fn every_backend_reports_the_same_ladder_for_every_planning_kind() {
    let ladders = ladders();
    let kinds = described_kinds(&ladders);
    assert!(
        kinds.len() >= 8,
        "{} ladders: the fixture is the real tree",
        kinds.len()
    );

    let memory = EntityBackend::shaped(
        entity_store::MemoryStore::new(),
        Identity::with_lifecycles(ladders.clone()),
    )
    .expect("opens");
    let sqlite = EntityBackend::shaped(
        entity_sqlite::SqliteStore::in_memory().expect("a database"),
        Identity::with_lifecycles(ladders.clone()),
    )
    .expect("opens");
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("describe-type-markdown");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("a scratch store");
    let markdown = MarkdownBackend::open(
        &scratch,
        std::iter::empty(),
        Timestamp::from_epoch_millis(1_700_000_000_000),
        ActorRef::parse("human:operator").expect("an actor"),
        ladders.clone(),
    )
    .expect("opens");

    for kind in &kinds {
        let from_memory = describe(&memory, kind);
        assert_eq!(
            describe(&sqlite, kind),
            from_memory,
            "{kind}: SQLite differs"
        );
        assert_eq!(
            describe(&markdown, kind),
            from_memory,
            "{kind}: markdown differs"
        );
        assert!(
            !from_memory.statuses.is_empty() && !from_memory.transitions.is_empty(),
            "{kind}: a ladder with states and edges"
        );
    }

    // The rung that costs evidence is reported as such: gap-register :39 read from the far side.
    let story = describe(&memory, &ArtifactKind::Story);
    let implemented = ArtifactStatus::parse("implemented").expect("a status");
    let costs = story
        .requires
        .iter()
        .find(|(rung, _)| rung == &implemented)
        .map(|(_, requirements)| requirements.clone())
        .expect("`implemented` costs evidence in this repository's story ladder");
    assert_eq!(costs, vec![(EvidenceKind::TestResult, 1)]);
}

#[test]
fn the_descriptor_edges_are_what_protocol_artifact_lifecycle_prints() {
    let ladders = ladders();
    let backend = EntityBackend::shaped(
        entity_store::MemoryStore::new(),
        Identity::with_lifecycles(ladders.clone()),
    )
    .expect("opens");

    for kind in described_kinds(&ladders) {
        let printed = Command::new(env!("CARGO_BIN_EXE_protocol"))
            .args(["artifact", "lifecycle", kind.as_str(), "--format", "json"])
            .arg("--root")
            .arg(root())
            .arg("--store")
            .arg(root().join(".engineering/planning"))
            .current_dir(root())
            .output()
            .expect("the protocol binary runs");
        assert!(
            printed.status.success(),
            "{kind}: {}",
            String::from_utf8_lossy(&printed.stderr)
        );
        let view: serde_json::Value =
            serde_json::from_slice(&printed.stdout).expect("JSON from the lifecycle verb");
        // Compared as sets: the two renderings order a rung's targets differently (one by the
        // status vocabulary's own order, one alphabetically), and the claim is about which edges
        // exist, not how they are listed.
        let printed_edges: BTreeMap<String, BTreeSet<String>> =
            serde_json::from_value(view["transitions"].clone()).expect("a transitions map");

        let described = describe(&backend, kind);
        let described_edges: BTreeMap<String, BTreeSet<String>> = described
            .transitions
            .iter()
            .map(|(from, to)| {
                (
                    from.as_str().to_owned(),
                    to.iter().map(|status| status.as_str().to_owned()).collect(),
                )
            })
            .collect();
        assert_eq!(described_edges, printed_edges, "{kind}: the edges differ");
        assert_eq!(
            described.initial.as_str(),
            view["initial"],
            "{kind}: the start differs"
        );
    }
}
