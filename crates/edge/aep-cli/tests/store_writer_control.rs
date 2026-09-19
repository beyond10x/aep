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
    let mut holder = command.spawn().expect("holder starts");
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
    let mut stdout = BufReader::new(holder.stdout.take().expect("holder stdout"));
    let mut line = String::new();
    stdout
        .read_line(&mut line)
        .expect("holder stop observation");
    assert!(line.contains("Retained stop evidence"), "{line}");
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
    let (success, applied, error) = run(root, &apply);
    assert!(success, "SQL public apply: {applied} {error}");
    assert!(applied["outcome"]["value"]["receipt"].is_object());
    stop_holder(&mut holder);
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
    let (success, history_after, error) = run(root, &history_args);
    assert!(success, "selected history: {history_after} {error}");
    assert_eq!(
        history_after, history_before,
        "ordinary history survives migration"
    );
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
fn public_sqlite_source_applies_under_observed_writer_stop() {
    let (root, selector) = project();
    let engineering = root.join(".engineering");
    seed_sqlite(
        &engineering.join("planning"),
        &engineering.join("plan.sqlite3"),
    );
    fs::write(&selector,
        "{\"protocol\":\"adp/1\",\"profile\":\"development.standard\",\"protocols\":\"../protocols\",\"store\":{\"sqlite\":\"plan.sqlite3\"}}\n")
        .expect("SQLite selector");
    apply_selected_sql_source(&root, &selector, "control-sqlite-migration");
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
    seed_postgres(&root.join(".engineering/planning"), &scoped_url);
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
