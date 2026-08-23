//! `integrations/workflow-coverage.yaml` is checked, not read.
//!
//! # What this file is for
//!
//! The map says which shipped plugin surface teaches which workflow state, and — where none does —
//! names the gap. Both halves rot the same way: a workflow gains a state and the map keeps claiming
//! coverage of a machine that has changed shape underneath it, or a skill is renamed and the map
//! goes on pointing at a path that is not there. Neither would fail anything, which is what makes a
//! map written as prose worth less than no map at all.
//!
//! So every claim in that document is refused here **by name**. The rule that does the most work is
//! **totality**: every state of every workflow is either covered by a surface or named in a gap. A
//! map listing only what is covered goes green on the day a workflow grows a state, which is exactly
//! the day the claim stopped being true.
//!
//! # Why this test lives in `protocol-cli`
//!
//! It needs two things at once: the workflow parser, so a workflow is keyed by the id it **declares**
//! rather than by its filename (invariant 10), and a YAML reader for the map. This crate has both,
//! and it already carries the tests that hold two committed trees to each other —
//! `metaharness_frame_contract.rs` and `metaharness_contract_result.rs`. This is the same shape,
//! with both trees inside the repository.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use aep_domain::workflow::Workflow;

/// The repository root, from this crate's manifest directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// The map, as the repository ships it.
const MAP: &str = "integrations/workflow-coverage.yaml";

/// The map document as written. Deliberately its own types rather than a domain type: this is not a
/// protocol document kind, nothing in `protocol validate` reads it, and giving it a home in
/// `aep-domain` would publish a schema for a file the engine has no opinion about.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CoverageMap {
    format: String,
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    title: String,
    workflows: Vec<Entry>,
}

/// One workflow's row.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    workflow: String,
    document: String,
    #[serde(default)]
    covered_by: Vec<Covering>,
    #[serde(default)]
    gaps: Vec<Gap>,
}

/// A plugin surface, and the states it teaches.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Covering {
    surface: String,
    #[allow(dead_code)]
    harness: String,
    states: Vec<String>,
    teaches: String,
}

/// States nothing teaches, and why.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Gap {
    id: String,
    states: Vec<String>,
    reason: String,
}

/// The map, parsed.
fn map() -> CoverageMap {
    let path = root().join(MAP);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{MAP} is committed: {error}"));
    let parsed: CoverageMap = serde_yaml::from_str(&text)
        .unwrap_or_else(|error| panic!("{MAP} must parse as the map this test reads:\n{error}"));
    assert_eq!(
        parsed.format, "plugin-coverage/1",
        "{MAP} declares a format this test does not read"
    );
    parsed
}

/// Every workflow document in the tree, by the id it declares, with the path it was read from.
///
/// Keyed by declared id and never by filename, because invariant 10 says document identity comes
/// from document content — a map indexed by path would go green after a rename that changed which
/// machine the file describes.
fn workflows() -> BTreeMap<String, (PathBuf, Workflow)> {
    let mut found = BTreeMap::new();
    let base = root().join("workflows");
    let mut directories = vec![base.clone()];

    while let Some(directory) = directories.pop() {
        let entries = std::fs::read_dir(&directory)
            .unwrap_or_else(|_| panic!("{} is readable", directory.display()));
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                directories.push(path);
                continue;
            }
            if path.extension().and_then(|extension| extension.to_str()) != Some("yaml") {
                continue;
            }
            let relative = relative(&path);
            let text = std::fs::read_to_string(&path).expect("a workflow document is readable");
            let workflow = aep_schema::parse::workflow(&text, Some(&relative))
                .unwrap_or_else(|error| panic!("{relative} must parse as a workflow:\n{error}"));
            found.insert(workflow.id.to_string(), (path, workflow));
        }
    }

    assert!(
        !found.is_empty(),
        "no workflow document was found under `workflows/`; this test would then check nothing"
    );
    found
}

/// A path as the repository writes it — relative to the root, forward slashes.
fn relative(path: &Path) -> String {
    path.strip_prefix(root())
        .expect("the path is inside the repository")
        .to_string_lossy()
        .replace('\\', "/")
}

#[test]
fn every_shipped_workflow_has_a_row_in_the_coverage_map() {
    // The acceptance the story is written around: a workflow file added without a map entry turns
    // the gate red, and the message names the workflow rather than a count.
    let map = map();
    let declared: BTreeSet<String> = map
        .workflows
        .iter()
        .map(|entry| entry.workflow.clone())
        .collect();

    let mut unmapped: Vec<String> = Vec::new();
    for (id, (path, _)) in workflows() {
        if !declared.contains(&id) {
            unmapped.push(format!("  - {id} ({})", relative(&path)));
        }
    }

    assert!(
        unmapped.is_empty(),
        "{} workflow(s) with no entry in {MAP}. Add a row naming the plugin surface that teaches \
         each state, or a gap saying nothing does:\n{}",
        unmapped.len(),
        unmapped.join("\n")
    );
}

#[test]
fn every_map_entry_names_a_workflow_that_exists_at_the_path_it_gives() {
    // The converse, and the two halves of it are different defects: an entry for a workflow that
    // was deleted, and an entry whose `document:` no longer holds the id beside it.
    let workflows = workflows();
    let mut refusals: Vec<String> = Vec::new();

    for entry in map().workflows {
        let Some((path, _)) = workflows.get(&entry.workflow) else {
            refusals.push(format!(
                "  - `{}` is in the map and no document under `workflows/` declares it",
                entry.workflow
            ));
            continue;
        };
        let actual = relative(path);
        if actual != entry.document {
            refusals.push(format!(
                "  - `{}` says its document is `{}`; the file that declares it is `{actual}`",
                entry.workflow, entry.document
            ));
        }
    }

    assert!(
        refusals.is_empty(),
        "{} stale entr(ies) in {MAP}:\n{}",
        refusals.len(),
        refusals.join("\n")
    );
}

#[test]
fn every_plugin_surface_the_map_names_is_a_file_that_ships() {
    // The other acceptance: a map entry naming a skill path that is not there is refused by name.
    // Scoped to `integrations/` deliberately — this document is a claim about the plugin surface,
    // and a row pointing at a design document or a crate would be a claim about something else.
    let mut refusals: Vec<String> = Vec::new();

    for entry in map().workflows {
        for covering in entry.covered_by {
            let path = root().join(&covering.surface);
            if !covering.surface.starts_with("integrations/") {
                refusals.push(format!(
                    "  - `{}` (for `{}`) is outside `integrations/`; this map covers the plugin \
                     surface and nothing else",
                    covering.surface, entry.workflow
                ));
            } else if !path.is_file() {
                refusals.push(format!(
                    "  - `{}` (for `{}`) is named in the map and is not a file in this tree",
                    covering.surface, entry.workflow
                ));
            }
        }
    }

    assert!(
        refusals.is_empty(),
        "{} surface(s) in {MAP} that do not ship:\n{}",
        refusals.len(),
        refusals.join("\n")
    );
}

#[test]
fn every_state_the_map_names_is_a_state_its_workflow_declares() {
    // What stops the map surviving a rename. A row claiming coverage of `establish_verifier` — one
    // character off — would otherwise sit in the tree looking like an assertion.
    let workflows = workflows();
    let mut refusals: Vec<String> = Vec::new();

    for entry in map().workflows {
        let Some((_, workflow)) = workflows.get(&entry.workflow) else {
            continue; // reported by the test above; not reported twice here
        };
        let declared: BTreeSet<&str> = workflow
            .states
            .keys()
            .map(aep_domain::ids::StateId::as_str)
            .collect();

        let named = entry
            .covered_by
            .iter()
            .flat_map(|covering| {
                covering
                    .states
                    .iter()
                    .map(|state| (covering.surface.as_str(), state))
            })
            .chain(
                entry
                    .gaps
                    .iter()
                    .flat_map(|gap| gap.states.iter().map(|state| (gap.id.as_str(), state))),
            );

        for (source, state) in named {
            if !declared.contains(state.as_str()) {
                refusals.push(format!(
                    "  - `{}` names state `{state}` under `{source}`, and the workflow declares \
                     {declared:?}",
                    entry.workflow
                ));
            }
        }
    }

    assert!(
        refusals.is_empty(),
        "{} state name(s) in {MAP} that no workflow declares:\n{}",
        refusals.len(),
        refusals.join("\n")
    );
}

#[test]
fn every_state_of_every_workflow_is_either_covered_or_named_in_a_gap() {
    // The rule that makes the map worth having. Coverage stated only where it exists is an opinion:
    // it reads identically whether the uncovered states were considered and named, or never looked
    // at. This is also what turns "a workflow grew a state" into a red gate rather than a silent
    // widening of what the map is quiet about.
    let workflows = workflows();
    let mut refusals: Vec<String> = Vec::new();

    for entry in map().workflows {
        let Some((_, workflow)) = workflows.get(&entry.workflow) else {
            continue;
        };
        let covered: BTreeSet<&str> = entry
            .covered_by
            .iter()
            .flat_map(|covering| covering.states.iter().map(String::as_str))
            .collect();
        let gapped: BTreeSet<&str> = entry
            .gaps
            .iter()
            .flat_map(|gap| gap.states.iter().map(String::as_str))
            .collect();

        for state in workflow.states.keys() {
            let state = state.as_str();
            match (covered.contains(state), gapped.contains(state)) {
                (false, false) => refusals.push(format!(
                    "  - `{}` state `{state}` is neither covered by a surface nor named in a gap",
                    entry.workflow
                )),
                (true, true) => refusals.push(format!(
                    "  - `{}` state `{state}` is claimed as covered *and* named in a gap; the map \
                     cannot say both",
                    entry.workflow
                )),
                _ => {}
            }
        }
    }

    assert!(
        refusals.is_empty(),
        "{} state(s) {MAP} does not account for:\n{}",
        refusals.len(),
        refusals.join("\n")
    );
}

#[test]
fn every_row_says_something_a_reader_can_act_on() {
    // A gap with no reason and a coverage row with no `teaches` are the two ways this document
    // could satisfy every check above and still tell nobody anything. Cheap to assert, and the
    // failure mode is the one a map like this actually meets.
    let mut refusals: Vec<String> = Vec::new();

    for entry in map().workflows {
        if entry.covered_by.is_empty() && entry.gaps.is_empty() {
            refusals.push(format!(
                "  - `{}` has neither a covering surface nor a gap; it accounts for nothing",
                entry.workflow
            ));
        }
        for covering in &entry.covered_by {
            if covering.teaches.trim().is_empty() {
                refusals.push(format!(
                    "  - `{}` claims `{}` covers {:?} and does not say what it teaches",
                    entry.workflow, covering.surface, covering.states
                ));
            }
            if covering.states.is_empty() {
                refusals.push(format!(
                    "  - `{}` names `{}` as covering and lists no state",
                    entry.workflow, covering.surface
                ));
            }
        }
        for gap in &entry.gaps {
            if gap.reason.trim().is_empty() {
                refusals.push(format!(
                    "  - `{}` gap `{}` names no reason; a gap without one is a shrug",
                    entry.workflow, gap.id
                ));
            }
            if gap.states.is_empty() {
                refusals.push(format!(
                    "  - `{}` gap `{}` names no state",
                    entry.workflow, gap.id
                ));
            }
        }
    }

    assert!(
        refusals.is_empty(),
        "{} row(s) in {MAP} that assert nothing:\n{}",
        refusals.len(),
        refusals.join("\n")
    );
}

#[test]
fn the_map_is_refused_when_a_workflow_it_never_heard_of_appears() {
    // Verifying the guard by breaking it, in the shape `AGENTS.md` § *Conventions* asks for —
    // without touching the committed tree. The mutation is the one this story exists to catch: a
    // fifth workflow document lands and nobody updates the map.
    let declared: BTreeSet<String> = map()
        .workflows
        .into_iter()
        .map(|entry| entry.workflow)
        .collect();

    assert!(
        !declared.contains("adp/second-opinion"),
        "the fixture id must not be a real entry, or this test proves nothing"
    );

    let mut with_a_newcomer: BTreeSet<String> = workflows().into_keys().collect();
    with_a_newcomer.insert("adp/second-opinion".to_owned());

    let unmapped: Vec<&String> = with_a_newcomer
        .iter()
        .filter(|id| !declared.contains(*id))
        .collect();

    assert_eq!(
        unmapped,
        vec!["adp/second-opinion"],
        "the rule `every_shipped_workflow_has_a_row_in_the_coverage_map` applies must name exactly \
         the unmapped workflow"
    );
}
