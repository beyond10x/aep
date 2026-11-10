//! A packed record is the same record it was when loose: the verbs answer over `aep.pack/1` as over
//! files, a write that would rewrite a packed artifact refuses by name, and `aep.project/6` loads.
//!
//! The fixture is a committed `aep.project/5` store whose closed artifacts and their evidence the
//! test then moves into a pack rendered by the pack module (no verb writes packs yet), flips to
//! `aep.project/6`, and commits.

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

const CLOSED: [&str; 2] = ["closed-a", "closed-b"];

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

/// Moves the closed stories and every evidence file under their directories into one pack,
/// rendered by the pack module, flips the project to `aep.project/6` and commits. Returns the
/// pack's name.
fn pack_closed(project: &Path) -> String {
    let engineering = project.join(".engineering");
    let mut records: Vec<(String, Vec<u8>)> = Vec::new();
    let mut moved: Vec<PathBuf> = Vec::new();
    for name in CLOSED {
        let document = format!("planning/story/{name}.md");
        records.push((
            document.clone(),
            std::fs::read(engineering.join(&document)).expect("the story reads"),
        ));
        moved.push(engineering.join(&document));
        let directory = engineering.join(format!("evidence/story/{name}"));
        if let Ok(entries) = std::fs::read_dir(&directory) {
            for entry in entries.map(|entry| entry.expect("an entry")) {
                let file = entry.file_name().into_string().expect("a printable name");
                records.push((
                    format!("evidence/story/{name}/{file}"),
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
    for name in CLOSED {
        let _ = std::fs::remove_dir(engineering.join(format!("evidence/story/{name}")));
    }
    std::fs::write(
        engineering.join("project.yaml"),
        project_file(project, "aep.project/6"),
    )
    .expect("writable");
    commit(project, "pack the closed artifacts");
    name
}

/// The verbs whose answers must not depend on where a record is kept.
fn verbs() -> Vec<Vec<&'static str>> {
    vec![
        vec!["list"],
        vec!["list", "--status", "implemented"],
        vec!["graph"],
        vec!["show", "story:closed-a"],
        vec!["show", "story:closed-b"],
        vec!["history", "story:closed-a"],
        vec!["explain", "story:closed-a"],
        vec!["findings", "story:closed-a"],
        vec!["review-value"],
        vec!["blocked"],
        vec!["waves", "--status", "active"],
        vec!["spend"],
    ]
}

fn captured(project: &Path) -> Vec<(String, Option<i32>, String)> {
    verbs()
        .into_iter()
        .map(|args| {
            let output = aep(project, &args);
            (args.join(" "), output.status.code(), stdout(&output))
        })
        .collect()
}

#[test]
fn every_verb_answers_the_same_over_a_packed_store_as_over_loose_files() {
    let project = loose_store("same-answers");
    let before = captured(&project);
    for (verb, code, out) in &before {
        assert_eq!(
            *code,
            Some(0),
            "`{verb}` exits 0 over the loose store: {out}"
        );
        assert!(!out.trim().is_empty(), "`{verb}` prints something: {out}");
    }
    assert!(
        before
            .iter()
            .any(|(verb, _, out)| verb == "history story:closed-a"
                && out.contains("test_result recorded from task check")),
        "the fixture's history holds an evidence record: {before:?}"
    );

    let pack = pack_closed(&project);
    assert!(
        !project
            .join(".engineering/planning/story/closed-a.md")
            .exists(),
        "the story left the working tree"
    );
    assert!(project
        .join(format!(".engineering/packs/{pack}.pack"))
        .is_file());

    let after = captured(&project);
    assert_eq!(before, after);
    ok(&project, &["validate"]);
}

#[test]
fn an_open_artifact_still_validates_against_a_packed_target_and_relate_changes_only_its_file() {
    let project = loose_store("relate-open");
    pack_closed(&project);
    ok(&project, &["validate"]);

    ok(&project, &["new", "story", "open-d", "--title", "Open D"]);
    let before = files(&project);
    ok(
        &project,
        &["relate", "story:open-d", "depends_on", "story:closed-b"],
    );
    let after = files(&project);
    assert_eq!(
        changed(&before, &after),
        vec![".engineering/planning/story/open-d.md".to_owned()],
        "relate writes the open artifact's file and nothing else"
    );
    ok(&project, &["validate"]);
}

#[test]
fn new_for_a_packed_id_refuses_naming_the_pack() {
    let project = loose_store("new-packed");
    let pack = pack_closed(&project);
    let before = files(&project);
    let output = aep(&project, &["new", "story", "closed-a", "--title", "Again"]);
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    assert!(
        said(&output).contains(&format!("{pack}.pack")) && said(&output).contains("story:closed-a"),
        "names the pack and the id: {}",
        said(&output)
    );
    assert_eq!(files(&project), before, "nothing was written");
}

#[test]
fn a_write_that_would_rewrite_a_packed_artifact_refuses_naming_unpack_and_writes_nothing() {
    let project = loose_store("write-packed");
    let pack = pack_closed(&project);
    let body = project.join("replacement.md");
    std::fs::write(&body, "# Replaced\n").expect("writable");
    commit(&project, "a body file");

    let attempts: Vec<Vec<&str>> = vec![
        vec!["move", "story:closed-a", "--to", "archived"],
        vec![
            "body",
            "story:closed-a",
            "--from",
            body.to_str().expect("printable"),
        ],
        vec!["set", "story:closed-a", "--title", "Renamed"],
        vec!["scope", "story:closed-a", "--add", "crates/y/src/lib.rs"],
        vec!["relate", "story:closed-a", "informed_by", "story:open-c"],
        vec![
            "unrelate",
            "story:closed-a",
            "informed_by",
            "story:closed-b",
        ],
        vec![
            "redact",
            "story:closed-a",
            "--literal",
            "Acme",
            "--with",
            "X",
            "--reason",
            "test",
        ],
    ];
    for args in attempts {
        let before = files(&project);
        let output = aep(&project, &args);
        assert_eq!(
            output.status.code(),
            Some(1),
            "`{}`: {}",
            args.join(" "),
            said(&output)
        );
        let said = said(&output);
        assert!(
            said.contains("aep plan store unpack story:closed-a"),
            "`{}` names the way out: {said}",
            args.join(" ")
        );
        assert!(
            said.contains(&format!("{pack}.pack")),
            "`{}` names the pack: {said}",
            args.join(" ")
        );
        assert_eq!(
            files(&project),
            before,
            "`{}` left the tree as it was",
            args.join(" ")
        );
    }
}

#[test]
fn evidence_for_a_packed_id_writes_one_loose_file_and_changes_no_pack() {
    let project = loose_store("evidence-packed");
    let pack = pack_closed(&project);
    let before = files(&project);
    ok(
        &project,
        &[
            "evidence",
            "story:closed-a",
            "--kind",
            "test_result",
            "--source",
            "task check again",
            "--at",
            "2026-10-01T10:00:00Z",
        ],
    );
    let after = files(&project);
    let written = changed(&before, &after);
    assert_eq!(written.len(), 1, "one new file: {written:?}");
    assert!(
        written[0].starts_with(".engineering/evidence/story/closed-a/")
            && Path::new(&written[0])
                .extension()
                .is_some_and(|ext| ext == "json"),
        "{written:?}"
    );
    assert!(
        !before.iter().any(|(name, _)| name == &written[0]),
        "the file is new, not a rewrite"
    );
    let pack_file = format!(".engineering/packs/{pack}.pack");
    assert_eq!(
        before.iter().find(|(name, _)| name == &pack_file),
        after.iter().find(|(name, _)| name == &pack_file),
        "the pack is as it was"
    );
    let history = ok(&project, &["history", "story:closed-a"]);
    assert!(
        history.contains("task check again") && history.contains("task check"),
        "the loose record joins the packed ones: {history}"
    );
}

#[test]
fn a_v6_store_loads_in_every_verb_that_accepts_v5_and_a_v7_store_is_refused_by_name() {
    let project = loose_store("versions");
    std::fs::write(
        project.join(".engineering/project.yaml"),
        project_file(&project, "aep.project/6"),
    )
    .expect("writable");
    commit(&project, "v6");
    for args in verbs() {
        let output = aep(&project, &args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "`{}`: {}",
            args.join(" "),
            said(&output)
        );
    }
    ok(&project, &["validate"]);
    let doctor = Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(["doctor"])
        .current_dir(&project)
        .output()
        .expect("runs");
    assert!(
        !said(&doctor).contains("unsupported_protocol_version"),
        "doctor reads /6: {}",
        said(&doctor)
    );

    std::fs::write(
        project.join(".engineering/project.yaml"),
        project_file(&project, "aep.project/7"),
    )
    .expect("writable");
    for args in [vec!["list"], vec!["validate"]] {
        let output = aep(&project, &args);
        assert_ne!(output.status.code(), Some(0), "`{}`", args.join(" "));
        assert!(
            said(&output).contains("unsupported_protocol_version")
                && said(&output).contains("aep.project/7"),
            "`{}`: {}",
            args.join(" "),
            said(&output)
        );
    }
}

#[test]
fn migrate_git_refuses_a_v6_store_by_name_and_says_to_unpack_first() {
    let project = loose_store("migrate-v6");
    pack_closed(&project);
    let before = files(&project);
    let output = Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(["plan", "store", "migrate", "git"])
        .current_dir(&project)
        .output()
        .expect("runs");
    assert_ne!(output.status.code(), Some(0), "{}", said(&output));
    assert!(
        said(&output).contains("aep.project/6")
            && said(&output).contains("aep plan store unpack --all"),
        "{}",
        said(&output)
    );
    assert_eq!(files(&project), before, "nothing was written");
}

#[test]
fn doctor_counts_packed_evidence_as_it_counted_it_loose() {
    let project = loose_store("doctor-packed");
    let loose = Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(["doctor"])
        .current_dir(&project)
        .output()
        .expect("runs");
    pack_closed(&project);
    let packed = Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(["doctor"])
        .current_dir(&project)
        .output()
        .expect("runs");
    assert_eq!(said(&loose), said(&packed));
}
