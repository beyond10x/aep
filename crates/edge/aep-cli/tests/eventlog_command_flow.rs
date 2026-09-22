//! Real-process coverage for an ordinary command flow over the file Eventlog planning backend.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use entity_eventlog::{Authority, EventlogOperationContext};
use time::OffsetDateTime;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn scratch() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "aep-eventlog-command-flow-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".engineering")).expect("engineering root");
    std::fs::create_dir_all(root.join("protocols")).expect("protocol root");
    root
}

fn invoke(binary: &str, root: &Path, args: &[&str]) -> Output {
    Command::new(binary)
        .args(args)
        .current_dir(root)
        .output()
        .expect("AEP binary runs")
}

fn assert_success(output: &Output, operation: &str) {
    assert!(
        output.status.success(),
        "{operation} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
// This is one ordered subprocess acceptance scenario: discovery, mutation, query, history and
// inspection must all observe the same ordinary Eventlog store.
#[allow(clippy::too_many_lines)]
fn ordinary_discovery_mutates_queries_histories_and_inspects_file_eventlog() {
    let root = scratch();
    let engineering = root.join(".engineering");
    let authority_root = engineering.join("state");
    let identity = aep_backend_eventlog::prepare_file(&authority_root, "tenant-command-flow")
        .expect("provider prepares a disposable file authority");
    let authority = Authority {
        logical_scope: "planning-command-flow".to_owned(),
        tenant: "tenant-command-flow".to_owned(),
        stream_identity: identity.clone(),
    };
    aep_backend_eventlog::provision_file(
        &authority_root,
        authority.logical_scope.clone(),
        authority.tenant.clone(),
        authority.stream_identity.clone(),
        EventlogOperationContext {
            subject: "eventlog-command-flow-test".to_owned(),
            actor: "eventlog-command-flow-test".to_owned(),
            request_id: "eventlog-command-flow-binding".to_owned(),
            trace_id: "eventlog-command-flow".to_owned(),
            causation_id: None,
            causation_depth: 0,
            occurred_at: OffsetDateTime::UNIX_EPOCH,
        },
    )
    .expect("disposable authority binding");
    std::fs::write(
        engineering.join("project.yaml"),
        format!(
            concat!(
                "{{\"version\":\"aep.project/2\",",
                "\"protocol\":\"adp/1\",\"profile\":\"development.standard\",",
                "\"protocols\":\"../protocols\",",
                "\"store\":{{\"eventlog\":{{\"path\":\"state\",\"projection\":\"planning\"}}}},",
                "\"planning_scope\":\"planning-command-flow\",",
                "\"planning_tenant\":\"tenant-command-flow\",",
                "\"planning_identity\":{:?}}}\n"
            ),
            identity,
        ),
    )
    .expect("v2 selector");

    let aep = env!("CARGO_BIN_EXE_aep");
    let protocol = env!("CARGO_BIN_EXE_protocol");
    let created = invoke(
        aep,
        &root,
        &[
            "plan",
            "artifact",
            "new",
            "story",
            "one",
            "--title",
            "One",
            "--command-identity",
            "command-flow-create",
            "--format",
            "json",
        ],
    );
    assert_success(&created, "ordinary mutation");
    let mutation: serde_json::Value =
        serde_json::from_slice(&created.stdout).expect("mutation result is JSON");
    assert_eq!(mutation["format"], "aep.planning-mutation/1");

    let list_args = ["plan", "artifact", "list", "--format", "json"];
    // Create the nested ordinary-discovery location only after provisioning; no explicit store or
    // project path is passed to either spelling.
    std::fs::create_dir_all(root.join("nested")).expect("nested discovery directory");
    let listed = invoke(aep, &root.join("nested"), &list_args);
    assert_success(&listed, "ordinary discovered query");
    assert!(String::from_utf8_lossy(&listed.stdout).contains("story:one"));
    let aliased = invoke(protocol, &root.join("nested"), &list_args);
    assert_success(&aliased, "protocol alias query");
    assert_eq!(
        listed.stdout, aliased.stdout,
        "aep/protocol aliases diverged"
    );

    let history = invoke(
        aep,
        &root,
        &[
            "plan",
            "artifact",
            "history",
            "story:one",
            "--format",
            "json",
        ],
    );
    assert_success(&history, "ordinary history");
    assert!(String::from_utf8_lossy(&history.stdout).contains("story:one"));

    let inspected = invoke(
        aep,
        &root,
        &["plan", "store", "inspect", "--format", "json"],
    );
    assert_success(&inspected, "authority inspection");
    let inspection: serde_json::Value =
        serde_json::from_slice(&inspected.stdout).expect("inspection result is JSON");
    assert_eq!(inspection["format"], "aep.planning-inspection/1");

    let snapshot = aep_backend_eventlog::complete_file_snapshot(&authority_root, authority)
        .expect("complete current authority");
    let coordinate = aep_contract::migration::AuthorityCoordinateV1 {
        logical_scope: aep_contract::migration::AuthorityValueV1::new("planning-command-flow")
            .expect("scope"),
        tenant: aep_contract::migration::AuthorityValueV1::new("tenant-command-flow")
            .expect("tenant"),
        stream_identity: aep_contract::migration::AuthorityValueV1::new(identity)
            .expect("identity"),
    };
    let (snapshot_id, _) =
        aep_planning_migration::authority_snapshot_identity(&coordinate, &snapshot)
            .expect("snapshot identity");
    let snapshot_wire = snapshot_id.0.as_wire();
    let rebuilt = invoke(
        aep,
        &root,
        &[
            "plan",
            "store",
            "rebuild",
            "--authority-snapshot",
            snapshot_wire.as_str(),
            "--format",
            "json",
        ],
    );
    assert_eq!(rebuilt.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&rebuilt.stdout).contains("writer_exclusion_unavailable"));

    let _ = std::fs::remove_dir_all(root);
}
