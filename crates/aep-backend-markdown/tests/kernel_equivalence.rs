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

/// The ladders this repository ships, which govern both stores.
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
/// Everything a rung asks for: enough evidence, and an instant that satisfies any date guard.
///
/// The comparison here is about **which moves the ladder has**. What a rung *costs* — evidence,
/// a date — is a second axis with its own tests (`tests/evidence.rs`). Feeding each pair exactly
/// what its rung asks for holds the first axis still, so a disagreement here means the translation
/// moved rather than that a rung costs something.
///
/// The dates are chosen to satisfy the guard rather than read from anywhere: `after` gets a date
/// already past, `before` one still ahead. A rung guarded both ways on one key would be a ladder
/// that can never open, and this would say so by failing.
fn enough_for(lifecycle: &ArtifactLifecycle, to: &ArtifactStatus) -> kernel::OnHand {
    const NOW: &str = "2026-06-15";
    let mut dates = std::collections::BTreeMap::new();
    if let Some(guard) = lifecycle.timing_for(to) {
        if let Some(key) = &guard.after {
            dates.insert(key.clone(), "2026-01-01".to_owned());
        }
        if let Some(key) = &guard.before {
            dates.insert(key.clone(), "2026-12-31".to_owned());
        }
    }
    kernel::OnHand {
        evidence: lifecycle
            .requirements_for(to)
            .iter()
            .map(|requirement| (requirement.evidence, requirement.at_least))
            .collect(),
        now: Some(NOW.to_owned()),
        dates,
    }
}

/// Every ordered pair over the named vocabulary **and** the rungs this ladder invents.
///
/// `every_pair` alone covers only `ArtifactStatus::ALL`, which was every status there was until the
/// vocabulary opened. `obligation`'s rungs — `open`, `met`, `slipped` — are names no enum holds, so
/// comparing over `ALL` covered none of its moves and the vacuity guard said so. Union, not
/// replacement: the named statuses are still worth iterating, because most of the illegal moves a
/// ladder must refuse are named ones.
fn pairs_for(lifecycle: &ArtifactLifecycle) -> Vec<(ArtifactStatus, ArtifactStatus)> {
    let mut statuses: BTreeSet<ArtifactStatus> = ArtifactStatus::ALL.iter().cloned().collect();
    statuses.extend(lifecycle.statuses());
    statuses
        .iter()
        .flat_map(|from| {
            statuses
                .iter()
                .map(move |to| (from.clone(), to.clone()))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn agree_on_every_pair(kind: Option<&ArtifactKind>, lifecycle: &ArtifactLifecycle, label: &str) {
    let mut permitted = 0;
    let mut disagreements = Vec::new();
    for (from, to) in pairs_for(lifecycle) {
        let store = lifecycle.permits_transition(&from, &to);
        let kernel = kernel::decide(kind, lifecycle, &from, &to, &enough_for(lifecycle, &to))
            == kernel::Verdict::Permitted;
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
            kernel::decide(None, &permissive, &from, &to, &kernel::OnHand::default())
                == kernel::Verdict::Permitted,
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
        kernel::definition_for(Some(&custom), lifecycle)
            .expect("a lifecycle the document tree parsed is one the kernel reads")
            .entity,
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
        for (from, to) in pairs_for(lifecycle) {
            total += 1;
            if lifecycle.permits_transition(&from, &to) {
                legal += 1;
            }
        }
    }
    // Derived rather than written down. The number was `800` while eight ladders shipped, and the
    // day a ninth was added the gate failed on arithmetic instead of on the property this test is
    // about. What must not drift is the *coverage*, and
    // `every_ladder_this_repository_ships_is_named_by_the_fixture` is what holds that.
    // Derived, and no longer one number times another: a ladder that invents rungs has more pairs
    // than one that does not, which is what an open status vocabulary means for arithmetic.
    let expected: usize = covered_kinds()
        .iter()
        .map(|kind| pairs_for(registry.for_kind(kind).expect("a ladder")).len())
        .sum();
    assert_eq!(total, expected, "{} kinds", covered_kinds().len());

    // The tripwire the old literal `800` was by accident, kept on purpose and put where it bites.
    //
    // `every_pair` is `ArtifactStatus::ALL` squared, and `ALL` is no longer the whole vocabulary — a
    // status is now whatever a lifecycle document declares. So `ALL` is a list somebody could
    // shorten, and the derived `kinds * pairs` above cannot notice: both sides move together and
    // the comparison still passes over a smaller world.
    //
    // The floor is on `pairs`, deliberately not on `total`. A floor on the product is leaky in
    // exactly the case worth catching — `ALL` dropping 10 -> 9 while a tenth ladder is added gives
    // 810, which clears any 800 floor while the status vocabulary quietly shrank. Adding a ladder
    // never touches this line; removing a status does.
    assert!(
        every_pair().len() >= 100,
        "ArtifactStatus::ALL shrank: {} ordered pairs, so this compares a smaller vocabulary \
         than the one it was written against",
        every_pair().len()
    );
    assert!(
        legal * 4 < total,
        "{legal} of {total} moves are legal, which is too many for this to be a real comparison"
    );
}
