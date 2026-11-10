//! Adversary cases for `aep plan store pack` and `unpack`: a rerun after an interrupted write
//! converges, the merge Git reports between two unpacks resolves into packs that still read, a
//! clone's line-ending setting does not change what is packed, a pack is never written outside the
//! repository, a non-Git store is not packed, and a round trip keeps what Git records of a file.

use std::os::unix::fs::{symlink, PermissionsExt};
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

fn scratch(name: &str) -> PathBuf {
    let directory =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("store-pack-adv-{name}"));
    if directory.exists() {
        make_writable(&directory);
        std::fs::remove_dir_all(&directory).expect("the previous scratch is removable");
    }
    directory
}

fn make_writable(directory: &Path) {
    if let Ok(metadata) = std::fs::symlink_metadata(directory) {
        if metadata.is_dir() {
            let _ = std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o755));
            if let Ok(entries) = std::fs::read_dir(directory) {
                for entry in entries.flatten() {
                    make_writable(&entry.path());
                }
            }
        }
    }
}

fn set_mode(path: &Path, mode: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("chmod");
}

fn git_output(project: &Path, args: &[&str]) -> Output {
    Command::new("git")
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
        .expect("git runs")
}

fn git(project: &Path, args: &[&str]) -> String {
    let output = git_output(project, args);
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

fn said(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn ok(project: &Path, args: &[&str]) {
    let mut full = vec!["plan", "artifact"];
    full.extend_from_slice(args);
    let output = aep_in(project, &full);
    assert!(
        output.status.success(),
        "`{}`: {}",
        args.join(" "),
        said(&output)
    );
}

fn store(project: &Path, args: &[&str]) -> Output {
    let mut full = vec!["plan", "store"];
    full.extend_from_slice(args);
    aep_in(project, &full)
}

fn store_ok(project: &Path, args: &[&str]) -> String {
    let output = store(project, args);
    assert!(
        output.status.success(),
        "`store {}` failed: {}",
        args.join(" "),
        said(&output)
    );
    said(&output)
}

fn engineering(project: &Path) -> PathBuf {
    project.join(".engineering")
}

fn packs(project: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(engineering(project).join("packs"))
        .map(|entries| {
            entries
                .map(|entry| entry.expect("an entry").file_name().into_string().unwrap())
                .filter(|name| Path::new(name).extension().is_some_and(|e| e == "pack"))
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

fn project_version(project: &Path) -> String {
    std::fs::read_to_string(engineering(project).join("project.yaml"))
        .expect("the project file reads")
        .lines()
        .next()
        .expect("a first line")
        .to_owned()
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

fn story(project: &Path, name: &str, rungs: &[&str]) {
    ok(project, &["new", "story", name, "--title", name]);
    for rung in rungs {
        ok(project, &["move", &format!("story:{name}"), "--to", rung]);
    }
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
/// (archived) and `open-c` (active), on branch `main`.
fn loose_store(name: &str) -> PathBuf {
    let project = scratch(name);
    std::fs::create_dir_all(engineering(&project)).expect("the scratch is writable");
    git(&project, &["init", "--quiet", "--initial-branch=main"]);
    std::fs::write(
        engineering(&project).join("project.yaml"),
        format!(
            "version: aep.project/5\nplanning_scope: store-pack-adversary\nprotocol: adp/1\n\
             profile: development.standard\nprotocols: {}\nstore:\n  git: {{}}\n",
            relative(&engineering(&project), &repository())
        ),
    )
    .expect("the project file is writable");
    story(&project, "closed-b", &["proposed", "active", "archived"]);
    story(&project, "closed-a", &["proposed", "active"]);
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
    story(&project, "open-c", &["proposed", "active"]);
    commit(&project, "a loose store");
    project
}

/// Every loose file of `story:closed-a`'s evidence directory.
fn closed_a_evidence(project: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> =
        std::fs::read_dir(engineering(project).join("evidence/story/closed-a"))
            .expect("the evidence directory lists")
            .map(|entry| entry.expect("an entry").path())
            .collect();
    found.sort();
    found
}

/// `unpack --all` restored every file and deleted the last pack, then failed writing
/// `project.yaml` (its directory is read-only: the same state a crash at that point leaves). The
/// rerun must finish the job: no pack is left, so `project.yaml` returns to `aep.project/5`. Today
/// the rerun answers "nothing to unpack" and leaves `aep.project/6`, which `migrate git` then
/// refuses with "run `aep plan store unpack --all` first" — a loop with no exit.
#[test]
fn unpack_all_rerun_after_an_interrupted_unpack_returns_project_yaml_to_v5() {
    let project = loose_store("interrupted-unpack");
    store_ok(&project, &["pack"]);
    assert_eq!(project_version(&project), "version: aep.project/6");

    set_mode(&engineering(&project), 0o555);
    let interrupted = store(&project, &["unpack", "--all"]);
    set_mode(&engineering(&project), 0o755);
    assert_eq!(
        interrupted.status.code(),
        Some(1),
        "the project file write fails: {}",
        said(&interrupted)
    );
    assert_eq!(
        packs(&project),
        Vec::<String>::new(),
        "the last pack was deleted"
    );

    let rerun = store_ok(&project, &["unpack", "--all"]);
    assert_eq!(
        project_version(&project),
        "version: aep.project/5",
        "no pack remains, so the store is `/5` again: {rerun}"
    );
    assert!(
        !engineering(&project).join("packs").exists(),
        "the emptied packs directory goes with the last pack: {rerun}"
    );
    assert_eq!(git(&project, &["status", "--porcelain"]), "");
}

/// Two branches each unpack a different artifact from one pack; with Git's default rename
/// detection the merge reports a rename/rename conflict on the pack (known, reported by the unit).
/// Resolving it the way the conflict suggests — keep both new packs, drop the old one — must leave
/// packs that read, so that `unpack` can then resolve the leftovers. `.gitattributes` holds only
/// `*.pack -text`, so Git still merges pack files as text and writes conflict markers into both.
#[test]
fn a_rename_conflict_between_two_unpacks_resolved_by_keeping_both_packs_leaves_packs_that_read() {
    let project = loose_store("rename-merge");
    // Records both branches keep, so that each rewrite is similar enough to the old pack for Git
    // to pair it with it on both sides (rename/rename), as on any store with more closed work.
    for name in ["closed-d", "closed-e", "closed-f"] {
        story(&project, name, &["proposed", "active", "archived"]);
    }
    commit(&project, "more closed work");
    store_ok(&project, &["pack"]);
    commit(&project, "packed");
    let old = packs(&project);
    assert_eq!(old.len(), 1);
    for (branch, id) in [
        ("unpack-a", "story:closed-a"),
        ("unpack-b", "story:closed-b"),
    ] {
        git(&project, &["checkout", "--quiet", "-b", branch, "main"]);
        store_ok(&project, &["unpack", id]);
        commit(&project, &format!("unpack {id}"));
        git(&project, &["checkout", "--quiet", "main"]);
    }
    git(&project, &["merge", "--quiet", "--no-edit", "unpack-a"]);
    let merge = git_output(&project, &["merge", "--no-edit", "unpack-b"]);
    assert!(
        !merge.status.success(),
        "the merge conflicts on the pack (the known behaviour): {}",
        said(&merge)
    );

    // Keep both new packs, drop the old one.
    for name in packs(&project) {
        if name != old[0] {
            git(&project, &["add", &format!(".engineering/packs/{name}")]);
        }
    }
    git(
        &project,
        &[
            "rm",
            "--quiet",
            "--cached",
            "--ignore-unmatch",
            &format!(".engineering/packs/{}", old[0]),
        ],
    );
    let _ = std::fs::remove_file(engineering(&project).join("packs").join(&old[0]));
    git(&project, &["commit", "--quiet", "--no-verify", "--no-edit"]);

    let kept = packs(&project);
    assert_eq!(kept.len(), 2, "{kept:?}");
    for name in &kept {
        let bytes = std::fs::read(engineering(&project).join("packs").join(name)).expect("reads");
        if let Err(error) = pack::Pack::parse(&bytes) {
            panic!(
                "{name} does not read after the merge: [{}] {}\n{}",
                error.code().as_str(),
                error.detail(),
                String::from_utf8_lossy(&bytes)
            );
        }
    }
    store_ok(&project, &["unpack", "story:closed-a"]);
    store_ok(&project, &["unpack", "story:closed-b"]);
    assert_eq!(project_version(&project), "version: aep.project/5");
}

/// Two clones of one commit, one with `core.autocrlf=true`. Both report a clean `git status`; the
/// acceptance has `pack` write the same pack on both. Today the autocrlf clone compares its CRLF
/// working bytes, unfiltered, with the LF blobs, calls every candidate "differs from HEAD" and
/// packs nothing.
#[test]
fn pack_on_a_clone_with_core_autocrlf_writes_the_pack_a_plain_clone_writes() {
    let origin = loose_store("autocrlf-origin");
    let plain = scratch("autocrlf-plain");
    let crlf = scratch("autocrlf-crlf");
    git(
        &origin,
        &["clone", "--quiet", ".", plain.to_str().expect("printable")],
    );
    git(
        &origin,
        &[
            "clone",
            "--quiet",
            "-c",
            "core.autocrlf=true",
            ".",
            crlf.to_str().expect("printable"),
        ],
    );
    assert_eq!(
        git(&crlf, &["status", "--porcelain"]),
        "",
        "Git sees the clone clean"
    );

    store_ok(&plain, &["pack"]);
    let said_crlf = store_ok(&crlf, &["pack"]);
    let (one, two) = (packs(&plain), packs(&crlf));
    assert_eq!(one.len(), 1);
    assert_eq!(one, two, "the same pack on both clones: {said_crlf}");
}

/// A `.engineering/packs` that is a symbolic link to a directory outside the repository. `pack`
/// writes the pack there and removes the loose files, so the records leave the Git work tree: the
/// next commit records their deletion and no pack. It must refuse, as `unpack` refuses a target
/// behind a symbolic link, and keep every loose file.
#[test]
fn pack_refuses_a_packs_directory_that_is_a_symbolic_link_and_keeps_the_loose_files() {
    let project = loose_store("packs-symlink");
    let outside = scratch("packs-symlink-outside");
    std::fs::create_dir_all(&outside).expect("writable");
    symlink(&outside, engineering(&project).join("packs")).expect("links");
    let before = files(&engineering(&project).join("planning"));

    let output = store(&project, &["pack"]);
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    assert!(said(&output).contains("symbolic link"), "{}", said(&output));
    assert_eq!(files(&engineering(&project).join("planning")), before);
    assert_eq!(
        files(&outside),
        Vec::new(),
        "nothing is written outside the repository"
    );
}

/// A store whose `store:` selects SQLite keeps its plan in the database; Markdown files left under
/// `.engineering/planning` are not its records. `pack` must refuse it and leave `project.yaml` at
/// `aep.project/5`; today it packs the files and selects `aep.project/6`.
#[test]
fn pack_refuses_a_store_whose_selector_is_not_git_native() {
    let project = loose_store("sqlite-selector");
    let selector = engineering(&project).join("project.yaml");
    let text = std::fs::read_to_string(&selector).expect("reads");
    std::fs::write(
        &selector,
        text.replace("  git: {}\n", "  sqlite: { path: plan.sqlite3 }\n"),
    )
    .expect("writes");
    commit(&project, "a sqlite selector");
    let before = files(&engineering(&project));

    let output = store(&project, &["pack"]);
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    assert_eq!(files(&engineering(&project)), before, "nothing was written");
}

/// A committed evidence file with the executable bit (mode 100755). The pack format carries bytes
/// only, so `unpack --all` writes the file back 0644 and `git status` reports a mode change: the
/// acceptance's empty `git status --porcelain` after the round trip does not hold. Either `pack`
/// keeps such a file loose, or the round trip restores the mode.
#[test]
fn unpack_all_after_pack_leaves_git_status_empty_for_an_executable_evidence_file() {
    let project = loose_store("executable");
    let evidence = closed_a_evidence(&project);
    set_mode(&evidence[0], 0o755);
    commit(&project, "an executable evidence file");
    assert!(git(&project, &["ls-files", "-s", "--", ".engineering/evidence"]).contains("100755"));

    store_ok(&project, &["pack"]);
    store_ok(&project, &["unpack", "--all"]);
    assert_eq!(git(&project, &["status", "--porcelain"]), "");
}

/// Probe that holds today, kept as a guard: `pack` interrupted between writing the pack and removing
/// the loose files (one directory read-only) keeps every record on disk, and the rerun converges to
/// the uninterrupted result.
#[test]
fn pack_interrupted_while_removing_loose_files_keeps_every_record_and_the_rerun_converges() {
    let project = loose_store("interrupted-pack");
    let originals = files(&engineering(&project));
    set_mode(&engineering(&project).join("planning/story"), 0o555);
    let interrupted = store(&project, &["pack"]);
    set_mode(&engineering(&project).join("planning/story"), 0o755);
    assert_eq!(interrupted.status.code(), Some(1), "{}", said(&interrupted));

    let held: Vec<pack::Pack> = packs(&project)
        .iter()
        .map(|name| {
            pack::Pack::parse(
                &std::fs::read(engineering(&project).join("packs").join(name)).expect("reads"),
            )
            .expect("the pack parses")
        })
        .collect();
    let records = originals
        .iter()
        .filter(|(path, _)| path.starts_with("planning/") || path.starts_with("evidence/"));
    for (path, bytes) in records {
        let loose = std::fs::read(engineering(&project).join(path)).ok();
        let packed = held
            .iter()
            .find_map(|p| p.get(path).map(|r| r.bytes().to_vec()));
        assert!(
            loose.as_deref() == Some(bytes.as_slice())
                || packed.as_deref() == Some(bytes.as_slice()),
            "{path} is held by neither a loose file nor a pack"
        );
    }

    store_ok(&project, &["pack"]);
    assert_eq!(packs(&project).len(), 1);
    assert!(!engineering(&project)
        .join("planning/story/closed-a.md")
        .exists());
    store_ok(&project, &["unpack", "--all"]);
    assert_eq!(git(&project, &["status", "--porcelain"]), "");
}
