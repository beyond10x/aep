//! The planning store, held to the same sixteen suites as every other backend.
//!
//! This is what deviation **D-P1** cost and what closing it buys: until the store answered as a
//! contract implementation, "there is a durable backend" was a claim the suites did not support,
//! and `AGENTS.md` § *Current state* had to say both halves.

use std::path::{Path, PathBuf};

use aep_backend_markdown::backend::MarkdownBackend;
use aep_domain::entity::ActorRef;
use aep_domain::time::Timestamp;

/// A scratch store of this name, emptied first so a rerun is a fresh read.
fn scratch(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("conformance")
        .join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch directory");
    root
}

/// The ladders these tests hold a store to.
///
/// Empty, so every kind falls back to the permissive lifecycle — these tests are about the contract
/// and durability, not about any particular ladder. `a_status_off_the_ladder_is_refused` builds a
/// real one.
fn ladders() -> aep_domain::artifact::LifecycleRegistry {
    aep_domain::artifact::LifecycleRegistry::default()
}

fn backend(name: &str) -> MarkdownBackend {
    MarkdownBackend::open(
        scratch(name),
        std::iter::empty(),
        Timestamp::from_epoch_millis(1_700_000_000_000),
        ActorRef::parse("human:conformance").expect("a well-formed actor"),
        ladders(),
    )
    .expect("an empty store opens")
}

#[test]
fn the_markdown_store_conforms() {
    let store = backend("suites");
    let report = aep_conformance::run(&store, aep_conformance::Level::Full);
    assert!(
        report.passed(),
        "MarkdownBackend failed {} of {} checks:\n{}",
        report.failures(),
        report.checks(),
        report
            .failing_suites()
            .flat_map(
                |suite| suite
                    .checks
                    .iter()
                    .filter(|check| !check.passed)
                    .map(|check| format!(
                        "  {}: {}",
                        check.name,
                        check.detail.as_deref().unwrap_or("")
                    ))
            )
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_suites_that_pass_here_catch_a_backend_that_is_wrong() {
    // The guard on the test above, and the lesson of 2026-08-26: a suite that passes proves nothing
    // until something proves the suite can fail. `entity-sqlite`'s "rolls back both halves" test
    // passed verbatim against the store documented as unable to make that promise, because it
    // asserted a refusal that happened before either write.
    //
    // `FaultyBackend` is the contract's own known-wrong implementation. If these suites ever stop
    // catching it, `the_markdown_store_conforms` has stopped being evidence.
    for fault in [
        aep_conformance::Fault::ReplayApplies,
        aep_conformance::Fault::IgnoreExpectedRevision,
        aep_conformance::Fault::DropRejectionAudit,
    ] {
        // Wrapping *this* backend, not a different one: the question is whether these suites catch
        // a fault in the store under test, not whether they catch one somewhere else.
        let faulty = aep_conformance::FaultyBackend::new(backend("faulty"), fault);
        let report = aep_conformance::run(&faulty, aep_conformance::Level::Full);
        assert!(
            !report.passed(),
            "the suites passed a backend injected with {fault:?}, so they are not evidence"
        );
    }
}

#[test]
fn the_suites_run_are_the_sixteen_and_not_a_subset() {
    // A count in prose goes stale; a count asserted against the runner does not. `Level::Full` is
    // what "the store runs the suites" means, and a level that quietly narrowed would read exactly
    // like a store that passed more.
    let store = backend("count");
    let report = aep_conformance::run(&store, aep_conformance::Level::Full);
    assert_eq!(
        report.suites.len(),
        16,
        "sixteen suites, as `docs/plan/gap-register.md:37` and the P3 story both say"
    );
    assert!(
        report.checks() > 16,
        "and each suite makes more than one check"
    );
}

/// Writes a planning document into a scratch store.
fn write_story(root: &Path, name: &str, status: &str, body: &str) {
    let directory = root.join("story");
    std::fs::create_dir_all(&directory).expect("the kind's directory");
    std::fs::write(
        directory.join(format!("{name}.md")),
        format!(
            "---\nformat: aep.planning-md/1\nid: story:{name}\nkind: story\nstatus: {status}\n\
             title: A story\nrevision: 1\n---\n\n{body}\n"
        ),
    )
    .expect("the document");
}

#[test]
fn a_command_that_moves_a_story_survives_a_reopen() {
    // The whole point of P3. Before it, the suites did not run against this store and "there is a
    // durable backend" was a claim nothing supported: writes went through the store's own
    // `create`/`update` rather than through the contract's one door.
    use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
    use aep_contract::query::QueryService;
    use aep_contract::testing::block_on;
    use aep_domain::command::{Command, UpdateEntity};
    use aep_domain::entity::{EntityLocator, EntityRef};

    let root = scratch("durable");
    write_story(
        &root,
        "one",
        "draft",
        "# Story\n\nThe prose nobody may lose.",
    );

    let at = Timestamp::from_epoch_millis(1_700_000_000_000);
    let actor = ActorRef::parse("human:operator").expect("an actor");
    let store = MarkdownBackend::open(&root, std::iter::empty(), at, actor.clone(), ladders())
        .expect("the store opens");

    let locator = EntityLocator::parse("ep://planning/store/story/one").expect("a locator");
    let id = block_on(store.resolve(&locator)).expect("the seeded entity resolves");

    let payload = Command::UpdateEntity(UpdateEntity {
        target: EntityRef::new(id.clone()),
        changes: [("status".to_owned(), aep_domain::node::Node::from("active"))]
            .into_iter()
            .collect(),
    });
    let context = CommandContext::new(
        "req-move".parse().expect("a request id"),
        "key-move".parse().expect("an idempotency key"),
        actor,
        "corr-move".parse().expect("a correlation id"),
        at,
    );
    let mut envelope = CommandEnvelope::new(
        "cmd-move".parse().expect("a command id"),
        "aep.entity.update/v1",
        payload,
        context,
    );
    envelope.target = Some(EntityRef::new(id));
    block_on(store.execute(envelope)).expect("the move is permitted");

    // The file, read by something that never saw the backend.
    let written = std::fs::read_to_string(root.join("story").join("one.md")).expect("the document");
    assert!(
        written.contains("status: active"),
        "the command reached the file:\n{written}"
    );
    assert!(
        written.contains("The prose nobody may lose."),
        "and the body survived it — an entity carries no prose, so rebuilding a document from one \
         would delete every Outcome and Acceptance in the store:\n{written}"
    );

    // And the journal, which is what D-P3 was about.
    let (entries, unreadable) = aep_backend_markdown::journal::read(&root);
    assert_eq!(unreadable, 0);
    assert!(
        entries
            .iter()
            .any(|entry| entry.artifact.to_string() == "story:one"),
        "the move is in the journal, answerable without reading git"
    );
}

#[test]
fn a_created_entity_becomes_a_document_this_store_holds() {
    // The first of the two projections `persist` was missing, and the one that blocks
    // `protocol artifact new` from routing through a command. Before it, an entity created through
    // the contract lived only in this process: correct by the suites, and gone on restart.
    use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
    use aep_contract::testing::block_on;
    use aep_domain::command::{Command, CreateEntity};
    use aep_domain::entity::{EntityLocator, EntityType};

    let root = scratch("created");
    let at = Timestamp::from_epoch_millis(1_700_000_000_000);
    let actor = ActorRef::parse("human:operator").expect("an actor");
    let store = MarkdownBackend::open(&root, std::iter::empty(), at, actor.clone(), ladders())
        .expect("an empty store opens");

    let payload = Command::CreateEntity(CreateEntity {
        entity_type: EntityType::parse("aep.story/v1").expect("a type"),
        locator: EntityLocator::parse("ep://planning/store/story/new-one").expect("a locator"),
        data: aep_domain::node::Node::Map(
            [
                ("status".to_owned(), aep_domain::node::Node::from("draft")),
                (
                    "title".to_owned(),
                    aep_domain::node::Node::from("A new story"),
                ),
            ]
            .into_iter()
            .collect(),
        ),
    });
    let context = CommandContext::new(
        "req-new".parse().expect("a request id"),
        "key-new".parse().expect("an idempotency key"),
        actor,
        "corr-new".parse().expect("a correlation id"),
        at,
    );
    let envelope = CommandEnvelope::new(
        "cmd-new".parse().expect("a command id"),
        "aep.entity.create/v1",
        payload,
        context,
    );
    block_on(store.execute(envelope)).expect("the create is permitted");

    let written = std::fs::read_to_string(root.join("story").join("new-one.md"))
        .expect("the document the command produced");
    assert!(written.contains("id: story:new-one"), "{written}");
    assert!(written.contains("status: draft"), "{written}");
    assert!(written.contains("title: A new story"), "{written}");

    let (entries, _) = aep_backend_markdown::journal::read(&root);
    assert!(
        entries
            .iter()
            .any(|entry| entry.artifact.to_string() == "story:new-one"),
        "and the creation is in the journal"
    );
}

#[test]
fn an_entity_this_store_is_not_addressed_for_gets_no_invented_file() {
    // The other half of the same decision. A conformance suite's entities are real to the contract
    // and this store has no document shape for them; writing one anyway would put a document nobody
    // wrote into somebody's plan. They are reported instead.
    let root = scratch("unprojected");
    let at = Timestamp::from_epoch_millis(1_700_000_000_000);
    let store = MarkdownBackend::open(
        &root,
        std::iter::empty(),
        at,
        ActorRef::parse("human:operator").expect("an actor"),
        ladders(),
    )
    .expect("an empty store opens");

    let report = aep_conformance::run(&store, aep_conformance::Level::Full);
    assert!(report.passed(), "the suites still pass");
    assert!(
        !store.unprojected().is_empty(),
        "and the entities they invented are reported rather than filed"
    );
    assert!(
        !root.join("suite").exists(),
        "no directory was invented for them"
    );
}

#[test]
fn a_relation_command_becomes_an_edge_in_the_frontmatter() {
    // The second projection, and the one `protocol artifact relate` has no command path without.
    // The contract models an edge as a `Relation` record, not as a field on an entity, so nothing
    // about updating a body brings it along.
    use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
    use aep_contract::query::QueryService;
    use aep_contract::testing::block_on;
    use aep_domain::artifact::RelationKind;
    use aep_domain::command::{Command, CreateRelation};
    use aep_domain::entity::{EntityLocator, EntityRef};

    let root = scratch("related");
    write_story(&root, "one", "draft", "# One\n\nProse.");
    write_story(&root, "two", "draft", "# Two\n\nProse.");

    let at = Timestamp::from_epoch_millis(1_700_000_000_000);
    let actor = ActorRef::parse("human:operator").expect("an actor");
    let store = MarkdownBackend::open(&root, std::iter::empty(), at, actor.clone(), ladders())
        .expect("the store opens");

    let one = block_on(
        store.resolve(&EntityLocator::parse("ep://planning/store/story/one").expect("a locator")),
    )
    .expect("story:one is seeded");
    let two = block_on(
        store.resolve(&EntityLocator::parse("ep://planning/store/story/two").expect("a locator")),
    )
    .expect("story:two is seeded");

    let context = CommandContext::new(
        "req-rel".parse().expect("a request id"),
        "key-rel".parse().expect("an idempotency key"),
        actor,
        "corr-rel".parse().expect("a correlation id"),
        at,
    );
    let envelope = CommandEnvelope::new(
        "cmd-rel".parse().expect("a command id"),
        "aep.relation.create/v1",
        Command::CreateRelation(CreateRelation {
            kind: RelationKind::DependsOn,
            source: EntityRef::new(one),
            target: EntityRef::new(two),
        }),
        context,
    );
    block_on(store.execute(envelope)).expect("the relation is permitted");

    let written = std::fs::read_to_string(root.join("story").join("one.md")).expect("the document");
    assert!(
        written.contains("depends_on: story:two"),
        "the edge reached the file:\n{written}"
    );
    assert!(
        written.contains("Prose."),
        "and the body survived it:\n{written}"
    );
}

#[test]
fn a_status_off_the_ladder_is_refused_however_it_arrives() {
    // **The bypass this closes, and the test that used to bless it.**
    //
    // The contract is storage-agnostic and permits a `status` key on an `UpdateEntity` — its own
    // conformance suites use one (`aep-conformance/src/suites/causation.rs`). So the store is the
    // only layer that can refuse an illegal move, and until this check it did not: a story at
    // `draft` reached `active` with `draft: [proposed, archived]` declared, because nothing
    // consulted a ladder on that path. The test written for it asserted the bypass as correct.
    use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
    use aep_contract::query::QueryService;
    use aep_contract::testing::block_on;
    use aep_domain::artifact::{
        ArtifactKind, ArtifactLifecycle, ArtifactStatus, LifecycleRegistry,
    };
    use aep_domain::command::{Command, UpdateEntity};
    use aep_domain::entity::{EntityLocator, EntityRef};

    let root = scratch("ladder");
    write_story(&root, "one", "draft", "# One\n\nProse.");

    // `draft` goes to `proposed`, and nowhere else.
    let mut lifecycle = ArtifactLifecycle::permissive();
    lifecycle.kind = Some(ArtifactKind::parse("story").expect("a kind"));
    lifecycle.initial = ArtifactStatus::parse("draft").expect("a status");
    lifecycle.transitions.clear();
    lifecycle.transitions.insert(
        ArtifactStatus::parse("draft").expect("a status"),
        [ArtifactStatus::parse("proposed").expect("a status")]
            .into_iter()
            .collect(),
    );
    let mut ladders = LifecycleRegistry::new();
    ladders.insert(ArtifactKind::parse("story").expect("a kind"), lifecycle);

    let at = Timestamp::from_epoch_millis(1_700_000_000_000);
    let actor = ActorRef::parse("human:operator").expect("an actor");
    let store = MarkdownBackend::open(&root, std::iter::empty(), at, actor.clone(), ladders)
        .expect("the store opens");

    let id = block_on(
        store.resolve(&EntityLocator::parse("ep://planning/store/story/one").expect("a locator")),
    )
    .expect("it resolves");

    let error = block_on(
        store.execute(CommandEnvelope::new(
            "cmd-jump".parse().expect("a command id"),
            "aep.entity.update/v1",
            Command::UpdateEntity(UpdateEntity {
                target: EntityRef::new(id),
                changes: [("status".to_owned(), aep_domain::node::Node::from("active"))]
                    .into_iter()
                    .collect(),
            }),
            CommandContext::new(
                "req-jump".parse().expect("a request id"),
                "key-jump".parse().expect("an idempotency key"),
                actor,
                "corr-jump".parse().expect("a correlation id"),
                at,
            ),
        )),
    )
    .expect_err("`active` is not on `draft`'s ladder");
    assert!(
        error.to_string().contains("is not on its ladder"),
        "and it says so: {error}"
    );

    let written = std::fs::read_to_string(root.join("story").join("one.md")).expect("document");
    assert!(
        written.contains("status: draft"),
        "and the file did not move:\n{written}"
    );
}

#[test]
fn a_command_can_carry_the_document_prose_and_absence_leaves_it_alone() {
    // The decision that lets `protocol artifact new` route through a command: prose is data under a
    // reserved key, exactly as `status` and `title` already are.
    //
    // The half that matters more is the second assertion. An entity that does **not** carry the key
    // leaves the prose alone — absent is not empty. A backend that read absence as "delete the
    // prose" would empty every artifact in the store on its first status move.
    use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
    use aep_contract::query::QueryService;
    use aep_contract::testing::block_on;
    use aep_domain::command::{Command, CreateEntity, UpdateEntity};
    use aep_domain::entity::{EntityLocator, EntityRef, EntityType};

    let root = scratch("prose");
    let at = Timestamp::from_epoch_millis(1_700_000_000_000);
    let actor = ActorRef::parse("human:operator").expect("an actor");
    let store = MarkdownBackend::open(&root, std::iter::empty(), at, actor.clone(), ladders())
        .expect("an empty store opens");

    let context = |name: &str| {
        CommandContext::new(
            format!("req-{name}").parse().expect("a request id"),
            format!("key-{name}").parse().expect("an idempotency key"),
            actor.clone(),
            "corr-prose".parse().expect("a correlation id"),
            at,
        )
    };

    block_on(
        store.execute(CommandEnvelope::new(
            "cmd-create".parse().expect("a command id"),
            "aep.entity.create/v1",
            Command::CreateEntity(CreateEntity {
                entity_type: EntityType::parse("aep.story/v1").expect("a type"),
                locator: EntityLocator::parse("ep://planning/store/story/prosaic")
                    .expect("a locator"),
                data: aep_domain::node::Node::Map(
                    [
                        ("status".to_owned(), aep_domain::node::Node::from("draft")),
                        (
                            aep_backend_markdown::backend::BODY_KEY.to_owned(),
                            aep_domain::node::Node::from(
                                "# Prosaic\n\nThe template a person will fill in.",
                            ),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                ),
            }),
            context("create"),
        )),
    )
    .expect("the create is permitted");

    let written = std::fs::read_to_string(root.join("story").join("prosaic.md")).expect("document");
    assert!(
        written.contains("The template a person will fill in."),
        "the command carried the prose:\n{written}"
    );

    // Now a change that says nothing about the body.
    let id =
        block_on(store.resolve(
            &EntityLocator::parse("ep://planning/store/story/prosaic").expect("a locator"),
        ))
        .expect("it resolves");
    block_on(
        store.execute(CommandEnvelope::new(
            "cmd-title".parse().expect("a command id"),
            "aep.entity.update/v1",
            Command::UpdateEntity(UpdateEntity {
                target: EntityRef::new(id),
                changes: [("title".to_owned(), aep_domain::node::Node::from("Renamed"))]
                    .into_iter()
                    .collect(),
            }),
            context("title"),
        )),
    )
    .expect("the update is permitted");

    let after = std::fs::read_to_string(root.join("story").join("prosaic.md")).expect("document");
    assert!(
        after.contains("title: Renamed"),
        "the change landed:\n{after}"
    );
    assert!(
        after.contains("The template a person will fill in."),
        "and a command that said nothing about the body left it alone:\n{after}"
    );
}

#[test]
fn the_only_write_path_out_of_this_crate_is_a_command() {
    // P3's other half: "a source scan finds no other write path in the crate". `MarkdownStore`
    // still *has* `create` and `update` — they are how a file gets written — but nothing inside
    // this crate may reach them except `MarkdownProvider::commit`, the one door the adapter in
    // `aep-backend-entity` writes through (wave G, story 2; until then it was
    // `MarkdownBackend::persist_document`). Anything else would be a second write path, and a
    // second write path is a second place for idempotency, revision checks and the audit record to
    // be forgotten.
    //
    // A scan, because that is the only thing that can see an omission: a module added next year
    // that calls `store.update` directly would compile, pass every test, and quietly reopen D-P1.
    const SOURCES: &[(&str, &str)] = &[
        ("backend.rs", include_str!("../src/backend.rs")),
        ("assembly.rs", include_str!("../src/assembly.rs")),
        ("journal.rs", include_str!("../src/journal.rs")),
        ("claim.rs", include_str!("../src/claim.rs")),
        ("document.rs", include_str!("../src/document.rs")),
        ("frontmatter.rs", include_str!("../src/frontmatter.rs")),
        ("kernel.rs", include_str!("../src/kernel.rs")),
        ("lib.rs", include_str!("../src/lib.rs")),
        // **`store.rs` was missing, and it is the module that holds the write primitive.** The
        // scan claimed all eight modules and read seven; a helper added beside `create`/`update`
        // calling the private `write` was invisible to it.
        ("store.rs", include_str!("../src/store.rs")),
        // Wave G: the provider holds the one permitted write, and the projection beside it must
        // hold none. Both listed, for the reason `store.rs` is.
        ("provider.rs", include_str!("../src/provider.rs")),
        ("projection.rs", include_str!("../src/projection.rs")),
    ];

    let mut offenders = Vec::new();
    for (name, source) in SOURCES {
        // Production code only. A `#[cfg(test)] mod tests` exercising `create`/`update` directly is
        // testing the primitive, which is what a unit test of a store is for — it is not a path
        // anything outside this crate can reach.
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for (number, line) in production.lines().enumerate() {
            if !writes_to_the_store(line) {
                continue;
            }
            // The one permitted site, bounded by the function itself rather than by a fixed
            // window. A window is a guess about how long `persist` is, and the first time it grew
            // this scan reported its own crate — right answer, wrong reason.
            if *name == "provider.rs" && within_the_write_site(source, number) {
                continue;
            }
            offenders.push(format!("{name}:{}: {}", number + 1, line.trim()));
        }
    }

    assert!(
        offenders.is_empty(),
        "a write outside `MarkdownProvider::commit` is a second write path, which invariant 14 \
         exists to forbid — model the change as a command and let `execute` carry it:\n  {}",
        offenders.join("\n  ")
    );
}

/// Whether one line of source writes to the store.
///
/// **The scan and its guard call this, and neither restates it.** The guard used to carry its own
/// copy of the predicate, so weakening the real one left the guard green — which is precisely the
/// failure this whole file exists to refuse: a test that cannot fail is not evidence, and a guard
/// that guards a copy of the thing guards nothing.
fn writes_to_the_store(line: &str) -> bool {
    let code = line.trim_start();
    if code.starts_with("//") || code.starts_with("///") || code.starts_with('*') {
        return false;
    }
    code.contains(".create(&") || code.contains(".update(&")
}

#[test]
fn the_scan_sees_a_write_it_should_refuse() {
    // The guard, and it calls the real predicate rather than a copy of it. Delete `.update(&` from
    // `writes_to_the_store` and this fails, which is the whole point.
    //
    // If this test ever starts passing while `the_only_write_path_out_of_this_crate_is_a_command`
    // still does, that one has stopped being evidence.
    assert!(
        writes_to_the_store("    store.update(&relative, &document)?;"),
        "the scan's own predicate must catch a planted write"
    );
    assert!(
        writes_to_the_store("        self.store.create(&updated)?;"),
        "and a create"
    );
    assert!(
        !writes_to_the_store("    /// Calls `store.update(&relative, &document)` on its behalf."),
        "and must not fire on prose that names it"
    );
}

/// Whether line `number` (0-based) falls inside `fn commit` — the provider's one write.
///
/// Bounded by the next item at the same indentation, so the answer does not depend on how long the
/// function happens to be today.
fn within_the_write_site(source: &str, number: usize) -> bool {
    let lines: Vec<&str> = source.lines().collect();
    let Some(start) = lines.iter().position(|line| line.contains("fn commit(")) else {
        return false;
    };
    if number < start {
        return false;
    }
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, line)| line.starts_with("    fn ") || line.starts_with("    pub fn "))
        .map_or(lines.len(), |(at, _)| at);
    number < end
}
