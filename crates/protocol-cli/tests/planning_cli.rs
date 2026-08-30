//! `protocol artifact` integration tests.
//!
//! These drive the real binary against a real directory, because that is what the verb family is:
//! a plan is a tree of files, and a test that called the library would not catch an argument that
//! never reaches it, a `--format` declared twice, or a document written to the wrong path.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The repository root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// Runs `protocol` with `args`, always against the repository's own document tree.
fn protocol(args: &[&str]) -> Output {
    protocol_in(&root(), args)
}

/// Runs `protocol` with `args` from `directory`, for the verbs that discover a project.
fn protocol_in(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(args)
        .current_dir(directory)
        .output()
        .expect("the protocol binary runs")
}

/// Runs `protocol` with `args` against the repository's own tree, with `input` on standard input.
fn protocol_with_stdin(args: &[&str], input: &str) -> Output {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(args)
        .current_dir(root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the protocol binary runs");
    child
        .stdin
        .take()
        .expect("a piped standard input")
        .write_all(input.as_bytes())
        .expect("the body is written");
    child.wait_with_output().expect("the protocol binary exits")
}

/// Runs `protocol` with an isolated source cache.
fn protocol_in_with_cache(directory: &Path, cache: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(args)
        .current_dir(directory)
        .env("AEP_CACHE_DIR", cache)
        .output()
        .expect("the protocol binary runs")
}

/// Runs Git for a source fixture and returns its standard output.
fn git(directory: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(directory)
        .output()
        .expect("git runs");
    assert!(output.status.success(), "{}", stderr(&output));
    stdout(&output).trim().to_owned()
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

/// A fixture path as an argument.
fn printable(path: &Path) -> &str {
    path.to_str().expect("a printable path")
}

/// An empty scratch directory to build a store in.
fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(name);
    std::fs::remove_dir_all(&directory).ok();
    std::fs::create_dir_all(&directory).expect("the temporary tree is writable");
    directory
}

/// Copies a committed fixture store into a scratch directory, so a test can add to it.
///
/// The passkeys fixture is asserted verbatim by fifteen sites; a test that needs it *plus*
/// something works on a copy rather than adding a document to the tree everybody counts.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the temporary tree is writable");
    for entry in std::fs::read_dir(from).expect("the fixture is readable") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("the fixture copies");
        }
    }
}

/// Writes a fixture file, creating the directories above it.
fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the temporary tree is writable");
    }
    std::fs::write(path, contents).expect("the fixture is writable");
}

/// The committed planning store: one initiative, one epic, three stories, two tasks.
const FIXTURE: &str = "examples/planning-passkeys/.engineering/planning";

/// How many artifacts that fixture holds.
const FIXTURE_ARTIFACTS: usize = 7;

/// Every verb in the family, for the sweeps that have to cover all of them.
const VERBS: &[&str] = &[
    "new",
    "move",
    "relate",
    "body",
    "set",
    "show",
    "list",
    "board",
    "blocked",
    "graph",
    "validate",
    "kinds",
    "relations",
    "lifecycle",
];

#[test]
fn every_verb_can_be_built_and_asked_for_help() {
    // `clap` refuses a subcommand with two arguments of one name — but only when that subcommand is
    // built, which happens when it is invoked. `protocol artifact graph` panicked exactly that way
    // during development, because `--format` arrived both from the shared arguments and from the
    // graph's own `dot|json`. `--help` builds every one of them.
    for verb in VERBS {
        let output = protocol(&["artifact", verb, "--help"]);
        assert_eq!(
            code(&output),
            0,
            "`protocol artifact {verb} --help` failed: {}",
            stderr(&output)
        );
        assert!(
            stdout(&output).contains("--store") || *verb == "help",
            "`protocol artifact {verb} --help` does not mention --store"
        );
    }
}

#[test]
fn a_new_story_is_written_where_its_id_says_and_validates_clean() {
    let store = scratch("aep-planning-new");

    let created = protocol(&[
        "artifact",
        "new",
        "story",
        "demo",
        "--title",
        "Demo",
        "--store",
        printable(&store),
    ]);
    assert_eq!(code(&created), 0, "{}", stderr(&created));
    assert!(
        stdout(&created).contains("story:demo"),
        "{}",
        stdout(&created)
    );

    let written = store.join("story/demo.md");
    assert!(written.is_file(), "nothing was written to {written:?}");
    let text = std::fs::read_to_string(&written).expect("readable");
    assert!(
        text.starts_with("---\nformat: aep.planning-md/1\n"),
        "{text}"
    );
    assert!(
        text.contains("status: draft"),
        "the story lifecycle starts at draft: {text}"
    );

    let validated = protocol(&["artifact", "validate", "--store", printable(&store)]);
    assert_eq!(code(&validated), 0, "{}", stderr(&validated));
    assert!(
        stdout(&validated).contains("valid"),
        "{}",
        stdout(&validated)
    );
}

#[test]
fn creating_the_same_artifact_twice_is_refused_rather_than_overwriting_it() {
    let store = scratch("aep-planning-twice");
    let arguments = [
        "artifact",
        "new",
        "story",
        "demo",
        "--title",
        "Demo",
        "--store",
        printable(&store),
    ];

    assert_eq!(code(&protocol(&arguments)), 0);
    // Something a person would have lost: the body they wrote after creating it.
    let written = store.join("story/demo.md");
    write(
        &written,
        "---\nid: story:demo\nkind: story\nstatus: draft\n---\n# Hand-written\n",
    );

    let again = protocol(&arguments);
    assert_eq!(code(&again), 1, "{}", stdout(&again));
    assert!(
        stderr(&again).contains("already exists"),
        "{}",
        stderr(&again)
    );
    assert!(
        std::fs::read_to_string(&written)
            .expect("readable")
            .contains("Hand-written"),
        "the refused create overwrote the document anyway"
    );
}

#[test]
fn a_legal_move_rewrites_the_document_and_bumps_the_revision() {
    let store = scratch("aep-planning-move-legal");
    assert_eq!(
        code(&protocol(&[
            "artifact",
            "new",
            "story",
            "demo",
            "--title",
            "Demo",
            "--store",
            printable(&store),
        ])),
        0
    );

    let moved = protocol(&[
        "artifact",
        "move",
        "story:demo",
        "--to",
        "proposed",
        "--store",
        printable(&store),
    ]);
    assert_eq!(code(&moved), 0, "{}", stderr(&moved));

    let text = std::fs::read_to_string(store.join("story/demo.md")).expect("readable");
    assert!(text.contains("status: proposed"), "{text}");
    assert!(
        text.contains("revision: 2"),
        "a write that changed the file did not move the revision: {text}"
    );
}

#[test]
fn a_body_replacement_preserves_machine_owned_frontmatter() {
    let store = scratch("aep-planning-body");
    let body = store
        .parent()
        .expect("scratch has a parent")
        .join("aep-planning-body.md");
    write(&body, "# Deliberate body\n\nExact bytes.\n");
    assert_eq!(
        code(&protocol(&[
            "artifact",
            "new",
            "story",
            "demo",
            "--title",
            "Demo",
            "--store",
            printable(&store),
        ])),
        0
    );

    let replaced = protocol(&[
        "artifact",
        "body",
        "story:demo",
        "--from",
        printable(&body),
        "--store",
        printable(&store),
    ]);
    assert_eq!(code(&replaced), 0, "{}", stderr(&replaced));

    let text = std::fs::read_to_string(store.join("story/demo.md")).expect("readable");
    assert!(text.contains("id: story:demo"), "{text}");
    assert!(text.contains("kind: story"), "{text}");
    assert!(text.contains("status: draft"), "{text}");
    assert!(text.contains("revision: 2"), "{text}");
    assert!(
        text.ends_with("# Deliberate body\n\nExact bytes.\n"),
        "{text}"
    );

    std::fs::remove_file(body).ok();
}

#[test]
fn replacing_a_body_with_identical_bytes_does_not_invent_a_revision() {
    let store = scratch("aep-planning-body-identical");
    assert_eq!(
        code(&protocol(&[
            "artifact",
            "new",
            "story",
            "demo",
            "--title",
            "Demo",
            "--store",
            printable(&store),
        ])),
        0
    );
    let document = store.join("story/demo.md");
    let text = std::fs::read_to_string(&document).expect("readable");
    let body = text
        .split_once("\n---\n")
        .map(|(_, body)| body)
        .expect("the document has a closing fence");
    let source = store
        .parent()
        .expect("scratch has a parent")
        .join("aep-planning-same-body.md");
    write(&source, body);

    let replaced = protocol(&[
        "artifact",
        "body",
        "story:demo",
        "--from",
        printable(&source),
        "--store",
        printable(&store),
    ]);
    assert_eq!(code(&replaced), 0, "{}", stderr(&replaced));
    assert!(
        stdout(&replaced).contains("nothing to do"),
        "{}",
        stdout(&replaced)
    );
    let after = std::fs::read_to_string(document).expect("readable");
    assert_eq!(after, text, "an identical body changed the document");

    std::fs::remove_file(source).ok();
}

#[test]
fn an_illegal_move_exits_one_and_names_every_legal_target() {
    // The refusal has to answer the question it creates. A reader told only "no" goes and opens
    // `artifacts/lifecycles/story.yaml`; a reader told what is legal types the next command.
    let store = scratch("aep-planning-move-illegal");
    assert_eq!(
        code(&protocol(&[
            "artifact",
            "new",
            "story",
            "demo",
            "--title",
            "Demo",
            "--store",
            printable(&store),
        ])),
        0
    );

    let refused = protocol(&[
        "artifact",
        "move",
        "story:demo",
        "--to",
        "implemented",
        "--store",
        printable(&store),
    ]);
    assert_eq!(
        code(&refused),
        1,
        "an illegal move is a refusal, not a success"
    );

    let said = format!("{}{}", stdout(&refused), stderr(&refused));
    assert!(said.contains("story:demo is draft"), "{said}");
    for legal in ["proposed", "archived"] {
        assert!(
            said.contains(legal),
            "the refusal does not name `{legal}`, which is a legal move: {said}"
        );
    }

    let text = std::fs::read_to_string(store.join("story/demo.md")).expect("readable");
    assert!(
        text.contains("status: draft"),
        "a refused move changed the file: {text}"
    );
    assert!(
        text.contains("revision: 1"),
        "a refused move bumped the revision: {text}"
    );
}

#[test]
fn an_edge_to_an_artifact_the_store_does_not_hold_is_refused() {
    let store = scratch("aep-planning-dangling");
    assert_eq!(
        code(&protocol(&[
            "artifact",
            "new",
            "story",
            "demo",
            "--title",
            "Demo",
            "--store",
            printable(&store),
        ])),
        0
    );

    let refused = protocol(&[
        "artifact",
        "relate",
        "story:demo",
        "decomposes",
        "epic:absent",
        "--store",
        printable(&store),
    ]);
    assert_eq!(code(&refused), 1);
    assert!(
        stderr(&refused).contains("epic:absent"),
        "{}",
        stderr(&refused)
    );

    let text = std::fs::read_to_string(store.join("story/demo.md")).expect("readable");
    assert!(
        !text.contains("decomposes"),
        "the refused edge was written anyway: {text}"
    );
}

#[test]
fn validate_lists_every_problem_in_a_broken_store() {
    // Three problems of three different classes, and an exact count: "some problems" would pass
    // with a validator that reported the first and stopped, which is the failure this whole
    // accumulate-everything shape exists to prevent.
    let store = scratch("aep-planning-broken");
    write(
        &store.join("story/good.md"),
        &story("story:good", "draft", ""),
    );
    // 1. a file that is not a planning document at all
    write(&store.join("story/loose.md"), "# Just markdown\n");
    // 2. a document filed under a directory that names another kind
    write(
        &store.join("epic/misfiled.md"),
        &story("story:misfiled", "draft", ""),
    );
    // 3. a status the story lifecycle does not have
    write(
        &store.join("story/odd.md"),
        &story("story:odd", "in_review", ""),
    );

    let output = protocol(&["artifact", "validate", "--store", printable(&store)]);
    assert_eq!(code(&output), 1, "a broken store is not valid");

    let text = stdout(&output);
    assert!(text.contains("3 problem(s):"), "{text}");
    for expected in ["loose.md", "misfiled.md", "in_review"] {
        assert!(
            text.contains(expected),
            "no problem mentions `{expected}`: {text}"
        );
    }
    assert_eq!(
        text.lines().filter(|line| line.starts_with("  - ")).count(),
        3,
        "{text}"
    );
}

#[test]
fn a_store_that_cannot_be_read_whole_is_never_written_to() {
    // The rule that makes the refusal above more than a report: two files claiming one id means
    // whichever one a mutation picked, the other would still be there afterwards saying something
    // different.
    let store = scratch("aep-planning-unclean-write");
    write(
        &store.join("story/demo.md"),
        &story("story:demo", "draft", ""),
    );
    write(
        &store.join("story/copy.md"),
        &story("story:demo", "draft", ""),
    );

    let refused = protocol(&[
        "artifact",
        "move",
        "story:demo",
        "--to",
        "proposed",
        "--store",
        printable(&store),
    ]);
    assert_eq!(code(&refused), 1);
    let said = stderr(&refused);
    assert!(said.contains("nothing was written"), "{said}");
    assert!(
        std::fs::read_to_string(store.join("story/demo.md"))
            .expect("readable")
            .contains("status: draft"),
        "the store was written to anyway"
    );
}

#[test]
fn the_fixture_store_validates_clean() {
    let output = protocol(&["artifact", "validate", "--store", FIXTURE]);
    assert_eq!(code(&output), 0, "{}", stdout(&output));
    let text = stdout(&output);
    assert!(
        text.contains(&format!("{FIXTURE_ARTIFACTS} artifact(s)")),
        "{text}"
    );
    assert!(text.contains("valid"), "{text}");
}

#[test]
fn listing_the_fixture_as_json_is_byte_identical_across_two_runs() {
    // Invariant 9 at the command line. Nothing here reads a clock or a hash map, so two runs over
    // one store have to produce one document — otherwise every `--format json` diff is noise and
    // nobody can commit the output of this verb.
    let once = protocol(&["artifact", "list", "--store", FIXTURE, "--format", "json"]);
    let twice = protocol(&["artifact", "list", "--store", FIXTURE, "--format", "json"]);
    assert_eq!(code(&once), 0, "{}", stderr(&once));
    assert_eq!(once.stdout, twice.stdout, "two runs, two documents");

    let text = stdout(&once);
    assert!(text.contains("\"story:passkey-login\""), "{text}");
    assert_eq!(
        text.matches("\"kind\":").count(),
        FIXTURE_ARTIFACTS,
        "{text}"
    );
}

#[test]
fn listing_narrows_by_kind_and_by_status() {
    let by_kind = protocol(&["artifact", "list", "--store", FIXTURE, "--kind", "task"]);
    assert_eq!(code(&by_kind), 0, "{}", stderr(&by_kind));
    assert_eq!(stdout(&by_kind).lines().count(), 2, "{}", stdout(&by_kind));

    let by_status = protocol(&[
        "artifact", "list", "--store", FIXTURE, "--status", "proposed",
    ]);
    assert_eq!(code(&by_status), 0);
    let text = stdout(&by_status);
    assert_eq!(text.lines().count(), 1, "{text}");
    assert!(text.contains("story:passkey-recovery"), "{text}");
}

#[test]
fn the_board_groups_the_fixture_into_status_columns() {
    let output = protocol(&["artifact", "board", "--store", FIXTURE]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let text = stdout(&output);
    for column in ["proposed (1)", "active (4)", "implemented (2)"] {
        assert!(text.contains(column), "no `{column}` column: {text}");
    }
}

/// The passkeys fixture plus two blockers: a column exists for a rung only a lifecycle document
/// names, in the order that document puts its rungs in.
///
/// `board` used to build its columns from `ArtifactStatus::ALL`, the list compiled into the
/// binary, so a `blocker` at `open` — a rung `artifacts/lifecycles/blocker.yaml` declares and this
/// crate does not name — appeared in no column at all. On a copy of the fixture rather than in it:
/// fifteen sites assert that store's counts verbatim.
#[test]
fn the_board_has_a_column_for_a_rung_only_a_lifecycle_document_names() {
    let store = scratch("aep-planning-board-columns");
    let repository = root();
    copy_tree(&repository.join(FIXTURE), &store);
    let at = printable(&store);
    let tree = printable(&repository);

    // Two blockers so both rungs of the blocker ladder are held at once, which is the only way the
    // order between them is a claim: alphabetically `cleared` comes first, and the ladder says
    // `open` does.
    let stuck = protocol(&[
        "artifact",
        "new",
        "credential-blocker",
        "api-token-scope",
        "--title",
        "CI cannot mint a read-scope API token",
        "--withholds",
        "test_result",
        "--relate",
        "blocks:story:passkey-login",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&stuck), 0, "{}", stderr(&stuck));

    let lifted = protocol(&[
        "artifact",
        "new",
        "decision-blocker",
        "recovery-owner",
        "--title",
        "Nobody owned account recovery",
        "--relate",
        "blocks:story:passkey-recovery",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&lifted), 0, "{}", stderr(&lifted));
    let cleared = protocol(&[
        "artifact",
        "move",
        "decision-blocker:recovery-owner",
        "--to",
        "cleared",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&cleared), 0, "{}", stderr(&cleared));

    let board = protocol(&["artifact", "board", "--store", at, "--root", tree]);
    assert_eq!(code(&board), 0, "{}", stderr(&board));
    let text = stdout(&board);

    // 1. The adopter's rung is a column, and the blocker is in it.
    assert!(text.contains("open (1)"), "no `open` column: {text}");
    let card = text
        .lines()
        .skip_while(|line| !line.starts_with("open ("))
        .nth(1)
        .unwrap_or_default();
    assert!(
        card.contains("credential-blocker:api-token-scope"),
        "the blocker is not in its own column: {text}"
    );

    // 2. The rungs the compiled vocabulary knows are still columns, with the same counts.
    for column in ["proposed (1)", "active (4)", "implemented (2)"] {
        assert!(text.contains(column), "no `{column}` column: {text}");
    }

    // 3. Ladder order, not the alphabet: `open` before `cleared`.
    let open_at = text.find("open (1)").expect("an `open` column");
    let cleared_at = text
        .find("cleared (1)")
        .unwrap_or_else(|| panic!("no `cleared` column: {text}"));
    assert!(
        open_at < cleared_at,
        "the blocker ladder runs open -> cleared: {text}"
    );

    // 4. A rung every story ladder declares and nothing in the store holds is still no column.
    assert!(
        !text.contains("draft ("),
        "an empty column is noise, not information: {text}"
    );

    // 5. `--kind` narrows the columns to that kind's ladder.
    let narrowed = protocol(&[
        "artifact", "board", "--kind", "blocker", "--store", at, "--root", tree,
    ]);
    assert_eq!(code(&narrowed), 0, "{}", stderr(&narrowed));
    let narrowed = stdout(&narrowed);
    assert!(narrowed.contains("open (1)"), "{narrowed}");
    assert!(narrowed.contains("cleared (1)"), "{narrowed}");
    for absent in ["proposed (", "active (", "implemented ("] {
        assert!(
            !narrowed.contains(absent),
            "`{absent}` is on no blocker ladder: {narrowed}"
        );
    }

    let validated = protocol(&["artifact", "validate", "--store", at, "--root", tree]);
    assert_eq!(code(&validated), 0, "{}", stdout(&validated));
}

/// Every artifact the board was given lands in a column, whatever the tree says.
///
/// `board` is `list` regrouped, not `list` filtered: a card the board cannot place is the defect
/// `story:board-columns-come-from-the-ladders` exists against — "today it appears in no column at
/// all". Reading the ladders is best-effort by design (`ladders_or_none`: "a document tree that
/// cannot be read is not a reason a *listing* should stop answering"), so a root with no
/// `artifacts/` tree is a supported way to run this verb, and it is the one where a
/// `credential-blocker` at `open` still has nowhere to go.
#[test]
fn every_artifact_the_board_lists_lands_in_a_column() {
    let store = scratch("aep-planning-board-totality");
    let bare = scratch("aep-planning-board-bare-root");
    let repository = root();
    let at = printable(&store);
    let tree = printable(&repository);
    let bare = printable(&bare);

    // Created against the repository's own ladders, so `open` is a rung a document declared and
    // not a typo: `artifacts/lifecycles/blocker.yaml` starts a blocker there.
    let stuck = protocol(&[
        "artifact",
        "new",
        "credential-blocker",
        "api-token-scope",
        "--title",
        "CI cannot mint a read-scope API token",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&stuck), 0, "{}", stderr(&stuck));
    let ordinary = protocol(&[
        "artifact", "new", "story", "alpha", "--title", "Alpha", "--store", at, "--root", tree,
    ]);
    assert_eq!(code(&ordinary), 0, "{}", stderr(&ordinary));

    // Read back through a root that declares no ladders at all. `list` still prints both rows.
    let listed = protocol(&["artifact", "list", "--store", at, "--root", bare]);
    assert_eq!(code(&listed), 0, "{}", stderr(&listed));
    let listed = stdout(&listed);
    assert!(
        listed.contains("credential-blocker:api-token-scope"),
        "the store still holds the blocker: {listed}"
    );
    assert_eq!(listed.lines().count(), 2, "{listed}");

    // So the board has to account for both of them.
    let board = protocol(&["artifact", "board", "--store", at, "--root", bare]);
    assert_eq!(code(&board), 0, "{}", stderr(&board));
    let board = stdout(&board);
    assert!(
        board.contains("open (1)"),
        "no column for the rung the blocker is on: {board}"
    );
    assert!(
        board.contains("credential-blocker:api-token-scope"),
        "`list` prints the blocker and the board drops it without a word: {board}"
    );
}

/// A rung with nowhere to go is printed after the rungs that lead to it.
///
/// `artifacts/lifecycles/task.yaml` runs draft -> proposed -> active -> implemented, and
/// `archived` is where a task goes to stop. "In ladder order" cannot mean a board whose first
/// column is the one nothing leaves: `archived` is one step from `draft` and reachable from every
/// other rung, so shortest-distance-from-`initial` puts the end of the ladder third.
#[test]
fn the_board_prints_a_terminal_rung_after_the_rungs_that_lead_to_it() {
    let store = scratch("aep-planning-board-terminal-rung");
    let repository = root();
    let at = printable(&store);
    let tree = printable(&repository);

    // `task`, not `story`: a story's `implemented` rung requires a test_result, and this is a
    // question about column order, not about evidence.
    for (name, title) in [("alpha", "Alpha"), ("beta", "Beta"), ("gamma", "Gamma")] {
        let made = protocol(&[
            "artifact", "new", "task", name, "--title", title, "--store", at, "--root", tree,
        ]);
        assert_eq!(code(&made), 0, "{}", stderr(&made));
    }
    let route: &[(&str, &[&str])] = &[
        ("task:alpha", &["archived"]),
        ("task:beta", &["proposed", "active"]),
        ("task:gamma", &["proposed", "active", "implemented"]),
    ];
    for (id, rungs) in route {
        for rung in *rungs {
            let moved = protocol(&[
                "artifact", "move", id, "--to", rung, "--store", at, "--root", tree,
            ]);
            assert_eq!(code(&moved), 0, "{}", stderr(&moved));
        }
    }

    let board = protocol(&["artifact", "board", "--store", at, "--root", tree]);
    assert_eq!(code(&board), 0, "{}", stderr(&board));
    let text = stdout(&board);
    let column = |name: &str| {
        text.find(&format!("{name} (1)"))
            .unwrap_or_else(|| panic!("no `{name}` column: {text}"))
    };
    assert!(
        column("active") < column("archived"),
        "a task ladder runs active -> ... -> archived: {text}"
    );
    assert!(
        column("implemented") < column("archived"),
        "a task ladder runs implemented -> archived: {text}"
    );
}

/// One ladder's column order does not move because something else is in the store.
///
/// `artifacts/lifecycles/architecture-decision-record.yaml` says `proposed: [accepted, rejected]`,
/// and adding an unrelated `story:alpha` at `draft` changes nothing about that document. A board
/// whose ADR columns swap when a story is filed is not printing the ADR ladder's order; it is
/// printing an artefact of the order the ladders happened to be merged in.
#[test]
fn one_ladders_column_order_does_not_depend_on_another_kind_being_in_the_store() {
    let repository = root();
    let tree = printable(&repository);
    let alone = scratch("aep-planning-board-adrs-alone");
    let shared = scratch("aep-planning-board-adrs-and-a-story");

    for store in [&alone, &shared] {
        let at = printable(store);
        for (name, title, rung) in [
            ("yes-decision", "Adopt passkeys", "accepted"),
            ("no-decision", "Adopt SMS codes", "rejected"),
        ] {
            let made = protocol(&[
                "artifact",
                "new",
                "architecture-decision-record",
                name,
                "--title",
                title,
                "--store",
                at,
                "--root",
                tree,
            ]);
            assert_eq!(code(&made), 0, "{}", stderr(&made));
            let moved = protocol(&[
                "artifact",
                "move",
                &format!("architecture-decision-record:{name}"),
                "--to",
                rung,
                "--store",
                at,
                "--root",
                tree,
            ]);
            assert_eq!(code(&moved), 0, "{}", stderr(&moved));
        }
    }
    let filed = protocol(&[
        "artifact",
        "new",
        "story",
        "alpha",
        "--title",
        "Alpha",
        "--store",
        printable(&shared),
        "--root",
        tree,
    ]);
    assert_eq!(code(&filed), 0, "{}", stderr(&filed));

    let read = |store: &Path| {
        let output = protocol(&[
            "artifact",
            "board",
            "--store",
            printable(store),
            "--root",
            tree,
        ]);
        assert_eq!(code(&output), 0, "{}", stderr(&output));
        let text = stdout(&output);
        let at = |name: &str| {
            text.find(&format!("{name} (1)"))
                .unwrap_or_else(|| panic!("no `{name}` column: {text}"))
        };
        let swapped = at("rejected") < at("accepted");
        (swapped, text)
    };
    let (alone_swapped, alone_text) = read(&alone);
    let (shared_swapped, shared_text) = read(&shared);
    assert_eq!(
        alone_swapped, shared_swapped,
        "one story changed the ADR ladder's own column order:\n--- ADRs alone ---\n{alone_text}\n--- ADRs and a story ---\n{shared_text}"
    );
}

/// One malformed ladder document does not silently empty the board of everything it governs.
///
/// `board` reads its ladders through `ladders_or_none`, which answers "none" for a tree it cannot
/// parse — deliberately, so a listing keeps answering. But the columns are now read off those
/// ladders, so *one* unrelated document with a typo in it takes every adopter-declared rung off
/// the board and takes its cards with it: exit 0, no column, no warning, while `list` prints the
/// same artifacts and `validate` names the broken file. Losing a card is worse than losing an
/// order. Either outcome is defensible — refuse and name the file, or keep the card — and
/// pretending the store is smaller than it is, is not.
#[test]
fn a_malformed_ladder_document_does_not_silently_empty_the_board() {
    let store = scratch("aep-planning-board-broken-ladder-store");
    let tree = scratch("aep-planning-board-broken-ladder-root");
    let repository = root();
    let at = printable(&store);
    copy_tree(&repository.join("artifacts"), &tree.join("artifacts"));
    let tree = printable(&tree);

    let stuck = protocol(&[
        "artifact",
        "new",
        "credential-blocker",
        "api-token-scope",
        "--title",
        "CI cannot mint a read-scope API token",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&stuck), 0, "{}", stderr(&stuck));

    // The ladders are intact, so the rung the blocker document declared has a column.
    let before = protocol(&["artifact", "board", "--store", at, "--root", tree]);
    assert_eq!(code(&before), 0, "{}", stderr(&before));
    assert!(stdout(&before).contains("open (1)"), "{}", stdout(&before));

    // A second document, about a kind nothing in this store is, gains a typo.
    write(
        &Path::new(tree).join("artifacts/lifecycles/broken.yaml"),
        "kind: nonsense\ninitial: [not, a, status]\n",
    );

    let after = protocol(&["artifact", "board", "--store", at, "--root", tree]);
    let text = stdout(&after);
    let named = stderr(&after);
    assert!(
        code(&after) != 0 || text.contains("credential-blocker:api-token-scope"),
        "one broken ladder took the blocker off the board without saying so \
         (exit {}):\nstdout:\n{text}\nstderr:\n{named}",
        code(&after)
    );
}

/// A rung on no cycle at all is not printed before the rung that leads to it.
///
/// `ladder_order` states two invariants in its own doc comment: "a rung is printed only once every
/// rung that leads to it has been", and "the middle key only ever separates rungs in a cycle, where
/// no edge has an opinion left to give". Both are false for a ladder whose cycle does not contain
/// its `initial`.
///
/// `charter` runs `intake -> triage`, `triage -> {rework, approved}`, `rework -> triage`. The only
/// cycle is `{triage, rework}` and `intake` is not in it, so the cut key falls through to
/// precedence — over *every* stuck rung, not only over the rungs of the cycle. `approved` is stuck
/// solely because `triage` has not been printed, is on no cycle, and is rung 3 of the compiled
/// vocabulary while `triage` and `rework` are rungs the vocabulary does not name; so `approved`
/// wins the cut and is printed second, ahead of the only rung that reaches it.
///
/// It is also strictly worse as an ordering, not merely a different one: `intake, approved, rework,
/// triage` prints two of the ladder's four edges backwards (`triage -> approved` and
/// `triage -> rework`) where `intake, triage, rework, approved` prints one (`rework -> triage`,
/// which no order can avoid). A cut has to break a cycle; it does not have to break an edge outside
/// one.
#[test]
fn a_rung_on_no_cycle_is_not_printed_before_the_rung_that_leads_to_it() {
    let tree = scratch("aep-planning-board-cycle-without-initial-root");
    let store = scratch("aep-planning-board-cycle-without-initial-store");
    let at = printable(&store);
    write(
        &tree.join("artifacts/lifecycles/charter.yaml"),
        "kind: charter\n\
         initial: intake\n\
         transitions:\n  \
           intake: [triage]\n  \
           triage: [rework, approved]\n  \
           rework: [triage]\n  \
           approved: []\n",
    );
    let tree = printable(&tree);

    for name in ["one", "two", "three", "four"] {
        let made = protocol(&[
            "artifact", "new", "charter", name, "--title", "Charter", "--store", at, "--root", tree,
        ]);
        assert_eq!(code(&made), 0, "{}", stderr(&made));
    }
    let route: &[(&str, &[&str])] = &[
        ("charter:two", &["triage"]),
        ("charter:three", &["triage", "rework"]),
        ("charter:four", &["triage", "approved"]),
    ];
    for (id, rungs) in route {
        for rung in *rungs {
            let moved = protocol(&[
                "artifact", "move", id, "--to", rung, "--store", at, "--root", tree,
            ]);
            assert_eq!(code(&moved), 0, "{}", stderr(&moved));
        }
    }

    let board = protocol(&["artifact", "board", "--store", at, "--root", tree]);
    assert_eq!(code(&board), 0, "{}", stderr(&board));
    let text = stdout(&board);
    let column = |name: &str| {
        text.find(&format!("{name} (1)"))
            .unwrap_or_else(|| panic!("no `{name}` column: {text}"))
    };
    assert!(
        column("triage") < column("approved"),
        "`approved` is on no cycle and `triage -> approved` is the only way to reach it: {text}"
    );
}

/// A ladder's own column order does not move because a second kind was filed, when the second
/// kind's ladder names some of the same rungs.
///
/// This is the property `one_ladders_column_order_does_not_depend_on_another_kind_being_in_the_store`
/// asserts, and the one `ladder_order` claims in its own doc comment — "what keeps a ladder's own
/// columns where they were when something unrelated is filed ... whatever else the store holds".
/// That test pairs two ladders whose union happens to be acyclic, so it holds for an implementation
/// that has the property and for one that does not.
///
/// Both ladders here are acyclic *on their own*. `checklist` runs draft -> proposed -> active ->
/// archived; `escalation` runs active -> waiting -> proposed and stops. Their union has a cycle,
/// `proposed -> active -> waiting -> proposed`, which is cut at `active` because `active` is
/// `escalation`'s `initial` — and `checklist`'s `proposed` column, second when nothing else is in
/// the store, is then printed last, behind `active`, behind `archived`, and behind a rung of a kind
/// `checklist` has never heard of.
///
/// An adopter reading `protocol artifact lifecycle checklist` is told the ladder runs draft ->
/// proposed -> active -> archived. Filing one artifact of an unrelated kind should not make the
/// board disagree with that document.
///
/// **What actually holds, and it is weaker than the sentence above.** Two ladders that share rung
/// names and disagree on their order cannot both keep theirs; one has to bend. The tie-break is
/// **kind order**, decided by the operator on 2026-08-30 and documented on `board_order`, so the
/// ladder whose kind sorts first keeps its order and the other yields. This case holds because
/// `checklist` sorts before `escalation` — rename the second kind to `alert` and the same test
/// fails for `checklist` instead, which was measured rather than reasoned about.
///
/// So this asserts a **deterministic** tie-break, not a universal invariant. It is still worth
/// having: before the fix, the loser was whichever kind the union happened to visit first, and
/// nothing said which.
// Two ladders, two stores and one comparison: the property is that the *same* checklists print the
// same way with and without the escalation, so both stores have to be built here.
#[allow(clippy::too_many_lines)]
#[test]
fn a_ladders_column_order_survives_a_second_kind_that_shares_its_rung_names() {
    let tree = scratch("aep-planning-board-shared-rungs-root");
    write(
        &tree.join("artifacts/lifecycles/checklist.yaml"),
        "kind: checklist\n\
         initial: draft\n\
         transitions:\n  \
           draft: [proposed]\n  \
           proposed: [active]\n  \
           active: [archived]\n  \
           archived: []\n",
    );
    write(
        &tree.join("artifacts/lifecycles/escalation.yaml"),
        "kind: escalation\n\
         initial: active\n\
         transitions:\n  \
           active: [waiting]\n  \
           waiting: [proposed]\n  \
           proposed: []\n",
    );
    let tree = printable(&tree);

    let alone = scratch("aep-planning-board-checklist-alone");
    let shared = scratch("aep-planning-board-checklist-and-escalation");
    for store in [&alone, &shared] {
        let at = printable(store);
        for name in ["one", "two", "three", "four"] {
            let made = protocol(&[
                "artifact",
                "new",
                "checklist",
                name,
                "--title",
                "Checklist",
                "--store",
                at,
                "--root",
                tree,
            ]);
            assert_eq!(code(&made), 0, "{}", stderr(&made));
        }
        let route: &[(&str, &[&str])] = &[
            ("checklist:two", &["proposed"]),
            ("checklist:three", &["proposed", "active"]),
            ("checklist:four", &["proposed", "active", "archived"]),
        ];
        for (id, rungs) in route {
            for rung in *rungs {
                let moved = protocol(&[
                    "artifact", "move", id, "--to", rung, "--store", at, "--root", tree,
                ]);
                assert_eq!(code(&moved), 0, "{}", stderr(&moved));
            }
        }
    }
    // One artifact of the second kind, on a rung no `checklist` can ever be on.
    let filed = protocol(&[
        "artifact",
        "new",
        "escalation",
        "five",
        "--title",
        "Escalation",
        "--store",
        printable(&shared),
        "--root",
        tree,
    ]);
    assert_eq!(code(&filed), 0, "{}", stderr(&filed));
    let raised = protocol(&[
        "artifact",
        "move",
        "escalation:five",
        "--to",
        "waiting",
        "--store",
        printable(&shared),
        "--root",
        tree,
    ]);
    assert_eq!(code(&raised), 0, "{}", stderr(&raised));

    // The column headings, in the order they were printed, keeping only the rungs the `checklist`
    // ladder names: what the second kind adds is not the question, where it puts them is.
    let checklist_columns = |store: &Path| {
        let output = protocol(&[
            "artifact",
            "board",
            "--store",
            printable(store),
            "--root",
            tree,
        ]);
        assert_eq!(code(&output), 0, "{}", stderr(&output));
        let text = stdout(&output);
        let rungs: Vec<String> = text
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with(' '))
            .filter_map(|line| line.split(" (").next().map(str::to_owned))
            .filter(|rung| ["draft", "proposed", "active", "archived"].contains(&rung.as_str()))
            .collect();
        (rungs, text)
    };
    let (alone_rungs, alone_text) = checklist_columns(&alone);
    let (shared_rungs, shared_text) = checklist_columns(&shared);
    assert_eq!(
        alone_rungs, shared_rungs,
        "one artifact of a later-sorting kind reordered the `checklist` ladder's own columns \
         (the tie-break is kind order: `checklist` sorts before `escalation`, so it keeps its \
         order and `escalation` yields):\n\
         --- checklists alone ---\n{alone_text}\n--- checklists and one escalation ---\n{shared_text}"
    );
}

/// The compiled vocabulary still orders two rungs it knows when the kind holding them declares no
/// ladder.
///
/// The acceptance defines the board's columns as the union of the ladders "the store's kinds
/// declare (`protocol artifact lifecycle <kind>` for every kind present), in ladder order, and the
/// compiled list is used for nothing but the default ordering of the statuses it knows". For a kind
/// with no document that verb answers with a ladder rather than with nothing:
///
/// ```console
/// $ protocol artifact lifecycle mystery
/// mystery declares no lifecycle, so every status and every move is permitted
///   accepted -> draft, proposed, in_review, approved, accepted, rejected, active, ...
/// ```
///
/// `board` does not read it. So a `mystery` at `draft` — rung 0 of the compiled vocabulary — is
/// appended after a `review-result` at `active`, rung 6, although the only ladder this store can
/// read is `artifacts/lifecycles/review-result.yaml`, which says `active -> archived` and has no
/// edge to `draft` or `proposed` at all. Nothing in the store has an opinion about the pair except
/// the compiled list, and the compiled list is the thing the acceptance says decides it.
#[test]
fn the_compiled_order_still_separates_two_known_rungs_when_a_kind_declares_no_ladder() {
    let store = scratch("aep-planning-board-undeclared-kind-order");
    let repository = root();
    let at = printable(&store);
    let tree = printable(&repository);

    // The one built-in ladder that starts at `active`, so it is the whole of what the board reads.
    let recorded = protocol(&[
        "artifact",
        "new",
        "review-result",
        "one",
        "--title",
        "Alpha",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&recorded), 0, "{}", stderr(&recorded));
    for name in ["two", "three"] {
        let made = protocol(&[
            "artifact", "new", "mystery", name, "--title", "Beta", "--store", at, "--root", tree,
        ]);
        assert_eq!(code(&made), 0, "{}", stderr(&made));
    }
    let moved = protocol(&[
        "artifact",
        "move",
        "mystery:three",
        "--to",
        "proposed",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&moved), 0, "{}", stderr(&moved));

    let board = protocol(&["artifact", "board", "--store", at, "--root", tree]);
    assert_eq!(code(&board), 0, "{}", stderr(&board));
    let text = stdout(&board);
    let column = |name: &str| {
        text.find(&format!("{name} (1)"))
            .unwrap_or_else(|| panic!("no `{name}` column: {text}"))
    };
    assert!(
        column("draft") < column("active"),
        "`draft` is rung 0 of the compiled vocabulary, `active` is rung 6, and no ladder this \
         store can read relates them: {text}"
    );
    assert!(
        column("proposed") < column("active"),
        "`proposed` is rung 1 of the compiled vocabulary, `active` is rung 6, and no ladder this \
         store can read relates them: {text}"
    );
}

#[test]
fn the_graph_draws_every_artifact_and_every_edge() {
    let output = protocol(&["artifact", "graph", "--store", FIXTURE]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.starts_with("digraph planning {"), "{text}");
    assert_eq!(
        text.matches(" -> ").count(),
        8,
        "the fixture declares eight edges: {text}"
    );
    assert!(
        text.contains("\"epic:passkey-sign-in\" -> \"initiative:passwordless-authentication\""),
        "{text}"
    );
}

#[test]
fn the_entity_surface_counts_the_fixtures_artifacts() {
    // The same seeder the manifest goes through, fed from the store instead. What the entity
    // surface answers must not depend on which of the two sources it came from.
    let output = protocol(&["entity", "list", "--planning", FIXTURE]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let text = stdout(&output);
    assert_eq!(text.lines().count(), FIXTURE_ARTIFACTS, "{text}");
    assert!(text.contains("aep.initiative/v1"), "{text}");
    assert!(
        text.contains("ep://local/manifest/story/passkey-login"),
        "{text}"
    );
}

#[test]
fn the_entity_surface_refuses_both_sources_and_neither() {
    let neither = protocol(&["entity", "list"]);
    assert_eq!(code(&neither), 2, "a missing source is a usage error");
    assert!(
        stderr(&neither).contains("--artifacts"),
        "{}",
        stderr(&neither)
    );
    assert!(
        stderr(&neither).contains("--planning"),
        "{}",
        stderr(&neither)
    );

    let both = protocol(&[
        "entity",
        "list",
        "--artifacts",
        "examples/development-passkeys/artifacts.yaml",
        "--planning",
        FIXTURE,
    ]);
    assert_eq!(code(&both), 2, "two sources cannot be merged");
}

#[test]
fn the_store_defaults_to_the_planning_directory_of_the_project_it_is_run_in() {
    // The first command an adopting team types should not need a path.
    let project = root().join("examples/planning-passkeys");
    let output = protocol_in(&project, &["artifact", "list"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert_eq!(
        stdout(&output).lines().count(),
        FIXTURE_ARTIFACTS,
        "{}",
        stdout(&output)
    );
}

#[test]
fn validate_answers_the_same_from_a_subdirectory_as_it_does_from_the_root() {
    // `story:own-engineering-store` promises `protocol artifact validate` run **anywhere inside**
    // the project, with no flag. It used to resolve the workspace manifest against the working
    // directory rather than the project, so the same store validated at the root and reported every
    // cross-repository relation as undeclared one directory down — exit 0 and exit 1 for one store.
    let fixture = scratch("aep-validate-from-a-subdirectory");
    let project = fixture.join("project");
    let nested = project.join("crates/deep/inside");
    std::fs::create_dir_all(&nested).expect("the nested directory is writable");

    write(
        &project.join(".engineering/project.yaml"),
        "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\n\
         protocols: ../../tree\n",
    );
    // The manifest that declares the other repository. Found from the root before this fix, and
    // from nowhere else.
    write(
        &project.join(".engineering/workspace.yaml"),
        "version: aep.workspace/1\nmembers:\n  - name: other\n    source: ../other\n",
    );
    write(
        &fixture.join("tree/artifacts/lifecycles/story.yaml"),
        "kind: story\ninitial: draft\ntransitions:\n  draft: [active]\n  active: []\n",
    );
    write(
        &project.join(".engineering/planning/story/crossing.md"),
        "---\nformat: aep.planning-md/1\nid: story:crossing\nkind: story\nstatus: draft\n\
         title: A story that names another repository\nrelations:\n\
         - depends_on: other/story:theirs\nrevision: 1\n---\n# Story\n\nBody.\n",
    );

    let at_root = protocol_in(&project, &["artifact", "validate"]);
    let from_below = protocol_in(&nested, &["artifact", "validate"]);
    assert_eq!(
        code(&at_root),
        0,
        "the manifest declares the member: {}{}",
        stdout(&at_root),
        stderr(&at_root)
    );
    assert_eq!(
        code(&from_below),
        code(&at_root),
        "one store, one answer, whatever directory the person is standing in:\nroot: {}\nbelow: {}",
        stdout(&at_root),
        stdout(&from_below)
    );
    assert!(
        !stdout(&from_below).contains("undeclared_reference"),
        "the workspace is resolved against the project, not the working directory: {}",
        stdout(&from_below)
    );
}

#[test]
fn planning_documents_follow_the_protocol_tree_named_by_the_project() {
    // The store and its governing documents are one project configuration. Before this regression,
    // store discovery honored `project.yaml` while lifecycle and template discovery silently used
    // the working directory instead.
    let fixture = scratch("aep-planning-configured-tree");
    let project = fixture.join("project");
    let nested = project.join("crates/example");
    let configured = fixture.join("tree");
    let explicit = fixture.join("explicit-tree");
    std::fs::create_dir_all(&nested).expect("the nested project directory is writable");

    write(
        &project.join(".engineering/project.yaml"),
        "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\n\
         protocols: ../../tree\n",
    );
    // Resolving one configured path must not couple planning to unrelated project inputs.
    write(
        &project.join(".engineering/artifacts.yaml"),
        "not: [a manifest\n",
    );
    write(&project.join(".engineering/task.yaml"), "not: [a task\n");
    write(
        &configured.join("artifacts/lifecycles/story.yaml"),
        "kind: story\ninitial: proposed\ntransitions:\n  proposed: [active]\n  active: []\n",
    );
    write(
        &configured.join("artifacts/templates/story.md"),
        "# From the configured project tree\n",
    );

    let lifecycle = protocol_in(&nested, &["artifact", "lifecycle", "story"]);
    assert_eq!(code(&lifecycle), 0, "{}", stderr(&lifecycle));
    assert!(
        stdout(&lifecycle).contains("story starts at proposed"),
        "{}",
        stdout(&lifecycle)
    );

    let created = protocol_in(
        &nested,
        &[
            "artifact",
            "new",
            "story",
            "configured",
            "--title",
            "Configured",
        ],
    );
    assert_eq!(code(&created), 0, "{}", stderr(&created));
    assert!(
        stdout(&created).contains("(proposed)"),
        "{}",
        stdout(&created)
    );
    let document =
        std::fs::read_to_string(project.join(".engineering/planning/story/configured.md"))
            .expect("the story was created in the discovered store");
    assert!(
        document.contains("# From the configured project tree"),
        "{document}"
    );

    // An explicit command-line root remains authoritative.
    write(
        &explicit.join("artifacts/lifecycles/story.yaml"),
        "kind: story\ninitial: draft\ntransitions:\n  draft: [archived]\n  archived: []\n",
    );
    let explicit_lifecycle = protocol_in(
        &nested,
        &[
            "artifact",
            "lifecycle",
            "--root",
            printable(&explicit),
            "story",
        ],
    );
    assert_eq!(
        code(&explicit_lifecycle),
        0,
        "{}",
        stderr(&explicit_lifecycle)
    );
    assert!(
        stdout(&explicit_lifecycle).contains("story starts at draft"),
        "{}",
        stdout(&explicit_lifecycle)
    );
}

#[test]
fn a_pinned_git_protocol_source_is_materialized_once_and_then_read_from_cache() {
    let fixture = scratch("aep-planning-git-source");
    let remote = fixture.join("remote");
    let project = fixture.join("project");
    let cache = fixture.join("cache");
    std::fs::create_dir_all(&remote).expect("the source repository is writable");
    std::fs::create_dir_all(&project).expect("the project is writable");

    git(&remote, &["init", "--quiet"]);
    git(&remote, &["config", "user.name", "Protocol Test"]);
    git(
        &remote,
        &["config", "user.email", "protocol-test@example.invalid"],
    );
    write(
        &remote.join("artifacts/lifecycles/story.yaml"),
        "kind: story\ninitial: proposed\ntransitions:\n  proposed: [active]\n  active: []\n",
    );
    write(
        &remote.join("artifacts/templates/story.md"),
        "# From the pinned Git source\n",
    );
    git(&remote, &["add", "."]);
    git(&remote, &["commit", "--quiet", "-m", "protocol tree"]);
    let revision = git(&remote, &["rev-parse", "HEAD"]);
    let source = format!("git+file://{}#{revision}", remote.display());
    write(
        &project.join(".engineering/project.yaml"),
        &format!(
            "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\n\
             protocols: '{source}'\n"
        ),
    );

    let lifecycle = protocol_in_with_cache(&project, &cache, &["artifact", "lifecycle", "story"]);
    assert_eq!(code(&lifecycle), 0, "{}", stderr(&lifecycle));
    assert!(
        stdout(&lifecycle).contains("story starts at proposed"),
        "{}",
        stdout(&lifecycle)
    );

    // The second command must need neither the repository nor the network: the immutable revision
    // was materialized by the first command and is now an ordinary document tree in the cache.
    std::fs::remove_dir_all(&remote).expect("the source fixture can be removed");
    let created = protocol_in_with_cache(
        &project,
        &cache,
        &["artifact", "new", "story", "cached", "--title", "Cached"],
    );
    assert_eq!(code(&created), 0, "{}", stderr(&created));
    assert!(
        stdout(&created).contains("(proposed)"),
        "{}",
        stdout(&created)
    );
    let document = std::fs::read_to_string(project.join(".engineering/planning/story/cached.md"))
        .expect("the cached template seeded the story");
    assert!(
        document.contains("# From the pinned Git source"),
        "{document}"
    );
}

#[test]
fn outside_a_project_the_missing_store_says_what_to_pass() {
    let elsewhere = scratch("aep-planning-not-a-project");
    let output = protocol_in(&elsewhere, &["artifact", "list"]);
    assert_eq!(code(&output), 1);
    let said = stderr(&output);
    assert!(said.contains("--store"), "{said}");
    assert!(said.contains("project.yaml"), "{said}");
}

#[test]
fn the_vocabulary_verbs_answer_without_a_store() {
    // `kinds` and `relations` are questions about the vocabulary. Refusing them because the working
    // directory is not a project would be refusing for a reason unrelated to the question.
    let elsewhere = scratch("aep-planning-vocabulary");

    let kinds = protocol_in(&elsewhere, &["artifact", "kinds"]);
    assert_eq!(code(&kinds), 0, "{}", stderr(&kinds));
    let text = stdout(&kinds);
    assert!(text.contains("story"), "{text}");
    // Six compiled kinds are intent decomposition. The seventh `planning` row is the open blocker
    // family, which is not a kind and says so — and it is here rather than absent because *what can
    // I create* is the question this verb answers, and the answer does not depend on a store.
    let planning: Vec<&str> = text
        .lines()
        .filter(|line| line.contains("planning"))
        .collect();
    assert_eq!(planning.len(), 7, "{text}");
    assert!(
        planning
            .last()
            .expect("a row")
            .starts_with("<type>-blocker "),
        "{text}"
    );
    // No tree here, so nothing beyond the vocabulary and the family is listed.
    assert!(
        !text.contains("lifecycles declare it"),
        "a directory that is not a project declares no lifecycles: {text}"
    );

    let relations = protocol_in(&elsewhere, &["artifact", "relations"]);
    assert_eq!(code(&relations), 0, "{}", stderr(&relations));
    assert_eq!(
        stdout(&relations).lines().count(),
        14,
        "{}",
        stdout(&relations)
    );
}

#[test]
fn a_lifecycle_is_printed_from_the_documents_the_tree_declares() {
    let output = protocol(&["artifact", "lifecycle", "story"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("story starts at draft"), "{text}");
    assert!(text.contains("draft -> proposed, archived"), "{text}");

    // A kind nobody wrote a ladder for says so, rather than printing an empty one.
    let permissive = protocol(&["artifact", "lifecycle", "runbook"]);
    assert_eq!(code(&permissive), 0);
    assert!(
        stdout(&permissive).contains("declares no lifecycle"),
        "{}",
        stdout(&permissive)
    );
}

#[test]
fn the_new_kinds_have_the_ladder_the_store_needs() {
    // The three lifecycles this store made necessary. Before them an epic could be moved anywhere,
    // because a kind with no ladder is permissive — which reads exactly like a ladder that permits
    // everything, and is why they had to be written rather than assumed.
    for kind in ["epic", "task", "initiative"] {
        let output = protocol(&["artifact", "lifecycle", kind]);
        assert_eq!(code(&output), 0, "{}", stderr(&output));
        let text = stdout(&output);
        assert!(
            text.contains(&format!("{kind} starts at draft")),
            "{kind} declares no lifecycle: {text}"
        );
        assert!(text.contains("implemented -> archived"), "{text}");
    }
}

/// A planning document, for a fixture that needs a broken or an ordinary one.
fn story(id: &str, status: &str, extra: &str) -> String {
    format!("---\nid: {id}\nkind: story\nstatus: {status}\n{extra}---\n# {id}\n")
}

/// Whether one line of source writes the store or its journal directly.
///
/// **The scan and its guard call this, and neither restates it.** The guard used to carry its own
/// copy, so weakening the real predicate left the guard green — the failure mode this scan exists
/// to refuse, one level up.
fn writes_behind_the_contract(line: &str) -> bool {
    let code = line.trim_start();
    if code.starts_with("//") || code.starts_with("///") || code.starts_with("let planted") {
        return false;
    }
    // The store, and **the journal**. An evidence record is the input to the evidence-gated move
    // decision, so a verb appending one directly is writing the thing the decision reads without
    // passing the door every other write passes.
    code.contains("store.update(")
        || code.contains("store.create(")
        || code.contains("journal::append(")
}

#[test]
fn no_planning_verb_writes_to_the_store_except_through_a_command() {
    // **D-P1, closed and pinned.** The store used to be written by the verbs directly, through its
    // own `create`/`update`, which is a second write path — a second place for idempotency,
    // revision checks and the audit record to be forgotten, and what invariant 14 exists to forbid.
    //
    // Every verb now issues a command and `MarkdownBackend` writes the file. A verb added next year
    // that called the store directly would compile, pass every test, and quietly reopen it.
    let source = include_str!("../src/planning.rs");
    let production = source.split("#[cfg(test)]").next().unwrap_or(source);
    let offenders: Vec<String> = production
        .lines()
        .enumerate()
        .filter(|(_, line)| writes_behind_the_contract(line))
        .map(|(number, line)| format!("{}: {}", number + 1, line.trim()))
        .collect();

    assert!(
        offenders.is_empty(),
        "a verb writing the store or its journal directly is the second write path D-P1 was — \
         model the change as a command and let `MarkdownBackend` carry it:\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn the_command_only_scan_sees_a_write_it_should_refuse() {
    // The guard, calling the real predicate rather than a copy of it. Delete a clause from
    // `writes_behind_the_contract` and this fails, which is the whole point.
    //
    // If this test ever starts passing while the scan above still does, that one has stopped being
    // evidence.
    assert!(writes_behind_the_contract(
        "    store.update(&relative, &document)?;"
    ));
    assert!(writes_behind_the_contract(
        "    let path = store.create(&document)?;"
    ));
    assert!(writes_behind_the_contract(
        "    journal::append(store.root(), &entry)?;"
    ));
    assert!(
        !writes_behind_the_contract("    /// Calls `store.update(…)` on its behalf."),
        "and must not fire on prose that names it"
    );
}

/// **Grounding** (atlas ADR 0005): once a store declares an objective, agreed work says which it
/// serves — and `serves` points at an objective and nothing else.
#[test]
fn validate_holds_agreed_work_to_an_objective_once_the_store_declares_one() {
    let store = scratch("aep-planning-serves");
    // No objective declared: an active story that serves nothing is not a problem.
    write(
        &store.join("story/agreed.md"),
        &story("story:agreed", "active", ""),
    );
    let output = protocol(&["artifact", "validate", "--store", printable(&store)]);
    assert_eq!(code(&output), 0, "{}", stdout(&output));

    // An objective appears. Now the active story must say which it serves; the draft need not.
    write(
        &store.join("vision/O1.md"),
        "---\nid: vision:O1\nkind: vision\nstatus: approved\ntitle: Governed reach\n---\n# O1\n",
    );
    write(
        &store.join("story/thought.md"),
        &story("story:thought", "draft", ""),
    );
    let output = protocol(&["artifact", "validate", "--store", printable(&store)]);
    let text = stdout(&output);
    assert_eq!(code(&output), 1, "{text}");
    assert!(text.contains("1 problem(s):"), "{text}");
    assert!(
        text.contains("story:agreed is active and serves no objective"),
        "{text}"
    );
    assert!(
        !text.contains("story:thought"),
        "a draft is not charged:\n{text}"
    );

    // Said which: valid again.
    write(
        &store.join("story/agreed.md"),
        &story(
            "story:agreed",
            "active",
            "relations:\n- serves: vision:O1\n",
        ),
    );
    let output = protocol(&["artifact", "validate", "--store", printable(&store)]);
    assert_eq!(code(&output), 0, "{}", stdout(&output));

    // `serves` into anything but a vision is refused by name.
    write(
        &store.join("story/thought.md"),
        &story(
            "story:thought",
            "draft",
            "relations:\n- serves: story:agreed\n",
        ),
    );
    let output = protocol(&["artifact", "validate", "--store", printable(&store)]);
    let text = stdout(&output);
    assert_eq!(code(&output), 1, "{text}");
    assert!(
        text.contains(
            "story:thought says it serves story:agreed, which is a story and not a vision"
        ),
        "{text}"
    );
}

// One record through its whole life: created with a body, refused an edit, retired, refused the way
// back. Splitting it would lose that each refusal is about the state the previous step left.
#[allow(clippy::too_many_lines)]
#[test]
fn a_review_result_is_authored_whole_retired_by_its_ladder_and_edited_never() {
    // The second adopter's finding (`story:review-result-cannot-be-authored`): `new` took no body,
    // `body` was refused as immutable, and `move --to archived` — the one transition the kind's
    // lifecycle declares — was refused by the same guard. Three refusals, no legal path, and the
    // review ended up in a `docs/reviews/` directory the kind exists to replace.
    let store = scratch("aep-planning-review-result");
    // Beside the store, not in it: a loose markdown file inside the tree is a document that cannot
    // be read, and `new` refuses to write into a store it cannot read whole.
    let drafts = scratch("aep-planning-review-result-drafts");
    let review = drafts.join("review.md");
    write(
        &review,
        "# Backlog against the objectives\n\nEvery epic names an objective; two objectives have no \
         epic.\n",
    );
    let revised = drafts.join("revised.md");
    write(
        &revised,
        "# Backlog against the objectives\n\nOn reflection, all fine.\n",
    );

    // A review says what it reviews, or `validate` refuses it as an empty declaration.
    let subject = protocol(&[
        "artifact",
        "new",
        "epic",
        "objectives",
        "--title",
        "Objectives",
        "--store",
        printable(&store),
    ]);
    assert_eq!(code(&subject), 0, "{}", stderr(&subject));

    let created = protocol(&[
        "artifact",
        "new",
        "review-result",
        "backlog-vs-objectives",
        "--title",
        "Backlog against the objectives",
        "--relate",
        "reviews:epic:objectives",
        "--from",
        printable(&review),
        "--store",
        printable(&store),
    ]);
    assert_eq!(code(&created), 0, "{}", stderr(&created));
    assert!(
        stdout(&created).contains("(active)"),
        "a review is recorded, never drafted: {}",
        stdout(&created)
    );
    let written = store.join("review-result/backlog-vs-objectives.md");
    let text = std::fs::read_to_string(&written).expect("readable");
    assert!(
        text.contains("two objectives have no epic"),
        "the body did not arrive with the record: {text}"
    );

    // Immutable, still: the body that arrived is the body it keeps.
    let edited = protocol(&[
        "artifact",
        "body",
        "review-result:backlog-vs-objectives",
        "--from",
        printable(&revised),
        "--store",
        printable(&store),
    ]);
    assert_ne!(code(&edited), 0, "a review was edited after the fact");
    let said = format!("{}{}", stdout(&edited), stderr(&edited));
    assert!(said.contains("immutable"), "the refusal says why: {said}");
    let text = std::fs::read_to_string(&written).expect("readable");
    assert!(
        text.contains("two objectives have no epic") && !text.contains("all fine"),
        "a refused edit changed the file: {text}"
    );

    // The one transition its lifecycle declares, and the move the old guard closed.
    let retired = protocol(&[
        "artifact",
        "move",
        "review-result:backlog-vs-objectives",
        "--to",
        "archived",
        "--store",
        printable(&store),
    ]);
    assert_eq!(
        code(&retired),
        0,
        "{}{}",
        stdout(&retired),
        stderr(&retired)
    );
    assert!(
        stdout(&retired).contains("moved active -> archived"),
        "{}",
        stdout(&retired)
    );

    // And the ladder, not the guard, is what refuses the way back.
    let way_back = protocol(&[
        "artifact",
        "move",
        "review-result:backlog-vs-objectives",
        "--to",
        "active",
        "--store",
        printable(&store),
    ]);
    assert_eq!(code(&way_back), 1, "archived is terminal");
    let said = format!("{}{}", stdout(&way_back), stderr(&way_back));
    assert!(
        said.contains("review-result:backlog-vs-objectives is archived"),
        "{said}"
    );

    let validated = protocol(&["artifact", "validate", "--store", printable(&store)]);
    assert_eq!(code(&validated), 0, "{}", stdout(&validated));
}

#[test]
fn a_body_handed_to_new_on_standard_input_is_the_body_the_store_holds() {
    let store = scratch("aep-planning-new-stdin");
    let created = protocol_with_stdin(
        &[
            "artifact",
            "new",
            "story",
            "demo",
            "--title",
            "Demo",
            "--from",
            "-",
            "--store",
            printable(&store),
        ],
        "# Story: Demo\n\n## Outcome\n\nOne sentence, and it arrived with the record.\n",
    );
    assert_eq!(code(&created), 0, "{}", stderr(&created));

    let text = std::fs::read_to_string(store.join("story/demo.md")).expect("readable");
    assert!(
        text.contains("it arrived with the record"),
        "the body on standard input was not written: {text}"
    );
    assert!(
        !text.contains("Starting point for a `story` artifact"),
        "the template was written over the body that was handed in: {text}"
    );
    assert!(
        text.contains("revision: 1"),
        "one write, one revision: {text}"
    );

    let validated = protocol(&["artifact", "validate", "--store", printable(&store)]);
    assert_eq!(code(&validated), 0, "{}", stdout(&validated));
}

/// The whole of `story:blocker-relation`, in the order a person meets it: a blocker typed by what
/// would clear it, an item that reads as parked rather than as moving, one group per blocker, the
/// join to the evidence a gate is waiting for, and an unblocking that is a move with a record.
///
/// One test rather than five, because every step is about the state the previous one left: a
/// grouping is only interesting once two items share a blocker, and *unblocking leaves a record*
/// is only a claim once there is something to lift.
#[allow(clippy::too_many_lines)]
#[test]
fn a_blocker_is_typed_by_what_clears_it_and_says_so_in_every_listing() {
    let store = scratch("aep-planning-blocked");
    let at = printable(&store);
    let root = root();
    let root = printable(&root);

    for name in ["ci-evidence", "contract-checks", "unrelated"] {
        let made = protocol(&[
            "artifact", "new", "story", name, "--title", name, "--store", at, "--root", root,
        ]);
        assert_eq!(code(&made), 0, "{}", stderr(&made));
    }

    // The type is the kind, and nothing had to be released for it: `credential-blocker` reaches
    // the one `blocker` ladder by its last hyphen segment.
    let made = protocol(&[
        "artifact",
        "new",
        "credential-blocker",
        "api-token-scope",
        "--title",
        "CI cannot mint a read-scope API token",
        "--withholds",
        "test_result",
        "--relate",
        "blocks:story:ci-evidence",
        "--relate",
        "blocks:story:contract-checks",
        "--store",
        at,
        "--root",
        root,
    ]);
    assert_eq!(code(&made), 0, "{}", stderr(&made));
    assert!(
        stdout(&made).contains("credential-blocker:api-token-scope (open)"),
        "the blocker starts on the blocker ladder: {}",
        stdout(&made)
    );

    // 1. Distinguishable in `list` without opening the file — and by *type*, not by a bare flag.
    let listed = protocol(&["artifact", "list", "--store", at, "--root", root]);
    assert_eq!(code(&listed), 0, "{}", stderr(&listed));
    let text = stdout(&listed);
    let line = |id: &str| {
        text.lines()
            .find(|line| line.starts_with(id))
            .unwrap_or_else(|| panic!("no line for {id}: {text}"))
    };
    assert!(
        line("story:ci-evidence").contains("blocked: credential"),
        "{text}"
    );
    assert!(
        !line("story:unrelated").contains("blocked"),
        "an item nothing stops says nothing: {text}"
    );

    // The machine format carries the same fact, always written so `active` and `active but parked`
    // are two documents to a consumer as well as to a reader.
    let json = protocol(&[
        "artifact", "list", "--store", at, "--root", root, "--format", "json",
    ]);
    assert_eq!(code(&json), 0, "{}", stderr(&json));
    let json = stdout(&json);
    assert_eq!(json.matches("\"blocked_by\"").count(), 4, "{json}");
    assert!(json.contains("\"type\": \"credential\""), "{json}");

    // 2. The board marks the card, and leaves it in the column its status puts it in: a blocked
    // story is still `draft`, and a column of its own would be a status the ladder does not have.
    let board = protocol(&["artifact", "board", "--store", at, "--root", root]);
    assert_eq!(code(&board), 0, "{}", stderr(&board));
    let board = stdout(&board);
    assert!(board.contains("draft (3)"), "{board}");
    assert!(
        board.contains("story:ci-evidence  ci-evidence  [blocked: credential]"),
        "{board}"
    );

    // 3. One group per blocker: two stories on one credential are one conversation.
    let blocked = protocol(&["artifact", "blocked", "--store", at, "--root", root]);
    assert_eq!(code(&blocked), 0, "{}", stderr(&blocked));
    let blocked = stdout(&blocked);
    assert_eq!(
        blocked
            .lines()
            .filter(|line| line.starts_with("credential-blocker:api-token-scope"))
            .count(),
        1,
        "two blocked stories, one row: {blocked}"
    );
    assert_eq!(
        blocked
            .lines()
            .filter(|line| line.trim_start().starts_with("blocks "))
            .count(),
        2,
        "{blocked}"
    );
    assert!(blocked.contains("withholding test_result"), "{blocked}");

    // Narrowed by type, which is the whole reason the type exists.
    let other = protocol(&[
        "artifact", "blocked", "--type", "decision", "--store", at, "--root", root,
    ]);
    assert_eq!(code(&other), 0, "{}", stderr(&other));
    assert!(
        stdout(&other).contains("nothing is blocked"),
        "{}",
        stdout(&other)
    );

    // 4. `explain` names it, with the evidence kind nobody can produce. This is the join: the
    // question *why is there no record* is answered out of the store.
    let explained = protocol(&[
        "artifact",
        "explain",
        "story:ci-evidence",
        "--store",
        at,
        "--root",
        root,
    ]);
    assert_eq!(code(&explained), 0, "{}", stderr(&explained));
    let explained = stdout(&explained);
    assert!(
        explained.contains(
            "blocked by credential-blocker:api-token-scope (credential), withholding test_result"
        ),
        "{explained}"
    );

    // 5. Unblocking is a move, and the record survives it.
    let cleared = protocol(&[
        "artifact",
        "move",
        "credential-blocker:api-token-scope",
        "--to",
        "cleared",
        "--store",
        at,
        "--root",
        root,
    ]);
    assert_eq!(code(&cleared), 0, "{}", stderr(&cleared));

    let after = protocol(&["artifact", "blocked", "--store", at, "--root", root]);
    assert!(
        stdout(&after).contains("nothing is blocked"),
        "a ladder's last rung lifts the edge: {}",
        stdout(&after)
    );
    let listed = protocol(&["artifact", "list", "--store", at, "--root", root]);
    assert!(
        !stdout(&listed).contains("blocked: credential"),
        "{}",
        stdout(&listed)
    );

    // Not an edit that erases it: the journal still says it happened, and the rung is terminal, so
    // being stuck again is a new blocker with its own date rather than this one reopened.
    let history = protocol(&[
        "artifact",
        "history",
        "credential-blocker:api-token-scope",
        "--store",
        at,
        "--root",
        root,
    ]);
    let history = stdout(&history);
    assert!(history.contains("created as open"), "{history}");
    assert!(history.contains("moved open -> cleared"), "{history}");
    assert!(history.contains("blocks story:ci-evidence"), "{history}");

    let reopened = protocol(&[
        "artifact",
        "move",
        "credential-blocker:api-token-scope",
        "--to",
        "open",
        "--store",
        at,
        "--root",
        root,
    ]);
    assert_eq!(code(&reopened), 1, "{}", stdout(&reopened));
    assert!(
        stdout(&reopened).contains("at the end of its lifecycle"),
        "{}",
        stdout(&reopened)
    );
}

/// Evidence withheld from nothing is refused, and the refusal reaches the store's own validator
/// through the document — which is the part the domain's unit test cannot show.
#[test]
fn withheld_evidence_that_blocks_nothing_is_reported_by_validate() {
    let store = scratch("aep-planning-withholds");
    let at = printable(&store);
    let root = root();
    let root = printable(&root);

    let made = protocol(&[
        "artifact",
        "new",
        "credential-blocker",
        "orphan",
        "--title",
        "Withholds a fact nobody is waiting for",
        "--withholds",
        "test_result",
        "--store",
        at,
        "--root",
        root,
    ]);
    assert_eq!(code(&made), 0, "{}", stderr(&made));

    let validated = protocol(&["artifact", "validate", "--store", at, "--root", root]);
    let text = stdout(&validated);
    assert_eq!(code(&validated), 1, "{text}");
    assert!(text.contains("[missing_declaration]"), "{text}");
    assert!(
        text.contains("withholds test_result and blocks nothing"),
        "{text}"
    );

    // Joined to the work it is stopping, the same record validates.
    let related = protocol(&[
        "artifact",
        "new",
        "story",
        "ci-evidence",
        "--title",
        "Evidence job",
        "--store",
        at,
        "--root",
        root,
    ]);
    assert_eq!(code(&related), 0, "{}", stderr(&related));
    let related = protocol(&[
        "artifact",
        "relate",
        "credential-blocker:orphan",
        "blocks",
        "story:ci-evidence",
        "--store",
        at,
        "--root",
        root,
    ]);
    assert_eq!(code(&related), 0, "{}", stderr(&related));

    let validated = protocol(&["artifact", "validate", "--store", at, "--root", root]);
    assert_eq!(code(&validated), 0, "{}", stdout(&validated));

    // And an evidence kind the engine does not know is refused where it is written, rather than
    // carried through as text a reader would take for a fact something is tracking.
    let refused = protocol(&[
        "artifact",
        "new",
        "credential-blocker",
        "invented",
        "--title",
        "x",
        "--withholds",
        "green_build",
        "--store",
        at,
        "--root",
        root,
    ]);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("invalid evidence kind identifier"),
        "{}",
        stderr(&refused)
    );
}

/// The volatile fields of a journal line: when it was written, and the event id that carries a
/// clock in it. Everything else is what the write *was*, and is what two spellings of one edge
/// have to agree on.
fn without_the_clock(line: &str) -> String {
    let mut text = line.to_owned();
    for key in ["\"at\":", "\"recorded_at\":", "\"event_id\":"] {
        let mut out = String::with_capacity(text.len());
        let mut rest = text.as_str();
        while let Some(start) = rest.find(key) {
            out.push_str(&rest[..start]);
            let after = &rest[start + key.len()..];
            let end = after
                .find(',')
                .or_else(|| after.find('}'))
                .unwrap_or(after.len());
            rest = &after[end..];
        }
        out.push_str(rest);
        text = out;
    }
    text
}

/// The last line of a store's journal, which is the write the verb under test just made.
fn last_journal_line(store: &Path) -> String {
    let text = std::fs::read_to_string(store.join("journal.jsonl")).expect("a journal");
    text.lines()
        .last()
        .expect("the journal has an entry")
        .to_owned()
}

/// **One spelling for an edge.** `relate <id> <relation>:<target>` is `relate <id> <relation>
/// <target>`, down to the journal.
///
/// `cc946bc3#486`: `protocol artifact relate story:… serves:vision:O2` was refused for want of a
/// third positional, while `new --relate serves:vision:O2` had been taking those exact words all
/// along. Both stories were already `active`, so the store went red mid-run over a spelling.
#[test]
fn an_edge_written_as_one_word_is_the_edge_written_as_three() {
    let repository = root();
    let tree = printable(&repository);
    let one_word = scratch("aep-planning-relate-one-word");
    let three_words = scratch("aep-planning-relate-three-words");
    copy_tree(&repository.join(FIXTURE), &one_word);
    copy_tree(&repository.join(FIXTURE), &three_words);

    let joined = protocol(&[
        "artifact",
        "relate",
        "task:assertion-verification",
        "depends_on:task:webauthn-ceremony",
        "--store",
        printable(&one_word),
        "--root",
        tree,
    ]);
    assert_eq!(code(&joined), 0, "{}", stderr(&joined));

    let split = protocol(&[
        "artifact",
        "relate",
        "task:assertion-verification",
        "depends_on",
        "task:webauthn-ceremony",
        "--store",
        printable(&three_words),
        "--root",
        tree,
    ]);
    assert_eq!(code(&split), 0, "{}", stderr(&split));

    // 1. The same answer, naming the edge the same way whichever spelling asked for it.
    assert_eq!(stdout(&joined), stdout(&split));
    assert!(
        stdout(&joined).contains("task:assertion-verification depends_on task:webauthn-ceremony"),
        "{}",
        stdout(&joined)
    );

    // 2. The same document, byte for byte.
    let document = |store: &Path| {
        std::fs::read_to_string(store.join("task/assertion-verification.md")).expect("readable")
    };
    assert_eq!(document(&one_word), document(&three_words));
    assert!(
        document(&one_word).contains("depends_on: task:webauthn-ceremony"),
        "the edge is not in the document: {}",
        document(&one_word)
    );

    // 3. The same journal entry, once the two instants and the event id are taken out. This is
    //    what "journal identically" means: a reader three months later cannot tell which spelling
    //    was typed, because the store did not record a spelling — it recorded an edge.
    assert_eq!(
        without_the_clock(&last_journal_line(&one_word)),
        without_the_clock(&last_journal_line(&three_words))
    );
    assert!(
        last_journal_line(&one_word).contains(
            r#""change":{"change":"related","relation":"depends_on","target":"task:webauthn-ceremony"}"#
        ),
        "{}",
        last_journal_line(&one_word)
    );

    // 4. A relation naming no target at all is still refused, and says what to write.
    let bare = protocol(&[
        "artifact",
        "relate",
        "task:assertion-verification",
        "depends_on",
        "--store",
        printable(&three_words),
        "--root",
        tree,
    ]);
    assert_eq!(code(&bare), 1, "{}", stdout(&bare));
    assert!(
        stderr(&bare).contains("<relation>:<artifact-id>"),
        "{}",
        stderr(&bare)
    );
}

/// **Editing part of a body has a verb.** `--append` adds to the end, `--section` rewrites the
/// prose under one `##` heading, and both go through the same `update` the whole-body form does.
///
/// The five sessions of `SYNTHESIS.md` CL-2 all did this with `python` and `cat >>` because there
/// was nothing to type: `9da4f51c#3310` appended a section with a shell redirect, which wrote the
/// bytes and skipped the journal entirely.
#[test]
#[allow(clippy::too_many_lines)]
fn a_section_and_an_append_are_body_verbs_rather_than_a_heredoc() {
    let repository = root();
    let tree = printable(&repository);
    let store = scratch("aep-planning-body-parts");
    copy_tree(&repository.join(FIXTURE), &store);
    let at = printable(&store);
    let scratch_root = store.parent().expect("scratch has a parent").to_path_buf();

    let addition = scratch_root.join("aep-planning-body-append.md");
    write(
        &addition,
        "## Risks\n\nThe authenticator may lie about its sign count.\n",
    );
    let appended = protocol(&[
        "artifact",
        "body",
        "task:assertion-verification",
        "--from",
        printable(&addition),
        "--append",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&appended), 0, "{}", stderr(&appended));
    assert!(
        stdout(&appended).contains("body appended to"),
        "{}",
        stdout(&appended)
    );

    let document = store.join("task/assertion-verification.md");
    let text = std::fs::read_to_string(&document).expect("readable");
    // 1. Appended, not replaced: the sections that were there are still there.
    assert!(
        text.contains("## What"),
        "the append replaced the body: {text}"
    );
    assert!(
        text.contains("## Risks"),
        "the section was not appended: {text}"
    );
    assert!(
        text.trim_end()
            .ends_with("The authenticator may lie about its sign count."),
        "the appended prose is not at the end: {text}"
    );
    // 2. Journalled as an update, which is what a heredoc does not do.
    assert!(
        last_journal_line(&store).contains(r#""change":{"change":"body_replaced"}"#),
        "{}",
        last_journal_line(&store)
    );

    let replacement = scratch_root.join("aep-planning-body-section.md");
    write(&replacement, "Verify the signature, and nothing else.\n");
    let sectioned = protocol(&[
        "artifact",
        "body",
        "task:assertion-verification",
        "--from",
        printable(&replacement),
        "--section",
        "What",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&sectioned), 0, "{}", stderr(&sectioned));
    assert!(
        stdout(&sectioned).contains("`## What` written"),
        "{}",
        stdout(&sectioned)
    );

    let text = std::fs::read_to_string(&document).expect("readable");
    // 3. That section and only that section: the heading survives, its prose is the new prose,
    //    and the section after it is untouched.
    assert!(
        text.contains("## What\n\nVerify the signature, and nothing else.\n"),
        "{text}"
    );
    assert!(
        !text.contains("Verify the assertion signature against the stored public key"),
        "{text}"
    );
    assert!(
        text.contains("## Why"),
        "the following section was eaten: {text}"
    );
    assert!(
        text.contains("## Risks"),
        "the appended section was eaten: {text}"
    );

    // 4. A heading the document does not have is added at the end rather than refused: a caller
    //    asking for a section that is not there meant to write one.
    let invented = protocol(&[
        "artifact",
        "body",
        "task:assertion-verification",
        "--from",
        printable(&replacement),
        "--section",
        "Rollout",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&invented), 0, "{}", stderr(&invented));
    let text = std::fs::read_to_string(&document).expect("readable");
    assert!(
        text.trim_end()
            .ends_with("## Rollout\n\nVerify the signature, and nothing else."),
        "{text}"
    );

    let validated = protocol(&["artifact", "validate", "--store", at, "--root", tree]);
    assert_eq!(code(&validated), 0, "{}", stdout(&validated));

    std::fs::remove_file(addition).ok();
    std::fs::remove_file(replacement).ok();
}

/// `show --body-only` prints what `body --from` would write straight back, and nothing else.
#[test]
fn show_body_only_prints_the_bytes_body_from_would_write_back() {
    let printed = protocol(&[
        "artifact",
        "show",
        "task:assertion-verification",
        "--store",
        FIXTURE,
        "--body-only",
    ]);
    assert_eq!(code(&printed), 0, "{}", stderr(&printed));
    let body = stdout(&printed);

    // 1. The bytes the store holds, with none of the labels the plain rendering carries.
    let document =
        std::fs::read_to_string(root().join(FIXTURE).join("task/assertion-verification.md"))
            .expect("readable");
    let held = document
        .split_once("\n---\n")
        .map(|(_, body)| body)
        .expect("the document has a closing fence");
    assert_eq!(body, held);
    for label in ["revision", "status", "relations"] {
        assert!(
            !body.contains(&format!("{label}  ")),
            "a label leaked into the body: {body}"
        );
    }

    // 2. And it is refused where the promise cannot be kept: a machine format would wrap the bytes.
    let wrapped = protocol(&[
        "artifact",
        "show",
        "task:assertion-verification",
        "--store",
        FIXTURE,
        "--body-only",
        "--format",
        "json",
    ]);
    assert_eq!(code(&wrapped), 1, "{}", stdout(&wrapped));
    assert!(
        stderr(&wrapped).contains("--body-only"),
        "{}",
        stderr(&wrapped)
    );
}

/// **One frontmatter field has a verb, and four of them are refused by name.**
///
/// `ed007513#209-#274` spent about twenty-five turns writing documents with heredocs because no
/// verb changed a title, a summary or an owner. `11727595#818` patched `revision:` with `python`
/// and was caught as drift — so `set` refuses that field, and says what the field is for instead of
/// reporting an unrecognised flag.
#[test]
#[allow(clippy::too_many_lines)]
fn set_changes_a_frontmatter_field_and_refuses_the_four_it_does_not_own() {
    let repository = root();
    let tree = printable(&repository);
    let store = scratch("aep-planning-set");
    copy_tree(&repository.join(FIXTURE), &store);
    let at = printable(&store);
    let document = store.join("task/assertion-verification.md");
    let before = std::fs::read_to_string(&document).expect("readable");
    assert!(
        before.contains("title: Verify a sign-in assertion"),
        "{before}"
    );
    assert!(
        !before.contains("tags:"),
        "the fixture carries no tags: {before}"
    );

    let changed = protocol(&[
        "artifact",
        "set",
        "task:assertion-verification",
        "--title",
        "Verify a sign-in assertion, replay included",
        "--summary",
        "Signature, origin, sign count.",
        "--owner",
        "identity-platform",
        "--tag",
        "webauthn",
        "--tag",
        "security",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&changed), 0, "{}", stderr(&changed));
    assert!(
        stdout(&changed).contains("title, summary, owner, tags set"),
        "{}",
        stdout(&changed)
    );

    let text = std::fs::read_to_string(&document).expect("readable");
    assert!(
        text.contains("title: Verify a sign-in assertion, replay included"),
        "{text}"
    );
    assert!(
        text.contains("summary: Signature, origin, sign count."),
        "{text}"
    );
    assert!(text.contains("owner: identity-platform"), "{text}");
    assert!(text.contains("- webauthn"), "{text}");
    assert!(text.contains("- security"), "{text}");
    // The fields it was not asked about are the fields it did not touch.
    assert!(
        text.contains("- decomposes: story:passkey-login"),
        "the edge went missing: {text}"
    );
    assert!(text.contains("status: active"), "the status moved: {text}");
    assert!(
        text.contains("revision: 3"),
        "the store counts its own writes: {text}"
    );
    // And the prose, which is the thing a frontmatter splitter loses.
    assert!(
        text.contains("## Done When"),
        "the body was rewritten: {text}"
    );

    // `--untag` removes exactly the label it names, and leaves the one it does not.
    let untagged = protocol(&[
        "artifact",
        "set",
        "task:assertion-verification",
        "--untag",
        "webauthn",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&untagged), 0, "{}", stderr(&untagged));
    let text = std::fs::read_to_string(&document).expect("readable");
    assert!(!text.contains("- webauthn"), "{text}");
    assert!(
        text.contains("- security"),
        "the label it was not asked about went too: {text}"
    );

    // A write with nothing in it is a revision nobody can explain.
    let again = protocol(&[
        "artifact",
        "set",
        "task:assertion-verification",
        "--owner",
        "identity-platform",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&again), 0, "{}", stderr(&again));
    assert!(
        stdout(&again).contains("nothing to do"),
        "{}",
        stdout(&again)
    );

    // The four this verb will not change, each refused by name with the thing to type instead.
    for (flag, value, says) in [
        ("--status", "implemented", "protocol artifact move"),
        ("--revision", "9", "the store's own count"),
        ("--id", "task:renamed", "identity"),
        ("--kind", "story", "identity"),
    ] {
        let refused = protocol(&[
            "artifact",
            "set",
            "task:assertion-verification",
            flag,
            value,
            "--store",
            at,
            "--root",
            tree,
        ]);
        assert_eq!(
            code(&refused),
            1,
            "`{flag}` was not refused: {}",
            stdout(&refused)
        );
        assert!(
            stderr(&refused).contains(says),
            "`{flag}`: {}",
            stderr(&refused)
        );
    }
    let unchanged = std::fs::read_to_string(&document).expect("readable");
    assert!(
        unchanged.contains("status: active"),
        "a refused set wrote anyway: {unchanged}"
    );

    let validated = protocol(&["artifact", "validate", "--store", at, "--root", tree]);
    assert_eq!(code(&validated), 0, "{}", stdout(&validated));
}

/// **`move` refuses what `validate` would report a second later.**
///
/// `114c2340#92` and `4d4c15a4#149`: `move --to active` succeeded and the very next `validate`
/// answered `[empty_declaration] … is active and serves no objective`, because `validate_grounding`
/// ran only in `validate`. The rules now run on the store the move *would* leave, and the refusal is
/// the finding itself — its own words and its own hint, so the two verbs cannot say different things
/// about one document.
#[test]
fn a_move_that_would_leave_the_store_invalid_is_refused_with_the_finding() {
    let repository = root();
    let tree = printable(&repository);
    let store = scratch("aep-planning-move-grounding");
    let at = printable(&store);

    write(
        &store.join("vision/O1.md"),
        "---\nid: vision:O1\nkind: vision\nstatus: approved\ntitle: An objective\n---\n# O1\n",
    );
    write(
        &store.join("story/grounded.md"),
        "---\nid: story:grounded\nkind: story\nstatus: draft\ntitle: A story\n---\n# Story\n",
    );
    // The fixture has reached the state where the rule is load-bearing: a store that declares an
    // objective, and a story that is not yet agreed and therefore not yet held to one.
    let clean = protocol(&["artifact", "validate", "--store", at, "--root", tree]);
    assert_eq!(code(&clean), 0, "{}", stdout(&clean));

    let refused = protocol(&[
        "artifact",
        "move",
        "story:grounded",
        "--to",
        "proposed",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let said = stdout(&refused);
    assert!(said.contains("would not validate"), "{said}");
    assert!(
        said.contains("serves no objective"),
        "the finding's own text: {said}"
    );
    assert!(said.contains("hint:"), "the finding's own hint: {said}");

    // Refused means nothing was written, and the store still validates.
    let text = std::fs::read_to_string(store.join("story/grounded.md")).expect("readable");
    assert!(
        text.contains("status: draft"),
        "a refused move wrote anyway: {text}"
    );
    let after = protocol(&["artifact", "validate", "--store", at, "--root", tree]);
    assert_eq!(code(&after), 0, "{}", stdout(&after));

    // With the edge the finding asked for, the same move goes through — the refusal is about the
    // graph the move would leave, not about the rung.
    let related = protocol(&[
        "artifact",
        "relate",
        "story:grounded",
        "serves:vision:O1",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&related), 0, "{}", stderr(&related));
    let moved = protocol(&[
        "artifact",
        "move",
        "story:grounded",
        "--to",
        "proposed",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&moved), 0, "{}{}", stdout(&moved), stderr(&moved));
}

/// **`validate --strict` refuses what `validate` only reports**, and plain `validate` is untouched.
///
/// `ed007513#1300`: four stories reached `implemented` on an assertion — a swallowed `--reference`
/// typo — and `validate` printed the four bullets and then `valid`, exit 0. `9da4f51c` ran it 37
/// times, always `valid`. A gate that wants those to bite now has a flag; nobody else's exit code
/// moved.
#[test]
fn strict_validate_fails_on_what_plain_validate_only_reports() {
    let store = scratch("aep-planning-strict");
    let at = printable(&store);
    let make = |args: &[&str]| {
        let output = protocol_in(&root(), args);
        assert_eq!(code(&output), 0, "{}", stderr(&output));
    };
    make(&[
        "artifact", "new", "story", "asserted", "--title", "Demo", "--store", at,
    ]);
    for to in ["proposed", "active"] {
        make(&[
            "artifact",
            "move",
            "story:asserted",
            "--to",
            to,
            "--store",
            at,
        ]);
    }
    // The rung that asks for evidence, reached on a bare count nothing can check.
    make(&[
        "artifact",
        "move",
        "story:asserted",
        "--to",
        "implemented",
        "--evidence",
        "test_result=1",
        "--store",
        at,
    ]);

    // 1. Plain `validate` reports it and exits 0, which is `story:completion-needs-evidence`'s
    //    recorded position and is not what this flag changes.
    let plain = protocol(&["artifact", "validate", "--store", at]);
    assert_eq!(code(&plain), 0, "{}", stdout(&plain));
    assert!(
        stdout(&plain).contains("closed on an assertion"),
        "{}",
        stdout(&plain)
    );
    assert!(stdout(&plain).contains("valid"), "{}", stdout(&plain));

    // 2. `--strict` prints the same lines and exits 1, naming which class decided.
    let strict = protocol(&["artifact", "validate", "--strict", "--store", at]);
    assert_eq!(code(&strict), 1, "{}", stdout(&strict));
    assert!(
        stdout(&strict).contains("closed on an assertion"),
        "{}",
        stdout(&strict)
    );
    assert!(
        stdout(&strict).contains("--strict: refusing on 1 closed on an assertion"),
        "{}",
        stdout(&strict)
    );

    // 3. A document that predates the event log is the second class, and the committed fixture is
    //    a store made entirely of them — read only, and clean to plain `validate`.
    let committed = protocol(&["artifact", "validate", "--store", FIXTURE]);
    assert_eq!(code(&committed), 0, "{}", stdout(&committed));
    let refused = protocol(&["artifact", "validate", "--strict", "--store", FIXTURE]);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stdout(&refused).contains(&format!("{FIXTURE_ARTIFACTS} predating the event log")),
        "{}",
        stdout(&refused)
    );
}

/// **An empty body is not a body**, and the refusal names the flag that produced one.
///
/// `11727595#10819`: `body --from -` on empty standard input wrote the empty string over the prose
/// and bumped the revision, so the store held a document with nothing in it and a record saying
/// somebody meant that.
#[test]
fn a_body_that_is_empty_after_trimming_is_refused_naming_the_flag() {
    let store = scratch("aep-planning-empty-body");
    let at = printable(&store);
    assert_eq!(
        code(&protocol(&[
            "artifact", "new", "story", "demo", "--title", "Demo", "--store", at,
        ])),
        0
    );
    let before = std::fs::read_to_string(store.join("story/demo.md")).expect("readable");

    // A pipe that produced nothing.
    let piped = protocol_with_stdin(
        &[
            "artifact",
            "body",
            "story:demo",
            "--from",
            "-",
            "--store",
            at,
        ],
        "",
    );
    assert_eq!(code(&piped), 1, "{}", stdout(&piped));
    assert!(stderr(&piped).contains("--from"), "{}", stderr(&piped));

    // A file that turned out to hold whitespace.
    let blank = store
        .parent()
        .expect("scratch has a parent")
        .join("aep-planning-blank-body.md");
    write(&blank, "\n  \n\t\n");
    let from_file = protocol(&[
        "artifact",
        "body",
        "story:demo",
        "--from",
        printable(&blank),
        "--store",
        at,
    ]);
    assert_eq!(code(&from_file), 1, "{}", stdout(&from_file));
    assert!(
        stderr(&from_file).contains("--from"),
        "{}",
        stderr(&from_file)
    );

    // Neither wrote, and neither moved the revision.
    let after = std::fs::read_to_string(store.join("story/demo.md")).expect("readable");
    assert_eq!(after, before, "a refused body write changed the document");

    std::fs::remove_file(blank).ok();
}

/// **`kinds` lists what can be created**, which is more than the list compiled into the binary.
///
/// Reproduced live on 2026-08-30 (`fcf5873a#361`): `protocol artifact kinds | grep -i block`
/// returned nothing while `protocol artifact lifecycle third-party-blocker` answered — the verb the
/// skill names as the authority on what can be created did not name the family that had a ladder,
/// because it iterated `ArtifactKind::NAMED` and the blocker family is open.
#[test]
fn kinds_lists_the_ladders_a_store_declares_and_the_open_blocker_family() {
    let listed = protocol(&["artifact", "kinds"]);
    assert_eq!(code(&listed), 0, "{}", stderr(&listed));
    let text = stdout(&listed);

    // 1. The compiled vocabulary is still all there.
    for compiled in [
        "story",
        "task",
        "architecture-decision-record",
        "review-result",
    ] {
        assert!(
            text.lines()
                .any(|line| line.starts_with(&format!("{compiled} "))),
            "`{compiled}` fell out of the listing: {text}"
        );
    }

    // 2. A kind only `artifacts/lifecycles/*.yaml` declares is listed, and said to be the store's.
    let blocker = text
        .lines()
        .find(|line| line.starts_with("blocker "))
        .unwrap_or_else(|| panic!("`blocker` has a ladder in this tree and is not listed: {text}"));
    assert!(
        blocker.contains("planning"),
        "a blocker is intent, not output: {blocker}"
    );
    assert!(blocker.contains("lifecycles declare it"), "{blocker}");

    // 3. The family no list can enumerate is one row that says so.
    let family = text
        .lines()
        .find(|line| line.starts_with("<type>-blocker "))
        .unwrap_or_else(|| panic!("the open blocker family is not listed: {text}"));
    assert!(family.contains("planning"), "{family}");
    assert!(family.contains("open family"), "{family}");

    // 4. And it is the answer `blocked` sends a reader to, so the two verbs agree.
    let json = protocol(&["artifact", "kinds", "--format", "json"]);
    assert_eq!(code(&json), 0, "{}", stderr(&json));
    assert!(
        stdout(&json).contains("\"kind\": \"<type>-blocker\""),
        "{}",
        stdout(&json)
    );
}

/// **`blocked` says when the store has no blocker kind at all**, rather than reporting good news.
///
/// `431986de#7007`: `blocked` answered `nothing is blocked` in a store whose pin predates
/// `artifacts/kinds/blocker.yaml`, so the mechanism did not exist there — and the operator at
/// `#7024` asked "what are you talking about blockers".
#[test]
fn blocked_says_when_no_ladder_declares_a_blocker_at_all() {
    let repository = root();
    let store = scratch("aep-planning-blocked-no-ladder");
    let bare = scratch("aep-planning-blocked-bare-tree");
    copy_tree(&repository.join(FIXTURE), &store);
    let at = printable(&store);

    // 1. A tree that declares no blocker ladder: the answer is about the store's vocabulary, and
    //    points at the verb that lists what could be created instead.
    let without = protocol(&[
        "artifact",
        "blocked",
        "--store",
        at,
        "--root",
        printable(&bare),
    ]);
    assert_eq!(code(&without), 0, "{}", stderr(&without));
    assert_eq!(
        stdout(&without).trim(),
        "this store's lifecycles declare no blocker kind; `protocol artifact kinds` lists what can be created"
    );

    // 2. The same store read against a tree that does declare one: nothing is blocked, and that is
    //    now a fact about the plan rather than about the vocabulary.
    let with = protocol(&[
        "artifact",
        "blocked",
        "--store",
        at,
        "--root",
        printable(&repository),
    ]);
    assert_eq!(code(&with), 0, "{}", stderr(&with));
    assert_eq!(stdout(&with).trim(), "nothing is blocked");

    // 3. And with something actually blocked, the ladder-aware answer is the listing itself.
    let stuck = protocol(&[
        "artifact",
        "new",
        "credential-blocker",
        "token-scope",
        "--title",
        "No token",
        "--relate",
        "blocks:story:passkey-login",
        "--store",
        at,
        "--root",
        printable(&repository),
    ]);
    assert_eq!(code(&stuck), 0, "{}", stderr(&stuck));
    let listed = protocol(&[
        "artifact",
        "blocked",
        "--store",
        at,
        "--root",
        printable(&repository),
    ]);
    assert_eq!(code(&listed), 0, "{}", stderr(&listed));
    assert!(
        stdout(&listed).contains("credential-blocker:token-scope"),
        "{}",
        stdout(&listed)
    );
    assert!(
        !stdout(&listed).contains("nothing is blocked"),
        "{}",
        stdout(&listed)
    );
}

/// A title and a summary are prose, and prose begins with a dash often enough.
///
/// `114c2340#196`: `--summary "--strict is now a flag"` failed clap parsing, and `--summary=…` is a
/// workaround you have to already know. One retry per session, in three sessions.
#[test]
fn a_title_and_a_summary_may_begin_with_a_dash() {
    let store = scratch("aep-planning-hyphen-values");
    let at = printable(&store);
    let created = protocol(&[
        "artifact",
        "new",
        "story",
        "dashy",
        "--title",
        "--strict is now a flag",
        "--summary",
        "--strict changes the exit code and nothing else",
        "--store",
        at,
    ]);
    assert_eq!(code(&created), 0, "{}", stderr(&created));
    let text = std::fs::read_to_string(store.join("story/dashy.md")).expect("readable");
    assert!(text.contains("title: --strict is now a flag"), "{text}");
    assert!(text.contains("--strict changes the exit code"), "{text}");

    // `set` takes the same values, for the same reason.
    let changed = protocol(&[
        "artifact",
        "set",
        "story:dashy",
        "--summary",
        "--strict is opt-in",
        "--store",
        at,
    ]);
    assert_eq!(code(&changed), 0, "{}", stderr(&changed));
    let text = std::fs::read_to_string(store.join("story/dashy.md")).expect("readable");
    assert!(text.contains("--strict is opt-in"), "{text}");
}

/// **`relations` is a list, empty or not** — never a key that disappears.
///
/// `3130470e#132`: the documented `jq` shape broke on the first artifact with no edges, because a
/// key a machine format omits is a branch every consumer has to write.
#[test]
fn a_listing_says_no_relations_with_an_empty_list_rather_than_by_omission() {
    let listed = protocol(&["artifact", "list", "--store", FIXTURE, "--format", "json"]);
    assert_eq!(code(&listed), 0, "{}", stderr(&listed));
    let rows: serde_json::Value =
        serde_json::from_str(&stdout(&listed)).expect("the listing is JSON");
    let rows = rows.as_array().expect("the listing is an array");
    assert_eq!(rows.len(), FIXTURE_ARTIFACTS);

    // Every row has the key, whether or not the artifact has an edge.
    for row in rows {
        let relations = row
            .get("relations")
            .unwrap_or_else(|| panic!("a row with no `relations` key: {row}"));
        assert!(relations.is_array(), "`relations` is not a list: {row}");
    }

    // The store holds one of each, which is what makes this a claim rather than a shape check.
    let by_id = |id: &str| {
        rows.iter()
            .find(|row| row.get("id").and_then(serde_json::Value::as_str) == Some(id))
            .unwrap_or_else(|| panic!("no `{id}` in the listing"))
    };
    assert_eq!(
        by_id("initiative:passwordless-authentication")["relations"]
            .as_array()
            .expect("a list")
            .len(),
        0,
        "the top of the tree points at nothing and says so with an empty list"
    );
    let story = by_id("story:passkey-login")["relations"]
        .as_array()
        .expect("a list");
    assert_eq!(story.len(), 2, "{story:?}");
    assert_eq!(story[0]["relation"], "decomposes");
    assert_eq!(story[0]["target"], "epic:passkey-sign-in");

    // `show` answers the same way about the same artifact, which is what makes the two verbs one
    // shape a consumer can rely on.
    let shown = protocol(&[
        "artifact",
        "show",
        "initiative:passwordless-authentication",
        "--store",
        FIXTURE,
        "--format",
        "json",
    ]);
    assert_eq!(code(&shown), 0, "{}", stderr(&shown));
    let shown: serde_json::Value = serde_json::from_str(&stdout(&shown)).expect("JSON");
    assert_eq!(shown["relations"].as_array().expect("a list").len(), 0);
}

/// **`move --via` walks the rungs nothing guards, and stops at the first one that is.**
///
/// `draft -> proposed -> active` is two commands per story on every wave (`8cffc110#184`), and
/// `9da4f51c#3303` is a `python` loop issuing four commands for each of eight stories. One command
/// now, and still one journal entry per hop — a walk that recorded one move would be a history
/// saying the story was never proposed.
#[test]
#[allow(clippy::too_many_lines)]
fn a_walk_crosses_unguarded_rungs_and_stops_at_a_guarded_one() {
    let tree = scratch("aep-planning-via-root");
    let store = scratch("aep-planning-via-store");
    let at = printable(&store);
    write(
        &tree.join("artifacts/lifecycles/charter.yaml"),
        "kind: charter\n\
         initial: intake\n\
         transitions:\n  \
           intake: [triage]\n  \
           triage: [approved]\n  \
           approved: []\n",
    );
    // The same shape with a rung that asks for something in the middle of it.
    write(
        &tree.join("artifacts/lifecycles/warrant.yaml"),
        "kind: warrant\n\
         initial: intake\n\
         transitions:\n  \
           intake: [signed]\n  \
           signed: [approved]\n  \
           approved: []\n\
         requires:\n  \
           signed:\n    \
             - evidence: approval\n      \
               at_least: 1\n",
    );
    let tree = printable(&tree);

    for (kind, name) in [("charter", "open"), ("warrant", "gated")] {
        let made = protocol(&[
            "artifact", "new", kind, name, "--title", "X", "--store", at, "--root", tree,
        ]);
        assert_eq!(code(&made), 0, "{}", stderr(&made));
    }

    // 1. Two unguarded rungs, one command, two lines out.
    let walked = protocol(&[
        "artifact",
        "move",
        "charter:open",
        "--to",
        "approved",
        "--via",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&walked), 0, "{}{}", stdout(&walked), stderr(&walked));
    let said = stdout(&walked);
    assert!(
        said.contains("charter:open moved intake -> triage (revision 2)"),
        "{said}"
    );
    assert!(
        said.contains("charter:open moved triage -> approved (revision 3)"),
        "{said}"
    );

    // 2. Each hop is its own entry in the journal, and the document is where the walk ended.
    let journal = std::fs::read_to_string(store.join("journal.jsonl")).expect("a journal");
    let hops: Vec<&str> = journal
        .lines()
        .filter(|line| line.contains(r#""change":"moved""#) && line.contains(r#""id":"open""#))
        .collect();
    assert_eq!(
        hops.len(),
        2,
        "a walk journals every rung it crossed: {journal}"
    );
    let text = std::fs::read_to_string(store.join("charter/open.md")).expect("readable");
    assert!(text.contains("status: approved"), "{text}");
    assert!(text.contains("revision: 3"), "{text}");

    // 3. A guarded rung in the middle stops the walk in that rung's own words, and writes nothing.
    let stopped = protocol(&[
        "artifact",
        "move",
        "warrant:gated",
        "--to",
        "approved",
        "--via",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&stopped), 1, "{}", stdout(&stopped));
    let said = stdout(&stopped);
    assert!(said.contains("warrant:gated is intake"), "{said}");
    assert!(
        said.contains("approval"),
        "the guarded rung's own refusal: {said}"
    );
    assert!(
        !said.contains("moved"),
        "a refused walk moved something: {said}"
    );
    let text = std::fs::read_to_string(store.join("warrant/gated.md")).expect("readable");
    assert!(
        text.contains("status: intake"),
        "a refused walk wrote anyway: {text}"
    );
    assert!(text.contains("revision: 1"), "{text}");

    // 4. **And it is guarded even when the evidence is on hand**, which is where the rule is
    //    load-bearing: `--via` crosses rungs nothing asks anything of, so one asserted count must
    //    not carry an artifact across two gates at once. The same rung, moved to on its own with
    //    the same evidence, goes through — so the refusal is about the walk and not about the
    //    evidence.
    let laundered = protocol(&[
        "artifact",
        "move",
        "warrant:gated",
        "--to",
        "approved",
        "--via",
        "--evidence",
        "approval=1",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&laundered), 1, "{}", stdout(&laundered));
    assert!(
        stdout(&laundered).contains("`--via` walks rungs nothing guards"),
        "{}",
        stdout(&laundered)
    );
    assert!(
        stdout(&laundered).contains("signed is guarded"),
        "{}",
        stdout(&laundered)
    );
    let alone = protocol(&[
        "artifact",
        "move",
        "warrant:gated",
        "--to",
        "signed",
        "--evidence",
        "approval=1",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&alone), 0, "{}{}", stdout(&alone), stderr(&alone));

    // 5. Without `--via`, the two-rung request is the ordinary single-hop refusal it always was.
    let direct = protocol(&[
        "artifact",
        "move",
        "charter:open",
        "--to",
        "intake",
        "--store",
        at,
        "--root",
        tree,
    ]);
    assert_eq!(code(&direct), 1, "{}", stdout(&direct));
    assert!(
        stdout(&direct).contains("charter:open is approved"),
        "{}",
        stdout(&direct)
    );
}

/// **`explain` ends by saying what the next rung costs**, so the requirement is read rather than
/// learnt by being refused.
///
/// `11727595#3402-#3407`: `explain` said "no status move is recorded" and nothing about what the
/// next rung wanted, so the requirement was found out by `move` refusing twice. A rung's price is a
/// line in a lifecycle document; there is no reason to be refused to see it.
#[test]
fn explain_ends_with_what_each_legal_next_rung_costs() {
    let store = scratch("aep-planning-explain-next");
    let at = printable(&store);
    let run = |args: &[&str]| {
        let output = protocol(args);
        assert_eq!(code(&output), 0, "{}", stderr(&output));
        output
    };
    run(&[
        "artifact", "new", "story", "demo", "--title", "Demo", "--store", at,
    ]);
    run(&[
        "artifact",
        "move",
        "story:demo",
        "--to",
        "active",
        "--via",
        "--store",
        at,
    ]);

    // 1. Held nothing: the rung that asks for something says how much, and that none is held.
    let explained = run(&["artifact", "explain", "story:demo", "--store", at]);
    let text = stdout(&explained);
    assert!(
        text.contains("next: implemented needs 1 test_result record(s); held: 0"),
        "{text}"
    );
    // A rung the ladder asks nothing of is still a line, because *nothing* is the answer to the
    // same question and a missing line reads as a rung that is not legal.
    assert!(text.contains("next: archived needs no record"), "{text}");
    // Only the rungs this status leads to.
    assert!(
        !text.contains("next: proposed"),
        "`active` does not lead to `proposed`: {text}"
    );

    // 2. The count is the store's own, about this artifact, and moves when a record is admitted.
    run(&[
        "artifact",
        "evidence",
        "story:demo",
        "--kind",
        "test_result",
        "--source",
        "task check",
        "--store",
        at,
    ]);
    let explained = run(&["artifact", "explain", "story:demo", "--store", at]);
    assert!(
        stdout(&explained).contains("next: implemented needs 1 test_result record(s); held: 1"),
        "{}",
        stdout(&explained)
    );

    // 3. And the machine format carries the same three numbers rather than the sentence.
    let json = run(&[
        "artifact",
        "explain",
        "story:demo",
        "--store",
        at,
        "--format",
        "json",
    ]);
    let value: serde_json::Value = serde_json::from_str(&stdout(&json)).expect("JSON");
    let next = value["next"].as_array().expect("a list of rungs");
    let implemented = next
        .iter()
        .find(|rung| rung["status"] == "implemented")
        .expect("the guarded rung");
    assert_eq!(implemented["needs"][0]["kind"], "test_result");
    assert_eq!(implemented["needs"][0]["at_least"], 1);
    assert_eq!(implemented["needs"][0]["held"], 1);
}

/// **An evidence-kind refusal ends with the two kinds people actually reach for.**
///
/// `431986de#6957` wrote `measurement`; `e70b8018 s1#694` wrote `cross_repo_dependency`. Both have
/// a kind in this vocabulary and neither name is it, and a list of fifteen is not findable by
/// someone who does not have the word.
#[test]
fn a_refused_evidence_kind_names_the_nearest_two_that_exist() {
    let store = scratch("aep-planning-evidence-kind-hint");
    let at = printable(&store);
    assert_eq!(
        code(&protocol(&[
            "artifact", "new", "story", "demo", "--title", "Demo", "--store", at,
        ])),
        0
    );
    let advice = "for an observation of a running system use `health_observation`; \
                  for a relation to another store's artifact use `artifact`";

    // The `evidence` verb, with the word one session actually typed.
    let recorded = protocol(&[
        "artifact",
        "evidence",
        "story:demo",
        "--kind",
        "measurement",
        "--source",
        "grafana",
        "--store",
        at,
    ]);
    assert_eq!(code(&recorded), 1, "{}", stdout(&recorded));
    assert!(
        stderr(&recorded).trim_end().ends_with(advice),
        "the advice is not the last thing said: {}",
        stderr(&recorded)
    );

    // And `move --evidence`, with the word the other one typed.
    let moved = protocol(&[
        "artifact",
        "move",
        "story:demo",
        "--to",
        "proposed",
        "--evidence",
        "cross_repo_dependency=1",
        "--store",
        at,
    ]);
    assert_eq!(code(&moved), 1, "{}", stdout(&moved));
    assert!(
        stderr(&moved).trim_end().ends_with(advice),
        "the advice is not the last thing said: {}",
        stderr(&moved)
    );

    // Both kinds it names are kinds, which is what makes the advice worth taking.
    for kind in ["health_observation", "artifact"] {
        let output = protocol(&[
            "artifact",
            "evidence",
            "story:demo",
            "--kind",
            kind,
            "--source",
            "x",
            "--store",
            at,
        ]);
        assert_eq!(code(&output), 0, "`{kind}`: {}", stderr(&output));
    }
}
