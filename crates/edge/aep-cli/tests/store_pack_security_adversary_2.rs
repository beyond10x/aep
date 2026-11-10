//! Second adversarial pass against the fixes of the store-pack security review (round 2).
//!
//! 1. `MarkdownStore::load` now reports a path held twice with different bytes as a pack failure
//!    and stops: its own comment says the documents that read are still loaded so `validate`
//!    reports on them. A second, unrelated problem in another document must still be reported.
//! 2. A path held twice in **evidence** (not planning): every read verb refuses naming both packs,
//!    and `unpack --from` is still the way out.
//! 3. Provenance must not pass a packed record that is not the committed blob because of the
//!    environment. `GIT_GRAFT_FILE` and `GIT_REPLACE_REF_BASE` are environment-only variables that
//!    change which commit is `<added>^` and which bytes a blob name reads; neither is removed from
//!    the `git` processes provenance runs.
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
        "version: {version}\nplanning_scope: store-pack-security-adversary-2\nprotocol: adp/1\n\
         profile: development.standard\nprotocols: {}\nstore:\n  git: {{}}\n",
        relative(&project.join(".engineering"), &repository())
    )
}

fn git_env(project: &Path, args: &[&str], env: &[(&str, &str)]) -> String {
    let mut command = Command::new("git");
    command
        .args([
            "-c",
            "user.name=tester",
            "-c",
            "user.email=tester@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "advice.graftFileDeprecated=false",
        ])
        .args(args)
        .current_dir(project)
        .env_remove("GIT_CEILING_DIRECTORIES");
    for (key, value) in env {
        command.env(key, value);
    }
    let output = command.output().expect("git runs");
    assert!(
        output.status.success(),
        "git {}: {}{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn git(project: &Path, args: &[&str]) -> String {
    git_env(project, args, &[])
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

/// `aep plan store <args>`.
fn store(project: &Path, args: &[&str]) -> Output {
    let mut full = vec!["plan", "store"];
    full.extend_from_slice(args);
    aep_with(project, &full, &[])
}

/// `aep plan store <args>`, required to succeed.
fn store_ok(project: &Path, args: &[&str]) -> String {
    let output = store(project, args);
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
    let directory = scratch_root().join(format!("store-pack-security-adversary-2-{name}"));
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

/// `bytes` with the one occurrence of `from` replaced by `to`, which has the same length.
fn swap_same_length(bytes: &[u8], from: &str, to: &str) -> Vec<u8> {
    assert_eq!(from.len(), to.len(), "a same-length edit");
    let text = String::from_utf8(bytes.to_vec()).expect("utf-8");
    assert_eq!(
        text.matches(from).count(),
        1,
        "`{from}` occurs once in: {text}"
    );
    text.replacen(from, to, 1).into_bytes()
}

/// A second well-formed pack holding `path` alone, with `from` swapped for `to` in the bytes the
/// store's own pack holds; written beside the first and committed, as the design's two-branch
/// merge resolution leaves it. Answers the two pack file names, the original first.
fn second_pack_with(project: &Path, path: &str, from: &str, to: &str) -> (String, String) {
    let first = pack_holding(project, path);
    let original = read_pack(project, &first)
        .get(path)
        .expect("held")
        .bytes()
        .to_vec();
    let other = swap_same_length(&original, from, to);
    let rendered = pack::render(&[(path.to_owned(), other)]).expect("renders");
    let second = pack::file_name(&pack::name(&rendered));
    std::fs::write(packs_directory(project).join(&second), &rendered).expect("writable");
    commit(project, "the merge kept both rewrites");
    assert!(
        !engineering(project).join(path).exists(),
        "precondition: no loose copy of {path}"
    );
    (first, second)
}

/// The one packed evidence record of `story:closed-a`.
fn closed_a_evidence(project: &Path) -> String {
    let file = pack_holding(project, "planning/story/closed-a.md");
    read_pack(project, &file)
        .records()
        .iter()
        .map(|record| record.path().to_owned())
        .find(|path| path.starts_with("evidence/story/closed-a/"))
        .expect("the evidence record of closed-a is packed")
}

// ---------------------------------------------------------------------------------------------
// 1. `validate` still reports the documents that read while a path is held twice.
// ---------------------------------------------------------------------------------------------

/// Two packs hold `planning/story/closed-a.md` with different bytes (the merge state), and the
/// loose `open-c` has had its `status:` line edited by hand, which `validate` reports on any
/// store as a document that does not read (status and last transition disagree). `MarkdownStore::load` says, where it reports the
/// path held twice, that "the documents that read are still loaded, so `validate` reports on
/// them": `validate` must report both problems, not stop at the conflict and count no artifact.
#[test]
fn validate_reports_other_documents_problems_while_a_path_is_held_twice() {
    let project = packed_store("validate-still-reads");
    let (first, second) = second_pack_with(
        &project,
        "planning/story/closed-a.md",
        "title: Closed A",
        "title: Closed X",
    );
    let open = engineering(&project).join("planning/story/open-c.md");
    let text = std::fs::read_to_string(&open).expect("reads");
    std::fs::write(
        &open,
        text.replacen("status: active", "status: proposed", 1),
    )
    .expect("writable");

    // Control: without the conflict, the hand edit is reported.
    let control_project = loose_store("validate-still-reads-control");
    let control_open = engineering(&control_project).join("planning/story/open-c.md");
    let control_text = std::fs::read_to_string(&control_open).expect("reads");
    std::fs::write(
        &control_open,
        control_text.replacen("status: active", "status: proposed", 1),
    )
    .expect("writable");
    let control = artifact(&control_project, &["validate"]);
    assert!(
        said(&control).contains(
            "`status: proposed` and the last transition, which moved to `active`, disagree"
        ),
        "precondition: validate reports the hand-edited status: {}",
        said(&control)
    );

    let output = artifact(&project, &["validate"]);
    let text = said(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(
        text.contains("pack_path_held_twice") && text.contains(&first) && text.contains(&second),
        "precondition: validate reports the two copies: {text}"
    );
    assert!(
        text.contains(
            "`status: proposed` and the last transition, which moved to `active`, disagree"
        ),
        "while a path is held twice, validate no longer reports the hand-edited status of \
         another document (the store stops loading documents): {text}"
    );
}

// ---------------------------------------------------------------------------------------------
// 2. A path held twice in evidence: every read verb refuses naming both; `unpack --from` resolves.
// ---------------------------------------------------------------------------------------------

/// Two packs hold `story:closed-a`'s evidence record with different bytes and no loose copy.
/// Every read verb refuses naming both packs (decision 6 for evidence, not only planning), and
/// `aep plan store unpack story:closed-a --from <pack>` is still the documented way out: it
/// succeeds and the read verbs answer again.
#[test]
fn an_evidence_path_held_twice_is_refused_by_read_verbs_and_unpack_from_resolves_it() {
    let project = packed_store("evidence-held-twice");
    let path = closed_a_evidence(&project);
    let (first, second) = second_pack_with(&project, &path, "task check", "task chock");
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
        vec!["show", "story:open-c"],
        vec!["history", "story:closed-a"],
        vec!["explain", "story:closed-a"],
        vec!["board"],
        vec!["graph"],
        vec!["blocked"],
    ] {
        let output = artifact(&project, &args);
        let text = said(&output);
        if !(output.status.code() == Some(1) && text.contains(&first) && text.contains(&second)) {
            wrong.push(format!(
                "`{}` exit {:?}:\n{text}",
                args.join(" "),
                output.status.code()
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "an evidence path two packs hold with different bytes is not refused naming both:\n{}",
        wrong.join("\n---\n")
    );

    let chosen = second.trim_end_matches(".pack").to_owned();
    store_ok(&project, &["unpack", "story:closed-a", "--from", &chosen]);
    let restored = std::fs::read_to_string(engineering(&project).join(&path)).expect("restored");
    assert!(restored.contains("task chock"), "{restored}");
    ok(&project, &["history", "story:closed-a"]);
    ok(&project, &["show", "story:closed-a"]);
}

// ---------------------------------------------------------------------------------------------
// 3. Provenance under environment-only Git variables that rewrite history.
// ---------------------------------------------------------------------------------------------

/// A packed store whose pack holding `closed-a` has been replaced by a well-formed forged pack
/// (`title: Forged A`), committed: the state `validate`'s provenance check exists to refuse.
/// Answers the forged bytes of `planning/story/closed-a.md` and the commit `loose_store` made.
fn forged_pack_store(name: &str) -> (PathBuf, Vec<u8>, String) {
    let project = packed_store(name);
    let loose_commit = git(&project, &["rev-parse", "HEAD~1"]).trim().to_owned();
    let file = pack_holding(&project, "planning/story/closed-a.md");
    let location = packs_directory(&project).join(&file);
    let parsed = pack::Pack::parse(&std::fs::read(&location).expect("reads")).expect("parses");
    let mut forged_bytes = Vec::new();
    let forged: Vec<(String, Vec<u8>)> = parsed
        .records()
        .iter()
        .map(|record| {
            let bytes = if record.path() == "planning/story/closed-a.md" {
                forged_bytes =
                    swap_same_length(record.bytes(), "title: Closed A", "title: Forged A");
                forged_bytes.clone()
            } else {
                record.bytes().to_vec()
            };
            (record.path().to_owned(), bytes)
        })
        .collect();
    let rendered = pack::render(&forged).expect("renders");
    std::fs::remove_file(&location).expect("removable");
    std::fs::write(
        packs_directory(&project).join(pack::file_name(&pack::name(&rendered))),
        &rendered,
    )
    .expect("writable");
    commit(&project, "a pack no loose file ever held");

    let control = artifact(&project, &["validate"]);
    assert_eq!(
        control.status.code(),
        Some(1),
        "precondition: the forged pack is refused: {}",
        said(&control)
    );
    assert!(
        said(&control).contains("[pack_record_not_committed_bytes]"),
        "{}",
        said(&control)
    );
    (project, forged_bytes, loose_commit)
}

/// The forged bytes written as a blob into the repository's object store (no ref, no commit on
/// any branch): answers its object id.
fn forged_blob(project: &Path, bytes: &[u8]) -> String {
    let file = project.join(".git").join("aep-adversary-forged");
    std::fs::write(&file, bytes).expect("writable");
    let id = git(
        project,
        &["hash-object", "-w", file.to_str().expect("utf-8")],
    )
    .trim()
    .to_owned();
    std::fs::remove_file(&file).expect("removable");
    id
}

/// `GIT_GRAFT_FILE` names a file that gives the commit adding the forged pack another parent: a
/// commit (on no branch) whose tree is the loose store with `closed-a.md` holding the forged
/// bytes. Git honours the variable for `rev-parse <added>^` and `cat-file`, so the "last
/// committed blob before the commit that added the pack" is the forged one. Provenance may fail
/// closed under it; it must not pass the forged record.
#[test]
fn git_graft_file_does_not_make_provenance_pass_a_forged_pack() {
    let (project, forged_bytes, loose_commit) = forged_pack_store("graft-file");
    let added = git(&project, &["rev-parse", "HEAD"]).trim().to_owned();
    let blob = forged_blob(&project, &forged_bytes);
    let index = project.join(".git").join("aep-adversary-index");
    let index = index.to_str().expect("utf-8");
    let with_index = [("GIT_INDEX_FILE", index)];
    git_env(&project, &["read-tree", &loose_commit], &with_index);
    git_env(
        &project,
        &[
            "update-index",
            "--cacheinfo",
            &format!("100644,{blob},.engineering/planning/story/closed-a.md"),
        ],
        &with_index,
    );
    let tree = git_env(&project, &["write-tree"], &with_index)
        .trim()
        .to_owned();
    let forged_parent = git(
        &project,
        &["commit-tree", &tree, "-p", &loose_commit, "-m", "forged"],
    )
    .trim()
    .to_owned();
    let grafts = project.join(".git").join("aep-adversary-grafts");
    std::fs::write(&grafts, format!("{added} {forged_parent}\n")).expect("writable");
    let grafts = grafts.to_str().expect("utf-8").to_owned();
    let env = [("GIT_GRAFT_FILE", grafts.as_str())];

    let seen = git_env(
        &project,
        &[
            "cat-file",
            "-p",
            &format!("{added}^:.engineering/planning/story/closed-a.md"),
        ],
        &env,
    );
    assert!(
        seen.contains("title: Forged A"),
        "precondition: under the variable Git answers the forged bytes before the pack: {seen}"
    );

    let output = aep_with(&project, &["plan", "artifact", "validate"], &env);
    let text = said(&output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "GIT_GRAFT_FILE rewrites the parent of the commit that added the pack and the forged \
         pack validates: {text}"
    );
    assert!(text.contains("[pack_record_not_committed_bytes]"), "{text}");
}

/// `GIT_REPLACE_REF_BASE` names a ref namespace whose refs replace objects: a tag (tags are
/// fetched by a default clone) named by the original blob's id and pointing at the forged blob
/// makes every read of the original blob answer the forged bytes. Provenance may fail closed
/// under it; it must not pass the forged record.
#[test]
fn git_replace_ref_base_does_not_make_provenance_pass_a_forged_pack() {
    let (project, forged_bytes, loose_commit) = forged_pack_store("replace-ref-base");
    let original = git(
        &project,
        &[
            "rev-parse",
            &format!("{loose_commit}:.engineering/planning/story/closed-a.md"),
        ],
    )
    .trim()
    .to_owned();
    let blob = forged_blob(&project, &forged_bytes);
    git(
        &project,
        &[
            "update-ref",
            &format!("refs/tags/aep-adversary/{original}"),
            &blob,
        ],
    );
    let env = [("GIT_REPLACE_REF_BASE", "refs/tags/aep-adversary/")];

    let seen = git_env(
        &project,
        &[
            "cat-file",
            "-p",
            &format!("{loose_commit}:.engineering/planning/story/closed-a.md"),
        ],
        &env,
    );
    assert!(
        seen.contains("title: Forged A"),
        "precondition: under the variable Git answers the forged bytes for the original blob: \
         {seen}"
    );

    let output = aep_with(&project, &["plan", "artifact", "validate"], &env);
    let text = said(&output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "GIT_REPLACE_REF_BASE replaces the committed blob and the forged pack validates: {text}"
    );
    assert!(text.contains("[pack_record_not_committed_bytes]"), "{text}");
}

// ---------------------------------------------------------------------------------------------
// 4. A check that predates packs, where the repository is selected by the environment only.
// ---------------------------------------------------------------------------------------------

/// The pre-existing `validate` check *a committed evidence file differs from its committed blob*
/// on a loose store with no pack, in a checkout whose Git directory lives outside the work tree
/// and is selected by `GIT_DIR` and `GIT_WORK_TREE` (the separate-git-dir layout; nothing in the
/// tree is named `.git`). Plain `git` reads that repository and says the file changed; before the
/// security fixes `validate` ran plain `git` and refused the rewrite. With the repository-selecting
/// variables removed and `repository_above` finding no `.git`, the store counts as unversioned and
/// the rewrite validates.
#[test]
fn validate_still_reports_a_rewritten_evidence_file_where_git_dir_selects_the_repository() {
    let project = loose_store("env-git-dir");
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
    std::fs::write(&file, bytes.replacen("task check", "task chock", 1)).expect("writable");

    let control = artifact(&project, &["validate"]);
    assert!(
        control.status.code() == Some(1)
            && said(&control).contains("differs from its committed blob"),
        "precondition: the rewrite is refused with `.git` in the tree: {}",
        said(&control)
    );

    let outside = scratch_root().join("store-pack-security-adversary-2-env-git-dir.git");
    if outside.exists() {
        std::fs::remove_dir_all(&outside).expect("the previous scratch is removable");
    }
    std::fs::rename(project.join(".git"), &outside).expect("the git directory moves");
    let git_dir = outside.display().to_string();
    let work_tree = project.display().to_string();
    let env = [
        ("GIT_DIR", git_dir.as_str()),
        ("GIT_WORK_TREE", work_tree.as_str()),
    ];
    let changed = git_env(&project, &["diff", "--name-only", "HEAD"], &env);
    assert!(
        changed.contains(".engineering/evidence/story/closed-a/"),
        "precondition: plain git with the environment sees the rewrite: {changed}"
    );

    let output = aep_with(&project, &["plan", "artifact", "validate"], &env);
    let text = said(&output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "with the repository selected by GIT_DIR the rewritten evidence file validates: {text}"
    );
    assert!(text.contains("differs from its committed blob"), "{text}");
}
