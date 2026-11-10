//! A Git store of a thousand closed stories packs and unpacks with every answer unchanged: only
//! the tracked file count moves, and it moves by exactly the files packed and the packs written.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Instant;

const IMPLEMENTED: usize = 400;
const REJECTED: usize = 300;
const ARCHIVED: usize = 300;
const OPEN: usize = 40;
const REVIEWED: usize = 150;

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

fn git(project: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=tester",
            "-c",
            "user.email=tester@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(project)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {}: {}{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn commit(project: &Path, message: &str) {
    git(project, &["add", "--all"]);
    git(
        project,
        &["commit", "--quiet", "--no-verify", "-m", message],
    );
}

fn aep(project: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(args)
        .current_dir(project)
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

/// Runs `aep plan artifact <args>` and requires it to succeed.
fn ok(project: &Path, args: &[&str]) {
    let mut full = vec!["plan", "artifact"];
    full.extend_from_slice(args);
    let output = aep(project, &full);
    assert!(
        output.status.success(),
        "`{}` failed: {}",
        args.join(" "),
        said(&output)
    );
}

/// Runs `aep plan store <args>`, requires success, and returns stdout.
fn store_ok(project: &Path, args: &[&str]) -> String {
    let mut full = vec!["plan", "store"];
    full.extend_from_slice(args);
    let output = aep(project, &full);
    assert!(
        output.status.success(),
        "`store {}` failed: {}",
        args.join(" "),
        said(&output)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn tracked_under_engineering(project: &Path) -> usize {
    git(project, &["ls-files", ".engineering"]).lines().count()
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("a writable directory");
    std::fs::write(path, text).expect("a writable file");
}

fn planning(project: &Path, kind: &str, name: &str) -> PathBuf {
    project
        .join(".engineering/planning")
        .join(kind)
        .join(format!("{name}.md"))
}

fn evidence_directory(project: &Path, name: &str) -> PathBuf {
    project.join(".engineering/evidence/story").join(name)
}

/// The template files one shape leaves, read back so the clones can rename them.
struct Template {
    document: String,
    evidence: Vec<(String, String)>,
}

fn take_template(project: &Path, kind: &str, name: &str) -> Template {
    let path = planning(project, kind, name);
    let document = std::fs::read_to_string(&path).expect("the template reads");
    std::fs::remove_file(&path).expect("the template is removable");
    let mut evidence = Vec::new();
    let directory = evidence_directory(project, name);
    if directory.exists() {
        for entry in std::fs::read_dir(&directory).expect("readable") {
            let entry = entry.expect("an entry");
            evidence.push((
                entry.file_name().into_string().expect("a printable name"),
                std::fs::read_to_string(entry.path()).expect("reads"),
            ));
        }
        std::fs::remove_dir_all(&directory).expect("removable");
    }
    Template { document, evidence }
}

/// Writes one clone of a story template named `name`, the template's name replaced throughout.
fn clone_story(project: &Path, template: &Template, from: &str, name: &str) {
    write(
        &planning(project, "story", name),
        &template.document.replace(from, name),
    );
    for (file, text) in &template.evidence {
        write(
            &evidence_directory(project, name).join(file),
            &text.replace(from, name),
        );
    }
}

fn story_with_evidence(project: &Path, name: &str, rungs: &[&str]) {
    ok(project, &["new", "story", name, "--title", name]);
    let id = format!("story:{name}");
    for rung in rungs {
        if *rung == "evidence" {
            ok(
                project,
                &[
                    "evidence",
                    &id,
                    "--kind",
                    "test_result",
                    "--source",
                    "task check",
                    "--at",
                    "2026-09-27T10:00:00Z",
                ],
            );
        } else {
            ok(project, &["move", &id, "--to", rung]);
        }
    }
}

fn findings_body(message: &str) -> String {
    format!(
        "# Attack\n\n## Findings\n\n```findings\n- file: src/lib.rs\n  line: 10\n  \
         category: correctness\n  severity: blocker\n  verdict: CONFIRMED\n  \
         origin: introduced\n  message: {message}\n```\n"
    )
}

fn review(project: &Path, name: &str, subject: &str, message: &str) {
    let body = project.join("review-body.md");
    write(&body, &findings_body(message));
    ok(
        project,
        &[
            "new",
            "review-result",
            name,
            "--title",
            name,
            "--relate",
            &format!("reviews:{subject}"),
            "--from",
            body.to_str().expect("a printable path"),
        ],
    );
    ok(
        project,
        &["move", &format!("review-result:{name}"), "--to", "archived"],
    );
    std::fs::remove_file(&body).expect("removable");
}

/// A committed `aep.project/5` Git store of `IMPLEMENTED + REJECTED + ARCHIVED` closed stories
/// with evidence, archived reviews with findings blocks, and open stories relating to closed ones.
/// Five shapes go through the CLI once; the rest are renamed clones, then one commit.
fn generated_store() -> PathBuf {
    let project = Path::new(env!("CARGO_TARGET_TMPDIR")).join("store-pack-end-to-end");
    if project.exists() {
        std::fs::remove_dir_all(&project).expect("the previous scratch project is removable");
    }
    let engineering = project.join(".engineering");
    std::fs::create_dir_all(&engineering).expect("writable");
    git(&project, &["init", "--quiet"]);
    write(
        &engineering.join("project.yaml"),
        &format!(
            "version: aep.project/5\nplanning_scope: store-pack-end-to-end\nprotocol: adp/1\n\
             profile: development.standard\nprotocols: {}\nstore:\n  git: {{}}\n",
            relative(&engineering, &repository())
        ),
    );

    story_with_evidence(
        &project,
        "tpl-impl",
        &["proposed", "active", "evidence", "implemented"],
    );
    story_with_evidence(&project, "tpl-rej", &["proposed", "evidence", "rejected"]);
    story_with_evidence(
        &project,
        "tpl-arc",
        &["proposed", "active", "evidence", "archived"],
    );
    story_with_evidence(&project, "tpl-open", &["proposed", "active", "evidence"]);
    ok(
        &project,
        &["relate", "story:tpl-open", "depends_on", "story:tpl-impl"],
    );
    review(&project, "tpl-rev", "story:tpl-impl", "tpl-msg");

    let implemented = take_template(&project, "story", "tpl-impl");
    let rejected = take_template(&project, "story", "tpl-rej");
    let archived = take_template(&project, "story", "tpl-arc");
    let open = take_template(&project, "story", "tpl-open");
    let review_path = planning(&project, "review-result", "tpl-rev");
    let reviewed = std::fs::read_to_string(&review_path).expect("reads");
    std::fs::remove_file(&review_path).expect("removable");

    for index in 0..IMPLEMENTED {
        clone_story(
            &project,
            &implemented,
            "tpl-impl",
            &format!("impl-{index:04}"),
        );
    }
    for index in 0..REJECTED {
        clone_story(&project, &rejected, "tpl-rej", &format!("rej-{index:04}"));
    }
    for index in 0..ARCHIVED {
        clone_story(&project, &archived, "tpl-arc", &format!("arc-{index:04}"));
    }
    for index in 0..OPEN {
        let name = format!("open-{index:04}");
        let mut template = Template {
            document: open
                .document
                .replace("story:tpl-impl", &format!("story:impl-{index:04}")),
            evidence: open.evidence.clone(),
        };
        template.document = template.document.replace("tpl-open", &name);
        clone_story(&project, &template, "tpl-open", &name);
    }
    let mut reviews = Vec::new();
    for index in 0..REVIEWED {
        reviews.push((
            format!("rev-impl-{index:04}-a"),
            format!("story:impl-{index:04}"),
        ));
    }
    reviews.push(("rev-impl-0001-b".to_owned(), "story:impl-0001".to_owned()));
    reviews.push(("rev-open-0001-a".to_owned(), "story:open-0001".to_owned()));
    reviews.push(("rev-open-0001-b".to_owned(), "story:open-0001".to_owned()));
    for (index, (name, subject)) in reviews.iter().enumerate() {
        let text = reviewed
            .replace("review-result:tpl-rev", &format!("review-result:{name}"))
            .replace("story:tpl-impl", subject)
            .replace("tpl-rev", name)
            .replace("tpl-msg", &format!("finding number {}", index % 3));
        write(&planning(&project, "review-result", name), &text);
    }
    commit(&project, "a loose store of a thousand closed stories");
    project
}

/// What one read verb printed and how it exited.
#[derive(Debug, PartialEq, Eq)]
struct Answer {
    stdout: Vec<u8>,
    code: Option<i32>,
}

fn answers(project: &Path) -> Vec<(String, Answer)> {
    let mut calls: Vec<Vec<String>> = vec![
        vec!["validate".into()],
        vec!["list".into(), "--status".into(), "implemented".into()],
        vec!["graph".into()],
    ];
    for id in [
        "story:impl-0001",
        "story:impl-0399",
        "story:rej-0007",
        "story:arc-0299",
        "review-result:rev-impl-0001-b",
        "story:open-0001",
    ] {
        for verb in ["history", "explain", "findings"] {
            let deep = matches!(id, "story:impl-0001" | "story:rej-0007" | "story:open-0001");
            if verb == "history" || deep {
                calls.push(vec![verb.into(), id.into()]);
            }
        }
    }
    calls
        .into_iter()
        .map(|call| {
            let mut full = vec!["plan".to_owned(), "artifact".to_owned()];
            full.extend(call.iter().cloned());
            let arguments: Vec<&str> = full.iter().map(String::as_str).collect();
            let call_started = Instant::now();
            let output = aep(project, &arguments);
            eprintln!(
                "  {:6.2}s {}",
                call_started.elapsed().as_secs_f64(),
                call.join(" ")
            );
            (
                call.join(" "),
                Answer {
                    stdout: output.stdout,
                    code: output.status.code(),
                },
            )
        })
        .collect()
}

fn assert_same(label: &str, expected: &[(String, Answer)], found: &[(String, Answer)]) {
    assert_eq!(expected.len(), found.len());
    for ((call, before), (_, after)) in expected.iter().zip(found) {
        assert_eq!(before.code, after.code, "{label}: exit of `{call}`");
        assert!(
            before.stdout == after.stdout,
            "{label}: stdout of `{call}` differs\n--- before\n{}\n--- after\n{}",
            String::from_utf8_lossy(&before.stdout),
            String::from_utf8_lossy(&after.stdout)
        );
    }
}

#[test]
fn a_thousand_closed_stories_pack_and_unpack_with_every_answer_unchanged() {
    let started = Instant::now();
    let project = generated_store();

    let loose = tracked_under_engineering(&project);
    let before = answers(&project);
    assert_eq!(before[0].0, "validate");
    assert_eq!(before[0].1.code, Some(0), "the generated store validates");
    let listed = String::from_utf8_lossy(&before[1].1.stdout).into_owned();
    assert!(
        listed.contains("story:impl-0001") && !listed.contains("story:rej-0001"),
        "{listed}"
    );
    let findings = String::from_utf8_lossy(&before[5].1.stdout).into_owned();
    assert!(
        findings.contains("finding number"),
        "the findings ledger answers: {findings}"
    );

    let packed_output = store_ok(&project, &["pack"]);
    let removed = packed_output
        .lines()
        .filter(|line| line.starts_with("  remove "))
        .count();
    commit(&project, "packed");
    let packed = tracked_under_engineering(&project);
    let packs = std::fs::read_dir(project.join(".engineering/packs"))
        .expect("the packs directory exists")
        .map(|entry| entry.expect("an entry").file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    let pack_files = packs
        .iter()
        .filter(|name| Path::new(name).extension().is_some_and(|e| e == "pack"))
        .count();
    let attributes = packs
        .iter()
        .filter(|name| *name == ".gitattributes")
        .count();
    let after_pack = answers(&project);

    store_ok(&project, &["unpack", "--all"]);
    commit(&project, "unpacked");
    let unpacked = tracked_under_engineering(&project);
    let after_unpack = answers(&project);

    let wall = started.elapsed();
    eprintln!("tracked files under .engineering before pack:  {loose}");
    eprintln!(
        "tracked files under .engineering after pack:   {packed} ({removed} packed, {pack_files} pack(s), {attributes} attributes file)"
    );
    eprintln!("tracked files under .engineering after unpack: {unpacked}");
    eprintln!("wall time: {:.1}s", wall.as_secs_f64());

    assert!(
        removed >= IMPLEMENTED + REJECTED + ARCHIVED,
        "every closed story was packed: {removed}"
    );
    assert_eq!(
        packed,
        loose - removed + pack_files + attributes,
        "after pack: before minus the files packed plus the packs written"
    );
    assert!(packed < loose / 10, "the store shrank: {packed} of {loose}");
    assert_eq!(unpacked, loose, "after unpack: the count before");

    assert_same("after pack", &before, &after_pack);
    assert_same("after unpack", &before, &after_unpack);
}
