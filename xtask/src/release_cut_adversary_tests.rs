//! Adversarial cases for story:release-cutting-commands.
//!
//! Test-only: compiled under `#[cfg(test)]` as a child of `release_cut`, so it reaches the same
//! private functions the unit's own tests do, without changing them.

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;

use super::{prepare, record, tag, Forge, GitWriter, Recorder, ReleaseRun};
use crate::dep_tests::tempdir::Root;

const CHANGELOG: &str = "# Changelog\n\n## [Unreleased]\n\n### Added\n\n- a thing\n\n\
    ## [{v}] — 2026-01-01\n\n- the first\n";

fn git(root: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf-8")
        .trim()
        .to_owned()
}

/// A checkout released at `version`, with an Unreleased section and `main` on a bare `origin`.
fn checkout(label: &str, version: &str) -> (Root, PathBuf) {
    let scratch = Root::new(label);
    let root = scratch.path().join("work");
    fs::create_dir_all(&root).expect("mkdir");
    fs::create_dir_all(scratch.path().join("no-hooks")).expect("mkdir");
    git(scratch.path(), &["init", "-q", "--bare", "origin.git"]);
    git(&root, &["init", "-q", "-b", "main"]);
    for (key, value) in [
        ("user.name", "fixture"),
        ("user.email", "fixture@example.invalid"),
        ("commit.gpgsign", "false"),
        ("tag.gpgsign", "false"),
        ("core.hooksPath", "../no-hooks"),
    ] {
        git(&root, &["config", key, value]);
    }
    let origin = scratch.path().join("origin.git");
    git(
        &root,
        &["remote", "add", "origin", &origin.to_string_lossy()],
    );
    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[workspace]\nmembers = [\"a\"]\n\n[workspace.package]\nversion = \"{version}\"\n\
             edition = \"2021\"\n"
        ),
    )
    .expect("write");
    fs::write(
        root.join("Cargo.lock"),
        format!("version = 4\n\n[[package]]\nname = \"a\"\nversion = \"{version}\"\n"),
    )
    .expect("write");
    fs::write(root.join("CHANGELOG.md"), CHANGELOG.replace("{v}", version)).expect("write");
    git(&root, &["add", "."]);
    git(&root, &["commit", "-q", "-m", "chore: release"]);
    git(
        &root,
        &["tag", "-a", version, "-m", &format!("aep {version}")],
    );
    git(&root, &["push", "-q", "origin", "main"]);
    (scratch, root)
}

fn files(root: &Path) -> String {
    ["Cargo.toml", "Cargo.lock", "CHANGELOG.md"]
        .iter()
        .map(|name| fs::read_to_string(root.join(name)).expect("readable"))
        .collect()
}

// prepare: version comparison

/// `prepare 1.0.01` is (1, 0, 1) to `parse_version`, so it passes as above `1.0.0` and is written
/// into the manifest; Cargo refuses a leading zero in a SemVer number, so the release commit does
/// not build and the tag that follows is not a version anything else reads as `1.0.1`.
#[test]
fn prepare_refuses_a_version_with_a_leading_zero() {
    let (_scratch, root) = checkout("adv-leading-zero", "1.0.0");
    let before = files(&root);
    let outcome = prepare(&root, "1.0.01", "2026-01-02");
    assert!(
        outcome.is_err(),
        "1.0.01 is not a bare SemVer version, and prepare wrote it: {}",
        fs::read_to_string(root.join("Cargo.toml")).expect("readable")
    );
    assert_eq!(files(&root), before, "a refusal changes nothing");
}

/// The comparison is numeric, not textual: 0.10.0 is above 0.9.9, and 0.9.10 is not above 0.10.0.
#[test]
fn prepare_compares_versions_by_number_not_by_text() {
    let (_scratch, root) = checkout("adv-numeric-up", "0.9.9");
    prepare(&root, "0.10.0", "2026-01-02").expect("0.10.0 is above 0.9.9");

    let (_scratch, root) = checkout("adv-numeric-down", "0.10.0");
    let refusal = prepare(&root, "0.9.10", "2026-01-02").expect_err("0.9.10 is below 0.10.0");
    assert!(format!("{refusal:#}").contains("not above"), "{refusal:#}");
}

// tag

/// A lightweight tag at `HEAD` is not the annotated `aep <version>` tag the story asks for, and
/// `tag` reports it as already done (`tag_commit` peels to the commit and compares only that).
#[test]
fn tag_does_not_accept_a_lightweight_tag_at_head_as_the_release_tag() {
    let (_scratch, root) = checkout("adv-lightweight", "1.0.0");
    prepare(&root, "9.9.9", "2026-01-02").expect("prepare");
    git(&root, &["commit", "-q", "-am", "chore: release 9.9.9"]);
    git(&root, &["tag", "9.9.9"]);
    let outcome = tag(&root, &GitWriter::plain());
    let kind = git(&root, &["cat-file", "-t", "9.9.9"]);
    assert!(
        outcome.is_err() || kind == "tag",
        "tag accepted a lightweight tag as the release tag (outcome {outcome:?}, object {kind})"
    );
}

// record

struct OneRun;

impl Forge for OneRun {
    fn release_run(&self, _root: &Path, _tag: &str, _commit: &str) -> Result<Option<ReleaseRun>> {
        Ok(Some(ReleaseRun {
            id: 42,
            url: "https://github.com/beyond10x/aep/actions/runs/42".to_owned(),
            conclusion: "success".to_owned(),
        }))
    }
}

#[derive(Default)]
struct Calls(RefCell<Vec<Vec<String>>>);

impl Recorder for Calls {
    fn aep(&self, _root: &Path, arguments: &[String]) -> Result<()> {
        self.0.borrow_mut().push(arguments.to_vec());
        Ok(())
    }
}

/// `record` treats the release as recorded when any `test_result` anywhere in the store mentions
/// the tag's commit, abbreviated or not (`release_recorded`, `xtask/src/main.rs`). A story's own
/// gate record naming the release branch head, which is the tag commit, is such a line, and the
/// release epic then never gets its record while `record` prints "already recorded".
#[test]
fn record_writes_the_release_record_when_another_test_result_names_the_tag_commit() {
    let (_scratch, root) = checkout("adv-record-other", "1.0.0");
    prepare(&root, "9.9.9", "2026-01-02").expect("prepare");
    git(&root, &["commit", "-q", "-am", "chore: release 9.9.9"]);
    tag(&root, &GitWriter::plain()).expect("tag");
    git(&root, &["push", "-q", "origin", "main"]);
    let commit = git(&root, &["rev-list", "-n1", "9.9.9"]);

    let story = root.join(".engineering/evidence/story/some-story");
    fs::create_dir_all(&story).expect("mkdir");
    let other = serde_json::json!({
        "change": {"change": "evidence", "kind": "test_result",
                   "source": format!("task check exit 0 at {}", &commit[..7])}
    });
    fs::write(story.join("gate.json"), other.to_string()).expect("write");

    let aep = Calls::default();
    record(&root, &OneRun, &aep).expect("record");
    let calls = aep.0.borrow();
    assert_eq!(
        calls.len(),
        1,
        "the release epic has no record for {commit}, and record wrote none: {calls:?}"
    );
    assert!(calls[0].contains(&"epic:planning-on-entity-runtime".to_owned()));
}
