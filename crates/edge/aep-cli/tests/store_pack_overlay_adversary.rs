//! Adversary cases: a packed record is the same record it was when loose, for the readers and the
//! fixtures `store_pack_overlay.rs` does not reach (a store with no loose document left, a pack that
//! does not parse, packed review-results, `validate`, a new review of a packed story).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use aep_backend_markdown::pack;

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

fn project_file(project: &Path, version: &str) -> String {
    format!(
        "version: {version}\nplanning_scope: pack-overlay-test\nprotocol: adp/1\n\
         profile: development.standard\nprotocols: {}\nstore:\n  git: {{}}\n",
        relative(&project.join(".engineering"), &repository())
    )
}

fn project(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("pack-overlay-{name}"));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("the previous scratch project is removable");
    }
    std::fs::create_dir_all(directory.join(".engineering")).expect("the scratch is writable");
    let status = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&directory)
        .status()
        .expect("git runs");
    assert!(status.success(), "git init");
    std::fs::write(
        directory.join(".engineering/project.yaml"),
        project_file(&directory, "aep.project/5"),
    )
    .expect("the project file is writable");
    directory
}

fn aep(project: &Path, args: &[&str]) -> Output {
    let mut full = vec!["plan", "artifact"];
    full.extend_from_slice(args);
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(&full)
        .current_dir(project)
        .env("AEP_ACTOR", "human:tester")
        .output()
        .expect("the binary runs")
}

fn ok(project: &Path, args: &[&str]) -> String {
    let output = aep(project, args);
    assert!(
        output.status.success(),
        "`{}` failed: {}{}",
        args.join(" "),
        stdout(&output),
        stderr(&output)
    );
    stdout(&output)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn said(output: &Output) -> String {
    format!("{}{}", stdout(output), stderr(output))
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

fn files(directory: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &Path, directory: &Path, found: &mut Vec<(String, Vec<u8>)>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.map(|entry| entry.expect("a directory entry")) {
            let path = entry.path();
            if entry.file_name() == ".git" {
                continue;
            }
            if path.is_dir() {
                walk(base, &path, found);
            } else {
                found.push((
                    path.strip_prefix(base)
                        .expect("below the base")
                        .display()
                        .to_string(),
                    std::fs::read(&path).expect("a listed file reads"),
                ));
            }
        }
    }
    let mut found = Vec::new();
    walk(directory, directory, &mut found);
    found.sort();
    found
}

fn changed(before: &[(String, Vec<u8>)], after: &[(String, Vec<u8>)]) -> Vec<String> {
    let mut names: Vec<&String> = before.iter().chain(after).map(|(name, _)| name).collect();
    names.sort();
    names.dedup();
    names
        .into_iter()
        .filter(|name| {
            let find = |set: &[(String, Vec<u8>)]| {
                set.iter()
                    .find(|(candidate, _)| candidate == *name)
                    .map(|(_, bytes)| bytes.clone())
            };
            find(before) != find(after)
        })
        .map(Clone::clone)
        .collect()
}

/// A committed `aep.project/5` store: two closed stories with evidence, two reviews of one of them,
/// a review outcome, a task spend, and an open story that depends on a closed one.
#[allow(clippy::too_many_lines)]
fn loose_store(name: &str) -> PathBuf {
    let project = project(name);
    let drafts = project.join("drafts");
    std::fs::create_dir_all(&drafts).expect("writable");

    ok(
        &project,
        &["new", "story", "closed-b", "--title", "Closed B"],
    );
    for rung in ["proposed", "active", "archived"] {
        ok(&project, &["move", "story:closed-b", "--to", rung]);
    }
    ok(
        &project,
        &[
            "new",
            "story",
            "closed-a",
            "--title",
            "Closed A for Acme",
            "--relate",
            "informed_by:story:closed-b",
        ],
    );
    for rung in ["proposed", "active"] {
        ok(&project, &["move", "story:closed-a", "--to", rung]);
    }
    ok(
        &project,
        &[
            "evidence",
            "story:closed-a",
            "--kind",
            "test_result",
            "--source",
            "task check",
            "--at",
            "2026-09-28T10:00:00Z",
        ],
    );
    ok(&project, &["move", "story:closed-a", "--to", "implemented"]);

    for (review, line) in [("attack-1", 40), ("attack-2", 41)] {
        let body = drafts.join(format!("{review}.md"));
        std::fs::write(
            &body,
            format!(
                "# Attack\n\n## Findings\n\n```findings\n- file: src/a.rs\n  line: {line}\n  \
                 category: correctness\n  severity: blocker\n  verdict: CONFIRMED\n  \
                 origin: introduced\n  message: The loop never advances\n```\n"
            ),
        )
        .expect("writable");
        ok(
            &project,
            &[
                "new",
                "review-result",
                review,
                "--title",
                review,
                "--owner",
                "agent:adversary",
                "--relate",
                "reviews:story:closed-a",
                "--from",
                body.to_str().expect("printable"),
            ],
        );
    }
    ok(
        &project,
        &[
            "evidence",
            "story:closed-a",
            "--kind",
            "review_outcome",
            "--review",
            "review-result:attack-1",
            "--outcome",
            "fixed",
            "--ref",
            "runs/fix.manifest.yaml",
            "--at",
            "2026-09-29T10:00:00Z",
        ],
    );

    let transcript = project.join("runs/closed-a.jsonl");
    std::fs::create_dir_all(transcript.parent().expect("a parent")).expect("writable");
    std::fs::write(
        &transcript,
        concat!(
            r#"{"type":"system","subtype":"init","session_id":"5c0e7a1b-0000-4000-8000-000000000001","model":"claude-sonnet-5","cwd":"/work","tools":[],"permissionMode":"default"}"#,
            "\n",
            r#"{"type":"assistant","request_id":"req_0","session_id":"5c0e7a1b-0000-4000-8000-000000000001","message":{"model":"claude-sonnet-5","id":"msg_0","type":"message","role":"assistant","content":[{"type":"text","text":"step 0"}]}}"#,
            "\n",
            r#"{"type":"result","subtype":"success","is_error":false,"num_turns":1,"session_id":"5c0e7a1b-0000-4000-8000-000000000001"}"#,
            "\n",
        ),
    )
    .expect("writable");
    ok(
        &project,
        &[
            "evidence",
            "story:closed-a",
            "--kind",
            "task_spend",
            "--from",
            transcript.to_str().expect("printable"),
            "--at",
            "2026-09-30T10:00:00Z",
        ],
    );
    std::fs::remove_dir_all(&drafts).expect("removable");

    ok(
        &project,
        &[
            "new",
            "story",
            "open-c",
            "--title",
            "Open C",
            "--relate",
            "depends_on:story:closed-a",
        ],
    );
    ok(&project, &["move", "story:open-c", "--to", "proposed"]);
    ok(&project, &["move", "story:open-c", "--to", "active"]);
    ok(
        &project,
        &["scope", "story:open-c", "--add", "crates/x/src/lib.rs"],
    );
    commit(&project, "a loose store");
    ok(&project, &["validate"]);
    project
}

/// Moves the documents of `ids` and every evidence file under their directories into one pack,
/// flips the project to `aep.project/6` and commits. Returns the pack's name.
fn pack_ids(project: &Path, ids: &[(&str, &str)]) -> String {
    let engineering = project.join(".engineering");
    let mut records: Vec<(String, Vec<u8>)> = Vec::new();
    let mut moved: Vec<PathBuf> = Vec::new();
    for (kind, name) in ids {
        let document = format!("planning/{kind}/{name}.md");
        records.push((
            document.clone(),
            std::fs::read(engineering.join(&document)).expect("the document reads"),
        ));
        moved.push(engineering.join(&document));
        let directory = engineering.join(format!("evidence/{kind}/{name}"));
        if let Ok(entries) = std::fs::read_dir(&directory) {
            for entry in entries.map(|entry| entry.expect("an entry")) {
                let file = entry.file_name().into_string().expect("a printable name");
                records.push((
                    format!("evidence/{kind}/{name}/{file}"),
                    std::fs::read(entry.path()).expect("the record reads"),
                ));
                moved.push(entry.path());
            }
        }
    }
    let bytes = pack::render(&records).expect("the pack renders");
    let name = pack::name(&bytes);
    std::fs::create_dir_all(engineering.join("packs")).expect("writable");
    std::fs::write(
        engineering.join("packs").join(pack::file_name(&name)),
        bytes,
    )
    .expect("writable");
    for path in moved {
        std::fs::remove_file(path).expect("removable");
    }
    for (kind, name) in ids {
        let _ = std::fs::remove_dir(engineering.join(format!("evidence/{kind}/{name}")));
    }
    std::fs::write(
        engineering.join("project.yaml"),
        project_file(project, "aep.project/6"),
    )
    .expect("writable");
    commit(project, "pack");
    name
}

fn doctor(project: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(["doctor"])
        .current_dir(project)
        .output()
        .expect("runs")
}

/// A store whose every artifact is closed and packed has no loose document left, and Git keeps no
/// empty directory: a fresh clone of it has no `.engineering/planning/` at all. The store is still
/// there, in its packs, and every reader must say so.
#[test]
fn a_store_whose_every_artifact_is_packed_is_still_a_store_without_a_planning_directory() {
    let project = project("adv-all-packed");
    ok(&project, &["new", "story", "done", "--title", "Done"]);
    for rung in ["proposed", "active", "archived"] {
        ok(&project, &["move", "story:done", "--to", rung]);
    }
    commit(&project, "one closed story");
    let pack = pack_ids(&project, &[("story", "done")]);
    std::fs::remove_dir_all(project.join(".engineering/planning"))
        .expect("the empty planning directory is removable, as a clone never has it");

    let list = ok(&project, &["list"]);
    assert!(
        list.contains("story:done"),
        "the packed story is listed: {list}"
    );

    let output = doctor(&project);
    let said = said(&output);
    assert!(
        !said.contains("there is no planning store"),
        "doctor reports a store held in .engineering/packs/{pack}.pack as absent: {said}"
    );
}

/// A pack that does not parse hides every record it holds, and `MarkdownStore::load` stops before
/// reading the loose ones too. A checkout that rewrote the pack's line ends (`core.autocrlf`) is
/// the ordinary way to get one. A read verb must then name the pack, not answer an empty store with
/// a warning that one document is missing.
#[test]
fn a_pack_that_does_not_parse_is_named_by_the_read_verbs() {
    let project = loose_store("adv-corrupt-pack");
    let pack = pack_ids(&project, &[("story", "closed-a"), ("story", "closed-b")]);
    let file = project.join(format!(".engineering/packs/{pack}.pack"));
    let bytes = std::fs::read(&file).expect("the pack reads");
    let mut crlf = Vec::with_capacity(bytes.len());
    for byte in bytes {
        if byte == b'\n' {
            crlf.push(b'\r');
        }
        crlf.push(byte);
    }
    std::fs::write(&file, crlf).expect("writable");

    for args in [
        vec!["list"],
        vec!["history", "story:closed-a"],
        vec!["show", "story:open-c"],
    ] {
        let output = aep(&project, &args);
        let said = said(&output);
        assert!(
            output.status.code() != Some(0) || said.contains(&format!("{pack}.pack")),
            "`{}` exits 0 over a pack that does not parse and never names it: {said}",
            args.join(" ")
        );
    }
}

/// `findings`, `review-value` and `show` read review-results from the overlay: archived
/// review-results are closed and packed with the story they review.
#[test]
fn findings_answer_the_same_when_the_review_results_are_packed_too() {
    let project = loose_store("adv-packed-reviews");
    for review in ["attack-1", "attack-2"] {
        ok(
            &project,
            &[
                "move",
                &format!("review-result:{review}"),
                "--to",
                "archived",
            ],
        );
    }
    commit(&project, "archive the reviews");
    let verbs: Vec<Vec<&str>> = vec![
        vec!["list"],
        vec!["show", "review-result:attack-1"],
        vec!["findings", "story:closed-a"],
        vec!["review-value"],
        vec!["history", "review-result:attack-2"],
        vec!["explain", "story:closed-a"],
    ];
    let capture = |project: &Path| -> Vec<(String, Option<i32>, String)> {
        verbs
            .iter()
            .map(|args| {
                let output = aep(project, args);
                (args.join(" "), output.status.code(), stdout(&output))
            })
            .collect()
    };
    let before = capture(&project);
    pack_ids(
        &project,
        &[
            ("story", "closed-a"),
            ("story", "closed-b"),
            ("review-result", "attack-1"),
            ("review-result", "attack-2"),
        ],
    );
    assert!(!project
        .join(".engineering/planning/review-result/attack-1.md")
        .exists());
    assert_eq!(before, capture(&project));
}

/// `files_read` counts packed records (coordinator's decision), so `validate` says the same thing
/// about a store before and after its closed artifacts are packed.
#[test]
fn validate_says_the_same_before_and_after_the_closed_artifacts_are_packed() {
    let project = loose_store("adv-validate-same");
    let before = aep(&project, &["validate"]);
    pack_ids(&project, &[("story", "closed-a"), ("story", "closed-b")]);
    let after = aep(&project, &["validate"]);
    assert_eq!(
        (before.status.code(), stdout(&before)),
        (after.status.code(), stdout(&after))
    );
}

/// A new review of a packed story is a new file; the story it reviews is not rewritten.
#[test]
fn a_new_review_of_a_packed_story_writes_only_the_review() {
    let project = loose_store("adv-review-packed");
    pack_ids(&project, &[("story", "closed-a"), ("story", "closed-b")]);
    let before = files(&project);
    ok(
        &project,
        &[
            "new",
            "review-result",
            "attack-3",
            "--title",
            "attack-3",
            "--owner",
            "agent:adversary",
            "--relate",
            "reviews:story:closed-a",
        ],
    );
    assert_eq!(
        changed(&before, &files(&project)),
        vec![".engineering/planning/review-result/attack-3.md".to_owned()]
    );
}
