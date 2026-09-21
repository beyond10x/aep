//! This repository's own plan, kept in SQLite: seeded through the contract, reopened by a second
//! process, and answering the same history the markdown backend answers.
//!
//! `story:sqlite-hydrates-on-open`'s last two acceptance lines. Through the contract rather than the
//! `protocol artifact` verbs, because every verb opening through a `store:` in `project.yaml` is
//! wave H, story 1 — what this pins is that the two backends hold one plan the same way.

use std::path::{Path, PathBuf};
use std::time::Instant;

use aep_backend_markdown::backend::{MarkdownBackend, ORGANISATION, SPACE};
use aep_backend_markdown::store::MarkdownStore;
use aep_backend_memory::seed;
use aep_backend_sqlite::SqliteBackend;
use aep_contract::query::QueryService;
use aep_contract::testing::block_on;
use aep_contract::QueryConsistency;
use aep_domain::entity::{ActorRef, EntityRef};
use aep_domain::time::Timestamp;
use aep_domain::workspace::Membership;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// Where this repository's store stands in the workspace it declares: who it is, and who else it
/// names — the same value `protocol artifact` reads.
///
/// `own` matters here and not only in principle: this repository's own declaration names itself
/// (`engineering-protocols`, `source: ..`), so without it an edge written
/// `engineering-protocols/story:x` in this very store would seed as a crossing on one arm.
/// `crate::planning::declared_membership` is the reader the commands use and is not reachable from
/// an integration test; this mirrors its rule — the member whose path source resolves to this
/// repository — and nothing else.
fn declared_membership(root: &Path) -> Membership {
    let Ok(Some(workspace)) = aep_project::project::load_workspace(root) else {
        return Membership::default();
    };
    let engineering = root.join(".engineering");
    let here = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let own = workspace
        .members
        .iter()
        .find(|member| {
            aep_project::project::resolve_member(member, &engineering)
                .is_ok_and(|path| path.canonicalize().unwrap_or(path) == here)
        })
        .map(|member| member.name.clone());
    Membership::new(
        own,
        workspace.members.iter().map(|member| member.name.clone()),
    )
}

#[test]
fn this_repositorys_plan_round_trips_through_sqlite_and_answers_the_markdown_backends_history() {
    let root = repository_root();
    let planning = root.join(".engineering/planning");
    let membership = declared_membership(&root);
    let store = MarkdownStore::open(&planning);
    let report = store.load();
    assert!(report.is_clean(), "this repository's plan reads cleanly");
    let graph = report
        .graph_in_workspace(membership.clone())
        .expect("the plan is a graph");

    let at = Timestamp::from_epoch_millis(1_700_000_000_000);
    let actor = ActorRef::parse("human:conformance").expect("an actor");
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("own-plan.sqlite3");
    let _ = std::fs::remove_file(&path);

    // Process 1: the plan goes into SQLite through the same seeding the markdown backend uses.
    let sqlite = SqliteBackend::open(&path).expect("a database");
    let started = Instant::now();
    let seeded = seed::from_manifest(&sqlite, &graph, ORGANISATION, SPACE, at, &actor)
        .expect("the plan seeds");
    let seeding = started.elapsed();
    assert_eq!(seeded.entities, graph.len());
    drop(sqlite);

    // Process 2: opens the file and hydrates.
    let started = Instant::now();
    let reopened = SqliteBackend::open(&path).expect("the file reopens");
    let opening = started.elapsed();
    assert_eq!(reopened.len(), graph.len(), "every artifact came back");

    // The markdown backend over the same plan, same instant, same actor: the two seeds issue the
    // same commands in the same order, so the identities and the history must agree.
    let markdown = MarkdownBackend::open(
        &planning,
        membership,
        at,
        actor,
        aep_domain::artifact::LifecycleRegistry::default(),
    )
    .expect("the markdown backend opens");

    // A document this repository has since written through the contract has a journal of its own —
    // the events of its `new`, `move`, `relate` — which no seed can replay: the seed knows the
    // document as it stands, not how it got there. Those are compared by what they hold; the ones
    // that predate the log are compared by history too, and there must be enough of them for the
    // comparison to still be evidence.
    let mut predating = 0;
    for (artifact, id) in &seeded.by_id {
        let reference = EntityRef::new(id.clone());
        if !aep_backend_markdown::journal::history(&planning, artifact)
            .0
            .is_empty()
        {
            continue;
        }
        predating += 1;
        let ours = block_on(reopened.history(&reference)).expect("history from SQLite");
        let theirs = block_on(markdown.history(&reference))
            .unwrap_or_else(|error| panic!("{artifact}: the markdown backend holds {id}: {error}"));
        assert_eq!(
            ours, theirs,
            "{artifact}: history differs between the two backends"
        );

        let held = block_on(reopened.get(&reference, QueryConsistency::Current)).expect("held");
        let theirs = block_on(markdown.get(&reference, QueryConsistency::Current)).expect("held");
        assert_eq!(
            held.metadata, theirs.metadata,
            "{artifact}: metadata differs between the two backends"
        );
    }

    assert!(
        predating >= 20,
        "only {predating} documents predate the event log; the comparison has stopped being evidence"
    );

    // Written down for the story rather than asserted: a threshold here would turn a slow CI box
    // into a red gate about a number nobody chose.
    eprintln!(
        "sqlite plan: {} artifacts, {} relations — seeded in {seeding:?}, reopened (hydrated) in \
         {opening:?}",
        seeded.entities, seeded.relations
    );
}
