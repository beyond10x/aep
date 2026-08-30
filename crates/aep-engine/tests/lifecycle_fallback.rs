//! The lifecycle a tree falls back to when nothing nearer governs a kind.
//!
//! [`ArtifactKind`](aep_domain::artifact::ArtifactKind) is an open vocabulary: a team may name a
//! kind this crate has never heard of, and the engine accepts it. What used to happen next is that
//! the kind was governed by *nothing* — no lifecycle document could reach it, so every status was
//! legal and a misspelt one was not a refusal but a shrug. A lifecycle document with no `kind:` is
//! how a tree says what holds for the kinds nobody enumerated, and these tests are its reachability:
//! a tree on disk, loaded the way the CLI loads one, answering for a kind it never names.

use std::path::{Path, PathBuf};

use aep_domain::artifact::{
    Artifact, ArtifactGraph, ArtifactId, ArtifactKind, ArtifactLocation, ArtifactStatus,
};
use aep_project::load_tree_report;

/// Builds a throwaway document tree, holding only the lifecycles each test writes into it.
///
/// Rooted at `CARGO_TARGET_TMPDIR` rather than the system temporary directory: the tree is written
/// and read back within one test, and a `/tmp` that drops a write under pressure would show up as a
/// document the loader never saw rather than as the disk problem it is.
fn tree(name: &str, lifecycles: &[(&str, &str)]) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("lifecycle-fallback-{name}"));
    std::fs::remove_dir_all(&root).ok();
    let directory = root.join("artifacts/lifecycles");
    std::fs::create_dir_all(&directory).expect("the tree is writable");
    for (file, contents) in lifecycles {
        std::fs::write(directory.join(file), contents).expect("the document is writable");
    }
    root
}

/// A lifecycle for kinds a tree does name: `log`, and by lineage every `*-log`.
const LOG: &str = "\
kind: log
initial: draft
transitions:
  draft: [active]
  active: [archived]
";

/// The same document with its `kind:` taken off — the fallback.
const FALLBACK: &str = "\
initial: draft
transitions:
  draft: [proposed]
  proposed: [accepted]
";

/// A second one, which a tree may not have.
const SECOND_FALLBACK: &str = "\
initial: proposed
transitions:
  proposed: [rejected]
";

/// One ladder for the `digest` family: every `*-digest` a team invents is held to it.
const DIGEST: &str = "\
kind: digest
initial: draft
transitions:
  draft: [active]
  active: [archived]
";

/// A second family, with a deliberately different ladder — `briefing`.
const BRIEFING: &str = "\
kind: briefing
initial: proposed
transitions:
  proposed: [approved]
";

/// A third, different again — `insight`. Three ladders nobody upstream declared.
const INSIGHT: &str = "\
kind: insight
initial: draft
transitions:
  draft: [accepted]
";

fn artifact(id: &str, kind: &str, status: ArtifactStatus) -> Artifact {
    Artifact::new(
        ArtifactId::new(id).expect("artifact id"),
        ArtifactKind::parse(kind).expect("artifact kind"),
        status,
        ArtifactLocation::Inline,
    )
}

fn clean(root: &Path) {
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn a_kind_less_document_governs_every_kind_nothing_nearer_names() {
    let root = tree(
        "reachable",
        &[("log.yaml", LOG), ("fallback.yaml", FALLBACK)],
    );
    let outcome = load_tree_report(&root);
    assert!(
        outcome.failures.is_empty(),
        "a lifecycle document without `kind:` is the fallback, not a broken document: {}",
        outcome
            .failures
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    );

    let lifecycles = outcome.registry.lifecycles();
    assert_eq!(lifecycles.len(), 2, "one kinded lifecycle and one fallback");

    // The kind nobody registered, and the point of the whole mechanism.
    let digest = ArtifactKind::parse("weekly-digest").expect("kind");
    let governing = lifecycles
        .for_kind(&digest)
        .expect("an unregistered kind is governed by the fallback");
    assert_eq!(governing.initial, ArtifactStatus::Draft);
    assert!(governing.permits_transition(&ArtifactStatus::Draft, &ArtifactStatus::Proposed));
    assert!(
        !governing.permits(&ArtifactStatus::Active),
        "the fallback is a lifecycle somebody wrote, not a permissive one"
    );

    // Nearer wins: the lineage rule finds `log` before the fallback is consulted.
    let observation_log = ArtifactKind::parse("observation-log").expect("kind");
    assert_eq!(
        lifecycles.for_kind(&observation_log),
        lifecycles.for_kind_exact(&ArtifactKind::parse("log").expect("kind")),
        "a lifecycle registered on the parent kind beats the fallback"
    );

    clean(&root);
}

#[test]
fn without_the_fallback_document_the_same_kind_is_governed_by_nothing() {
    // The guard, broken: the same tree minus one file. If this still resolved, the test above
    // would be passing on the lineage rule rather than on the fallback.
    let root = tree("absent", &[("log.yaml", LOG)]);
    let outcome = load_tree_report(&root);
    assert!(outcome.failures.is_empty());

    let digest = ArtifactKind::parse("weekly-digest").expect("kind");
    assert!(outcome.registry.lifecycles().for_kind(&digest).is_none());
    assert!(outcome.registry.lifecycles().fallback().is_none());

    clean(&root);
}

#[test]
fn a_second_kind_less_document_is_one_refusal_and_the_first_still_stands() {
    let root = tree(
        "duplicate",
        &[
            ("a-fallback.yaml", FALLBACK),
            ("b-fallback.yaml", SECOND_FALLBACK),
        ],
    );
    let outcome = load_tree_report(&root);

    assert_eq!(
        outcome.failures.len(),
        1,
        "exactly one refusal, naming the second document: {}",
        outcome
            .failures
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    );
    let failure = outcome.failures[0].to_string();
    assert!(failure.contains("duplicate_declaration"), "{failure}");
    assert!(failure.contains("b-fallback.yaml"), "{failure}");

    // The refusal is not a silent overwrite: the file read first is still the fallback.
    let fallback = outcome
        .registry
        .lifecycles()
        .fallback()
        .expect("the first document registered");
    assert_eq!(fallback.initial, ArtifactStatus::Draft);

    clean(&root);
}

#[test]
fn the_fallback_makes_a_status_on_an_unregistered_kind_refusable() {
    // What the mechanism is for: a manifest entry in a status the tree does not declare stops
    // being legal by default.
    let root = tree("manifest", &[("fallback.yaml", FALLBACK)]);
    let registry = load_tree_report(&root).registry;

    let graph = ArtifactGraph::build([artifact(
        "digest:2026-w34",
        "weekly-digest",
        ArtifactStatus::Active,
    )])
    .expect("the graph is well formed");

    let errors = graph.validate_lifecycles(registry.lifecycles());
    assert_eq!(
        errors.len(),
        1,
        "the fallback declares no `active`, so this one status is refused: {errors}"
    );
    assert!(errors.contains(aep_domain::error::ValidationCode::UnknownState));

    clean(&root);
}

#[test]
fn a_family_of_custom_kinds_shares_the_ladder_its_last_segment_names() {
    // Three families this crate has never heard of, one ladder each, and **no fallback in the
    // tree at all** — so anything that resolves here resolved through the last-segment rule and
    // not through something global.
    let root = tree(
        "families",
        &[
            ("digest.yaml", DIGEST),
            ("briefing.yaml", BRIEFING),
            ("insight.yaml", INSIGHT),
        ],
    );
    let outcome = load_tree_report(&root);
    assert!(
        outcome.failures.is_empty(),
        "a lifecycle for a kind nobody upstream declared is an ordinary document: {}",
        outcome
            .failures
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    );

    let lifecycles = outcome.registry.lifecycles();
    assert_eq!(
        lifecycles.len(),
        3,
        "three ladders, and none of them a fallback"
    );
    assert!(
        lifecycles.fallback().is_none(),
        "no kind-less document, so nothing here can answer for everything"
    );

    for (member, family) in [
        ("weekly-digest", "digest"),
        ("monday-briefing", "briefing"),
        ("market-insight", "insight"),
    ] {
        let member_kind = ArtifactKind::parse(member).expect("artifact kind");
        let family_kind = ArtifactKind::parse(family).expect("artifact kind");
        assert!(
            lifecycles.for_kind_exact(&member_kind).is_none(),
            "nothing is registered for `{member}` itself, so the family rule is what answers"
        );
        assert_eq!(
            lifecycles.for_kind(&member_kind),
            lifecycles.for_kind_exact(&family_kind),
            "`{member}` is a `{family}` and shares its ladder"
        );
    }

    // The three ladders differ, so the equalities above could not have held by their being alike.
    assert_ne!(
        lifecycles.for_kind_exact(&ArtifactKind::parse("digest").expect("artifact kind")),
        lifecycles.for_kind_exact(&ArtifactKind::parse("briefing").expect("artifact kind")),
        "the fixture distinguishes the families it is asserting about"
    );
    // And the ladder is shared within a family rather than across the tree.
    assert!(
        lifecycles
            .for_kind(&ArtifactKind::parse("retrospective").expect("artifact kind"))
            .is_none(),
        "a kind in none of the three families is governed by none of their ladders"
    );

    clean(&root);
}

#[test]
fn the_family_ladder_wins_over_the_fallback_and_the_fallback_answers_last() {
    // The disambiguation, with both candidates in one tree: a `digest` ladder and a kind-less
    // fallback. `weekly-digest` could resolve either way, and the order is exact kind, then the
    // parent chain nearest ancestor first, then the fallback — so the ladder wins.
    let root = tree(
        "precedence",
        &[("digest.yaml", DIGEST), ("fallback.yaml", FALLBACK)],
    );
    let outcome = load_tree_report(&root);
    assert!(
        outcome.failures.is_empty(),
        "a kinded ladder and a fallback are one tree's ordinary contents: {}",
        outcome
            .failures
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    );

    let lifecycles = outcome.registry.lifecycles();
    let digest = ArtifactKind::parse("digest").expect("artifact kind");
    let weekly = ArtifactKind::parse("weekly-digest").expect("artifact kind");
    let fallback = lifecycles.fallback().expect("the tree declares one");

    // The fixture reached the state the rule is load-bearing in: both candidates exist, and they
    // are different documents, so whichever answers is visible in the answer.
    assert!(
        lifecycles.for_kind_exact(&digest).is_some(),
        "a `digest` ladder is registered"
    );
    assert_ne!(
        lifecycles.for_kind_exact(&digest),
        Some(fallback),
        "the ladder and the fallback are distinguishable"
    );
    assert!(
        lifecycles.for_kind_exact(&weekly).is_none(),
        "nothing is registered for `weekly-digest` itself, so this is the chain answering"
    );

    assert_eq!(
        lifecycles.for_kind(&weekly),
        lifecycles.for_kind_exact(&digest),
        "the nearest ancestor's ladder is nearer than the tree's fallback"
    );
    assert!(
        lifecycles
            .for_kind(&weekly)
            .expect("a ladder")
            .permits(&ArtifactStatus::Active),
        "and it is the digest ladder's rungs that govern, not the fallback's"
    );

    // A kind with nothing in its chain reaches the fallback, which is what the fallback is for.
    assert_eq!(
        lifecycles.for_kind(&ArtifactKind::parse("retrospective").expect("artifact kind")),
        Some(fallback),
        "nothing nearer answers for this one, so the fallback does"
    );

    clean(&root);
}
