//! Public subprocess acceptance for the manually held writer-control edge.

#![cfg(target_os = "linux")]

use std::fs;
use std::io::{BufRead as _, BufReader, Write as _};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use aep_backend_markdown::MarkdownStore;
use aep_backend_memory::seed;
use aep_domain::entity::ActorRef;
use aep_domain::time::Timestamp;

static NEXT: AtomicU64 = AtomicU64::new(0);

fn project() -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "aep-public-writer-control-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let engineering = root.join(".engineering");
    fs::create_dir_all(engineering.join("planning")).expect("disposable planning source");
    fs::create_dir_all(engineering.join("planning/story")).expect("story directory");
    fs::write(engineering.join("planning/story/one.md"),
        "---\nformat: aep.planning-md/1\nid: story:one\nkind: story\nstatus: draft\ntitle: One\nrelations: []\nrevision: 1\n---\n")
        .expect("legacy story");
    fs::create_dir_all(root.join("protocols")).expect("disposable protocol root");
    let selector = engineering.join("project.yaml");
    fs::write(&selector, "{\"protocol\":\"adp/1\",\"profile\":\"development.standard\",\"protocols\":\"../protocols\"}\n")
        .expect("legacy selector");
    (root, selector)
}

fn run(root: &PathBuf, args: &[&str]) -> (bool, serde_json::Value, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(args)
        .current_dir(root)
        .output()
        .expect("AEP runs");
    let value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "JSON command result: {} / {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (
        output.status.success(),
        value,
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn hold(
    root: &PathBuf,
    selector: &PathBuf,
    identity: &[&str],
    resume: bool,
    before_stop: Option<&[&str]>,
) -> (Child, BufReader<std::process::ChildStdout>) {
    let mut writer = (!resume).then(|| {
        Command::new("sleep")
            .arg("60")
            .spawn()
            .expect("disposable writer process")
    });
    let mut command = Command::new(env!("CARGO_BIN_EXE_aep"));
    command
        .args(["plan", "store", "writer-control", "hold", "--project"])
        .arg(selector)
        .args(identity)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(root);
    if resume {
        command.arg("--resume-stop");
    } else {
        command.args([
            "--writer-pid",
            &writer.as_ref().expect("live writer").id().to_string(),
        ]);
    }
    let holder = command.spawn().expect("holder starts");
    if let Some(writer) = writer.as_mut() {
        // The holder samples the live identity before the operator stops it.
        thread::sleep(Duration::from_millis(300));
        if let Some(args) = before_stop {
            let (success, refused, _) = run(root, args);
            assert!(
                !success && refused.to_string().contains("writer_exclusion_unavailable"),
                "live writer is not excluded: {refused}"
            );
        }
        writer.kill().expect("operator stops the disposable writer");
        writer.wait().expect("writer drained");
    }
    activate_holder(holder)
}

fn hold_already_idle(
    root: &PathBuf,
    selector: &PathBuf,
    identity: &[&str],
) -> (Child, BufReader<std::process::ChildStdout>) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_aep"));
    command
        .args(["plan", "store", "writer-control", "hold", "--project"])
        .arg(selector)
        .args(identity)
        .arg("--already-idle")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(root);
    activate_holder(command.spawn().expect("already-idle holder starts"))
}

fn activate_holder(mut holder: Child) -> (Child, BufReader<std::process::ChildStdout>) {
    let mut stdout = BufReader::new(holder.stdout.take().expect("holder stdout"));
    let mut line = String::new();
    stdout
        .read_line(&mut line)
        .expect("holder custody preparation");
    assert!(
        line.contains("Retained stop evidence") || line.contains("Accepted already-idle assertion"),
        "{line}"
    );
    line.clear();
    stdout.read_line(&mut line).expect("custody prompt");
    assert!(line.contains("Type HOLD"), "{line}");
    holder
        .stdin
        .as_mut()
        .expect("holder input")
        .write_all(b"HOLD\n")
        .expect("operator takes custody");
    line.clear();
    stdout.read_line(&mut line).expect("holder active");
    assert!(line.contains("Writer control held"), "{line}");
    (holder, stdout)
}

fn holder_output(
    root: &PathBuf,
    selector: &PathBuf,
    identity: &[&str],
    mode: &[&str],
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(["plan", "store", "writer-control", "hold", "--project"])
        .arg(selector)
        .args(identity)
        .args(mode)
        .current_dir(root)
        .output()
        .expect("holder refusal runs")
}

fn stop_holder(holder: &mut Child) {
    holder
        .stdin
        .as_mut()
        .expect("holder input")
        .write_all(b"\n")
        .expect("release custody");
    assert!(holder.wait().expect("holder exits").success());
}

fn seed_sqlite(planning: &PathBuf, database: &PathBuf) {
    let report = MarkdownStore::open(planning).load();
    assert!(report.is_clean());
    let graph = report.graph().expect("legacy graph");
    let backend = aep_backend_sqlite::SqliteBackend::open(database).expect("SQLite source");
    seed::from_manifest(
        &backend,
        &graph,
        aep_backend_markdown::backend::ORGANISATION,
        aep_backend_markdown::backend::SPACE,
        Timestamp::from_epoch_millis(1_700_000_000_000),
        &ActorRef::parse("human:writer-control-fixture").expect("actor"),
    )
    .expect("SQLite source seeded");
}

fn align_replica_id_with_markdown_mapping(database: &PathBuf) -> aep_domain::entity::EntityId {
    let digest = aep_contract::migration::digest_parts_v1(
        "aep.migration.markdown-entity/1",
        &[b"story/one.md".to_vec(), b"story:one".to_vec()],
    )
    .expect("fixed mapping input frames");
    let mapped = aep_domain::entity::EntityId::new(format!(
        "MIG{}",
        digest.as_wire().trim_start_matches("sha256:")
    ))
    .expect("mapped Markdown identity is admitted");
    let mut connection = rusqlite::Connection::open(database).expect("SQLite source reopens");
    let transaction = connection
        .transaction()
        .expect("identity fixture transaction");
    let document: String = transaction
        .query_row(
            "SELECT document FROM instances WHERE entity = 'aep.entity' ORDER BY id LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("replica subject exists");
    let mut instance: entity_core::EntityInstance =
        serde_json::from_str(&document).expect("replica terminal is typed");
    let original = instance.id.clone();
    instance.id = mapped.to_string();
    instance
        .fields
        .get_mut("$aep")
        .and_then(serde_json::Value::as_object_mut)
        .and_then(|value| value.get_mut("metadata"))
        .and_then(serde_json::Value::as_object_mut)
        .expect("replica terminal has AEP metadata")
        .insert(
            "id".to_owned(),
            serde_json::Value::String(mapped.to_string()),
        );
    transaction
        .execute(
            "UPDATE instances SET id = ?1, document = ?2 WHERE entity = 'aep.entity' AND id = ?3",
            rusqlite::params![
                mapped.to_string(),
                serde_json::to_string(&instance).expect("terminal serialises"),
                original
            ],
        )
        .expect("replica terminal identity aligns");
    let events = {
        let mut statement = transaction
            .prepare(
                "SELECT revision, position, document FROM events \
                 WHERE entity = 'aep.entity' AND id = ?1 ORDER BY revision, position",
            )
            .expect("event query prepares");
        statement
            .query_map([&original], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .expect("events query")
            .collect::<Result<Vec<_>, _>>()
            .expect("events collect")
    };
    for (revision, position, document) in events {
        let mut event: entity_core::DomainEvent =
            serde_json::from_str(&document).expect("replica event is typed");
        event.id = mapped.to_string();
        transaction
            .execute(
                "UPDATE events SET id = ?1, document = ?2 \
                 WHERE entity = 'aep.entity' AND id = ?3 AND revision = ?4 AND position = ?5",
                rusqlite::params![
                    mapped.to_string(),
                    serde_json::to_string(&event).expect("event serialises"),
                    original,
                    revision,
                    position
                ],
            )
            .expect("replica event identity aligns");
    }
    transaction
        .execute(
            "UPDATE legacy_origins SET id = ?1 WHERE entity = 'aep.entity' AND id = ?2",
            rusqlite::params![mapped.to_string(), original],
        )
        .expect("replica origin identity aligns");
    transaction.commit().expect("identity fixture commits");
    mapped
}

fn seed_postgres(planning: &PathBuf, url: &str) {
    let report = MarkdownStore::open(planning).load();
    assert!(report.is_clean());
    let graph = report.graph().expect("legacy graph");
    let backend = aep_backend_postgres::PostgresBackend::connect(url).expect("PostgreSQL source");
    seed::from_manifest(
        &backend,
        &graph,
        aep_backend_markdown::backend::ORGANISATION,
        aep_backend_markdown::backend::SPACE,
        Timestamp::from_epoch_millis(1_700_000_000_000),
        &ActorRef::parse("human:writer-control-fixture").expect("actor"),
    )
    .expect("PostgreSQL source seeded");
}

fn apply_selected_sql_source(root: &PathBuf, selector: &PathBuf, migration: &str) {
    let history_args = [
        "plan",
        "artifact",
        "history",
        "story:one",
        "--format",
        "json",
    ];
    let (success, history_before, error) = run(root, &history_args);
    assert!(success, "source history: {history_before} {error}");
    apply_selected_source(root, selector, migration);
    let (success, history_after, error) = run(root, &history_args);
    assert!(success, "selected history: {history_after} {error}");
    assert_eq!(
        history_after, history_before,
        "ordinary history survives migration"
    );
}

fn apply_selected_source(root: &PathBuf, selector: &PathBuf, migration: &str) {
    let (success, applied, error) = run_selected_source_apply(root, selector, migration);
    assert!(success, "SQL public apply: {applied} {error}");
    assert!(applied["outcome"]["value"]["receipt"].is_object());
    let selector_text = selector.to_string_lossy().into_owned();
    let verify = [
        "plan",
        "store",
        "verify",
        "--project",
        &selector_text,
        "--format",
        "json",
    ];
    let (success, verified, error) = run(root, &verify);
    assert!(success, "SQL selected verify: {verified} {error}");
}

fn run_selected_source_apply(
    root: &PathBuf,
    selector: &PathBuf,
    migration: &str,
) -> (bool, serde_json::Value, String) {
    let selector_text = selector.to_string_lossy().into_owned();
    let dry = [
        "plan",
        "store",
        "migrate",
        "dry-run",
        "--project",
        &selector_text,
        "--authority-scope",
        "control-sql",
        "--authority-tenant",
        "control-sql-tenant",
        "--authority-new",
        "--format",
        "json",
    ];
    let (success, preview, error) = run(root, &dry);
    assert!(success, "SQL dry-run: {preview} {error}");
    let snapshot = preview["outcome"]["value"]["source_snapshot"]
        .as_str()
        .expect("SQL source snapshot");
    let apply = [
        "plan",
        "store",
        "migrate",
        "apply",
        "--project",
        &selector_text,
        "--authority-scope",
        "control-sql",
        "--authority-tenant",
        "control-sql-tenant",
        "--authority-new",
        "--snapshot",
        snapshot,
        "--migration",
        migration,
        "--format",
        "json",
    ];
    let (mut holder, _output) = hold(
        root,
        selector,
        &["--migration", migration, "--snapshot", snapshot],
        false,
        None,
    );
    let outcome = run(root, &apply);
    stop_holder(&mut holder);
    outcome
}

#[test]
fn public_markdown_migration_preserves_original_journal_history() {
    let (root, selector) = project();
    let entry = serde_json::json!({
        "at": "2026-01-01T00:00:00Z", "actor": "human:fixture",
        "artifact": "story:one", "kind": "story", "revision": 1,
        "change": {"change": "created", "status": "draft"}
    });
    let old_evidence = serde_json::json!({
        "at": "2026-01-01T00:00:01Z", "actor": "human:fixture",
        "artifact": "story:one", "kind": "story", "revision": 1,
        "change": {"change": "evidence", "kind": "test_result", "source": "original-check"}
    });
    fs::write(
        root.join(".engineering/planning/journal.jsonl"),
        format!("{entry}\n{old_evidence}\n"),
    )
    .expect("original journal entry");
    // A real legacy write appends the newer DomainEvent shape. Its caller-supplied instant is
    // earlier: source line order, not timestamp sorting, is the order that must survive.
    let evidence = [
        "plan",
        "artifact",
        "evidence",
        "story:one",
        "--kind",
        "test_result",
        "--source",
        "event-check",
        "--at",
        "2025-12-31T23:59:59Z",
        "--format",
        "json",
    ];
    let (success, recorded, error) = run(&root, &evidence);
    assert!(success, "legacy event: {recorded} {error}");
    let history = [
        "plan",
        "artifact",
        "history",
        "story:one",
        "--format",
        "json",
    ];
    let (success, before, error) = run(&root, &history);
    assert!(success, "mixed history: {before} {error}");
    assert_eq!(before.as_array().expect("history entries").len(), 3);
    apply_selected_sql_source(&root, &selector, "control-history-migration");
    let (success, explained, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "explain",
            "story:one",
            "--format",
            "json",
        ],
    );
    assert!(success, "explanation: {explained} {error}");
    assert_eq!(
        explained["recorded_since"]
            .as_array()
            .expect("retained evidence")
            .len(),
        2
    );
    let suffix = evidence.map(|part| {
        if part == "event-check" {
            "new-check"
        } else {
            part
        }
    });
    let (success, recorded, error) = run(&root, &suffix);
    assert!(success, "new recorded suffix: {recorded} {error}");
    let (success, after, error) = run(&root, &history);
    assert!(success, "history after reopen: {after} {error}");
    let after = after.as_array().expect("complete history");
    let before = before.as_array().expect("original history");
    assert_eq!(&after[..before.len()], before);
    assert_eq!(after.len(), before.len() + 1, "suffix appended once");
    assert_eq!(
        after.last().expect("new entry")["change"]["source"],
        "new-check"
    );
    repair_committed_projection_failure(&root, &suffix, before);
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

fn repair_committed_projection_failure(
    root: &PathBuf,
    suffix: &[&str],
    before: &[serde_json::Value],
) {
    // A later evidence write advances physical storage again without advancing the artifact.
    // A foreign projection conflict must report its durable commit and permit the same retry.
    let foreign = root.join(".engineering/planning/foreign.md");
    fs::write(&foreign, "---\nformat: unknown\n---\n").expect("foreign projection conflict");
    let mut retry = suffix
        .iter()
        .copied()
        .map(|part| {
            if part == "new-check" {
                "recovery-check"
            } else {
                part
            }
        })
        .collect::<Vec<_>>();
    retry.extend(["--command-identity", "history-projection-recovery"]);
    let (success, failed, error) = run(root, &retry);
    assert!(!success, "projection conflict must fail: {failed} {error}");
    assert!(
        failed.to_string().contains("committed_projection_failure"),
        "{failed}"
    );
    fs::remove_file(foreign).expect("operator resolves the foreign conflict");
    let (success, repaired, error) = run(root, &retry);
    assert!(
        success,
        "same command repairs projection: {repaired} {error}"
    );
    let (success, repaired_again, error) = run(root, &retry);
    assert!(success, "completed retry: {repaired_again} {error}");
    assert_eq!(repaired_again, repaired);
    let history = [
        "plan",
        "artifact",
        "history",
        "story:one",
        "--format",
        "json",
    ];
    let (success, final_history, error) = run(root, &history);
    assert!(success, "history after repair: {final_history} {error}");
    let final_history = final_history.as_array().expect("complete repaired history");
    assert_eq!(&final_history[..before.len()], before);
    assert_eq!(
        final_history.len(),
        before.len() + 2,
        "retries append nothing"
    );
    assert_eq!(final_history.last().expect("evidence entry")["revision"], 1);
    // A semantic change after evidence must advance the artifact once, independently of the
    // wrapper revisions spent recording evidence and repairing the projection.
    let body = root.join("new-body.md");
    fs::write(&body, "Preserved history, updated body.\n").expect("new body source");
    let (success, changed, error) = run(
        root,
        &[
            "plan",
            "artifact",
            "body",
            "story:one",
            "--from",
            body.to_str().expect("test path"),
            "--format",
            "json",
        ],
    );
    assert!(
        success,
        "semantic mutation after evidence: {changed} {error}"
    );
    let (success, changed_history, error) = run(root, &history);
    assert!(
        success,
        "history after semantic mutation: {changed_history} {error}"
    );
    let changed_history = changed_history.as_array().expect("history entries");
    assert_eq!(&changed_history[..before.len()], before);
    assert_eq!(changed_history.len(), before.len() + 3);
    assert_eq!(changed_history.last().expect("body entry")["revision"], 2);
}

#[test]
fn sqlite_projection_refuses_unowned_valid_markdown_before_migration_writes() {
    let (root, selector) = project();
    let engineering = root.join(".engineering");
    let planning = engineering.join("planning");
    let story = planning.join("story/one.md");
    let original_story = fs::read(&story).expect("unowned valid Markdown fixture");
    seed_sqlite(&planning, &engineering.join("plan.sqlite3"));
    let original_selector = b"{\"protocol\":\"adp/1\",\"profile\":\"development.standard\",\"protocols\":\"../protocols\",\"store\":{\"sqlite\":\"plan.sqlite3\"}}\n";
    fs::write(&selector, original_selector).expect("SQLite selector");
    let selector_text = selector.to_string_lossy().into_owned();
    let dry = [
        "plan",
        "store",
        "migrate",
        "dry-run",
        "--project",
        &selector_text,
        "--authority-scope",
        "control-sql",
        "--authority-tenant",
        "control-sql-tenant",
        "--authority-new",
        "--format",
        "json",
    ];
    let (success, preview, error) = run(&root, &dry);
    assert!(
        success,
        "SQL dry-run reports the collision: {preview} {error}"
    );
    assert_eq!(
        preview["outcome"]["value"]["destination_requirements"]["foreign_content"]
            .as_array()
            .expect("foreign paths")
            .len(),
        1
    );

    let (success, refused, error) =
        run_selected_source_apply(&root, &selector, "control-sqlite-foreign-projection");
    assert!(!success, "unowned collision must refuse: {refused} {error}");
    assert_eq!(
        fs::read(&selector).expect("selector survives"),
        original_selector
    );
    assert_eq!(
        fs::read(&story).expect("foreign story survives"),
        original_story
    );
    assert!(
        !engineering.join("migrations").exists(),
        "preflight refusal must not create durable migration evidence"
    );
    fs::remove_dir_all(root).expect("remove disposable collision fixture");
}

#[test]
fn public_sqlite_source_applies_under_observed_writer_stop() {
    let (root, selector) = project();
    let engineering = root.join(".engineering");
    let planning = engineering.join("planning");
    seed_sqlite(&planning, &engineering.join("plan.sqlite3"));
    fs::remove_dir_all(planning).expect("positive SQL projection begins unoccupied");
    fs::write(&selector,
        "{\"protocol\":\"adp/1\",\"profile\":\"development.standard\",\"protocols\":\"../protocols\",\"store\":{\"sqlite\":\"plan.sqlite3\"}}\n")
        .expect("SQLite selector");
    apply_selected_sql_source(&root, &selector, "control-sqlite-migration");
}

#[test]
#[allow(clippy::too_many_lines)] // One public journey compares the selected prefix, losing retained evidence, and recorded suffix.
fn replica_authoritative_hybrid_keeps_only_selected_history_and_appends_recorded_suffix() {
    let (root, selector) = project();
    let engineering = root.join(".engineering");
    let planning = engineering.join("planning");
    let database = engineering.join("plan.sqlite3");
    seed_sqlite(&planning, &database);
    let replica_id = align_replica_id_with_markdown_mapping(&database);
    let replica = aep_backend_sqlite::SqliteBackend::open(&database).expect("replica opens");
    let selected_before = replica
        .as_entity_backend()
        .events_of(&replica_id)
        .expect("replica-selected history before migration");
    assert!(
        !selected_before.is_empty(),
        "replica history fixture is nonempty"
    );
    let duplicate = serde_json::to_vec(&serde_json::json!({
        "at": "2023-11-14T22:13:20Z",
        "actor": "human:writer-control-fixture",
        "artifact": "story:one",
        "kind": "story",
        "revision": 1,
        "change": {"change": "created", "status": "draft"}
    }))
    .expect("duplicate local entry serialises");
    let conflicting = serde_json::to_vec(&serde_json::json!({
        "at": "2026-01-01T00:00:01Z",
        "actor": "human:local-losing-copy",
        "artifact": "story:one",
        "kind": "story",
        "revision": 1,
        "change": {"change": "evidence", "kind": "test_result", "source": "local-only"}
    }))
    .expect("conflicting local entry serialises");
    fs::write(
        planning.join("journal.jsonl"),
        [duplicate.as_slice(), b"\n", conflicting.as_slice(), b"\n"].concat(),
    )
    .expect("losing local history fixture");
    fs::write(
        &selector,
        "{\"protocol\":\"adp/1\",\"profile\":\"development.standard\",\"protocols\":\"../protocols\",\"store\":{\"hybrid\":{\"authority\":\"replica\",\"read\":\"replica-first\",\"on_unreachable\":\"refuse\",\"on_divergence\":\"record\",\"local\":\"markdown\",\"replica\":{\"sqlite\":\"plan.sqlite3\"}}}}\n",
    )
    .expect("replica-authoritative hybrid selector");
    let history = [
        "plan",
        "artifact",
        "history",
        "story:one",
        "--format",
        "json",
    ];
    apply_selected_source(&root, &selector, "control-hybrid-replica-migration");

    let (success, migrated, error) = run(&root, &history);
    assert!(
        success,
        "selected history after migration: {migrated} {error}"
    );
    let migrated = migrated.as_array().expect("migrated selected history");
    assert_eq!(
        migrated.len(),
        selected_before.len(),
        "losing local entries must not be prepended or duplicate replica history"
    );
    assert!(
        !migrated
            .iter()
            .any(|entry| entry.to_string().contains("local-only")),
        "losing local detail became public history: {migrated:?}"
    );

    let evidence = [
        "plan",
        "artifact",
        "evidence",
        "story:one",
        "--kind",
        "test_result",
        "--source",
        "post-migration",
        "--at",
        "2026-01-02T00:00:00Z",
        "--format",
        "json",
    ];
    let (success, recorded, error) = run(&root, &evidence);
    assert!(success, "post-migration suffix records: {recorded} {error}");
    let (success, selected_after, error) = run(&root, &history);
    assert!(
        success,
        "replica-selected history after suffix: {selected_after} {error}"
    );
    let selected_after = selected_after.as_array().expect("selected history");
    assert_eq!(
        &selected_after[..migrated.len()],
        migrated,
        "the recorded suffix must follow the selected replica history"
    );
    assert_eq!(selected_after.len(), migrated.len() + 1);
    assert_eq!(
        selected_after.last().expect("recorded suffix")["change"]["source"],
        "post-migration"
    );
    fs::remove_dir_all(root).expect("remove disposable hybrid fixture");
}

struct PgCleanup {
    root: PathBuf,
    url: String,
    schema: String,
}

impl Drop for PgCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
        if let Ok(mut client) = postgres::Client::connect(&self.url, postgres::NoTls) {
            let _ = client.batch_execute(&format!("DROP SCHEMA {} CASCADE", self.schema));
        }
    }
}

#[test]
fn public_postgres_source_applies_under_observed_writer_stop_when_configured() {
    let Ok(url) = std::env::var("ENTITY_POSTGRES_URL") else {
        return;
    };
    let (root, selector) = project();
    let schema = format!(
        "aep_control_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let cleanup = PgCleanup {
        root: root.clone(),
        url: url.clone(),
        schema: schema.clone(),
    };
    let mut client =
        postgres::Client::connect(&url, postgres::NoTls).expect("disposable PostgreSQL server");
    client
        .batch_execute(&format!("CREATE SCHEMA {schema}"))
        .expect("isolated SQL namespace");
    let separator = if url.contains('?') { '&' } else { '?' };
    let scoped_url = format!("{url}{separator}options=-csearch_path%3D{schema}");
    let planning = root.join(".engineering/planning");
    seed_postgres(&planning, &scoped_url);
    fs::remove_dir_all(planning).expect("positive PostgreSQL projection begins unoccupied");
    let config = serde_json::json!({
        "protocol":"adp/1", "profile":"development.standard", "protocols":"../protocols",
        "store":{"postgres":scoped_url}
    });
    fs::write(&selector, format!("{config}\n")).expect("PostgreSQL selector");
    apply_selected_sql_source(&root, &selector, "control-postgres-migration");
    drop(cleanup);
}

#[test]
#[allow(clippy::too_many_lines)]
fn public_apply_requires_live_custody_and_preserves_original_retry() {
    let (root, selector) = project();
    let planning = root.join(".engineering/planning");
    fs::write(
        planning.join("story/one.md"),
        "---\nformat: aep.planning-md/1\nid: story:one\nkind: story\nstatus: draft\ntitle: One\nrelations:\n- depends_on: story:two\nrevision: 1\n---\n",
    )
    .expect("legacy story relates to the second owned artifact");
    fs::write(
        planning.join("story/two.md"),
        "---\nformat: aep.planning-md/1\nid: story:two\nkind: story\nstatus: draft\ntitle: Two\nrelations: []\nrevision: 1\n---\n",
    )
    .expect("second legacy story");
    let selector_text = selector.to_string_lossy().into_owned();
    let dry_args = [
        "plan",
        "store",
        "migrate",
        "dry-run",
        "--project",
        &selector_text,
        "--authority-scope",
        "control-test",
        "--authority-tenant",
        "control-tenant",
        "--authority-new",
        "--format",
        "json",
    ];
    let (success, dry, error) = run(&root, &dry_args);
    assert!(success, "dry-run: {dry} {error}");
    let snapshot = dry["outcome"]["value"]["source_snapshot"]
        .as_str()
        .expect("dry-run names source snapshot")
        .to_owned();
    let apply = [
        "plan",
        "store",
        "migrate",
        "apply",
        "--project",
        &selector_text,
        "--authority-scope",
        "control-test",
        "--authority-tenant",
        "control-tenant",
        "--authority-new",
        "--snapshot",
        &snapshot,
        "--migration",
        "control-migration",
        "--format",
        "json",
    ];
    let (success, refused, _) = run(&root, &apply);
    assert!(
        !success && refused.to_string().contains("writer_exclusion_unavailable"),
        "absent control: {refused}"
    );
    let unsupported_format = Command::new(env!("CARGO_BIN_EXE_aep"))
        .args([
            "plan",
            "store",
            "writer-control",
            "hold",
            "--project",
            &selector_text,
            "--migration",
            "control-migration",
            "--snapshot",
            &snapshot,
            "--writer-pid",
            &std::process::id().to_string(),
            "--format",
            "json",
        ])
        .output()
        .expect("interactive format request runs");
    assert!(!unsupported_format.status.success());
    assert!(
        String::from_utf8_lossy(&unsupported_format.stderr).contains("supports only --format text")
    );

    let (mut holder, _output) = hold(
        &root,
        &selector,
        &["--migration", "control-migration", "--snapshot", &snapshot],
        false,
        Some(&apply),
    );
    let original_selector = fs::read(&selector).expect("original selector");
    let mut changed_selector = original_selector.clone();
    changed_selector.extend_from_slice(b"# changed configuration bytes\n");
    fs::write(&selector, changed_selector).expect("different config bytes");
    let (success, refused, _) = run(&root, &apply);
    assert!(
        !success && refused.to_string().contains("writer_exclusion_unavailable"),
        "wrong config binding: {refused}"
    );
    fs::write(&selector, "{\"protocol\":\"adp/1\",\"profile\":\"development.standard\",\"protocols\":\"../protocols\",\"store\":{\"sqlite\":\"other.sqlite\"}}\n")
        .expect("substituted source selector");
    let (success, refused, _) = run(&root, &apply);
    assert!(
        !success && refused.to_string().contains("writer_exclusion_unavailable"),
        "wrong source binding: {refused}"
    );
    fs::write(&selector, &original_selector).expect("restore selected config");
    let wrong_migration = apply.map(|part| {
        if part == "control-migration" {
            "another-migration"
        } else {
            part
        }
    });
    let (success, refused, _) = run(&root, &wrong_migration);
    assert!(
        !success && refused.to_string().contains("writer_exclusion_unavailable"),
        "wrong migration binding: {refused}"
    );
    let wrong_snapshot = format!(
        "{}{}",
        &snapshot[..snapshot.len() - 1],
        if snapshot.ends_with('0') { '1' } else { '0' }
    );
    let wrong_snapshot_args = apply.map(|part| {
        if part == snapshot {
            &*wrong_snapshot
        } else {
            part
        }
    });
    let (success, refused, _) = run(&root, &wrong_snapshot_args);
    assert!(
        !success && refused.to_string().contains("writer_exclusion_unavailable"),
        "wrong snapshot binding: {refused}"
    );
    let (success, applied, error) = run(&root, &apply);
    assert!(success, "apply: {applied} {error}");
    let receipt = applied["outcome"]["value"]["receipt"].clone();
    assert!(
        receipt.is_object(),
        "complete result has original receipt: {applied}"
    );
    let (success, retry, error) = run(&root, &apply);
    assert!(success, "matching retry: {retry} {error}");
    assert_eq!(retry["outcome"]["value"]["receipt"], receipt);

    holder.kill().expect("simulate holder crash");
    holder.wait().expect("crashed holder reaped");
    let (success, refused, _) = run(&root, &apply);
    assert!(
        !success && refused.to_string().contains("writer_exclusion_unavailable"),
        "lost control: {refused}"
    );

    let (mut resumed, _output) = hold(
        &root,
        &selector,
        &["--migration", "control-migration", "--snapshot", &snapshot],
        true,
        None,
    );
    let (success, retry, error) = run(&root, &apply);
    assert!(success, "reacquired custody: {retry} {error}");
    assert_eq!(retry["outcome"]["value"]["receipt"], receipt);
    stop_holder(&mut resumed);

    let verify = [
        "plan",
        "store",
        "verify",
        "--project",
        &selector_text,
        "--format",
        "json",
    ];
    let (success, verified, error) = run(&root, &verify);
    assert!(success, "verify: {verified} {error}");
    assert!(verified.to_string().contains("verified"));

    let authority_snapshot = verified["outcome"]["value"]["authority"]["snapshot_id"]
        .as_str()
        .expect("verified authority snapshot")
        .to_owned();
    let projected_two = planning.join("story/two.md");
    let authoritative_two = fs::read(&projected_two).expect("second authoritative projection");
    assert!(
        fs::read_to_string(planning.join("story/one.md"))
            .expect("first authoritative projection")
            .contains("depends_on: story:two"),
        "the remaining owned document must reference the deleted artifact"
    );
    let history_args = [
        "plan",
        "artifact",
        "history",
        "story:two",
        "--format",
        "json",
    ];
    let (success, history_before_rebuild, error) = run(&root, &history_args);
    assert!(
        success,
        "history before rebuild: {history_before_rebuild} {error}"
    );
    fs::remove_file(&projected_two).expect("delete exactly one owned projection");
    let (success, drifted, _) = run(&root, &verify);
    assert!(
        !success && drifted.to_string().contains("projection_drift"),
        "ordinary verification must report the missing owned file as drift: {drifted}"
    );
    let rebuild_args = [
        "plan",
        "store",
        "rebuild",
        "--project",
        &selector_text,
        "--authority-snapshot",
        &authority_snapshot,
        "--format",
        "json",
    ];
    let (success, refused, _) = run(&root, &rebuild_args);
    assert!(
        !success && refused.to_string().contains("writer_exclusion_unavailable"),
        "rebuild needs current custody: {refused}"
    );
    let (mut rebuild_holder, _output) = hold(
        &root,
        &selector,
        &["--authority-snapshot", &authority_snapshot],
        false,
        Some(&rebuild_args),
    );
    let (success, rebuilt, error) = run(&root, &rebuild_args);
    assert!(success, "rebuild: {rebuilt} {error}");
    assert!(rebuilt.to_string().contains("rebuilt"));
    stop_holder(&mut rebuild_holder);
    assert_eq!(
        fs::read(&projected_two).expect("missing owned projection is restored"),
        authoritative_two
    );
    let (success, verified_after_rebuild, error) = run(&root, &verify);
    assert!(
        success,
        "verification after partial projection rebuild: {verified_after_rebuild} {error}"
    );
    assert_eq!(
        verified_after_rebuild["outcome"]["value"]["projection"]["authority_snapshot"],
        authority_snapshot,
        "rebuild must project the exact requested authority snapshot"
    );
    assert_eq!(
        verified_after_rebuild["outcome"]["value"]["projection"]["drift"],
        "current"
    );
    let (success, history_after_rebuild, error) = run(&root, &history_args);
    assert!(
        success,
        "history after rebuild: {history_after_rebuild} {error}"
    );
    assert_eq!(
        history_after_rebuild, history_before_rebuild,
        "projection recovery must preserve authoritative history"
    );

    let explicit = Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(["plan", "artifact", "list", "--store"])
        .arg(root.join(".engineering/planning"))
        .current_dir(std::env::temp_dir())
        .output()
        .expect("explicit legacy reader runs");
    assert!(
        !explicit.status.success(),
        "retired source must not reopen as legacy authority"
    );
    assert!(
        String::from_utf8_lossy(&explicit.stderr).contains("Eventlog projection"),
        "legacy locator names why the selected projection is retired"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn public_already_idle_custody_applies_retries_and_rebuilds_without_stop_witness() {
    let (root, selector) = project();
    let selector_text = selector.to_string_lossy().into_owned();
    let dry = [
        "plan",
        "store",
        "migrate",
        "dry-run",
        "--project",
        &selector_text,
        "--authority-scope",
        "idle-control-test",
        "--authority-tenant",
        "idle-control-tenant",
        "--authority-new",
        "--format",
        "json",
    ];
    let (success, preview, error) = run(&root, &dry);
    assert!(success, "idle dry-run: {preview} {error}");
    let snapshot = preview["outcome"]["value"]["source_snapshot"]
        .as_str()
        .expect("idle source snapshot")
        .to_owned();
    let identity = ["--migration", "idle-migration", "--snapshot", &snapshot];
    let apply = [
        "plan",
        "store",
        "migrate",
        "apply",
        "--project",
        &selector_text,
        "--authority-scope",
        "idle-control-test",
        "--authority-tenant",
        "idle-control-tenant",
        "--authority-new",
        "--snapshot",
        &snapshot,
        "--migration",
        "idle-migration",
        "--format",
        "json",
    ];

    let omitted = holder_output(&root, &selector, &identity, &[]);
    assert!(
        !omitted.status.success(),
        "omitting every writer-control mode must refuse"
    );
    let conflict = holder_output(
        &root,
        &selector,
        &identity,
        &["--already-idle", "--resume-stop"],
    );
    assert!(
        !conflict.status.success(),
        "already-idle and resume-stop must conflict"
    );
    let current_pid = std::process::id().to_string();
    let conflict = holder_output(
        &root,
        &selector,
        &identity,
        &["--already-idle", "--writer-pid", &current_pid],
    );
    assert!(
        !conflict.status.success(),
        "already-idle and observed writer PIDs must conflict"
    );

    let mut unconfirmed = Command::new(env!("CARGO_BIN_EXE_aep"));
    unconfirmed
        .args(["plan", "store", "writer-control", "hold", "--project"])
        .arg(&selector)
        .args(identity)
        .arg("--already-idle")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(&root);
    let mut unconfirmed = unconfirmed.spawn().expect("unconfirmed idle holder starts");
    let mut output = BufReader::new(unconfirmed.stdout.take().expect("unconfirmed stdout"));
    let mut line = String::new();
    output.read_line(&mut line).expect("idle assertion");
    assert!(line.contains("Accepted already-idle assertion"), "{line}");
    line.clear();
    output.read_line(&mut line).expect("idle custody prompt");
    assert!(line.contains("Type HOLD"), "{line}");
    unconfirmed
        .stdin
        .as_mut()
        .expect("unconfirmed stdin")
        .write_all(b"NO\n")
        .expect("decline custody");
    assert!(!unconfirmed
        .wait()
        .expect("unconfirmed holder exits")
        .success());

    let (mut holder, _output) = hold_already_idle(&root, &selector, &identity);
    let wrong_migration = apply.map(|part| {
        if part == "idle-migration" {
            "other-idle-migration"
        } else {
            part
        }
    });
    let (success, refused, _) = run(&root, &wrong_migration);
    assert!(
        !success && refused.to_string().contains("writer_exclusion_unavailable"),
        "wrong idle migration binding: {refused}"
    );
    let wrong_snapshot = format!(
        "{}{}",
        &snapshot[..snapshot.len() - 1],
        if snapshot.ends_with('0') { '1' } else { '0' }
    );
    let wrong_snapshot_args = apply.map(|part| {
        if part == snapshot {
            &*wrong_snapshot
        } else {
            part
        }
    });
    let (success, refused, _) = run(&root, &wrong_snapshot_args);
    assert!(
        !success && refused.to_string().contains("writer_exclusion_unavailable"),
        "wrong idle source snapshot: {refused}"
    );
    let original_selector = fs::read(&selector).expect("idle selector");
    let mut changed_selector = original_selector.clone();
    changed_selector.extend_from_slice(b"# changed while idle custody is held\n");
    fs::write(&selector, changed_selector).expect("changed idle selector");
    let (success, refused, _) = run(&root, &apply);
    assert!(
        !success && refused.to_string().contains("writer_exclusion_unavailable"),
        "changed idle selector: {refused}"
    );
    fs::write(&selector, original_selector).expect("restore idle selector");

    let (success, applied, error) = run(&root, &apply);
    assert!(success, "already-idle apply: {applied} {error}");
    let receipt = applied["outcome"]["value"]["receipt"].clone();
    let (success, retry, error) = run(&root, &apply);
    assert!(success, "already-idle matching retry: {retry} {error}");
    assert_eq!(retry["outcome"]["value"]["receipt"], receipt);

    holder.kill().expect("simulate already-idle holder crash");
    holder.wait().expect("already-idle holder reaped");
    let (success, refused, _) = run(&root, &apply);
    assert!(
        !success && refused.to_string().contains("writer_exclusion_unavailable"),
        "dead already-idle holder: {refused}"
    );
    let resume = holder_output(&root, &selector, &identity, &["--resume-stop"]);
    assert!(
        !resume.status.success(),
        "idle custody must not mint stop evidence"
    );
    assert!(
        String::from_utf8_lossy(&resume.stderr).contains("no observed stop witness"),
        "idle custody must not be resumable as observed-stop evidence: {}",
        String::from_utf8_lossy(&resume.stderr)
    );

    let (mut reacquired, _output) = hold_already_idle(&root, &selector, &identity);
    let (success, retry, error) = run(&root, &apply);
    assert!(success, "fresh idle custody retry: {retry} {error}");
    assert_eq!(retry["outcome"]["value"]["receipt"], receipt);
    stop_holder(&mut reacquired);

    let verify = [
        "plan",
        "store",
        "verify",
        "--project",
        &selector_text,
        "--format",
        "json",
    ];
    let (success, verified, error) = run(&root, &verify);
    assert!(success, "idle verify: {verified} {error}");
    let authority_snapshot = verified["outcome"]["value"]["authority"]["snapshot_id"]
        .as_str()
        .expect("idle authority snapshot")
        .to_owned();
    let projected = root.join(".engineering/planning/story/one.md");
    let expected = fs::read(&projected).expect("idle projection");
    fs::remove_file(&projected).expect("remove idle projection");
    let rebuild_identity = ["--authority-snapshot", &authority_snapshot];
    let rebuild_args = [
        "plan",
        "store",
        "rebuild",
        "--project",
        &selector_text,
        "--authority-snapshot",
        &authority_snapshot,
        "--format",
        "json",
    ];
    let (mut rebuild_holder, _output) = hold_already_idle(&root, &selector, &rebuild_identity);
    let (success, rebuilt, error) = run(&root, &rebuild_args);
    assert!(success, "already-idle rebuild: {rebuilt} {error}");
    stop_holder(&mut rebuild_holder);
    assert_eq!(
        fs::read(&projected).expect("idle projection rebuilt"),
        expected
    );
    let (success, verified, error) = run(&root, &verify);
    assert!(
        success,
        "verification after idle rebuild: {verified} {error}"
    );
    fs::remove_dir_all(root).expect("remove already-idle fixture");
}

/// A legacy Markdown plan with one story, migrated through the public path, then moved once
/// through the public command path.
///
/// The fixture for `validate` on an Eventlog plan. The legacy journal is written in both shapes
/// the projection can carry — the journal's own `created` entry and the `DomainEvent` a real
/// legacy write appends — because `drift::detect` reads the second per artifact and the journal's
/// reconciliation reads the first. After migration that file is the migrated store's record,
/// frozen where the migration left it; the move is recorded by the authority alone.
fn migrated_eventlog_plan_after_one_governed_move(migration: &str) -> (PathBuf, PathBuf) {
    let (root, selector) = project();
    let created = serde_json::json!({
        "at": "2026-01-01T00:00:00Z", "actor": "human:fixture",
        "artifact": "story:one", "kind": "story", "revision": 1,
        "change": {"change": "created", "status": "draft"}
    });
    fs::write(
        root.join(".engineering/planning/journal.jsonl"),
        format!("{created}\n"),
    )
    .expect("legacy journal");
    let (success, recorded, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "evidence",
            "story:one",
            "--kind",
            "test_result",
            "--source",
            "legacy-check",
            "--format",
            "json",
        ],
    );
    assert!(success, "legacy evidence write: {recorded} {error}");
    apply_selected_source(&root, &selector, migration);
    let validate = ["plan", "artifact", "validate", "--format", "json"];
    let (success, before, error) = run(&root, &validate);
    assert!(success, "validate right after migration: {before} {error}");
    assert_eq!(before["problems"], serde_json::json!([]), "{before}");
    let identity = format!("{migration}-move");
    let (success, moved, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "move",
            "story:one",
            "--to",
            "proposed",
            "--command-identity",
            &identity,
            "--format",
            "json",
        ],
    );
    assert!(
        success,
        "governed move on the Eventlog plan: {moved} {error}"
    );
    assert_eq!(moved["format"], "aep.planning-mutation/1", "{moved}");
    assert_eq!(moved["outcome"]["kind"], "complete", "{moved}");
    (root, selector)
}

#[test]
fn validate_answers_a_migrated_eventlog_plan_from_its_authority_not_the_frozen_legacy_journal() {
    let (root, selector) =
        migrated_eventlog_plan_after_one_governed_move("validate-v2-governed-move");
    let selector_text = selector.to_string_lossy().into_owned();
    let verify = [
        "plan",
        "store",
        "verify",
        "--project",
        &selector_text,
        "--format",
        "json",
    ];
    let (success, verified, error) = run(&root, &verify);
    assert!(success, "verify after the move: {verified} {error}");
    assert_eq!(
        verified["outcome"]["value"]["projection"]["drift"], "current",
        "{verified}"
    );
    let (success, after, error) = run(&root, &["plan", "artifact", "validate", "--format", "json"]);
    assert!(
        success,
        "validate called a governed move a hand edit: {after} {error}"
    );
    assert_eq!(after["problems"], serde_json::json!([]), "{after}");
    assert!(after.get("drift").is_none(), "{after}");
    assert!(after.get("forged").is_none(), "{after}");
    assert_eq!(after["pre_provider"], 0, "{after}");
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

#[test]
fn validate_reports_a_hand_edited_eventlog_projection_as_drift_decided_by_the_authority() {
    let (root, selector) = migrated_eventlog_plan_after_one_governed_move("validate-v2-hand-edit");
    let projected = root.join(".engineering/planning/story/one.md");
    let text = fs::read_to_string(&projected).expect("projected story");
    assert!(text.contains("status: proposed"), "{text}");
    fs::write(
        &projected,
        text.replace("status: proposed", "status: active"),
    )
    .expect("an edit made outside a command");
    let (success, edited, error) =
        run(&root, &["plan", "artifact", "validate", "--format", "json"]);
    assert!(
        !success,
        "a hand-edited projection passed validate: {edited} {error}"
    );
    let drift = edited["drift"].as_array().expect("drift findings");
    assert_eq!(drift.len(), 1, "{edited}");
    let finding = drift[0].as_str().expect("a finding is text");
    assert!(
        finding.contains("drifted from its authority") && finding.contains("plan store verify"),
        "{finding}"
    );
    assert_eq!(
        edited["problems"].as_array().expect("problems").len(),
        1,
        "{edited}"
    );
    assert!(edited.get("forged").is_none(), "{edited}");
    let selector_text = selector.to_string_lossy().into_owned();
    let (success, verified, error) = run(
        &root,
        &[
            "plan",
            "store",
            "verify",
            "--project",
            &selector_text,
            "--format",
            "json",
        ],
    );
    assert!(
        !success,
        "verify agrees the projection drifted: {verified} {error}"
    );
    assert!(
        verified.to_string().contains("projection_drift"),
        "{verified}"
    );
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// The same migrated plan, with one review recorded **before** the migration and one after it.
///
/// The frozen journal under the projection holds the first review's creation and can hold nothing
/// about the second, so a reader that takes its order from that file holds a *partial* order over
/// the two — and under `Option`'s ordering the review it says nothing about sorts first, which is
/// the reverse of when the two rounds happened.
fn migrated_eventlog_plan_with_a_review_from_before_the_migration(migration: &str) -> PathBuf {
    let (root, selector) = project();
    let created = serde_json::json!({
        "at": "2026-01-01T00:00:00Z", "actor": "human:fixture",
        "artifact": "story:one", "kind": "story", "revision": 1,
        "change": {"change": "created", "status": "draft"}
    });
    fs::write(
        root.join(".engineering/planning/journal.jsonl"),
        format!("{created}\n"),
    )
    .expect("legacy journal");
    let (success, first, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "new",
            "review-result",
            "alpha",
            "--title",
            "First round",
            "--owner",
            "agent:alpha",
            "--relate",
            "reviews:story:one",
            "--format",
            "json",
        ],
    );
    assert!(
        success,
        "a review recorded before the migration: {first} {error}"
    );
    apply_selected_source(&root, &selector, migration);
    let identity = format!("{migration}-second-round");
    let (success, second, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "new",
            "review-result",
            "beta",
            "--title",
            "Second round",
            "--owner",
            "agent:beta",
            "--relate",
            "reviews:story:one",
            "--command-identity",
            &identity,
            "--format",
            "json",
        ],
    );
    assert!(
        success,
        "a review recorded after the migration: {second} {error}"
    );
    root
}

/// **An evidence-gated move on a migrated plan counts the records the authority holds.**
///
/// `Opened::evidence_on_hand` matched on the projected files, so on an Eventlog plan it counted
/// the journal under the projection: the migrated store's, frozen where the migration left it.
/// Every record written after the cut-over was invisible to the rung that asked for it, so the
/// first evidence-gated move on a migrated store was refused by the store's own reader while the
/// authority held the record the rung wanted.
#[test]
#[allow(clippy::too_many_lines)] // One journey: the rung, the record, the move, and what it rested on.
fn an_evidence_gated_move_on_a_migrated_plan_counts_evidence_recorded_after_the_migration() {
    let (root, _selector) =
        migrated_eventlog_plan_after_one_governed_move("evidence-v2-gated-move");
    // A rung that asks for something the frozen journal cannot hold: the fixture recorded one
    // `test_result` before the migration and never an `approval`, so the approval recorded below
    // is a record that exists in the authority and nowhere else.
    fs::create_dir_all(root.join("protocols/artifacts/lifecycles")).expect("lifecycle directory");
    fs::write(
        root.join("protocols/artifacts/lifecycles/story.yaml"),
        "kind: story\n\
         initial: draft\n\
         transitions:\n  \
           draft: [proposed]\n  \
           proposed: [signed]\n  \
           signed: []\n\
         requires:\n  \
           signed:\n    \
             - evidence: approval\n      \
               at_least: 1\n",
    )
    .expect("a guarded rung");

    let explain = [
        "plan",
        "artifact",
        "explain",
        "story:one",
        "--format",
        "json",
    ];
    let (success, before, error) = run(&root, &explain);
    assert!(success, "explain before the record: {before} {error}");
    assert_eq!(
        before["next"],
        serde_json::json!([
            {"status": "signed", "needs": [{"kind": "approval", "at_least": 1, "held": 0}]}
        ]),
        "the rung asks for one approval and nothing holds one yet: {before}"
    );

    let (success, recorded, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "evidence",
            "story:one",
            "--kind",
            "approval",
            "--source",
            "human:reviewer",
            "--command-identity",
            "evidence-v2-gated-move-approval",
            "--format",
            "json",
        ],
    );
    assert!(
        success,
        "recording an approval after the migration: {recorded} {error}"
    );

    let (success, after, error) = run(&root, &explain);
    assert!(success, "explain after the record: {after} {error}");
    assert_eq!(
        after["next"],
        serde_json::json!([
            {"status": "signed", "needs": [{"kind": "approval", "at_least": 1, "held": 1}]}
        ]),
        "the approval the authority holds is not on hand: {after}"
    );

    let (success, moved, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "move",
            "story:one",
            "--to",
            "signed",
            "--command-identity",
            "evidence-v2-gated-move-signed",
            "--format",
            "json",
        ],
    );
    assert!(
        success,
        "the rung refused a record the store itself holds: {moved} {error}"
    );
    assert_eq!(moved["outcome"]["kind"], "complete", "{moved}");

    let (success, explained, error) = run(&root, &explain);
    assert!(success, "explain after the move: {explained} {error}");
    let reached = explained["reached"]
        .as_array()
        .expect("the statuses it reached");
    let last = reached.last().expect("the move just made");
    assert_eq!(last["to"], "signed", "{explained}");
    let rested = last["rested_on"]
        .as_array()
        .expect("what the move rested on");
    assert_eq!(
        rested.len(),
        1,
        "the move names the record it rested on: {explained}"
    );
    assert_eq!(rested[0]["kind"], "approval", "{explained}");
    assert_eq!(rested[0]["source"], "human:reviewer", "{explained}");
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// **What became of a review recorded after the migration is what the store answers with.**
///
/// `outcomes_of` and `review_records` read the journal under the projection, so on a migrated plan
/// a `review_outcome` recorded after the cut-over was held by the authority and reported by
/// neither `show` nor `review-value` — the two verbs whose whole question is whether a review ever
/// changed anything.
#[test]
#[allow(clippy::too_many_lines)] // One record, asked of the three verbs that answer for a review.
fn a_review_outcome_recorded_after_migration_is_shown_and_counted() {
    let (root, _selector) =
        migrated_eventlog_plan_after_one_governed_move("evidence-v2-review-outcome");
    let (success, created, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "new",
            "review-result",
            "alpha",
            "--title",
            "A round",
            "--owner",
            "agent:alpha",
            "--relate",
            "reviews:story:one",
            "--command-identity",
            "evidence-v2-review-outcome-review",
            "--format",
            "json",
        ],
    );
    assert!(
        success,
        "a review recorded after the migration: {created} {error}"
    );
    let (success, recorded, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "evidence",
            "story:one",
            "--kind",
            "review_outcome",
            "--review",
            "review-result:alpha",
            "--outcome",
            "fixed",
            "--source",
            "human:maintainer",
            "--command-identity",
            "evidence-v2-review-outcome-record",
            "--format",
            "json",
        ],
    );
    assert!(
        success,
        "recording what became of the review: {recorded} {error}"
    );

    let (success, shown, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "show",
            "review-result:alpha",
            "--format",
            "json",
        ],
    );
    assert!(success, "show on the review: {shown} {error}");
    let outcomes = shown["outcomes"].as_array().expect("what became of it");
    assert_eq!(
        outcomes.len(),
        1,
        "the record the authority holds is not shown: {shown}"
    );
    assert_eq!(outcomes[0]["reviewed"], "story:one", "{shown}");
    assert_eq!(outcomes[0]["outcome"], "fixed", "{shown}");
    assert_eq!(outcomes[0]["source"], "human:maintainer", "{shown}");

    let (success, table, error) = run(
        &root,
        &["plan", "artifact", "review-value", "--format", "json"],
    );
    assert!(success, "the review-value table: {table} {error}");
    let rows = table["reviewers"].as_array().expect("one row per reviewer");
    assert_eq!(rows.len(), 1, "{table}");
    assert_eq!(rows[0]["reviewer"], "agent:alpha", "{table}");
    assert_eq!(
        rows[0]["fixed"], 1,
        "the outcome the authority holds is not counted: {table}"
    );

    // The same record, read the way `explain` has always read one — from the contract. Asserted
    // beside the two above so the readings cannot drift apart again.
    let (success, explained, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "explain",
            "story:one",
            "--format",
            "json",
        ],
    );
    assert!(
        success,
        "explain on the reviewed artifact: {explained} {error}"
    );
    let since = explained["recorded_since"]
        .as_array()
        .expect("records since the last move");
    assert!(
        since
            .iter()
            .any(|admitted| admitted["kind"] == "review_outcome"),
        "{explained}"
    );
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// **A migrated plan's reviews are not ordered by the journal the migration froze.**
///
/// `reviews_of` ordered the rounds by their position in the journal under the projection. After a
/// migration that file holds every review recorded before the cut-over and none recorded since, so
/// the order was partial — and a review it says nothing about sorts ahead of one it does. The
/// default pair `findings` compares was then the second round against the first, reported the
/// wrong way round, which is a ledger that calls every resolved finding new.
#[test]
fn the_reviews_of_a_migrated_plan_are_not_ordered_by_the_frozen_journal() {
    let root =
        migrated_eventlog_plan_with_a_review_from_before_the_migration("evidence-v2-review-order");
    let (success, ledger, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "findings",
            "story:one",
            "--format",
            "json",
        ],
    );
    assert!(success, "the ledger over two rounds: {ledger} {error}");
    assert_eq!(ledger["reviews"], 2, "{ledger}");
    assert_eq!(
        ledger["from"], "review-result:alpha",
        "the round recorded after the migration was compared as the earlier one: {ledger}"
    );
    assert_eq!(ledger["to"], "review-result:beta", "{ledger}");
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// Copies a directory tree, preserving each file's mode, for a case that has to put a projection
/// back the way an earlier command published it.
fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    fs::create_dir_all(to).expect("copy destination");
    for entry in fs::read_dir(from).expect("readable source") {
        let entry = entry.expect("directory entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("entry type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy one file");
        }
    }
}

/// **A document planted in an Eventlog projection is reported by nothing today**, and this is the
/// record of that rather than a claim it is right.
///
/// The drift check both `verify` and `validate` make asks whether the files the authority *owns*
/// digest to a watermark it published. A path the ownership marker never listed is digested by
/// neither side, so the watermark still matches, `verify` answers `current`, and the plan's
/// documents — which come from the authority — never mention `story:two` at all. The gap is filed
/// as `story:unowned-document-in-eventlog-projection-is-reported` and is not closed in this wave.
///
/// The case asserts what the build does, so the gap cannot close silently: when that story lands,
/// this is the assertion that fails, and the answer is to rewrite it as the refusal the story
/// promises — never to loosen it, and never to mark it ignored, which would delete the only record
/// that anybody knows.
#[test]
fn a_foreign_document_planted_in_an_eventlog_projection_is_reported_by_nothing_today() {
    let (root, selector) =
        migrated_eventlog_plan_after_one_governed_move("validate-v2-foreign-file");
    fs::write(
        root.join(".engineering/planning/story/two.md"),
        "---\nformat: aep.planning-md/1\nid: story:two\nkind: story\nstatus: draft\ntitle: \
         Two\nrelations: []\nrevision: 1\n---\n",
    )
    .expect("a document written into the projection outside a command");
    let selector_text = selector.to_string_lossy().into_owned();
    let (success, verified, error) = run(
        &root,
        &[
            "plan",
            "store",
            "verify",
            "--project",
            &selector_text,
            "--format",
            "json",
        ],
    );
    assert!(
        success,
        "`story:unowned-document-in-eventlog-projection-is-reported` has landed: verify now \
         reports the planted document. Assert the refusal that story promises here: {verified} \
         {error}"
    );
    assert_eq!(
        verified["outcome"]["value"]["projection"]["drift"], "current",
        "`story:unowned-document-in-eventlog-projection-is-reported` has landed: the ownership \
         marker now covers an unlisted path. Assert the drift that story promises here: {verified}"
    );
    let (success, after, error) = run(&root, &["plan", "artifact", "validate", "--format", "json"]);
    assert!(
        success,
        "`story:unowned-document-in-eventlog-projection-is-reported` has landed: validate now \
         reports the planted document. Assert the finding that story promises here: {after} \
         {error}"
    );
    assert_eq!(after["problems"], serde_json::json!([]), "{after}");
    assert_eq!(
        after["artifacts"], 1,
        "the plan's documents are the authority's, and the planted file is in none of them: \
         {after}"
    );
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// The deleted boundary of the same claim: a projected document removed rather than edited.
#[test]
fn validate_reports_a_projected_document_deleted_outside_a_command_on_an_eventlog_plan() {
    let (root, _selector) =
        migrated_eventlog_plan_after_one_governed_move("validate-v2-deleted-file");
    fs::remove_file(root.join(".engineering/planning/story/one.md"))
        .expect("a document removed outside a command");
    let (success, after, error) = run(&root, &["plan", "artifact", "validate", "--format", "json"]);
    assert!(
        !success,
        "a deleted projected document passed validate: {after} {error}"
    );
    let drift = after["drift"].as_array().expect("drift findings");
    assert_eq!(drift.len(), 1, "{after}");
    assert!(
        drift[0]
            .as_str()
            .expect("a finding is text")
            .contains("drifted from its authority"),
        "{after}"
    );
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// The projection put back the way the *previous* command published it, while the authority holds
/// a later move. Every owned file digests to a watermark the authority recorded — an older one —
/// so a check that asks only *is this some published state* answers current, and the reader of
/// `story/one.md` sees `proposed` where the plan says `active`.
#[test]
fn validate_reports_an_eventlog_projection_left_at_an_earlier_published_state() {
    let (root, selector) =
        migrated_eventlog_plan_after_one_governed_move("validate-v2-stale-projection");
    let projection = root.join(".engineering/planning");
    let kept = root.join("projection-after-the-first-move");
    copy_tree(&projection, &kept);
    let (success, moved, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "move",
            "story:one",
            "--to",
            "active",
            "--command-identity",
            "validate-v2-stale-projection-second",
            "--format",
            "json",
        ],
    );
    assert!(success, "second governed move: {moved} {error}");
    let projected = projection.join("story/one.md");
    assert!(
        fs::read_to_string(&projected)
            .expect("republished story")
            .contains("status: active"),
        "the second move republished the projection"
    );
    fs::remove_dir_all(&projection).expect("remove the republished projection");
    copy_tree(&kept, &projection);
    assert!(
        fs::read_to_string(&projected)
            .expect("restored story")
            .contains("status: proposed"),
        "the projection is back at the earlier published state"
    );
    let selector_text = selector.to_string_lossy().into_owned();
    let (_, verified, _) = run(
        &root,
        &[
            "plan",
            "store",
            "verify",
            "--project",
            &selector_text,
            "--format",
            "json",
        ],
    );
    let (success, after, error) = run(&root, &["plan", "artifact", "validate", "--format", "json"]);
    assert!(
        !success,
        "the projection says proposed where the authority says active, and validate reported the \
         plan clean: {after} — verify said {verified} — {error}"
    );
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// A `review-result` recorded before the migration, never answered by a `review_outcome`, and old
/// enough for `--outcome-within`. Its creation instant is in the legacy journal migration left
/// under the projection, and the document is in the authority, so the class `validate` documents
/// (`website/docs/reference/cli.md`) has everything it needs on a migrated plan.
#[test]
fn strict_validate_reports_a_pre_migration_review_without_an_outcome_on_an_eventlog_plan() {
    let (root, selector) = project();
    fs::create_dir_all(root.join(".engineering/planning/review-result")).expect("review directory");
    fs::write(
        root.join(".engineering/planning/review-result/old.md"),
        "---\nformat: aep.planning-md/1\nid: review-result:old\nkind: review-result\nstatus: \
         active\ntitle: Old\nrelations:\n- reviews: story:one\nrevision: 1\n---\n# Old\n\nProse \
         only.\n",
    )
    .expect("legacy review document");
    let story = serde_json::json!({
        "at": "2026-01-01T00:00:00Z", "actor": "human:fixture",
        "artifact": "story:one", "kind": "story", "revision": 1,
        "change": {"change": "created", "status": "draft"}
    });
    let review = serde_json::json!({
        "at": "2026-01-01T00:00:00Z", "actor": "human:fixture",
        "artifact": "review-result:old", "kind": "review-result", "revision": 1,
        "change": {"change": "created", "status": "active"}
    });
    fs::write(
        root.join(".engineering/planning/journal.jsonl"),
        format!("{story}\n{review}\n"),
    )
    .expect("legacy journal");
    apply_selected_source(&root, &selector, "validate-v2-review-outcome");
    let (_, after, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "validate",
            "--strict",
            "--outcome-within",
            "7",
            "--format",
            "json",
        ],
    );
    assert_eq!(
        after["artifacts"], 2,
        "the migrated plan holds both: {after}"
    );
    assert!(
        after["without_findings"]
            .as_array()
            .is_some_and(|prose| prose.len() == 1),
        "validate sees the review on the migrated plan: {after}"
    );
    let without = after
        .get("without_an_outcome")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        without.iter().any(|finding| finding
            .as_str()
            .is_some_and(|text| text.contains("review-result:old"))),
        "a review recorded before the migration and never answered is reported by no class once \
         the plan is Eventlog: {after} {error}"
    );
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

// ---------------------------------------------------------------------------------------------
// Independent verification pass 1 over `task:evidence-on-hand-v2-journal` (11cc13e10).
//
// Every case below asserts an invariant the unit's own contract or its CHANGELOG entry states.
// Nothing above this line is altered, skipped or weakened.
// ---------------------------------------------------------------------------------------------

/// The same command, kept as its exit status and its raw streams, for a case whose subject is a
/// refusal: a refused move is not obliged to put a document on stdout, and a helper that insists
/// on parsing one would report a missing refusal as a panic.
fn run_for_status(root: &PathBuf, args: &[&str]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(args)
        .current_dir(root)
        .output()
        .expect("AEP runs");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// A lifecycle whose last rung asks for two records of one kind, for a case about how many the
/// store thinks it holds.
fn a_rung_asking_for_two(root: &std::path::Path, kind: &str) {
    fs::create_dir_all(root.join("protocols/artifacts/lifecycles")).expect("lifecycle directory");
    fs::write(
        root.join("protocols/artifacts/lifecycles/story.yaml"),
        format!(
            "kind: story\n\
             initial: draft\n\
             transitions:\n  \
               draft: [proposed]\n  \
               proposed: [signed]\n  \
               signed: []\n\
             requires:\n  \
               signed:\n    \
                 - evidence: {kind}\n      \
                   at_least: 2\n"
        ),
    )
    .expect("a guarded rung");
}

/// **A record made before the migration is on hand exactly once after it.**
///
/// `Opened::evidence_on_hand` now answers an Eventlog plan from the contract
/// (`evidence_from_events`), and `PlanBackend::entries_of` builds that answer from two sources in
/// sequence: the journal lines the migration retained in the authority, and then the provider's
/// own events. A pre-migration record that the migration both retained *and* imported as an event
/// is therefore a candidate to be counted twice, and this count is the input to an evidence gate:
/// a rung asking for two would open on one.
///
/// The fixture records exactly one `test_result` before the cut-over, so the number here is one.
#[test]
fn evidence_recorded_before_a_migration_is_counted_once_by_a_gated_rung() {
    let (root, _selector) =
        migrated_eventlog_plan_after_one_governed_move("review-1-evidence-counted-once");
    a_rung_asking_for_two(&root, "test_result");

    let (success, explained, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "explain",
            "story:one",
            "--format",
            "json",
        ],
    );
    assert!(success, "explain on the migrated plan: {explained} {error}");
    assert_eq!(
        explained["next"],
        serde_json::json!([
            {"status": "signed", "needs": [{"kind": "test_result", "at_least": 2, "held": 1}]}
        ]),
        "the fixture recorded one `test_result` before the migration and none since; a larger \
         count is that one record counted twice — once from the journal the migration retained \
         and once from the event it was imported as — and the gate is what reads it: {explained}"
    );

    let (success, stdout, stderr) = run_for_status(
        &root,
        &[
            "plan",
            "artifact",
            "move",
            "story:one",
            "--to",
            "signed",
            "--command-identity",
            "review-1-evidence-counted-once-signed",
            "--format",
            "json",
        ],
    );
    assert!(
        !success,
        "the rung asks for two `test_result` records, the store holds one, and the move was \
         admitted: {stdout} {stderr}"
    );
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// **The same evidence command, issued twice under one `--command-identity`, is one record.**
///
/// Command identity is what makes a retried write safe, and on a migrated plan the count that
/// decides a gated move is now assembled by reading the authority's history rather than by
/// reading a journal line per write. A retry that lands as a second entry would pay for a rung
/// nobody recorded twice.
#[test]
fn a_retried_evidence_command_on_a_migrated_plan_is_counted_once() {
    let (root, _selector) =
        migrated_eventlog_plan_after_one_governed_move("review-1-retried-evidence");
    a_rung_asking_for_two(&root, "approval");
    let record = [
        "plan",
        "artifact",
        "evidence",
        "story:one",
        "--kind",
        "approval",
        "--source",
        "human:reviewer",
        "--command-identity",
        "review-1-retried-evidence-approval",
        "--format",
        "json",
    ];
    let (success, first, error) = run(&root, &record);
    assert!(success, "the first attempt: {first} {error}");
    // The retry is allowed to be admitted as the same record or refused as a repeat; what it may
    // not do is add a second one, so its outcome is reported rather than asserted.
    let (retried, retry_out, retry_error) = run_for_status(&root, &record);

    let (success, explained, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "explain",
            "story:one",
            "--format",
            "json",
        ],
    );
    assert!(success, "explain after the retry: {explained} {error}");
    assert_eq!(
        explained["next"],
        serde_json::json!([
            {"status": "signed", "needs": [{"kind": "approval", "at_least": 2, "held": 1}]}
        ]),
        "one approval was recorded, under one command identity, and the retry (admitted: \
         {retried}) made it two on hand — a rung asking for two opens on one record: \
         {explained} / {retry_out} {retry_error}"
    );
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// Two rounds on a migrated plan, whose **ids sort the opposite way round to when they happened**:
/// the first round is `review-result:zulu`, recorded before the cut-over, and the second is
/// `review-result:alpha`, recorded after it.
///
/// The implementor's own fixture names the first round `alpha` and the second `beta`, where the id
/// order and the round order agree, so it cannot tell an order taken from the store's history
/// apart from one taken from the id.
fn migrated_plan_whose_rounds_sort_against_their_order(migration: &str) -> PathBuf {
    let (root, selector) = project();
    let created = serde_json::json!({
        "at": "2026-01-01T00:00:00Z", "actor": "human:fixture",
        "artifact": "story:one", "kind": "story", "revision": 1,
        "change": {"change": "created", "status": "draft"}
    });
    fs::write(
        root.join(".engineering/planning/journal.jsonl"),
        format!("{created}\n"),
    )
    .expect("legacy journal");
    let (success, first, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "new",
            "review-result",
            "zulu",
            "--title",
            "First round",
            "--owner",
            "agent:zulu",
            "--relate",
            "reviews:story:one",
            "--format",
            "json",
        ],
    );
    assert!(
        success,
        "the first round, recorded before the migration: {first} {error}"
    );
    apply_selected_source(&root, &selector, migration);
    let identity = format!("{migration}-second-round");
    let (success, second, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "new",
            "review-result",
            "alpha",
            "--title",
            "Second round",
            "--owner",
            "agent:alpha",
            "--relate",
            "reviews:story:one",
            "--command-identity",
            &identity,
            "--format",
            "json",
        ],
    );
    assert!(
        success,
        "the second round, recorded after the migration: {second} {error}"
    );
    root
}

/// **The ledger compares the first round against the second on a migrated plan.**
///
/// The CHANGELOG entry this unit adds says that on an Eventlog plan "the order `plan artifact
/// findings` compares two rounds in ... now come[s] from the authority rather than from the legacy
/// `journal.jsonl`". `reviews_of` reads `Opened::journal`, which is `None` for an Eventlog plan, so
/// the order it actually takes comes from neither: every review falls to the id tiebreak. Where two
/// rounds are named by their reviewer rather than by their number, the ledger then reports the
/// second round as the earlier one — which is the same wrong-way-round ledger, calling every
/// resolved finding new, that the fix is written to stop.
#[test]
fn the_findings_ledger_on_a_migrated_plan_compares_the_first_round_as_the_earlier_one() {
    let root = migrated_plan_whose_rounds_sort_against_their_order("review-1-round-order");
    let (success, ledger, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "findings",
            "story:one",
            "--format",
            "json",
        ],
    );
    assert!(success, "the ledger over two rounds: {ledger} {error}");
    assert_eq!(ledger["reviews"], 2, "{ledger}");
    assert_eq!(
        ledger["from"], "review-result:zulu",
        "`review-result:zulu` is the round recorded before the migration and `review-result:alpha` \
         the one recorded after it; the ledger compares them by id instead of by when the store \
         recorded them, so the second round is being read as the first: {ledger}"
    );
    assert_eq!(ledger["to"], "review-result:alpha", "{ledger}");
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// **A review answered before the migration is answered exactly once after it, and is not
/// reported as one nobody acted on.**
///
/// The quadrant the unit's own cases leave open: they record a `review_outcome` *after* the
/// cut-over (`show`, `review-value`) and a review *without* one from before it
/// (`validate --strict`). A `review_outcome` recorded before the migration reaches every reader
/// only through the journal lines the migration retained in the authority, which
/// `PlanBackend::entries_of` reads ahead of the provider's own events — so it is the record that
/// is counted twice if the import also stands for it as an event, and the record that goes
/// missing if the retained line does not read back as an `Entry`. Three verbs change their answer
/// either way: `show` lists the outcome once, `review-value` counts it once, and `validate
/// --strict` does not report the review as unanswered.
#[test]
#[allow(clippy::too_many_lines)] // One pre-migration record, asked of the three verbs that read it.
fn a_review_answered_before_the_migration_is_answered_once_after_it() {
    let (root, selector) = project();
    let created = serde_json::json!({
        "at": "2026-01-01T00:00:00Z", "actor": "human:fixture",
        "artifact": "story:one", "kind": "story", "revision": 1,
        "change": {"change": "created", "status": "draft"}
    });
    fs::write(
        root.join(".engineering/planning/journal.jsonl"),
        format!("{created}\n"),
    )
    .expect("legacy journal");
    let (success, review, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "new",
            "review-result",
            "old",
            "--title",
            "Old round",
            "--owner",
            "agent:old",
            "--relate",
            "reviews:story:one",
            "--format",
            "json",
        ],
    );
    assert!(
        success,
        "a review recorded before the migration: {review} {error}"
    );
    let (success, answered, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "evidence",
            "story:one",
            "--kind",
            "review_outcome",
            "--review",
            "review-result:old",
            "--outcome",
            "fixed",
            "--source",
            "human:maintainer",
            "--format",
            "json",
        ],
    );
    assert!(
        success,
        "what became of it, recorded before the migration: {answered} {error}"
    );
    apply_selected_source(&root, &selector, "review-1-pre-migration-outcome");

    let (success, shown, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "show",
            "review-result:old",
            "--format",
            "json",
        ],
    );
    assert!(success, "show on the migrated plan: {shown} {error}");
    let outcomes = shown["outcomes"].as_array().expect("what became of it");
    assert_eq!(
        outcomes.len(),
        1,
        "one `review_outcome` was recorded before the migration; zero is the retained journal \
         line left unread, two is the same record read once from the retained line and again from \
         an event: {shown}"
    );
    assert_eq!(outcomes[0]["outcome"], "fixed", "{shown}");
    assert_eq!(outcomes[0]["source"], "human:maintainer", "{shown}");

    let (success, table, error) = run(
        &root,
        &["plan", "artifact", "review-value", "--format", "json"],
    );
    assert!(success, "the review-value table: {table} {error}");
    let rows = table["reviewers"].as_array().expect("one row per reviewer");
    assert_eq!(rows.len(), 1, "{table}");
    assert_eq!(
        rows[0]["fixed"], 1,
        "the outcome recorded before the migration is counted once, not zero times and not \
         twice: {table}"
    );

    let (_, validated, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "validate",
            "--strict",
            "--outcome-within",
            "0",
            "--format",
            "json",
        ],
    );
    let without = validated
        .get("without_an_outcome")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        !without.iter().any(|finding| finding
            .as_str()
            .is_some_and(|text| text.contains("review-result:old"))),
        "the review was answered before the migration, and validate reports it as one nobody \
         acted on: {validated} {error}"
    );
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

// ---------------------------------------------------------------------------------------------
// Independent verification pass 2 over `task:validate-v2-projection-awareness` (11cc13e10), adopted
// here. Five cases stand as written; four that were red against 11cc13e10 assert today's behaviour
// and name `story:strict-advisory-classes-on-an-eventlog-plan` as the story that inverts them.
// ---------------------------------------------------------------------------------------------

/// The `closed_on_an_assertion` entries `validate` reported, as plain strings.
fn closed_on_an_assertion(summary: &serde_json::Value) -> Vec<String> {
    summary
        .get("closed_on_an_assertion")
        .and_then(serde_json::Value::as_array)
        .map(|notes| {
            notes
                .iter()
                .filter_map(|note| note.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// A legacy Markdown plan whose one story was moved on **asserted** evidence, then migrated.
///
/// The move is made through the public command path before the cut-over, so the legacy journal
/// records it as resting on an assertion — and the migration retains those journal lines in the
/// authority, which is where `Opened::history_of` reads a migrated plan's history from.
fn migrated_eventlog_plan_whose_legacy_move_rested_on_an_assertion(
    migration: &str,
) -> (PathBuf, serde_json::Value) {
    let (root, selector) = project();
    let created = serde_json::json!({
        "at": "2026-01-01T00:00:00Z", "actor": "human:fixture",
        "artifact": "story:one", "kind": "story", "revision": 1,
        "change": {"change": "created", "status": "draft"}
    });
    fs::write(
        root.join(".engineering/planning/journal.jsonl"),
        format!("{created}\n"),
    )
    .expect("legacy journal");
    let (success, moved, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "move",
            "story:one",
            "--to",
            "proposed",
            "--evidence",
            "test_result=1",
            "--format",
            "json",
        ],
    );
    assert!(
        success,
        "a move on asserted evidence before the migration: {moved} {error}"
    );
    let (_, before, error) = run(
        &root,
        &[
            "plan", "artifact", "validate", "--strict", "--format", "json",
        ],
    );
    assert!(
        closed_on_an_assertion(&before)
            .iter()
            .any(|note| note.contains("story:one")),
        "the fixture never made an assertion the Markdown plan itself reports: {before} {error}"
    );
    apply_selected_source(&root, &selector, migration);
    (root, before)
}

/// **A status closed on an assertion before the migration is reported by nothing once the plan
/// is an Eventlog plan — today.**
///
/// `closed_on_an_assertion` is one of the classes `--strict` turns into exit 1. It is built from
/// `Opened::journal()`, which is `None` for every Eventlog plan, because the journal under the
/// projection is the migrated store's and stops where the migration did; the authority holds the
/// move's `decided_on` in the retained journal lines, but reading it for every artifact is the
/// store-wide read this unit removed, and no cheaper equivalent exists yet. The operator decided
/// that the class is documented as not computed on an Eventlog plan for now, and the faithful port
/// is `story:strict-advisory-classes-on-an-eventlog-plan`. This case pins today's answer: the fixture
/// asserts the Markdown plan reported the move before the cut-over, and after it the class is
/// empty and `--strict` exits 0. **When the story lands, invert these two assertions; never
/// loosen them and never `#[ignore]` this case.**
#[test]
fn a_status_closed_on_an_assertion_before_migration_is_reported_by_nothing_today() {
    let (root, before) =
        migrated_eventlog_plan_whose_legacy_move_rested_on_an_assertion("validate-v2-asserted-pre");
    let (success, after, error) = run(
        &root,
        &[
            "plan", "artifact", "validate", "--strict", "--format", "json",
        ],
    );
    assert_eq!(after["artifacts"], 1, "the migrated plan holds it: {after}");
    assert!(
        closed_on_an_assertion(&after).is_empty(),
        "`closed_on_an_assertion` is not computed on an Eventlog plan today, and this run computed \
         it — `story:strict-advisory-classes-on-an-eventlog-plan` is the port; when it lands, invert this \
         assertion to `.any(|note| note.contains(\"story:one\"))`: before {before}, after {after} \
         {error}"
    );
    assert!(
        success,
        "with no class computed, `--strict` has nothing to refuse on today; when \
         `story:strict-advisory-classes-on-an-eventlog-plan` lands this becomes `!success`: {after} {error}"
    );
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// **A status closed on an assertion *after* the migration is reported by nothing today, while
/// the store itself knows.**
///
/// The governed move is made on the Eventlog plan itself, so the record is an authority event
/// carrying the move's `decided_on`, which `PlanBackend::entries_of` reads and `explain` prints.
/// `validate` builds `closed_on_an_assertion` from `Opened::journal()`, `None` here, so the class
/// is empty and `--strict` exits 0 — the operator's decision for now, with the faithful port as
/// `story:strict-advisory-classes-on-an-eventlog-plan`. The `explain` assertion is the control: the fact is
/// in the store, and it is `validate` that does not read it. **When the story lands, invert the
/// last two assertions; never loosen them and never `#[ignore]` this case.**
#[test]
fn a_status_closed_on_an_assertion_after_migration_is_reported_by_nothing_today() {
    let (root, selector) = project();
    let created = serde_json::json!({
        "at": "2026-01-01T00:00:00Z", "actor": "human:fixture",
        "artifact": "story:one", "kind": "story", "revision": 1,
        "change": {"change": "created", "status": "draft"}
    });
    fs::write(
        root.join(".engineering/planning/journal.jsonl"),
        format!("{created}\n"),
    )
    .expect("legacy journal");
    apply_selected_source(&root, &selector, "validate-v2-asserted-post");
    let (success, moved, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "move",
            "story:one",
            "--to",
            "proposed",
            "--evidence",
            "test_result=1",
            "--command-identity",
            "validate-v2-asserted-post-move",
            "--format",
            "json",
        ],
    );
    assert!(
        success,
        "a governed move on asserted evidence after the migration: {moved} {error}"
    );
    let (success, after, error) = run(
        &root,
        &[
            "plan", "artifact", "validate", "--strict", "--format", "json",
        ],
    );
    let (_, explained, explain_error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "explain",
            "story:one",
            "--format",
            "json",
        ],
    );
    assert!(
        explained.to_string().contains("asserted"),
        "the store itself knows the move rested on an assertion: {explained} {explain_error}"
    );
    assert!(
        closed_on_an_assertion(&after).is_empty(),
        "`closed_on_an_assertion` is not computed on an Eventlog plan today, and this run computed \
         it for a governed move — `story:strict-advisory-classes-on-an-eventlog-plan` is the port; when it \
         lands, invert this assertion to `.any(|note| note.contains(\"story:one\"))`: {after} \
         {error}"
    );
    assert!(
        success,
        "with no class computed, `--strict` has nothing to refuse on today; when \
         `story:strict-advisory-classes-on-an-eventlog-plan` lands this becomes `!success`: {after} {error}"
    );
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// **A review recorded *after* the migration and never answered is reported too.**
///
/// The adopted case covers a review whose creation instant the frozen journal still holds. This is
/// the other half: a `review-result` created on the Eventlog plan itself, whose creation is an
/// authority event and nothing else, asked for at `--outcome-within 0` so that the instant rather
/// than the age decides. If `Opened::history_of` cannot date a post-migration document, the
/// class is half alive and the half that is missing is the half that grows.
#[test]
fn strict_validate_reports_a_post_migration_review_without_an_outcome_on_an_eventlog_plan() {
    let (root, selector) = project();
    let created = serde_json::json!({
        "at": "2026-01-01T00:00:00Z", "actor": "human:fixture",
        "artifact": "story:one", "kind": "story", "revision": 1,
        "change": {"change": "created", "status": "draft"}
    });
    fs::write(
        root.join(".engineering/planning/journal.jsonl"),
        format!("{created}\n"),
    )
    .expect("legacy journal");
    apply_selected_source(&root, &selector, "validate-v2-review-after");
    let (success, review, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "new",
            "review-result",
            "fresh",
            "--title",
            "After the cut-over",
            "--owner",
            "agent:fresh",
            "--relate",
            "reviews:story:one",
            "--command-identity",
            "validate-v2-review-after-new",
            "--format",
            "json",
        ],
    );
    assert!(
        success,
        "a review recorded after the migration: {review} {error}"
    );
    let (success, after, error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "validate",
            "--strict",
            "--outcome-within",
            "0",
            "--format",
            "json",
        ],
    );
    assert_eq!(after["artifacts"], 2, "the plan holds both: {after}");
    let without = after
        .get("without_an_outcome")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        without.iter().any(|note| note
            .as_str()
            .is_some_and(|text| text.contains("review-result:fresh"))),
        "a review recorded after the migration and never answered is dated by nothing, so the \
         class skips it: {after} {error}"
    );
    assert!(
        !success,
        "--strict must refuse on a review with no recorded outcome: {after} {error}"
    );
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// Two owned projection files wrong at once — one edited, one deleted.
///
/// The boundary above the unit's own single-fault cases: `projection_current` answers one
/// `Result`, so the question is whether the second fault can mask the first, and whether the exit
/// status still says the plan is not clean. Neither fault may be swallowed.
#[test]
fn validate_reports_an_eventlog_projection_with_one_file_edited_and_another_deleted() {
    let (root, selector) = project();
    fs::write(root.join(".engineering/planning/story/two.md"),
        "---\nformat: aep.planning-md/1\nid: story:two\nkind: story\nstatus: draft\ntitle: Two\nrelations: []\nrevision: 1\n---\n")
        .expect("second legacy story");
    let one = serde_json::json!({
        "at": "2026-01-01T00:00:00Z", "actor": "human:fixture",
        "artifact": "story:one", "kind": "story", "revision": 1,
        "change": {"change": "created", "status": "draft"}
    });
    let two = serde_json::json!({
        "at": "2026-01-01T00:00:00Z", "actor": "human:fixture",
        "artifact": "story:two", "kind": "story", "revision": 1,
        "change": {"change": "created", "status": "draft"}
    });
    fs::write(
        root.join(".engineering/planning/journal.jsonl"),
        format!("{one}\n{two}\n"),
    )
    .expect("legacy journal");
    apply_selected_source(&root, &selector, "validate-v2-two-faults");
    let (success, clean, error) = run(&root, &["plan", "artifact", "validate", "--format", "json"]);
    assert!(success, "validate right after migration: {clean} {error}");
    assert_eq!(clean["problems"], serde_json::json!([]), "{clean}");
    let edited = root.join(".engineering/planning/story/one.md");
    let text = fs::read_to_string(&edited).expect("projected story");
    fs::write(&edited, text.replace("status: draft", "status: active"))
        .expect("an edit made outside a command");
    fs::remove_file(root.join(".engineering/planning/story/two.md"))
        .expect("a deletion made outside a command");
    let (success, after, error) = run(&root, &["plan", "artifact", "validate", "--format", "json"]);
    assert!(
        !success,
        "two owned files wrong at once and validate passed: {after} {error}"
    );
    let drift = after
        .get("drift")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        !drift.is_empty(),
        "neither fault is reported as drift: {after} {error}"
    );
    assert!(
        !after["problems"].as_array().expect("problems").is_empty(),
        "neither fault is a problem: {after} {error}"
    );
    // The unit's contract is *one* authority-decided finding in each list, whatever the number of
    // owned files that disagree: the projection is compared as a whole. Measured, not eyeballed.
    assert_eq!(
        drift.len(),
        1,
        "two faults, and the projection is still compared as one whole: {after}"
    );
    assert_eq!(
        after["problems"].as_array().expect("problems").len(),
        1,
        "{after}"
    );
    eprintln!("two faults at once, validate answered: {after}");
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// The projection's ownership marker removed: the file that decides what `projection_current`
/// digests at all.
///
/// With it gone there is nothing to compare, and the one answer `validate` must not give is that
/// the plan is clean — a check whose input can be deleted to silence it is not a check.
#[test]
fn validate_reports_an_eventlog_projection_whose_ownership_marker_was_removed() {
    let (root, selector) = project();
    let created = serde_json::json!({
        "at": "2026-01-01T00:00:00Z", "actor": "human:fixture",
        "artifact": "story:one", "kind": "story", "revision": 1,
        "change": {"change": "created", "status": "draft"}
    });
    fs::write(
        root.join(".engineering/planning/journal.jsonl"),
        format!("{created}\n"),
    )
    .expect("legacy journal");
    apply_selected_source(&root, &selector, "validate-v2-marker-removed");
    let marker = root.join(".engineering/planning/.aep-projection-ownership.json");
    assert!(
        marker.is_file(),
        "the migration published an ownership marker at {}",
        marker.display()
    );
    fs::remove_file(&marker).expect("the marker removed outside a command");
    let (success, after, error) = run(&root, &["plan", "artifact", "validate", "--format", "json"]);
    assert!(
        !success,
        "the projection's ownership marker was deleted and validate called the plan clean: \
         {after} {error}"
    );
    assert!(
        !after["problems"].as_array().expect("problems").is_empty(),
        "a projection with no ownership marker is no problem: {after} {error}"
    );
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// Like [`run`], for a command whose answer may not be JSON: the exit status, whatever JSON stdout
/// held, and both streams as text. A case about a plan whose projection is *gone* must not panic
/// inside its own fixture when the edge answers with prose on stderr instead.
fn run_expecting_any(root: &PathBuf, args: &[&str]) -> (bool, Option<serde_json::Value>, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(args)
        .current_dir(root)
        .output()
        .expect("AEP runs");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (
        output.status.success(),
        serde_json::from_slice(&output.stdout).ok(),
        format!("stdout: {stdout} / stderr: {stderr}"),
    )
}

/// `story:one` given one real event through the command path, so the legacy log has heard of it
/// and it is *not* a document predating the event log. The compact journal line the other
/// fixtures write is a journal entry and not an event, and `drift::detect` counts events.
fn story_one_with_one_recorded_event(root: &PathBuf) {
    let one = serde_json::json!({
        "at": "2026-01-01T00:00:00Z", "actor": "human:fixture",
        "artifact": "story:one", "kind": "story", "revision": 1,
        "change": {"change": "created", "status": "draft"}
    });
    fs::write(
        root.join(".engineering/planning/journal.jsonl"),
        format!("{one}\n"),
    )
    .expect("legacy journal that knows only story:one");
    let (success, recorded, error) = run(
        root,
        &[
            "plan",
            "artifact",
            "evidence",
            "story:one",
            "--kind",
            "test_result",
            "--source",
            "legacy-check",
            "--format",
            "json",
        ],
    );
    assert!(success, "legacy evidence write: {recorded} {error}");
}

/// A second legacy story with **no journal line and no event at all** — a document written before
/// the log existed, which the Markdown plan reports as *predating the event log*.
fn a_second_legacy_story_the_journal_never_heard_of(root: &PathBuf) {
    fs::write(
        root.join(".engineering/planning/story/two.md"),
        "---\nformat: aep.planning-md/1\nid: story:two\nkind: story\nstatus: draft\ntitle: \
         Two\nrelations: []\nrevision: 1\n---\n",
    )
    .expect("a legacy story the journal never recorded");
    story_one_with_one_recorded_event(root);
}

/// **A document predating the journal is reported as predating the event log by nothing once the
/// plan is an Eventlog plan — today.**
///
/// `pre_provider` is one of the classes `--strict` refuses on (`strictly_refused`). `findings`
/// takes the count from `log_findings`, which runs only when `Opened::journal()` is `Some`, so on
/// every Eventlog plan it is `0` by construction rather than measured. The operator decided the
/// class is documented as not computed on an Eventlog plan for now; the faithful port is
/// `story:strict-advisory-classes-on-an-eventlog-plan`. The fixture asserts the Markdown plan reports the
/// document before the migration, which is the control, and pins the `0` and the exit 0 after it.
/// **When the story lands, invert the last two assertions; never loosen them and never `#[ignore]`
/// this case.**
#[test]
fn a_document_predating_the_journal_is_reported_by_nothing_today_on_an_eventlog_plan() {
    let (root, selector) = project();
    a_second_legacy_story_the_journal_never_heard_of(&root);
    let strict = [
        "plan", "artifact", "validate", "--strict", "--format", "json",
    ];
    let (success, before, error) = run(&root, &strict);
    assert_eq!(
        before["pre_provider"], 1,
        "the fixture never made a document the Markdown plan reports as predating the log: \
         {before} {error}"
    );
    assert!(
        !success,
        "--strict on the Markdown plan refuses on the class: {before} {error}"
    );
    apply_selected_source(&root, &selector, "validate-v2-pre-provider");
    let (_, history, history_error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "history",
            "story:two",
            "--format",
            "json",
        ],
    );
    let (success, after, error) = run(&root, &strict);
    assert_eq!(
        after["artifacts"], 2,
        "the migrated plan holds both: {after}"
    );
    assert_eq!(
        after["pre_provider"], 0,
        "`pre_provider` is `0` by construction on an Eventlog plan today, and this run measured it \
         — `story:strict-advisory-classes-on-an-eventlog-plan` is the port; when it lands, this becomes \
         `1` — the authority's history of the document reads {history} {history_error}: before \
         {before}, after {after} {error}"
    );
    assert!(
        success,
        "with the class not computed, `--strict` has nothing to refuse on today; when \
         `story:strict-advisory-classes-on-an-eventlog-plan` lands this becomes `!success`: {after} {error}"
    );
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// **A review the journal never heard of is named by neither journal class once the plan is an
/// Eventlog plan — today — and only the prose-only findings class still names it.**
///
/// On a Markdown plan a `review-result` with no journal line is reported once, as a document
/// predating the event log, and `reviews_without_an_outcome` leaves it out on purpose — its own
/// doc comment says reporting it twice "would say two things about one gap". Once the plan is
/// Eventlog the first class is `0` by construction, and the second cannot date the review either:
/// the authority holds no `Created` entry for a document the migration imported without a journal
/// line, so at `--outcome-within 0` it is left out rather than guessed at. A review nobody ever
/// answered has fallen out of both classes; what still names it is `without_findings`, because its
/// body is prose only, and that is the class `--strict` refuses on here. The operator decided the
/// two classes are documented as not computed on an Eventlog plan for now; the faithful port is
/// `story:strict-advisory-classes-on-an-eventlog-plan`. **When the story lands, invert the `predating`/
/// `without_an_outcome` assertion; never loosen it and never `#[ignore]` this case.**
#[test]
fn a_pre_journal_review_nobody_answered_is_named_by_neither_journal_class_today() {
    let (root, selector) = project();
    fs::create_dir_all(root.join(".engineering/planning/review-result")).expect("review directory");
    fs::write(
        root.join(".engineering/planning/review-result/old.md"),
        "---\nformat: aep.planning-md/1\nid: review-result:old\nkind: review-result\nstatus: \
         active\ntitle: Old\nrelations:\n- reviews: story:one\nrevision: 1\n---\n# Old\n\nProse \
         only.\n",
    )
    .expect("a legacy review the journal never recorded");
    story_one_with_one_recorded_event(&root);
    let strict = [
        "plan",
        "artifact",
        "validate",
        "--strict",
        "--outcome-within",
        "0",
        "--format",
        "json",
    ];
    let (_, before, error) = run(&root, &strict);
    assert_eq!(
        before["pre_provider"], 1,
        "the Markdown plan reports the review as predating the log: {before} {error}"
    );
    apply_selected_source(&root, &selector, "validate-v2-pre-journal-review");
    let (_, history, history_error) = run(
        &root,
        &[
            "plan",
            "artifact",
            "history",
            "review-result:old",
            "--format",
            "json",
        ],
    );
    let (success, after, error) = run(&root, &strict);
    assert_eq!(
        after["artifacts"], 2,
        "the migrated plan holds both: {after}"
    );
    let predating = after["pre_provider"].as_u64().unwrap_or_default();
    let without_an_outcome = after
        .get("without_an_outcome")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|notes| {
            notes.iter().any(|note| {
                note.as_str()
                    .is_some_and(|text| text.contains("review-result:old"))
            })
        });
    assert!(
        predating == 0 && !without_an_outcome,
        "a review the journal never recorded and nobody answered is named by neither journal class \
         on an Eventlog plan today, and this run named it — `story:strict-advisory-classes-on-an-eventlog-plan` \
         is the port; when it lands, invert this to `predating > 0 || without_an_outcome` — the \
         authority's history of it reads {history} {history_error}: before {before}, after {after} \
         {error}"
    );
    let prose_only = after
        .get("without_findings")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|notes| {
            notes.iter().any(|note| {
                note.as_str()
                    .is_some_and(|text| text.contains("review-result:old"))
            })
        });
    assert!(
        prose_only && !success,
        "the one class that names the review today is the prose-only findings class, and it is \
         what `--strict` refuses on — neither journal class is involved: {after} {error}"
    );
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// The projection directory removed whole, authority and selector left in place.
///
/// Above the marker case: nothing under the projection root exists to digest, and the plan's
/// documents still come from the authority, so `validate` can load the plan and must then say the
/// projection is not what the authority published — as `verify` does. The two verbs read the same
/// facts (`projection_current` is `verify`'s check, asked by `validate`), so they must not answer
/// differently here.
#[test]
fn validate_reports_an_eventlog_plan_whose_projection_directory_is_missing() {
    let (root, selector) =
        migrated_eventlog_plan_after_one_governed_move("validate-v2-projection-gone");
    let projection = root.join(".engineering/planning");
    fs::remove_dir_all(&projection).expect("the projection removed outside a command");
    let selector_text = selector.to_string_lossy().into_owned();
    let (verify_success, verified, verify_error) = run(
        &root,
        &[
            "plan",
            "store",
            "verify",
            "--project",
            &selector_text,
            "--format",
            "json",
        ],
    );
    assert!(
        !verify_success,
        "verify called a missing projection current: {verified} {verify_error}"
    );
    let (success, after, streams) =
        run_expecting_any(&root, &["plan", "artifact", "validate", "--format", "json"]);
    assert!(
        !success,
        "the projection directory is gone and validate called the plan clean: {after:?} — verify \
         said {verified} — {streams}"
    );
    let after = after.unwrap_or_else(|| {
        panic!(
            "validate refused a plan whose projection is gone but did not answer as a validation \
             — verify said {verified} — {streams}"
        )
    });
    let drift = after
        .get("drift")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        drift.len(),
        1,
        "one authority-decided finding, as for an edited or deleted owned file: {after}"
    );
    assert!(
        drift[0]
            .as_str()
            .is_some_and(|text| text.contains("drifted from its authority")),
        "{after}"
    );
    assert_eq!(
        after["problems"].as_array().expect("problems").len(),
        1,
        "{after}"
    );
    fs::remove_dir_all(root).expect("remove disposable fixture");
}

/// The ownership marker edited to disown the projected story, the story itself untouched.
///
/// The marker decides what `projection_current` digests, so an edit that shortens its list is an
/// attempt to shrink the check to a set of files nothing published. The inventory over the
/// remaining files digests to no watermark the authority holds, and the answer must be the same
/// one an edited owned file gets.
#[test]
fn validate_reports_an_eventlog_projection_whose_ownership_marker_was_edited() {
    let (root, _selector) =
        migrated_eventlog_plan_after_one_governed_move("validate-v2-marker-edited");
    let marker = root.join(".engineering/planning/.aep-projection-ownership.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&marker).expect("ownership marker"))
            .expect("the marker is JSON");
    let owned = value["owned"]
        .as_array_mut()
        .expect("the marker lists owned files");
    let listed = owned.len();
    owned.retain(|entry| entry["path"] != "story/one.md");
    assert_eq!(
        owned.len(),
        listed - 1,
        "the marker listed the projected story before the edit: {value}"
    );
    let mut bytes = serde_json::to_vec(&value).expect("marker bytes");
    bytes.push(b'\n');
    fs::write(&marker, bytes).expect("the marker edited outside a command");
    let (success, after, error) = run(&root, &["plan", "artifact", "validate", "--format", "json"]);
    assert!(
        !success,
        "the marker no longer lists story/one.md and validate called the plan clean: {after} \
         {error}"
    );
    let drift = after
        .get("drift")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert_eq!(drift.len(), 1, "{after}");
    assert!(
        drift[0]
            .as_str()
            .is_some_and(|text| text.contains("drifted from its authority")),
        "{after}"
    );
    fs::remove_dir_all(root).expect("remove disposable fixture");
}
