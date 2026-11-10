//! `validate` holds every pack to the bytes it replaced: each refusal has its own stable code, exit
//! 1, and names the pack file (and the record path where one exists).
//!
//! The fixture is a committed `aep.project/5` store whose closed artifacts and their evidence the
//! test moves into a pack rendered by the pack module (no verb writes packs here), flips to
//! `aep.project/6` and commits; each case then corrupts that store in one way.

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
        "version: {version}\nplanning_scope: validate-packs-test\nprotocol: adp/1\n\
         profile: development.standard\nprotocols: {}\nstore:\n  git: {{}}\n",
        relative(&project.join(".engineering"), &repository())
    )
}

fn project(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("validate-packs-{name}"));
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
        "`{}` failed: {}",
        args.join(" "),
        said(&output)
    );
    stdout(&output)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn said(output: &Output) -> String {
    format!(
        "{}{}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    )
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
            "--allow-empty",
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

fn engineering(project: &Path) -> PathBuf {
    project.join(".engineering")
}

/// A committed `aep.project/5` store: closed-a (implemented, one evidence record), closed-b
/// (archived) and an open story.
fn loose_store(name: &str) -> PathBuf {
    let project = project(name);
    ok(
        &project,
        &["new", "story", "closed-b", "--title", "Closed B"],
    );
    for rung in ["proposed", "active", "archived"] {
        ok(&project, &["move", "story:closed-b", "--to", rung]);
    }
    ok(
        &project,
        &["new", "story", "closed-a", "--title", "Closed A"],
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
    ok(&project, &["new", "story", "open-c", "--title", "Open C"]);
    commit(&project, "a loose store");
    ok(&project, &["validate"]);
    project
}

/// The store paths of closed-a and closed-b and of the evidence under them.
fn closed_paths(project: &Path) -> Vec<String> {
    let mut paths = vec![
        "planning/story/closed-a.md".to_owned(),
        "planning/story/closed-b.md".to_owned(),
    ];
    for name in ["closed-a", "closed-b"] {
        let directory = engineering(project).join(format!("evidence/story/{name}"));
        if let Ok(entries) = std::fs::read_dir(&directory) {
            for entry in entries.map(|entry| entry.expect("an entry")) {
                paths.push(format!(
                    "evidence/story/{name}/{}",
                    entry.file_name().into_string().expect("printable")
                ));
            }
        }
    }
    paths.sort();
    paths
}

/// The records of `paths` as the working tree holds them.
fn records_of(project: &Path, paths: &[String]) -> Vec<(String, Vec<u8>)> {
    paths
        .iter()
        .map(|path| {
            (
                path.clone(),
                std::fs::read(engineering(project).join(path)).expect("a record reads"),
            )
        })
        .collect()
}

/// Writes `bytes` as `<name>.pack` under the packs directory and returns the name.
fn write_pack(project: &Path, bytes: &[u8], name: &str) -> String {
    let packs = engineering(project).join("packs");
    std::fs::create_dir_all(&packs).expect("writable");
    std::fs::write(packs.join(pack::file_name(name)), bytes).expect("writable");
    name.to_owned()
}

/// Packs `paths`: writes the pack, removes the loose files, flips the project to `/6`.
fn pack_away(project: &Path, paths: &[String]) -> String {
    let bytes = pack::render(&records_of(project, paths)).expect("the pack renders");
    let name = write_pack(project, &bytes, &pack::name(&bytes));
    for path in paths {
        std::fs::remove_file(engineering(project).join(path)).expect("removable");
    }
    for name in ["closed-a", "closed-b"] {
        let _ = std::fs::remove_dir(engineering(project).join(format!("evidence/story/{name}")));
    }
    std::fs::write(
        engineering(project).join("project.yaml"),
        project_file(project, "aep.project/6"),
    )
    .expect("writable");
    name
}

/// The committed packed store: [`loose_store`] with everything closed moved into one pack.
fn packed_store(name: &str) -> (PathBuf, String) {
    let project = loose_store(name);
    let paths = closed_paths(&project);
    let pack = pack_away(&project, &paths);
    commit(&project, "pack the closed artifacts");
    (project, pack)
}

fn pack_file(project: &Path, name: &str) -> PathBuf {
    engineering(project)
        .join("packs")
        .join(pack::file_name(name))
}

/// Replaces the pack `old` with `bytes`, named by its content, and commits.
fn replace_pack(project: &Path, old: &str, bytes: &[u8]) -> String {
    std::fs::remove_file(pack_file(project, old)).expect("removable");
    let name = write_pack(project, bytes, &pack::name(bytes));
    commit(project, "corrupt the pack");
    name
}

fn parsed(project: &Path, name: &str) -> Vec<(String, Vec<u8>)> {
    let bytes = std::fs::read(pack_file(project, name)).expect("the pack reads");
    pack::Pack::parse(&bytes)
        .expect("the pack parses")
        .records()
        .iter()
        .map(|record| (record.path().to_owned(), record.bytes().to_vec()))
        .collect()
}

/// A pack written by hand: each record's declared digest is of its own bytes.
fn raw_pack(records: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = format!("{}\n", pack::FORMAT).into_bytes();
    for (path, bytes) in records {
        out.extend_from_slice(
            format!(
                "--- {path} {} sha256:{}\n",
                bytes.len(),
                pack::sha256_hex(bytes)
            )
            .as_bytes(),
        );
        out.extend_from_slice(bytes);
        out.push(b'\n');
    }
    out
}

/// `validate` must refuse with exit 1, print every one of `codes` and name `file`.
fn refused(project: &Path, codes: &[&str], file: Option<&str>) -> String {
    let output = aep(project, &["validate"]);
    let text = said(&output);
    assert_eq!(output.status.code(), Some(1), "validate refuses: {text}");
    for code in codes {
        assert!(text.contains(&format!("[{code}]")), "[{code}] in: {text}");
    }
    if let Some(file) = file {
        assert!(text.contains(file), "{file} named in: {text}");
    }
    text
}

fn valid(project: &Path) {
    let output = aep(project, &["validate"]);
    assert!(
        output.status.success(),
        "validate answers valid: {}",
        said(&output)
    );
    assert!(said(&output).contains("valid"), "{}", said(&output));
}

#[test]
fn a_committed_pack_of_closed_artifacts_and_evidence_is_valid() {
    let (project, _) = packed_store("valid");
    valid(&project);
}

#[test]
fn a_pack_with_a_bad_header_is_refused_by_format() {
    let (project, old) = packed_store("bad-header");
    let mut bytes = std::fs::read(pack_file(&project, &old)).expect("reads");
    bytes[..10].copy_from_slice(b"aep.pack/9");
    let name = replace_pack(&project, &old, &bytes);
    refused(
        &project,
        &["pack_not_parsed"],
        Some(&format!("{name}.pack")),
    );
}

#[test]
fn a_truncated_pack_is_refused_by_format() {
    let (project, old) = packed_store("truncated");
    let mut bytes = std::fs::read(pack_file(&project, &old)).expect("reads");
    bytes.truncate(bytes.len() - 9);
    let name = replace_pack(&project, &old, &bytes);
    refused(
        &project,
        &["pack_not_parsed"],
        Some(&format!("{name}.pack")),
    );
}

#[test]
fn a_pack_whose_name_is_not_its_content_digest_is_refused() {
    let (project, old) = packed_store("misnamed");
    let bytes = std::fs::read(pack_file(&project, &old)).expect("reads");
    std::fs::remove_file(pack_file(&project, &old)).expect("removable");
    write_pack(&project, &bytes, "0123456789abcdef");
    commit(&project, "rename");
    refused(
        &project,
        &["pack_name_not_digest"],
        Some("0123456789abcdef.pack"),
    );
}

#[test]
fn a_record_byte_changed_in_a_pack_is_refused_by_digest() {
    let (project, old) = packed_store("digest");
    let mut bytes = std::fs::read(pack_file(&project, &old)).expect("reads");
    let at = bytes
        .windows(8)
        .position(|window| window == b"Closed A")
        .expect("the title is in the pack");
    bytes[at] = b'c';
    let name = replace_pack(&project, &old, &bytes);
    let text = refused(
        &project,
        &["pack_record_digest_differs"],
        Some(&format!("{name}.pack")),
    );
    assert!(text.contains("planning/story/closed-a.md"), "{text}");
}

#[test]
fn a_record_changed_with_its_digest_and_name_recomputed_differs_from_the_committed_blob() {
    let (project, old) = packed_store("provenance");
    let mut records = parsed(&project, &old);
    for (path, bytes) in &mut records {
        if path == "planning/story/closed-a.md" {
            let text = String::from_utf8(bytes.clone()).expect("utf-8");
            *bytes = text.replace("Closed A", "Closed Z").into_bytes();
        }
    }
    let rendered = pack::render(&records).expect("renders");
    let name = replace_pack(&project, &old, &rendered);
    let text = refused(
        &project,
        &["pack_record_not_committed_bytes"],
        Some(&format!("{name}.pack")),
    );
    assert!(text.contains("planning/story/closed-a.md"), "{text}");
    assert!(
        !text.contains("[pack_record_digest_differs]") && !text.contains("[pack_name_not_digest]"),
        "only provenance fails: {text}"
    );
}

#[test]
fn unsafe_record_paths_are_refused_by_path() {
    for (index, path) in ["planning/../x.md", "/etc/x", "evidence/a\\b/c.json"]
        .into_iter()
        .enumerate()
    {
        let (project, old) = packed_store(&format!("path-{index}"));
        let bytes = raw_pack(&[(path, b"x\n")]);
        let name = replace_pack(&project, &old, &bytes);
        let text = refused(
            &project,
            &["pack_path_unsafe"],
            Some(&format!("{name}.pack")),
        );
        assert!(
            text.contains("x.md") || text.contains("etc") || text.contains("c.json"),
            "{text}"
        );
    }
}

#[test]
fn a_path_loose_and_packed_with_different_bytes_is_refused_naming_both_places() {
    let (project, pack_name) = packed_store("held-loose");
    let loose = engineering(&project).join("planning/story/closed-b.md");
    let mut bytes = parsed(&project, &pack_name)
        .into_iter()
        .find(|(path, _)| path == "planning/story/closed-b.md")
        .expect("held")
        .1;
    bytes.extend_from_slice(b"\nextra\n");
    std::fs::write(&loose, bytes).expect("writable");
    commit(&project, "a loose copy that differs");
    let text = refused(
        &project,
        &["pack_path_held_twice"],
        Some(&format!("{pack_name}.pack")),
    );
    assert!(text.contains("planning/story/closed-b.md"), "{text}");
}

#[test]
fn a_path_in_two_packs_with_different_bytes_is_refused_naming_both_packs() {
    let (project, first) = packed_store("held-twice-packs");
    let mut changed = parsed(&project, &first)
        .into_iter()
        .find(|(path, _)| path == "planning/story/closed-b.md")
        .expect("held");
    changed.1.extend_from_slice(b"\nextra\n");
    let bytes = pack::render(&[changed]).expect("renders");
    let second = write_pack(&project, &bytes, &pack::name(&bytes));
    commit(&project, "a second pack");
    refused(
        &project,
        &["pack_path_held_twice"],
        Some(&format!("{first}.pack")),
    );
    let text = refused(
        &project,
        &["pack_path_held_twice"],
        Some(&format!("{second}.pack")),
    );
    assert!(text.contains("planning/story/closed-b.md"), "{text}");
}

#[test]
fn a_path_held_twice_with_identical_bytes_is_admitted() {
    let (project, first) = packed_store("held-identical");
    let same = parsed(&project, &first)
        .into_iter()
        .find(|(path, _)| path == "planning/story/closed-b.md")
        .expect("held");
    std::fs::write(
        engineering(&project).join("planning/story/closed-b.md"),
        &same.1,
    )
    .expect("writable");
    let bytes = pack::render(&[same]).expect("renders");
    write_pack(&project, &bytes, &pack::name(&bytes));
    commit(&project, "the same record, loose and in a second pack");
    valid(&project);
}

#[test]
fn a_packed_artifact_that_is_not_closed_for_its_kind_is_refused() {
    let project = loose_store("not-closed");
    let paths = vec!["planning/story/open-c.md".to_owned()];
    let name = pack_away(&project, &paths);
    commit(&project, "pack an open story");
    let text = refused(
        &project,
        &["pack_artifact_not_closed"],
        Some(&format!("{name}.pack")),
    );
    assert!(text.contains("planning/story/open-c.md"), "{text}");
}

#[test]
fn packed_evidence_filed_under_another_artifact_is_refused() {
    let (project, old) = packed_store("misfiled");
    let mut records = parsed(&project, &old);
    let (path, bytes) = records
        .iter()
        .find(|(path, _)| path.starts_with("evidence/story/closed-a/"))
        .expect("an evidence record")
        .clone();
    records.retain(|(held, _)| *held != path);
    let moved = path.replace("closed-a", "closed-b");
    records.push((moved.clone(), bytes));
    records.sort();
    let rendered = pack::render(&records).expect("renders");
    let name = replace_pack(&project, &old, &rendered);
    let text = refused(
        &project,
        &["pack_evidence_misfiled"],
        Some(&format!("{name}.pack")),
    );
    assert!(text.contains(&moved), "{text}");
}

#[test]
fn a_pack_in_a_version_5_store_is_refused_naming_the_project_file() {
    let (project, _) = packed_store("pack-in-v5");
    std::fs::write(
        engineering(&project).join("project.yaml"),
        project_file(&project, "aep.project/5"),
    )
    .expect("writable");
    commit(&project, "back to version 5");
    refused(&project, &["pack_version_disagrees"], Some("project.yaml"));
}

#[test]
fn a_version_6_store_without_a_pack_is_refused_naming_the_project_file() {
    let project = loose_store("v6-without-pack");
    std::fs::write(
        engineering(&project).join("project.yaml"),
        project_file(&project, "aep.project/6"),
    )
    .expect("writable");
    commit(&project, "version 6 with nothing packed");
    refused(&project, &["pack_version_disagrees"], Some("project.yaml"));
}

#[test]
fn an_evidence_file_committed_twice_while_loose_and_then_packed_is_refused() {
    let project = loose_store("rewritten-then-packed");
    let paths = closed_paths(&project);
    let evidence = paths
        .iter()
        .find(|path| path.starts_with("evidence/"))
        .expect("an evidence record")
        .clone();
    let file = engineering(&project).join(&evidence);
    let mut bytes = std::fs::read(&file).expect("reads");
    bytes.push(b'\n');
    std::fs::write(&file, bytes).expect("writable");
    commit(&project, "an evidence file written twice");
    let name = pack_away(&project, &paths);
    commit(&project, "pack");
    let text = refused(
        &project,
        &["pack_record_rewritten"],
        Some(&format!("{name}.pack")),
    );
    assert!(text.contains(&evidence), "{text}");
}

#[test]
fn a_pack_written_and_the_loose_files_removed_but_not_committed_reports_no_rewrite() {
    let project = loose_store("uncommitted-pack");
    let paths = closed_paths(&project);
    pack_away(&project, &paths);
    let output = aep(&project, &["validate"]);
    let text = said(&output);
    assert!(output.status.success(), "validate answers valid: {text}");
    assert!(!text.contains("differs from its committed blob"), "{text}");
}

#[test]
fn a_packed_artifact_that_is_not_utf_8_is_refused_by_code() {
    let project = loose_store("not-utf8");
    let path = "planning/story/closed-b.md";
    let mut bytes = std::fs::read(engineering(&project).join(path)).expect("reads");
    bytes.push(0xff);
    std::fs::write(engineering(&project).join(path), &bytes).expect("writable");
    commit(&project, "closed-b with a byte that is not text");
    let name = pack_away(&project, &[path.to_owned()]);
    commit(&project, "pack it");
    refused(
        &project,
        &["pack_record_unreadable"],
        Some(&format!("{name}.pack")),
    );
}

#[test]
fn a_pack_removed_with_its_records_restored_loose_is_not_a_loss() {
    let (project, old) = packed_store("unpacked");
    for (path, bytes) in parsed(&project, &old) {
        let file = engineering(&project).join(&path);
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("writable");
        std::fs::write(file, bytes).expect("writable");
    }
    std::fs::remove_file(pack_file(&project, &old)).expect("removable");
    std::fs::write(
        engineering(&project).join("project.yaml"),
        project_file(&project, "aep.project/5"),
    )
    .expect("writable");
    valid(&project);
}

const ACME_BODY: &str = "## Verdict\n\nAcme approved it; ACME signed; acme paid.\n";

/// A committed store with a review of `subject` that said `ACME_BODY`, redacted, archived and
/// then packed with its redaction record. Returns the project and the pack's name.
fn packed_redacted_review(name: &str) -> (PathBuf, String) {
    let project = project(name);
    let file = project.join("review-body.md");
    std::fs::write(&file, ACME_BODY).expect("writable");
    ok(&project, &["new", "story", "subject", "--title", "Subject"]);
    ok(
        &project,
        &[
            "new",
            "review-result",
            "looked",
            "--title",
            "Looked at it",
            "--relate",
            "reviews:story:subject",
            "--from",
            file.to_str().expect("printable"),
        ],
    );
    std::fs::remove_file(&file).expect("removable");
    commit(&project, "review");
    ok(
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
    ok(
        &project,
        &["move", "review-result:looked", "--to", "archived"],
    );
    commit(&project, "redact and archive");
    ok(&project, &["validate"]);
    let pack = pack_review(&project);
    (project, pack)
}

/// Packs the committed review `review-result:looked` with its evidence into one pack, commits, and
/// returns the pack's name.
fn pack_review(project: &Path) -> String {
    let mut paths = vec!["planning/review-result/looked.md".to_owned()];
    let directory = engineering(project).join("evidence/review-result/looked");
    for entry in std::fs::read_dir(&directory).expect("the redaction wrote a record") {
        paths.push(format!(
            "evidence/review-result/looked/{}",
            entry
                .expect("an entry")
                .file_name()
                .into_string()
                .expect("printable")
        ));
    }
    paths.sort();
    let pack = pack_away(project, &paths);
    let _ = std::fs::remove_dir(&directory);
    commit(project, "pack the review");
    pack
}

#[test]
fn a_packed_review_with_a_redaction_record_is_valid() {
    let (project, _) = packed_redacted_review("packed-redaction");
    valid(&project);
}

#[test]
fn a_packed_copy_that_still_holds_a_redacted_literal_is_refused_without_printing_it() {
    let (project, old) = packed_redacted_review("packed-literal");
    let mut records = parsed(&project, &old);
    for (path, bytes) in &mut records {
        if path == "planning/review-result/looked.md" {
            let text = String::from_utf8(bytes.clone()).expect("utf-8");
            *bytes = text.replace("customer A", "Acme").into_bytes();
        }
    }
    let rendered = pack::render(&records).expect("renders");
    let name = replace_pack(&project, &old, &rendered);
    let text = refused(
        &project,
        &["pack_redacted_literal_exposed"],
        Some(&format!("{name}.pack")),
    );
    assert!(
        !text.to_lowercase().contains("acme"),
        "no literal in: {text}"
    );
}

#[test]
fn a_loose_copy_that_still_holds_a_redacted_literal_is_refused_without_printing_it() {
    let (project, pack_name) = packed_redacted_review("loose-literal");
    let git = Command::new("git")
        .args([
            "show",
            "HEAD~2:.engineering/planning/review-result/looked.md",
        ])
        .current_dir(&project)
        .output()
        .expect("git runs");
    assert!(git.status.success(), "the first version is committed");
    let loose = engineering(&project).join("planning/review-result/looked.md");
    std::fs::write(&loose, &git.stdout).expect("writable");
    commit(&project, "a loose copy with the literal");
    let text = refused(
        &project,
        &["pack_redacted_literal_exposed"],
        Some("looked.md"),
    );
    assert!(
        !text.to_lowercase().contains("acme"),
        "no literal in: {text}"
    );
    let _ = pack_name;
}

/// `aep plan artifact <args>` as `actor`.
fn aep_as(project: &Path, actor: &str, args: &[&str]) -> String {
    let mut full = vec!["plan", "artifact"];
    full.extend_from_slice(args);
    let output = Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(&full)
        .current_dir(project)
        .env("AEP_ACTOR", actor)
        .output()
        .expect("the binary runs");
    assert!(
        output.status.success(),
        "`{}` failed: {}",
        args.join(" "),
        said(&output)
    );
    stdout(&output)
}

/// A committed store with a review `looked` of `subject`, created by `actor`, whose title,
/// summary and body are as given, redacted of `literal`, archived and committed.
fn redacted_review_by(
    name: &str,
    actor: &str,
    (title, summary, body): (&str, &str, &str),
    literal: &str,
) -> PathBuf {
    let project = project(name);
    let file = project.join("review-body.md");
    std::fs::write(&file, body).expect("writable");
    ok(&project, &["new", "story", "subject", "--title", "Subject"]);
    aep_as(
        &project,
        actor,
        &[
            "new",
            "review-result",
            "looked",
            "--title",
            title,
            "--summary",
            summary,
            "--relate",
            "reviews:story:subject",
            "--from",
            file.to_str().expect("printable"),
        ],
    );
    std::fs::remove_file(&file).expect("removable");
    commit(&project, "review");
    ok(
        &project,
        &[
            "redact",
            "review-result:looked",
            "--literal",
            literal,
            "--with",
            "the reviewer",
            "--reason",
            "names a person",
        ],
    );
    aep_as(
        &project,
        actor,
        &["move", "review-result:looked", "--to", "archived"],
    );
    commit(&project, "redact and archive");
    project
}

const PLAIN: (&str, &str, &str) = ("Looked at it", "A look", "## Verdict\n\nFine.\n");

#[test]
fn a_redacted_review_whose_creating_actor_is_the_literal_is_valid_loose_and_packed() {
    let project = redacted_review_by(
        "actor-literal",
        "human:quillon",
        (
            "Looked at it",
            "A look",
            "## Verdict\n\nQuillon approved it.\n",
        ),
        "Quillon",
    );
    valid(&project);
    pack_review(&project);
    valid(&project);
}

/// The first committed version of the review, with `from` put back for `to` in its text.
fn reexposed_loose_copy(project: &Path, from: &str, to: &str) {
    let git = Command::new("git")
        .args([
            "show",
            "HEAD~1:.engineering/planning/review-result/looked.md",
        ])
        .current_dir(project)
        .output()
        .expect("git runs");
    assert!(git.status.success(), "the first version is committed");
    let text = String::from_utf8(git.stdout).expect("utf-8");
    assert!(text.contains(to), "the first version holds `{to}`");
    let loose = engineering(project).join("planning/review-result/looked.md");
    std::fs::write(&loose, text.replace(from, to)).expect("writable");
    commit(project, "a loose copy with the literal");
}

#[test]
fn a_loose_copy_that_holds_the_literal_in_its_title_or_summary_is_refused() {
    for (name, texts) in [
        (
            "title",
            ("Quillon looked at it", "A look", "## Verdict\n\nFine.\n"),
        ),
        (
            "summary",
            ("Looked at it", "Quillon looked", "## Verdict\n\nFine.\n"),
        ),
    ] {
        let project =
            redacted_review_by(&format!("loose-{name}"), "human:tester", texts, "Quillon");
        valid(&project);
        reexposed_loose_copy(&project, "the reviewer", "Quillon");
        refused(
            &project,
            &["pack_redacted_literal_exposed"],
            Some("looked.md"),
        );
    }
}

#[test]
fn a_packed_copy_that_holds_the_literal_in_its_title_or_summary_is_refused() {
    for (name, texts) in [
        (
            "title",
            ("Quillon looked at it", "A look", "## Verdict\n\nFine.\n"),
        ),
        (
            "summary",
            ("Looked at it", "Quillon looked", "## Verdict\n\nFine.\n"),
        ),
    ] {
        let project =
            redacted_review_by(&format!("packed-{name}"), "human:tester", texts, "Quillon");
        let old = pack_review(&project);
        let mut records = parsed(&project, &old);
        for (path, bytes) in &mut records {
            if path == "planning/review-result/looked.md" {
                let text = String::from_utf8(bytes.clone()).expect("utf-8");
                *bytes = text.replace("the reviewer", "Quillon").into_bytes();
            }
        }
        let rendered = pack::render(&records).expect("renders");
        let pack_name = replace_pack(&project, &old, &rendered);
        refused(
            &project,
            &["pack_redacted_literal_exposed"],
            Some(&format!("{pack_name}.pack")),
        );
    }
}

#[test]
fn a_review_that_never_held_the_literal_in_its_actor_is_still_valid() {
    let project = redacted_review_by(
        "plain-review",
        "human:tester",
        (PLAIN.0, PLAIN.1, "## Verdict\n\nQuillon approved it.\n"),
        "Quillon",
    );
    valid(&project);
}

// Adversary pass 1 on the prose-only scan.

/// A committed store with a review `looked` recorded `--prose-only` with `reason`, whose body is
/// `body`, redacted of `literal`, archived and committed. `redact` rewrites the title, summary
/// and body only, so a literal also in the prose-only reason stays there, readable by `show`.
fn redacted_prose_only_review(name: &str, reason: &str, body: &str, literal: &str) -> PathBuf {
    let project = project(name);
    let file = project.join("review-body.md");
    std::fs::write(&file, body).expect("writable");
    ok(&project, &["new", "story", "subject", "--title", "Subject"]);
    ok(
        &project,
        &[
            "new",
            "review-result",
            "looked",
            "--title",
            "Looked at it",
            "--relate",
            "reviews:story:subject",
            "--prose-only",
            reason,
            "--from",
            file.to_str().expect("printable"),
        ],
    );
    std::fs::remove_file(&file).expect("removable");
    commit(&project, "review");
    ok(
        &project,
        &[
            "redact",
            "review-result:looked",
            "--literal",
            literal,
            "--with",
            "the reviewer",
            "--reason",
            "names a person",
        ],
    );
    ok(
        &project,
        &["move", "review-result:looked", "--to", "archived"],
    );
    commit(&project, "redact and archive");
    let loose =
        std::fs::read_to_string(engineering(&project).join("planning/review-result/looked.md"))
            .expect("the review reads");
    assert!(
        loose.contains(&format!("prose_only: {reason}")),
        "the prose-only reason is still in the redacted copy: {loose}"
    );
    project
}

const PROSE_ONLY_REASON: &str = "Quillon asked for prose only";
const PROSE_ONLY_BODY: &str = "## Verdict\n\nQuillon approved it.\n";

#[test]
fn a_loose_copy_that_holds_the_literal_in_its_prose_only_reason_is_refused() {
    let project = redacted_prose_only_review(
        "adv-prose-only-loose",
        PROSE_ONLY_REASON,
        PROSE_ONLY_BODY,
        "Quillon",
    );
    let text = refused(
        &project,
        &["pack_redacted_literal_exposed"],
        Some("looked.md"),
    );
    assert!(
        !text.to_lowercase().contains("quillon"),
        "no literal in: {text}"
    );
}

#[test]
fn a_packed_copy_that_holds_the_literal_in_its_prose_only_reason_is_refused() {
    let project = redacted_prose_only_review(
        "adv-prose-only-packed",
        PROSE_ONLY_REASON,
        PROSE_ONLY_BODY,
        "Quillon",
    );
    let pack_name = pack_review(&project);
    let text = refused(
        &project,
        &["pack_redacted_literal_exposed"],
        Some(&format!("{pack_name}.pack")),
    );
    assert!(
        !text.to_lowercase().contains("quillon"),
        "no literal in: {text}"
    );
}

#[test]
fn a_packed_copy_that_does_not_parse_and_holds_the_literal_is_still_refused_for_it() {
    let (project, old) = packed_redacted_review("adv-unparseable-packed");
    let mut records = parsed(&project, &old);
    for (path, bytes) in &mut records {
        if path == "planning/review-result/looked.md" {
            let text = String::from_utf8(bytes.clone()).expect("utf-8");
            let text = text.replace("customer A", "Acme");
            let broken = text.strip_prefix("---\n").expect("the copy opens a fence");
            assert!(broken.contains("Acme"), "the broken copy holds the literal");
            *bytes = broken.as_bytes().to_vec();
        }
    }
    let rendered = pack::render(&records).expect("renders");
    let name = replace_pack(&project, &old, &rendered);
    let text = refused(
        &project,
        &["pack_record_unreadable", "pack_redacted_literal_exposed"],
        Some(&format!("{name}.pack")),
    );
    assert!(
        !text.to_lowercase().contains("acme"),
        "no literal in: {text}"
    );
}
