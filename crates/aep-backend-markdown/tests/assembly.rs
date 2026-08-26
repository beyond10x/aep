//! Several stores read as one graph.
//!
//! Written against real directories under `CARGO_TARGET_TMPDIR` rather than `/tmp`: the tmpfs on at
//! least one machine here drops writes under pressure, and a store test that loses a file tests
//! nothing while looking like it passed.

use std::path::{Path, PathBuf};

use aep_backend_markdown::assembly::Assembly;
use aep_domain::artifact::ArtifactId;
use aep_domain::workspace::{MemberName, Resolution, WorkspaceRef};

/// A scratch directory of this name, emptied first so a rerun is a fresh read.
fn scratch(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("assembly")
        .join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch directory");
    root
}

/// Writes a planning document carrying one relation.
fn write_with_relation(root: &Path, id: &str, title: &str, relation: &str, target: &str) {
    let id = ArtifactId::new(id).expect("a valid id");
    let directory = root.join(id.namespace());
    std::fs::create_dir_all(&directory).expect("the kind's directory");
    std::fs::write(
        directory.join(format!("{}.md", id.name())),
        format!(
            "---\nformat: aep.planning-md/1\nid: {id}\nkind: {}\nstatus: draft\ntitle: {title}\nrelations:\n- {relation}: {target}\nrevision: 1\n---\n\n# {title}\n",
            id.namespace()
        ),
    )
    .expect("a document");
}

/// Writes one planning document into `root`, at the path its id determines.
fn write(root: &Path, id: &str, title: &str) {
    let id = ArtifactId::new(id).expect("a valid id");
    let directory = root.join(id.namespace());
    std::fs::create_dir_all(&directory).expect("the kind's directory");
    std::fs::write(
        directory.join(format!("{}.md", id.name())),
        format!(
            "---\nformat: aep.planning-md/1\nid: {id}\nkind: {}\nstatus: draft\ntitle: {title}\nrevision: 1\n---\n\n# {title}\n",
            id.namespace()
        ),
    )
    .expect("a document");
}

fn member(name: &str) -> MemberName {
    MemberName::parse(name).expect("a member name")
}

#[test]
fn an_artifact_keeps_the_member_it_came_from() {
    let one = scratch("one/planning");
    let two = scratch("two/planning");
    write(&one, "story:alpha", "Alpha");
    write(&two, "story:beta", "Beta");

    let assembly = Assembly::read([
        (member("one"), one.as_path()),
        (member("two"), two.as_path()),
    ]);

    assert_eq!(assembly.len(), 2);
    let held: Vec<String> = assembly
        .documents()
        .map(|(member, id, _)| format!("{member}/{id}"))
        .collect();
    assert_eq!(held, vec!["one/story:alpha", "two/story:beta"]);
}

#[test]
fn one_name_in_two_members_is_two_artifacts_and_an_unqualified_reference_to_it_is_refused() {
    // This is the case the whole namespacing story exists for. `story:passkey-login` is a name two
    // teams would each reach for, and resolving it to whichever store was read first would answer
    // the question with a fact about the other repository — with nothing in the output saying so.
    let one = scratch("shared-one/planning");
    let two = scratch("shared-two/planning");
    write(&one, "story:passkey-login", "Passkey login, here");
    write(&two, "story:passkey-login", "Passkey login, elsewhere");

    let assembly = Assembly::read([
        (member("one"), one.as_path()),
        (member("two"), two.as_path()),
    ]);

    assert_eq!(
        assembly.len(),
        2,
        "both are held; neither overwrote the other"
    );
    assert_eq!(assembly.shared_ids().len(), 1);

    let unqualified = WorkspaceRef::parse("story:passkey-login").expect("parses");
    match assembly.resolve(&unqualified) {
        Resolution::Ambiguous(members) => assert_eq!(members.len(), 2),
        other => panic!("expected ambiguous, got {other:?}"),
    }
    assert!(
        assembly.get(&unqualified).is_none(),
        "an ambiguous reference yields no document, because returning one would be the guess"
    );

    let qualified = WorkspaceRef::parse("two/story:passkey-login").expect("parses");
    let (found_in, document) = assembly
        .get(&qualified)
        .expect("a qualified reference resolves");
    assert_eq!(found_in.as_str(), "two");
    assert_eq!(
        document.document.frontmatter.title.as_deref(),
        Some("Passkey login, elsewhere")
    );
}

#[test]
fn a_member_whose_store_is_not_there_reads_as_empty_rather_than_failing() {
    // A workspace is read on machines that checked out different subsets of it.
    let one = scratch("absent-one/planning");
    write(&one, "story:alpha", "Alpha");
    let missing = Path::new(env!("CARGO_TARGET_TMPDIR")).join("assembly/absent-two/planning");

    let assembly = Assembly::read([
        (member("one"), one.as_path()),
        (member("two"), missing.as_path()),
    ]);

    assert_eq!(assembly.len(), 1);
    assert_eq!(
        assembly.members().len(),
        2,
        "the member is present and empty, not dropped"
    );
    assert!(assembly.failures().is_empty(), "absent is not a failure");
}

#[test]
fn a_broken_file_is_reported_against_the_member_that_holds_it() {
    // An assembly that answered from two members when asked about three would give a smaller answer
    // that looks exactly like a complete one, so a failure carries the member it came from.
    let one = scratch("broken-one/planning");
    let two = scratch("broken-two/planning");
    write(&one, "story:alpha", "Alpha");
    std::fs::create_dir_all(two.join("story")).expect("a kind directory");
    std::fs::write(two.join("story/broken.md"), "not a planning document").expect("a broken file");

    let assembly = Assembly::read([
        (member("one"), one.as_path()),
        (member("two"), two.as_path()),
    ]);

    let failures = assembly.failures();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].0.as_str(), "two");
    assert_eq!(assembly.len(), 1, "what did load is still readable");
}

#[test]
fn a_reference_into_a_member_that_does_not_hold_it_finds_nothing_rather_than_the_other_member() {
    let one = scratch("miss-one/planning");
    let two = scratch("miss-two/planning");
    write(&one, "story:alpha", "Alpha");
    write(&two, "story:beta", "Beta");

    let assembly = Assembly::read([
        (member("one"), one.as_path()),
        (member("two"), two.as_path()),
    ]);

    let wrong_member = WorkspaceRef::parse("two/story:alpha").expect("parses");
    assert_eq!(assembly.resolve(&wrong_member), Resolution::Absent);
    assert!(assembly.get(&wrong_member).is_none());
}

#[test]
fn a_relation_into_another_member_resolves_and_says_which_member() {
    // The edge a workspace exists to carry: a story here depending on a story there. A single store
    // cannot check this one, because the target is outside it by construction.
    let one = scratch("cross-one/planning");
    let two = scratch("cross-two/planning");
    write_with_relation(&one, "story:alpha", "Alpha", "depends_on", "two/story:beta");
    write(&two, "story:beta", "Beta");

    let assembly = Assembly::read([
        (member("one"), one.as_path()),
        (member("two"), two.as_path()),
    ]);

    let crossings = assembly.crossing_relations();
    assert_eq!(crossings.len(), 1);
    assert_eq!(crossings[0].from_member.as_str(), "one");
    assert_eq!(crossings[0].kind, "depends_on");
    assert_eq!(crossings[0].to.to_string(), "two/story:beta");
    assert_eq!(crossings[0].resolution, Resolution::Unique(member("two")));
    assert!(crossings[0].is_resolved());
}

#[test]
fn a_crossing_into_a_member_nobody_checked_out_is_reported_and_not_an_error() {
    // A workspace is read on machines holding different subsets of it. Refusing here would fail a
    // plan for a reason that has nothing to do with the plan.
    let one = scratch("cross-absent-one/planning");
    let two = scratch("cross-absent-two/planning");
    write_with_relation(
        &one,
        "story:alpha",
        "Alpha",
        "blocks",
        "two/story:never-written",
    );
    write(&two, "story:beta", "Beta");

    let assembly = Assembly::read([
        (member("one"), one.as_path()),
        (member("two"), two.as_path()),
    ]);

    let crossings = assembly.crossing_relations();
    assert_eq!(crossings.len(), 1);
    assert_eq!(crossings[0].resolution, Resolution::Absent);
    assert!(!crossings[0].is_resolved());
    assert!(
        assembly.failures().is_empty(),
        "an unresolved crossing is a reported fact, not a store failure"
    );
}

#[test]
fn a_relation_that_stays_inside_its_member_is_not_a_crossing() {
    let one = scratch("cross-local/planning");
    write_with_relation(&one, "story:alpha", "Alpha", "depends_on", "story:beta");
    write(&one, "story:beta", "Beta");

    let assembly = Assembly::read([(member("one"), one.as_path())]);
    assert!(
        assembly.crossing_relations().is_empty(),
        "an unqualified relation means this member, and needs no workspace to check"
    );
}
