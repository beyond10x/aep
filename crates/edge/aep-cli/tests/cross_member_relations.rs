//! A store writes the cross-member edge the workspace reads.
//!
//! `story:cross-member-relations-are-writable`. A workspace whose `.engineering/workspace.yaml`
//! declares `alpha` and `beta`: inside `alpha`, `relate`, `new --relate` and `unrelate` take a
//! target written `beta/<kind>:<name>`, the edge lands in the source's `relations`, and
//! `aep plan workspace crossings` reads it. Before this, `relate` refused it as "an edge to
//! nothing" and `new --relate` refused it as an identifier with a `/` in its kind, while
//! `validate` and `crossings` already read the very same edge when it was written by hand.
//!
//! Every fixture lives under `CARGO_TARGET_TMPDIR`, each member a project of its own, every run of
//! `aep` its own process from inside the member it writes to — which is how an owner runs it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The repository root, which is the protocol tree every member points at.
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// A relative path from one existing directory to another existing path.
///
/// A project file refuses an absolute `protocols:`, so each member's is computed from where the
/// fixture actually sits.
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

/// Which store a member keeps its plan in.
#[derive(Clone, Copy)]
enum Store {
    Git,
    Sqlite,
}

/// Two members, `alpha` and `beta`, each declaring both. `alpha` keeps its plan in `alpha_store`;
/// `beta` is always Markdown, so `crossings` can read it.
fn workspace(name: &str, alpha_store: Store) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("xmember-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    for (member, store) in [("alpha", alpha_store), ("beta", Store::Git)] {
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
        std::fs::write(
            engineering.join("workspace.yaml"),
            format!(
                "version: aep.workspace/1\nmembers:\n  - name: alpha\n    source: {}\n  \
                 - name: beta\n    source: {}\n",
                if member == "alpha" {
                    ".."
                } else {
                    "../../alpha"
                },
                if member == "beta" { ".." } else { "../../beta" },
            ),
        )
        .expect("workspace.yaml");
        if matches!(store, Store::Sqlite) {
            std::fs::remove_dir_all(engineering.join("planning")).expect("no files here");
        }
    }
    root
}

/// Runs `aep` from inside `member`, with no flag naming a store.
fn aep(member: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(args)
        .current_dir(member)
        // The fixture is not a Git repository; without a ceiling a target directory inside this
        // checkout would put it in this repository's work tree.
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

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Runs `aep` and asserts it exited 0, returning its standard output.
fn ok(member: &Path, args: &[&str]) -> String {
    let output = aep(member, args);
    assert_eq!(
        output.status.code(),
        Some(0),
        "`aep {}` failed:\n{}{}",
        args.join(" "),
        stdout(&output),
        stderr(&output)
    );
    stdout(&output)
}

/// Creates `story:<name>` in `member`.
fn story(member: &Path, name: &str) {
    ok(
        member,
        &["plan", "artifact", "new", "story", name, "--title", name],
    );
}

/// The edges `id` declares, as `show --format json` prints them, each `<relation> <target>`, sorted
/// so a count is a count and not an order.
fn edges_of(member: &Path, id: &str) -> Vec<String> {
    let shown = ok(
        member,
        &["plan", "artifact", "show", id, "--format", "json"],
    );
    let document: serde_json::Value = serde_json::from_str(&shown).expect("show prints JSON");
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

/// `crossings --format json` run from `member`.
fn crossings(member: &Path) -> serde_json::Value {
    let printed = ok(
        member,
        &["plan", "workspace", "crossings", "--format", "json"],
    );
    serde_json::from_str(&printed).expect("crossings prints JSON")
}

fn document(member: &Path, kind: &str, name: &str) -> String {
    std::fs::read_to_string(
        member
            .join(".engineering/planning")
            .join(kind)
            .join(format!("{name}.md")),
    )
    .expect("the document is readable")
}

#[test]
fn relate_writes_an_edge_to_another_members_story_and_crossings_lists_it_resolved() {
    let root = workspace("relate", Store::Git);
    let (alpha, beta) = (root.join("alpha"), root.join("beta"));
    story(&alpha, "a");
    story(&beta, "b");

    ok(
        &alpha,
        &[
            "plan",
            "artifact",
            "relate",
            "story:a",
            "depends_on",
            "beta/story:b",
        ],
    );

    let written = document(&alpha, "story", "a");
    assert!(
        written.contains("depends_on: beta/story:b"),
        "the crossing is in the source's frontmatter `relations`: {written}"
    );
    let report = crossings(&alpha);
    let listed = report["crossings"].as_array().expect("a list of crossings");
    assert_eq!(listed.len(), 1, "exactly the one crossing: {report}");
    assert_eq!(listed[0]["from"], "alpha/story:a");
    assert_eq!(listed[0]["relation"], "depends_on");
    assert_eq!(listed[0]["to"], "beta/story:b");
    assert_eq!(
        listed[0]["resolved"], true,
        "beta holds story:b, so the crossing resolves: {report}"
    );
    let validated = ok(&alpha, &["plan", "artifact", "validate"]);
    assert!(
        !validated.contains("undeclared_reference"),
        "validate accepts the store the verb wrote: {validated}"
    );
}

#[test]
fn a_crossing_moves_the_document_revision_exactly_as_a_local_edge_does() {
    let root = workspace("revision", Store::Git);
    let (alpha, beta) = (root.join("alpha"), root.join("beta"));
    story(&alpha, "local-edge");
    story(&alpha, "crossing-edge");
    story(&alpha, "local");
    story(&beta, "b");
    let revision_of = |name: &str| {
        document(&alpha, "story", name)
            .lines()
            .find(|line| line.starts_with("revision:"))
            .expect("the document has a revision")
            .to_owned()
    };

    ok(
        &alpha,
        &[
            "plan",
            "artifact",
            "relate",
            "story:local-edge",
            "depends_on",
            "story:local",
        ],
    );
    ok(
        &alpha,
        &[
            "plan",
            "artifact",
            "relate",
            "story:crossing-edge",
            "depends_on",
            "beta/story:b",
        ],
    );

    // An edge is a record of its own and does not move its source's revision
    // (`story:relation-bumps-a-document-revision-but-not-an-entity`). A crossing is an edge; that
    // it has to travel as an update does not make it a different change.
    assert_eq!(
        revision_of("crossing-edge"),
        revision_of("local-edge"),
        "a crossing leaves the revision where a local edge leaves it"
    );
}

#[test]
fn new_with_a_relate_naming_another_members_story_writes_the_edge_at_creation() {
    let root = workspace("new", Store::Git);
    let (alpha, beta) = (root.join("alpha"), root.join("beta"));
    story(&beta, "b");

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
            "depends_on:beta/story:b",
        ],
    );

    let written = document(&alpha, "story", "c");
    assert!(
        written.contains("depends_on: beta/story:b"),
        "the crossing given at creation is in the frontmatter: {written}"
    );
    let report = crossings(&alpha);
    let listed = report["crossings"].as_array().expect("a list of crossings");
    assert_eq!(listed.len(), 1, "{report}");
    assert_eq!(listed[0]["from"], "alpha/story:c");
    assert_eq!(listed[0]["resolved"], true, "{report}");
    ok(&alpha, &["plan", "artifact", "validate"]);
}

#[test]
fn unrelate_takes_back_an_edge_to_another_members_story() {
    let root = workspace("unrelate", Store::Git);
    let (alpha, beta) = (root.join("alpha"), root.join("beta"));
    story(&alpha, "a");
    story(&alpha, "local");
    story(&beta, "b");
    ok(
        &alpha,
        &[
            "plan",
            "artifact",
            "relate",
            "story:a",
            "depends_on",
            "story:local",
        ],
    );
    ok(
        &alpha,
        &[
            "plan",
            "artifact",
            "relate",
            "story:a",
            "depends_on",
            "beta/story:b",
        ],
    );

    ok(
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

    let written = document(&alpha, "story", "a");
    assert!(
        !written.contains("beta/story:b"),
        "the crossing is gone from the frontmatter: {written}"
    );
    assert!(
        written.contains("depends_on: story:local"),
        "the local edge beside it stays: {written}"
    );
    let report = crossings(&alpha);
    assert_eq!(
        report["crossings"].as_array().map(Vec::len),
        Some(0),
        "{report}"
    );
    ok(&alpha, &["plan", "artifact", "validate"]);
}

#[test]
fn a_target_naming_a_member_the_workspace_does_not_declare_is_refused() {
    let root = workspace("undeclared", Store::Git);
    let alpha = root.join("alpha");
    story(&alpha, "a");

    let related = aep(
        &alpha,
        &[
            "plan",
            "artifact",
            "relate",
            "story:a",
            "depends_on",
            "gamma/story:b",
        ],
    );
    assert_eq!(related.status.code(), Some(1), "{}", stdout(&related));
    assert!(
        stderr(&related).contains("`gamma`") && stderr(&related).contains("workspace.yaml"),
        "the refusal names the undeclared member and the file that declares members: {}",
        stderr(&related)
    );

    let created = aep(
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
            "depends_on:gamma/story:b",
        ],
    );
    assert_eq!(created.status.code(), Some(1), "{}", stdout(&created));
    assert!(
        stderr(&created).contains("`gamma`") && stderr(&created).contains("workspace.yaml"),
        "the refusal names the undeclared member and the file that declares members: {}",
        stderr(&created)
    );
    assert!(
        !alpha.join(".engineering/planning/story/c.md").exists(),
        "a refused `new` writes nothing"
    );
    assert!(
        !document(&alpha, "story", "a").contains("gamma"),
        "a refused `relate` writes nothing"
    );
}

#[test]
fn a_target_naming_this_stores_own_member_that_it_does_not_hold_is_refused() {
    let root = workspace("own-missing", Store::Git);
    let alpha = root.join("alpha");
    story(&alpha, "a");

    let related = aep(
        &alpha,
        &[
            "plan",
            "artifact",
            "relate",
            "story:a",
            "depends_on",
            "alpha/story:missing",
        ],
    );
    assert_eq!(related.status.code(), Some(1), "{}", stdout(&related));
    assert!(
        stderr(&related).contains("story:missing") && stderr(&related).contains("edge to nothing"),
        "`alpha/story:missing` is this store's own `story:missing`, refused as a dangling edge: {}",
        stderr(&related)
    );
    assert!(!document(&alpha, "story", "a").contains("missing"));
}

#[test]
fn a_target_another_checked_out_member_does_not_hold_is_refused_naming_its_store() {
    let root = workspace("other-missing", Store::Git);
    let (alpha, beta) = (root.join("alpha"), root.join("beta"));
    story(&alpha, "a");
    story(&beta, "b");

    let related = aep(
        &alpha,
        &[
            "plan",
            "artifact",
            "relate",
            "story:a",
            "depends_on",
            "beta/story:absent",
        ],
    );
    assert_eq!(related.status.code(), Some(1), "{}", stdout(&related));
    assert!(
        stderr(&related).contains("beta") && stderr(&related).contains("story:absent"),
        "the refusal names the member and the artifact it does not hold: {}",
        stderr(&related)
    );
    assert!(!document(&alpha, "story", "a").contains("absent"));
}

#[test]
fn a_target_in_a_member_nobody_checked_out_is_admitted() {
    let root = workspace("not-checked-out", Store::Git);
    let alpha = root.join("alpha");
    std::fs::remove_dir_all(root.join("beta")).expect("beta is not on this machine");
    story(&alpha, "a");

    ok(
        &alpha,
        &[
            "plan",
            "artifact",
            "relate",
            "story:a",
            "depends_on",
            "beta/story:b",
        ],
    );
    assert!(document(&alpha, "story", "a").contains("depends_on: beta/story:b"));
}

#[test]
fn a_crossing_that_closes_a_cycle_across_members_is_refused() {
    let root = workspace("cycle", Store::Git);
    let (alpha, beta) = (root.join("alpha"), root.join("beta"));
    story(&alpha, "a");
    story(&beta, "b");
    ok(
        &beta,
        &[
            "plan",
            "artifact",
            "relate",
            "story:b",
            "depends_on",
            "alpha/story:a",
        ],
    );

    let closing = aep(
        &alpha,
        &[
            "plan",
            "artifact",
            "relate",
            "story:a",
            "depends_on",
            "beta/story:b",
        ],
    );
    assert_eq!(
        closing.status.code(),
        Some(1),
        "the edge that closes the loop is refused: {}{}",
        stdout(&closing),
        stderr(&closing)
    );
    assert!(
        format!("{}{}", stdout(&closing), stderr(&closing)).contains("cycle"),
        "the refusal says it is a cycle: {}{}",
        stdout(&closing),
        stderr(&closing)
    );
    assert!(!document(&alpha, "story", "a").contains("beta/story:b"));
}

#[test]
fn a_crossing_in_a_sqlite_store_survives_reopening_once_and_is_taken_back() {
    let root = workspace("sqlite", Store::Sqlite);
    let (alpha, beta) = (root.join("alpha"), root.join("beta"));
    story(&alpha, "a");
    story(&alpha, "local");
    story(&beta, "b");
    ok(
        &alpha,
        &[
            "plan",
            "artifact",
            "relate",
            "story:a",
            "depends_on",
            "story:local",
        ],
    );
    ok(
        &alpha,
        &[
            "plan",
            "artifact",
            "relate",
            "story:a",
            "depends_on",
            "beta/story:b",
        ],
    );
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
            "depends_on:beta/story:b",
        ],
    );

    // Each `show` is a process of its own, so each one rebuilds the plan's documents from the
    // database: neither may drop the crossing or count it twice.
    for _ in 0..2 {
        assert_eq!(
            edges_of(&alpha, "story:a"),
            vec![
                "depends_on beta/story:b".to_owned(),
                "depends_on story:local".to_owned(),
            ],
            "the local edge and the crossing, each exactly once"
        );
        assert_eq!(
            edges_of(&alpha, "story:c"),
            vec!["depends_on beta/story:b".to_owned()],
            "the crossing given at creation, exactly once"
        );
    }
    ok(&alpha, &["plan", "artifact", "validate"]);

    ok(
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
    assert_eq!(
        edges_of(&alpha, "story:a"),
        vec!["depends_on story:local".to_owned()],
        "the crossing is gone and the local edge stays"
    );
    ok(&alpha, &["plan", "artifact", "validate"]);
}
