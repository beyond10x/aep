//! Adversarial cases against the fixes of the store-pack security review.
//!
//! 1. Decision 6: a path two packs hold with different bytes is refused by every reader, and the
//!    refusal names both places. A read verb that answers without the record, or that says the
//!    store holds no such artifact, picks neither copy and still answers silently.
//! 2. Decision 4: removing the redirecting Git variables from every `git` the planning code runs
//!    must not break a legitimate run of a check that predates packs. `safe.directory` given in
//!    the environment (`GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_n`/`GIT_CONFIG_VALUE_n`, Git's
//!    command-line scope, which Git honours for that key) is how a container is told to trust a
//!    checkout mounted from the host.
//!
//! Every fixture is a real store built by the real verbs in a scratch Git repository under the
//! test target directory.

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

fn scratch_root() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(&root).expect("the target scratch is writable");
    root.canonicalize().expect("the target scratch resolves")
}

fn project_file(project: &Path, version: &str) -> String {
    format!(
        "version: {version}\nplanning_scope: store-pack-security-adversary\nprotocol: adp/1\n\
         profile: development.standard\nprotocols: {}\nstore:\n  git: {{}}\n",
        relative(&project.join(".engineering"), &repository())
    )
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
        .env_remove("GIT_CEILING_DIRECTORIES")
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
        &[
            "commit",
            "--quiet",
            "--no-verify",
            "--allow-empty",
            "-m",
            message,
        ],
    );
}

fn aep_with(project: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_aep"));
    command
        .args(args)
        .current_dir(project)
        .env("AEP_ACTOR", "human:tester")
        .env_remove("GIT_CEILING_DIRECTORIES");
    for (key, value) in env {
        command.env(key, value);
    }
    command.output().expect("the binary runs")
}

fn said(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// `aep plan artifact <args>`.
fn artifact(project: &Path, args: &[&str]) -> Output {
    let mut full = vec!["plan", "artifact"];
    full.extend_from_slice(args);
    aep_with(project, &full, &[])
}

/// `aep plan artifact <args>`, required to succeed.
fn ok(project: &Path, args: &[&str]) -> String {
    let output = artifact(project, args);
    assert!(
        output.status.success(),
        "`{}` failed: {}",
        args.join(" "),
        said(&output)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// `aep plan store <args>`, required to succeed.
fn store_ok(project: &Path, args: &[&str]) -> String {
    let mut full = vec!["plan", "store"];
    full.extend_from_slice(args);
    let output = aep_with(project, &full, &[]);
    assert!(
        output.status.success(),
        "`store {}` failed: {}",
        args.join(" "),
        said(&output)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn engineering(project: &Path) -> PathBuf {
    project.join(".engineering")
}

fn packs_directory(project: &Path) -> PathBuf {
    engineering(project).join("packs")
}

/// The pack file names, sorted.
#[allow(clippy::case_sensitive_file_extension_comparisons)] // A pack's extension is exactly `.pack`.
fn packs(project: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(packs_directory(project))
        .map(|entries| {
            entries
                .map(|entry| {
                    entry
                        .expect("an entry")
                        .file_name()
                        .into_string()
                        .expect("utf-8")
                })
                .filter(|name| name.ends_with(".pack"))
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

fn read_pack(project: &Path, file: &str) -> pack::Pack {
    pack::Pack::parse(&std::fs::read(packs_directory(project).join(file)).expect("reads"))
        .expect("the pack parses")
}

/// The pack file holding `path`.
fn pack_holding(project: &Path, path: &str) -> String {
    packs(project)
        .into_iter()
        .find(|file| read_pack(project, file).get(path).is_some())
        .unwrap_or_else(|| panic!("a pack holds {path}"))
}

fn fresh(name: &str) -> PathBuf {
    let directory = scratch_root().join(format!("store-pack-security-adversary-{name}"));
    if directory.exists() || directory.is_symlink() {
        std::fs::remove_dir_all(&directory).expect("the previous scratch is removable");
    }
    std::fs::create_dir_all(engineering(&directory)).expect("writable");
    git(&directory, &["init", "--quiet"]);
    std::fs::write(
        engineering(&directory).join("project.yaml"),
        project_file(&directory, "aep.project/5"),
    )
    .expect("writable");
    directory
}

fn build_story(project: &Path, name: &str, title: &str, rungs: &[&str]) {
    ok(project, &["new", "story", name, "--title", title]);
    for rung in rungs {
        ok(project, &["move", &format!("story:{name}"), "--to", rung]);
    }
}

/// A committed `aep.project/5` store: `closed-a` (implemented, one evidence record), `closed-b`
/// (archived) and `open-c` (active).
fn loose_store(name: &str) -> PathBuf {
    let project = fresh(name);
    build_story(
        &project,
        "closed-b",
        "Closed B",
        &["proposed", "active", "archived"],
    );
    build_story(&project, "closed-a", "Closed A", &["proposed", "active"]);
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
    build_story(&project, "open-c", "Open C", &["proposed", "active"]);
    commit(&project, "a loose store");
    project
}

/// [`loose_store`], packed by `aep plan store pack` and committed.
fn packed_store(name: &str) -> PathBuf {
    let project = loose_store(name);
    store_ok(&project, &["pack"]);
    commit(&project, "pack the closed artifacts");
    project
}

// ---------------------------------------------------------------------------------------------
// Decision 6: a path held twice with different bytes is refused by every reader, naming both.
// ---------------------------------------------------------------------------------------------

/// A packed store and a second, well-formed pack (every digest and its name verify) holding
/// `planning/story/closed-a.md` with other bytes and no loose copy: the state the design's own
/// two-branch merge resolution leaves (`merge.renames=false`, keep both rewrites; the security
/// review's redaction-merge case builds it with the real verbs). `validate` reports it as
/// `pack_path_held_twice`. Decision 6: every reader refuses it, and the refusal names both packs.
/// A read verb that answers exit 0 without the story, or that says the store holds no
/// `story:closed-a`, has not refused: it dropped the record and said nothing about where it is.
#[test]
fn read_verbs_refuse_a_path_two_packs_hold_with_different_bytes_naming_both_packs() {
    let project = packed_store("two-packs-differ");
    let path = "planning/story/closed-a.md";
    let first = pack_holding(&project, path);
    let original = read_pack(&project, &first)
        .get(path)
        .expect("held")
        .bytes()
        .to_vec();
    let text = String::from_utf8(original).expect("utf-8");
    assert_eq!(text.matches("title: Closed A").count(), 1, "{text}");
    let other = text.replacen("title: Closed A", "title: Closed A, the other branch", 1);
    let rendered = pack::render(&[(path.to_owned(), other.into_bytes())]).expect("renders");
    let second = pack::file_name(&pack::name(&rendered));
    std::fs::write(packs_directory(&project).join(&second), &rendered).expect("writable");
    commit(&project, "the merge kept both rewrites");
    assert!(
        !engineering(&project).join(path).exists(),
        "precondition: no loose copy"
    );
    for name in [&first, &second] {
        let parsed = read_pack(&project, name);
        assert!(
            parsed
                .records()
                .iter()
                .all(pack::PackRecord::digest_matches)
                && format!("{}.pack", parsed.name()) == *name,
            "precondition: {name} verifies"
        );
    }
    let validate = artifact(&project, &["validate"]);
    assert!(
        said(&validate).contains("pack_path_held_twice"),
        "precondition: validate reports the two copies: {}",
        said(&validate)
    );

    let mut wrong = Vec::new();
    for args in [
        vec!["list"],
        vec!["show", "story:closed-a"],
        vec!["history", "story:closed-a"],
    ] {
        let output = artifact(&project, &args);
        let text = said(&output);
        let refused = output.status.code() == Some(1);
        let names_both = text.contains(&first) && text.contains(&second);
        if !(refused && names_both) {
            wrong.push(format!(
                "`{}` exit {:?}, names {first}: {}, names {second}: {}:\n{text}",
                args.join(" "),
                output.status.code(),
                text.contains(&first),
                text.contains(&second)
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "a path two packs hold with different bytes is not refused naming both packs:\n{}",
        wrong.join("\n---\n")
    );
}

// ---------------------------------------------------------------------------------------------
// Decision 4: a check that predates packs still runs where `safe.directory` is in the environment.
// ---------------------------------------------------------------------------------------------

/// The pre-existing `validate` check *a committed evidence file differs from its committed blob*
/// (`git_record::changed_evidence`, plain `git` before this fix) on a loose store with no pack.
/// Git is told the repository belongs to another user (`GIT_TEST_ASSUME_DIFFERENT_OWNER=1`, Git's
/// own switch, as `validate_packs_adversary_2` uses it) and told to trust it anyway through
/// `safe.directory` in the environment, which is a working setup: plain `git` reads the repository.
/// Before the fix `validate` saw the rewrite; with every `git` stripped of `GIT_CONFIG_*`, Git
/// refuses the repository, the store counts as unversioned and the rewrite validates.
#[test]
fn validate_still_reports_a_rewritten_evidence_file_where_safe_directory_comes_from_the_environment(
) {
    let project = loose_store("env-safe-directory");
    let directory = engineering(&project).join("evidence/story/closed-a");
    let file = std::fs::read_dir(&directory)
        .expect("the evidence directory lists")
        .map(|entry| entry.expect("an entry").path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .expect("one evidence record");
    let bytes = std::fs::read_to_string(&file).expect("reads");
    assert_eq!(bytes.matches("task check").count(), 1, "{bytes}");
    std::fs::write(&file, bytes.replacen("task check", "task chock", 1)).expect("writable");

    let control = artifact(&project, &["validate"]);
    assert_eq!(
        control.status.code(),
        Some(1),
        "precondition: the rewrite is refused: {}",
        said(&control)
    );
    assert!(
        said(&control).contains("differs from its committed blob"),
        "{}",
        said(&control)
    );

    let trusted = [
        ("GIT_TEST_ASSUME_DIFFERENT_OWNER", "1"),
        ("GIT_CONFIG_COUNT", "1"),
        ("GIT_CONFIG_KEY_0", "safe.directory"),
        ("GIT_CONFIG_VALUE_0", "*"),
    ];
    let mut plain = Command::new("git");
    plain
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(engineering(&project))
        .env_remove("GIT_CEILING_DIRECTORIES");
    for (key, value) in trusted {
        plain.env(key, value);
    }
    let plain = plain.output().expect("git runs");
    assert!(
        plain.status.success(),
        "precondition: plain git reads the repository with that environment: {}",
        String::from_utf8_lossy(&plain.stderr)
    );

    let output = aep_with(&project, &["plan", "artifact", "validate"], &trusted);
    let text = said(&output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "with safe.directory in the environment the rewritten evidence file validates: {text}"
    );
    assert!(text.contains("differs from its committed blob"), "{text}");
}

/// The same environment on a clean, committed loose store that holds no pack at all: `validate`
/// passes without the environment and must pass with it. Plain `git` reads the repository there,
/// so a refusal is the stripped `GIT_CONFIG_*`, not the repository.
#[test]
fn validate_admits_a_clean_store_without_packs_where_safe_directory_comes_from_the_environment() {
    let project = loose_store("env-safe-directory-clean");
    assert!(
        packs(&project).is_empty(),
        "precondition: the store holds no pack"
    );
    let control = artifact(&project, &["validate"]);
    assert!(
        control.status.success(),
        "precondition: the store validates: {}",
        said(&control)
    );
    let trusted = [
        ("GIT_TEST_ASSUME_DIFFERENT_OWNER", "1"),
        ("GIT_CONFIG_COUNT", "1"),
        ("GIT_CONFIG_KEY_0", "safe.directory"),
        ("GIT_CONFIG_VALUE_0", "*"),
    ];
    let output = aep_with(&project, &["plan", "artifact", "validate"], &trusted);
    let text = said(&output);
    assert!(
        output.status.success(),
        "a clean store without packs is refused where safe.directory is in the environment: \
         {text}"
    );
}
