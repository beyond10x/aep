//! Adversarial cases for `story:cross-member-relations-are-writable`.
//!
//! Each fixture is a workspace of Markdown or SQLite members under `CARGO_TARGET_TMPDIR`, every
//! member declaring every other, and every `aep` run is a process of its own from inside the member
//! it writes to.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let from = from.canonicalize().expect("the source directory exists");
    let to = to.canonicalize().expect("the target path exists");
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| left == right)
        .count();
    let mut relative = PathBuf::new();
    for _ in common..from.len() {
        relative.push("..");
    }
    for component in &to[common..] {
        relative.push(component.as_os_str());
    }
    relative
}

#[derive(Clone, Copy, PartialEq)]
enum Store {
    Git,
    Sqlite,
}

/// `members` each declaring all of `members`; the first one keeps its plan in `first_store`, the
/// rest are Markdown.
fn workspace(name: &str, members: &[&str], first_store: Store) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("xmember-adv-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    for (index, member) in members.iter().enumerate() {
        let store = if index == 0 { first_store } else { Store::Git };
        let engineering = root.join(member).join(".engineering");
        std::fs::create_dir_all(engineering.join("planning")).expect("a member store");
        let protocols = relative_path(&engineering, &repository());
        let selector = match store {
            Store::Git => "store:\n  git: {}\n".to_owned(),
            Store::Sqlite => "store:\n  sqlite:\n    path: plan.sqlite3\n".to_owned(),
        };
        std::fs::write(
            engineering.join("project.yaml"),
            format!(
                "version: aep.project/5\nplanning_scope: {member}\nprotocol: adp/1\n\
                 profile: development.standard\nprotocols: {}\n{selector}",
                protocols.display()
            ),
        )
        .expect("project.yaml");
        let mut declared = String::from("version: aep.workspace/1\nmembers:\n");
        for other in members {
            let source = if other == member {
                "..".to_owned()
            } else {
                format!("../../{other}")
            };
            writeln!(declared, "  - name: {other}\n    source: {source}")
                .expect("a String accepts a write");
        }
        std::fs::write(engineering.join("workspace.yaml"), declared).expect("workspace.yaml");
        if store == Store::Sqlite {
            std::fs::remove_dir_all(engineering.join("planning")).expect("no files here");
        }
    }
    root
}

fn aep(member: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(args)
        .current_dir(member)
        .env(
            "GIT_CEILING_DIRECTORIES",
            member
                .parent()
                .and_then(Path::parent)
                .expect("the fixture has a parent"),
        )
        .output()
        .expect("the protocol binary runs")
}

fn both(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn ok(member: &Path, args: &[&str]) -> String {
    let output = aep(member, args);
    assert_eq!(
        output.status.code(),
        Some(0),
        "`aep {}` failed:\n{}",
        args.join(" "),
        both(&output)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn story(member: &Path, name: &str) {
    ok(
        member,
        &["plan", "artifact", "new", "story", name, "--title", name],
    );
}

fn relate(member: &Path, id: &str, kind: &str, target: &str) {
    ok(member, &["plan", "artifact", "relate", id, kind, target]);
}

fn unrelate(member: &Path, id: &str, kind: &str, target: &str) {
    ok(member, &["plan", "artifact", "unrelate", id, kind, target]);
}

fn shown(member: &Path, id: &str) -> serde_json::Value {
    let printed = ok(
        member,
        &["plan", "artifact", "show", id, "--format", "json"],
    );
    serde_json::from_str(&printed).expect("show prints JSON")
}

fn edges_of(member: &Path, id: &str) -> Vec<String> {
    let document = shown(member, id);
    let mut edges: Vec<String> = document["relations"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|edge| {
            format!(
                "{} {}",
                edge["relation"].as_str().expect("a relation"),
                edge["target"].as_str().expect("a target")
            )
        })
        .collect();
    edges.sort();
    edges
}

fn document_path(member: &Path, kind: &str, name: &str) -> PathBuf {
    member
        .join(".engineering/planning")
        .join(kind)
        .join(format!("{name}.md"))
}

fn bytes(member: &Path, kind: &str, name: &str) -> Vec<u8> {
    std::fs::read(document_path(member, kind, name)).expect("the document is readable")
}

fn sorted(edges: &[&str]) -> Vec<String> {
    let mut edges: Vec<String> = edges.iter().map(|edge| (*edge).to_owned()).collect();
    edges.sort();
    edges
}

/// A cross-member cycle can stand in a workspace without any refusal: each side was written while
/// the other member was not checked out, which the unit admits by design. A **second, unrelated**
/// cycle of the same relation kind must still be refused when a crossing closes it.
#[test]
fn a_crossing_closing_a_new_cycle_is_refused_while_an_older_cross_member_cycle_stands() {
    let root = workspace("masked-cycle", &["alpha", "beta"], Store::Git);
    let (alpha, beta) = (root.join("alpha"), root.join("beta"));
    story(&alpha, "a");
    story(&alpha, "c");
    story(&beta, "b");
    story(&beta, "d");

    // The older cycle alpha/story:a <-> beta/story:b, each half written with the other side absent.
    let away = root.join("away");
    std::fs::rename(&beta, &away).expect("beta leaves this machine");
    relate(&alpha, "story:a", "depends_on", "beta/story:b");
    std::fs::rename(&away, &beta).expect("beta is back");
    std::fs::rename(&alpha, &away).expect("alpha leaves this machine");
    relate(&beta, "story:b", "depends_on", "alpha/story:a");
    std::fs::rename(&away, &alpha).expect("alpha is back");

    // A new, separate loop alpha/story:c -> beta/story:d -> alpha/story:c.
    relate(&beta, "story:d", "depends_on", "alpha/story:c");
    let before = bytes(&alpha, "story", "c");
    let closing = aep(
        &alpha,
        &[
            "plan",
            "artifact",
            "relate",
            "story:c",
            "depends_on",
            "beta/story:d",
        ],
    );
    assert_eq!(
        closing.status.code(),
        Some(1),
        "story:c -> beta/story:d closes a new cycle and must be refused: {}",
        both(&closing)
    );
    assert_eq!(
        bytes(&alpha, "story", "c"),
        before,
        "a refusal writes nothing"
    );
}

/// The cycle check guards crossings; a **local** edge can close the very same kind of loop through
/// another member, and the unit made that loop reachable through the CLI.
#[test]
fn a_local_edge_that_closes_a_cycle_through_another_member_is_refused() {
    let root = workspace("local-closes", &["alpha", "beta"], Store::Git);
    let (alpha, beta) = (root.join("alpha"), root.join("beta"));
    story(&alpha, "a");
    story(&alpha, "c");
    story(&beta, "b");
    relate(&alpha, "story:a", "depends_on", "beta/story:b");
    relate(&beta, "story:b", "depends_on", "alpha/story:c");

    let before = bytes(&alpha, "story", "c");
    let closing = aep(
        &alpha,
        &[
            "plan",
            "artifact",
            "relate",
            "story:c",
            "depends_on",
            "story:a",
        ],
    );
    assert_eq!(
        closing.status.code(),
        Some(1),
        "alpha/c -> alpha/a -> beta/b -> alpha/c is a cycle across members: {}",
        both(&closing)
    );
    assert_eq!(bytes(&alpha, "story", "c"), before);
}

/// Local edges, crossings to two members and two kinds to one target, added and removed in mixed
/// order: nothing but the edge named goes.
#[test]
fn mixed_local_edges_and_crossings_to_two_members_survive_mixed_order_unrelate() {
    for store in [Store::Git, Store::Sqlite] {
        let name = if store == Store::Git {
            "mixed-git"
        } else {
            "mixed-sqlite"
        };
        let root = workspace(name, &["alpha", "beta", "gamma"], store);
        let (alpha, beta, gamma) = (root.join("alpha"), root.join("beta"), root.join("gamma"));
        story(&alpha, "a");
        story(&alpha, "l1");
        story(&alpha, "l2");
        story(&beta, "b");
        story(&gamma, "g");

        relate(&alpha, "story:a", "depends_on", "story:l1");
        relate(&alpha, "story:a", "depends_on", "beta/story:b");
        relate(&alpha, "story:a", "informed_by", "beta/story:b");
        relate(&alpha, "story:a", "depends_on", "gamma/story:g");
        relate(&alpha, "story:a", "informed_by", "story:l2");
        unrelate(&alpha, "story:a", "depends_on", "story:l1");
        assert_eq!(
            edges_of(&alpha, "story:a"),
            sorted(&[
                "depends_on beta/story:b",
                "informed_by beta/story:b",
                "depends_on gamma/story:g",
                "informed_by story:l2",
            ]),
            "[{name}] removing a local edge keeps every crossing"
        );
        unrelate(&alpha, "story:a", "informed_by", "beta/story:b");
        assert_eq!(
            edges_of(&alpha, "story:a"),
            sorted(&[
                "depends_on beta/story:b",
                "depends_on gamma/story:g",
                "informed_by story:l2",
            ]),
            "[{name}] removing one kind to a target keeps the other kind and the other member"
        );
        unrelate(&alpha, "story:a", "depends_on", "gamma/story:g");
        relate(&alpha, "story:a", "depends_on", "story:l1");
        assert_eq!(
            edges_of(&alpha, "story:a"),
            sorted(&[
                "depends_on beta/story:b",
                "depends_on story:l1",
                "informed_by story:l2",
            ]),
            "[{name}] after mixed order"
        );
        ok(&alpha, &["plan", "artifact", "validate"]);
    }
}

/// A pinned crossing keeps its pin, and `unrelate` takes it back.
#[test]
fn a_crossing_pinned_to_a_version_keeps_its_pin_and_is_taken_back() {
    for store in [Store::Git, Store::Sqlite] {
        let name = if store == Store::Git {
            "pin-git"
        } else {
            "pin-sqlite"
        };
        let root = workspace(name, &["alpha", "beta"], store);
        let (alpha, beta) = (root.join("alpha"), root.join("beta"));
        story(&alpha, "a");
        story(&beta, "b");
        relate(&alpha, "story:a", "depends_on", "beta/story:b@1");
        assert_eq!(
            edges_of(&alpha, "story:a"),
            sorted(&["depends_on beta/story:b@1"]),
            "[{name}] the pin survives"
        );
        unrelate(&alpha, "story:a", "depends_on", "beta/story:b");
        assert_eq!(
            edges_of(&alpha, "story:a"),
            Vec::<String>::new(),
            "[{name}]"
        );
    }
}

/// `alpha/story:x` written inside `alpha` is `story:x`, for every verb.
#[test]
fn the_long_spelling_of_this_members_artifact_is_written_short_by_every_verb() {
    let root = workspace("own-long", &["alpha", "beta"], Store::Git);
    let alpha = root.join("alpha");
    story(&alpha, "a");
    story(&alpha, "local");
    relate(&alpha, "story:a", "depends_on", "alpha/story:local");
    let written = String::from_utf8(bytes(&alpha, "story", "a")).expect("utf-8");
    assert!(
        written.contains("depends_on: story:local") && !written.contains("alpha/story:local"),
        "relate writes the short spelling: {written}"
    );
    ok(
        &alpha,
        &[
            "plan",
            "artifact",
            "new",
            "story",
            "n",
            "--title",
            "N",
            "--relate",
            "depends_on:alpha/story:local",
        ],
    );
    let written = String::from_utf8(bytes(&alpha, "story", "n")).expect("utf-8");
    assert!(
        written.contains("depends_on: story:local") && !written.contains("alpha/story:local"),
        "new --relate writes the short spelling: {written}"
    );
    unrelate(&alpha, "story:a", "depends_on", "alpha/story:local");
    assert_eq!(edges_of(&alpha, "story:a"), Vec::<String>::new());
    ok(&alpha, &["plan", "artifact", "validate"]);
}

/// Invariant *Refusals change nothing*, to the byte, for every refusal a crossing can meet.
#[test]
fn every_refused_crossing_leaves_the_source_document_byte_identical() {
    let root = workspace("byte-identical", &["alpha", "beta"], Store::Git);
    let (alpha, beta) = (root.join("alpha"), root.join("beta"));
    story(&alpha, "a");
    story(&beta, "b");
    relate(&beta, "story:b", "depends_on", "alpha/story:a");
    let before = bytes(&alpha, "story", "a");
    for target in ["beta/story:absent", "gamma/story:b", "beta/story:b"] {
        let refused = aep(
            &alpha,
            &[
                "plan",
                "artifact",
                "relate",
                "story:a",
                "depends_on",
                target,
            ],
        );
        assert_eq!(
            refused.status.code(),
            Some(1),
            "{target}: {}",
            both(&refused)
        );
        assert_eq!(bytes(&alpha, "story", "a"), before, "{target} wrote bytes");
    }
    let refused = aep(
        &alpha,
        &[
            "plan",
            "artifact",
            "unrelate",
            "story:a",
            "depends_on",
            "beta/story:b",
        ],
    );
    assert_eq!(refused.status.code(), Some(1), "{}", both(&refused));
    assert_eq!(bytes(&alpha, "story", "a"), before);
}

/// The same crossing twice is one edge.
#[test]
fn relating_the_same_crossing_twice_declares_it_once() {
    for store in [Store::Git, Store::Sqlite] {
        let name = if store == Store::Git {
            "twice-git"
        } else {
            "twice-sqlite"
        };
        let root = workspace(name, &["alpha", "beta"], store);
        let (alpha, beta) = (root.join("alpha"), root.join("beta"));
        story(&alpha, "a");
        story(&beta, "b");
        relate(&alpha, "story:a", "depends_on", "beta/story:b");
        relate(&alpha, "story:a", "depends_on", "beta/story:b");
        assert_eq!(
            edges_of(&alpha, "story:a"),
            sorted(&["depends_on beta/story:b"]),
            "[{name}]"
        );
    }
}

/// In SQLite a crossing is an `UpdateEntity`, and the contract's conformance suite requires an
/// update to advance the entity revision by exactly one; a local edge is a relation record and
/// leaves it. Pinned so a change to either side is seen rather than drifting.
#[test]
fn in_sqlite_a_crossing_advances_the_revision_by_one_where_a_local_edge_does_not() {
    let root = workspace("sqlite-revision", &["alpha", "beta"], Store::Sqlite);
    let (alpha, beta) = (root.join("alpha"), root.join("beta"));
    story(&alpha, "local-edge");
    story(&alpha, "crossing-edge");
    story(&alpha, "local");
    story(&beta, "b");
    relate(&alpha, "story:local-edge", "depends_on", "story:local");
    relate(&alpha, "story:crossing-edge", "depends_on", "beta/story:b");
    let local = shown(&alpha, "story:local-edge")["revision"]
        .as_u64()
        .expect("a local edge's document has a numeric revision");
    let crossing = shown(&alpha, "story:crossing-edge")["revision"]
        .as_u64()
        .expect("a crossing's document has a numeric revision");
    assert_eq!(
        crossing,
        local + 1,
        "a crossing is one entity update, so it advances the revision by exactly one"
    );
}

/// A later update of the same artifact (a title change, a status move) neither drops a crossing nor
/// brings back one that was taken back.
#[test]
fn later_updates_neither_drop_a_crossing_nor_resurrect_a_removed_one() {
    for store in [Store::Git, Store::Sqlite] {
        let name = if store == Store::Git {
            "later-git"
        } else {
            "later-sqlite"
        };
        let root = workspace(name, &["alpha", "beta"], store);
        let (alpha, beta) = (root.join("alpha"), root.join("beta"));
        story(&alpha, "a");
        story(&beta, "b");
        story(&beta, "c");
        relate(&alpha, "story:a", "depends_on", "beta/story:b");
        relate(&alpha, "story:a", "depends_on", "beta/story:c");
        unrelate(&alpha, "story:a", "depends_on", "beta/story:c");
        ok(
            &alpha,
            &["plan", "artifact", "set", "story:a", "--title", "Renamed"],
        );
        assert_eq!(
            edges_of(&alpha, "story:a"),
            sorted(&["depends_on beta/story:b"]),
            "[{name}] after `set`"
        );
    }
}

/// `new` with a local edge and a crossing in a SQLite store: each exactly once, after reopening.
#[test]
fn new_with_a_local_edge_and_a_crossing_in_sqlite_holds_each_once() {
    let root = workspace("sqlite-new-mixed", &["alpha", "beta"], Store::Sqlite);
    let (alpha, beta) = (root.join("alpha"), root.join("beta"));
    story(&alpha, "local");
    story(&beta, "b");
    story(&beta, "b2");
    ok(
        &alpha,
        &[
            "plan",
            "artifact",
            "new",
            "story",
            "c",
            "--title",
            "C",
            "--relate",
            "depends_on:story:local",
            "--relate",
            "depends_on:beta/story:b",
            "--relate",
            "informed_by:beta/story:b2",
        ],
    );
    for _ in 0..2 {
        assert_eq!(
            edges_of(&alpha, "story:c"),
            sorted(&[
                "depends_on beta/story:b",
                "depends_on story:local",
                "informed_by beta/story:b2",
            ])
        );
    }
}
