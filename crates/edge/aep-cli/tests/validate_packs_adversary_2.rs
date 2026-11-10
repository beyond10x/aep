//! Adversary pass 2 on `story:validate-checks-packs`: the fix classifies every Git answer as
//! *unversioned*, *shallow*, *failed* or *blobs*, and skips provenance only when the store is
//! unversioned. These cases build a store that Git versions and in which a forged pack is served by
//! the readers, and reach the *unversioned* answer anyway.
//!
//! The fixture is the one `validate_packs_adversary.rs` builds: a committed `aep.project/5` store
//! whose closed artifacts and evidence are moved into packs rendered by the pack module, flipped
//! to `/6`.

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
        "version: {version}\nplanning_scope: validate-packs-adversary-2\nprotocol: adp/1\n\
         profile: development.standard\nprotocols: {}\nstore:\n  git: {{}}\n",
        relative(&project.join(".engineering"), &repository())
    )
}

fn scratch(name: &str) -> PathBuf {
    let directory =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("validate-packs-adv2-{name}"));
    if directory.exists() || directory.is_symlink() {
        std::fs::remove_dir_all(&directory).expect("the previous scratch is removable");
    }
    directory
}

fn project(name: &str) -> PathBuf {
    let directory = scratch(name);
    std::fs::create_dir_all(directory.join(".engineering")).expect("the scratch is writable");
    git(&directory, &["init", "--quiet"]);
    std::fs::write(
        directory.join(".engineering/project.yaml"),
        project_file(&directory, "aep.project/5"),
    )
    .expect("the project file is writable");
    directory
}

fn git(directory: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .args(args)
        .current_dir(directory)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

/// Runs `aep plan artifact <args>` in `directory` with `env` added. Git looks no further up than
/// the scratch directory.
fn aep_with(directory: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut full = vec!["plan", "artifact"];
    full.extend_from_slice(args);
    let mut command = Command::new(env!("CARGO_BIN_EXE_aep"));
    command
        .args(&full)
        .current_dir(directory)
        .env("AEP_ACTOR", "human:tester")
        .env("GIT_CEILING_DIRECTORIES", env!("CARGO_TARGET_TMPDIR"));
    for (key, value) in env {
        command.env(key, value);
    }
    command.output().expect("the binary runs")
}

fn aep(directory: &Path, args: &[&str]) -> Output {
    aep_with(directory, args, &[])
}

fn said(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn ok(directory: &Path, args: &[&str]) {
    let output = aep(directory, args);
    assert!(
        output.status.success(),
        "`{}` failed: {}",
        args.join(" "),
        said(&output)
    );
}

fn valid(project: &Path) {
    let output = aep(project, &["validate"]);
    assert!(
        output.status.success(),
        "validate answers valid: {}",
        said(&output)
    );
}

fn commit(project: &Path, message: &str) {
    git(project, &["add", "--all"]);
    git(
        project,
        &[
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
    );
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
    valid(&project);
    project
}

/// The store paths of `name`'s artifact and of the evidence under it.
fn paths_of(project: &Path, name: &str) -> Vec<String> {
    let mut paths = vec![format!("planning/story/{name}.md")];
    let directory = engineering(project).join(format!("evidence/story/{name}"));
    if let Ok(entries) = std::fs::read_dir(&directory) {
        for entry in entries.map(|entry| entry.expect("an entry")) {
            paths.push(format!(
                "evidence/story/{name}/{}",
                entry.file_name().into_string().expect("printable")
            ));
        }
    }
    paths.sort();
    paths
}

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

/// Writes `records` as a pack into `packs` and returns its name.
fn write_pack(packs: &Path, records: &[(String, Vec<u8>)]) -> String {
    let bytes = pack::render(records).expect("the pack renders");
    let name = pack::name(&bytes);
    std::fs::create_dir_all(packs).expect("writable");
    std::fs::write(packs.join(pack::file_name(&name)), bytes).expect("writable");
    name
}

/// Removes the loose copies of `paths` and the evidence directories they leave empty.
fn remove_loose(project: &Path, paths: &[String]) {
    for path in paths {
        std::fs::remove_file(engineering(project).join(path)).expect("removable");
        if let Some(parent) = engineering(project).join(path).parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }
}

fn to_version_6(project: &Path) {
    std::fs::write(
        engineering(project).join("project.yaml"),
        project_file(project, "aep.project/6"),
    )
    .expect("writable");
}

/// [`loose_store`] with closed-a and its evidence in one committed pack and closed-b in another.
fn two_pack_store(name: &str) -> (PathBuf, String, String) {
    let project = loose_store(name);
    let packs = engineering(&project).join("packs");
    let a = paths_of(&project, "closed-a");
    let b = paths_of(&project, "closed-b");
    let first = write_pack(&packs, &records_of(&project, &a));
    let second = write_pack(&packs, &records_of(&project, &b));
    remove_loose(&project, &a);
    remove_loose(&project, &b);
    to_version_6(&project);
    commit(&project, "pack the closed artifacts");
    valid(&project);
    (project, first, second)
}

/// Replaces the pack `name` in `packs` by one whose `planning/story/closed-a.md` says "Forged Z"
/// instead of "Closed A", digest and name recomputed, and returns the forged pack's name.
fn forge_closed_a(packs: &Path, name: &str) -> String {
    let original = std::fs::read(packs.join(pack::file_name(name))).expect("reads");
    let forged: Vec<(String, Vec<u8>)> = pack::Pack::parse(&original)
        .expect("parses")
        .records()
        .iter()
        .map(|record| {
            let bytes = if record.path() == "planning/story/closed-a.md" {
                String::from_utf8(record.bytes().to_vec())
                    .expect("utf-8")
                    .replace("Closed A", "Forged Z")
                    .into_bytes()
            } else {
                record.bytes().to_vec()
            };
            (record.path().to_owned(), bytes)
        })
        .collect();
    std::fs::remove_file(packs.join(pack::file_name(name))).expect("removable");
    write_pack(packs, &forged)
}

/// A committed forged pack, refused when Git answers, and the same store when Git refuses to read
/// the repository because it belongs to another user (`safe.directory`; a container with the
/// checkout mounted from the host, as `docker run -v "$PWD:/work"` gives). The store is versioned
/// and committed; only the reading is refused. The fix promises that a Git failure is a refusal and
/// never a skip, and the readers serve the forged record all the same.
///
/// `GIT_TEST_ASSUME_DIFFERENT_OWNER=1` is Git's own switch for the ownership check (its test suite,
/// t0033), so the case does not need a second user.
#[test]
fn a_repository_git_refuses_to_read_for_its_owner_does_not_skip_provenance() {
    let (project, first, _) = two_pack_store("dubious-owner");
    let forged = forge_closed_a(&engineering(&project).join("packs"), &first);
    commit(&project, "a pack nobody's loose file ever held");

    let control = aep(&project, &["validate"]);
    assert_eq!(
        control.status.code(),
        Some(1),
        "the forged pack is refused when Git reads the history: {}",
        said(&control)
    );

    let dubious = [("GIT_TEST_ASSUME_DIFFERENT_OWNER", "1")];
    let shown = aep_with(&project, &["show", "story:closed-a"], &dubious);
    assert!(
        said(&shown).contains("Forged Z"),
        "readers serve the forged record without Git: {}",
        said(&shown)
    );
    let output = aep_with(&project, &["validate"], &dubious);
    let text = said(&output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a versioned store Git refuses to read is not an unversioned one; the forged pack \
         {forged} is refused, not waved through: {text}"
    );
    assert!(
        text.contains("[pack_record_not_committed_bytes]"),
        "refused for provenance: {text}"
    );
}

/// The same forged pack, validated where no `git` program is on the `PATH` (a container image
/// with aep and without Git, the checkout mounted in). The repository is there and the store is
/// committed; the history simply cannot be asked.
#[test]
fn no_git_program_on_the_path_does_not_skip_provenance_of_a_versioned_store() {
    let (project, first, _) = two_pack_store("no-git");
    forge_closed_a(&engineering(&project).join("packs"), &first);
    commit(&project, "a pack nobody's loose file ever held");
    let empty = scratch("no-git-path");
    std::fs::create_dir_all(&empty).expect("writable");
    let path = [("PATH", empty.to_str().expect("printable"))];

    let shown = aep_with(&project, &["show", "story:closed-a"], &path);
    assert!(
        said(&shown).contains("Forged Z"),
        "readers serve the forged record without Git: {}",
        said(&shown)
    );
    let output = aep_with(&project, &["validate"], &path);
    let text = said(&output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "the store has a `.git`; with no Git to ask, the forged pack is not admitted: {text}"
    );
    assert!(
        text.contains("[pack_record_not_committed_bytes]"),
        "refused for provenance: {text}"
    );
}

/// A Git-native store outside a project directory (`--store <dir>`), its closed story in a
/// committed forged pack. `--store store` from the repository refuses it. `--store .` from inside
/// the store is the same store, served the same forged record, and must be refused the same way:
/// `unversioned(".")` has no file name to ask `check-ignore` about and answers *unversioned*.
#[test]
fn a_store_named_dot_is_held_to_provenance_as_the_same_store_named_by_its_path() {
    let project = project("store-dot");
    let store = project.join("store");
    let s = ["--store", "store"];
    let with = |args: &[&str]| -> Vec<String> {
        args.iter()
            .chain(s.iter())
            .map(|arg| (*arg).to_owned())
            .collect()
    };
    let run = |args: &[&str]| {
        let full = with(args);
        let refs: Vec<&str> = full.iter().map(String::as_str).collect();
        ok(&project, &refs);
    };
    run(&["new", "story", "closed-a", "--title", "Closed A"]);
    for rung in ["proposed", "active", "archived"] {
        run(&["move", "story:closed-a", "--to", rung]);
    }
    run(&["new", "story", "open-c", "--title", "Open C"]);
    commit(&project, "a store outside the project directory");
    let loose = store.join("story/closed-a.md");
    let bytes = std::fs::read(&loose).expect("the closed story is where --store put it");
    let forged_bytes = String::from_utf8(bytes)
        .expect("utf-8")
        .replace("Closed A", "Forged Z")
        .into_bytes();
    let forged = write_pack(
        &store.join("packs"),
        &[("planning/story/closed-a.md".to_owned(), forged_bytes)],
    );
    std::fs::remove_file(&loose).expect("removable");
    commit(&project, "pack the closed story, forged");

    let control = aep(&project, &["validate", "--store", "store"]);
    let control_text = said(&control);
    assert_eq!(
        control.status.code(),
        Some(1),
        "`--store store` refuses the forged pack: {control_text}"
    );
    assert!(
        control_text.contains("[pack_record_not_committed_bytes]"),
        "{control_text}"
    );

    let shown = aep(&store, &["show", "story:closed-a", "--store", "."]);
    assert!(
        said(&shown).contains("Forged Z"),
        "`--store .` serves the forged record: {}",
        said(&shown)
    );
    let output = aep(&store, &["validate", "--store", "."]);
    let text = said(&output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "`--store .` is the same committed store and its forged pack {forged} is refused: {text}"
    );
    assert!(
        text.contains("[pack_record_not_committed_bytes]"),
        "refused for provenance: {text}"
    );
}

/// A shallow clone of a valid packed store whose loose file was removed in one commit and packed in
/// a later one — the flow `blobs_before_pack` documents ("a path deleted before that commit ...
/// answers with the last version it had"). With the clone's boundary on the removal commit, the
/// pack's own commit is not the boundary, so the fix's shallow answer is not given, and the record
/// is accused of bytes nobody committed although the history that would say otherwise was simply
/// not fetched.
#[test]
fn a_shallow_clone_whose_boundary_is_the_removal_before_the_pack_names_the_shallow_clone() {
    let project = loose_store("shallow-two-step");
    let paths = paths_of(&project, "closed-b");
    let records = records_of(&project, &paths);
    remove_loose(&project, &paths);
    commit(&project, "remove closed-b's loose file");
    write_pack(&engineering(&project).join("packs"), &records);
    to_version_6(&project);
    commit(&project, "pack closed-b");
    ok(&project, &["new", "story", "later", "--title", "Later"]);
    commit(&project, "a later commit");
    valid(&project);

    let clone = scratch("shallow-two-step-clone");
    let source = format!("file://{}", project.display());
    git(
        Path::new(env!("CARGO_TARGET_TMPDIR")),
        &[
            "clone",
            "--quiet",
            "--depth",
            "3",
            &source,
            clone.to_str().expect("printable"),
        ],
    );
    let output = aep(&clone, &["validate"]);
    let text = said(&output);
    assert!(
        output.status.success() || text.contains("[pack_history_shallow]"),
        "a shallow clone of a valid packed store is valid, or the refusal names the shallow \
         clone: {text}"
    );
    assert!(
        !text.contains("[pack_record_not_committed_bytes]"),
        "no record is accused of bytes nobody committed: {text}"
    );
}

/// Design item 7 (`architecture-design:store-pack`): "no copy of an artifact (loose or any pack)
/// may contain a window matching one of its redaction records' digest and length". The check runs
/// only when the store holds a pack, so in a store with none, a redacted review-result whose body
/// is put back to what it said before the redaction is `valid`: its body equals its first committed
/// version, which the replay check accepts without replaying, and nothing else looks.
#[test]
fn a_loose_review_result_that_puts_back_a_redacted_literal_is_refused_without_a_pack() {
    let project = project("literal-loose");
    let file = project.join("review-body.md");
    std::fs::write(&file, "## Verdict\n\nAcme approved it.\n").expect("writable");
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
    commit(&project, "redact");
    valid(&project);

    let document = engineering(&project).join("planning/review-result/looked.md");
    let redacted = std::fs::read_to_string(&document).expect("reads");
    assert!(
        !redacted.contains("Acme"),
        "the redaction removed it: {redacted}"
    );
    std::fs::write(&document, redacted.replace("customer A", "Acme")).expect("writable");
    commit(&project, "put the literal back");

    let output = aep(&project, &["validate"]);
    let text = said(&output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a loose copy that re-exposes a redacted literal is refused: {text}"
    );
    assert!(
        !text.contains("Acme"),
        "and the refusal does not print the literal: {text}"
    );
}
