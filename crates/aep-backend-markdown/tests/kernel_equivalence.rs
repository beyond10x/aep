//! Phase 2's whole claim: the kernel refuses exactly what this repository already refuses.
//!
//! `Document::move_status` now asks `entity-core` whether a move is permitted, evaluating the
//! ladder as data instead of by a lookup written here. That is only safe if it changes no verdict,
//! and "no verdict" is not a thing a reviewer can check by reading — so it is checked here, by
//! exhaustion: every kind either store holds, every ladder this repository ships, and **every
//! ordered pair of statuses**, legal and illegal alike.
//!
//! The kinds come from `tests/fixtures/store-kinds.md`, committed rather than read from the stores
//! at test time. `agentic-principles` is a sibling checkout on one machine and nothing at all on
//! another; a test whose coverage depends on which is which is a test that says different things
//! in different places.
//!
//! What this does **not** claim: that the ladders are right, that the vocabulary should be open, or
//! that anything about the protocol changed. Only that the decision moved without moving with it.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use aep_backend_markdown::kernel;
use aep_domain::artifact::{ArtifactKind, ArtifactLifecycle, ArtifactStatus, LifecycleRegistry};

/// The eight ladders this repository ships, which govern both stores.
fn lifecycles_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/lifecycles")
}

fn fixture() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/store-kinds.md");
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

/// The kinds the fixture's fenced block lists.
fn covered_kinds() -> Vec<ArtifactKind> {
    let text = fixture();
    let block = text
        .split("```")
        .nth(1)
        .expect("the fixture lists its kinds in a fenced block");
    block
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|name| {
            ArtifactKind::from_str(name)
                .unwrap_or_else(|error| panic!("`{name}` is an artifact kind: {error}"))
        })
        .collect()
}

fn registry() -> LifecycleRegistry {
    let dir = lifecycles_dir();
    let mut registry = LifecycleRegistry::new();
    let mut found = 0;
    for entry in fs::read_dir(&dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display())) {
        let path = entry.expect("a readable directory entry").path();
        if path.extension().is_none_or(|extension| extension != "yaml") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("a readable lifecycle");
        let lifecycle: ArtifactLifecycle = serde_yaml::from_str(&text)
            .unwrap_or_else(|error| panic!("{} parses: {error}", path.display()));
        let kind = lifecycle
            .kind
            .clone()
            .unwrap_or_else(|| panic!("{} declares its kind", path.display()));
        registry.insert(kind, lifecycle);
        found += 1;
    }
    assert!(found >= 8, "{found} ladders read from {}", dir.display());
    registry
}

/// Every ordered pair of statuses, legal and illegal — a hundred per kind.
fn every_pair() -> Vec<(ArtifactStatus, ArtifactStatus)> {
    ArtifactStatus::ALL
        .iter()
        .flat_map(|from| {
            ArtifactStatus::ALL
                .iter()
                .map(move |to| (from.clone(), to.clone()))
        })
        .collect()
}

/// Asserts the two readings agree, and reports what they disagreed about rather than that they
/// disagreed.
fn agree_on_every_pair(kind: Option<&ArtifactKind>, lifecycle: &ArtifactLifecycle, label: &str) {
    let mut permitted = 0;
    let mut disagreements = Vec::new();
    for (from, to) in every_pair() {
        let store = lifecycle.permits_transition(&from, &to);
        let kernel = kernel::permits_transition(kind, lifecycle, &from, &to);
        if store {
            permitted += 1;
        }
        if store != kernel {
            disagreements.push(format!(
                "{} -> {}: the ladder says {store}, the kernel says {kernel}",
                from.as_str(),
                to.as_str()
            ));
        }
    }
    assert!(
        disagreements.is_empty(),
        "{label}: {}",
        disagreements.join("; ")
    );
    // A ladder that permits nothing would make agreement vacuous.
    assert!(permitted > 0, "{label}: the ladder permits no move at all");
}

#[test]
fn the_kernel_and_the_ladder_agree_on_every_move_of_every_kind_either_store_holds() {
    let registry = registry();
    let kinds = covered_kinds();
    assert!(kinds.len() >= 8, "the fixture covers {} kinds", kinds.len());

    for kind in &kinds {
        let lifecycle = registry
            .for_kind(kind)
            .unwrap_or_else(|| panic!("`{}` has a ladder", kind.as_str()));
        agree_on_every_pair(Some(kind), lifecycle, kind.as_str());
    }
}

/// The fixture is coverage, so a kind with a ladder and no line in it is a hole nobody would see.
#[test]
fn every_ladder_this_repository_ships_is_named_by_the_fixture() {
    let shipped: BTreeSet<String> = fs::read_dir(lifecycles_dir())
        .expect("readable")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "yaml")
        })
        .map(|path| {
            path.file_stem()
                .expect("a stem")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    let covered: BTreeSet<String> = covered_kinds()
        .iter()
        .map(|kind| kind.as_str().to_owned())
        .collect();
    assert_eq!(
        shipped, covered,
        "artifacts/lifecycles/ and tests/fixtures/store-kinds.md must name the same kinds"
    );
}

/// The shrug. A kind with no ladder anywhere in its lineage is handed
/// `ArtifactLifecycle::permissive`, which permits every move — including the ones a real ladder
/// forbids. The kernel has to shrug identically, or `protocol artifact move` starts refusing
/// `runbook` for a ladder nobody wrote.
#[test]
fn the_permissive_fallback_still_permits_every_move() {
    let permissive = ArtifactLifecycle::permissive();
    for (from, to) in every_pair() {
        assert!(
            permissive.permits_transition(&from, &to),
            "the ladder's own reading changed"
        );
        assert!(
            kernel::permits_transition(None, &permissive, &from, &to),
            "permissive refused {} -> {}",
            from.as_str(),
            to.as_str()
        );
    }
}

/// A custom kind reaches a ladder through `ArtifactKind::parent`, so the kernel must be handed the
/// lifecycle `for_kind` resolved and not one it looked up by name — and must still name the custom
/// kind in what it builds.
#[test]
fn a_custom_kind_is_governed_by_the_ladder_its_lineage_reaches() {
    let registry = registry();
    let custom = ArtifactKind::from_str("feature-design").expect("a kind");
    let lifecycle = registry
        .for_kind(&custom)
        .expect("feature-design reaches design's ladder through its lineage");

    assert_eq!(
        kernel::definition_for(Some(&custom), lifecycle).entity,
        "feature-design",
        "the refusal must name the kind that was moved, not the ladder that governs it"
    );
    agree_on_every_pair(Some(&custom), lifecycle, "feature-design");
}

/// The comparison is only worth running if the ladders actually refuse things. Ninety of the
/// hundred pairs being illegal is what makes "the same verdict" a claim rather than a tautology.
#[test]
fn the_ladders_refuse_most_moves_so_agreement_is_not_vacuous() {
    let registry = registry();
    let mut legal = 0;
    let mut total = 0;
    for kind in covered_kinds() {
        let lifecycle = registry.for_kind(&kind).expect("a ladder");
        for (from, to) in every_pair() {
            total += 1;
            if lifecycle.permits_transition(&from, &to) {
                legal += 1;
            }
        }
    }
    assert_eq!(total, 800, "eight kinds, a hundred pairs each");
    assert!(
        legal * 4 < total,
        "{legal} of {total} moves are legal, which is too many for this to be a real comparison"
    );
}
