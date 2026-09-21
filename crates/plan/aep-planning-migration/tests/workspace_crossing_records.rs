//! Independent verification of `story:migration-mapper-reads-the-declared-workspace`.
//!
//! The story's `## Acceptance` says, verbatim:
//!
//! > the cross-member relation survives migration as a relation record and is read back
//! > identically on the Eventlog arm (history equality on the artifact that carries it).
//!
//! These cases measured the mapper against that sentence and it did not hold, and the coordinator
//! **withdrew the wording** rather than stretch the code: a relation record whose target is
//! another workspace member is a model change, and the authority's relation surface has no notion
//! of a foreign member. The amended requirement is that the crossing *survives* and the receipt
//! does not lie about it.
//!
//! The cases still hold the mapper to a measured number, and now to both halves of the boundary.
//! `markdown_boundaries_raw` produces exactly the subject histories the migration imports into the
//! destination authority, so what reaches the authority is a question about its output and nothing
//! else: a relation record is a subject whose entity is `aep.relation`, and the crossing is the
//! authored relation data carried in the subject's own record.

use aep_contract::migration::{
    HexBytesV1, HostPathV1, MarkdownNodeKindV1, MarkdownNodeV1, MarkdownRawV1,
    RegularMarkdownNodeV1,
};
use aep_domain::workspace::MemberName;
use aep_planning_migration::markdown_boundaries_raw;

const RELATION_ENTITY: &str = "aep.relation";

fn member(name: &str) -> MemberName {
    MemberName::parse(name).expect("a member name")
}

fn node(relative: &str, text: &str) -> MarkdownNodeV1 {
    MarkdownNodeV1 {
        relative: HostPathV1::Unix(HexBytesV1::new(relative.as_bytes().to_vec())),
        node: MarkdownNodeKindV1::Regular(RegularMarkdownNodeV1 {
            bytes: HexBytesV1::new(text.as_bytes().to_vec()),
        }),
    }
}

/// Two stories: `story:local`, and `story:crossing` carrying exactly one `informed_by` at
/// `relations[0]` that points at `target`.
///
/// The same shape as the real AEP store's `story:assemble-across-sources`, which carries
/// `- informed_by: entity-runtime/story:typed-references` and is the document the whole unit
/// exists for.
fn capture(target: &str) -> MarkdownRawV1 {
    MarkdownRawV1 {
        nodes: vec![
            node(
                "story/local.md",
                "---\nformat: aep.planning-md/1\nid: story:local\nkind: story\nstatus: draft\n\
                 title: Local\nrelations: []\nrevision: 1\n---\n",
            ),
            node(
                "story/crossing.md",
                &format!(
                    "---\nformat: aep.planning-md/1\nid: story:crossing\nkind: story\n\
                     status: draft\ntitle: Crossing\nrelations:\n\
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

/// The control, and the proof that the measurement above can see a relation record at all.
///
/// The identical fixture with a **local** target produces one `aep.relation` subject. Nothing
/// about the assertion, the fixture or the accessor is what decides the crossing case below.
#[test]
fn a_relation_to_a_local_artifact_is_migrated_as_a_relation_record() {
    let histories = markdown_boundaries_raw(&capture("story:local"), &[])
        .expect("a relation between two local artifacts is an ordinary edge");
    assert_eq!(
        relation_subjects(&histories).len(),
        1,
        "a local edge becomes one `{RELATION_ENTITY}` subject: {:?}",
        histories
            .iter()
            .map(|history| history.subject.clone())
            .collect::<Vec<_>>()
    );
}

/// `story:migration-mapper-reads-the-declared-workspace` `## Acceptance`, clause 2, as amended.
///
/// The store is admitted — that half of the fix works. The clause originally read "survives
/// migration as a relation **record**", and the coordinator withdrew that wording after this case
/// measured it: a relation record whose target is another workspace member is a model change the
/// authority's relation surface has no notion of, and a cutover unit does not make one.
///
/// What is required instead is that the crossing **survives** and the receipt does not lie. This
/// case holds both halves of the boundary at once: no `aep.relation` record is invented for a
/// destination that does not exist here, and the exact authored relation travels in the subject's
/// own record, which is what every read path then answers from.
#[test]
fn a_declared_crossing_is_migrated_as_the_subject_s_own_relation_data() {
    let histories = markdown_boundaries_raw(&capture("other/story:theirs"), &[member("other")])
        .expect("the declaration every ordinary read command reads admits the crossing");

    // The admission half, so a failure below cannot be mistaken for the refusal this unit fixed.
    assert_eq!(
        histories
            .iter()
            .filter(|history| history.subject.entity == "aep.entity")
            .count(),
        2,
        "both artifacts are migrated as subjects"
    );

    assert_eq!(
        relation_subjects(&histories).len(),
        0,
        "a crossing has no destination entity in this authority, so no relation record is \
         invented for it; subjects produced: {:?}",
        histories
            .iter()
            .map(|history| history.subject.clone())
            .collect::<Vec<_>>()
    );

    let bodies: Vec<serde_json::Value> = histories
        .iter()
        .filter(|history| history.subject.entity == "aep.entity")
        .map(|history| match &history.origin {
            entity_store::asynchronous::HistoryOrigin::Imported(anchor) => {
                serde_json::Value::Object(anchor.instance.fields.clone().into_iter().collect())
            }
            entity_store::asynchronous::HistoryOrigin::Genesis => serde_json::Value::Null,
        })
        .collect();
    assert!(
        bodies.iter().any(|body| {
            body["relations"].as_array().is_some_and(|relations| {
                relations.iter().any(|relation| {
                    relation["target"] == "other/story:theirs"
                        || relation["informed_by"] == "other/story:theirs"
                })
            })
        }),
        "the authored crossing travels in the subject's own imported body: {bodies:?}"
    );
}

/// The difference stated as a number, so the size of it stays on the record.
///
/// One store, two spellings of the same edge: `story:local` (local) and `other/story:theirs`
/// (a crossing the workspace declares). Both are valid graphs under their own declaration and
/// both are admitted, and only one of them reaches the authority as a relation record. That
/// difference is exactly what `inventory.workspace_crossings` exists to report, and it is pinned
/// here so it can never again be a difference nobody counted.
#[test]
fn a_crossing_and_a_local_edge_do_not_migrate_to_the_same_number_of_relation_records() {
    let local = markdown_boundaries_raw(&capture("story:local"), &[]).expect("local edge maps");
    let crossing = markdown_boundaries_raw(&capture("other/story:theirs"), &[member("other")])
        .expect("declared crossing maps");
    assert_eq!(
        relation_subjects(&local).len(),
        1,
        "a local edge becomes one relation record"
    );
    assert_eq!(
        relation_subjects(&crossing).len(),
        0,
        "a declared crossing becomes none, because its destination is not in this authority"
    );
    assert_eq!(
        relation_subjects(&local).len() - relation_subjects(&crossing).len(),
        1,
        "the difference is one relation record per crossing, which is what the receipt must say"
    );
}
