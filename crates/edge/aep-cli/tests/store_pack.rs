//! `aep plan store pack` and `aep plan store unpack` over fixture Git stores: what the dry run
//! names is what the verb moves, the opposite verb restores it byte for byte, merge leftovers
//! resolve, and nothing is written when a refusal applies.

use std::os::unix::fs::symlink;
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
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
}

fn project_file(project: &Path, protocols: &Path, version: &str) -> String {
    format!(
        "version: {version}\nplanning_scope: store-pack-test\nprotocol: adp/1\n\
         profile: development.standard\nprotocols: {}\nstore:\n  git: {{}}\n",
        relative(&project.join(".engineering"), protocols)
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

fn aep_in(project: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(args)
        .current_dir(project)
        .env("AEP_ACTOR", "human:tester")
        .output()
        .expect("the binary runs")
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

/// Runs `aep plan artifact <args>` and requires it to succeed.
fn ok(project: &Path, args: &[&str]) -> String {
    let mut full = vec!["plan", "artifact"];
    full.extend_from_slice(args);
    let output = aep_in(project, &full);
    assert!(
        output.status.success(),
        "`{}` failed: {}",
        args.join(" "),
        said(&output)
    );
    stdout(&output)
}

/// Runs `aep plan store <args>`.
fn store(project: &Path, args: &[&str]) -> Output {
    let mut full = vec!["plan", "store"];
    full.extend_from_slice(args);
    aep_in(project, &full)
}

/// Runs `aep plan store <args>` and requires it to succeed.
fn store_ok(project: &Path, args: &[&str]) -> String {
    let output = store(project, args);
    assert!(
        output.status.success(),
        "`store {}` failed: {}",
        args.join(" "),
        said(&output)
    );
    stdout(&output)
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

/// Files under `directory` whose name is not a dot-name, the count the verbs print.
fn count(directory: &Path) -> usize {
    files(directory)
        .iter()
        .filter(|(name, _)| !name.rsplit('/').next().is_some_and(|n| n.starts_with('.')))
        .count()
}

fn engineering(project: &Path) -> PathBuf {
    project.join(".engineering")
}

fn packs(project: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(engineering(project).join("packs"))
        .map(|entries| {
            entries
                .map(|entry| entry.expect("an entry").file_name().into_string().unwrap())
                .filter(|name| {
                    std::path::Path::new(name)
                        .extension()
                        .is_some_and(|e| e == "pack")
                })
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

fn read_pack(project: &Path, file: &str) -> pack::Pack {
    pack::Pack::parse(
        &std::fs::read(engineering(project).join("packs").join(file)).expect("the pack reads"),
    )
    .expect("the pack parses")
}

fn project_version(project: &Path) -> String {
    std::fs::read_to_string(engineering(project).join("project.yaml"))
        .expect("the project file reads")
        .lines()
        .next()
        .expect("a first line")
        .to_owned()
}

fn empty_project(name: &str, protocols: &Path, git_init: bool) -> PathBuf {
    let directory = scratch_root().join(format!("store-pack-{name}"));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("the previous scratch project is removable");
    }
    std::fs::create_dir_all(engineering(&directory)).expect("the scratch is writable");
    if git_init {
        git(&directory, &["init", "--quiet"]);
    }
    std::fs::write(
        engineering(&directory).join("project.yaml"),
        project_file(&directory, protocols, "aep.project/5"),
    )
    .expect("the project file is writable");
    directory
}

fn build_story(project: &Path, name: &str, rungs: &[&str]) {
    ok(project, &["new", "story", name, "--title", name]);
    for rung in rungs {
        ok(project, &["move", &format!("story:{name}"), "--to", rung]);
    }
}

/// An implemented story with one evidence file, left uncommitted.
fn build_closed(project: &Path, name: &str) {
    build_story(project, name, &["proposed", "active"]);
    record(
        project,
        &format!("story:{name}"),
        "task check",
        "2026-09-27T10:00:00Z",
    );
    ok(
        project,
        &["move", &format!("story:{name}"), "--to", "implemented"],
    );
}

fn record(project: &Path, id: &str, source: &str, at: &str) {
    ok(
        project,
        &[
            "evidence",
            id,
            "--kind",
            "test_result",
            "--source",
            source,
            "--at",
            at,
        ],
    );
}

/// A committed `aep.project/5` store: `closed-a` (implemented, two evidence files), `closed-b`
/// (archived) and `open-c` (active, one evidence file).
fn store_under(name: &str, protocols: &Path) -> PathBuf {
    let project = empty_project(name, protocols, true);
    build_story(&project, "closed-b", &["proposed", "active", "archived"]);
    build_story(&project, "closed-a", &["proposed", "active"]);
    record(
        &project,
        "story:closed-a",
        "task check",
        "2026-09-28T10:00:00Z",
    );
    record(
        &project,
        "story:closed-a",
        "task lint",
        "2026-09-28T11:00:00Z",
    );
    ok(&project, &["move", "story:closed-a", "--to", "implemented"]);
    build_story(&project, "open-c", &["proposed", "active"]);
    record(
        &project,
        "story:open-c",
        "task check",
        "2026-09-29T10:00:00Z",
    );
    commit(&project, "a loose store");
    project
}

fn loose_store(name: &str) -> PathBuf {
    store_under(name, &repository())
}

/// A protocol tree whose lifecycles declare no `closed:`: the repository's own, `artifacts/`
/// copied without the key, everything else linked.
fn protocols_without_closed(name: &str) -> PathBuf {
    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).expect("writable");
        for entry in std::fs::read_dir(from).expect("readable") {
            let entry = entry.expect("an entry");
            let target = to.join(entry.file_name());
            if entry.path().is_dir() {
                copy(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), &target).expect("copies");
            }
        }
    }
    let tree = scratch_root().join(format!("store-pack-protocols-{name}"));
    if tree.exists() {
        std::fs::remove_dir_all(&tree).expect("removable");
    }
    std::fs::create_dir_all(&tree).expect("writable");
    for entry in std::fs::read_dir(repository()).expect("readable") {
        let entry = entry.expect("an entry");
        let file_name = entry.file_name();
        if matches!(file_name.to_str(), Some(".git" | "target")) {
            continue;
        }
        if file_name == "artifacts" {
            copy(&entry.path(), &tree.join("artifacts"));
        } else {
            symlink(entry.path(), tree.join(&file_name)).expect("links");
        }
    }
    let lifecycles = tree.join("artifacts/lifecycles");
    for entry in std::fs::read_dir(&lifecycles).expect("readable") {
        let path = entry.expect("an entry").path();
        let text = std::fs::read_to_string(&path).expect("reads");
        let kept: Vec<&str> = text
            .lines()
            .filter(|line| !line.starts_with("closed:"))
            .collect();
        std::fs::write(&path, format!("{}\n", kept.join("\n"))).expect("writes");
    }
    tree
}

/// The lines of `output` that start with `prefix`, in order.
fn lines_with<'a>(output: &'a str, prefix: &str) -> Vec<&'a str> {
    output
        .lines()
        .filter(|line| line.starts_with(prefix))
        .collect()
}

fn removed_paths(output: &str) -> Vec<String> {
    lines_with(output, "  remove ")
        .into_iter()
        .map(|line| line["  remove ".len()..].to_owned())
        .collect()
}

#[test]
fn store_help_lists_pack_and_unpack() {
    let project = loose_store("help");
    let help = stdout(&store(&project, &["--help"]));
    assert!(
        help.lines()
            .any(|line| line.trim_start().starts_with("pack ")),
        "{help}"
    );
    assert!(
        help.lines()
            .any(|line| line.trim_start().starts_with("unpack ")),
        "{help}"
    );
}

#[test]
fn pack_dry_run_reports_counts_name_skips_and_file_counts_and_changes_nothing() {
    let project = loose_store("dry-run");
    // An untracked closed story and a committed one whose file now differs from `HEAD`.
    build_closed(&project, "closed-e");
    std::fs::write(
        engineering(&project).join("planning/story/closed-b.md"),
        format!(
            "{}\nan edit not committed\n",
            std::fs::read_to_string(engineering(&project).join("planning/story/closed-b.md"))
                .expect("reads")
        ),
    )
    .expect("writes");
    let before = files(&project);
    let status = git(&project, &["status", "--porcelain"]);
    let index = std::fs::read(project.join(".git/index")).expect("the index reads");

    let output = store(&project, &["pack", "--dry-run"]);
    let said = said(&output);
    assert_eq!(output.status.code(), Some(0), "{said}");
    assert!(
        said.contains("story implemented: 1 artifact(s), 2 evidence record(s)"),
        "{said}"
    );
    assert!(
        said.contains("planning/story/closed-e.md: untracked"),
        "{said}"
    );
    assert!(
        said.contains("planning/story/closed-b.md: differs from HEAD"),
        "{said}"
    );
    assert!(said.contains("pack: "), "{said}");
    let before_planning = count(&engineering(&project).join("planning"));
    let before_evidence = count(&engineering(&project).join("evidence"));
    assert!(
        said.contains(&format!(
            "files under .engineering/planning: {before_planning} -> {}",
            before_planning - 1
        )),
        "{said}"
    );
    assert!(
        said.contains(&format!(
            "files under .engineering/evidence: {before_evidence} -> {}",
            before_evidence - 2
        )),
        "{said}"
    );
    assert!(
        said.contains("files under .engineering/packs: 0 -> 1"),
        "{said}"
    );
    assert_eq!(files(&project), before, "a dry run writes nothing");
    assert_eq!(git(&project, &["status", "--porcelain"]), status);
    assert_eq!(
        std::fs::read(project.join(".git/index")).expect("the index reads"),
        index,
        "a dry run leaves the index alone"
    );
}

#[test]
fn pack_dry_run_says_a_kind_without_closed_states_packs_its_terminal_states_only() {
    let protocols = protocols_without_closed("no-closed");
    let project = store_under("no-closed", &protocols);
    let output = store(&project, &["pack", "--dry-run"]);
    let said = said(&output);
    assert_eq!(output.status.code(), Some(0), "{said}");
    assert!(
        said.contains("story: declares no `closed:`; packs its terminal states only"),
        "{said}"
    );
    assert!(
        said.contains("story archived: 1 artifact(s), 0 evidence record(s)"),
        "{said}"
    );
    assert!(!said.contains("story implemented"), "{said}");
}

#[test]
fn pack_writes_the_pack_the_dry_run_named_and_removes_the_files_it_listed() {
    let project = loose_store("pack-writes");
    build_closed(&project, "closed-e");
    let dry = stdout(&store(&project, &["pack", "--dry-run"]));
    let real = store_ok(&project, &["pack"]);

    let written = packs(&project);
    assert_eq!(written.len(), 1, "exactly one pack: {written:?}");
    let name = written[0].trim_end_matches(".pack");
    assert!(dry.contains(&format!("pack: {name}")), "{dry}");
    assert!(real.contains(&format!("pack: {name}")), "{real}");
    assert_eq!(removed_paths(&dry), removed_paths(&real));
    assert_eq!(
        removed_paths(&real).len(),
        4,
        "two stories and two evidence files"
    );
    for path in removed_paths(&real) {
        assert!(
            !engineering(&project).join(&path).exists(),
            "{path} was removed"
        );
    }
    for kept in [
        "planning/story/open-c.md",
        "planning/story/closed-e.md",
        "project.yaml",
    ] {
        assert!(engineering(&project).join(kept).exists(), "{kept} stays");
    }
    let held = read_pack(&project, &written[0]);
    let paths: Vec<&str> = held.records().iter().map(pack::PackRecord::path).collect();
    assert_eq!(
        paths
            .iter()
            .filter(|path| path.starts_with("planning/"))
            .count(),
        2,
        "{paths:?}"
    );
    assert_eq!(
        paths
            .iter()
            .filter(|path| path.starts_with("evidence/story/closed-a/"))
            .count(),
        2,
        "{paths:?}"
    );
    assert!(
        !paths.iter().any(|path| path.contains("closed-e")
            || path.contains("open-c")
            || path.contains("project.yaml")),
        "{paths:?}"
    );
    for (counted, directory) in [
        ("planning", "planning"),
        ("evidence", "evidence"),
        ("packs", "packs"),
    ] {
        let after = count(&engineering(&project).join(directory));
        assert!(
            dry.contains(&format!("files under .engineering/{counted}:"))
                && dry.contains(&format!("-> {after}\n")),
            "{dry}"
        );
        assert!(real.contains(&format!("-> {after}\n")), "{real}");
    }
    assert_eq!(
        project_version(&project),
        "version: aep.project/6",
        "the store is `/6`"
    );
    assert_eq!(
        std::fs::read_to_string(engineering(&project).join("packs/.gitattributes"))
            .expect("the packs directory carries its attributes"),
        "*.pack -text -merge\n"
    );
    let attribute = git(
        &project,
        &[
            "check-attr",
            "text",
            "--",
            &format!(".engineering/packs/{}", written[0]),
        ],
    );
    assert!(attribute.contains("text: unset"), "{attribute}");
    let listed = ok(&project, &["list"]);
    assert!(listed.contains("story:closed-a"), "{listed}");
}

#[test]
fn pack_on_two_clones_of_one_commit_writes_the_same_bytes_under_the_same_name() {
    let first = loose_store("clone-first");
    let second = scratch_root().join("store-pack-clone-second");
    if second.exists() {
        std::fs::remove_dir_all(&second).expect("removable");
    }
    git(
        &first,
        &["clone", "--quiet", ".", second.to_str().expect("printable")],
    );
    store_ok(&first, &["pack"]);
    store_ok(&second, &["pack"]);
    let (one, two) = (packs(&first), packs(&second));
    assert_eq!(one.len(), 1);
    assert_eq!(one, two, "the same name");
    assert_eq!(
        std::fs::read(engineering(&first).join("packs").join(&one[0])).expect("reads"),
        std::fs::read(engineering(&second).join("packs").join(&two[0])).expect("reads"),
        "the same bytes"
    );
}

#[test]
fn pack_folds_in_later_loose_evidence_and_removes_a_loose_copy_a_pack_holds() {
    let project = loose_store("fold");
    store_ok(&project, &["pack"]);
    commit(&project, "packed");
    let first = packs(&project);
    assert_eq!(first.len(), 1);

    record(
        &project,
        "story:closed-a",
        "task check",
        "2026-10-01T10:00:00Z",
    );
    let held = read_pack(&project, &first[0]);
    let duplicate = held
        .records()
        .iter()
        .find(|record| record.path().starts_with("evidence/"))
        .expect("the pack holds evidence")
        .clone();
    let loose_copy = engineering(&project).join(duplicate.path());
    std::fs::create_dir_all(loose_copy.parent().expect("a parent")).expect("writable");
    std::fs::write(&loose_copy, duplicate.bytes()).expect("writes");
    commit(&project, "later evidence and a copy a pack holds");
    let new_evidence: Vec<String> = files(&engineering(&project).join("evidence/story/closed-a"))
        .into_iter()
        .map(|(name, _)| format!("evidence/story/closed-a/{name}"))
        .filter(|path| path != duplicate.path())
        .collect();
    assert_eq!(new_evidence.len(), 1, "{new_evidence:?}");

    let said = store_ok(&project, &["pack"]);
    let all = packs(&project);
    assert_eq!(all.len(), 2, "one new pack beside the first: {all:?}");
    let created: Vec<&String> = all.iter().filter(|name| **name != first[0]).collect();
    let fresh = read_pack(&project, created[0]);
    let paths: Vec<&str> = fresh.records().iter().map(pack::PackRecord::path).collect();
    assert_eq!(paths, vec![new_evidence[0].as_str()], "{said}");
    assert!(!loose_copy.exists(), "the duplicate loose copy was removed");
    assert!(
        !engineering(&project).join(&new_evidence[0]).exists(),
        "the folded-in file was removed"
    );

    // Only a copy a pack holds is left: it is removed, and no pack is written.
    std::fs::create_dir_all(loose_copy.parent().expect("a parent")).expect("writable");
    std::fs::write(&loose_copy, duplicate.bytes()).expect("writes");
    commit(&project, "a copy again");
    let said = store_ok(&project, &["pack"]);
    assert!(said.contains("no new pack"), "{said}");
    assert!(!loose_copy.exists());
    assert_eq!(packs(&project), all);
}

#[test]
fn pack_outside_a_git_work_tree_exits_one_naming_the_reason_and_writes_nothing() {
    let project = loose_store("no-git");
    std::fs::remove_dir_all(project.join(".git")).expect("removable");
    let before = files(&project);
    let output = store(&project, &["pack"]);
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    assert!(said(&output).contains("Git work tree"), "{}", said(&output));
    assert_eq!(files(&project), before);
}

#[test]
fn pack_with_nothing_eligible_writes_nothing_and_says_so() {
    let project = empty_project("nothing", &repository(), true);
    build_story(&project, "open-a", &["proposed", "active"]);
    commit(&project, "only open work");
    let before = files(&project);
    let output = store(&project, &["pack"]);
    assert_eq!(output.status.code(), Some(0), "{}", said(&output));
    assert!(
        stdout(&output).contains("nothing to pack"),
        "{}",
        said(&output)
    );
    assert_eq!(files(&project), before);
}

#[test]
fn unpack_all_dry_run_lists_what_it_would_restore_and_writes_nothing() {
    let project = loose_store("unpack-dry");
    store_ok(&project, &["pack"]);
    commit(&project, "packed");
    let before = files(&project);
    let said = store_ok(&project, &["unpack", "--all", "--dry-run"]);
    for path in ["planning/story/closed-a.md", "planning/story/closed-b.md"] {
        assert!(said.contains(&format!("  restore {path}")), "{said}");
    }
    assert_eq!(files(&project), before);
}

#[test]
fn unpack_all_after_pack_leaves_the_tree_the_commit_before_pack_held() {
    let project = loose_store("round-trip");
    store_ok(&project, &["pack"]);
    assert_ne!(
        git(&project, &["status", "--porcelain"]),
        "",
        "the pack changes the tree"
    );
    store_ok(&project, &["unpack", "--all"]);
    assert_eq!(git(&project, &["status", "--porcelain"]), "");
    assert_eq!(packs(&project), Vec::<String>::new());
    assert!(!engineering(&project).join("packs").exists());
    assert_eq!(project_version(&project), "version: aep.project/5");
}

#[test]
fn unpack_one_restores_its_files_and_rewrites_the_pack_under_its_new_name() {
    let project = loose_store("unpack-one");
    let closed_a_before = closed_a(&project);
    assert_eq!(closed_a_before.len(), 3, "the story and two evidence files");
    store_ok(&project, &["pack"]);
    commit(&project, "packed");
    let old = packs(&project);
    let said = store_ok(&project, &["unpack", "story:closed-a"]);

    for path in ["planning/story/closed-a.md", "evidence/story/closed-a"] {
        assert!(engineering(&project).join(path).exists(), "{path}: {said}");
    }
    assert!(!engineering(&project)
        .join("planning/story/closed-b.md")
        .exists());
    let now = packs(&project);
    assert_eq!(now.len(), 1, "{now:?}");
    assert_ne!(now, old, "the pack has a new name");
    let rest = read_pack(&project, &now[0]);
    let paths: Vec<&str> = rest.records().iter().map(pack::PackRecord::path).collect();
    assert_eq!(paths, vec!["planning/story/closed-b.md"]);
    assert_eq!(project_version(&project), "version: aep.project/6");
    assert_eq!(
        closed_a(&project),
        closed_a_before,
        "the restored files are the ones that were packed"
    );
}

/// The files of `story:closed-a` and their bytes.
fn closed_a(project: &Path) -> Vec<(String, Vec<u8>)> {
    files(&engineering(project))
        .into_iter()
        .filter(|(name, _)| name.contains("closed-a"))
        .collect()
}

#[test]
fn unpack_refuses_a_pack_with_a_record_path_outside_the_grammar_and_writes_nothing() {
    let project = loose_store("bad-path");
    store_ok(&project, &["pack"]);
    commit(&project, "packed");
    let bytes = format!(
        "aep.pack/1\n--- planning/../x.md 3 sha256:{}\nabc\n",
        pack::sha256_hex(b"abc")
    );
    let bad = format!("{}.pack", pack::name(bytes.as_bytes()));
    std::fs::write(engineering(&project).join("packs").join(&bad), bytes).expect("writes");
    let before = files(&project);
    let output = store(&project, &["unpack", "--all"]);
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    let text = said(&output);
    assert!(text.contains(bad.trim_end_matches(".pack")), "{text}");
    assert!(text.contains(".."), "{text}");
    assert_eq!(files(&project), before);
}

#[test]
fn unpack_refuses_a_target_behind_a_symbolic_link_and_writes_nothing() {
    let project = loose_store("symlink");
    store_ok(&project, &["pack"]);
    commit(&project, "packed");
    let elsewhere = project.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("writable");
    let planning_story = engineering(&project).join("planning/story");
    std::fs::rename(&planning_story, elsewhere.join("story")).expect("moves");
    symlink(elsewhere.join("story"), &planning_story).expect("links");
    let before = files(&project);
    let output = store(&project, &["unpack", "story:closed-a"]);
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    assert!(said(&output).contains("symbolic link"), "{}", said(&output));
    assert_eq!(files(&project), before);
    assert!(!engineering(&project)
        .join("evidence/story/closed-a")
        .exists());
}

/// Branch `name` off the current commit with `id` unpacked and committed; back on `base`.
fn unpack_on_branch(project: &Path, base: &str, name: &str, id: &str) {
    git(project, &["checkout", "--quiet", "-b", name, base]);
    store_ok(project, &["unpack", id]);
    commit(project, &format!("unpack {id}"));
    git(project, &["checkout", "--quiet", base]);
}

#[test]
fn unpack_keeps_the_loose_copy_and_drops_the_packed_one_a_merge_left_in_two_places() {
    let project = loose_store("merge-loose");
    store_ok(&project, &["pack"]);
    commit(&project, "packed");
    let base = git(&project, &["branch", "--show-current"])
        .trim()
        .to_owned();
    unpack_on_branch(&project, &base, "unpack-a", "story:closed-a");
    unpack_on_branch(&project, &base, "unpack-b", "story:closed-b");
    // Git pairs a rewritten pack with the pack it replaces by similarity and then reports a
    // rename/delete conflict when the other branch deleted the old one; the merge is built with
    // rename detection off, as the design describes it.
    for branch in ["unpack-a", "unpack-b"] {
        git(
            &project,
            &[
                "-c",
                "merge.renames=false",
                "merge",
                "--quiet",
                "--no-edit",
                branch,
            ],
        );
    }
    let story_a = std::fs::read(engineering(&project).join("planning/story/closed-a.md"))
        .expect("loose after the merge");
    assert_eq!(packs(&project).len(), 2, "{:?}", packs(&project));

    store_ok(&project, &["unpack", "story:closed-a"]);
    assert_eq!(
        std::fs::read(engineering(&project).join("planning/story/closed-a.md")).expect("reads"),
        story_a,
        "the loose copy is kept"
    );
    let left = packs(&project);
    assert_eq!(left.len(), 1, "{left:?}");
    let rest = read_pack(&project, &left[0]);
    assert!(
        rest.records()
            .iter()
            .all(|record| !record.path().contains("closed-a")),
        "the packed copies of closed-a are gone"
    );
    store_ok(&project, &["unpack", "story:closed-b"]);
    assert_eq!(packs(&project), Vec::<String>::new());
    assert_eq!(project_version(&project), "version: aep.project/5");
}

#[test]
fn unpack_refuses_a_path_two_packs_hold_differently_until_from_chooses() {
    let project = loose_store("merge-packs");
    store_ok(&project, &["pack"]);
    commit(&project, "packed");
    let original = packs(&project);
    let first = read_pack(&project, &original[0]);
    let records: Vec<(String, Vec<u8>)> = first
        .records()
        .iter()
        .map(|record| {
            let mut bytes = record.bytes().to_vec();
            if record.path() == "planning/story/closed-a.md" {
                bytes.extend_from_slice(b"\nanother branch's copy\n");
            }
            (record.path().to_owned(), bytes)
        })
        .collect();
    let rendered = pack::render(&records).expect("renders");
    let second = pack::name(&rendered);
    std::fs::write(
        engineering(&project)
            .join("packs")
            .join(pack::file_name(&second)),
        &rendered,
    )
    .expect("writes");
    let chosen_bytes = records
        .iter()
        .find(|(path, _)| path == "planning/story/closed-a.md")
        .expect("the story")
        .1
        .clone();
    let before = files(&project);
    let output = store(&project, &["unpack", "story:closed-a"]);
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    let text = said(&output);
    assert!(
        text.contains(original[0].trim_end_matches(".pack")) && text.contains(&second),
        "both packs are named: {text}"
    );
    assert_eq!(files(&project), before);

    store_ok(&project, &["unpack", "story:closed-a", "--from", &second]);
    assert_eq!(
        std::fs::read(engineering(&project).join("planning/story/closed-a.md")).expect("reads"),
        chosen_bytes
    );
    for name in packs(&project) {
        let pack = read_pack(&project, &name);
        assert!(
            pack.records()
                .iter()
                .all(|record| !record.path().contains("closed-a")),
            "{name} no longer holds closed-a"
        );
    }
}
