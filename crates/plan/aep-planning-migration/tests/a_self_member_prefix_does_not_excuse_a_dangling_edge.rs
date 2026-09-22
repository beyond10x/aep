//! The hole `validate_edges` says it closed is still open behind this store's own member name.
//!
//! `ArtifactGraph::validate_edges` (`aep-domain/src/artifact.rs:2526-2533`) states the rule it
//! enforces, verbatim:
//!
//! > A member this workspace does **not** declare is a different thing entirely, and exempting it
//! > was a hole: with no workspace file at all, every dangling edge could be hidden behind a `/`,
//! > and a misspelled member name passed silently in a plain single-repository store.
//!
//! The exemption is keyed on `ArtifactRelation::crosses_to_a_declared_member`
//! (`aep-domain/src/artifact.rs:1324`), which asks only whether the target names *a* declared
//! member. It has no notion of which member **this store** is. A workspace that names itself — the
//! shape this repository's own `.engineering/workspace.yaml` has, `engineering-protocols` with
//! `source: ..` — therefore restores the hole in full: prefix any dangling edge with this store's
//! own member name and the "the manifest does not declare it" check is skipped, because per
//! `aep_domain::workspace::WorkspaceRef` a reference naming a member is read as *that member's*
//! artifact, and that member is here.
//!
//! The control is the same dangling target with no prefix: it is refused, which is what makes this
//! a hole rather than a property of the fixture.

use aep_contract::migration::{
    HexBytesV1, HostPathV1, MarkdownNodeKindV1, MarkdownNodeV1, MarkdownRawV1,
    RegularMarkdownNodeV1,
};
use aep_domain::workspace::{MemberName, Membership};
use aep_planning_migration::markdown_boundaries_raw;

/// The member name this repository's own `.engineering/workspace.yaml` gives itself.
const OWN_MEMBER: &str = "engineering-protocols";

/// An artifact no document in the store declares.
const MISSING: &str = "story:typo-that-does-not-exist";

/// This store declares `OWN_MEMBER`, and it **is** `OWN_MEMBER` — the input the rule was missing.
fn declaring_itself() -> Membership {
    Membership::new(Some(member(OWN_MEMBER)), [member(OWN_MEMBER)])
}

fn member(name: &str) -> MemberName {
    MemberName::parse(name).expect("a member name")
}

fn capture(target: &str) -> MarkdownRawV1 {
    MarkdownRawV1 {
        nodes: vec![MarkdownNodeV1 {
            relative: HostPathV1::Unix(HexBytesV1::new(b"story/pointer.md".to_vec())),
            node: MarkdownNodeKindV1::Regular(RegularMarkdownNodeV1 {
                bytes: HexBytesV1::new(
                    format!(
                        "---\nformat: aep.planning-md/1\nid: story:pointer\nkind: story\n\
                         status: draft\ntitle: Pointer\nrelations:\n\
                         - informed_by: {target}\nrevision: 1\n---\n"
                    )
                    .into_bytes(),
                ),
            }),
        }],
    }
}

/// The control: an unprefixed dangling edge is refused, with the same member declared.
#[test]
fn an_unprefixed_dangling_edge_is_refused_even_when_this_member_is_declared() {
    assert!(
        markdown_boundaries_raw(&capture(MISSING), &declaring_itself()).is_err(),
        "`{MISSING}` is declared by no document in this store, so the store does not migrate"
    );
}

/// The same dangling edge, prefixed with this store's own declared member name, is admitted.
///
/// Nothing in the store declares the target and nothing ever will: the reference names this
/// member, and this member is the store being read. A store migrated with an edge that dangles for
/// real is what `mapping.rs:249-253` says the member list exists to prevent.
#[test]
fn the_same_dangling_edge_behind_this_store_s_own_member_name_is_still_a_dangling_edge() {
    let target = format!("{OWN_MEMBER}/{MISSING}");
    let mapped = markdown_boundaries_raw(&capture(&target), &declaring_itself());
    assert!(
        mapped.is_err(),
        "`{target}` names this store's own member, so the target is this store's own `{MISSING}`, \
         which no document declares; prefixing a dangling edge with the reader's own member name \
         cannot turn it into a crossing an assembly will resolve. The store was admitted and \
         produced {} subject(s).",
        mapped.map(|histories| histories.len()).unwrap_or_default()
    );
}
