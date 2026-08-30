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
    assert_eq!(
        text.lines()
            .filter(|line| line.contains("planning"))
            .count(),
        6,
        "six kinds are intent decomposition: {text}"
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
