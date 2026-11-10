//! Adversary pass 1 on `story:validate-checks-packs`: ways a pack holds bytes nobody committed, or a
//! committed record vanishes, while `validate` says `valid`; and a history `validate` misreads.
//!
//! The fixture is the one `validate_packs.rs` builds: a committed `aep.project/5` store whose closed
//! artifacts and evidence are moved into packs rendered by the pack module, flipped to `/6`.

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
        "version: {version}\nplanning_scope: validate-packs-adversary\nprotocol: adp/1\n\
         profile: development.standard\nprotocols: {}\nstore:\n  git: {{}}\n",
        relative(&project.join(".engineering"), &repository())
    )
}

fn scratch(name: &str) -> PathBuf {
    let directory =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("validate-packs-adv-{name}"));
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

/// Runs `aep plan artifact <args>` in `project`. Git looks no further up than the scratch
/// directory, so a directory beside the project is outside every work tree wherever the build
/// directory happens to be.
fn aep(project: &Path, args: &[&str]) -> Output {
    let mut full = vec!["plan", "artifact"];
    full.extend_from_slice(args);
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(&full)
        .current_dir(project)
        .env("AEP_ACTOR", "human:tester")
        .env("GIT_CEILING_DIRECTORIES", env!("CARGO_TARGET_TMPDIR"))
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

fn ok(project: &Path, args: &[&str]) {
    let output = aep(project, args);
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

/// The security point of the unit: a packs directory that is a symbolic link out of the work tree
/// holds a pack whose closed-a record nobody committed (title rewritten, digest and name
/// recomputed). `validate` must refuse it — by the provenance check or by refusing the link — and
/// never answer `valid`.
#[test]
fn a_packs_directory_linked_out_of_the_work_tree_does_not_skip_provenance() {
    let (project, first, second) = two_pack_store("linked-packs");
    let outside = scratch("linked-packs-outside");
    std::fs::create_dir_all(&outside).expect("writable");
    let packs = engineering(&project).join("packs");
    // closed-b's pack, unchanged.
    std::fs::copy(
        packs.join(pack::file_name(&second)),
        outside.join(pack::file_name(&second)),
    )
    .expect("copies");
    // closed-a's pack, forged: bytes no commit ever held at the record's path.
    let original = std::fs::read(packs.join(pack::file_name(&first))).expect("reads");
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
    let forged_name = write_pack(&outside, &forged);
    std::fs::remove_dir_all(&packs).expect("removable");
    std::os::unix::fs::symlink(&outside, &packs).expect("the link is writable");
    commit(&project, "the packs live elsewhere");

    let shown = aep(&project, &["show", "story:closed-a"]);
    assert!(
        said(&shown).contains("Forged Z"),
        "readers serve the forged record: {}",
        said(&shown)
    );
    let output = aep(&project, &["validate"]);
    let text = said(&output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a record nobody committed, behind a linked packs directory, is refused: {text}"
    );
    assert!(
        text.contains("[pack_record_not_committed_bytes]") || text.contains("[pack_path_unsafe]"),
        "refused for provenance or for the link: {text}"
    );
    assert!(
        text.contains(&forged_name) || text.contains("packs"),
        "names the pack or the packs directory: {text}"
    );
}

/// A committed pack deleted from the working tree takes a committed evidence record with it. The
/// same deletion of the loose file is refused today (`changed_evidence`); the packed one must not
/// pass silently.
#[test]
fn a_committed_pack_deleted_from_the_working_tree_does_not_lose_its_evidence_silently() {
    let (project, first, _) = two_pack_store("lost-pack");
    let evidence: Vec<String> = paths_of(&project, "closed-a");
    let packed_evidence = pack::Pack::parse(
        &std::fs::read(
            engineering(&project)
                .join("packs")
                .join(pack::file_name(&first)),
        )
        .expect("reads"),
    )
    .expect("parses")
    .records()
    .iter()
    .map(|record| record.path().to_owned())
    .find(|path| path.starts_with("evidence/"))
    .expect("closed-a's pack holds its evidence");
    assert!(
        evidence.len() == 1,
        "closed-a is loose no more: {evidence:?}"
    );

    // The loose parity: deleting a committed loose evidence file is refused.
    let loose = loose_store("lost-loose");
    let loose_evidence = paths_of(&loose, "closed-a")
        .into_iter()
        .find(|path| path.starts_with("evidence/"))
        .expect("an evidence record");
    std::fs::remove_file(engineering(&loose).join(&loose_evidence)).expect("removable");
    let output = aep(&loose, &["validate"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a deleted loose evidence file is refused today: {}",
        said(&output)
    );

    std::fs::remove_file(
        engineering(&project)
            .join("packs")
            .join(pack::file_name(&first)),
    )
    .expect("removable");
    let output = aep(&project, &["validate"]);
    let text = said(&output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a committed pack gone from the working tree, and the committed evidence record \
         `{packed_evidence}` with it, is refused: {text}"
    );
    assert!(
        text.contains(&first) || text.contains(&packed_evidence),
        "names the pack or the record: {text}"
    );
}

/// A shallow clone (`actions/checkout` without `fetch-depth: 0`) of a valid packed store: the pack
/// looks added by the clone's boundary commit, which has no parent here. `validate` must not
/// accuse every record of holding bytes nobody committed; the history that would tell is simply
/// not fetched, and the refusal must say so, as the review-dating check does.
#[test]
fn a_shallow_clone_of_a_valid_packed_store_names_the_shallow_clone_not_forgery() {
    let (project, _, _) = two_pack_store("shallow-source");
    ok(&project, &["new", "story", "later", "--title", "Later"]);
    commit(&project, "a later commit, so the pack is not the tip's");
    valid(&project);
    let clone = scratch("shallow-clone");
    let source = format!("file://{}", project.display());
    git(
        Path::new(env!("CARGO_TARGET_TMPDIR")),
        &[
            "clone",
            "--quiet",
            "--depth",
            "1",
            &source,
            clone.to_str().expect("printable"),
        ],
    );
    let output = aep(&clone, &["validate"]);
    let text = said(&output);
    assert!(
        output.status.success() || text.contains("shallow"),
        "a shallow clone of a valid packed store is valid, or the refusal names the shallow \
         clone: {text}"
    );
    assert!(
        !text.contains("[pack_record_not_committed_bytes]"),
        "no record is accused of bytes nobody committed: {text}"
    );
}

/// The same skip without a link: the packs directory is a repository of its own with no commit
/// (`git init` there), so `git log` run inside it fails, and a forged pack beside it passes.
#[test]
fn a_packs_directory_that_is_a_repository_of_its_own_does_not_skip_provenance() {
    let (project, first, _) = two_pack_store("nested-packs");
    let packs = engineering(&project).join("packs");
    let original = std::fs::read(packs.join(pack::file_name(&first))).expect("reads");
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
    std::fs::remove_file(packs.join(pack::file_name(&first))).expect("removable");
    let forged_name = write_pack(&packs, &forged);
    git(&packs, &["init", "--quiet"]);
    let output = aep(&project, &["validate"]);
    let text = said(&output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a record nobody committed, in a packs directory that is its own repository, is \
         refused: {text}"
    );
    assert!(
        text.contains("[pack_record_not_committed_bytes]") && text.contains(&forged_name),
        "refused for provenance, naming the pack: {text}"
    );
}

/// Probe: closed-b committed as A, edited and committed as B, then packed holding A (an older
/// committed version, not the last). Refused by provenance.
#[test]
fn a_pack_holding_an_older_committed_version_is_refused() {
    let project = loose_store("older-version");
    let path = "planning/story/closed-b.md".to_owned();
    let first = std::fs::read(engineering(&project).join(&path)).expect("reads");
    let mut second = first.clone();
    second.extend_from_slice(b"\nAn edit after closing.\n");
    std::fs::write(engineering(&project).join(&path), &second).expect("writable");
    commit(&project, "edit the closed story");
    let name = write_pack(
        &engineering(&project).join("packs"),
        &[(path.clone(), first)],
    );
    remove_loose(&project, std::slice::from_ref(&path));
    to_version_6(&project);
    commit(&project, "pack the older version");
    let output = aep(&project, &["validate"]);
    let text = said(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(
        text.contains("[pack_record_not_committed_bytes]") && text.contains(&name),
        "{text}"
    );
}

/// Probe: the `--format json` and `--format yaml` answers for a packed copy that re-exposes a
/// redacted literal hold no copy of the literal either.
#[test]
fn no_output_format_prints_a_redacted_literal_a_packed_copy_re_exposes() {
    let project = project("literal-formats");
    let file = project.join("review-body.md");
    std::fs::write(
        &file,
        "## Verdict\n\nAcme approved it; ACME signed; acme paid.\n",
    )
    .expect("writable");
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
    let unredacted = std::fs::read(engineering(&project).join("planning/review-result/looked.md"))
        .expect("reads");
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
    let mut paths = vec!["planning/review-result/looked.md".to_owned()];
    for entry in std::fs::read_dir(engineering(&project).join("evidence/review-result/looked"))
        .expect("the redaction wrote a record")
    {
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
    let mut records = records_of(&project, &paths);
    for (path, bytes) in &mut records {
        if path == "planning/review-result/looked.md" {
            bytes.clone_from(&unredacted);
        }
    }
    write_pack(&engineering(&project).join("packs"), &records);
    remove_loose(&project, &paths);
    to_version_6(&project);
    commit(&project, "pack the unredacted copy");
    for format in ["text", "json", "yaml"] {
        let output = aep(&project, &["validate", "--format", format]);
        let text = said(&output);
        assert_eq!(output.status.code(), Some(1), "{format}: {text}");
        assert!(
            text.contains("pack_redacted_literal_exposed"),
            "{format}: {text}"
        );
        assert!(
            !text.to_lowercase().contains("acme"),
            "{format}: no literal in: {text}"
        );
    }
}
