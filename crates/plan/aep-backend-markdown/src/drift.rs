//! A document compared with the last thing its log says happened to it.
//!
//! Wave G, story 4 of `docs/plan/store-waves-f-g-h.md`. Deviations **D-P2** (*an out-of-band file
//! edit is not tracked*) and **D-P4** (*`rm` deletes an artifact, and nothing prevents it*) were
//! opened with the store, when there was nothing to compare a file against. Now there is: runtime
//! R-89 says an event records the state before and after and the fields written, so the last event
//! of an instance says what its document should contain.
//!
//! Four findings, kept apart because they send a person to four different places:
//!
//! | finding | what it means | exit |
//! |---|---|---|
//! | **drift** | the document's frontmatter disagrees with its last event — somebody edited `status:`, `title:`, an edge, in an editor | 1 |
//! | **forged revision** | the document claims a revision no logged write produced — higher than any event for it records | 1 |
//! | **deleted** | the log holds events for a document that is not there — somebody `rm`ed it instead of moving it to `archived` | 1 |
//! | **pre-provider** | the document has no events at all — it predates the provider, a normal condition and not a defect | 0 |
//!
//! # A revision nothing wrote
//!
//! An event carries the revision the instance was at *after* the write that emitted it, so the
//! highest revision the log holds for a document is the revision the last command left behind. A
//! document claiming more than that is claiming a write that never happened — `revision: 99` beside
//! a log whose last event is revision 1. Ordinary drift already reported the disagreement, but as
//! *somebody edited a field*, which is the wrong place to send the reader: nothing recovers the
//! state a forged revision claims, because there is no such state. It is its own finding for that
//! reason, and only that reason.
//!
//! It is **not** the same test as *fewer events than the revision*. A store's history predates the
//! provider — the journal's older entries are another shape entirely — and an evidence record is an
//! event at the current revision that writes nothing, so counting events answers neither question.
//! The comparison is against the highest revision **recorded**, and a document at or below it is
//! not forged however many events sit under it.
//!
//! This detects and does **not** enforce. Nothing here refuses a write, and `protocol artifact
//! validate` grew no refusal: a forged revision is reported after the fact, exactly as an
//! out-of-band edit is. Enforcement needs to know who wrote a document, which is gap register
//! **D-3** (attestation by signature) and is still proposed.
//!
//! # Detection, and the prevention that was refused
//!
//! Both deviations close **by detection**. Prevention was considered and refused, on the record: a
//! `PreToolUse` hook is bypassed by `Bash` (the design's own § 3.3 says so), and a lock on a
//! directory of markdown files is a lock somebody deletes. A check that runs in the gate cannot be
//! routed around, and `protocol artifact validate` is that check. Repairing drift is not here
//! either: a repair is a write, and a write is a command somebody issues — `protocol artifact move`
//! with the ladder consulted, not a verb that copies the file's claim into the log.
//!
//! The body is not compared. It is a person's prose, and editing it in an editor is what the format
//! is for; what the log is authoritative about is the frontmatter a command writes.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

use aep_domain::artifact::ArtifactId;
use entity_core::DomainEvent;
use entity_store::EventProvider as _;

use crate::journal::JOURNAL;
use crate::provider::{instance_of, MarkdownProvider, BODY_FIELD};
use crate::store::StoredDocument;

/// A document whose frontmatter disagrees with its last event.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Drift {
    /// Which artifact.
    pub artifact: ArtifactId,
    /// Which fields disagree — `status`, `revision`, or a frontmatter key the event wrote.
    pub fields: Vec<String>,
    /// The event the document disagrees with, by its id.
    pub event: String,
}

impl fmt::Display for Drift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} drifted from its log: {} disagree{} with event {} — an edit made outside a command \
             is a change nothing decided",
            self.artifact,
            self.fields.join(", "),
            if self.fields.len() == 1 { "s" } else { "" },
            self.event
        )
    }
}

/// A document claiming a revision no logged write produced.
///
/// Separate from [`Drift`] because it sends the reader somewhere else. Drift says *the document and
/// the log disagree about a field*, and the log is the record to trust; this says *the document
/// claims a write that is not in the log at all*, and there is no earlier state to restore it to.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ForgedRevision {
    /// Which artifact.
    pub artifact: ArtifactId,
    /// The revision the frontmatter claims.
    pub claimed: u64,
    /// The highest revision any event for it records — what the last logged write left behind.
    pub logged: u64,
    /// How many events the log holds for it, said out loud so the reader can see how thin the
    /// record is without opening it.
    pub events: usize,
    /// The last event that changed anything, by its id.
    pub event: String,
}

impl fmt::Display for ForgedRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} claims revision {}, and no write produced it: {} event(s) logged, the highest at \
             revision {} (event {}) — a revision above the log's own was written by hand, not by a \
             command",
            self.artifact, self.claimed, self.events, self.logged, self.event
        )
    }
}

/// A document the log has events for and the store does not hold.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Deleted {
    /// Which artifact.
    pub artifact: ArtifactId,
    /// Its last event, by id.
    pub event: String,
}

impl fmt::Display for Deleted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} was deleted: its log ends at event {} and the store holds no document — nothing is \
             physically deleted through a command, so this was `rm`. Restore the document from \
             version control, then retire it with `protocol artifact move {} --to archived`, which \
             keeps its record",
            self.artifact, self.event, self.artifact
        )
    }
}

/// What one comparison of the documents against the log found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Documents disagreeing with their last event.
    pub drift: Vec<Drift>,
    /// Documents claiming a revision no logged write produced.
    pub forged: Vec<ForgedRevision>,
    /// Documents the log knows and the store no longer holds.
    pub deleted: Vec<Deleted>,
    /// Documents with no events at all.
    pub pre_provider: usize,
}

/// Compares every document with the last event the log holds for it, and the log with the store.
///
/// `documents` is what the store holds, as [`crate::store::MarkdownStore::load`] reports it.
#[must_use]
pub fn detect(root: &Path, documents: &BTreeMap<ArtifactId, StoredDocument>) -> Report {
    let provider = MarkdownProvider::open(root);
    let mut report = Report::default();

    // The coordinates every event line names, from one read of the journal — the deleted documents
    // are the ones the log knows and the store does not.
    let mut logged: BTreeMap<(String, String), DomainEvent> = BTreeMap::new();
    if let Ok(text) = std::fs::read_to_string(root.join(JOURNAL)) {
        for line in text.lines() {
            if let Ok(event) = serde_json::from_str::<DomainEvent>(line) {
                logged.insert((event.entity.clone(), event.id.clone()), event);
            }
        }
    }
    let mut held: BTreeSet<(String, String)> = BTreeSet::new();

    for stored in documents.values() {
        let Some((directory, name)) = coordinates(&stored.relative_path) else {
            continue;
        };
        held.insert((directory.clone(), name.clone()));
        let events = provider.events(&directory, &name).unwrap_or_default();
        // The last event that changed something. An observation about the document — an evidence
        // record — is an event at its current revision that wrote nothing, and is not what the
        // document should look like.
        let Some(last) = events
            .iter()
            .rev()
            .find(|event| {
                !(event.changed.is_empty()
                    && event.from_state.as_deref() == Some(event.to_state.as_str()))
            })
            .or(events.last())
        else {
            report.pre_provider += 1;
            continue;
        };
        let instance = instance_of(&directory, &name, &stored.document);
        let mut fields = Vec::new();
        if instance.lifecycle_state != last.to_state {
            fields.push("status".to_owned());
        }
        // The revision the last logged write left behind. An event carries the revision *after*
        // its operation, so nothing a command did can put the document above this number.
        let logged = events
            .iter()
            .map(|event| event.revision)
            .max()
            .unwrap_or(last.revision);
        if instance.revision > logged {
            // Said once, and as the thing it is: a revision nothing wrote is not a field somebody
            // edited, and reporting it as ordinary drift would send the reader to the log for a
            // state that was never in it.
            report.forged.push(ForgedRevision {
                artifact: stored.document.frontmatter.id.clone(),
                claimed: instance.revision,
                logged,
                events: events.len(),
                event: event_id(last),
            });
        } else if instance.revision != logged {
            // Against the highest revision the log records, not against `last`: a write that
            // changed nothing — `artifact body` handed an empty body — is still a write, at a
            // revision the log holds, and a document standing at it is where its log left it.
            fields.push("revision".to_owned());
        }
        // The fold of the events: every field any event wrote, the latest value winning. A field
        // no event ever wrote — a title given by a verb that predates the log — is not the log's
        // to judge, and is left alone.
        let mut expected: BTreeMap<&String, &serde_json::Value> = BTreeMap::new();
        for event in &events {
            for (field, value) in &event.changed {
                expected.insert(field, value);
            }
        }
        for (field, value) in expected {
            if field == BODY_FIELD {
                continue;
            }
            let held = instance.fields.get(field);
            // **`null` in the log is a field a command removed.** Every list this format carries —
            // `tags`, `refs`, `scope` — is omitted from the document when it is empty, so taking
            // the last entry out leaves an instance with no such key, which
            // `aep_backend_entity::changed_between` records as `null`. Absent and `null` are the
            // same claim to a document that omits its empty lists, so either satisfies it — and a
            // document that still carries the field is drift, exactly as before.
            if value.is_null() {
                if held.is_some_and(|held| !held.is_null()) {
                    fields.push(field.clone());
                }
                continue;
            }
            if held != Some(value) {
                fields.push(field.clone());
            }
        }
        if !fields.is_empty() {
            report.drift.push(Drift {
                artifact: stored.document.frontmatter.id.clone(),
                fields,
                event: event_id(last),
            });
        }
    }

    for ((entity, id), last) in &logged {
        if held.contains(&(entity.clone(), id.clone())) {
            continue;
        }
        if let Ok(artifact) = ArtifactId::new(format!("{entity}:{id}")) {
            report.deleted.push(Deleted {
                artifact,
                event: event_id(last),
            });
        }
    }

    report
}

/// `<directory>/<name>.md` as the provider's coordinates.
fn coordinates(relative_path: &str) -> Option<(String, String)> {
    let stem = relative_path.strip_suffix(".md")?;
    let (directory, name) = stem.split_once('/')?;
    Some((directory.to_owned(), name.to_owned()))
}

/// The seal's event id, or the event's coordinates when a line carries no seal.
fn event_id(event: &DomainEvent) -> String {
    event
        .payload
        .get("event_id")
        .and_then(serde_json::Value::as_str)
        .map_or_else(
            || format!("{}:{}@{}", event.entity, event.id, event.revision),
            ToOwned::to_owned,
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::store::MarkdownStore;

    /// A scratch tree this process alone owns.
    ///
    /// **The pid is not decoration** — the reason is written out on `store.rs`'s own `scratch`:
    /// `temp_dir()` is one directory for every session and every worktree on a machine, and the
    /// first thing this function does is delete it.
    fn scratch(name: &str) -> MarkdownStore {
        let root = std::env::temp_dir()
            .join("aep-markdown-drift")
            .join(format!("{}-{}", std::process::id(), name));
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(&root).expect("the scratch tree is writable");
        MarkdownStore::open(root)
    }

    /// One document, with whatever revision and status the test needs it to claim.
    fn document(store: &MarkdownStore, name: &str, status: &str, revision: u64) {
        let path = store.root().join("story").join(format!("{name}.md"));
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("writable");
        std::fs::write(
            path,
            format!(
                "---\nformat: aep.planning-md/1\nid: story:{name}\nkind: story\nstatus: \
                 {status}\ntitle: A story\nrevision: {revision}\n---\n# A story\n"
            ),
        )
        .expect("writable");
    }

    /// One event line, in the runtime's shape, appended to the journal.
    fn event(
        store: &MarkdownStore,
        name: &str,
        revision: u64,
        from_state: Option<&str>,
        changed: &serde_json::Value,
    ) {
        let line = serde_json::json!({
            "entity": "story",
            "version": 1,
            "id": name,
            "revision": revision,
            "type": "aep.entity.update/v1",
            "from_state": from_state,
            "to_state": "draft",
            "changed": changed.clone(),
            "args": {},
            "payload": { "event_id": format!("story:{name}@{revision}") },
        });
        let path = store.root().join(JOURNAL);
        let mut text = std::fs::read_to_string(&path).unwrap_or_default();
        text.push_str(&line.to_string());
        text.push('\n');
        std::fs::write(path, text).expect("writable");
    }

    fn report(store: &MarkdownStore) -> Report {
        detect(store.root(), &store.load().documents)
    }

    /// Clearing the last entry of a list is a write, and a write is not drift.
    ///
    /// The document carries no `tags:` key once the last tag is gone, and the event that removed it
    /// says `tags: null`. Reading those as a disagreement turned `artifact set --untag` and
    /// `artifact scope --remove` into commands that made `validate` report their own work as an
    /// edit made outside a command.
    #[test]
    fn a_list_a_command_emptied_is_not_reported_as_an_edit_outside_a_command() {
        let store = scratch("emptied");
        document(&store, "one", "draft", 2);
        event(
            &store,
            "one",
            1,
            None,
            &serde_json::json!({ "title": "A story", "tags": ["one"] }),
        );
        event(
            &store,
            "one",
            2,
            Some("draft"),
            &serde_json::json!({ "tags": null }),
        );

        let found = report(&store);
        assert!(
            found.drift.is_empty(),
            "the document holds no `tags:` because the command took the last one out: {found:?}"
        );

        // And the rule is load-bearing only where the log **says** the field was removed: a
        // document that lost a tag no event took out is drift, exactly as it was.
        let store = scratch("emptied-partly");
        document(&store, "one", "draft", 2);
        event(
            &store,
            "one",
            1,
            None,
            &serde_json::json!({ "title": "A story" }),
        );
        event(
            &store,
            "one",
            2,
            Some("draft"),
            &serde_json::json!({ "tags": ["one"] }),
        );
        let found = report(&store);
        assert_eq!(found.drift.len(), 1, "{found:?}");
        assert_eq!(found.drift[0].fields, vec!["tags".to_owned()]);
    }

    #[test]
    fn a_revision_above_every_event_is_forged_and_is_not_reported_as_ordinary_drift() {
        let store = scratch("forged");
        document(&store, "one", "draft", 99);
        event(
            &store,
            "one",
            1,
            None,
            &serde_json::json!({ "title": "A story" }),
        );

        let found = report(&store);
        assert_eq!(found.forged.len(), 1, "{found:?}");
        let forged = &found.forged[0];
        assert_eq!(forged.claimed, 99);
        assert_eq!(forged.logged, 1, "the one event the log holds");
        assert_eq!(forged.events, 1);
        assert!(
            found.drift.is_empty(),
            "said once, as a revision nothing wrote: {found:?}"
        );
        assert_eq!(found.pre_provider, 0);
        let said = forged.to_string();
        assert!(
            said.contains("no write produced it"),
            "the finding says what it is: {said}"
        );
        assert!(
            said.contains("1 event(s) logged"),
            "and how thin the record is: {said}"
        );
    }

    #[test]
    fn a_document_at_the_revision_its_events_record_is_not_forged() {
        let store = scratch("at-the-revision");
        document(&store, "one", "draft", 2);
        event(
            &store,
            "one",
            1,
            None,
            &serde_json::json!({ "title": "A story" }),
        );
        event(
            &store,
            "one",
            2,
            Some("draft"),
            &serde_json::json!({ "title": "A story" }),
        );

        let found = report(&store);
        assert!(found.forged.is_empty(), "{found:?}");
        assert!(found.drift.is_empty(), "{found:?}");
    }

    #[test]
    fn a_document_at_the_revision_of_a_write_that_changed_nothing_is_not_drift() {
        // Seen on a driven run, 2026-08-29: `protocol artifact body` handed an empty body wrote
        // an `update` event at revision 2 with `changed: {}`, the document stood at revision 2,
        // and `validate` reported the revision as disagreeing with the *create* event at 1 —
        // because the last event that changed something was the create. A revision the log
        // records is not a revision somebody typed.
        let store = scratch("write-that-changed-nothing");
        document(&store, "one", "draft", 2);
        event(
            &store,
            "one",
            1,
            None,
            &serde_json::json!({ "title": "A story" }),
        );
        event(&store, "one", 2, Some("draft"), &serde_json::json!({}));

        let found = report(&store);
        assert!(found.forged.is_empty(), "{found:?}");
        assert!(found.drift.is_empty(), "{found:?}");
    }

    #[test]
    fn more_events_than_the_revision_is_not_forged_because_an_evidence_record_writes_nothing() {
        // The test the count-of-events reading would fail: three events, revision 1, and every one
        // of them legitimate — an observation is an event at the current revision that wrote
        // nothing, and a store's history predates the provider besides.
        let store = scratch("evidence-records");
        document(&store, "one", "draft", 1);
        event(
            &store,
            "one",
            1,
            None,
            &serde_json::json!({ "title": "A story" }),
        );
        event(&store, "one", 1, Some("draft"), &serde_json::json!({}));
        event(&store, "one", 1, Some("draft"), &serde_json::json!({}));

        let found = report(&store);
        assert!(found.forged.is_empty(), "{found:?}");
        assert!(found.drift.is_empty(), "{found:?}");
    }

    #[test]
    fn a_document_with_no_events_predates_the_log_and_is_not_forged() {
        // `pre_provider` keeps its meaning: a document the log has never heard of cannot be
        // compared with anything, whatever revision it claims. Whether a driven run's scratch store
        // should read it differently is open (native-arm-store-integrity design § 8, OQ5) and is
        // not decided here.
        let store = scratch("no-events");
        document(&store, "one", "draft", 99);

        let found = report(&store);
        assert_eq!(found.pre_provider, 1);
        assert!(found.forged.is_empty(), "{found:?}");
        assert!(found.drift.is_empty(), "{found:?}");
    }

    #[test]
    fn a_forged_revision_beside_an_edited_status_is_two_findings_and_not_one() {
        let store = scratch("forged-and-edited");
        document(&store, "one", "active", 99);
        event(
            &store,
            "one",
            1,
            None,
            &serde_json::json!({ "title": "A story" }),
        );

        let found = report(&store);
        assert_eq!(found.forged.len(), 1, "{found:?}");
        assert_eq!(found.drift.len(), 1, "{found:?}");
        assert_eq!(
            found.drift[0].fields,
            vec!["status".to_owned()],
            "the revision is the other finding's, and is not repeated here: {found:?}"
        );
    }
}
