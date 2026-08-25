//! Gap-register `:39`: a story's `implemented` is a claim nothing checks.
//!
//! It is checked now, when a ladder asks. A lifecycle document may declare what reaching a status
//! costs, and the move is decided by `entity-core` against evidence the caller presents — three
//! outcomes, not two.
//!
//! The third is the point and it is why three-valued rules were built first. *Nobody presented any
//! evidence* and *evidence was presented and there is not enough of it* are different facts, and a
//! store that reported both as "refused" would send an author to argue about a test result that was
//! never produced.

use std::collections::BTreeMap;

use aep_backend_markdown::kernel::{self, EvidenceOnHand, Verdict};
use aep_domain::artifact::{ArtifactKind, ArtifactLifecycle, ArtifactStatus};
use aep_domain::evidence::EvidenceKind;

/// A four-rung ladder where `implemented` costs one test result.
fn guarded() -> ArtifactLifecycle {
    serde_yaml::from_str(
        "kind: story\n\
         initial: draft\n\
         transitions:\n  \
           draft: [active]\n  \
           active: [implemented, archived]\n  \
           implemented: [archived]\n  \
           archived: []\n\
         requires:\n  \
           implemented:\n    \
             - evidence: test_result\n      \
               at_least: 1\n",
    )
    .expect("the fixture ladder parses")
}

fn on_hand(pairs: &[(EvidenceKind, usize)]) -> EvidenceOnHand {
    pairs.iter().map(|(kind, count)| (*kind, *count)).collect()
}

fn verdict(lifecycle: &ArtifactLifecycle, to: &str, evidence: &EvidenceOnHand) -> Verdict {
    kernel::decide(
        Some(&ArtifactKind::Story),
        lifecycle,
        &ArtifactStatus::Active,
        &ArtifactStatus::parse(to).expect("a status"),
        evidence,
    )
}

/// The three outcomes, on one ladder, from one starting rung.
#[test]
fn a_guarded_rung_tells_nobody_looked_from_it_does_not_hold() {
    let ladder = guarded();

    // 1. Nobody presented evidence of that kind. The rule could not be evaluated, and the refusal
    //    names the address nothing supplied rather than claiming the requirement failed.
    match verdict(&ladder, "implemented", &on_hand(&[])) {
        Verdict::EvidenceUnobserved {
            unobserved,
            message,
        } => {
            assert_eq!(unobserved, vec!["$args.evidence.test_result".to_owned()]);
            assert!(message.contains("test_result"), "{message}");
            assert!(message.contains("implemented"), "{message}");
        }
        other => panic!("expected the unobserved verdict, got {other:?}"),
    }

    // 2. Evidence of that kind was presented, and there is not enough of it. A different fact, and
    //    a different sentence.
    assert!(
        matches!(
            verdict(
                &ladder,
                "implemented",
                &on_hand(&[(EvidenceKind::TestResult, 0)])
            ),
            Verdict::EvidenceInsufficient { .. }
        ),
        "zero records presented is an observation, not an absence"
    );

    // 3. Enough of it.
    assert_eq!(
        verdict(
            &ladder,
            "implemented",
            &on_hand(&[(EvidenceKind::TestResult, 1)])
        ),
        Verdict::Permitted
    );
}

/// A requirement guards the rung it is declared on and nothing else. `archived` is reachable from
/// `active` on the same ladder and costs nothing.
#[test]
fn a_requirement_guards_only_the_rung_it_is_declared_on() {
    assert_eq!(
        verdict(&guarded(), "archived", &on_hand(&[])),
        Verdict::Permitted,
        "archiving is not gated by the evidence `implemented` needs"
    );
}

/// Evidence cannot buy a move the ladder does not have. The two checks are in the right order:
/// the rung first, the cost second.
#[test]
fn evidence_does_not_buy_a_move_the_ladder_does_not_declare() {
    let mut plenty = BTreeMap::new();
    plenty.insert(EvidenceKind::TestResult, 99);
    assert_eq!(
        kernel::decide(
            Some(&ArtifactKind::Story),
            &guarded(),
            &ArtifactStatus::Draft,
            &ArtifactStatus::Implemented,
            &plenty,
        ),
        Verdict::NotOnTheLadder,
        "draft does not reach implemented at any price"
    );
}

/// The property that keeps `tests/kernel_equivalence.rs` meaningful: with no `requires`, a ladder
/// cannot reach either evidence verdict, so comparing against `permits_transition` — which knows
/// nothing about evidence — is still comparing like with like.
#[test]
fn a_ladder_that_requires_nothing_behaves_exactly_as_it_did() {
    let plain: ArtifactLifecycle = serde_yaml::from_str(
        "kind: story\ninitial: draft\ntransitions:\n  draft: [active]\n  active: []\n",
    )
    .expect("parses");
    assert!(plain.requires.is_empty());

    for evidence in [on_hand(&[]), on_hand(&[(EvidenceKind::TestResult, 5)])] {
        assert_eq!(
            kernel::decide(
                Some(&ArtifactKind::Story),
                &plain,
                &ArtifactStatus::Draft,
                &ArtifactStatus::Active,
                &evidence,
            ),
            Verdict::Permitted
        );
    }
}

/// What the shipped ladders actually cost, asserted rather than assumed.
///
/// This replaces a tripwire that asserted no ladder required anything — which was true until the
/// day `story` gained a requirement, and whose whole job was to make that day loud rather than let
/// `kernel_equivalence.rs` fail looking like a translation defect. It did its job; the equivalence
/// test now feeds each rung exactly what it asks for, and this records the answer instead.
#[test]
fn the_shipped_ladders_cost_what_this_repository_thinks_they_cost() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/lifecycles");
    let mut guarded: Vec<String> = Vec::new();
    let mut read = 0;
    for entry in std::fs::read_dir(&dir).expect("readable") {
        let path = entry.expect("entry").path();
        if path.extension().is_none_or(|kind| kind != "yaml") {
            continue;
        }
        let lifecycle: ArtifactLifecycle =
            serde_yaml::from_str(&std::fs::read_to_string(&path).expect("readable"))
                .unwrap_or_else(|error| panic!("{} parses: {error}", path.display()));
        read += 1;
        for (status, requirements) in &lifecycle.requires {
            for requirement in requirements {
                guarded.push(format!(
                    "{}: {} needs {} {}",
                    path.file_stem().expect("a stem").to_string_lossy(),
                    status.as_str(),
                    requirement.at_least,
                    requirement.evidence.as_str()
                ));
            }
        }
    }
    guarded.sort();
    assert!(read >= 9, "{read} ladders read");
    assert_eq!(
        guarded,
        vec!["story: implemented needs 1 test_result".to_owned()],
        "a rung gaining or losing a cost is a change to what everybody's `move` costs, and it \
         should be seen here first"
    );
}
