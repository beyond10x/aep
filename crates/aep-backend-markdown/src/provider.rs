//! The plan's documents as an `entity-store` provider.
//!
//! Wave G, story 1 of `docs/plan/store-waves-f-g-h.md`: the markdown files under
//! `.engineering/planning/` held to a storage suite written by somebody who has never seen them —
//! `entity-runtime`'s — and passing it. After this, *"the markdown store is durable"* is a claim two
//! independent suites support, and `MarkdownBackend` can be the one adapter over this provider
//! (story 2) instead of a second hand-written durability layer.
//!
//! # The mapping
//!
//! ```text
//! <root>/<entity>/<id>.md            the instance: frontmatter + body
//! <root>/journal.jsonl               the event log, one DomainEvent per line, appended
//! ```
//!
//! | document | instance |
//! |---|---|
//! | the directory | `entity` — the kind, for a plan |
//! | the file stem | `id` — the name |
//! | `status:` | `lifecycle_state` |
//! | `revision:` | `revision` |
//! | every other frontmatter key | a field of the same name |
//! | the markdown body | the `body` field, absent when empty |
//!
//! `format`, `id` and `kind` are what the path and this build already say, and are not repeated as
//! fields — an instance committed as `{title}` reads back as `{title}`. They become fields only
//! where the document spells them differently: a kind filed under an accepted alias, or an id that
//! is not `<directory>:<stem>`. Numbers travel as the frontmatter's [`Node`] carries them, which is
//! floating point: `sprint: 42` reads as `42.0`.
//!
//! A document is read and written through [`PlanningDocument`], the same parser and renderer every
//! `protocol artifact` verb uses, so what this provider writes is byte for byte what the store would
//! have written — which is what lets story 2 prove that nobody can tell. The frontmatter's own
//! validation applies: a `kind` and a `status` are kebab-case words from an open vocabulary, so a
//! conformance suite's `conformance-ticket` in state `open` is as valid a document as a `story`
//! in `draft`.
//!
//! # The log, and where it begins
//!
//! `journal.jsonl` already holds this store's history in its 0.19.0 shape — one `Entry` per line:
//! who, when, artifact, revision, what changed, provenance. Those lines are left exactly as they
//! are and are read by `protocol artifact history` as before. This provider appends
//! [`DomainEvent`]s, one per line, and reads back only those; the boundary between the two shapes
//! is the first event this provider ever wrote. A document a person wrote by hand, with no journal
//! line at all, loads with an empty log — a plan that predates the provider is a normal condition,
//! and refusing it would refuse this repository's own store.
//!
//! # The order of a commit, and what a torn write leaves
//!
//! `commit` checks `Expect` against the document's `revision`, writes the document through the
//! store's temporary-file-and-rename path (one temporary per writer, `sync_all` before the rename),
//! and **then** appends the events. If the append fails, the document is at its new revision and
//! the log stops one revision short: a reader finds a document ahead of its history, which is the
//! same shape as an out-of-band edit and is what wave G's story 4 reports as drift. `rehydrate`
//! over the log rebuilds the instance the events reach — the previous revision — and refuses
//! nothing, because nothing in the log is wrong; what is missing is the line that never landed.
//! The other order — events first — would leave a recorded fact whose document did not change,
//! which reads as a lie in the one file people trust most, so this provider takes the first.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use aep_domain::artifact::{ArtifactId, ArtifactKind, ArtifactRelation, ArtifactStatus};
use aep_domain::node::Node;
use entity_core::{Decision, DomainEvent, EntityInstance};
use entity_store::{check, EventProvider, Expect, StateProvider, Store, StoreError};
use serde_json::{Map, Value};

use crate::document::PlanningDocument;
use crate::frontmatter::{PlanningFrontmatter, PLANNING_FORMAT};
use crate::journal::JOURNAL;
use crate::store::MarkdownStore;

/// The field a document's markdown body travels under.
pub const BODY_FIELD: &str = "body";

/// A directory of planning documents, answering as an `entity_store::Store`.
#[derive(Debug, Clone)]
pub struct MarkdownProvider {
    store: MarkdownStore,
}

impl MarkdownProvider {
    /// The provider rooted at `root`. The directory need not exist yet.
    pub fn open(root: impl Into<PathBuf>) -> Self {
        Self {
            store: MarkdownStore::open(root),
        }
    }

    /// The directory it reads and writes.
    pub fn root(&self) -> &Path {
        self.store.root()
    }

    /// Where the document for `entity`/`id` is.
    fn relative(entity: &str, id: &str) -> String {
        format!("{entity}/{id}.md")
    }

    /// The document for `entity`/`id`, if there is one, parsed.
    fn read(&self, entity: &str, id: &str) -> Result<Option<PlanningDocument>, StoreError> {
        let path = self.store.root().join(entity).join(format!("{id}.md"));
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(backend("reading", &path, &error)),
        };
        PlanningDocument::parse(&text, Some(&Self::relative(entity, id)))
            .map(Some)
            .map_err(|error| backend("parsing", &path, &error))
    }
}

/// Turns any IO or parse failure into a backend error carrying what it was doing.
fn backend(operation: &str, path: &Path, error: &impl std::fmt::Display) -> StoreError {
    StoreError::Backend(format!("{operation} {}: {error}", path.display()))
}

/// The instance a document stands for.
#[must_use]
pub fn instance_of(entity: &str, id: &str, document: &PlanningDocument) -> EntityInstance {
    let frontmatter = &document.frontmatter;
    let mut fields = Map::new();
    // What the path already says is not repeated as a field, so an instance committed with
    // `{title}` reads back as `{title}` — the runtime's suite compares the two. `format` is a
    // constant of this build; `id` and `kind` are the directory and the stem, and are carried
    // only where the document spells them differently: a kind filed under an accepted alias, or an
    // id that is not `<directory>:<stem>` (which the store itself reports as misfiled).
    let derived = format!("{entity}:{id}");
    if frontmatter.id.to_string() != derived {
        fields.insert("id".to_owned(), Value::from(frontmatter.id.to_string()));
    }
    if frontmatter.kind.as_str() != entity {
        fields.insert("kind".to_owned(), Value::from(frontmatter.kind.as_str()));
    }
    for (key, value) in [
        ("title", &frontmatter.title),
        ("summary", &frontmatter.summary),
        ("owner", &frontmatter.owner),
    ] {
        if let Some(value) = value {
            fields.insert(key.to_owned(), Value::from(value.clone()));
        }
    }
    if !frontmatter.tags.is_empty() {
        fields.insert(
            "tags".to_owned(),
            Value::Array(frontmatter.tags.iter().cloned().map(Value::from).collect()),
        );
    }
    if !frontmatter.relations.is_empty() {
        fields.insert(
            "relations".to_owned(),
            serde_json::to_value(&frontmatter.relations).unwrap_or(Value::Null),
        );
    }
    for (key, node) in &frontmatter.extra {
        fields.insert(
            key.clone(),
            serde_json::to_value(node).unwrap_or(Value::Null),
        );
    }
    if !document.body.is_empty() {
        fields.insert(BODY_FIELD.to_owned(), Value::from(document.body.clone()));
    }
    EntityInstance {
        entity: entity.to_owned(),
        version: 1,
        id: id.to_owned(),
        lifecycle_state: frontmatter.status.as_str().to_owned(),
        revision: frontmatter.revision,
        fields,
    }
}

/// The document an instance stands for — the inverse of [`instance_of`].
///
/// # Errors
///
/// [`StoreError::Backend`] when the instance is not a document this store can hold: an `id` field
/// that does not name this file, a `kind` that is not the directory, a `format` this build does not
/// write, a non-text body, a field of the wrong shape, or revision `0`.
pub fn document_of(instance: &EntityInstance) -> Result<PlanningDocument, StoreError> {
    let refuse = |detail: String| {
        StoreError::Backend(format!(
            "`{}/{}` is not a document this store can hold: {detail}",
            instance.entity, instance.id
        ))
    };
    let mut fields = instance.fields.clone();

    let body = match fields.remove(BODY_FIELD) {
        None => String::new(),
        Some(Value::String(body)) => body,
        Some(other) => return Err(refuse(format!("the body is {other}, not text"))),
    };

    let (artifact, kind) = identity_of(instance, &mut fields).map_err(refuse)?;

    let status: ArtifactStatus = instance
        .lifecycle_state
        .parse()
        .map_err(|error| refuse(format!("the state `{}`: {error}", instance.lifecycle_state)))?;

    let mut text = |key: &str| -> Result<Option<String>, StoreError> {
        match fields.remove(key) {
            None => Ok(None),
            Some(Value::String(value)) => Ok(Some(value)),
            Some(other) => Err(refuse(format!("the `{key}` field is {other}, not text"))),
        }
    };
    let title = text("title")?;
    let summary = text("summary")?;
    let owner = text("owner")?;

    let tags = tags_of(&mut fields).map_err(refuse)?;

    let relations: Vec<ArtifactRelation> = match fields.remove("relations") {
        None => Vec::new(),
        Some(value) => serde_json::from_value(value)
            .map_err(|error| refuse(format!("the `relations` field: {error}")))?,
    };

    if instance.revision == 0 {
        return Err(refuse(
            "revision 0 names a state before the document was written".to_owned(),
        ));
    }

    let mut extra = BTreeMap::new();
    for (key, value) in fields {
        let node: Node = serde_json::from_value(value)
            .map_err(|error| refuse(format!("the `{key}` field: {error}")))?;
        extra.insert(key, node);
    }

    Ok(PlanningDocument {
        frontmatter: PlanningFrontmatter {
            id: artifact,
            kind,
            status,
            title,
            summary,
            owner,
            tags,
            relations,
            revision: instance.revision,
            extra,
        },
        body,
    })
}

/// The artifact id and kind an instance is filed under, checked against the path.
///
/// The `id` field, when present, must name this file; the `kind` field, when present, must be the
/// directory's kind — possibly through an accepted alias (`adr/` for `architecture-decision-record`),
/// the same rule the store's own path check applies. A `format` field must be this build's.
fn identity_of(
    instance: &EntityInstance,
    fields: &mut Map<String, Value>,
) -> Result<(ArtifactId, ArtifactKind), String> {
    let artifact = match fields.remove("id") {
        None => ArtifactId::new(format!("{}:{}", instance.entity, instance.id))
            .map_err(|error| error.to_string())?,
        Some(Value::String(id)) => ArtifactId::new(&id).map_err(|error| error.to_string())?,
        Some(other) => return Err(format!("the `id` field is {other}, not text")),
    };
    if artifact.namespace() != instance.entity || artifact.name() != instance.id {
        return Err(format!(
            "the `id` field names `{artifact}`, and a document is filed where its id says — \
             `{}/{}.md`",
            artifact.namespace(),
            artifact.name()
        ));
    }
    let kind: ArtifactKind = match fields.remove("kind") {
        None => instance
            .entity
            .parse()
            .map_err(|error| format!("{error}"))?,
        Some(Value::String(kind)) => kind.parse().map_err(|error| format!("{error}"))?,
        Some(other) => return Err(format!("the `kind` field is {other}, not text")),
    };
    let directory: ArtifactKind = instance
        .entity
        .parse()
        .map_err(|error| format!("the directory `{}`: {error}", instance.entity))?;
    if directory != kind {
        return Err(format!(
            "the `kind` field says `{kind}` and the directory says `{directory}`"
        ));
    }
    if let Some(format) = fields.remove("format") {
        if format != PLANNING_FORMAT {
            return Err(format!(
                "the `format` field is {format}; this build writes `{PLANNING_FORMAT}`"
            ));
        }
    }
    Ok((artifact, kind))
}

/// The `tags` field as the frontmatter's set, or nothing.
fn tags_of(fields: &mut Map<String, Value>) -> Result<BTreeSet<String>, String> {
    match fields.remove("tags") {
        None => Ok(BTreeSet::new()),
        Some(Value::Array(tags)) => tags
            .into_iter()
            .map(|tag| match tag {
                Value::String(tag) => Ok(tag),
                other => Err(format!("a tag is {other}, not text")),
            })
            .collect(),
        Some(other) => Err(format!("the `tags` field is {other}, not a list")),
    }
}

impl StateProvider for MarkdownProvider {
    fn load(&self, entity: &str, id: &str) -> Result<Option<EntityInstance>, StoreError> {
        Ok(self
            .read(entity, id)?
            .map(|document| instance_of(entity, id, &document)))
    }

    fn ids(&self, entity: &str) -> Result<Vec<String>, StoreError> {
        let directory = self.store.root().join(entity);
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(backend("listing", &directory, &error)),
        };
        let mut ids = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| backend("listing", &directory, &error))?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            // A temporary a write goes through starts with a dot and is not a document yet.
            if name.starts_with('.') {
                continue;
            }
            if let Some(stem) = name.strip_suffix(".md") {
                ids.push(stem.to_owned());
            }
        }
        ids.sort();
        Ok(ids)
    }
}

impl EventProvider for MarkdownProvider {
    fn events(&self, entity: &str, id: &str) -> Result<Vec<DomainEvent>, StoreError> {
        let path = self.store.root().join(JOURNAL);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(backend("reading", &path, &error)),
        };
        // Only the lines that are events. The journal's older entries are another shape, read by
        // another reader (`crate::journal`), and a line that is neither is a half-written line from
        // a killed process — skipped here as it is there, rather than making the log unreadable.
        Ok(text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str::<DomainEvent>(line).ok())
            .filter(|event| event.entity == entity && event.id == id)
            .collect())
    }
}

impl Store for MarkdownProvider {
    fn commit(&mut self, decision: &Decision, expect: Expect) -> Result<(), StoreError> {
        let instance = &decision.instance;
        let (entity, id) = (instance.entity.as_str(), instance.id.as_str());

        // Checked before anything is written, so a refusal leaves both files exactly as they were.
        let existing = self.read(entity, id)?;
        check(
            entity,
            id,
            expect,
            existing.as_ref().map(|held| held.frontmatter.revision),
        )?;

        let document = document_of(instance)?;
        let written = if existing.is_some() {
            self.store.update(&Self::relative(entity, id), &document)
        } else {
            self.store.create(&document)
        };
        written.map_err(|error| StoreError::Backend(error.to_string()))?;

        // Events after the document: see the module doc for what a failure here leaves.
        if decision.events.is_empty() {
            return Ok(());
        }
        let path = self.store.root().join(JOURNAL);
        let mut lines = String::new();
        for event in &decision.events {
            lines.push_str(
                &serde_json::to_string(event).map_err(|error| {
                    StoreError::Backend(format!("serialising an event: {error}"))
                })?,
            );
            lines.push('\n');
        }
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut file| file.write_all(lines.as_bytes()))
            .map_err(|error| backend("appending to", &path, &error))
    }
}
