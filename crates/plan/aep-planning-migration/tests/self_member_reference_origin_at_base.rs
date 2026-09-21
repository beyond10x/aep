//! Origin, settled in this tree rather than by argument.
//!
//! At `763d195be` the mapper did not take a member list at all:
//! `markdown_boundaries_raw(raw: &MarkdownRawV1)`, and inside it `report.graph()`
//! (`mapping.rs:181` at base), which is `graph_in_workspace(std::iter::empty())`
//! (`aep-backend-markdown/src/store.rs:360-362`, unchanged at head). So the base behaviour for any
//! member-qualified target — including one qualified with this store's own member name — is
//! exactly what the head function does when handed an empty member list.
//!
//! This case reproduces that call and shows the base answer was a **refusal**: the store could not
//! be migrated, so no relation record was lost, because nothing was imported. The head admits the
//! same store and drops the edge (see `self_member_reference_is_not_a_crossing.rs`). The defect is
//! therefore `introduced` — the unit's diff reaches a path nothing reached before — and not a
//! pre-existing one this pass happens to have noticed.

use aep_contract::migration::{
    HexBytesV1, HostPathV1, MarkdownNodeKindV1, MarkdownNodeV1, MarkdownRawV1,
    RegularMarkdownNodeV1,
};
use aep_planning_migration::markdown_boundaries_raw;

const OWN_MEMBER: &str = "engineering-protocols";

fn node(relative: &str, text: &str) -> MarkdownNodeV1 {
    MarkdownNodeV1 {
        relative: HostPathV1::Unix(HexBytesV1::new(relative.as_bytes().to_vec())),
        node: MarkdownNodeKindV1::Regular(RegularMarkdownNodeV1 {
            bytes: HexBytesV1::new(text.as_bytes().to_vec()),
        }),
    }
}

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

/// The base call — no member list — refuses the store outright.
///
/// Green here and green at base. It is the pair with the red case in
/// `self_member_reference_is_not_a_crossing.rs` that settles origin: base refused, head admits and
/// silently imports nothing for the edge.
#[test]
fn with_no_member_list_the_base_call_refuses_a_self_member_qualified_edge() {
    let target = format!("{OWN_MEMBER}/story:local");
    let mapped = markdown_boundaries_raw(
        &capture(&target),
        &aep_domain::workspace::Membership::default(),
    );
    assert!(
        mapped.is_err(),
        "at `763d195be` the mapper called `report.graph()`, which is \
         `graph_in_workspace(std::iter::empty())`, so `{target}` was a dangling edge and the store \
         was refused before any relation was mapped; head admits the same store"
    );
}
