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
