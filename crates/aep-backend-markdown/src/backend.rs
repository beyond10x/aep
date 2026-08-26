//! The planning store as a contract implementation.
//!
//! [`MarkdownStore`] says what the files contain. This says what the *contract* means over them:
//! [`MarkdownBackend`] implements [`CommandService`] and [`QueryService`], so a plan held as
//! markdown answers the same questions, and refuses the same writes, as any other backend.
//!
//! # What this closes
//!
//! Deviation **D-P1**: the store wrote through its own `create`/`update` rather than through the
//! contract's one write path, so the sixteen `aep-conformance` suites did not run against it — and
//! "there is a durable backend" was a claim the suites did not support.
//!
//! # The contract logic is not written twice
//!
//! Every command is handed to [`MemoryBackend`], and this type adds durability around it. That is
//! deliberate: idempotency, revision conflicts, "a refusal still leaves an audit record", "nothing
//! is ever physically deleted" — each is a decision with a wrong version that looks right, and two
//! implementations of them drift in exactly the ways a suite run months apart discovers. There is
//! one implementation, and the suites police it once.
//!
//! # The order, and the window it leaves
//!
//! A command is applied in memory and **then** written to disk. The reverse is not available: what
//! must be written is the *result* of applying it, which does not exist until it has been applied.
//!
//! So there is a window. If the file write fails, the in-memory store holds a command the disk does
//! not. This backend does not paper over that: the write failure is returned as
//! [`CommandError::Conflict`] naming the file, and the backend **latches** — every later call
//! refuses with the same fault rather than serving state that is not durable. A backend that
//! carried on would be answering from memory about a plan that no longer matches the files anybody
//! else can read.
//!
//! Closing the window properly needs a durable intent log, which is the shape `P6` is for.

use std::path::Path;
use std::sync::Mutex;

use aep_backend_memory::seed::{self, SeedReport};
use aep_backend_memory::MemoryBackend;
use aep_contract::command::{CommandEnvelope, CommandResult, CommandService};
use aep_contract::error::{CommandError, QueryError};
use aep_contract::query::{
    AuditQuery, EntityEnvelope, EntityQuery, Page, QueryService, Relation, RelationQuery,
    RevisionRecord,
};
use aep_contract::registry::TypeDescriptor;
use aep_contract::testing::block_on;
use aep_contract::QueryConsistency;
use aep_domain::artifact::ArtifactId;
use aep_domain::audit::AuditRecord;
use aep_domain::command::Command;
use aep_domain::entity::{ActorRef, EntityId, EntityLocator, EntityRef, EntityType};
use aep_domain::time::Timestamp;
use aep_domain::workspace::MemberName;

use crate::journal;
use crate::store::MarkdownStore;

/// Where a plan seeded from markdown lives, by convention.
pub const ORGANISATION: &str = "planning";
/// The space it lives in.
pub const SPACE: &str = "store";

/// A planning store that answers as a backend.
#[derive(Debug)]
pub struct MarkdownBackend {
    inner: MemoryBackend,
    store: MarkdownStore,
    /// Which file each entity came from, so a write lands where the document was read.
    ///
    /// Kept rather than derived: a document filed under an accepted alias is rewritten where it
    /// lives, which is the same reason [`MarkdownStore::update`] takes a path.
    paths: Mutex<Vec<(EntityId, ArtifactId, String)>>,
    /// The fault that made this backend untrustworthy, if one has happened.
    latched: Mutex<Option<String>>,
    /// Entities accepted by the contract that this store has no document shape for.
    unprojected: Mutex<Vec<EntityId>>,
    actor: ActorRef,
    /// The instant this backend stamps journal entries with.
    ///
    /// Handed in, never read: this crate has no clock, for the same reason the kernel next door
    /// has none — a record that dated itself could not be replayed.
    at: Timestamp,
}

impl MarkdownBackend {
    /// Opens the store at `root` and hydrates a backend from it.
    ///
    /// Hydration goes through `CreateEntity` commands, not through a side door: the entities exist
    /// because commands created them, which is what the audit trail then says.
    ///
    /// # Errors
    ///
    /// If the store cannot be read cleanly, if its edges do not resolve, or if seeding refuses.
    pub fn open(
        root: impl AsRef<Path>,
        members: impl IntoIterator<Item = MemberName>,
        at: Timestamp,
        actor: ActorRef,
    ) -> Result<Self, CommandError> {
        let store = MarkdownStore::open(root.as_ref());
        let report = store.load();
        if !report.is_clean() {
            let detail = report
                .failures
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            return Err(CommandError::Conflict {
                reason: format!(
                    "the store at {} could not be read: {detail}",
                    root.as_ref().display()
                ),
            });
        }

        let graph =
            report
                .graph_in_workspace(members)
                .map_err(|errors| CommandError::Conflict {
                    reason: format!("the plan does not build a graph: {errors}"),
                })?;

        let inner = MemoryBackend::new();
        let seeded: SeedReport =
            seed::from_manifest(&inner, &graph, ORGANISATION, SPACE, at, &actor)?;

        // The path each entity came from, matched by artifact id.
        let mut paths = Vec::new();
        for (artifact, entity) in &seeded.by_id {
            if let Some(stored) = report
                .documents
                .values()
                .find(|stored| &stored.document.frontmatter.id == artifact)
            {
                paths.push((
                    entity.clone(),
                    artifact.clone(),
                    stored.relative_path.clone(),
                ));
            }
        }

        Ok(Self {
            inner,
            store,
            paths: Mutex::new(paths),
            latched: Mutex::new(None),
            unprojected: Mutex::new(Vec::new()),
            actor,
            at,
        })
    }

    /// The fault that made this backend untrustworthy, if one has happened.
    ///
    /// Once set, every call refuses with it. See the module note on the write window.
    pub fn latched(&self) -> Option<String> {
        self.latched
            .lock()
            .expect("the latch is not poisoned")
            .clone()
    }

    /// How many entities the plan holds.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// `true` when the plan holds nothing.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// The backend the contract logic lives in, for a caller that needs to look.
    pub const fn inner(&self) -> &MemoryBackend {
        &self.inner
    }

    /// Refuses when a previous write left memory and disk disagreeing.
    fn check_latch(&self) -> Result<(), CommandError> {
        match self.latched() {
            None => Ok(()),
            Some(detail) => Err(CommandError::Conflict {
                reason: format!(
                    "this backend is not answering: a write failed and the files no longer match \
                     what it holds — {detail}. Reopen it once the cause is fixed."
                ),
            }),
        }
    }

    /// Latches, and reports the fault that caused it.
    fn latch(&self, detail: impl AsRef<str>) -> CommandError {
        let detail = detail.as_ref();
        *self.latched.lock().expect("the latch is not poisoned") = Some(detail.to_owned());
        CommandError::Conflict {
            reason: format!("the plan was changed in memory but not on disk: {detail}"),
        }
    }
}

impl CommandService for MarkdownBackend {
    type Command = Command;

    // The `async` belongs to the contract, not to this backend: the body completes without
    // awaiting, because file IO here is synchronous and making it otherwise would pull an async
    // runtime into a crate that needs none. `aep-backend-memory` carries the same note.
    #[allow(clippy::unused_async_trait_impl)]
    async fn execute(
        &self,
        envelope: CommandEnvelope<Self::Command>,
    ) -> Result<CommandResult, CommandError> {
        self.check_latch()?;
        let result = block_on(self.inner.execute(envelope))?;
        self.persist(&result)?;
        Ok(result)
    }
}

impl MarkdownBackend {
    /// Writes every entity a command touched back to its file, and journals the change.
    ///
    /// A command that changed nothing writes nothing: `CommandResult::affected` is empty for a
    /// replay, which is what makes replaying one safe to do on a store somebody else is reading.
    fn persist(&self, result: &CommandResult) -> Result<(), CommandError> {
        for reference in &result.affected {
            // Where this entity's document is: the one it was read from, or — for an entity
            // created through the contract and not yet on disk — the one its address says it
            // belongs in. An entity addressed somewhere else entirely is a conformance suite's, and
            // inventing a file for it would put a document nobody wrote into somebody's plan.
            let placed = match self.file_for(&reference.id) {
                Some(found) => Some(found),
                None => self.artifact_for(&reference.id)?,
            };
            let Some((artifact, relative)) = placed else {
                self.note_unprojected(&reference.id);
                continue;
            };

            let body = block_on(self.inner.get(
                &EntityRef::new(reference.id.clone()),
                QueryConsistency::Current,
            ))
            .map_err(|error| self.latch(format!("the entity could not be read back: {error}")))?;

            // Re-read rather than re-render from scratch. **The prose is the document.** An
            // artifact's body — the Outcome, the Acceptance, the argument somebody wrote — exists
            // nowhere in the entity, so rebuilding a file from the entity alone would silently
            // delete it. Only the frontmatter fields the body actually carries are touched.
            let report = self.store.load();
            let found = report
                .documents
                .values()
                .find(|stored| stored.relative_path == relative)
                .map(|stored| stored.document.clone());
            // A create writes a document that is not there yet. Its frontmatter is what the entity
            // carries and its body is **empty** — a body this crate invented would be prose nobody
            // is accountable for, in a document that reads as though somebody had thought about it.
            let creating = found.is_none();
            let existing = match found {
                Some(document) => document,
                None => crate::document::PlanningDocument {
                    frontmatter: crate::frontmatter::PlanningFrontmatter::new(
                        artifact.clone(),
                        artifact.namespace().parse().map_err(|error| {
                            self.latch(format!("`{artifact}` has no usable kind: {error}"))
                        })?,
                        "draft".parse().map_err(|error| {
                            self.latch(format!("`draft` is not a status: {error}"))
                        })?,
                    ),
                    body: String::new(),
                },
            };
            let mut updated = existing.clone();
            apply_body(&mut updated.frontmatter, &body.data);
            updated.frontmatter.revision = reference.revision.get();

            if updated == existing {
                continue;
            }

            let before = existing.frontmatter.status.clone();
            if creating {
                self.store
                    .create(&updated)
                    .map_err(|error| self.latch(error.to_string()))?;
            } else {
                self.store
                    .update(&relative, &updated)
                    .map_err(|error| self.latch(error.to_string()))?;
            }

            let change = if creating {
                journal::Change::Created {
                    status: updated.frontmatter.status.clone(),
                }
            } else if before == updated.frontmatter.status {
                journal::Change::BodyReplaced
            } else {
                journal::Change::Moved {
                    from: before,
                    to: updated.frontmatter.status.clone(),
                    decided_on: journal::Provenance::default(),
                }
            };
            journal::append(
                self.store.root(),
                &journal::Entry {
                    at: self.at.to_string(),
                    actor: self.actor.to_string(),
                    artifact: artifact.clone(),
                    kind: updated.frontmatter.kind,
                    revision: updated.frontmatter.revision,
                    change,
                },
            )
            .map_err(|error| self.latch(format!("the journal could not be appended: {error}")))?;
        }
        Ok(())
    }

    /// The artifact an entity **should** be filed as, when it has no document yet.
    ///
    /// `Ok(None)` for an entity that is not addressed into this store — a conformance suite's, say.
    /// That is not a failure: the contract holds entities this store has no document shape for, and
    /// inventing a file for one would put a document nobody wrote into somebody's plan.
    ///
    /// The address is the whole of the decision. `seed` files an artifact at
    /// `ep://<organisation>/<space>/<kind>/<name>`, so an entity under this store's organisation and
    /// space, whose `kind` segment names a kind this vocabulary knows, is one of ours and its file
    /// follows from its id.
    ///
    /// # Errors
    ///
    /// If the entity cannot be read back at all, which means memory and disk have already parted.
    fn artifact_for(&self, id: &EntityId) -> Result<Option<(ArtifactId, String)>, CommandError> {
        let envelope = block_on(
            self.inner
                .get(&EntityRef::new(id.clone()), QueryConsistency::Current),
        )
        .map_err(|error| self.latch(format!("the entity could not be read back: {error}")))?;
        let locator = &envelope.metadata.locator;
        if locator.organisation() != ORGANISATION || locator.space() != SPACE {
            return Ok(None);
        }
        let Ok(artifact) = format!("{}:{}", locator.kind(), locator.key()).parse::<ArtifactId>()
        else {
            return Ok(None);
        };
        // The path the store would file it at, which is what `MarkdownStore::create` derives too.
        let relative = format!("{}/{}.md", artifact.namespace(), artifact.name());
        Ok(Some((artifact, relative)))
    }

    /// The artifact and path an entity was read from, if this store holds one.
    fn file_for(&self, id: &EntityId) -> Option<(ArtifactId, String)> {
        self.paths
            .lock()
            .expect("the path map is not poisoned")
            .iter()
            .find(|(entity, _, _)| entity == id)
            .map(|(_, artifact, path)| (artifact.clone(), path.clone()))
    }

    /// Records an entity this store cannot project to a document.
    fn note_unprojected(&self, id: &EntityId) {
        self.unprojected
            .lock()
            .expect("the unprojected list is not poisoned")
            .push(id.clone());
    }

    /// Entities this backend accepted and could not write to a file.
    ///
    /// **Not empty is not a bug, and not a secret either.** A conformance suite creates entities
    /// under its own organisation and space; they are real to the contract and this store has no
    /// document shape for them. What matters is that a caller can ask, rather than discovering it
    /// when a restart loses them.
    pub fn unprojected(&self) -> Vec<EntityId> {
        self.unprojected
            .lock()
            .expect("the unprojected list is not poisoned")
            .clone()
    }
}

/// Copies the frontmatter fields an entity body carries, and only those.
///
/// Everything else in the frontmatter — tags, relations, `extra` — is left alone, because the body
/// says nothing about them and "absent from the body" is not the same claim as "removed".
fn apply_body(
    frontmatter: &mut crate::frontmatter::PlanningFrontmatter,
    body: &aep_domain::node::Node,
) {
    let aep_domain::node::Node::Map(fields) = body else {
        return;
    };
    if let Some(aep_domain::node::Node::Text(status)) =
        fields.get(aep_backend_memory::command::STATUS_KEY)
    {
        if let Ok(parsed) = status.parse() {
            frontmatter.status = parsed;
        }
    }
    for (key, slot) in [
        ("title", &mut frontmatter.title),
        ("summary", &mut frontmatter.summary),
        ("owner", &mut frontmatter.owner),
    ] {
        if let Some(aep_domain::node::Node::Text(value)) = fields.get(key) {
            *slot = Some(value.clone());
        }
    }
}

impl QueryService for MarkdownBackend {
    type AuditRecord = AuditRecord;

    #[allow(clippy::unused_async_trait_impl)]
    async fn get(
        &self,
        reference: &EntityRef,
        consistency: QueryConsistency,
    ) -> Result<EntityEnvelope, QueryError> {
        block_on(self.inner.get(reference, consistency))
    }

    #[allow(clippy::unused_async_trait_impl)]
    async fn resolve(&self, locator: &EntityLocator) -> Result<EntityId, QueryError> {
        block_on(self.inner.resolve(locator))
    }

    #[allow(clippy::unused_async_trait_impl)]
    async fn query(&self, query: &EntityQuery) -> Result<Page<EntityEnvelope>, QueryError> {
        block_on(self.inner.query(query))
    }

    #[allow(clippy::unused_async_trait_impl)]
    async fn relations(&self, query: &RelationQuery) -> Result<Page<Relation>, QueryError> {
        block_on(self.inner.relations(query))
    }

    #[allow(clippy::unused_async_trait_impl)]
    async fn history(&self, reference: &EntityRef) -> Result<Vec<RevisionRecord>, QueryError> {
        block_on(self.inner.history(reference))
    }

    #[allow(clippy::unused_async_trait_impl)]
    async fn audit(&self, query: &AuditQuery) -> Result<Page<Self::AuditRecord>, QueryError> {
        block_on(self.inner.audit(query))
    }

    #[allow(clippy::unused_async_trait_impl)]
    async fn describe_type(&self, entity_type: &EntityType) -> Result<TypeDescriptor, QueryError> {
        block_on(self.inner.describe_type(entity_type))
    }
}
