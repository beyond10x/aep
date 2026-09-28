//! The plan's documents as an `entity-store` provider.
//!
//! Wave G, story 1 of `docs/plan/archive/store-waves-f-g-h.md`: the markdown files under
//! `.engineering/planning/` held to a storage suite written by somebody who has never seen them —
//! `entity-runtime`'s — and passing it. `MarkdownBackend` is the one adapter over this provider
//! instead of a second hand-written durability layer.
//!
//! # The mapping
//!
//! ```text
//! <root>/<entity>/<id>.md                         the instance: frontmatter + body
//! <evidence>/<entity>/<id>/<instant>-<digest>.json one recorded observation, never rewritten
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
//! `aep plan artifact` verb uses. The frontmatter's own validation applies: a `kind` and a `status`
//! are kebab-case words from an open vocabulary, so a conformance suite's `conformance-ticket` in
//! state `open` is as valid a document as a `story` in `draft`.
//!
//! # No log beside the documents (`aep.project/5`)
//!
//! The documents are the authority. A commit whose event says `moved` appends a [`Transition`] to
//! the document's `transitions` before writing it; one whose event says `evidence` writes one record
//! through [`crate::journal::write_evidence`]; every other change is the document write alone.
//! Documents are written as `aep.planning-md/3` and `events` answers nothing: what happened is read
//! back from the documents and the evidence files by [`crate::journal::read_git`].
//!
//! # Single commits and command batches
//!
//! `commit` checks `Expect` against the document's `revision` and writes the document through the
//! store's temporary-file-and-rename path (one temporary per writer, `sync_all` before the rename),
//! then its evidence records.
//!
//! The stronger [`AtomicBatchStore`] path writes the complete ordered command to
//! [`.aep-batch.pending.json`](PENDING_BATCH) before applying any entry. If the process stops after
//! one document, every state and event read completes that intent idempotently before answering.
//! The pending record is removed only after the whole batch lands.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use aep_domain::artifact::{
    ArtifactId, ArtifactKind, ArtifactRelation, ArtifactStatus, ExternalRef, ScopeEntry,
};
use aep_domain::node::Node;
use entity_core::{Decision, DomainEvent, EntityInstance};
use entity_store::{
    check, AtomicBatchStore, AtomicCommit, EventProvider, Expect, StateProvider, Store, StoreError,
};
use serde_json::{Map, Value};

use crate::document::PlanningDocument;
use crate::frontmatter::{PlanningFormat, PlanningFrontmatter};
use crate::journal::{Change, Entry, Transition};
use crate::store::MarkdownStore;

/// The field a document's markdown body travels under.
pub const BODY_FIELD: &str = "body";

/// The field a document's `transitions` travel under — present only when there are some.
pub const TRANSITIONS_FIELD: &str = "transitions";

/// The recoverable intent that makes a multi-document command one logical write.
pub const PENDING_BATCH: &str = ".aep-batch.pending.json";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingBatch {
    commits: Vec<PendingCommit>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingCommit {
    decision: Decision,
    expect: PendingExpect,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum PendingExpect {
    Absent,
    Revision(u64),
}

impl From<Expect> for PendingExpect {
    fn from(expect: Expect) -> Self {
        match expect {
            Expect::Absent => Self::Absent,
            Expect::Revision(revision) => Self::Revision(revision),
        }
    }
}

impl From<PendingExpect> for Expect {
    fn from(expect: PendingExpect) -> Self {
        match expect {
            PendingExpect::Absent => Self::Absent,
            PendingExpect::Revision(revision) => Self::Revision(revision),
        }
    }
}

/// A store shaped like a plan: kinds as entity types, names as ids, documents as instances.
///
/// What [`crate::projection::MarkdownProjection`] hydrates from and writes to; [`MarkdownProvider`]
/// is the one this crate has. The two things a `Store` cannot say are asked here: which kinds there
/// are (the SPI enumerates ids under one entity type, never the types), and where the documents
/// are, for a message.
pub trait PlanStore: Store {
    /// The directory the plan's documents are in.
    fn root(&self) -> &Path;

    /// Every kind that has a directory, sorted.
    ///
    /// # Errors
    ///
    /// If the directory cannot be listed.
    fn kinds(&self) -> Result<Vec<String>, StoreError>;
}

impl PlanStore for MarkdownProvider {
    fn root(&self) -> &Path {
        Self::root(self)
    }

    fn kinds(&self) -> Result<Vec<String>, StoreError> {
        let root = self.store.root();
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(backend("listing", root, &error)),
        };
        let mut kinds = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| backend("listing", root, &error))?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if name.starts_with('.') || !entry.path().is_dir() {
                continue;
            }
            // Evidence kept under the documents' own root is not a kind of document.
            if self.evidence == entry.path() {
                continue;
            }
            kinds.push(name.to_owned());
        }
        kinds.sort();
        Ok(kinds)
    }
}

/// A directory of planning documents, answering as an `entity_store::Store`.
#[derive(Debug, Clone)]
pub struct MarkdownProvider {
    store: MarkdownStore,
    /// The directory evidence records are written under, `<kind>/<name>/<record>.json`.
    evidence: PathBuf,
}

impl MarkdownProvider {
    /// The provider rooted at `root`, writing evidence under `evidence`.
    ///
    /// `commit` writes the document — with a status move appended to its `transitions` — and, for
    /// an observation, one evidence file. Neither directory need exist yet.
    pub fn open_git(root: impl Into<PathBuf>, evidence: impl Into<PathBuf>) -> Self {
        Self {
            store: MarkdownStore::open(root),
            evidence: evidence.into(),
        }
    }

    /// The directory it reads and writes.
    pub fn root(&self) -> &Path {
        self.store.root()
    }

    /// The directory it writes evidence records under.
    pub fn evidence(&self) -> &Path {
        &self.evidence
    }

    /// The instance a decision leaves on disk and the evidence records it writes.
    ///
    /// A `moved` change appends its [`Transition`](crate::journal::Transition) to the instance's
    /// `transitions` field — once: a transition already listed is not appended again, so completing
    /// an interrupted batch cannot double it. An `evidence` change becomes an [`Entry`]; every other
    /// change is the document write alone.
    fn effective(decision: &Decision) -> Result<(EntityInstance, Vec<Entry>), StoreError> {
        let mut instance = decision.instance.clone();
        let mut transitions: Vec<Transition> = match instance.fields.get(TRANSITIONS_FIELD) {
            None => Vec::new(),
            Some(value) => serde_json::from_value(value.clone()).map_err(|error| {
                StoreError::Backend(format!(
                    "`{}/{}` carries transitions that do not read: {error}",
                    instance.entity, instance.id
                ))
            })?,
        };
        let mut records = Vec::new();
        for event in &decision.events {
            let Some(payload) = event.payload.as_object() else {
                continue;
            };
            let Some(change) = payload.get("change") else {
                continue;
            };
            let change: Change = serde_json::from_value(change.clone()).map_err(|error| {
                StoreError::Backend(format!(
                    "an event about `{}/{}` carries a change that does not read: {error}",
                    event.entity, event.id
                ))
            })?;
            match change {
                Change::Moved {
                    from,
                    to,
                    decided_on,
                    ..
                } => {
                    let text = |key: &str| {
                        payload
                            .get(key)
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned()
                    };
                    let actor = text("actor");
                    let (executor, correlation) = crate::journal::attribution(payload, &actor);
                    let transition = Transition {
                        at: text("recorded_at"),
                        actor,
                        revision: event.revision,
                        from,
                        to,
                        decided_on,
                        imported: false,
                        executor,
                        correlation,
                    };
                    if !transitions.contains(&transition) {
                        transitions.push(transition);
                    }
                }
                Change::Evidence { .. } => {
                    let entry = crate::journal::entry_of(event).ok_or_else(|| {
                        StoreError::Backend(format!(
                            "an observation about `{}/{}` names no instant, actor or artifact, \
                             and an evidence record without them cannot be counted",
                            event.entity, event.id
                        ))
                    })?;
                    records.push(entry);
                }
                _ => {}
            }
        }
        if transitions.is_empty() {
            instance.fields.remove(TRANSITIONS_FIELD);
        } else {
            instance.fields.insert(
                TRANSITIONS_FIELD.to_owned(),
                serde_json::to_value(&transitions).map_err(|error| {
                    StoreError::Backend(format!("serialising transitions: {error}"))
                })?,
            );
        }
        Ok((instance, records))
    }

    /// Writes each evidence record, where the Git layout keeps them. A record already written is
    /// left as it is.
    fn write_records(&self, records: &[Entry]) -> Result<(), StoreError> {
        let evidence = &self.evidence;
        for record in records {
            crate::journal::write_evidence(evidence, record)
                .map_err(|error| backend("writing evidence under", evidence, &error))?;
        }
        Ok(())
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

    /// Completes a batch whose intent was durable before an interruption.
    fn recover_pending(&self) -> Result<(), StoreError> {
        let path = self.store.root().join(PENDING_BATCH);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(backend("reading", &path, &error)),
        };
        let pending: PendingBatch =
            serde_json::from_str(&text).map_err(|error| backend("parsing", &path, &error))?;
        let mut writer = self.clone();
        for commit in pending.commits {
            writer.apply_recoverable(&commit.decision, commit.expect.into())?;
        }
        fs::remove_file(&path).map_err(|error| backend("removing", &path, &error))
    }

    /// Applies one intended entry, completing its evidence records if its document already landed.
    fn apply_recoverable(&mut self, decision: &Decision, expect: Expect) -> Result<(), StoreError> {
        let (effective, records) = Self::effective(decision)?;
        let held = self
            .read(&effective.entity, &effective.id)?
            .map(|document| instance_of(&effective.entity, &effective.id, &document));
        if held.as_ref() == Some(&effective) {
            return self.write_records(&records);
        }
        self.commit(decision, expect)
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
    if !frontmatter.refs.is_empty() {
        fields.insert(
            "refs".to_owned(),
            serde_json::to_value(&frontmatter.refs).unwrap_or(Value::Null),
        );
    }
    if !frontmatter.relations.is_empty() {
        fields.insert(
            "relations".to_owned(),
            serde_json::to_value(&frontmatter.relations).unwrap_or(Value::Null),
        );
    }
    if !frontmatter.scope.is_empty() {
        fields.insert(
            "scope".to_owned(),
            serde_json::to_value(&frontmatter.scope).unwrap_or(Value::Null),
        );
    }
    if let Some(withholds) = frontmatter.withholds {
        fields.insert("withholds".to_owned(), Value::from(withholds.as_str()));
    }
    // The model digest, and it is here because it was missing here. `document_of` read the key and
    // `apply_body` wrote it, so a digest set by `aep artifact set --model-digest` was applied to a
    // document and then dropped on the way back out to the instance the store commits: the command
    // reported `model_digest set (revision 2)` and the file kept revision 1 with no digest. A field
    // this function does not name does not survive a round trip, whatever the other two do.
    if let Some(digest) = &frontmatter.model_digest {
        fields.insert("model_digest".to_owned(), Value::from(digest.as_str()));
    }
    // Carried so a later move appends to the list rather than replacing it: the document is
    // re-read into an instance before every write, and what this does not name is lost.
    if !frontmatter.transitions.is_empty() {
        fields.insert(
            TRANSITIONS_FIELD.to_owned(),
            serde_json::to_value(&frontmatter.transitions).unwrap_or(Value::Null),
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
    let refs = refs_of(&mut fields).map_err(refuse)?;
    let scope = scope_of(&mut fields).map_err(refuse)?;

    let relations: Vec<ArtifactRelation> = match fields.remove("relations") {
        None => Vec::new(),
        Some(value) => serde_json::from_value(value)
            .map_err(|error| refuse(format!("the `relations` field: {error}")))?,
    };

    let withholds = match fields.remove("withholds") {
        None => None,
        Some(Value::String(value)) => Some(
            aep_domain::evidence::EvidenceKind::parse(&value)
                .map_err(|error| refuse(format!("the `withholds` field: {error}")))?,
        ),
        Some(other) => {
            return Err(refuse(format!(
                "the `withholds` field is {other}, not an evidence kind"
            )))
        }
    };

    let model_digest = match fields.remove("model_digest") {
        None => None,
        Some(Value::String(value)) => Some(
            aep_domain::evidence::SpecDigest::new(value)
                .map_err(|error| refuse(format!("the `model_digest` field: {error}")))?,
        ),
        Some(other) => {
            return Err(refuse(format!(
                "the `model_digest` field is {other}, not a digest"
            )))
        }
    };

    let transitions: Vec<Transition> = match fields.remove(TRANSITIONS_FIELD) {
        None => Vec::new(),
        Some(value) => serde_json::from_value(value)
            .map_err(|error| refuse(format!("the `{TRANSITIONS_FIELD}` field: {error}")))?,
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
            format: PlanningFormat::V1,
            id: artifact,
            kind,
            status,
            title,
            summary,
            owner,
            tags,
            refs,
            relations,
            scope,
            withholds,
            model_digest,
            revision: instance.revision,
            transitions,
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
        if format.as_str().and_then(PlanningFormat::from_tag).is_none() {
            return Err(format!(
                "the `format` field is {format}; this build writes `{}`, `{}` or `{}`",
                PlanningFormat::V1.as_str(),
                PlanningFormat::V2.as_str(),
                PlanningFormat::V3.as_str()
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

/// The `refs` field as the frontmatter's set, or nothing.
///
/// Both written forms are accepted here for the same reason they are accepted in a file: this is
/// the door a second backend's records come through, and one of them will have stored the
/// shorthand.
fn refs_of(fields: &mut Map<String, Value>) -> Result<BTreeSet<ExternalRef>, String> {
    match fields.remove("refs") {
        None => Ok(BTreeSet::new()),
        Some(Value::Array(refs)) => refs
            .into_iter()
            .map(|value| {
                let node: Node = serde_json::from_value(value)
                    .map_err(|error| format!("a reference: {error}"))?;
                ExternalRef::from_node(&node).map_err(|error| error.to_string())
            })
            .collect(),
        Some(other) => Err(format!("the `refs` field is {other}, not a list")),
    }
}

/// The scope entries an instance carries, in the document's own form.
///
/// Refused rather than dropped, for the reason a reference is: a surface nothing can read is a
/// surface a wave would silently treat as absent, and absent is what says *this story is safe to
/// run beside anything*.
fn scope_of(fields: &mut Map<String, Value>) -> Result<Vec<ScopeEntry>, String> {
    match fields.remove("scope") {
        None => Ok(Vec::new()),
        Some(Value::Array(entries)) => entries
            .into_iter()
            .map(|value| {
                let node: Node =
                    serde_json::from_value(value).map_err(|error| format!("a surface: {error}"))?;
                ScopeEntry::from_node(&node).map_err(|error| error.to_string())
            })
            .collect(),
        Some(other) => Err(format!("the `scope` field is {other}, not a list")),
    }
}

impl StateProvider for MarkdownProvider {
    fn load(&self, entity: &str, id: &str) -> Result<Option<EntityInstance>, StoreError> {
        self.recover_pending()?;
        Ok(self
            .read(entity, id)?
            .map(|document| instance_of(entity, id, &document)))
    }

    fn ids(&self, entity: &str) -> Result<Vec<String>, StoreError> {
        self.recover_pending()?;
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
    /// Nothing: the documents are the authority and keep no event log. What happened is read back
    /// from `transitions` and the evidence files by [`crate::journal::read_git`].
    fn events(&self, _entity: &str, _id: &str) -> Result<Vec<DomainEvent>, StoreError> {
        self.recover_pending()?;
        Ok(Vec::new())
    }
}

impl Store for MarkdownProvider {
    fn commit(&mut self, decision: &Decision, expect: Expect) -> Result<(), StoreError> {
        // The instance written carries the move's transition, and the records are the
        // observations' evidence files.
        let (instance, records) = Self::effective(decision)?;
        let (entity, id) = (instance.entity.as_str(), instance.id.as_str());

        // Checked before anything is written, so a refusal leaves every file exactly as it was.
        let existing = self.read(entity, id)?;
        check(
            entity,
            id,
            expect,
            existing.as_ref().map(|held| held.frontmatter.revision),
        )?;

        let mut document = document_of(&instance)?;
        document.frontmatter.format = PlanningFormat::V3;
        // An observation leaves its document as it was, and a write that changes nothing is not
        // made: the record is the evidence file, not a rewritten document.
        if existing.as_ref() == Some(&document) {
            return self.write_records(&records);
        }
        let written = if existing.is_some() {
            self.store.update(&Self::relative(entity, id), &document)
        } else {
            self.store.create(&document)
        };
        written.map_err(|error| StoreError::Backend(error.to_string()))?;
        self.write_records(&records)
    }
}

impl AtomicBatchStore for MarkdownProvider {
    fn commit_batch(&mut self, commits: &[AtomicCommit]) -> Result<(), StoreError> {
        self.recover_pending()?;
        if commits.is_empty() {
            return Ok(());
        }

        // Validate the full ordered batch before publishing its intent. Later entries see the
        // transaction-local instance produced by earlier ones.
        let mut view: BTreeMap<(String, String), Option<EntityInstance>> = BTreeMap::new();
        for commit in commits {
            let instance = &commit.decision.instance;
            let key = (instance.entity.clone(), instance.id.clone());
            if !view.contains_key(&key) {
                let held = self
                    .read(&instance.entity, &instance.id)?
                    .map(|document| instance_of(&instance.entity, &instance.id, &document));
                view.insert(key.clone(), held);
            }
            check(
                &instance.entity,
                &instance.id,
                commit.expect,
                view.get(&key)
                    .and_then(|held| held.as_ref().map(|held| held.revision)),
            )?;
            view.insert(key, Some(instance.clone()));
        }

        let pending = PendingBatch {
            commits: commits
                .iter()
                .map(|commit| PendingCommit {
                    decision: commit.decision.clone(),
                    expect: commit.expect.into(),
                })
                .collect(),
        };
        fs::create_dir_all(self.store.root())
            .map_err(|error| backend("creating", self.store.root(), &error))?;
        let path = self.store.root().join(PENDING_BATCH);
        let temporary = self
            .store
            .root()
            .join(format!("{PENDING_BATCH}.{}.tmp", std::process::id()));
        let bytes = serde_json::to_vec(&pending)
            .map_err(|error| StoreError::Backend(format!("serialising a batch intent: {error}")))?;
        let mut file = fs::File::create(&temporary)
            .map_err(|error| backend("creating", &temporary, &error))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| backend("writing", &temporary, &error))?;
        fs::rename(&temporary, &path).map_err(|error| backend("publishing", &path, &error))?;

        self.recover_pending()
    }
}
