//! `protocol reverse` integration tests.
//!
//! Every fixture here is written by the test that reads it. That is deliberate and it is the one
//! rule this file has: the verb's whole job is to read somebody else's repository, so a fixture
//! copied out of a real one would put that project's paths, job names and domain language into a
//! public repository — and a scanner is exactly the tool that would carry them in without anybody
//! noticing. A synthetic tree also lets a test assert an exact count, which a real one cannot.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// Runs `protocol` with `args` from `directory`.
fn protocol_in(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(args)
        .current_dir(directory)
        .output()
        .expect("the protocol binary runs")
}

/// Standard output as a string.
fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Standard error as a string.
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The exit code, which is part of the contract with a calling harness.
fn code(output: &Output) -> i32 {
    output.status.code().expect("the process exited normally")
}

/// An empty scratch directory.
fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(name);
    std::fs::remove_dir_all(&directory).ok();
    std::fs::create_dir_all(&directory).expect("the temporary tree is writable");
    directory
}

/// Writes a fixture file, creating the directories above it.
fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the temporary tree is writable");
    }
    std::fs::write(path, contents).expect("the fixture is writable");
}

/// The repository root, for the tests that adopt against this tree.
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// A path from `<project>/.engineering` to `target`, since a project file may not name one
/// absolutely.
///
/// The scratch directory and the repository are both absolute and both known, so this is arithmetic
/// rather than a guess: drop the common prefix, climb out of what is left of the base, descend into
/// what is left of the target. Written here rather than reached for from a crate because it is six
/// lines and a test dependency is forever.
fn relative_from_engineering(project: &Path, target: &Path) -> String {
    let base = project.join(".engineering");
    let base: Vec<_> = base.components().collect();
    let target: Vec<_> = target.components().collect();
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

/// A synthetic repository with one of everything a scan looks for.
///
/// Invented, and invented to be awkward in the specific ways a real tree is: a `TODO` that is a
/// legitimate function call, a heading inside a fenced block, a CI job that turns a suite off, and
/// an interface document whose *directory* names its kind while its file name does not.
fn fixture(name: &str) -> PathBuf {
    let root = scratch(name);

    write(
        &root.join("README.md"),
        "# Widget service\n\
         \n\
         ## Roadmap\n\
         \n\
         Stage one is the thing.\n\
         \n\
         ```sh\n\
         # not a heading, it is a shell comment\n\
         widgetctl run\n\
         ```\n\
         \n\
         ## Drivers\n",
    );

    write(
        &root.join("NOTES.md"),
        "# Loose notes\n\nSomething somebody wrote down once.\n",
    );

    write(
        &root.join("internal/widget/widget.go"),
        "package widget\n\
         \n\
         // TODO: the non-test mode is not implemented\n\
         func New() {}\n\
         \n\
         func Wait() {\n\
         \tctx := context.TODO()\n\
         \t_ = ctx\n\
         }\n\
         \n\
         // FIXME resource is never released\n\
         func Close() {}\n",
    );

    write(
        &root.join(".gitlab-ci.yml"),
        "stages:\n\
         \x20 - test\n\
         \n\
         variables:\n\
         \x20 IMAGE: widget\n\
         \n\
         test:\n\
         \x20 variables:\n\
         \x20   RUN_INTEGRATION: 'false'\n\
         \x20   LOGLEVEL: info\n\
         \n\
         .hidden:\n\
         \x20 script: echo skipped\n",
    );

    write(
        &root.join("Taskfile.yml"),
        "version: '3'\n\
         \n\
         tasks:\n\
         \x20 build:\n\
         \x20   desc: Builds the widget\n\
         \x20   cmds:\n\
         \x20     - go build ./...\n\
         \x20 test:\n\
         \x20   cmds:\n\
         \x20     - go test ./...\n",
    );

    write(
        &root.join("generated/openapi/widget-service.yaml"),
        "openapi: 3.1.0\n\
         info:\n\
         \x20 title: Widget service\n\
         \x20 version: v1\n\
         paths:\n\
         \x20 /widgets:\n\
         \x20   get:\n\
         \x20     operationId: listWidgets\n\
         \x20     responses:\n\
         \x20       '200':\n\
         \x20         description: The widgets.\n\
         \x20   post:\n\
         \x20     operationId: createWidget\n\
         \x20     responses:\n\
         \x20       '202':\n\
         \x20         description: Accepted.\n\
         \x20       '422':\n\
         \x20         description: Refused.\n",
    );

    // Nothing under here may reach the bundle: build output says what a toolchain did.
    write(
        &root.join("target/debug/generated.go"),
        "// TODO: not real\n",
    );
    write(&root.join(".git/config"), "[core]\n");

    root
}

/// The bundle a scan of `root` produces, as JSON.
fn bundle(root: &Path) -> Value {
    let output = protocol_in(root, &["reverse", "scan", ".", "--format", "json"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    serde_json::from_str(&stdout(&output)).expect("the bundle is JSON")
}

/// Entries of one bundle section.
fn section<'a>(bundle: &'a Value, name: &str) -> &'a Vec<Value> {
    bundle[name]
        .as_array()
        .unwrap_or_else(|| panic!("the bundle has a `{name}` array"))
}

#[test]
fn every_verb_can_be_built_and_asked_for_help() {
    let root = repository_root();
    for verb in ["scan", "init", "openapi", "history"] {
        let output = protocol_in(&root, &["reverse", verb, "--help"]);
        assert_eq!(
            code(&output),
            0,
            "`reverse {verb} --help`: {}",
            stderr(&output)
        );
    }
}

#[test]
fn one_tree_scans_to_the_same_bytes_twice() {
    // The property a committed bundle rests on. A `read_dir` order that leaked into the output
    // would make this pass on the machine that wrote the bundle and fail on the one reviewing it.
    let root = fixture("aep-reverse-determinism");
    let first = protocol_in(&root, &["reverse", "scan", ".", "--format", "json"]);
    let second = protocol_in(&root, &["reverse", "scan", ".", "--format", "json"]);
    assert_eq!(stdout(&first), stdout(&second));
    let text_first = protocol_in(&root, &["reverse", "scan", "."]);
    let text_second = protocol_in(&root, &["reverse", "scan", "."]);
    assert_eq!(stdout(&text_first), stdout(&text_second));
}

#[test]
fn every_citation_a_scan_emits_resolves_to_a_real_line() {
    // The one property that makes a bundle worth citing in an artifact. An entry pointing at a line
    // that is not there is worse than no entry: a reader who follows it concludes the artifact is
    // about something else.
    let root = fixture("aep-reverse-citations");
    let scanned = bundle(&root);

    for name in [
        "readme_outline",
        "todo_sites",
        "ci_jobs",
        "task_targets",
        "api_surfaces",
    ] {
        for entry in section(&scanned, name) {
            let path = entry["path"].as_str().expect("an entry carries a path");
            let line = usize::try_from(entry["line"].as_u64().expect("an entry carries a line"))
                .expect("a line number fits");
            let text = std::fs::read_to_string(root.join(path))
                .unwrap_or_else(|error| panic!("{name}: {path} is readable: {error}"));
            let count = text.lines().count();
            assert!(
                line >= 1 && line <= count,
                "{name}: {path}:{line} is outside the file's {count} lines"
            );
        }
    }

    // And the citation says what the entry claims, not merely somewhere in the same file.
    for heading in section(&scanned, "readme_outline") {
        let path = heading["path"].as_str().expect("a path");
        let line = usize::try_from(heading["line"].as_u64().expect("a line")).expect("fits");
        let wanted = heading["text"].as_str().expect("the heading text");
        let text = std::fs::read_to_string(root.join(path)).expect("readable");
        let actual = text.lines().nth(line - 1).expect("the cited line");
        assert!(
            actual.contains(wanted),
            "{path}:{line} is `{actual}`, which does not carry `{wanted}`"
        );
    }

    for site in section(&scanned, "todo_sites") {
        let path = site["path"].as_str().expect("a path");
        let line = usize::try_from(site["line"].as_u64().expect("a line")).expect("fits");
        let marker = site["marker"].as_str().expect("a marker");
        let text = std::fs::read_to_string(root.join(path)).expect("readable");
        let actual = text.lines().nth(line - 1).expect("the cited line");
        assert!(
            actual.contains(marker),
            "{path}:{line} is `{actual}`, which does not carry `{marker}`"
        );
    }
}

#[test]
fn a_scan_reads_what_the_fixture_says_and_not_what_it_does_not() {
    let root = fixture("aep-reverse-content");
    let scanned = bundle(&root);

    let headings: Vec<&str> = section(&scanned, "readme_outline")
        .iter()
        .map(|entry| entry["text"].as_str().expect("text"))
        .collect();
    assert_eq!(headings, vec!["Widget service", "Roadmap", "Drivers"]);

    // The fenced `# not a heading` is a shell comment. A plan built from a bundle that called it a
    // section would carry a heading nobody wrote.
    assert!(
        !headings
            .iter()
            .any(|heading| heading.contains("shell comment")),
        "a fenced comment was read as a heading: {headings:?}"
    );

    let markers: Vec<&str> = section(&scanned, "todo_sites")
        .iter()
        .map(|entry| entry["marker"].as_str().expect("marker"))
        .collect();
    assert_eq!(
        markers,
        vec!["TODO", "FIXME"],
        "one comment each, and no third"
    );

    let jobs: Vec<&str> = section(&scanned, "ci_jobs")
        .iter()
        .map(|entry| entry["name"].as_str().expect("name"))
        .collect();
    assert_eq!(
        jobs,
        vec!["test"],
        "`stages`, `variables` and `.hidden` are not jobs"
    );
    assert_eq!(
        section(&scanned, "ci_jobs")[0]["variables"]["RUN_INTEGRATION"],
        Value::from("false"),
        "a suite switched off in CI is the single most useful thing a scan finds"
    );

    let targets: Vec<&str> = section(&scanned, "task_targets")
        .iter()
        .map(|entry| entry["name"].as_str().expect("name"))
        .collect();
    assert_eq!(targets, vec!["build", "test"]);

    let surfaces = section(&scanned, "api_surfaces");
    assert_eq!(surfaces.len(), 1);
    assert_eq!(surfaces[0]["kind"], Value::from("openapi"));
    assert_eq!(
        surfaces[0]["operations"],
        Value::from(2),
        "two operations, and the two response codes under them are not operations"
    );

    let docs: Vec<&str> = section(&scanned, "root_docs")
        .iter()
        .map(|entry| entry["path"].as_str().expect("path"))
        .collect();
    assert_eq!(
        docs,
        vec!["NOTES.md"],
        "a README is not a loose root document"
    );
}

#[test]
fn a_language_call_named_todo_is_not_unfinished_work() {
    // Go's `context.TODO()` is a legitimate call used hundreds of times in a large service. A scan
    // that counted each one would bury the one comment that is a real piece of unfinished work, and
    // the bundle's most actionable section would be its least readable.
    let root = fixture("aep-reverse-standalone");
    let scanned = bundle(&root);
    for site in section(&scanned, "todo_sites") {
        let text = site["text"].as_str().expect("text");
        assert!(
            !text.contains("context.TODO()"),
            "`context.TODO()` was read as unfinished work: {text}"
        );
    }
}

#[test]
fn build_output_and_machine_state_are_not_part_of_a_repository_plan() {
    let root = fixture("aep-reverse-skips");
    let scanned = bundle(&root);
    for name in [
        "todo_sites",
        "readme_outline",
        "api_surfaces",
        "task_targets",
        "ci_jobs",
    ] {
        for entry in section(&scanned, name) {
            let path = entry["path"].as_str().expect("a path");
            assert!(
                !path.starts_with("target/") && !path.starts_with(".git/"),
                "{name} reached {path}"
            );
        }
    }
}

#[test]
fn an_unpinned_git_source_is_refused_and_nothing_is_written() {
    // The failure `docs/guide/adopting.md` warns about, and the reason the refusal has to happen
    // here rather than at first use: a project file naming a branch means a different tree on a
    // different day, and the run that discovers it is the one that was relying on the old one.
    let root = scratch("aep-reverse-unpinned");
    let output = protocol_in(
        &root,
        &[
            "reverse",
            "init",
            "--protocols",
            "git+https://example.com/tree.git#main",
            "--profile",
            "development.standard",
        ],
    );
    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("full") && stderr(&output).contains("commit id"),
        "the refusal must name the pinning rule: {}",
        stderr(&output)
    );
    assert!(
        !root.join(".engineering/project.yaml").exists(),
        "a refused adoption left a project file behind"
    );
}

#[test]
fn an_unsupported_source_scheme_is_refused() {
    let root = scratch("aep-reverse-scheme");
    let output = protocol_in(
        &root,
        &[
            "reverse",
            "init",
            "--protocols",
            "https://example.com/tree",
            "--profile",
            "development.standard",
        ],
    );
    assert_eq!(code(&output), 1);
    assert!(stderr(&output).contains("scheme"), "{}", stderr(&output));
}

#[test]
fn init_writes_a_project_the_next_command_can_read() {
    // The point of the verb: not that a file appears, but that the file makes every later command
    // work. `artifact new` resolves its lifecycles through the project's protocol source, so it
    // succeeding is the end-to-end assertion.
    let root = scratch("aep-reverse-init");
    let tree = relative_from_engineering(&root, &repository_root());
    let output = protocol_in(
        &root,
        &[
            "reverse",
            "init",
            "--protocols",
            &tree,
            "--profile",
            "development.standard",
            "--summary",
            "A synthetic repository, adopted by a test.",
        ],
    );
    assert_eq!(code(&output), 0, "{}", stderr(&output));

    let written = std::fs::read_to_string(root.join(".engineering/project.yaml"))
        .expect("the project file was written");
    assert!(written.contains("version: aep.project/1"));
    assert!(written.contains("profile: development.standard"));

    let created = protocol_in(
        &root,
        &[
            "artifact",
            "new",
            "story",
            "first",
            "--title",
            "The first story",
        ],
    );
    assert_eq!(code(&created), 0, "{}", stderr(&created));

    let again = protocol_in(
        &root,
        &[
            "reverse",
            "init",
            "--protocols",
            &tree,
            "--profile",
            "development.standard",
        ],
    );
    assert_eq!(code(&again), 1, "adopting twice must be refused");
    assert!(stderr(&again).contains("already"), "{}", stderr(&again));
}

#[test]
fn an_absolute_protocol_source_is_refused_and_the_repository_is_left_as_it_was() {
    // A project file is committed, so it is read on every machine that clones the repository. An
    // absolute path is true on the machine that typed it and on no other, and in CI on none at all.
    //
    // The second half of the assertion is the part worth having: a refused adoption must not leave
    // an empty `.engineering` behind. A directory that exists and holds no project file is the state
    // every later command reads as "not a project", which is right, but it is also litter that the
    // next `reverse init` has to be told is safe to write into.
    let root = scratch("aep-reverse-absolute");
    let output = protocol_in(
        &root,
        &[
            "reverse",
            "init",
            "--protocols",
            "/opt/aep",
            "--profile",
            "development.standard",
        ],
    );
    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("absolute path"),
        "the refusal must say what is wrong with it: {}",
        stderr(&output)
    );
    assert!(
        !root.join(".engineering").exists(),
        "a refused adoption created a project directory"
    );
}

#[test]
fn a_relative_source_resolves_through_a_directory_that_did_not_exist_yet() {
    // The regression this test exists for: the source used to be checked before `.engineering` was
    // created, so `../../tree` failed on the missing component rather than on the tree, and the
    // error named a path the adopter never wrote. Absolute paths hid it — they resolve without
    // touching the project directory at all — so it surfaced the moment they were refused.
    let root = scratch("aep-reverse-relative-first");
    assert!(!root.join(".engineering").exists());
    let tree = relative_from_engineering(&root, &repository_root());
    assert!(
        tree.starts_with(".."),
        "the fixture must exercise a climb: {tree}"
    );
    let output = protocol_in(
        &root,
        &[
            "reverse",
            "init",
            "--protocols",
            &tree,
            "--profile",
            "development.standard",
        ],
    );
    assert_eq!(code(&output), 0, "{}", stderr(&output));
}

#[test]
fn a_vision_cannot_be_implemented_and_a_story_can() {
    // The ladder `reverse` made reachable. Before `artifacts/lifecycles/vision.yaml` existed the
    // kind resolved to the permissive fallback, so `--to implemented` succeeded and the first
    // artifact an adopting repository writes was the one nothing validated. A vision is never
    // implemented: the work under it is.
    let root = scratch("aep-reverse-vision");
    let tree = relative_from_engineering(&root, &repository_root());
    let adopted = protocol_in(
        &root,
        &[
            "reverse",
            "init",
            "--protocols",
            &tree,
            "--profile",
            "development.standard",
        ],
    );
    assert_eq!(code(&adopted), 0, "{}", stderr(&adopted));

    let created = protocol_in(
        &root,
        &[
            "artifact",
            "new",
            "vision",
            "where-we-are-going",
            "--title",
            "Where we are going",
        ],
    );
    assert_eq!(code(&created), 0, "{}", stderr(&created));

    let refused = protocol_in(
        &root,
        &[
            "artifact",
            "move",
            "vision:where-we-are-going",
            "--to",
            "implemented",
        ],
    );
    assert_eq!(code(&refused), 1, "a vision is not a unit of work");

    let permitted = protocol_in(
        &root,
        &[
            "artifact",
            "move",
            "vision:where-we-are-going",
            "--to",
            "in_review",
        ],
    );
    assert_eq!(code(&permitted), 0, "{}", stderr(&permitted));
}

#[test]
fn a_draft_names_every_decision_it_could_not_take() {
    // The one failure mode worth ruling out. A reader cannot tell an absent lifecycle from an absent
    // decision about one, so a draft that silently omitted what it could not read would be a draft
    // that looks finished.
    let root = fixture("aep-reverse-openapi");
    let output = protocol_in(
        &root,
        &[
            "reverse",
            "openapi",
            "generated/openapi/widget-service.yaml",
            "--domain",
            "acme.widget",
        ],
    );
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let draft = stdout(&output);

    for owed in ["entities", "lifecycles", "invariants", "actors"] {
        assert!(
            draft.contains(&format!("UNMAPPED: {owed}")),
            "the draft is silent about {owed}"
        );
    }
    assert!(draft.contains("domain: acme.widget"));
    assert!(draft.contains("acme.widget.ListWidgets"));
    assert!(draft.contains("acme.widget.CreateWidget"));

    // Every error a command refers to is declared, or the first thing `ess validate` reports is this
    // file's own inconsistency rather than the decisions it is waiting for.
    for line in draft.lines() {
        if let Some(referenced) = line.trim().strip_prefix("error: ") {
            assert!(
                draft.contains(&format!("- name: {referenced}")),
                "`{referenced}` is referred to and never declared"
            );
        }
    }
}

#[test]
fn a_document_that_is_not_openapi_is_refused() {
    let root = fixture("aep-reverse-not-openapi");
    let output = protocol_in(
        &root,
        &[
            "reverse",
            "openapi",
            "Taskfile.yml",
            "--domain",
            "acme.widget",
        ],
    );
    assert_eq!(code(&output), 1);
    assert!(stderr(&output).contains("openapi"), "{}", stderr(&output));
}

/// Runs `git` in `directory`, with a fixed identity and a fixed clock.
///
/// Both are set here rather than inherited: a test that took the machine's `user.name` would write a
/// different repository on every developer's laptop, and one that took the wall clock would assert
/// against today. Every date this fixture produces is a constant in the test.
fn git(directory: &Path, at: &str, args: &[&str]) -> Output {
    let output = Command::new("git")
        .args(args)
        .current_dir(directory)
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .env("GIT_AUTHOR_DATE", at)
        .env("GIT_COMMITTER_DATE", at)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .expect("git runs");
    assert!(output.status.success(), "git {args:?}: {}", stderr(&output));
    output
}

/// A synthetic repository with a history worth reading.
///
/// Three commits on fixed dates: a marked line that survives untouched from the first, a test
/// switched off in the second, and a revert in the third. Invented, because a fixture copied out of
/// a real repository would carry that project's ticket keys and file paths into a public tree — and
/// a scanner is exactly the tool that would carry them in without anybody noticing.
fn history_fixture(name: &str) -> PathBuf {
    let root = scratch(name);
    let first = "2021-03-04T10:00:00+00:00";
    let second = "2022-06-15T10:00:00+00:00";
    let third = "2023-09-20T10:00:00+00:00";

    git(&root, first, &["init", "--quiet", "-b", "main"]);

    write(
        &root.join("widget.go"),
        "package widget\n\n// TODO: the non-test mode is not implemented\nfunc New() {}\n",
    );
    git(&root, first, &["add", "-A"]);
    git(
        &root,
        first,
        &[
            "commit",
            "--quiet",
            "-m",
            "feat(widget): first cut\n\nRefs: WID2-11",
        ],
    );

    write(
        &root.join("widget_test.go"),
        "package widget\n\nfunc TestNew(t *testing.T) {\n\tt.Skip(\"flaky in CI\")\n}\n",
    );
    git(&root, second, &["add", "-A"]);
    git(
        &root,
        second,
        &[
            "commit",
            "--quiet",
            "-m",
            "fix: skip TestNew for now until we find out why\n\nRefs: WID2-11",
        ],
    );

    write(&root.join("widget.go"), "package widget\n\n// TODO: the non-test mode is not implemented\nfunc New() {}\nfunc Close() {}\n");
    git(&root, third, &["add", "-A"]);
    git(
        &root,
        third,
        &[
            "commit",
            "--quiet",
            "-m",
            "Revert \"feat(widget): something else\"",
        ],
    );
    git(&root, third, &["tag", "v0.1.0"]);

    root
}

/// The history bundle for `root`, as JSON.
fn history(root: &Path) -> Value {
    let output = protocol_in(root, &["reverse", "history", ".", "--format", "json"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    serde_json::from_str(&stdout(&output)).expect("the bundle is JSON")
}

#[test]
fn a_history_reports_what_the_commits_say() {
    let root = history_fixture("aep-reverse-history");
    let read = history(&root);

    assert_eq!(read["span"]["commits"], Value::from(3));
    assert_eq!(read["span"]["first_commit"], Value::from("2021-03-04"));
    assert_eq!(read["span"]["last_commit"], Value::from("2023-09-20"));
    assert_eq!(read["span"]["authors"], Value::from(1));
    assert_eq!(read["span"]["tags"], Value::from(1));

    let types: Vec<&str> = section(&read, "commit_types")
        .iter()
        .map(|entry| entry["name"].as_str().expect("a name"))
        .collect();
    assert!(
        types.contains(&"feat") && types.contains(&"fix"),
        "{types:?}"
    );

    let tickets = section(&read, "tickets");
    assert_eq!(tickets.len(), 1, "one key, mentioned twice");
    assert_eq!(tickets[0]["id"], Value::from("WID2-11"));
    assert_eq!(tickets[0]["commits"], Value::from(2));
    assert_eq!(tickets[0]["last_seen"], Value::from("2022-06-15"));

    assert_eq!(section(&read, "reverted").len(), 1);
    assert_eq!(
        section(&read, "stated_expiry")[0]["date"],
        Value::from("2022-06-15"),
        "`for now` is the phrase this section exists to date"
    );
}

#[test]
fn a_marked_line_is_dated_from_the_commit_that_wrote_it_and_not_from_today() {
    // The join that makes two bundles worth having: `scan` says a marked line is here, `history`
    // says it has said that since 2021. The date comes from the commit, so it is the same answer on
    // any machine, in any timezone, on any day — which is what lets a bundle be committed and still
    // be true a year later.
    let root = history_fixture("aep-reverse-line-age");
    let read = history(&root);

    let ages = section(&read, "line_ages");
    let todo = ages
        .iter()
        .find(|entry| entry["path"] == "widget.go")
        .expect("the marked line is dated");
    assert_eq!(todo["line"], Value::from(3));
    assert_eq!(
        todo["last_written"],
        Value::from("2021-03-04"),
        "the third commit touched the file and not this line"
    );

    let skipped = ages
        .iter()
        .find(|entry| entry["path"] == "widget_test.go")
        .expect("the disabled test is dated");
    assert_eq!(skipped["last_written"], Value::from("2022-06-15"));

    // Oldest first: the ranking is the finding. A flat list of marked lines is 156 equal items; the
    // same list in date order is a shortest-standing-longest problem somebody can start at the top of.
    let dates: Vec<&str> = ages
        .iter()
        .map(|entry| entry["last_written"].as_str().expect("a date"))
        .collect();
    let mut sorted = dates.clone();
    sorted.sort_unstable();
    assert_eq!(dates, sorted, "line ages must be reported oldest first");
}

#[test]
fn a_history_of_one_tree_is_the_same_bytes_twice() {
    let root = history_fixture("aep-reverse-history-determinism");
    let first = protocol_in(&root, &["reverse", "history", ".", "--format", "json"]);
    let second = protocol_in(&root, &["reverse", "history", ".", "--format", "json"]);
    assert_eq!(stdout(&first), stdout(&second));
}

#[test]
fn a_directory_with_no_history_says_so_in_one_sentence() {
    // Nine empty sections read like nine findings. A tree that was exported, or downloaded as a
    // tarball, has no history and that is an ordinary state — but it is not the same state as a
    // repository whose history happens to hold nothing, and the two must not print alike.
    let root = fixture("aep-reverse-no-history");
    let output = protocol_in(&root, &["reverse", "history", "."]);
    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("not a Git working tree"),
        "{}",
        stderr(&output)
    );

    // And the verb that needs no history still works on the same directory.
    let scanned = protocol_in(&root, &["reverse", "scan", "."]);
    assert_eq!(code(&scanned), 0, "{}", stderr(&scanned));
}

#[test]
fn a_test_that_never_runs_is_told_apart_from_one_that_is_opted_into() {
    // The whole value of the section. A guarded skip appears in every healthy repository; an
    // unguarded one is a test nobody has run since the day it was switched off, and a green pipeline
    // reports the two identically.
    let root = scratch("aep-reverse-skips-split");
    write(
        &root.join("a_test.go"),
        "package a\n\
         \n\
         func TestGuarded(t *testing.T) {\n\
         \tif testing.Short() {\n\
         \t\tt.Skip(\"short mode\")\n\
         \t}\n\
         }\n\
         \n\
         func TestNever(t *testing.T) {\n\
         \tt.Skip(\"handle later\")\n\
         }\n\
         \n\
         func TestExits(t *testing.T) {\n\
         \tos.exit(1)\n\
         }\n",
    );
    let scanned = bundle(&root);
    let tests = section(&scanned, "disabled_tests");

    assert_eq!(
        tests.len(),
        2,
        "`os.exit(1)` is not a disabled test: {tests:?}"
    );
    let guarded: Vec<bool> = tests
        .iter()
        .map(|entry| entry["guarded"].as_bool().expect("a flag"))
        .collect();
    assert_eq!(guarded, vec![true, false]);
    assert_eq!(tests[1]["reason"], Value::from("handle later"));
}
