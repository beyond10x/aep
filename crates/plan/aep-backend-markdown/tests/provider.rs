//! The plan's documents, held to `entity-runtime`'s provider suite — a suite written by somebody who
//! has never seen them.

use std::path::{Path, PathBuf};

use aep_backend_markdown::provider::{document_of, instance_of, MarkdownProvider};
use aep_backend_markdown::store::MarkdownStore;
use entity_core::{Decision, DomainEvent, EntityInstance, Registry, Runtime};
use entity_store::{conformance, EventProvider, Expect, StateProvider, Store, StoreError};
use serde_json::json;

fn scratch(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("provider")
        .join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch directory");
    root
}

/// The provider over `root`, with its evidence beside it, as `aep.project/5` keeps it.
fn open(root: impl AsRef<Path>) -> MarkdownProvider {
    let root = root.as_ref();
    MarkdownProvider::open_git(root, root.with_extension("evidence"))
}

fn registry() -> Registry {
    let definition = serde_json::from_value(json!({
        "entity": "ticket",
        "version": 1,
        "schema": { "fields": { "title": { "type": "string", "required": true } } },
        "lifecycle": { "initial": "open", "states": ["open", "closed"] },
        "create": { "emit": { "type": "TicketOpened", "payload": { "ticket": "$id" } } },
        "operations": {
            "close": {
                "transitions": [{ "from": "open", "to": "closed" }],
                "emits": [{ "type": "TicketClosed", "payload": { "ticket": "$id" } }]
            }
        }
    }))
    .expect("the definition parses");
    let mut registry = Registry::new();
    registry.register(definition).expect("it validates");
    registry
}

/// The runtime's cases that read the event log back. The Git-native layout keeps no event log —
/// a move is a transition in its document and an observation is an evidence file — so these are
/// the cases it fails by design, and the only ones.
const EVENT_LOG_CASES: &[&str] = &[
    "state and events arrive together",
    "an observation lands at an unchanged revision, and so does the next",
    "an ordered batch sees its own earlier revision",
];

/// The cases `report` failed, each checked to be one that reads the event log back.
fn only_event_log_cases_fail(failed: &[&str], summary: &str) {
    for case in failed {
        assert!(
            EVENT_LOG_CASES.contains(case),
            "`{case}` failed, and it is not an event-log case:\n{summary}"
        );
    }
}

#[test]
fn the_markdown_provider_passes_every_state_case_of_the_provider_suite() {
    let mut provider = open(scratch("conforms"));
    let report = conformance::run(&mut provider);
    assert_eq!(report.outcomes.len(), 10, "the whole suite ran");
    let failed: Vec<&str> = report.failures().iter().map(|o| o.case).collect();
    only_event_log_cases_fail(&failed, &report.summary());
    assert!(
        report.outcomes.len() - failed.len() >= 8,
        "every state case passes:\n{}",
        report.summary()
    );
}

#[test]
fn the_markdown_provider_commits_atomic_batches() {
    let mut provider = open(scratch("atomic-conformance"));
    let report = conformance::run_atomic(&mut provider);
    let failed: Vec<&str> = report.failures().iter().map(|o| o.case).collect();
    only_event_log_cases_fail(&failed, &report.summary());
}

/// A copy of the provider that ignores the revision it was given — the runtime's `Broken`, written
/// over this provider so the suite is shown to catch *this* store being wrong, not only the
/// runtime's own memory store.
struct Broken(MarkdownProvider);

impl StateProvider for Broken {
    fn load(&self, entity: &str, id: &str) -> Result<Option<EntityInstance>, StoreError> {
        self.0.load(entity, id)
    }
    fn ids(&self, entity: &str) -> Result<Vec<String>, StoreError> {
        self.0.ids(entity)
    }
}
impl EventProvider for Broken {
    fn events(&self, entity: &str, id: &str) -> Result<Vec<DomainEvent>, StoreError> {
        self.0.events(entity, id)
    }
}
impl Store for Broken {
    fn commit(&mut self, decision: &Decision, _expect: Expect) -> Result<(), StoreError> {
        let expect = match self
            .0
            .load(&decision.instance.entity, &decision.instance.id)?
        {
            Some(held) => Expect::Revision(held.revision),
            None => Expect::Absent,
        };
        self.0.commit(decision, expect)
    }
}

#[test]
fn a_broken_copy_of_the_provider_is_caught() {
    let mut broken = Broken(open(scratch("broken")));
    let report = conformance::run(&mut broken);
    assert!(
        !report.is_clean(),
        "a provider ignoring revisions must not pass"
    );
    let caught: Vec<&str> = report.failures().iter().map(|o| o.case).collect();
    assert!(
        caught.contains(&"a stale write is refused"),
        "the stale-write case is the one that must catch it: {caught:?}"
    );
    assert!(
        report.failures().len() < report.outcomes.len(),
        "and the suite localises the defect rather than condemning the provider"
    );
}

#[test]
fn a_refused_commit_leaves_the_document_as_it_was() {
    // R-84: the bytes of the document, before and after.
    let root = scratch("refused");
    let mut provider = open(&root);
    let registry = registry();
    let created = Runtime::new(&registry)
        .create("ticket", 1, "one", json!({ "title": "A ticket" }))
        .expect("permitted");
    provider.commit(&created, Expect::Absent).expect("accepted");
    let document = root.join("ticket/one.md");
    let document_before = std::fs::read(&document).expect("the document landed");

    let closed = Runtime::new(&registry)
        .execute(&created.instance, "close", json!({}))
        .expect("permitted");
    let error = provider
        .commit(&closed, Expect::Revision(7))
        .expect_err("nothing is at revision 7");
    assert!(
        matches!(error, StoreError::RevisionConflict { found: Some(1), .. }),
        "{error}"
    );

    assert_eq!(
        std::fs::read(&document).expect("still there"),
        document_before
    );
}

#[test]
fn a_document_a_person_wrote_by_hand_loads_with_an_empty_log() {
    let root = scratch("by-hand");
    std::fs::create_dir_all(root.join("story")).expect("a kind directory");
    std::fs::write(
        root.join("story/passkey-login.md"),
        "---\nid: story:passkey-login\nkind: story\nstatus: draft\ntitle: Passkey login\ntags:\n- auth\nsprint: 42\n---\n# Story: Passkey login\n\nSome prose.\n",
    )
    .expect("a hand-written document");
    let provider = open(&root);

    let held = provider
        .load("story", "passkey-login")
        .expect("answers")
        .expect("the document is an instance");
    assert_eq!(held.lifecycle_state, "draft");
    assert_eq!(held.revision, 1, "no `revision:` reads as the first");
    assert_eq!(held.fields["title"], "Passkey login");
    assert_eq!(held.fields["tags"], json!(["auth"]));
    assert_eq!(
        held.fields["sprint"].as_f64(),
        Some(42.0),
        "a key this format does not name is still a field (numbers travel as the frontmatter's \
         floating-point `Node` carries them)"
    );
    assert!(
        !held.fields.contains_key("id") && !held.fields.contains_key("kind"),
        "what the path says is not repeated as a field: {:?}",
        held.fields
    );
    assert_eq!(
        held.fields["body"],
        "# Story: Passkey login\n\nSome prose.\n"
    );
    assert!(
        provider
            .events("story", "passkey-login")
            .expect("answers")
            .is_empty(),
        "the Git-native layout keeps no event log, and reading one is not an error"
    );
    assert_eq!(provider.ids("story").expect("answers"), ["passkey-login"]);
}

#[test]
fn this_repositorys_own_store_loads_and_every_document_round_trips_byte_for_byte() {
    // The mapping loses nothing: for every document in this repository's plan, the instance it
    // stands for renders back to the same bytes the store would have written. Story 2's golden
    // test rests on this.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.engineering/planning");
    let provider = open(&root);
    let report = MarkdownStore::open(&root).load();
    assert!(report.is_clean(), "this repository's plan reads cleanly");
    assert!(report.documents.len() > 100, "the fixture is the real plan");

    let mut checked = 0;
    for stored in report.documents.values() {
        let (entity, rest) = stored.relative_path.split_once('/').expect("kind/name.md");
        let id = rest.strip_suffix(".md").expect("a markdown file");
        let held = provider
            .load(entity, id)
            .expect("answers")
            .unwrap_or_else(|| panic!("{}: listed and absent", stored.relative_path));
        let mut document =
            document_of(&held).unwrap_or_else(|error| panic!("{}: {error}", stored.relative_path));
        // An instance carries no format: the format belongs to the projection, and this
        // repository's is a tree store's, rendered `aep.planning-md/2`.
        document.frontmatter.format = stored.document.frontmatter.format;
        let rendered = document.render();
        assert_eq!(
            rendered,
            stored.document.render(),
            "{}: the round trip changed the bytes",
            stored.relative_path
        );
        assert_eq!(held, instance_of(entity, id, &stored.document));
        checked += 1;
    }
    assert_eq!(checked, report.documents.len());
    assert!(
        provider
            .ids("story")
            .expect("answers")
            .contains(&"one-entity-runtime-pin".to_owned()),
        "the plan's stories are listed"
    );
}

#[test]
fn a_commit_leaves_no_temporary_behind() {
    // The store writes through one temporary per writer (pid + counter) and renames it into place;
    // a leftover would be a document-shaped file the loader skips only because it starts with a dot.
    let root = scratch("no-temporaries");
    let mut provider = open(&root);
    let registry = registry();
    let created = Runtime::new(&registry)
        .create("ticket", 1, "one", json!({ "title": "A ticket" }))
        .expect("permitted");
    provider.commit(&created, Expect::Absent).expect("accepted");
    let closed = Runtime::new(&registry)
        .execute(&created.instance, "close", json!({}))
        .expect("permitted");
    provider
        .commit(&closed, Expect::Revision(1))
        .expect("accepted");
    let leftovers: Vec<_> = std::fs::read_dir(root.join("ticket"))
        .expect("readable")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with('.'))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}
