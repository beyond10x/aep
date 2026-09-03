//! The two shapes of adoption in `docs/guide/adopting.md`, run as written.
//!
//! The guide tells an adopter two things that a reader cannot check by reading: that pointing at
//! somebody else's tree merges **principles and profiles and nothing else**, and that a team whose
//! work has a shape of its own owns a tree instead — the `protocols: .` form, whose last two lines
//! are load-bearing and easy to miss. Both were once implied rather than stated, and an adopter
//! found out by putting a workflow under `.engineering/workflows/` and watching nothing read it.
//!
//! These tests build each layout the guide prints, out of this repository's own vendored
//! documents, and load it the way every command does. A guide whose example does not load is a
//! defect in the guide.

use std::path::{Path, PathBuf};

use aep_domain::artifact::ArtifactKind;
use aep_domain::version::PrincipleRef;
use aep_project::project::load;

/// The directories the loader walks, and therefore the ones an owned tree vendors.
const VENDORED: &[&str] = &[
    "protocols",
    "principles",
    "workflows",
    "profiles",
    "artifacts/lifecycles",
    "drivers",
];

/// The guide's `protocols: .` example: the tree is `.engineering/` itself.
///
/// The profile is this repository's rather than the guide's fictional `acme.knowledge`, because a
/// project file naming a profile no document declares is refused — which is the guide's own point
/// about what a tree must contain, not a licence to skip it here.
const OWNED_TREE_PROJECT: &str = "\
version: aep.project/1
protocol: adp/1
profile: development.standard
protocols: .
principles: local/principles
profiles: local/profiles
";

/// The same file with the two load-bearing lines left off, which is the mistake the guide warns of.
const OWNED_TREE_WITHOUT_REDIRECTS: &str = "\
version: aep.project/1
protocol: adp/1
profile: development.standard
protocols: .
";

/// The other shape: a project pointing at a tree it does not own.
const POINTING_PROJECT: &str = "\
version: aep.project/1
protocol: adp/1
profile: development.standard
protocols: ../tree
";

/// A principle of the adopter's own — the kind of document pointing at a tree *does* merge.
const LOCAL_PRINCIPLE: &str = "\
id: knowledge-has-a-source
version: 1
title: Knowledge has a source
summary: >-
  A published claim names where it was read from, so a reader can check it without asking the
  person who wrote it.
capabilities:
  require_approval: [production.write]
";

/// A ladder for a kind of the adopter's own — the kind of document it does **not**.
const LOCAL_LIFECYCLE: &str = "\
kind: digest
initial: draft
transitions:
  draft: [active]
  active: [archived]
";

/// This repository's root, which is the tree every layout below vendors from.
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the repository root is readable")
}

/// Copies a document directory, recursively, the way an adopter vendoring by copy would.
fn copy_tree(from: &Path, into: &Path) {
    std::fs::create_dir_all(into).expect("the tree is writable");
    for entry in std::fs::read_dir(from).expect("the vendored directory is readable") {
        let entry = entry.expect("a directory entry");
        let source = entry.path();
        let target = into.join(entry.file_name());
        if source.is_dir() {
            copy_tree(&source, &target);
        } else {
            std::fs::copy(&source, &target).expect("the document is copyable");
        }
    }
}

/// An empty scratch repository, under the target directory rather than `/tmp`: these trees are
/// written and read back within one test, and a dropped write would read as a missing document.
fn scratch(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("adopting-guide-{name}"));
    std::fs::remove_dir_all(&root).ok();
    std::fs::create_dir_all(&root).expect("the tree is writable");
    root
}

/// Vendors this repository's documents into `tree`.
fn vendor(tree: &Path) {
    let repository = repository();
    for directory in VENDORED {
        copy_tree(&repository.join(directory), &tree.join(directory));
    }
}

/// Every profile document in a vendored tree — what the merge would read a second time.
fn profile_documents(tree: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(tree.join("profiles"))
        .expect("the vendored profiles are readable")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "yaml")
        })
        .collect();
    files.sort();
    files
}

#[test]
fn the_guides_owned_tree_example_loads_and_the_teams_own_ladder_is_in_force() {
    let root = scratch("owned");
    let engineering = root.join(".engineering");
    vendor(&engineering);
    // The document that could not be reached from a pointed-at tree at all, which is why the guide
    // says a lifecycle of your own means owning one.
    std::fs::write(
        engineering.join("artifacts/lifecycles/digest.yaml"),
        LOCAL_LIFECYCLE,
    )
    .expect("the document is writable");
    std::fs::write(engineering.join("project.yaml"), OWNED_TREE_PROJECT)
        .expect("the document is writable");

    let project = load(&root).unwrap_or_else(|errors| {
        panic!("the guide's `protocols: .` example must load as written: {errors}")
    });

    assert_eq!(
        project.paths.protocols, engineering,
        "the tree is `.engineering/` itself, which is what makes the two redirects necessary"
    );
    assert!(
        !project.paths.principles.exists() && !project.paths.profiles.exists(),
        "the merge paths point at directories the tree does not contain, as the guide says they may"
    );
    assert!(
        project
            .registry
            .protocol(&project.config.protocol)
            .is_some()
            && project.registry.profile(&project.config.profile).is_some(),
        "the vendored documents are the ones in force"
    );
    assert!(
        project
            .registry
            .lifecycles()
            .for_kind_exact(&ArtifactKind::parse("digest").expect("artifact kind"))
            .is_some(),
        "a ladder for a kind of the team's own is read, because the tree is theirs"
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn without_the_two_redirects_the_owned_tree_is_refused_exactly_as_the_guide_shows() {
    // The guard, broken the one way the guide predicts: leave `principles:` and `profiles:`
    // defaulted and the tree's own documents are read twice, once as the tree and once as the
    // project-local merge. If this loaded cleanly, the test above would be asserting nothing about
    // those two lines.
    let root = scratch("owned-without-redirects");
    let engineering = root.join(".engineering");
    vendor(&engineering);
    std::fs::write(
        engineering.join("project.yaml"),
        OWNED_TREE_WITHOUT_REDIRECTS,
    )
    .expect("the document is writable");

    let Err(errors) = load(&root) else {
        panic!("the tree's own documents are read twice, and a duplicate id is a refusal")
    };
    let failures: Vec<String> = errors.as_slice().iter().map(ToString::to_string).collect();

    // `principles/` holds only subdirectories and the merge does not recurse, so it is the profile
    // documents that come back twice — one refusal each, which is the count the guide prints.
    assert_eq!(
        errors.len(),
        profile_documents(&engineering).len(),
        "one refusal per profile document read twice: {}",
        failures.join("; ")
    );
    assert!(
        failures
            .iter()
            .all(|failure| failure.contains("duplicate_principle")),
        "every one of them is a duplicate declaration: {}",
        failures.join("; ")
    );
    assert!(
        failures.iter().any(|failure| {
            failure.contains("profiles/development-standard.yaml")
                && failure.contains("development.standard")
        }),
        "and the refusal names the file and the id, as the guide's console block does: {}",
        failures.join("; ")
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn pointing_at_a_tree_merges_principles_and_profiles_and_nothing_else() {
    // What `ProjectPaths` has always done, and what the guide now says: two of the six document
    // kinds are loaded over the tree's, and a lifecycle under `.engineering/` is not read at all.
    let root = scratch("pointing");
    let tree = root.join("tree");
    vendor(&tree);
    let engineering = root.join(".engineering");
    std::fs::create_dir_all(engineering.join("principles")).expect("the tree is writable");
    std::fs::create_dir_all(engineering.join("artifacts/lifecycles"))
        .expect("the tree is writable");
    std::fs::write(
        engineering.join("principles/knowledge-has-a-source.yaml"),
        LOCAL_PRINCIPLE,
    )
    .expect("the document is writable");
    std::fs::write(
        engineering.join("artifacts/lifecycles/digest.yaml"),
        LOCAL_LIFECYCLE,
    )
    .expect("the document is writable");
    std::fs::write(engineering.join("project.yaml"), POINTING_PROJECT)
        .expect("the document is writable");

    let project = load(&root).unwrap_or_else(|errors| {
        panic!("a project pointing at a tree, with a principle of its own: {errors}")
    });

    let local: PrincipleRef = "knowledge-has-a-source"
        .parse()
        .expect("principle reference");
    assert!(
        project.registry.principle(&local).is_some(),
        "a principle under `.engineering/principles/` is merged over the tree"
    );
    assert!(
        project
            .registry
            .lifecycles()
            .for_kind_exact(&ArtifactKind::parse("digest").expect("artifact kind"))
            .is_none(),
        "and a lifecycle beside it is not read at all — the file is right there, and pointing at a \
         tree does not reach it"
    );

    std::fs::remove_dir_all(&root).ok();
}
