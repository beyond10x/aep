//! Write verbs on artifact ids near the 200-character identifier limit.
//!
//! Every envelope identifier a verb builds comes from the artifact id, so a long id used to push
//! the request id, command id or idempotency key over the identifier cap and the verb refused.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

fn relative(from: &Path, to: &Path) -> String {
    let base: Vec<_> = from.components().collect();
    let target: Vec<_> = to.components().collect();
    let shared = base
        .iter()
        .zip(&target)
        .take_while(|(left, right)| left == right)
        .count();
    let mut parts = vec![".."; base.len() - shared];
    parts.extend(
        target[shared..]
            .iter()
            .map(|component| component.as_os_str().to_str().expect("a printable path")),
    );
    parts.join("/")
}

/// A fresh Git-native project named `name`.
fn project(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("long-ids-{name}"));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("the previous scratch project is removable");
    }
    let engineering = directory.join(".engineering");
    std::fs::create_dir_all(&engineering).expect("the scratch project is writable");
    let status = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&directory)
        .status()
        .expect("git runs");
    assert!(status.success(), "git init");
    std::fs::write(
        engineering.join("project.yaml"),
        format!(
            "version: aep.project/5\nplanning_scope: long-ids-test\nprotocol: adp/1\n\
             profile: development.standard\nprotocols: {}\nstore:\n  git: {{}}\n",
            relative(&engineering, &repository())
        ),
    )
    .expect("the project file is writable");
    directory
}

fn aep(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(["plan", "artifact"])
        .args(args)
        .current_dir(directory)
        .env("AEP_ACTOR", "human:tester")
        .output()
        .expect("the binary runs")
}

fn ok(directory: &Path, args: &[&str]) -> String {
    let output = aep(directory, args);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "`{}` failed: {stdout}{stderr}",
        args.join(" ")
    );
    stdout
}

/// An artifact id `story:<name>` of exactly `length` characters.
fn story_id(length: usize, letter: char) -> String {
    let name_length = length - "story:".len();
    format!("story:{}", letter.to_string().repeat(name_length))
}

fn create(project: &Path, id: &str) {
    let name = id.strip_prefix("story:").expect("a story id");
    ok(project, &["new", "story", name, "--title", "Long"]);
}

#[test]
fn relate_joins_a_120_character_id_to_a_100_character_id() {
    let project = project("relate");
    let source = story_id(120, 'a');
    let target = story_id(100, 'b');
    create(&project, &source);
    create(&project, &target);

    ok(&project, &["relate", &source, &format!("reviews:{target}")]);

    let shown = ok(&project, &["show", &source]);
    assert!(
        shown.contains(&target),
        "the relation is in the store: {shown}"
    );
}

#[test]
fn unrelate_removes_an_edge_between_long_ids() {
    let project = project("unrelate");
    let source = story_id(120, 'a');
    let target = story_id(100, 'b');
    create(&project, &source);
    create(&project, &target);
    ok(&project, &["relate", &source, &format!("reviews:{target}")]);

    ok(
        &project,
        &["unrelate", &source, &format!("reviews:{target}")],
    );

    let shown = ok(&project, &["show", &source]);
    assert!(!shown.contains(&target), "the relation is gone: {shown}");
}

#[test]
fn new_move_and_evidence_succeed_on_an_id_near_the_limit() {
    let project = project("near-limit");
    let id = story_id(198, 'c');

    create(&project, &id);
    assert!(
        ok(&project, &["list"]).contains(&id),
        "`new` filed the artifact"
    );

    ok(&project, &["move", &id, "--to", "proposed"]);
    assert!(
        ok(&project, &["show", &id]).contains("proposed"),
        "`move` took effect"
    );

    ok(
        &project,
        &[
            "evidence",
            &id,
            "--kind",
            "test_result",
            "--source",
            "task check",
            "--at",
            "2026-09-28T10:00:00Z",
        ],
    );
    let evidence = project.join(".engineering/evidence/story");
    assert!(evidence.is_dir(), "`evidence` filed a record");
}
