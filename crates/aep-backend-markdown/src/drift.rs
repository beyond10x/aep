//! A document compared with the last thing its log says happened to it.
//!
//! Wave G, story 4 of `docs/plan/store-waves-f-g-h.md`. Deviations **D-P2** (*an out-of-band file
//! edit is not tracked*) and **D-P4** (*`rm` deletes an artifact, and nothing prevents it*) were
//! opened with the store, when there was nothing to compare a file against. Now there is: runtime
//! R-89 says an event records the state before and after and the fields written, so the last event
//! of an instance says what its document should contain.
//!
//! Three findings, kept apart because they send a person to three different places:
//!
//! | finding | what it means | exit |
//! |---|---|---|
//! | **drift** | the document's frontmatter disagrees with its last event — somebody edited `status:`, `title:`, an edge, in an editor | 1 |
//! | **deleted** | the log holds events for a document that is not there — somebody `rm`ed it | 1 |
//! | **pre-provider** | the document has no events at all — it predates the provider, a normal condition and not a defect | 0 |
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
             physically deleted through a command, so this was `rm`",
            self.artifact, self.event
        )
    }
}

/// What one comparison of the documents against the log found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Documents disagreeing with their last event.
    pub drift: Vec<Drift>,
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
        if instance.revision != last.revision {
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
            if instance.fields.get(field) != Some(value) {
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
