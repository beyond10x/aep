//! `before_implementation` counts this task's specification, not the store's.
//!
//! The defect this holds shut was measured, not imagined. Run `NATIVE-1/1` moved
//! `establish_verifiers -> implement` in a store holding **zero** approvals of its own, because
//! `principles/development/spec-driven.yaml` asked for `kind: specification, status: approved`
//! with no relation — a query over every artifact there is — and two other stories' specifications
//! were approved in the same store. The principle's own header says an approved specification must
//! exist before implementation *of this work*; the rule as written could not say "this work".
//!
//! These tests run the shipped documents, not a fixture of them: the principle file, the
//! `adp/default` workflow and the `development.fast` profile as this repository publishes them. A
//! binding that holds in a hand-built registry and not in `principles/` would protect nobody.

use std::path::{Path, PathBuf};

use aep_domain::artifact::{
    Artifact, ArtifactGraph, ArtifactId, ArtifactKind, ArtifactLocation, ArtifactRef,
    ArtifactStatus, RelationKind,
};
use aep_domain::predicate::Truth;
use aep_domain::task::Task;
use aep_engine::evaluate::{evaluate, Requirement, RequirementSource};
use aep_engine::{Engine, Execution, Snapshot};
use aep_project::load_tree_report;

/// The repository root, from this crate's manifest directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// The engine, over this repository's own document tree.
fn engine() -> Engine {
    Engine::new(
        load_tree_report(&root())
            .into_result()
            .expect("the document tree is valid"),
    )
}

/// The task under test: it says what work it is, the way every task document does.
///
/// `development.fast` because it is the smallest profile carrying `spec-driven`; the binding is a
/// property of the principle, and every profile that takes the principle takes it.
fn task() -> Task {
    aep_schema::parse::task(
        "id: AUTH-142\n\
         kind: feature\n\
         objective: add-passkey-support\n\
         protocol: adp/1\n\
         profile: development.fast\n\
         derived_from:\n  - story:AUTH-141\n",
        None,
    )
    .expect("the task parses")
}

/// An approved specification carrying the edge a driven run's `specify` state actually writes.
fn approved_specification(id: &str, story: &str) -> Artifact {
    Artifact::new(
        ArtifactId::new(id).expect("id"),
        ArtifactKind::Specification,
        ArtifactStatus::Approved,
        ArtifactLocation::Inline,
    )
    .with_relation(
        RelationKind::Specifies,
        ArtifactRef::parse(story).expect("a story reference"),
    )
}

/// A story for a specification to be about.
fn story(id: &str) -> Artifact {
    Artifact::new(
        ArtifactId::new(id).expect("id"),
        ArtifactKind::Story,
        ArtifactStatus::Active,
        ArtifactLocation::Inline,
    )
}

/// An execution of [`task`] parked in `establish_verifiers`, which is the state `implement` is
/// entered from and therefore the state the `before_implementation` obligations are owed in.
///
/// Restored rather than driven: walking the workflow here would need every other guard's evidence,
/// and none of it is what these tests are about.
fn parked(artifacts: ArtifactGraph) -> Execution {
    let snapshot: Snapshot = serde_json::from_value(serde_json::json!({
        "execution": "AUTH-142.1",
        "task": "AUTH-142",
        "state": "establish_verifiers",
        "entered": ["receive", "specify", "decompose", "establish_verifiers"],
        "evidence": [],
        "events": [],
        "next_seq": 1,
    }))
    .expect("the snapshot deserialises");
    engine()
        .restore(task(), artifacts, snapshot)
        .expect("the snapshot restores against the shipped documents")
}

/// The `spec-driven` row on the transition into `implement`.
fn spec_driven_row(execution: &Execution) -> Requirement {
    let evaluation = evaluate(execution);
    let transition = evaluation
        .transitions
        .iter()
        .find(|transition| transition.to.as_str() == "implement")
        .expect("`establish_verifiers` leads to `implement`");
    let rows: Vec<&Requirement> = transition
        .requirements
        .iter()
        .filter(|requirement| {
            matches!(
                &requirement.source,
                RequirementSource::Principle { principle, .. } if principle.as_str() == "spec-driven"
            ) && requirement.outcome.flavour == aep_domain::requirement::RequirementFlavour::Artifact
        })
        .collect();
    assert_eq!(
        rows.len(),
        1,
        "one line per requirement, and this is the one: {:?}",
        transition
            .requirements
            .iter()
            .map(Requirement::line)
            .collect::<Vec<_>>()
    );
    rows[0].clone()
}

#[test]
fn another_storys_approved_specification_does_not_open_implementation() {
    // The fixture has to hold what the defect held: an approved specification of the right kind,
    // in the graph, satisfying every clause of the old rule. Assert that before asserting the
    // refusal, or the refusal proves only that the graph was empty.
    let mut artifacts = ArtifactGraph::new();
    artifacts.insert(story("story:OTHER-9"));
    artifacts.insert(approved_specification(
        "specification:other-work",
        "story:OTHER-9",
    ));
    let present: Vec<String> = artifacts
        .of_kind(&ArtifactKind::Specification)
        .map(|artifact| format!("{} ({})", artifact.id, artifact.status))
        .collect();
    assert_eq!(
        present,
        vec!["specification:other-work (approved)".to_owned()],
        "the store holds an approved specification; the only thing wrong with it is whose it is"
    );

    let row = spec_driven_row(&parked(artifacts));
    assert_eq!(
        row.outcome.truth,
        Truth::Unknown,
        "unknown, never false: this task's specification has not been written yet, and waiting is \
         what produces one — {}",
        row.line()
    );
    let detail = row.outcome.detail.as_deref().expect("a reason");
    assert!(
        detail.contains("specification:other-work") && detail.contains("story:AUTH-141"),
        "the row names the specification that is there and the work this task declared: {detail}"
    );
}

#[test]
fn the_tasks_own_approved_specification_opens_implementation() {
    // The other half. Without it the test above would pass against a rule that refuses everything,
    // which is a different defect wearing the same green.
    let mut artifacts = ArtifactGraph::new();
    artifacts.insert(story("story:OTHER-9"));
    artifacts.insert(approved_specification(
        "specification:other-work",
        "story:OTHER-9",
    ));
    artifacts.insert(story("story:AUTH-141"));
    artifacts.insert(approved_specification(
        "specification:passkeys-auth",
        "story:AUTH-141",
    ));

    let row = spec_driven_row(&parked(artifacts));
    assert_eq!(
        row.outcome.truth,
        Truth::True,
        "the specification of the story this task declares satisfies it: {}",
        row.line()
    );
}

#[test]
fn a_task_that_declares_no_work_admits_no_storys_specification() {
    // A task document with no `derived_from` has said nothing about which story it implements, so
    // the only work it declares is itself. The store's approved specification is about a story
    // this task never named, and saying nothing must not be the way to get it counted — that is
    // the defect restored as a default.
    let silent = aep_schema::parse::task(
        "id: AUTH-142\n\
         kind: feature\n\
         objective: add-passkey-support\n\
         protocol: adp/1\n\
         profile: development.fast\n",
        None,
    )
    .expect("the task parses");

    let mut artifacts = ArtifactGraph::new();
    artifacts.insert(story("story:AUTH-141"));
    artifacts.insert(approved_specification(
        "specification:passkeys-auth",
        "story:AUTH-141",
    ));

    let snapshot: Snapshot = serde_json::from_value(serde_json::json!({
        "execution": "AUTH-142.1",
        "task": "AUTH-142",
        "state": "establish_verifiers",
        "entered": ["receive", "specify", "decompose", "establish_verifiers"],
        "evidence": [],
        "events": [],
        "next_seq": 1,
    }))
    .expect("the snapshot deserialises");
    let execution = engine()
        .restore(silent, artifacts, snapshot)
        .expect("the snapshot restores");

    let row = spec_driven_row(&execution);
    assert_eq!(row.outcome.truth, Truth::Unknown, "{}", row.line());
    let detail = row.outcome.detail.as_deref().expect("a reason");
    assert!(
        detail.contains("this task's work is task:AUTH-142"),
        "the row says what the task did declare, so the repair — name the story — is not a guess: \
         {detail}"
    );
}

#[test]
fn a_specification_naming_the_task_itself_counts_as_this_tasks() {
    // `specifies: task:AUTH-142` is the relationship the rule's own name describes, and an author
    // who writes it has said exactly what is being asked for. The `derived_from` route above is
    // what runs write; this one is what a reader would try, and refusing it would be the rule
    // disagreeing with itself.
    let mut artifacts = ArtifactGraph::new();
    artifacts.insert(approved_specification(
        "specification:passkeys-auth",
        "task:AUTH-142",
    ));

    let row = spec_driven_row(&parked(artifacts));
    assert_eq!(row.outcome.truth, Truth::True, "{}", row.line());
}
