//! `aep plan store migrate texts` moves a tree store written by an older release
//! (`eventlog-tree/1`) to `eventlog-tree/2`, where each long text is stored once, and the plan
//! reads exactly as it did: the same list, the same artifact, the same projection.

use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

fn relative(from: &Path, to: &Path) -> PathBuf {
    let from: Vec<Component<'_>> = from.components().collect();
    let to: Vec<Component<'_>> = to.components().collect();
    let shared = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut path = PathBuf::new();
    for _ in shared..from.len() {
        path.push("..");
    }
    for component in &to[shared..] {
        path.push(component);
    }
    path
}

fn aep(project: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(args)
        .current_dir(project)
        .output()
        .expect("the aep binary runs")
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn copy_tree(from: &Path, into: &Path) {
    std::fs::create_dir_all(into).expect("the copy target");
    for entry in std::fs::read_dir(from).expect("readable").flatten() {
        let path = entry.path();
        let target = into.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            std::fs::copy(&path, &target).expect("the file copies");
        }
    }
}

/// Every file under `root` with its bytes, relative to it, `.cache/` and locks excluded.
fn snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("readable").flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if name != ".cache" {
                    pending.push(path);
                }
            } else if Path::new(&name)
                .extension()
                .is_none_or(|extension| extension != "lock")
            {
                let relative = path
                    .strip_prefix(root)
                    .expect("under root")
                    .display()
                    .to_string();
                found.push((relative, std::fs::read(&path).expect("readable")));
            }
        }
    }
    found.sort();
    found
}

fn count_under(root: &Path, kind: &str) -> usize {
    snapshot(root)
        .iter()
        .filter(|(path, _)| path.split('/').any(|part| part == kind))
        .count()
}

/// The fixture an older release wrote (`eventlog-tree/1`), with a story whose body is long enough
/// to be a text, written by this binary to that first-layout store.
fn older_store(name: &str) -> PathBuf {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tree-rendered-by-0-58-0");
    let project = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("tree-texts-{name}"));
    let _ = std::fs::remove_dir_all(&project);
    copy_tree(&fixture, &project);
    let engineering = project.join(".engineering");
    let selector = engineering.join("project.yaml");
    let mut document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&selector).expect("the selector reads"))
            .expect("the selector is JSON");
    document["protocols"] =
        serde_json::Value::String(relative(&engineering, &workspace()).display().to_string());
    std::fs::write(&selector, document.to_string()).expect("the selector writes");
    let created = aep(
        &project,
        &[
            "plan",
            "artifact",
            "new",
            "story",
            "long-body",
            "--title",
            "Long body",
        ],
    );
    assert!(created.status.success(), "{}", text(&created));
    let body = project.join("long-body.md");
    std::fs::write(
        &body,
        format!(
            "# Long body\n\n## Outcome\n\n{}",
            "A paragraph that says what the story changes.\n".repeat(60)
        ),
    )
    .expect("the body writes");
    let written = aep(
        &project,
        &[
            "plan",
            "artifact",
            "body",
            "story:long-body",
            "--from",
            body.to_str().expect("a UTF-8 path"),
        ],
    );
    assert!(written.status.success(), "{}", text(&written));
    std::fs::remove_file(&body).expect("the body file is removed");
    project
}

fn store_format(project: &Path) -> String {
    let manifest =
        std::fs::read(project.join(".engineering/state/store.json")).expect("store.json");
    serde_json::from_slice::<serde_json::Value>(&manifest).expect("JSON")["format"]
        .as_str()
        .expect("a format")
        .to_owned()
}

fn json(output: &Output) -> serde_json::Value {
    assert!(output.status.success(), "{}", text(output));
    serde_json::from_slice(&output.stdout).expect("the result is JSON")
}

#[test]
fn an_older_store_migrates_to_texts_stored_once_and_reads_exactly_as_before() {
    let project = older_store("migrate");
    let state = project.join(".engineering/state");
    assert_eq!(store_format(&project), "eventlog-tree/1");
    let list_before = aep(&project, &["plan", "artifact", "list"]);
    let show_before = aep(&project, &["plan", "artifact", "show", "story:long-body"]);
    assert!(show_before.status.success(), "{}", text(&show_before));
    let before = snapshot(&project);

    let checked = aep(&project, &["plan", "store", "migrate", "texts", "--check"]);
    assert_eq!(checked.status.code(), Some(1), "{}", text(&checked));
    let planned = json(&aep(
        &project,
        &[
            "plan",
            "store",
            "migrate",
            "texts",
            "--dry-run",
            "--format",
            "json",
        ],
    ));
    assert_eq!(snapshot(&project), before, "a dry run wrote");
    assert_eq!(planned["pending"], true);
    assert_eq!(planned["from_format"], "eventlog-tree/1");
    assert!(planned["blobs_split"].as_u64() > Some(0), "{planned}");
    assert_eq!(planned["verified_blobs"], serde_json::Value::Null);

    let applied = json(&aep(
        &project,
        &["plan", "store", "migrate", "texts", "--format", "json"],
    ));
    assert_eq!(applied["blobs_split"], planned["blobs_split"]);
    assert_eq!(applied["bytes_after"], planned["bytes_after"]);
    assert_eq!(applied["verified_blobs"], applied["blobs"]);
    assert_eq!(store_format(&project), "eventlog-tree/2");
    assert!(count_under(&state, "texts") > 0, "no text was stored");

    assert_eq!(
        aep(&project, &["plan", "artifact", "list"]).stdout,
        list_before.stdout
    );
    assert_eq!(
        aep(&project, &["plan", "artifact", "show", "story:long-body"]).stdout,
        show_before.stdout,
        "the migrated store reads the long story differently"
    );
    for (path, bytes) in &before {
        if path.starts_with(".engineering/planning/") || path.contains("/streams/") {
            assert!(
                snapshot(&project).contains(&(path.clone(), bytes.clone())),
                "the migration changed {path}"
            );
        }
    }
    let validated = aep(&project, &["plan", "artifact", "validate"]);
    assert!(validated.status.success(), "{}", text(&validated));

    let again = aep(&project, &["plan", "store", "migrate", "texts", "--check"]);
    assert!(
        again.status.success(),
        "a migrated store still had work: {}",
        text(&again)
    );
    let settled = snapshot(&project);
    let reapplied = aep(&project, &["plan", "store", "migrate", "texts"]);
    assert!(reapplied.status.success(), "{}", text(&reapplied));
    assert_eq!(
        snapshot(&project),
        settled,
        "a second migration changed the store"
    );
}

#[test]
fn a_store_that_is_not_a_tree_is_refused_by_name() {
    let project = Path::new(env!("CARGO_TARGET_TMPDIR")).join("tree-texts-markdown");
    let _ = std::fs::remove_dir_all(&project);
    let engineering = project.join(".engineering");
    std::fs::create_dir_all(engineering.join("planning")).expect("the store directory");
    std::fs::write(
        engineering.join("project.yaml"),
        format!(
            "{{\"protocol\":\"adp/1\",\"profile\":\"development.standard\",\"protocols\":{:?},\"version\":\"aep.project/1\"}}",
            relative(&engineering, &workspace()).display().to_string()
        ),
    )
    .expect("the selector writes");
    let refused = aep(
        &project,
        &["plan", "store", "migrate", "texts", "--dry-run"],
    );
    assert!(!refused.status.success());
    assert!(
        text(&refused).contains("migrates an aep.project/3 tree store"),
        "{}",
        text(&refused)
    );
}
