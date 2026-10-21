//! Adversarial cases for `aep plan artifact redact` and the replay `validate` runs
//! (story:immutable-record-text-is-redacted-by-record).
//!
//! Each test builds a disposable Git-native project under Cargo's per-target scratch directory and
//! drives the real binary against it, as `tests/git_store.rs` does.

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

fn project(name: &str) -> PathBuf {
    let directory =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("redaction-adversary-{name}"));
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
            "version: aep.project/5\nplanning_scope: redaction-adversary\nprotocol: adp/1\n\
             profile: development.standard\nprotocols: {}\nstore:\n  git: {{}}\n",
            relative(&engineering, &repository())
        ),
    )
    .expect("the project file is writable");
    directory
}

fn aep(directory: &Path, args: &[&str]) -> Output {
    let mut full = vec!["plan", "artifact"];
    full.extend_from_slice(args);
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(&full)
        .current_dir(directory)
        .env("AEP_ACTOR", "human:tester")
        .output()
        .expect("the binary runs")
}

fn said(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn ok(directory: &Path, args: &[&str]) -> String {
    let output = aep(directory, args);
    assert!(
        output.status.success(),
        "`{}` failed: {}",
        args.join(" "),
        said(&output)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn commit(project: &Path, message: &str) {
    for args in [
        vec!["add", "--all"],
        vec![
            "-c",
            "user.name=tester",
            "-c",
            "user.email=tester@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "--no-verify",
            "-m",
            message,
        ],
    ] {
        let status = Command::new("git")
            .args(&args)
            .current_dir(project)
            .status()
            .expect("git runs");
        assert!(status.success(), "git {}", args.join(" "));
    }
}

/// Every file below `directory` with its text.
fn texts(directory: &Path) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(next) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&next) else {
            continue;
        };
        for entry in entries.map(|entry| entry.expect("a directory entry")) {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let bytes = std::fs::read(&path).expect("a listed file reads");
                found.push((path, String::from_utf8_lossy(&bytes).into_owned()));
            }
        }
    }
    found.sort();
    found
}

fn fold(text: &str) -> String {
    text.chars().flat_map(char::to_lowercase).collect()
}

/// The files under `.engineering/` that hold `literal` in some letter case.
fn holding(project: &Path, literal: &str) -> Vec<String> {
    let folded = fold(literal);
    texts(&project.join(".engineering"))
        .into_iter()
        .filter(|(_, text)| fold(text).contains(&folded))
        .map(|(path, text)| format!("{}:\n{text}", path.display()))
        .collect()
}

/// A committed `review-result:looked` reviewing `story:subject`, with `extra` arguments to `new`.
fn committed_review(name: &str, title: &str, body: &str, extra: &[&str]) -> PathBuf {
    let project = project(name);
    let file =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("redaction-adversary-{name}.md"));
    std::fs::write(&file, body).expect("the body is written");
    ok(&project, &["new", "story", "subject", "--title", "Subject"]);
    let mut args = vec![
        "new",
        "review-result",
        "looked",
        "--title",
        title,
        "--relate",
        "reviews:story:subject",
        "--from",
        file.to_str().expect("a printable path"),
    ];
    args.extend_from_slice(extra);
    ok(&project, &args);
    commit(&project, "review");
    ok(&project, &["validate"]);
    project
}

const BODY: &str = "## Verdict\n\nAcme approved it.\n";

/// The verb refuses a `--with` that holds the literal, and must treat `--reason` the same way:
/// the reason is written verbatim into the evidence record and printed by `history`, so a reason
/// that names what is being taken out puts it straight back under `.engineering/`.
#[test]
fn a_reason_naming_the_literal_does_not_put_it_back_in_the_store() {
    let project = committed_review("reason", "Looked", BODY, &[]);
    let before = texts(&project.join(".engineering"));
    let output = aep(
        &project,
        &[
            "redact",
            "review-result:looked",
            "--literal",
            "Acme",
            "--with",
            "customer A",
            "--reason",
            "the record names Acme, a customer",
        ],
    );
    if output.status.success() {
        let left = holding(&project, "Acme");
        assert!(
            left.is_empty(),
            "redact succeeded and the literal is still under .engineering/: {left:#?}"
        );
        let history = ok(&project, &["history", "review-result:looked"]);
        assert!(
            !fold(&history).contains("acme"),
            "history prints the literal: {history}"
        );
    } else {
        assert_eq!(
            texts(&project.join(".engineering")),
            before,
            "a refusal writes nothing: {}",
            said(&output)
        );
    }
}

/// `new review-result --summary` is accepted, `set` is refused on a review-result, and `redact`
/// leaves the summary alone: the verb reports success and the literal stays in the same file.
#[test]
fn after_redact_no_field_of_the_review_keeps_the_literal() {
    let project = committed_review(
        "summary",
        "Looked at Acme",
        BODY,
        &["--summary", "Acme pass one"],
    );
    let said = ok(
        &project,
        &[
            "redact",
            "review-result:looked",
            "--literal",
            "Acme",
            "--with",
            "customer A",
            "--reason",
            "names a customer",
        ],
    );
    let left = holding(&project, "Acme");
    assert!(
        left.is_empty(),
        "redact said `{}` and the literal is still under .engineering/: {left:#?}",
        said.trim()
    );
    ok(&project, &["validate"]);
}

/// A hand-written redaction record is input `validate` reads; one with a width near `u64::MAX`
/// must be reported as a problem, not crash the validator.
#[test]
fn a_record_with_an_absurd_width_is_reported_not_a_panic() {
    let project = committed_review("width", "Looked", BODY, &[]);
    let document = project.join(".engineering/planning/review-result/looked.md");
    let text = std::fs::read_to_string(&document).expect("the document reads");
    std::fs::write(&document, text.replace("Acme approved", "Bcme approved"))
        .expect("the edit is written");
    let directory = project.join(".engineering/evidence/review-result/looked");
    std::fs::create_dir_all(&directory).expect("the evidence directory is writable");
    let record = serde_json::json!({
        "at": "2026-10-21T10:00:00Z",
        "actor": "human:tester",
        "artifact": "review-result:looked",
        "kind": "review-result",
        "revision": 2,
        "change": {
            "change": "redaction",
            "literal_sha256": "a".repeat(64),
            "literal_chars": u64::MAX,
            "replacement": "x",
            "count": 1,
            "reason": "r",
        },
    });
    std::fs::write(
        directory.join("20261021T100000Z-000-aaaaaaaaaaaa.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&record).expect("the record serialises")
        ),
    )
    .expect("the record is written");
    let output = aep(&project, &["validate"]);
    let text = said(&output);
    assert!(!text.contains("panicked"), "validate panicked: {text}");
    assert_eq!(
        output.status.code(),
        Some(1),
        "validate refuses with a problem: {text}"
    );
    assert!(text.contains("review-result:looked"), "{text}");
}

/// `İ` (U+0130) lowercases to two characters, `i` + U+0307, so `ACMEİ` lowercases to a
/// six-character string while the literal `Acmeİ` spans five. The lowercase spelling of the
/// literal is the literal "in another letter case", and the five-character window never sees it.
#[test]
fn a_literal_whose_lowercase_is_longer_leaves_no_letter_case_behind() {
    let project = committed_review(
        "dotted",
        "Looked",
        "## Verdict\n\nACME\u{130} approved it; acmei\u{307} signed.\n",
        &[],
    );
    ok(
        &project,
        &[
            "redact",
            "review-result:looked",
            "--literal",
            "Acme\u{130}",
            "--with",
            "customer A",
            "--reason",
            "names a customer",
        ],
    );
    let left = holding(&project, "Acme\u{130}");
    assert!(
        left.is_empty(),
        "the literal survives in its lowercase spelling: {left:#?}"
    );
}
