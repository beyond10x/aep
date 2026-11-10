//! `aep plan artifact scope` and `aep plan artifact waves`, driven as the binary.
//!
//! A wave is the claim that N units may be worked at once, and the property it rests on is that
//! they touch different surfaces. Until `scope` there was nowhere to write a surface down, so the
//! claim was a pairwise reading of prose; these tests hold the two verbs that replace it — the one
//! that records a surface, and the one that derives the waves and names what it excluded.
//!
//! Its own file rather than lines in `planning_cli.rs`: the fixture store, the byte-exact
//! recording and the read-only assertion are one subject, and a 3500-line test file is where a
//! subject goes to be un-findable.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The repository root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// The committed fixture store with a recorded answer.
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wave-plan")
}

/// Runs `aep` from the repository root, so the document tree resolves the way it does in use.
fn aep(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(args)
        .current_dir(root())
        .output()
        .expect("the protocol binary runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("the process exited normally")
}

fn printable(path: &Path) -> &str {
    path.to_str().expect("a printable path")
}

/// An empty scratch directory to build a store in.
///
/// `temp_dir`, which is what every other test file here uses, so the environment decides where a
/// test writes rather than this file naming a directory that may not be writable.
fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(name);
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("the previous scratch tree is removable");
    }
    std::fs::create_dir_all(&directory).expect("the temporary tree is writable");
    directory
}

/// Runs a command that must succeed, and says what it printed when it does not.
fn make(args: &[&str]) -> String {
    let output = aep(args);
    assert_eq!(
        code(&output),
        0,
        "`{}` failed: {}{}",
        args.join(" "),
        stdout(&output),
        stderr(&output)
    );
    stdout(&output)
}

/// Every file under a tree, by path, with its bytes — what a read-only verb must not change.
fn bytes_under(directory: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut found = BTreeMap::new();
    collect(directory, directory, &mut found);
    found
}

fn collect(base: &Path, at: &Path, into: &mut BTreeMap<PathBuf, Vec<u8>>) {
    for entry in std::fs::read_dir(at).expect("the tree is readable") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            collect(base, &path, into);
        } else {
            let relative = path.strip_prefix(base).expect("under the base").to_owned();
            into.insert(
                relative,
                std::fs::read(&path).expect("the file is readable"),
            );
        }
    }
}

/// One story document, written straight into the store.
///
/// For the two cases a command cannot produce: a `depends_on` cycle, which `relate` refuses, and
/// the committed fixture, which has to be readable bytes rather than a recording of somebody's
/// clock and user name.
fn hand_written(store: &Path, name: &str, scope: &[&str], depends_on: Option<&str>) {
    let mut document = String::from("---\nformat: aep.planning-md/1\n");
    let _ = write!(document, "id: story:{name}\nkind: story\nstatus: draft\n");
    let _ = writeln!(document, "title: Story {name}");
    if let Some(target) = depends_on {
        let _ = write!(document, "relations:\n- depends_on: {target}\n");
    }
    if !scope.is_empty() {
        document.push_str("scope:\n");
        for path in scope {
            let (path, confidence) = match path.strip_prefix('?') {
                Some(path) => (path, "inferred"),
                None => (*path, "cited"),
            };
            let _ = write!(document, "- path: {path}\n  confidence: {confidence}\n");
        }
    }
    document.push_str("revision: 1\n---\n\n# Story: ");
    document.push_str(name);
    document.push('\n');

    let directory = store.join("story");
    std::fs::create_dir_all(&directory).expect("the story directory is writable");
    std::fs::write(directory.join(format!("{name}.md")), document)
        .expect("the document is written");
}

/// A store holding one story per `(name, paths)`, each path `cited` unless it starts with `?`.
fn store_of(name: &str, stories: &[(&str, &[&str])]) -> PathBuf {
    let store = scratch(name);
    let at = printable(&store).to_owned();
    for (story, paths) in stories {
        make(&[
            "plan", "artifact", "new", "story", story, "--title", story, "--store", &at,
        ]);
        for path in *paths {
            let id = format!("story:{story}");
            match path.strip_prefix('?') {
                Some(inferred) => make(&[
                    "plan",
                    "artifact",
                    "scope",
                    &id,
                    "--add",
                    inferred,
                    "--inferred",
                    "--store",
                    &at,
                ]),
                None => make(&[
                    "plan", "artifact", "scope", &id, "--add", path, "--store", &at,
                ]),
            };
        }
    }
    store
}

/// The `--add` half: a typed entry in the frontmatter, and **one** revision for the write.
///
/// The field is what a computation reads and the `## Scope` section is what a person reads; this
/// is the door that writes the first, through the same command path every other write passes, so
/// the journal has the record.
#[test]
fn scope_add_writes_a_typed_entry_and_bumps_the_revision_once() {
    let store = scratch("aep-scope-add");
    let at = printable(&store);
    make(&[
        "plan", "artifact", "new", "story", "surface", "--title", "Surface", "--store", at,
    ]);

    let added = make(&[
        "plan",
        "artifact",
        "scope",
        "story:surface",
        "--add",
        "crates/govern/aep-domain/src/artifact.rs",
        "--store",
        at,
    ]);
    assert!(
        added.contains("revision 2"),
        "one write is one revision: {added}"
    );

    let document = std::fs::read_to_string(store.join("story/surface.md")).expect("the document");
    assert!(
        document.contains("scope:")
            && document.contains("crates/govern/aep-domain/src/artifact.rs"),
        "{document}"
    );
    assert!(
        document.contains("confidence: cited"),
        "a path recorded without `--inferred` is one somebody read: {document}"
    );

    // An `--add` of what the document already says is not a write, so it is not a revision.
    let again = make(&[
        "plan",
        "artifact",
        "scope",
        "story:surface",
        "--add",
        "crates/govern/aep-domain/src/artifact.rs",
        "--store",
        at,
    ]);
    assert!(
        again.contains("already reads that way"),
        "a write with nothing in it is a revision nobody can explain: {again}"
    );
}

/// `--inferred` is a first-class value and not a weasel word: a scope that mixes what was read
/// with what was guessed is trusted exactly where it is weakest, so the two are kept apart.
#[test]
fn scope_records_an_inferred_entry_apart_from_a_cited_one_and_remove_takes_it_out() {
    let store = scratch("aep-scope-inferred");
    let at = printable(&store);
    make(&[
        "plan", "artifact", "new", "story", "guessed", "--title", "Guessed", "--store", at,
    ]);
    make(&[
        "plan",
        "artifact",
        "scope",
        "story:guessed",
        "--add",
        "crates/edge/aep-cli/src/planning.rs",
        "--inferred",
        "--store",
        at,
    ]);
    let document = std::fs::read_to_string(store.join("story/guessed.md")).expect("the document");
    assert!(document.contains("confidence: inferred"), "{document}");

    let removed = make(&[
        "plan",
        "artifact",
        "scope",
        "story:guessed",
        "--remove",
        "crates/edge/aep-cli/src/planning.rs",
        "--store",
        at,
    ]);
    assert!(removed.contains("revision 3"), "{removed}");
    let document = std::fs::read_to_string(store.join("story/guessed.md")).expect("the document");
    assert!(
        !document.contains("scope:"),
        "an empty scope is an absent key, not an empty list: {document}"
    );

    // And the document still agrees with its own log. Taking the last path out writes `scope: []`
    // into the event; a store that then read the document as carrying no `scope` at all would
    // report the command's own write as an edit made outside a command.
    let validated = aep(&["plan", "artifact", "validate", "--store", at]);
    assert_eq!(
        code(&validated),
        0,
        "clearing a scope is a write, not drift: {}",
        stdout(&validated)
    );
    assert!(
        !stdout(&validated).contains("drifted"),
        "{}",
        stdout(&validated)
    );
}

/// Story only, and refused by name. The default `story:a-story-records-where-it-lands` records is
/// story-only for now: a task inherits its story's surface, and a required field on both doubles
/// the retroactive work for a property only one of them is selected on.
#[test]
fn scope_is_refused_on_a_kind_that_is_not_a_story() {
    let store = scratch("aep-scope-kind");
    let at = printable(&store);
    make(&[
        "plan",
        "artifact",
        "new",
        "epic",
        "elsewhere",
        "--title",
        "Elsewhere",
        "--store",
        at,
    ]);
    let refused = aep(&[
        "plan",
        "artifact",
        "scope",
        "epic:elsewhere",
        "--add",
        "crates/x.rs",
        "--store",
        at,
    ]);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("story") && stderr(&refused).contains("epic"),
        "the refusal names the kind it got and the kind it takes: {}",
        stderr(&refused)
    );
}

/// `show` prints it for a person and `--format json` carries it as an array for everything else.
#[test]
fn show_prints_the_scope_and_json_carries_it_as_an_array() {
    let store = scratch("aep-scope-show");
    let at = printable(&store);
    make(&[
        "plan", "artifact", "new", "story", "shown", "--title", "Shown", "--store", at,
    ]);
    make(&[
        "plan",
        "artifact",
        "scope",
        "story:shown",
        "--add",
        "crates/govern/aep-domain/src/artifact.rs",
        "--store",
        at,
    ]);
    make(&[
        "plan",
        "artifact",
        "scope",
        "story:shown",
        "--add",
        "crates/edge/aep-cli/src/planning.rs",
        "--inferred",
        "--store",
        at,
    ]);

    let text = make(&["plan", "artifact", "show", "story:shown", "--store", at]);
    assert!(
        text.contains("scope") && text.contains("crates/govern/aep-domain/src/artifact.rs"),
        "{text}"
    );
    assert!(
        text.contains("inferred"),
        "a person reading the block can tell the guessed line from the read one: {text}"
    );

    let json = make(&[
        "plan",
        "artifact",
        "show",
        "story:shown",
        "--format",
        "json",
        "--store",
        at,
    ]);
    let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let scope = value["scope"].as_array().expect("scope is an array");
    assert_eq!(scope.len(), 2, "{json}");
    // Path order, which is the store's and not the order the two `--add` calls arrived in.
    assert_eq!(scope[0]["path"], "crates/edge/aep-cli/src/planning.rs");
    assert_eq!(scope[0]["confidence"], "inferred");
    assert_eq!(scope[1]["path"], "crates/govern/aep-domain/src/artifact.rs");
    assert_eq!(scope[1]["confidence"], "cited");
}

/// Reported and **not** failed on, which is the tier `story:a-story-records-where-it-lands` argues
/// for: refusing would turn every unscoped story red on the day it landed, which is how a check
/// gets muted.
#[test]
fn validate_reports_a_non_draft_story_with_no_scope_and_still_exits_zero() {
    let store = scratch("aep-scope-validate");
    let at = printable(&store);
    make(&[
        "plan", "artifact", "new", "story", "unscoped", "--title", "Unscoped", "--store", at,
    ]);
    make(&[
        "plan", "artifact", "new", "story", "scoped", "--title", "Scoped", "--store", at,
    ]);
    make(&[
        "plan",
        "artifact",
        "scope",
        "story:scoped",
        "--add",
        "crates/x.rs",
        "--store",
        at,
    ]);
    for story in ["story:unscoped", "story:scoped"] {
        make(&[
            "plan", "artifact", "move", story, "--to", "proposed", "--store", at,
        ]);
    }

    let validated = aep(&["plan", "artifact", "validate", "--store", at]);
    assert_eq!(
        code(&validated),
        0,
        "an unscoped story is reported, never refused: {}",
        stdout(&validated)
    );
    assert!(
        stdout(&validated).contains("story:unscoped"),
        "{}",
        stdout(&validated)
    );
    assert!(
        !stdout(&validated).contains("story:scoped"),
        "a story that answered is not on the list: {}",
        stdout(&validated)
    );

    // A draft is not on it either: the gap is a list of stories being *proposed* without a
    // surface, not of stories somebody has started writing.
    let drafted = scratch("aep-scope-validate-draft");
    let drafted_at = printable(&drafted);
    make(&[
        "plan", "artifact", "new", "story", "early", "--title", "Early", "--store", drafted_at,
    ]);
    let validated = aep(&["plan", "artifact", "validate", "--store", drafted_at]);
    assert!(
        !stdout(&validated).contains("story:early"),
        "{}",
        stdout(&validated)
    );
}

/// Two stories that touch nothing in common run at once; the one that collides is pushed, and the
/// pair that caused it is named with the path they share.
#[test]
fn waves_places_disjoint_stories_together_and_pushes_the_pair_that_collides() {
    let store = store_of(
        "aep-waves-collision",
        &[
            ("alpha", &["crates/a.rs"]),
            ("beta", &["crates/b.rs"]),
            ("gamma", &["crates/a.rs"]),
        ],
    );
    let at = printable(&store);
    let printed = make(&["plan", "artifact", "waves", "--store", at]);

    assert!(
        printed.contains("wave 1\n  story:alpha\n  story:beta\n"),
        "{printed}"
    );
    assert!(printed.contains("wave 2\n  story:gamma\n"), "{printed}");
    assert!(
        printed.contains("collision: story:alpha story:gamma crates/a.rs"),
        "the pair is named with the path that excluded it: {printed}"
    );
}

/// A dependency is an ordering constraint: the thing depended on is never in the same wave as the
/// thing that depends on it, nor in a later one, however disjoint their surfaces are.
#[test]
fn waves_never_point_a_dependency_the_wrong_way() {
    let store = store_of(
        "aep-waves-order",
        &[
            ("base", &["crates/base.rs"]),
            ("later", &["crates/later.rs"]),
        ],
    );
    let at = printable(&store);
    make(&[
        "plan",
        "artifact",
        "relate",
        "story:later",
        "depends_on",
        "story:base",
        "--store",
        at,
    ]);

    let json = make(&[
        "plan", "artifact", "waves", "--format", "json", "--store", at,
    ]);
    let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let waves = value["waves"].as_array().expect("waves is an array");
    assert_eq!(waves.len(), 2, "{json}");
    assert_eq!(waves[0]["artifacts"][0]["id"], "story:base", "{json}");
    assert_eq!(waves[1]["artifacts"][0]["id"], "story:later", "{json}");
}

/// A cycle has no ordering, so the verb says which ids form it and exits 2 rather than printing a
/// sequence it cannot justify.
///
/// The store is **written by hand**, and that is the point: `artifact relate` refuses the second
/// edge — graph validation reports `depends_on` edges forming a cycle — so the only way a store
/// holds one is an edit made outside a command. That is exactly the store this verb has to survive,
/// and a cycle it walked into instead of reporting would be an infinite one.
#[test]
fn a_depends_on_cycle_prints_its_ids_and_exits_two() {
    let store = scratch("aep-waves-cycle");
    let at = printable(&store);
    hand_written(&store, "one", &["crates/one.rs"], Some("story:two"));
    hand_written(&store, "two", &["crates/two.rs"], Some("story:one"));

    let output = aep(&["plan", "artifact", "waves", "--store", at]);
    assert_eq!(code(&output), 2, "{}{}", stdout(&output), stderr(&output));
    let printed = stdout(&output);
    assert!(printed.contains("cycle:"), "{printed}");
    assert!(
        printed.contains("story:one") && printed.contains("story:two"),
        "the cycle's own ids, so a reader knows which edge to cut: {printed}"
    );
}

/// An unassessed story reads exactly like a safe one, and that is the defect. It is listed and
/// never placed.
#[test]
fn a_story_with_no_scope_is_unassessed_and_never_placed() {
    let store = store_of(
        "aep-waves-unassessed",
        &[("known", &["crates/known.rs"]), ("unknown", &[])],
    );
    let at = printable(&store);
    let json = make(&[
        "plan", "artifact", "waves", "--format", "json", "--store", at,
    ]);
    let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    assert_eq!(
        value["unassessed"].as_array().expect("an array"),
        &vec![serde_json::Value::String("story:unknown".to_owned())],
        "{json}"
    );
    let placed: Vec<String> = value["waves"]
        .as_array()
        .expect("an array")
        .iter()
        .flat_map(|wave| wave["artifacts"].as_array().expect("an array").clone())
        .map(|entry| entry["id"].as_str().expect("an id").to_owned())
        .collect();
    assert_eq!(placed, vec!["story:known".to_owned()], "{json}");
}

/// An inferred entry counts as a collision and says so, because a wave resting on a guessed
/// surface and one resting on a read surface are different claims.
#[test]
fn an_inferred_entry_collides_and_is_marked_as_inferred() {
    let store = store_of(
        "aep-waves-inferred",
        &[
            ("read", &["crates/shared.rs"]),
            ("guessed", &["?crates/shared.rs"]),
        ],
    );
    let at = printable(&store);
    let printed = make(&["plan", "artifact", "waves", "--store", at]);
    assert!(
        printed.contains("collision: story:guessed story:read crates/shared.rs (inferred)"),
        "{printed}"
    );

    let json = make(&[
        "plan", "artifact", "waves", "--format", "json", "--store", at,
    ]);
    let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(value["collisions"][0]["confidence"], "inferred", "{json}");
}

/// `--status` narrows what is sequenced, because a wave is assembled out of one part of a board.
#[test]
fn waves_answers_about_the_status_it_was_asked_about() {
    let store = store_of(
        "aep-waves-status",
        &[
            ("early", &["crates/early.rs"]),
            ("ready", &["crates/ready.rs"]),
        ],
    );
    let at = printable(&store);
    make(&[
        "plan",
        "artifact",
        "move",
        "story:ready",
        "--to",
        "proposed",
        "--store",
        at,
    ]);

    let printed = make(&[
        "plan", "artifact", "waves", "--status", "proposed", "--store", at,
    ]);
    assert!(printed.contains("story:ready"), "{printed}");
    assert!(!printed.contains("story:early"), "{printed}");
}

/// The verb reads and prints. Asserted on the bytes, because *it does not write* is the property a
/// coordinator running it on a shared store is relying on.
#[test]
fn waves_leaves_every_byte_of_the_store_where_it_was() {
    let store = store_of(
        "aep-waves-readonly",
        &[
            ("first", &["crates/first.rs"]),
            ("second", &["crates/first.rs"]),
        ],
    );
    let at = printable(&store);
    let before = bytes_under(&store);
    let output = aep(&["plan", "artifact", "waves", "--store", at]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert_eq!(
        bytes_under(&store),
        before,
        "a read verb that wrote anything is a read verb nobody can run on a shared store"
    );
}

/// A directory contains every file under it, so a story scoped to `roles/` and one scoped to
/// `roles/decision.yaml` land on one surface. Compared as strings they read as disjoint and a wave
/// put them together, which is the collision the verb exists to report before the merge does.
#[test]
fn a_directory_and_a_file_inside_it_collide_and_never_share_a_wave() {
    let store = store_of(
        "aep-waves-directory",
        &[("dir", &["roles/"]), ("file", &["roles/decision.yaml"])],
    );
    let at = printable(&store);
    let printed = make(&["plan", "artifact", "waves", "--store", at]);

    assert!(
        printed.contains("wave 1\n  story:dir\nwave 2\n  story:file\n"),
        "a directory and a file inside it are never in one wave: {printed}"
    );
    assert!(
        printed.contains("collision: story:dir story:file roles/decision.yaml\n"),
        "the pair is named with the surface both land on, the file: {printed}"
    );
    assert!(
        printed.ends_with("2 wave(s), 1 collision(s), 0 unassessed\n"),
        "{printed}"
    );
}

/// `roles` and `roles/` are one spelling choice, not two surfaces: a story that wrote one must not
/// read as disjoint from a story that wrote the other, and against a file inside the directory both
/// spellings give the same answer, byte for byte.
#[test]
fn a_trailing_separator_does_not_change_the_answer() {
    let with = store_of(
        "aep-waves-separator-with",
        &[("dir", &["roles/"]), ("file", &["roles/decision.yaml"])],
    );
    let without = store_of(
        "aep-waves-separator-without",
        &[("dir", &["roles"]), ("file", &["roles/decision.yaml"])],
    );
    let printed_with = make(&["plan", "artifact", "waves", "--store", printable(&with)]);
    let printed_without = make(&["plan", "artifact", "waves", "--store", printable(&without)]);
    assert_eq!(
        printed_with, printed_without,
        "`roles/` and `roles` must give the same answer against `roles/decision.yaml`"
    );
    assert!(
        printed_without.contains("collision: story:dir story:file roles/decision.yaml\n"),
        "{printed_without}"
    );

    let both = store_of(
        "aep-waves-separator-both",
        &[("slash", &["roles/"]), ("bare", &["roles"])],
    );
    let printed = make(&["plan", "artifact", "waves", "--store", printable(&both)]);
    assert!(
        printed.contains("wave 1\n  story:bare\nwave 2\n  story:slash\n"),
        "the two spellings of one directory are one surface: {printed}"
    );
    assert!(
        printed.contains("collision: story:bare story:slash roles\n"),
        "{printed}"
    );
}

/// Containment is by path component, not by leading characters: `roles` holds `roles/x.yaml` but
/// not its sibling file `roles.md`, nor the neighbouring directory `rolesx/`.
#[test]
fn a_sibling_of_the_directory_stays_disjoint_from_it() {
    let store = store_of(
        "aep-waves-sibling",
        &[
            ("dir", &["roles/"]),
            ("sibling", &["roles.md"]),
            ("neighbour", &["rolesx/lookup.yaml"]),
        ],
    );
    let at = printable(&store);
    let printed = make(&["plan", "artifact", "waves", "--store", at]);

    assert!(
        printed.contains("wave 1\n  story:dir\n  story:neighbour\n  story:sibling\n"),
        "{printed}"
    );
    assert!(
        printed.ends_with("1 wave(s), 0 collision(s), 0 unassessed\n"),
        "a shared leading string is not a shared surface: {printed}"
    );
}

/// `./roles/` is `roles/` spelled with a leading current-directory component, so it holds
/// `roles/decision.yaml` exactly as `roles/` does. Compared as text after only a trailing `/` is
/// cut, the two read as disjoint and share a wave.
#[test]
fn a_leading_current_directory_component_names_the_same_surface() {
    let store = store_of(
        "aep-waves-dot-slash",
        &[("dir", &["./roles/"]), ("file", &["roles/decision.yaml"])],
    );
    let printed = make(&["plan", "artifact", "waves", "--store", printable(&store)]);
    assert!(
        printed.contains("wave 1\n  story:dir\nwave 2\n  story:file\n"),
        "`./roles/` holds `roles/decision.yaml`, so they never share a wave: {printed}"
    );
    assert!(
        printed.ends_with("2 wave(s), 1 collision(s), 0 unassessed\n"),
        "{printed}"
    );
}

/// `roles//decision.yaml` is the file `roles/decision.yaml`: an empty component names nothing.
#[test]
fn a_doubled_separator_names_the_same_surface() {
    let store = store_of(
        "aep-waves-double-slash",
        &[
            ("doubled", &["roles//decision.yaml"]),
            ("single", &["roles/decision.yaml"]),
        ],
    );
    let printed = make(&["plan", "artifact", "waves", "--store", printable(&store)]);
    assert!(
        printed.contains("wave 1\n  story:doubled\nwave 2\n  story:single\n"),
        "one file spelled two ways is one surface: {printed}"
    );
    assert!(
        printed.ends_with("2 wave(s), 1 collision(s), 0 unassessed\n"),
        "{printed}"
    );
}

/// `.` is the repository root, a repository-relative path that holds every other one: a story
/// scoped to the whole tree collides with every story.
#[test]
fn the_repository_root_holds_every_path() {
    let store = store_of(
        "aep-waves-root",
        &[("everything", &["."]), ("file", &["roles/decision.yaml"])],
    );
    let printed = make(&["plan", "artifact", "waves", "--store", printable(&store)]);
    assert!(
        printed.contains("wave 1\n  story:everything\nwave 2\n  story:file\n"),
        "a story on the whole tree never shares a wave: {printed}"
    );
}

/// The commit makes `roles` and `roles/` one surface, so a story that wrote both spellings has
/// declared one surface, and its collision with another story on that surface is one collision —
/// "once per pair and path". Both spellings reach the store through `scope --add`, which dedups by
/// string.
#[test]
fn one_surface_spelled_twice_in_one_story_is_one_collision() {
    let store = store_of(
        "aep-waves-spelled-twice",
        &[("both", &["roles", "?roles/"]), ("other", &["roles"])],
    );
    let printed = make(&["plan", "artifact", "waves", "--store", printable(&store)]);
    assert_eq!(
        printed.matches("collision:").count(),
        1,
        "one pair on one surface is one collision: {printed}"
    );
    assert!(
        printed.contains("collision: story:both story:other roles (inferred)\n"),
        "the one collision carries the weaker confidence behind it: {printed}"
    );
}

/// `scope --add` replaces an entry for a path already declared, "never doubled: two entries for one
/// surface would be a document saying two things about it". `roles/` is now the surface `roles`
/// is, so adding it replaces rather than doubles.
#[test]
fn scope_add_of_the_other_spelling_of_a_surface_replaces_the_entry() {
    let store = store_of("aep-scope-spelled-twice", &[("both", &["roles"])]);
    let at = printable(&store);
    make(&[
        "plan",
        "artifact",
        "scope",
        "story:both",
        "--add",
        "roles/",
        "--inferred",
        "--store",
        at,
    ]);
    let document = std::fs::read_to_string(store.join("story/both.md")).expect("the document");
    assert_eq!(
        document.matches("path: roles").count(),
        1,
        "one surface is one entry: {document}"
    );
}

/// Three stories nested inside one another are three pairs, each named with its narrower path, and
/// the packing puts each in its own wave.
#[test]
fn three_nested_scopes_are_three_collisions_in_id_order() {
    let store = store_of(
        "aep-waves-nested",
        &[
            ("a", &["roles"]),
            ("b", &["roles/x"]),
            ("c", &["roles/x/y.yaml"]),
        ],
    );
    let printed = make(&["plan", "artifact", "waves", "--store", printable(&store)]);
    assert!(
        printed.contains("wave 1\n  story:a\nwave 2\n  story:b\nwave 3\n  story:c\n"),
        "{printed}"
    );
    assert!(
        printed.contains(
            "collision: story:a story:b roles/x\n\
             collision: story:a story:c roles/x/y.yaml\n\
             collision: story:b story:c roles/x/y.yaml\n"
        ),
        "{printed}"
    );
}

/// The recorded answer, byte for byte.
///
/// A committed store with waves, a dependency across them, a cited collision, an inferred one and
/// an unassessed story — and the exact bytes the verb prints over it. A change to the derivation
/// that is not a deliberate re-recording fails here rather than in a reader's head.
#[test]
fn the_fixture_store_prints_the_recorded_answer_byte_for_byte() {
    let store = fixture().join("store");
    let at = printable(&store);

    for (recorded, format) in [("waves.text", "text"), ("waves.json", "json")] {
        let expected = std::fs::read_to_string(fixture().join("reads").join(recorded))
            .expect("the recording is committed");
        let output = aep(&[
            "plan", "artifact", "waves", "--format", format, "--store", at,
        ]);
        assert_eq!(code(&output), 0, "{}", stderr(&output));
        assert_eq!(
            stdout(&output),
            expected,
            "`waves --format {format}` no longer prints what {recorded} records"
        );
    }
}

/// A `..` component names a surface the entries alone cannot place, so the comparison cannot say
/// the two are disjoint: unknown is not false, and the pair is kept apart and reported.
#[test]
fn a_parent_component_cannot_be_compared_and_collides() {
    let store = store_of(
        "aep-waves-parent",
        &[("climbs", &["roles/../x.yaml"]), ("plain", &["x.yaml"])],
    );
    let printed = make(&["plan", "artifact", "waves", "--store", printable(&store)]);
    assert!(
        printed.contains("wave 1\n  story:climbs\nwave 2\n  story:plain\n"),
        "a path the comparison cannot place never shares a wave: {printed}"
    );
    assert!(
        printed.contains("collision: story:climbs story:plain roles/../x.yaml\n"),
        "the collision names the entry that could not be compared: {printed}"
    );
}

/// A glob is not a path: `roles/*` may match `roles/x.yaml`, so the pair is kept apart.
#[test]
fn a_glob_cannot_be_compared_and_collides() {
    let store = store_of(
        "aep-waves-glob",
        &[("pattern", &["roles/*"]), ("file", &["roles/x.yaml"])],
    );
    let printed = make(&["plan", "artifact", "waves", "--store", printable(&store)]);
    assert!(
        printed.contains("wave 1\n  story:file\nwave 2\n  story:pattern\n"),
        "a glob never shares a wave with a path it may match: {printed}"
    );
}

/// `scope --remove` takes a surface, so either spelling removes the entry for it.
#[test]
fn scope_remove_takes_either_spelling_of_a_surface() {
    let store = store_of("aep-scope-remove-spelling", &[("dir", &["roles/"])]);
    let at = printable(&store);
    make(&[
        "plan",
        "artifact",
        "scope",
        "story:dir",
        "--remove",
        "roles",
        "--store",
        at,
    ]);
    let document = std::fs::read_to_string(store.join("story/dir.md")).expect("the document");
    assert!(
        !document.contains("path: roles"),
        "removing `roles` removes `roles/`: {document}"
    );
}
