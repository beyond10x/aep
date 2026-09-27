//! The one-time migration of an `aep.project/3` tree store into an `aep.project/4` one: the target
//! holds the source's history record for record, and each large value once.

use std::path::{Path, PathBuf};

use aep_backend_eventlog::{
    migrate_to_content, open_tree, open_tree_session, prepare_tree, provision_tree,
    verify_equivalent, AuthoritySession,
};
use aep_conformance::Level;
use aep_domain::artifact::LifecycleRegistry;
use entity_eventlog::EventlogOperationContext;
use time::OffsetDateTime;

const SCOPE: &str = "scope-planning";
const TENANT: &str = "planning";

fn context() -> EventlogOperationContext {
    EventlogOperationContext {
        subject: "aep".into(),
        actor: "aep".into(),
        request_id: "provision".into(),
        trace_id: "provision".into(),
        causation_id: None,
        causation_depth: 0,
        occurred_at: OffsetDateTime::UNIX_EPOCH,
    }
}

/// An `aep.project/3` store holding every history the conformance suites write.
fn populated_source(root: &Path) -> String {
    let identity = prepare_tree(root, TENANT).expect("a tree store is prepared");
    provision_tree(
        root,
        SCOPE.into(),
        TENANT.into(),
        identity.clone(),
        context(),
    )
    .expect("its binding is provisioned");
    let backend = open_tree(
        root.to_owned(),
        SCOPE.into(),
        TENANT.into(),
        identity.clone(),
        LifecycleRegistry::new(),
        None,
    )
    .expect("it opens");
    assert!(aep_conformance::run(&backend, Level::Full).passed());
    identity
}

fn session(root: &Path, identity: &str, blobs: Option<PathBuf>) -> AuthoritySession {
    open_tree_session(
        root.to_owned(),
        SCOPE.into(),
        TENANT.into(),
        identity.to_owned(),
        LifecycleRegistry::new(),
        blobs,
    )
    .expect("the session opens")
}

fn files_under(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_owned()];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory)
            .into_iter()
            .flatten()
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                if !path.ends_with(".cache") {
                    stack.push(path);
                }
            } else {
                found.push(path);
            }
        }
    }
    found
}

#[test]
fn a_migrated_store_holds_the_source_history_record_for_record_under_the_same_identity() {
    let directory = tempfile::tempdir().expect("directory");
    let (source_root, target_root, blobs) = (
        directory.path().join("state"),
        directory.path().join("state.new"),
        directory.path().join("blobs"),
    );
    let identity = populated_source(&source_root);
    let source = session(&source_root, &identity, None);

    let report = migrate_to_content(&source, &target_root, &blobs, &LifecycleRegistry::new())
        .expect("the migration replays");
    assert_eq!(report.stream_identity, identity);
    assert!(report.decisions > 0, "{report:?}");

    let target = session(&target_root, &identity, Some(blobs.clone()));
    let before = source.complete_snapshot().expect("the source reads");
    let after = target.complete_snapshot().expect("the target reads");
    let equivalence = verify_equivalent(&before, &after);
    assert_eq!(equivalence.differences, Vec::<String>::new());
    assert_eq!(equivalence.subjects, before.histories.len());
    assert!(equivalence.records > 0);

    // The same identities, so every recorded coordinate that names the authority still does.
    for file in ["store.json", "tenants/planning/identity.json"] {
        assert_eq!(
            std::fs::read(source_root.join(file)).expect("source identity"),
            std::fs::read(target_root.join(file)).expect("target identity"),
            "{file}"
        );
    }
    assert!(
        !files_under(&blobs).is_empty(),
        "no value reached the blob directory"
    );
    assert!(target.content().expect("content").verify().is_empty());
}

#[test]
fn a_migrated_store_whose_blob_is_missing_refuses_to_read_instead_of_reading_a_reference() {
    let directory = tempfile::tempdir().expect("directory");
    let (source_root, target_root, blobs) = (
        directory.path().join("state"),
        directory.path().join("state.new"),
        directory.path().join("blobs"),
    );
    let identity = populated_source(&source_root);
    let source = session(&source_root, &identity, None);
    migrate_to_content(&source, &target_root, &blobs, &LifecycleRegistry::new())
        .expect("the migration replays");
    for blob in files_under(&blobs) {
        std::fs::remove_file(blob).expect("removed");
    }
    let error = session(&target_root, &identity, Some(blobs))
        .complete_snapshot()
        .expect_err("a reference to a missing blob is refused");
    assert!(error.contains("content blob"), "{error}");
}

#[test]
fn a_store_that_already_keeps_content_blobs_or_a_target_that_exists_is_refused() {
    let directory = tempfile::tempdir().expect("directory");
    let (source_root, blobs) = (
        directory.path().join("state"),
        directory.path().join("blobs"),
    );
    let identity = populated_source(&source_root);
    let migrated = session(&source_root, &identity, Some(blobs.clone()));
    let error = migrate_to_content(
        &migrated,
        &directory.path().join("elsewhere"),
        &blobs,
        &LifecycleRegistry::new(),
    )
    .expect_err("refused");
    assert!(error.contains("already keeps content blobs"), "{error}");

    let source = session(&source_root, &identity, None);
    let error = migrate_to_content(&source, &source_root, &blobs, &LifecycleRegistry::new())
        .expect_err("refused");
    assert!(error.contains("already exists"), "{error}");
}
