//! A reference qualified with **this store's own member name** is a local edge, not a crossing.
//!
//! `aep_domain::workspace::WorkspaceRef` defines the two spellings and what each means:
//!
//! > | `story:provider-spi` | *this* member's story — whichever store the reference was written in |
//! > | `entity-runtime/story:provider-spi` | that member's story, wherever it is read from |
//!
//! So in a store whose workspace declares `engineering-protocols` as one of its members —
//! which this repository's own `.engineering/workspace.yaml` does, explicitly, with the comment
//! *"This repository. Named explicitly rather than implied, so every artifact in the assembled
//! graph carries a member and none of them is the special one that does not."* —
//! `engineering-protocols/story:local` and `story:local` name the **same artifact in this very
//! store**. One of them is a crossing and the other is not, and the difference decides whether the
//! migration imports a relation record for it.
//!
//! `ArtifactRelation::crosses_to_a_declared_member` (`aep-domain/src/artifact.rs:1324`) asks only
//! whether the target names *a* declared member. It has no notion of which member this store is,
//! so it answers `true` for a reference that resolves here. The mapper
//! (`aep-planning-migration/src/mapping.rs:324`) then skips it on the stated ground that "workspace
//! crossings have no destination entity in this authority" — but this destination entity *is* in
//! this authority, and it is right there in `identities` under its unqualified id.
//!
//! The control in this file is the identical fixture with the identical member declared and an
//! unqualified target: it produces one relation record, so nothing about the fixture, the member
//! list or the accessor is what decides the case below.

use aep_contract::migration::{
    HexBytesV1, HostPathV1, MarkdownNodeKindV1, MarkdownNodeV1, MarkdownRawV1,
    RegularMarkdownNodeV1,
};
use aep_domain::workspace::{MemberName, Membership};
use aep_planning_migration::markdown_boundaries_raw;

const RELATION_ENTITY: &str = "aep.relation";

/// The member name this repository's own `.engineering/workspace.yaml` gives itself.
const OWN_MEMBER: &str = "engineering-protocols";

fn member(name: &str) -> MemberName {
    MemberName::parse(name).expect("a member name")
}

/// This store's standing in the workspace: it declares `OWN_MEMBER`, and it **is** `OWN_MEMBER`.
///
/// The second half is the input the rule was missing. Written here rather than in each case so the
/// two spellings below are compared against one identical declaration.
fn declaring_itself() -> Membership {
    Membership::new(Some(member(OWN_MEMBER)), [member(OWN_MEMBER)])
}

fn node(relative: &str, text: &str) -> MarkdownNodeV1 {
    MarkdownNodeV1 {
        relative: HostPathV1::Unix(HexBytesV1::new(relative.as_bytes().to_vec())),
        node: MarkdownNodeKindV1::Regular(RegularMarkdownNodeV1 {
            bytes: HexBytesV1::new(text.as_bytes().to_vec()),
        }),
    }
}

/// Two stories, both held by this store: `story:local`, and `story:pointer` carrying exactly one
/// `informed_by` at `relations[0]` that points at `target`.
fn capture(target: &str) -> MarkdownRawV1 {
    MarkdownRawV1 {
        nodes: vec![
            node(
                "story/local.md",
                "---\nformat: aep.planning-md/1\nid: story:local\nkind: story\nstatus: draft\n\
                 title: Local\nrelations: []\nrevision: 1\n---\n",
            ),
            node(
                "story/pointer.md",
                &format!(
                    "---\nformat: aep.planning-md/1\nid: story:pointer\nkind: story\n\
                     status: draft\ntitle: Pointer\nrelations:\n\
                     - informed_by: {target}\nrevision: 1\n---\n"
                ),
            ),
        ],
    }
}

fn relation_subjects(histories: &[entity_store::asynchronous::SubjectHistory]) -> Vec<String> {
    histories
        .iter()
        .filter(|history| history.subject.entity == RELATION_ENTITY)
        .map(|history| history.subject.id.clone())
        .collect()
}

fn entity_subjects(histories: &[entity_store::asynchronous::SubjectHistory]) -> usize {
    histories
        .iter()
        .filter(|history| history.subject.entity == "aep.entity")
        .count()
}

/// The control: the same store, the same declaration, an unqualified target.
///
/// `engineering-protocols` is declared as a member here too, so the member list is not what
/// changes between this case and the next one. Only the spelling of the target changes.
#[test]
fn an_unqualified_local_edge_is_a_relation_record_even_when_this_member_is_declared() {
    let histories = markdown_boundaries_raw(&capture("story:local"), &declaring_itself())
        .expect("an edge between two artifacts of this store is an ordinary edge");
    assert_eq!(
        entity_subjects(&histories),
        2,
        "both artifacts are migrated"
    );
    assert_eq!(
        relation_subjects(&histories).len(),
        1,
        "a local edge becomes one `{RELATION_ENTITY}` subject"
    );
}

/// The same edge, written the way the workspace file invites, is silently not imported.
///
/// `engineering-protocols/story:local` resolves to `story:local` **in this store**. There is a
/// destination entity for it in this authority; the mapper drops the relation anyway, because the
/// only question anything asks is "does the target name a declared member", and this store's own
/// name is a declared member.
#[test]
fn a_reference_qualified_with_this_store_s_own_member_name_is_still_a_local_relation_record() {
    let target = format!("{OWN_MEMBER}/story:local");
    let histories = markdown_boundaries_raw(&capture(&target), &declaring_itself())
        .expect("the declaration admits the store");

    // The admission half first, so a failure below cannot be mistaken for the refusal this unit
    // exists to remove.
    assert_eq!(
        entity_subjects(&histories),
        2,
        "both artifacts are migrated"
    );

    assert_eq!(
        relation_subjects(&histories).len(),
        1,
        "`{target}` names an artifact of this very store, so the edge has a destination entity in \
         this authority and must be imported as a relation record exactly as `story:local` is; \
         subjects produced: {:?}",
        histories
            .iter()
            .map(|history| history.subject.clone())
            .collect::<Vec<_>>()
    );
}

/// The two spellings of one edge must migrate to the same number of relation records.
///
/// This is the invariant the unqualified/qualified distinction rests on: per `WorkspaceRef`, both
/// spellings name the same artifact when the reference is read inside the member it names, so no
/// count anywhere may depend on which spelling an author chose.
#[test]
fn the_two_spellings_of_one_local_edge_migrate_to_the_same_number_of_relation_records() {
    let members = declaring_itself();
    let unqualified =
        markdown_boundaries_raw(&capture("story:local"), &members).expect("unqualified maps");
    let qualified =
        markdown_boundaries_raw(&capture(&format!("{OWN_MEMBER}/story:local")), &members)
            .expect("self-qualified maps");
    assert_eq!(
        relation_subjects(&qualified).len(),
        relation_subjects(&unqualified).len(),
        "`{OWN_MEMBER}/story:local` and `story:local` are the same artifact read from this store, \
         so they cannot migrate to different numbers of relation records"
    );
}
