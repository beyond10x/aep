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
    /// The account the move currently being written rested on, when one arrived with it.
    decided_on: Mutex<Option<journal::Provenance>>,
    actor: ActorRef,
    /// The instant this backend stamps journal entries with, as the journal spells one: ISO-8601.
    ///
    /// Handed in, never read: this crate has no clock, for the same reason the kernel next door has
    /// none — a record that dated itself could not be replayed. Held as the string rather than as a
    /// `Timestamp`, because `Timestamp`'s own `Display` is epoch milliseconds — right for a wire
    /// value, wrong in a journal whose every other entry is a date somebody can read, and a journal
    /// carrying both spellings is one nobody can sort.
    ///
    /// Kept beside the `Timestamp` rather than derived from it at each write, because `Timestamp`'s
    /// own `Display` is epoch milliseconds — correct for a wire value, and wrong in a journal whose
    /// every other entry is a date somebody can read. A journal carrying both spellings is a
    /// journal nobody can sort.
    at_iso: String,
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
            decided_on: Mutex::new(None),
            actor,
            at_iso: iso_8601(at),
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
        // The endpoints a relation command changes the *document* of. The contract reports no
        // affected entity for one — an edge is a thing in its own right and neither endpoint's
        // revision moves — but the source's **document** does change, because a planning document
        // carries its edges in frontmatter. Read before executing, since the payload is consumed.
        // `RemoveRelation` is deliberately absent: `apply_relations` adds and does not remove, so
        // re-projecting the source after one would write the file back unchanged and journal a
        // change that did not happen. Projecting a removal is its own piece of work.
        let touched = match &envelope.payload {
            Command::CreateRelation(create) => vec![create.source.id.clone()],
            _ => Vec::new(),
        };
        // The account the move arrived with. Read before executing, because the payload is consumed
        // — and carried into the journal entry, because a status somebody asserted the evidence for
        // and one a record established are different facts, and defaulting it away would make every
        // move look equally well founded.
        let decided_on = match &envelope.payload {
            Command::MoveStatus(move_status) => match move_status.decided_on.as_ref() {
                Some(aep_domain::node::Node::Text(account)) => Some(
                    serde_json::from_str::<journal::Provenance>(account).map_err(|error| {
                        CommandError::Conflict {
                            reason: format!("the move's account is not a provenance: {error}"),
                        }
                    })?,
                ),
                // Not `.ok()` anywhere on this path. An account that failed to decode and was
                // silently dropped leaves a move looking as well founded as one nobody had evidence
                // for — which is the one distinction this field exists to carry.
                Some(other) => {
                    return Err(CommandError::Conflict {
                        reason: format!(
                            "the move's account is {other:?}, and an account travels as JSON text \
                             because `Node`'s numbers are floating point and a count of `1` that \
                             returns as `1.0` is not a count"
                        ),
                    })
                }
                None => None,
            },
            _ => None,
        };
        *self
            .decided_on
            .lock()
            .expect("the provenance slot is not poisoned") = decided_on;
        let result = block_on(self.inner.execute(envelope))?;
        self.persist(&result)?;
        for id in touched {
            self.persist_document(&id, None)?;
        }
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
            self.persist_document(&reference.id, Some(reference.revision.get()))?;
        }
        Ok(())
    }

    /// Writes one entity's document, at `revision` when the entity's own revision moved.
    ///
    /// `None` for a change that alters the document without altering the *entity* — a relation,
    /// whose edge lives in frontmatter while the contract holds it as a record of its own.
    ///
    /// **The document's revision advances either way**, because it counts changes to the document
    /// and an edge is one. What `None` says is only that the entity's own revision is not the
    /// number to take: a backend is opened per invocation and every artifact is seeded at revision
    /// 1, so an entity's revision after one command is always 2, and taking it would clamp every
    /// document in the store to 2 for ever.
    fn persist_document(
        &self,
        entity: &EntityId,
        revision: Option<u64>,
    ) -> Result<(), CommandError> {
        {
            // Where this entity's document is: the one it was read from, or — for an entity
            // created through the contract and not yet on disk — the one its address says it
            // belongs in. An entity addressed somewhere else entirely is a conformance suite's, and
            // inventing a file for it would put a document nobody wrote into somebody's plan.
            let placed = match self.file_for(entity) {
                Some(found) => Some(found),
                None => self.artifact_for(entity)?,
            };
            let Some((artifact, relative)) = placed else {
                self.note_unprojected(entity);
                return Ok(());
            };

            let body = block_on(
                self.inner
                    .get(&EntityRef::new(entity.clone()), QueryConsistency::Current),
            )
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
            // Absent leaves the prose alone; present replaces it. See `BODY_KEY`.
            if let aep_domain::node::Node::Map(fields) = &body.data {
                if let Some(aep_domain::node::Node::Text(prose)) = fields.get(BODY_KEY) {
                    updated.body.clone_from(prose);
                }
            }
            self.apply_relations(entity, &mut updated.frontmatter)?;
            // **The document's revision is the document's own count, not the entity's.** A backend
            // is opened per invocation and every artifact is seeded at revision 1, so an entity's
            // revision after one command is always 2 — taking it would clamp every document in the
            // store to 2 for ever, however many times it moved. What the file says it has been
            // through is what the file says, plus this change.
            let _ = revision;
            if !creating {
                updated.frontmatter.revision = existing.frontmatter.revision.saturating_add(1);
            }

            if updated == existing {
                return Ok(());
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
                    decided_on: self
                        .decided_on
                        .lock()
                        .expect("the provenance slot is not poisoned")
                        .clone()
                        .unwrap_or_default(),
                }
            };
            journal::append(
                self.store.root(),
                &journal::Entry {
                    at: self.at_iso.clone(),
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

    /// Writes the entity's outgoing relations into its frontmatter.
    ///
    /// The second projection `persist` needs, and the one `protocol artifact relate` has no command
    /// path without. An entity's relations are separate `Relation` records — the contract models an
    /// edge as a thing, not as a field — so nothing about updating an entity's body brings them
    /// along.
    ///
    /// # What it does not do, and why
    ///
    /// **It adds; it does not remove.** A relation whose target this store cannot name — an entity
    /// under some other organisation and space — is left out of the frontmatter, and an edge already
    /// in the file that the contract does not know about is left alone. Rewriting the list from the
    /// contract's view would delete, on the first status move, every edge that was written by hand
    /// into a document this backend has never been the author of. `RemoveRelation` is a command and
    /// belongs on the same path as the rest; until it is projected too, this side only grows.
    ///
    /// # Errors
    ///
    /// If the relations cannot be read, which means memory and disk have already parted.
    fn apply_relations(
        &self,
        id: &EntityId,
        frontmatter: &mut crate::frontmatter::PlanningFrontmatter,
    ) -> Result<(), CommandError> {
        let query = RelationQuery {
            source: Some(EntityRef::new(id.clone())),
            target: None,
            kind: None,
            limit: None,
            after: None,
            consistency: QueryConsistency::Current,
        };
        let page = block_on(self.inner.relations(&query))
            .map_err(|error| self.latch(format!("the relations could not be read: {error}")))?;

        for relation in &page.items {
            let Some((target, _)) = self.file_for(&relation.target.id).map_or_else(
                || self.artifact_for(&relation.target.id),
                |found| Ok(Some(found)),
            )?
            else {
                continue;
            };
            let already = frontmatter
                .relations
                .iter()
                .any(|existing| existing.kind == relation.kind && existing.target.id() == &target);
            if !already {
                frontmatter
                    .relations
                    .push(aep_domain::artifact::ArtifactRelation::new(
                        relation.kind,
                        aep_domain::artifact::ArtifactRef::new(target, None),
                    ));
            }
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

/// One instant, as a date a person can read.
///
/// The civil date from a Unix millisecond, without a date library: days since the epoch, then the
/// proleptic Gregorian calendar. A dependency whose only job is this would be one more thing to
/// audit for a clock of its own.
fn iso_8601(at: Timestamp) -> String {
    let millis = i64::try_from(at.epoch_millis()).unwrap_or(i64::MAX);
    let seconds = millis.div_euclid(1000);
    let days = seconds.div_euclid(86_400);
    let time = seconds.rem_euclid(86_400);

    // Howard Hinnant's civil_from_days, which is the standard way to do this without a library.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        time / 3600,
        (time % 3600) / 60,
        time % 60
    )
}

/// The key an entity carries its document's prose under.
///
/// # The decision, and why it is not "the entity becomes the document"
///
/// A document's prose had to reach this backend somehow, and the alternatives were worse. Leaving
/// the CLI to write bodies directly is a second write path, which is what invariant 14 forbids and
/// what D-P1 is. Inventing a body here is prose nobody is accountable for.
///
/// So prose is data, under a reserved key, exactly as `status` and `title` already are. The fear it
/// raised — *every status move now carries the whole body* — is not what happens: `UpdateEntity`
/// carries only the keys that changed, so a status move carries no body at all.
///
/// **An entity that does not carry it leaves the document's prose alone.** That is what makes this
/// safe against every artifact seeded before it existed: absent is not empty, and a backend that
/// read absence as "delete the prose" would empty the store on its first status move.
pub const BODY_KEY: &str = "body";

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
    // Tags, same rule as everything else here: present replaces, absent leaves alone. A command
    // that says nothing about tags is not a command that removed them.
    if let Some(aep_domain::node::Node::Seq(tags)) = fields.get("tags") {
        frontmatter.tags = tags
            .iter()
            .filter_map(|tag| match tag {
                aep_domain::node::Node::Text(text) => Some(text.clone()),
                _ => None,
            })
            .collect();
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
