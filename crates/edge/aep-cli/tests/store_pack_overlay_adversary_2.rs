//! Adversary cases, pass 2: a pack that does not parse fails every read verb closed, naming the
//! pack, and lets no write verb write; `validate` names it and exits 1; the workspace reader and
//! the listing walk do not lose records without a word.

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

/// Rewrites the pack's line ends to CRLF, as a checkout with `core.autocrlf` does to a text file.
fn break_pack(project: &Path, pack: &str) {
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
}

/// The fix of pass 1 says a pack that does not parse fails every read verb, naming the pack file.
/// Each verb the story's acceptance lists, and the other read verbs, is asked over such a store.
#[test]
fn every_read_verb_refuses_a_store_whose_pack_does_not_parse_and_names_the_pack() {
    let project = loose_store("adv2-read-verbs");
    let pack = pack_ids(&project, &[("story", "closed-a"), ("story", "closed-b")]);
    break_pack(&project, &pack);
    let named = format!("{pack}.pack");

    let verbs: Vec<Vec<&str>> = vec![
        vec!["list"],
        vec!["list", "--status", "implemented"],
        vec!["show", "story:open-c"],
        vec!["show", "story:closed-a"],
        vec!["history", "story:open-c"],
        vec!["history", "story:closed-a"],
        vec!["explain", "story:open-c"],
        vec!["findings", "story:closed-a"],
        vec!["graph"],
        vec!["board"],
        vec!["blocked"],
        vec!["waves", "--status", "active"],
        vec!["review-value"],
        vec!["spend"],
    ];
    let mut open: Vec<String> = Vec::new();
    for args in &verbs {
        let output = aep(&project, args);
        let said = said(&output);
        if output.status.code() == Some(0) || !said.contains(&named) {
            open.push(format!(
                "`{}` exits {:?}{}: {said}",
                args.join(" "),
                output.status.code(),
                if said.contains(&named) {
                    ""
                } else {
                    " and does not name the pack"
                }
            ));
        }
    }
    let output = doctor(&project);
    let said = said(&output);
    if output.status.code() == Some(0) || !said.contains(&named) {
        open.push(format!(
            "`doctor` exits {:?} over {named}: {said}",
            output.status.code()
        ));
    }
    assert!(
        open.is_empty(),
        "read verbs that answer over a pack that does not parse:\n{}",
        open.join("\n")
    );
}

/// No write verb writes into a store whose pack does not parse: the records it hides are records a
/// write could create a second time.
#[test]
fn no_write_verb_writes_into_a_store_whose_pack_does_not_parse() {
    let project = loose_store("adv2-write-verbs");
    let pack = pack_ids(&project, &[("story", "closed-a"), ("story", "closed-b")]);
    break_pack(&project, &pack);
    let named = format!("{pack}.pack");
    let body = project
        .parent()
        .expect("a parent")
        .join("adv2-write-verbs-body.md");
    std::fs::write(&body, "# Open C\n\nA new body.\n").expect("writable");
    let body = body.to_str().expect("printable").to_owned();

    let verbs: Vec<Vec<&str>> = vec![
        vec!["new", "story", "closed-a", "--title", "Closed A again"],
        vec!["new", "story", "fresh", "--title", "Fresh"],
        vec!["move", "story:open-c", "--to", "archived"],
        vec!["body", "story:open-c", "--from", body.as_str()],
        vec!["set", "story:open-c", "--title", "Open C, renamed"],
        vec!["scope", "story:open-c", "--add", "crates/y/src/lib.rs"],
        vec!["relate", "story:open-c", "informed_by", "story:closed-b"],
        vec!["unrelate", "story:open-c", "depends_on", "story:closed-a"],
        vec![
            "redact",
            "story:open-c",
            "--literal",
            "Open",
            "--with",
            "Shut",
            "--reason",
            "test",
        ],
        vec![
            "evidence",
            "story:open-c",
            "--kind",
            "test_result",
            "--source",
            "task check",
            "--at",
            "2026-10-01T10:00:00Z",
        ],
        vec![
            "evidence",
            "story:closed-a",
            "--kind",
            "test_result",
            "--source",
            "task check",
            "--at",
            "2026-10-01T10:00:00Z",
        ],
    ];
    let mut wrote: Vec<String> = Vec::new();
    for args in &verbs {
        let before = files(&project);
        let output = aep(&project, args);
        let changed = changed(&before, &files(&project));
        if output.status.code() == Some(0) || !changed.is_empty() || !said(&output).contains(&named)
        {
            wrote.push(format!(
                "`{}` exits {:?}, changes {changed:?}: {}",
                args.join(" "),
                output.status.code(),
                said(&output)
            ));
        }
    }
    assert!(
        wrote.is_empty(),
        "write verbs over a pack that does not parse:\n{}",
        wrote.join("\n")
    );
}

/// `validate` is the one reader that keeps going over a pack that does not parse: it reports the
/// pack as a problem, by its file, and fails.
#[test]
fn validate_names_a_pack_that_does_not_parse_and_exits_1() {
    let project = loose_store("adv2-validate");
    let pack = pack_ids(&project, &[("story", "closed-a"), ("story", "closed-b")]);
    break_pack(&project, &pack);
    let output = aep(&project, &["validate"]);
    let said = said(&output);
    assert_eq!(output.status.code(), Some(1), "{said}");
    assert!(
        said.contains(&format!("{pack}.pack")),
        "validate does not name the pack: {said}"
    );
}

/// A workspace member's store reads through the same overlay (`Assembly::read`). One pack that
/// does not parse makes `MarkdownStore::load` return no document at all for that member, loose ones
/// included, and `workspace list` must not answer as if the member held nothing.
#[test]
fn workspace_list_does_not_drop_a_member_whose_pack_does_not_parse() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("pack-overlay-adv2-workspace");
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("removable");
    }
    std::fs::create_dir_all(root.join(".engineering")).expect("writable");
    std::fs::write(
        root.join(".engineering/workspace.yaml"),
        "version: aep.workspace/1\nmembers:\n  - name: one\n    source: ../one\n  - name: two\n    \
         source: ../two\n",
    )
    .expect("writable");
    let document = |id: &str, title: &str| {
        let (kind, _) = id.split_once(':').expect("kind:name");
        format!(
            "---\nformat: aep.planning-md/1\nid: {id}\nkind: {kind}\nstatus: draft\ntitle: \
             {title}\nrevision: 1\n---\n\n# {title}\n"
        )
    };
    let member = |name: &str| root.join(name).join(".engineering");
    for (name, id, title) in [
        ("one", "story:alpha", "Alpha"),
        ("two", "story:beta", "Beta"),
    ] {
        let directory = member(name).join("planning/story");
        std::fs::create_dir_all(&directory).expect("writable");
        std::fs::write(
            directory.join(format!("{}.md", id.split_once(':').expect("kind:name").1)),
            document(id, title),
        )
        .expect("writable");
    }
    let bytes = pack::render(&[("planning/story/gamma.md", document("story:gamma", "Gamma"))])
        .expect("renders");
    let name = pack::name(&bytes);
    std::fs::create_dir_all(member("one").join("packs")).expect("writable");
    let file = member("one").join("packs").join(pack::file_name(&name));
    std::fs::write(&file, &bytes).expect("writable");

    let list = || {
        Command::new(env!("CARGO_BIN_EXE_aep"))
            .args(["plan", "workspace", "list", "--root"])
            .arg(&root)
            .output()
            .expect("runs")
    };
    let output = list();
    assert!(
        stdout(&output).contains("one/story:gamma") && stdout(&output).contains("one/story:alpha"),
        "the workspace lists member one's packed and loose stories: {}",
        said(&output)
    );

    let mut crlf = Vec::new();
    for byte in bytes {
        if byte == b'\n' {
            crlf.push(b'\r');
        }
        crlf.push(byte);
    }
    std::fs::write(&file, crlf).expect("writable");
    let output = list();
    let said = said(&output);
    assert!(
        output.status.code() != Some(0) || said.contains(&format!("{name}.pack")),
        "`workspace list` exits 0 without member one and never names {name}.pack: {said}"
    );
}

/// The listing walk moved from `PathBuf` to `String` names and skips a name that is not UTF-8.
/// On an `aep.project/5` store with no pack, `validate` reported a stray file in an evidence
/// directory as "not an evidence record" whatever its name; it must still see it.
#[test]
fn validate_still_reports_a_stray_evidence_file_whose_name_is_not_utf8() {
    use std::os::unix::ffi::OsStrExt as _;

    let project = loose_store("adv2-non-utf8");
    let directory = project.join(".engineering/evidence/story/closed-a");
    let stray = directory.join(std::ffi::OsStr::from_bytes(b"notes-\xff.json"));
    std::fs::write(&stray, "not a record\n").expect("writable");
    commit(&project, "a stray file");
    let output = aep(&project, &["validate"]);
    let said = said(&output);
    assert_ne!(
        output.status.code(),
        Some(0),
        "validate passes a store with a file in an evidence directory that is no record: {said}"
    );
}

/// A packs directory that cannot be listed hides every packed record exactly as a pack that does
/// not parse does; `Overlay::packs` reads it as no packs at all.
#[test]
fn a_packs_directory_that_cannot_be_listed_is_not_read_as_no_packs() {
    use std::os::unix::fs::PermissionsExt as _;

    let project = loose_store("adv2-packs-unlisted");
    pack_ids(&project, &[("story", "closed-a"), ("story", "closed-b")]);
    let packs = project.join(".engineering/packs");
    std::fs::set_permissions(&packs, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    let unlisted = std::fs::read_dir(&packs).is_err();
    let output = aep(&project, &["list"]);
    std::fs::set_permissions(&packs, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    assert!(unlisted, "the test runs as a user the mode applies to");
    let said = said(&output);
    assert!(
        output.status.code() != Some(0) || said.contains("packs"),
        "`list` answers without the packed stories and says nothing: {said}"
    );
}

/// `evidence_beside` takes the project directory as an argument; the CLI passes
/// `project_directory()` (`AEP_PROJECT_DIR`), `Assembly::read` passes the constant `.engineering`.
/// Under `AEP_PROJECT_DIR=.aep` a member whose store is `.aep/planning` keeps its packs in
/// `.aep/packs`, which the member's own `list` reads and the workspace does not.
#[test]
fn workspace_list_reads_a_members_packs_where_the_member_itself_reads_them() {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("pack-overlay-adv2-project-dir");
    if base.exists() {
        std::fs::remove_dir_all(&base).expect("removable");
    }
    let root = base.join("root");
    let one = root.join("one");
    std::fs::create_dir_all(root.join(".aep")).expect("writable");
    std::fs::write(
        root.join(".aep/workspace.yaml"),
        "version: aep.workspace/1\nmembers:\n  - name: one\n    source: ../one\n    store: \
         .aep/planning\n",
    )
    .expect("writable");
    let document = |id: &str, title: &str| {
        let (kind, _) = id.split_once(':').expect("kind:name");
        format!(
            "---\nformat: aep.planning-md/1\nid: {id}\nkind: {kind}\nstatus: draft\ntitle: \
             {title}\nrevision: 1\n---\n\n# {title}\n"
        )
    };
    std::fs::create_dir_all(one.join(".aep/planning/story")).expect("writable");
    // `.aep` sits where `.engineering` would, so the relative `protocols` path is the same.
    std::fs::write(
        one.join(".aep/project.yaml"),
        project_file(&one, "aep.project/6"),
    )
    .expect("writable");
    std::fs::write(
        one.join(".aep/planning/story/alpha.md"),
        document("story:alpha", "Alpha"),
    )
    .expect("writable");
    let bytes = pack::render(&[("planning/story/gamma.md", document("story:gamma", "Gamma"))])
        .expect("renders");
    std::fs::create_dir_all(one.join(".aep/packs")).expect("writable");
    std::fs::write(
        one.join(".aep/packs")
            .join(pack::file_name(&pack::name(&bytes))),
        &bytes,
    )
    .expect("writable");

    let member = Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(["plan", "artifact", "list"])
        .current_dir(&one)
        .env("AEP_PROJECT_DIR", ".aep")
        .output()
        .expect("runs");
    assert!(
        stdout(&member).contains("story:gamma"),
        "the member's own list reads .aep/packs: {}",
        said(&member)
    );

    let workspace = Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(["plan", "workspace", "list", "--root"])
        .arg(&root)
        .env("AEP_PROJECT_DIR", ".aep")
        .output()
        .expect("runs");
    assert!(
        stdout(&workspace).contains("one/story:gamma"),
        "the workspace lists the member's loose story and not its packed one: {}",
        said(&workspace)
    );
}
