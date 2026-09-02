//! `aep doctor`: one test per check, each naming the condition that makes it say what it says.
//!
//! Every check here is asserted the way AGENTS.md invariant 15 asks a guard to be: the condition is
//! **broken**, the named failure is observed, the condition is **restored**, and the passing line is
//! observed too. A test that only sees the red half cannot tell a working check from one that is
//! stuck on `fail`, and a test that only sees the green half cannot tell a working check from one
//! that is stuck on `ok` — which is the defect this whole verb exists to catch in other people's
//! checkouts.
//!
//! The assertions are on the **stable code** of a line and the substance of its reason, never on a
//! line's position: a check added between two others must not redden six tests.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// What `aep --version` prints, and what the `binary-version` line must agree with.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A scratch tree of this test's own, emptied first so a rerun is a fresh run.
///
/// Deliberately outside the repository's own work tree. Two checks here answer about the *absence*
/// of a Git checkout, and a fixture under `target/` would sit inside this repository's work tree —
/// `git rev-parse --is-inside-work-tree` says `true` inside an ignored directory as readily as
/// anywhere else, so the fixture would silently be testing the wrong tree.
fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("aep-doctor-{name}"));
    std::fs::remove_dir_all(&directory).ok();
    std::fs::create_dir_all(&directory).expect("a scratch directory can be made");
    directory
}

/// Writes one file, making its parents.
fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().expect("a file has a parent"))
        .expect("the parent directory can be made");
    std::fs::write(path, contents).expect("the fixture file can be written");
}

/// A project file naming this repository's own arrangement: the tree is the checkout itself.
const PROJECT_FILE: &str = "version: aep.project/1\n\
                            protocol: adp/1\n\
                            profile: development.standard\n\
                            protocols: ..\n";

/// One valid planning document, so a store that exists has something in it.
const ONE_STORY: &str = "---\n\
                         format: aep.planning-md/1\n\
                         id: story:only\n\
                         kind: story\n\
                         status: draft\n\
                         title: The only story in this fixture\n\
                         revision: 1\n\
                         ---\n\
                         # Story\n";

/// A checkout that passes every check that can pass without a Git repository or a plugin.
fn adopting_project(name: &str) -> PathBuf {
    let root = scratch(name);
    write(&root.join(".engineering/project.yaml"), PROJECT_FILE);
    write(&root.join(".engineering/planning/story/only.md"), ONE_STORY);
    root
}

/// Runs `aep doctor` over `root`, with any extra arguments.
fn doctor(root: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .arg("doctor")
        .arg("--root")
        .arg(root)
        .args(arguments)
        .output()
        .expect("the CLI starts")
}

/// The same, with one environment variable set.
fn doctor_with(root: &Path, name: &str, value: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .arg("doctor")
        .arg("--root")
        .arg(root)
        .env(name, value)
        .output()
        .expect("the CLI starts")
}

/// Every line carrying the stable code `code`, as `(status, detail)`.
///
/// A code and not an index: a line added between two others must not move an assertion.
fn checks(output: &Output, code: &str) -> Vec<(String, String)> {
    let stdout = String::from_utf8(output.stdout.clone()).expect("the report is UTF-8");
    let found: Vec<(String, String)> = stdout
        .lines()
        .filter_map(|line| {
            let (status, rest) = line.split_at(4);
            let (name, detail) = rest.trim_start().split_once(": ")?;
            (name == code).then(|| (status.trim().to_owned(), detail.to_owned()))
        })
        .collect();
    assert!(
        !found.is_empty(),
        "no `{code}` line in the report:\n{stdout}"
    );
    found
}

/// The one line carrying `code`.
fn check(output: &Output, code: &str) -> (String, String) {
    let mut found = checks(output, code);
    assert_eq!(found.len(), 1, "`{code}` is one line: {found:?}");
    found.remove(0)
}

/// The binary reports the version it was built from, and not a number written down beside it.
///
/// Held against `aep --version` rather than against a literal: a hard-coded version would pass a
/// literal assertion for ever, and it is exactly the *stale build* confusion this line exists to
/// resolve — a binary that cannot say which build it is.
#[test]
fn the_binary_version_line_agrees_with_what_the_version_flag_prints() {
    let root = adopting_project("binary-version");
    let (status, detail) = check(&doctor(&root, &[]), "binary-version");
    assert_eq!(status, "ok", "a binary always knows its own version");

    let printed = Command::new(env!("CARGO_BIN_EXE_aep"))
        .arg("--version")
        .output()
        .expect("the CLI starts");
    let printed = String::from_utf8(printed.stdout).expect("the version line is UTF-8");
    assert!(
        printed.contains(detail.trim()) && detail.trim() == VERSION,
        "`doctor` says {detail:?} and `--version` says {printed:?}; a preflight that disagrees \
         with the binary it is preflighting is worse than no preflight"
    );
}

/// A project file that is not there, and one that is there and does not parse, are different
/// failures and say so.
#[test]
fn a_project_file_that_is_absent_or_unparseable_fails_naming_the_file_and_the_defect() {
    let root = scratch("project-file");

    let (status, detail) = check(&doctor(&root, &[]), "project-file");
    assert_eq!(status, "fail", "there is no project file at all");
    assert!(
        detail.contains("project.yaml") && detail.contains("No such file"),
        "the line names the file it looked for and why it could not read it: {detail}"
    );

    write(
        &root.join(".engineering/project.yaml"),
        "protocol: [not a ref\n",
    );
    let (status, detail) = check(&doctor(&root, &[]), "project-file");
    assert_eq!(status, "fail", "the file is there and does not parse");
    assert!(
        detail.contains("project.yaml") && !detail.contains("No such file"),
        "an unparseable file is a different failure from a missing one: {detail}"
    );

    write(&root.join(".engineering/project.yaml"), PROJECT_FILE);
    let (status, _) = check(&doctor(&root, &[]), "project-file");
    assert_eq!(status, "ok", "restoring the file restores the line");
}

/// A `protocols:` path that resolves nowhere fails, naming the path it resolved to.
///
/// The resolved path and not only the spelling: `protocols: ../tree` is resolved against
/// `.engineering`, and every adopter who has got this wrong got it wrong by one directory.
#[test]
fn a_protocol_source_path_that_is_not_there_fails_naming_where_it_resolved_to() {
    let root = adopting_project("protocol-source-path");
    let project = root.join(".engineering/project.yaml");

    write(
        &project,
        &PROJECT_FILE.replace("protocols: ..", "protocols: ../no-such-tree"),
    );
    let (status, detail) = check(&doctor(&root, &[]), "protocol-source");
    assert_eq!(
        status, "fail",
        "the tree the project is governed by is not there"
    );
    assert!(
        detail.contains("no-such-tree") && detail.contains("not a directory"),
        "the line names the resolved path and what is wrong with it: {detail}"
    );

    write(&project, PROJECT_FILE);
    let (status, detail) = check(&doctor(&root, &[]), "protocol-source");
    assert_eq!(status, "ok", "restoring the path restores the line");
    assert!(
        detail.contains("the path"),
        "the line says which of the two shapes it read: {detail}"
    );
}

/// A pinned locator is checked for a cached snapshot and never fetched.
///
/// The repository in the fixture does not exist, which is the point: a resolution that reached the
/// network would fail rather than warn, and would take seconds rather than milliseconds. The
/// restore half creates the directory the warning itself names — so the test also holds `doctor` to
/// looking where it says it looked.
#[test]
fn a_pinned_protocol_source_warns_until_its_snapshot_is_cached_and_never_fetches_it() {
    let root = adopting_project("protocol-source-locator");
    let cache = scratch("protocol-source-cache");
    let revision = "0".repeat(40);
    write(
        &root.join(".engineering/project.yaml"),
        &PROJECT_FILE.replace(
            "protocols: ..",
            &format!("protocols: git+https://example.invalid/tree#{revision}"),
        ),
    );

    let uncached = doctor_with(&root, "AEP_CACHE_DIR", &cache);
    let (status, detail) = check(&uncached, "protocol-source");
    assert_eq!(
        status, "warn",
        "an uncached pinned source does not stop the other verbs — they fetch it"
    );
    assert!(
        detail.contains("well-formed") && detail.contains("no snapshot is cached"),
        "the line says which half held and which did not: {detail}"
    );
    assert_eq!(
        uncached.status.code(),
        Some(0),
        "nothing failed, so the preflight does not exit 1: {}",
        String::from_utf8_lossy(&uncached.stdout)
    );

    let snapshot = detail
        .rsplit_once("cached at ")
        .map(|(_, path)| PathBuf::from(path.split(';').next().expect("a path").trim()))
        .expect("the warning names the snapshot path it looked for");
    std::fs::create_dir_all(&snapshot).expect("the snapshot directory can be made");

    let (status, detail) = check(
        &doctor_with(&root, "AEP_CACHE_DIR", &cache),
        "protocol-source",
    );
    assert_eq!(
        status, "ok",
        "the snapshot is where the warning said it would be"
    );
    assert!(
        detail.contains("is cached at"),
        "the line says the snapshot is there: {detail}"
    );
}

/// A store that is not there, and a store that `artifact validate` would refuse, both fail — and
/// the second fails with the finding the verb itself reports.
///
/// The last assertion is the one that matters. `doctor` promises *the store the other verbs will
/// accept*, and a preflight reading the store more weakly than `validate` does would report ready
/// about a plan `validate` refuses. Holding the two outputs against each other is the only way to
/// see that they are one code path rather than two that agree today.
#[test]
fn a_planning_store_that_is_absent_or_invalid_fails_with_the_finding_artifact_validate_reports() {
    let root = scratch("planning-store");
    write(&root.join(".engineering/project.yaml"), PROJECT_FILE);

    let (status, detail) = check(&doctor(&root, &[]), "planning-store");
    assert_eq!(status, "fail", "there is no store to plan in");
    assert!(
        detail.contains("no planning store at") && detail.contains("planning"),
        "the line names where a store would be: {detail}"
    );

    let dangling = ONE_STORY.replace(
        "revision: 1\n",
        "relations:\n- decomposes: epic:no-such-epic\nrevision: 1\n",
    );
    write(&root.join(".engineering/planning/story/only.md"), &dangling);
    let (status, detail) = check(&doctor(&root, &[]), "planning-store");
    assert_eq!(status, "fail", "an edge points at nothing");

    let validated = Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(["artifact", "validate", "--store"])
        .arg(root.join(".engineering/planning"))
        .arg("--root")
        .arg(&root)
        .output()
        .expect("the CLI starts");
    let validated = String::from_utf8(validated.stdout).expect("the report is UTF-8");
    let finding = detail
        .split_once('`')
        .and_then(|(_, rest)| rest.split_once('`'))
        .map(|(finding, _)| finding.to_owned())
        .expect("the line quotes the finding");
    assert!(
        validated.contains(&finding),
        "`doctor` reported `{finding}` and `artifact validate` reported:\n{validated}\nThe two \
         must be one accumulation, not two that happen to agree"
    );

    write(&root.join(".engineering/planning/story/only.md"), ONE_STORY);
    let (status, detail) = check(&doctor(&root, &[]), "planning-store");
    assert_eq!(status, "ok", "dropping the dangling edge restores the line");
    assert!(
        detail.contains("1 artifact(s), no problems"),
        "the line says how much it read: {detail}"
    );
}

/// A plugin directory is checked for a manifest, and no directory at all is not a defect.
///
/// The `warn` half is the load-bearing one: AEP bundles no plugin sources and guesses no path
/// (invariant 12), so *no plugin directory* is the ordinary state of every checkout and a preflight
/// that failed on it would be a preflight nobody could pass.
#[test]
fn a_plugin_directory_without_a_manifest_fails_and_no_directory_at_all_only_warns() {
    let root = adopting_project("plugin-directory");
    let plugin = scratch("plugin-directory-plugin");

    let (status, detail) = check(&doctor(&root, &[]), "plugin-directory");
    assert_eq!(status, "warn", "no plugin directory stops no other verb");
    assert!(
        detail.contains("AEP_DRIVE_PLUGIN_DIR"),
        "the line says how to name one: {detail}"
    );

    let given = doctor(&root, &["--plugin-dir", plugin.to_str().expect("a path")]);
    let (status, detail) = check(&given, "plugin-directory");
    assert_eq!(
        status, "fail",
        "a directory was named and it carries no manifest"
    );
    assert!(
        detail.contains(".claude-plugin/plugin.json")
            && detail.contains(".codex-plugin/plugin.json"),
        "the line says what it looked for: {detail}"
    );
    assert_eq!(
        given.status.code(),
        Some(1),
        "a fail decides the exit status: {}",
        String::from_utf8_lossy(&given.stdout)
    );

    write(&plugin.join(".codex-plugin/plugin.json"), "{}\n");
    let (status, detail) = check(
        &doctor(&root, &["--plugin-dir", plugin.to_str().expect("a path")]),
        "plugin-directory",
    );
    assert_eq!(status, "ok", "the manifest is there now");
    assert!(
        detail.contains(".codex-plugin/plugin.json"),
        "the line says which manifest it found, because the two are different harnesses: {detail}"
    );
}

/// `AEP_DRIVE_PLUGIN_DIR` supplies a directory only when none was named, which is `drive`'s rule.
#[test]
fn the_environment_supplies_a_plugin_directory_only_when_the_command_line_named_none() {
    let root = adopting_project("plugin-directory-env");
    let from_environment = scratch("plugin-directory-env-value");
    let named = scratch("plugin-directory-env-named");
    write(&from_environment.join(".claude-plugin/plugin.json"), "{}\n");
    write(&named.join(".claude-plugin/plugin.json"), "{}\n");

    let (_, detail) = check(
        &doctor_with(&root, "AEP_DRIVE_PLUGIN_DIR", &from_environment),
        "plugin-directory",
    );
    assert!(
        detail.contains(from_environment.to_str().expect("a path")),
        "with no `--plugin-dir`, the environment's directory is the one checked: {detail}"
    );

    let both = Command::new(env!("CARGO_BIN_EXE_aep"))
        .arg("doctor")
        .arg("--root")
        .arg(&root)
        .arg("--plugin-dir")
        .arg(&named)
        .env("AEP_DRIVE_PLUGIN_DIR", &from_environment)
        .output()
        .expect("the CLI starts");
    let reported = checks(&both, "plugin-directory");
    assert_eq!(
        reported.len(),
        1,
        "the environment is a fallback and never an addition: {reported:?}"
    );
    assert!(
        reported[0].1.contains(named.to_str().expect("a path")),
        "the named directory is the one checked: {reported:?}"
    );
}

/// A checkout with no Git repository warns; it does not fail.
#[test]
fn a_root_that_is_not_a_git_checkout_warns_rather_than_failing() {
    let root = adopting_project("release-tag-no-git");
    let output = doctor(&root, &[]);
    let (status, detail) = check(&output, "release-tag");
    assert_eq!(
        status, "warn",
        "a repository without Git is not broken, it is a repository without Git"
    );
    assert!(
        detail.contains("not a Git checkout"),
        "the line says why there is nothing to compare: {detail}"
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "no check failed, so the preflight passes: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

/// The newest bare-version tag is compared with the binary's version, and a disagreement is
/// reported without failing.
///
/// Both halves in one test, over one fixture repository, because the second is made from the first
/// by adding a tag: with the tag that matches, `ok`; with a newer one that does not, `warn` naming
/// both numbers. A `fail` here would be wrong — an adopting repository's tags are its own, so the
/// two disagree in every checkout that is not this one's.
#[test]
fn a_newest_release_tag_that_disagrees_with_the_binary_is_named_and_does_not_fail() {
    let root = adopting_project("release-tag-git");
    let git = |arguments: &[&str]| {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(&root)
            .env("GIT_AUTHOR_NAME", "aep doctor test")
            .env("GIT_AUTHOR_EMAIL", "doctor@example.invalid")
            .env("GIT_COMMITTER_NAME", "aep doctor test")
            .env("GIT_COMMITTER_EMAIL", "doctor@example.invalid")
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "--quiet", "-b", "main"]);
    git(&["add", "-A"]);
    git(&[
        "-c",
        "commit.gpgsign=false",
        "commit",
        "--quiet",
        "-m",
        "fixture",
    ]);
    git(&["tag", VERSION]);
    // A slugged tag, to hold the shape filter: `-v:refname` sorts it above the bare version, and a
    // filter that accepted it would report the wrong tag as newest.
    git(&["tag", &format!("v{VERSION}")]);

    let (status, detail) = check(&doctor(&root, &[]), "release-tag");
    assert_eq!(status, "ok", "the tag and the binary agree: {detail}");
    assert!(
        detail.contains(VERSION),
        "the line names the tag it read: {detail}"
    );

    git(&["tag", "9999.0.0"]);
    let output = doctor(&root, &[]);
    let (status, detail) = check(&output, "release-tag");
    assert_eq!(
        status, "warn",
        "a newer tag than the binary is not a defect of the binary"
    );
    assert!(
        detail.contains("9999.0.0") && detail.contains(VERSION),
        "the line names both numbers, because either one could be the stale half: {detail}"
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "a tag disagreement does not stop the other verbs, so it does not decide the exit status"
    );
}

/// Two runs over one tree print identical bytes.
///
/// The check that stands in for *reads no clock*: a report carrying an instant, a duration or
/// anything else read from outside the tree would differ between two runs a millisecond apart, and
/// a diff of two reports would stop being a diff of two checkouts.
#[test]
fn two_runs_over_one_tree_print_identical_bytes() {
    let root = adopting_project("deterministic");
    let first = doctor(&root, &[]);
    let second = doctor(&root, &[]);
    assert_eq!(
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&second.stdout),
        "the report is a function of the tree"
    );
    assert_eq!(first.status.code(), second.status.code());
}

/// The JSON rendering carries the same verdicts as the text one, keyed by the stable codes.
#[test]
fn the_json_rendering_carries_the_same_codes_and_verdicts_as_the_text_one() {
    let root = adopting_project("json");
    let text = doctor(&root, &[]);
    let json = doctor(&root, &["--format", "json"]);
    assert_eq!(
        text.status.code(),
        json.status.code(),
        "a rendering does not change a verdict"
    );

    let parsed: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("the JSON rendering parses");
    let rendered: Vec<(String, String)> = parsed["checks"]
        .as_array()
        .expect("`checks` is an array")
        .iter()
        .map(|check| {
            (
                check["check"].as_str().expect("a code").to_owned(),
                check["status"].as_str().expect("a status").to_owned(),
            )
        })
        .collect();
    let printed = String::from_utf8(text.stdout).expect("the report is UTF-8");
    let expected: Vec<(String, String)> = printed
        .lines()
        .map(|line| {
            let (status, rest) = line.split_at(4);
            let (code, _) = rest.trim_start().split_once(": ").expect("a code");
            (code.to_owned(), status.trim().to_owned())
        })
        .collect();
    assert_eq!(
        rendered, expected,
        "the two renderings are one report:\n{printed}"
    );
    assert_eq!(
        parsed["failed"].as_u64(),
        Some(0),
        "the count that decided the exit status is stated: {printed}"
    );
}
