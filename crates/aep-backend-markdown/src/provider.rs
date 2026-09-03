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
//! # Single commits and command batches
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
//!
//! The stronger [`AtomicBatchStore`] path writes the complete ordered command to
//! [`.aep-batch.pending.json`](PENDING_BATCH) before applying any entry. If the process stops after
//! one document or before its events, every state and event read completes that intent
//! idempotently before answering. The pending record is removed only after the whole batch lands.

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
use crate::frontmatter::{PlanningFrontmatter, PLANNING_FORMAT};
use crate::journal::JOURNAL;
use crate::store::MarkdownStore;

/// The field a document's markdown body travels under.
pub const BODY_FIELD: &str = "body";

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
/// What [`crate::projection::MarkdownProjection`] hydrates from and writes to. [`MarkdownProvider`]
/// is one; a hybrid of it and a replica (`aep-backend-hybrid`) is another, and the projection does
/// not know which it has — every read goes through the `Store` traits, so a hybrid's declared read
/// path governs hydration as it governs everything else. The two things a `Store` cannot say are
/// asked here: which kinds there are (the SPI enumerates ids under one entity type, never the
/// types), and where the documents are, for a message.
pub trait PlanStore: Store {
    /// The directory the plan's documents are in — the local side, for a hybrid.
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

    /// Applies one intended entry, completing an event append if its document already landed.
    fn apply_recoverable(&mut self, decision: &Decision, expect: Expect) -> Result<(), StoreError> {
        let instance = &decision.instance;
        let held = self
            .read(&instance.entity, &instance.id)?
            .map(|document| instance_of(&instance.entity, &instance.id, &document));
        if held.as_ref() == Some(instance) {
            let present = self.events_raw(&instance.entity, &instance.id)?;
            let missing: Vec<DomainEvent> = decision
                .events
                .iter()
                .filter(|event| !present.contains(event))
                .cloned()
                .collect();
            return self.append_events(&missing);
        }
        self.commit(decision, expect)
    }

    fn events_raw(&self, entity: &str, id: &str) -> Result<Vec<DomainEvent>, StoreError> {
        let path = self.store.root().join(JOURNAL);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(backend("reading", &path, &error)),
        };
        Ok(text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str::<DomainEvent>(line).ok())
            .filter(|event| event.entity == entity && event.id == id)
            .collect())
    }

    fn append_events(&self, events: &[DomainEvent]) -> Result<(), StoreError> {
        if events.is_empty() {
            return Ok(());
        }
        let path = self.store.root().join(JOURNAL);
        let mut lines = String::new();
        for event in events {
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
            refs,
            relations,
            scope,
            withholds,
            model_digest,
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
    fn events(&self, entity: &str, id: &str) -> Result<Vec<DomainEvent>, StoreError> {
        self.recover_pending()?;
        // Only the lines that are events. The journal's older entries are another shape, read by
        // another reader (`crate::journal`), and a line that is neither is a half-written line from
        // a killed process — skipped here as it is there, rather than making the log unreadable.
        self.events_raw(entity, id)
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
        self.append_events(&decision.events)
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
