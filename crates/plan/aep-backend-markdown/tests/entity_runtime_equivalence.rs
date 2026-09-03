//! The other repository's reading of our ladders, checked from this side.
//!
//! `entity-runtime` carries every lifecycle document this repository ships as an entity definition
//! under its `examples/aep/`, and holds a committed copy of our `artifacts/lifecycles/*.yaml` to
//! check them against. That test runs in *their* gate, over *their* copy of *our* documents: it
//! tells us nothing on a day their copy is stale, which has happened — `vision.yaml` landed here
//! and their fixture stayed green about eight ladders while nine existed
//! (`entity-runtime/crates/entity-yaml/tests/fixtures/aep-lifecycles/PIN.md`, § *What the pin does
//! not do*).
//!
//! This is the mirror, and it is the evidence `story:entity-runtime-mapping` decides on. The
//! comparison is deliberately narrow and runs in **both directions** per kind:
//!
//! * every state our `transitions:` map names, their `lifecycle.states` names, and no other;
//! * the state our ladder starts in is the state their definition starts in;
//! * every `(from, to)` edge our map declares, their operations yield — and every edge their
//!   operations yield, our map declares. An edge invented there fails; a rung we grow and they do
//!   not express fails too.
//!
//! What it deliberately does **not** compare is the **operation names**. There is nothing here to
//! compare them against: our lifecycle documents declare target statuses only, and
//! `protocol artifact move --to <TO>` names a status, never a verb. `propose`, `activate`,
//! `implement` and the eight others in those files are their invention, they are read here only as
//! the value in an edge map, and this test endorses none of them — which is exactly what the
//! verdict in `story:entity-runtime-mapping` says. Phase 2 took the same line: the operations
//! `crate::kernel` builds are named for their target status, so no verb of theirs is on our wire.
//!
//! The definitions are read from a committed fixture (`tests/fixtures/entity-runtime-aep/`, pinned
//! at their tag `0.13.0`) rather than from a sibling checkout, for the reason their own pin gives:
//! a test whose coverage depends on which repositories happen to be beside it says a different
//! thing on every machine. Our side is read through `ArtifactLifecycle`, the type the store
//! actually decides moves with, so this compares what the code reads and not merely what the YAML
//! says.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use aep_domain::artifact::{ArtifactLifecycle, ArtifactStatus};
use sha2::{Digest, Sha256};

/// One edge of a ladder: the status it starts in, the status it ends in.
type Edge = (String, String);

/// The ladders that have no definition on the other side, and why one is allowed to.
///
/// `outbound-claim` landed here (`bba1a15`, `4d331a0`) after the commit their fixture pins, so no
/// definition for it exists there yet. `executable-system-specification` is the same case: the
/// kind had no ladder at all until this one, so their 0.13.0 fixture cannot hold a definition for
/// it. Naming them as a constant rather than filtering by absence is the point: a *fourteenth*
/// ladder growing here without a definition there fails
/// [`every_ladder_we_ship_has_a_definition_there_except_the_ones_named_here`] by name, instead of
/// being quietly skipped the way their fixture once skipped `vision`.
const LADDERS_WITHOUT_A_DEFINITION: &[&str] =
    &["outbound-claim", "executable-system-specification"];

/// Every `(from, to)` pair the eleven compared ladders declare, summed.
///
/// A total, on top of the per-kind comparison, because the per-kind tests iterate the kinds both
/// sides agree on: a whole ladder vanishing from both at once would leave them green. The count is
/// what the verdict cites, so it is written here where a change to it has to be deliberate.
const EDGES_COMPARED: usize = 77;

fn lifecycles_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../artifacts/lifecycles")
}

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/entity-runtime-aep")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

/// The `*.yaml` stems in a directory. `PIN.md` is prose and is not one of them.
fn stems_in(dir: &Path) -> BTreeSet<String> {
    fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", dir.display()))
        .map(|entry| entry.expect("a readable directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "yaml")
        })
        .map(|path| {
            path.file_stem()
                .expect("a yaml file has a stem")
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

/// Our ladders, keyed by the kind each one declares.
///
/// Read through `ArtifactLifecycle` — the type `Document::move_status` decides with — rather than
/// as raw YAML, so a document this repository's own parser reads differently from how it looks is
/// compared as it is read.
fn our_ladders() -> BTreeMap<String, ArtifactLifecycle> {
    let dir = lifecycles_dir();
    let mut ladders = BTreeMap::new();
    for stem in stems_in(&dir) {
        let path = dir.join(format!("{stem}.yaml"));
        let lifecycle: ArtifactLifecycle = serde_yaml::from_str(&read(&path))
            .unwrap_or_else(|error| panic!("{} parses: {error}", path.display()));
        let kind = lifecycle
            .kind
            .clone()
            .unwrap_or_else(|| panic!("{} declares its kind", path.display()));
        let previous = ladders.insert(kind.to_string(), lifecycle);
        assert!(previous.is_none(), "two ladders claim the kind `{kind}`");
    }
    assert!(!ladders.is_empty(), "{} holds ladders", dir.display());
    ladders
}

/// Their definition, reduced to the three things this equivalence is about.
///
/// Deliberately a local struct and not `entity_core::EntityDefinition`: this test asks whether the
/// *document* they publish says what our document says, and reading it through their parser would
/// make a change to their parser able to move the answer. `schema:`, `create:`, `emits:` and the
/// rest of a definition are ignored — unknown fields are dropped, which is what makes this a
/// reading of the parts under review rather than a second copy of their type.
#[derive(serde::Deserialize)]
struct Definition {
    /// The kind the definition claims to be, checked against the ladder's own `kind:`.
    entity: String,
    lifecycle: Lifecycle,
    #[serde(default)]
    operations: BTreeMap<String, Operation>,
}

#[derive(serde::Deserialize)]
struct Lifecycle {
    initial: String,
    states: Vec<String>,
}

#[derive(serde::Deserialize)]
struct Operation {
    #[serde(default)]
    transitions: Vec<Transition>,
}

#[derive(serde::Deserialize)]
struct Transition {
    from: From,
    to: String,
}

/// `from: draft` and `from: [approved, implemented]` are both legal there.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum From {
    One(String),
    Many(Vec<String>),
}

impl From {
    fn states(&self) -> Vec<&str> {
        match self {
            Self::One(state) => vec![state.as_str()],
            Self::Many(states) => states.iter().map(String::as_str).collect(),
        }
    }
}

fn their_definitions() -> BTreeMap<String, Definition> {
    let dir = fixture_dir();
    let mut definitions = BTreeMap::new();
    for stem in stems_in(&dir) {
        let path = dir.join(format!("{stem}.yaml"));
        let definition: Definition = serde_yaml::from_str(&read(&path))
            .unwrap_or_else(|error| panic!("{} parses: {error}", path.display()));
        definitions.insert(stem, definition);
    }
    assert!(
        !definitions.is_empty(),
        "{} holds definitions",
        dir.display()
    );
    definitions
}

/// The kinds compared: every ladder we ship that is not on the excused list.
fn compared_kinds() -> BTreeSet<String> {
    our_ladders()
        .into_keys()
        .filter(|kind| !LADDERS_WITHOUT_A_DEFINITION.contains(&kind.as_str()))
        .collect()
}

/// Every `(from, to)` pair our transitions map declares.
fn our_edges(lifecycle: &ArtifactLifecycle) -> BTreeSet<Edge> {
    lifecycle
        .transitions
        .iter()
        .flat_map(|(from, targets)| {
            targets
                .iter()
                .map(move |to| (from.as_str().to_owned(), to.as_str().to_owned()))
        })
        .collect()
}

/// Every `(from, to)` pair their operations yield, with the operation that yields it.
///
/// The operation name is carried only so a collision can be named. It is never compared with
/// anything of ours, because we publish no verb for it to be compared with.
fn their_edges(kind: &str, definition: &Definition) -> BTreeMap<Edge, String> {
    let mut edges: BTreeMap<Edge, String> = BTreeMap::new();
    for (name, operation) in &definition.operations {
        for transition in &operation.transitions {
            for from in transition.from.states() {
                let edge = (from.to_owned(), transition.to.clone());
                if let Some(previous) = edges.insert(edge.clone(), name.clone()) {
                    panic!("{kind}: {edge:?} is declared twice, by `{previous}` and by `{name}`");
                }
            }
        }
    }
    edges
}

#[test]
fn the_pinned_copy_is_the_bytes_this_pin_records() {
    let dir = fixture_dir();
    let pin = read(&dir.join("PIN.md"));
    let block = pin
        .split("```")
        .nth(1)
        .expect("PIN.md records its sums in a fenced block");

    let mut recorded: BTreeMap<String, String> = BTreeMap::new();
    for line in block.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let (sum, name) = line
            .split_once("  ")
            .unwrap_or_else(|| panic!("`{line}` is a `sha256sum` line"));
        recorded.insert(name.to_owned(), sum.to_owned());
    }

    let present: BTreeSet<String> = stems_in(&dir)
        .into_iter()
        .map(|stem| format!("{stem}.yaml"))
        .collect();
    let pinned: BTreeSet<String> = recorded.keys().cloned().collect();
    assert_eq!(
        pinned, present,
        "PIN.md and the fixture directory must name the same files — a sum with no file is a pin \
         of nothing, and a file with no sum is an unpinned copy beside a pinned one"
    );

    for (name, sum) in &recorded {
        let actual = Sha256::digest(read(&dir.join(name)).as_bytes())
            .iter()
            .fold(String::new(), |mut output, byte| {
                use std::fmt::Write as _;
                write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
                output
            });
        assert_eq!(
            &actual, sum,
            "{name} is not the bytes PIN.md records; refresh the pin deliberately or restore the \
             file"
        );
    }
}

#[test]
fn every_ladder_we_ship_has_a_definition_there_except_the_ones_named_here() {
    let ours: BTreeSet<String> = our_ladders().into_keys().collect();
    for excused in LADDERS_WITHOUT_A_DEFINITION {
        assert!(
            ours.contains(*excused),
            "`{excused}` is excused from the comparison and is not a ladder this repository ships \
             — a stale exclusion hides a kind rather than a gap"
        );
    }

    let theirs: BTreeSet<String> = their_definitions().into_keys().collect();
    assert_eq!(
        compared_kinds(),
        theirs,
        "the pinned definitions must cover every ladder we ship but the excused ones ({}) — a kind \
         on one side only is a ladder this comparison cannot see",
        LADDERS_WITHOUT_A_DEFINITION.join(", ")
    );
}

#[test]
fn each_definition_is_named_for_the_kind_whose_ladder_it_maps() {
    let ladders = our_ladders();
    let definitions = their_definitions();
    for kind in compared_kinds() {
        let ours = ladders[&kind]
            .kind
            .as_ref()
            .expect("a ladder keyed by kind declares one")
            .to_string();
        assert_eq!(definitions[&kind].entity, ours, "{kind}: entity name");
    }
}

#[test]
fn each_definition_starts_where_our_ladder_starts() {
    let ladders = our_ladders();
    let definitions = their_definitions();
    for kind in compared_kinds() {
        assert_eq!(
            definitions[&kind].lifecycle.initial,
            ladders[&kind].initial.as_str(),
            "{kind}: initial status"
        );
    }
}

#[test]
fn each_definition_declares_exactly_the_states_our_transitions_map_declares() {
    let ladders = our_ladders();
    let definitions = their_definitions();
    for kind in compared_kinds() {
        let ours: BTreeSet<&str> = ladders[&kind]
            .transitions
            .keys()
            .map(ArtifactStatus::as_str)
            .collect();
        let theirs: BTreeSet<&str> = definitions[&kind]
            .lifecycle
            .states
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(theirs, ours, "{kind}: states");
    }
}

/// The claim the verdict rests on, in both directions and with a total.
#[test]
fn each_definition_yields_exactly_the_edges_our_transitions_map_yields() {
    let ladders = our_ladders();
    let definitions = their_definitions();
    let mut compared = 0;

    for kind in compared_kinds() {
        let ours = our_edges(&ladders[&kind]);
        let theirs: BTreeSet<Edge> = their_edges(&kind, &definitions[&kind])
            .into_keys()
            .collect();

        let not_expressed: Vec<&Edge> = ours.difference(&theirs).collect();
        let not_in_the_ladder: Vec<&Edge> = theirs.difference(&ours).collect();
        assert!(
            not_expressed.is_empty() && not_in_the_ladder.is_empty(),
            "{kind}: the definition does not say what this repository's ladder says.\n  \
             in our ladder, not expressed there: {not_expressed:?}\n  \
             expressed there, not in our ladder: {not_in_the_ladder:?}"
        );
        compared += ours.len();
    }

    assert_eq!(
        compared, EDGES_COMPARED,
        "the two readings agree per kind but over {compared} edges rather than {EDGES_COMPARED} — \
         a ladder has left both sides at once, which the per-kind comparison cannot see"
    );
}
