//! The plan's shape, for the adapter: where a contract entity lands as a document.
//!
//! Wave G, story 2 of `docs/plan/store-waves-f-g-h.md`. `aep_backend_entity::EntityBackend` owns
//! applying a command, sealing its event and committing to a provider; what it does not know is
//! that a plan is one document per artifact with its prose in the body and its edges in frontmatter,
//! that a `status` arriving on a plain update has to be checked against the kind's ladder, or that
//! this store's journal spells a move, an edge and a recorded observation in its own words. That is
//! this type — the `Projection` the adapter runs with over [`MarkdownProvider`].
//!
//! # What is projected, and what is not
//!
//! * An entity addressed `ep://planning/store/<kind>/<name>` lands at `<kind>/<name>.md`. One
//!   addressed anywhere else — a conformance suite's — gets no file and is reported by
//!   [`MarkdownProjection::unprojected`]; inventing a document for it would put prose nobody wrote
//!   into somebody's plan.
//! * **The prose is the document.** An entity carries no prose, so a document is re-read and only
//!   the frontmatter fields the entity body carries are touched; the body changes only when a
//!   command carries it under [`BODY_KEY`]. Absent is not empty.
//! * **Relations are added and never removed** from frontmatter. Rewriting the list from the
//!   contract's view would delete, on the first status move, every edge written by hand into a
//!   document this backend has never been the author of.
//! * **The document's revision is the document's own count.** A backend is opened per invocation
//!   and every artifact is seeded at revision 1, so an entity's revision after one command is
//!   always 2 — taking it would clamp every document to 2 for ever. The placement carries the
//!   document's count, and the event with it.
//! * **The ladder is checked here.** The contract permits a `status` key on an `UpdateEntity` — its
//!   own suites use one — so the store is the only layer that can refuse an illegal move. A
//!   `MoveStatus` is exempt: it carries a decision the engine already took against the ladder and
//!   the evidence presented.
//! * Every placement carries, under the event payload's `change`, the [`journal::Change`] this
//!   store's journal spells the write as — so `protocol artifact history` reads a move made through
//!   the provider exactly as it reads one made before it.

use aep_backend_entity::{Placement, Projection, Record, Snapshot};
use aep_backend_memory::seed;
use aep_backend_memory::MemoryBackend;
use aep_contract::command::{CommandEnvelope, CommandResult};
use aep_contract::error::CommandError;
use aep_contract::query::{QueryService, RelationQuery};
use aep_contract::testing::block_on;
use aep_contract::QueryConsistency;
use aep_domain::artifact::ArtifactId;
use aep_domain::command::Command;
use aep_domain::entity::{ActorRef, EntityId, EntityRef};
use aep_domain::ids::IdempotencyKey;
use aep_domain::node::Node;
use aep_domain::time::Timestamp;
use aep_domain::workspace::MemberName;
use entity_store::StateProvider as _;

use crate::backend::{BODY_KEY, ORGANISATION, SPACE};
use crate::document::PlanningDocument;
use crate::frontmatter::PlanningFrontmatter;
use crate::journal;
use crate::provider::{document_of, instance_of, MarkdownProvider};
use crate::store::MarkdownStore;

/// The plan's shape over [`MarkdownProvider`].
#[derive(Debug)]
pub struct MarkdownProjection {
    members: Vec<MemberName>,
    at: Timestamp,
    actor: ActorRef,
    lifecycles: aep_domain::artifact::LifecycleRegistry,
    /// Which file each entity came from, so a write lands where the document was read.
    ///
    /// Kept rather than derived: a document filed under an accepted alias is rewritten where it
    /// lives, which is the same reason [`MarkdownStore::update`] takes a path.
    paths: Vec<(EntityId, ArtifactId, String)>,
    /// Entities accepted by the contract that this store has no document shape for.
    unprojected: Vec<EntityId>,
    /// What the command being written is, noted before the contract consumes it.
    current: Current,
}

/// What the projection noted about the command in flight.
#[derive(Debug, Default)]
struct Current {
    /// Whether a ladder has already ruled on this change (a `MoveStatus`).
    decided: bool,
    /// The account the move rested on, when one arrived with it.
    decided_on: Option<journal::Provenance>,
    /// Sources whose document a relation command changes without changing the entity.
    touched: Vec<EntityId>,
    /// An observation about an artifact: target, kind, source, reference.
    observation: Option<(EntityId, String, String, Option<String>)>,
}

impl MarkdownProjection {
    /// The plan's shape, seeding on open with `at` and `actor`, holding kinds to `lifecycles`.
    pub fn new(
        members: impl IntoIterator<Item = MemberName>,
        at: Timestamp,
        actor: ActorRef,
        lifecycles: aep_domain::artifact::LifecycleRegistry,
    ) -> Self {
        Self {
            members: members.into_iter().collect(),
            at,
            actor,
            lifecycles,
            paths: Vec::new(),
            unprojected: Vec::new(),
            current: Current::default(),
        }
    }

    /// Entities this projection accepted and could not write to a file.
    ///
    /// **Not empty is not a bug, and not a secret either.** A conformance suite creates entities
    /// under its own organisation and space; they are real to the contract and this store has no
    /// document shape for them. What matters is that a caller can ask, rather than discovering it
    /// when a restart loses them.
    pub fn unprojected(&self) -> Vec<EntityId> {
        self.unprojected.clone()
    }

    /// The artifact and path an entity was read from, if this store holds one.
    fn file_for(&self, id: &EntityId) -> Option<(ArtifactId, String)> {
        self.paths
            .iter()
            .find(|(entity, _, _)| entity == id)
            .map(|(_, artifact, path)| (artifact.clone(), path.clone()))
    }

    /// The artifact an entity **should** be filed as, when it has no document yet.
    ///
    /// `Ok(None)` for an entity that is not addressed into this store — a conformance suite's, say.
    /// The address is the whole of the decision: `seed` files an artifact at
    /// `ep://<organisation>/<space>/<kind>/<name>`, so an entity under this store's organisation and
    /// space is one of ours and its file follows from its id.
    fn artifact_for(
        inner: &MemoryBackend,
        id: &EntityId,
    ) -> Result<Option<(ArtifactId, String)>, CommandError> {
        let envelope = block_on(inner.get(&EntityRef::new(id.clone()), QueryConsistency::Current))
            .map_err(|error| CommandError::Conflict {
                reason: format!("the entity could not be read back: {error}"),
            })?;
        let locator = &envelope.metadata.locator;
        if locator.organisation() != ORGANISATION || locator.space() != SPACE {
            return Ok(None);
        }
        let Ok(artifact) = format!("{}:{}", locator.kind(), locator.key()).parse::<ArtifactId>()
        else {
            return Ok(None);
        };
        let relative = format!("{}/{}.md", artifact.namespace(), artifact.name());
        Ok(Some((artifact, relative)))
    }

    /// Where `id` lands, or `None` (noted as unprojected) when this store has no shape for it.
    fn locate(
        &mut self,
        inner: &MemoryBackend,
        id: &EntityId,
    ) -> Result<Option<(ArtifactId, String, String)>, CommandError> {
        let placed = match self.file_for(id) {
            Some(found) => Some(found),
            None => Self::artifact_for(inner, id)?,
        };
        let Some((artifact, relative)) = placed else {
            self.unprojected.push(id.clone());
            return Ok(None);
        };
        let stem = relative.strip_suffix(".md").unwrap_or(&relative);
        let (directory, name) = stem.split_once('/').map_or_else(
            || (artifact.namespace().to_owned(), artifact.name().to_owned()),
            |(directory, name)| (directory.to_owned(), name.to_owned()),
        );
        Ok(Some((artifact, directory, name)))
    }

    /// The document `directory/name.md` holds now, if any.
    fn existing(
        store: &MarkdownProvider,
        directory: &str,
        name: &str,
    ) -> Result<Option<PlanningDocument>, CommandError> {
        store
            .load(directory, name)
            .map_err(|error| CommandError::Conflict {
                reason: format!("the document could not be read: {error}"),
            })?
            .map(|instance| {
                document_of(&instance).map_err(|error| CommandError::Conflict {
                    reason: format!("the document could not be read back: {error}"),
                })
            })
            .transpose()
    }

    /// The placement for one entity's document: the fields the entity carries applied to the
    /// document as it stands, the ladder consulted, the edges added, the prose kept.
    fn place(
        &mut self,
        store: &MarkdownProvider,
        inner: &MemoryBackend,
        id: &EntityId,
    ) -> Result<Option<Placement>, CommandError> {
        let Some((artifact, directory, name)) = self.locate(inner, id)? else {
            return Ok(None);
        };
        let body = block_on(inner.get(&EntityRef::new(id.clone()), QueryConsistency::Current))
            .map_err(|error| CommandError::Conflict {
                reason: format!("the entity could not be read back: {error}"),
            })?;

        // Re-read rather than re-rendered from scratch. **The prose is the document.** An
        // artifact's body exists nowhere in the entity, so rebuilding a file from the entity alone
        // would silently delete it. Only the frontmatter fields the body actually carries are
        // touched. A create writes a document that is not there yet: its frontmatter is what the
        // entity carries and its body is **empty** — a body this crate invented would be prose
        // nobody is accountable for.
        let found = Self::existing(store, &directory, &name)?;
        let creating = found.is_none();
        let existing = match found {
            Some(document) => document,
            None => PlanningDocument {
                frontmatter: PlanningFrontmatter::new(
                    artifact.clone(),
                    artifact
                        .namespace()
                        .parse()
                        .map_err(|error| CommandError::Conflict {
                            reason: format!("`{artifact}` has no usable kind: {error}"),
                        })?,
                    "draft".parse().map_err(|error| CommandError::Conflict {
                        reason: format!("`draft` is not a status: {error}"),
                    })?,
                ),
                body: String::new(),
            },
        };
        let mut updated = existing.clone();
        apply_body(&mut updated.frontmatter, &body.data);
        self.check_the_ladder(creating, &existing, &updated, &artifact)?;
        if let Node::Map(fields) = &body.data {
            if let Some(Node::Text(prose)) = fields.get(BODY_KEY) {
                updated.body.clone_from(prose);
            }
        }
        self.apply_relations(inner, id, &mut updated.frontmatter)?;
        // **The comparison first, then the count.** Bumping before comparing made the no-op guard
        // dead on every update path: `updated` always differed by its revision, so a command that
        // changed nothing still rewrote the file and journalled a change that did not happen.
        if !creating && updated == existing {
            return Ok(None);
        }
        if !creating {
            updated.frontmatter.revision = existing.frontmatter.revision.saturating_add(1);
        }
        let change = change_for(
            creating,
            &existing,
            &updated,
            self.current.decided_on.clone().unwrap_or_default(),
        );
        let instance = instance_of(&directory, &name, &updated);
        Ok(Some(Placement {
            entity: directory,
            id: name,
            state: instance.lifecycle_state,
            revision: instance.revision,
            fields: instance.fields,
            note: Some(serde_json::to_value(change).unwrap_or_default()),
            always: false,
        }))
    }

    /// The placement an observation about an artifact makes: the document as it stands, at its
    /// current revision, with the observation as the change — an event at the same revision, and
    /// the count the evidence-gated move reads.
    fn observe(
        &mut self,
        store: &MarkdownProvider,
        inner: &MemoryBackend,
        target: &EntityId,
        kind: &str,
        source: &str,
        reference: Option<&str>,
    ) -> Result<Option<Placement>, CommandError> {
        let Some((artifact, directory, name)) = self.locate(inner, target)? else {
            return Ok(None);
        };
        let parsed = kind
            .parse::<aep_domain::evidence::EvidenceKind>()
            .map_err(|error| CommandError::Conflict {
                reason: format!("`{kind}` is not an evidence kind: {error}"),
            })?;
        let Some(existing) = Self::existing(store, &directory, &name)? else {
            return Err(CommandError::Conflict {
                reason: format!("`{artifact}` is no longer in the store"),
            });
        };
        let instance = instance_of(&directory, &name, &existing);
        let change = journal::Change::Evidence {
            kind: parsed,
            source: source.to_owned(),
            reference: reference.map(ToOwned::to_owned),
        };
        Ok(Some(Placement {
            entity: directory,
            id: name,
            state: instance.lifecycle_state,
            revision: instance.revision,
            fields: instance.fields,
            note: Some(serde_json::to_value(change).unwrap_or_default()),
            always: true,
        }))
    }

    /// Refuses a status change nothing has ruled on.
    ///
    /// The contract is storage-agnostic and permits a `status` key on an `UpdateEntity` — its own
    /// conformance suites use one — so **the store is the only layer that can refuse an illegal
    /// move**. Without this a story at `draft` reached `active` with `draft: [proposed, archived]`
    /// declared, and the test written for it asserted the bypass as correct.
    ///
    /// A `MoveStatus` is exempt, and that is not a hole: it carries a decision the engine already
    /// took against the kind's lifecycle **and the evidence presented**. Re-deciding it here without
    /// that evidence refuses moves that were correctly earned.
    fn check_the_ladder(
        &self,
        creating: bool,
        existing: &PlanningDocument,
        updated: &PlanningDocument,
        artifact: &ArtifactId,
    ) -> Result<(), CommandError> {
        if creating
            || self.current.decided
            || updated.frontmatter.status == existing.frontmatter.status
        {
            return Ok(());
        }
        let permissive = aep_domain::artifact::ArtifactLifecycle::permissive();
        let lifecycle = self
            .lifecycles
            .for_kind(&existing.frontmatter.kind)
            .unwrap_or(&permissive);
        if crate::kernel::permits_transition(
            Some(&existing.frontmatter.kind),
            lifecycle,
            &existing.frontmatter.status,
            &updated.frontmatter.status,
        ) {
            return Ok(());
        }
        Err(CommandError::Conflict {
            reason: format!(
                "`{artifact}` is {} and {} is not on its ladder; a status that reached a document \
                 without a ladder saying so is a transition nothing checked",
                existing.frontmatter.status, updated.frontmatter.status
            ),
        })
    }

    /// Writes the entity's outgoing relations into its frontmatter — adding, never removing.
    fn apply_relations(
        &self,
        inner: &MemoryBackend,
        id: &EntityId,
        frontmatter: &mut PlanningFrontmatter,
    ) -> Result<(), CommandError> {
        let query = RelationQuery {
            source: Some(EntityRef::new(id.clone())),
            target: None,
            kind: None,
            limit: None,
            after: None,
            consistency: QueryConsistency::Current,
        };
        let page = block_on(inner.relations(&query)).map_err(|error| CommandError::Conflict {
            reason: format!("the relations could not be read: {error}"),
        })?;
        for relation in &page.items {
            let target = match self.file_for(&relation.target.id) {
                Some((target, _)) => target,
                None => match Self::artifact_for(inner, &relation.target.id)? {
                    Some((target, _)) => target,
                    None => continue,
                },
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
}

impl Projection<MarkdownProvider> for MarkdownProjection {
    /// Hydration goes through `CreateEntity` commands, not through a side door: the entities exist
    /// because commands created them, which is what the audit trail then says.
    fn hydrate(
        &mut self,
        store: &MarkdownProvider,
        inner: &MemoryBackend,
    ) -> Result<(), CommandError> {
        let files = MarkdownStore::open(store.root());
        let report = files.load();
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
                    store.root().display()
                ),
            });
        }
        let graph = report
            .graph_in_workspace(self.members.clone())
            .map_err(|errors| CommandError::Conflict {
                reason: format!("the plan does not build a graph: {errors}"),
            })?;
        let seeded = seed::from_manifest(inner, &graph, ORGANISATION, SPACE, self.at, &self.actor)?;
        for (artifact, entity) in &seeded.by_id {
            if let Some(stored) = report
                .documents
                .values()
                .find(|stored| &stored.document.frontmatter.id == artifact)
            {
                self.paths.push((
                    entity.clone(),
                    artifact.clone(),
                    stored.relative_path.clone(),
                ));
            }
        }
        Ok(())
    }

    fn before(&mut self, envelope: &CommandEnvelope<Command>) -> Result<(), CommandError> {
        // The endpoints a relation command changes the *document* of. The contract reports no
        // affected entity for one — an edge is a thing in its own right and neither endpoint's
        // revision moves — but the source's document does change, because a planning document
        // carries its edges in frontmatter. `RemoveRelation` is deliberately absent: the frontmatter
        // adds and does not remove, so re-projecting the source after one would journal a change
        // that did not happen.
        let touched = match &envelope.payload {
            Command::CreateRelation(create) => vec![create.source.id.clone()],
            _ => Vec::new(),
        };
        // The account the move arrived with, carried into the journal's change because a status
        // somebody asserted the evidence for and one a record established are different facts.
        // Not `.ok()` anywhere on this path: an account that failed to decode and was silently
        // dropped leaves a move looking as well founded as one nobody had evidence for.
        let decided_on = match &envelope.payload {
            Command::MoveStatus(move_status) => match move_status.decided_on.as_ref() {
                Some(Node::Text(account)) => Some(
                    serde_json::from_str::<journal::Provenance>(account).map_err(|error| {
                        CommandError::Conflict {
                            reason: format!("the move's account is not a provenance: {error}"),
                        }
                    })?,
                ),
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
        let observation = match &envelope.payload {
            Command::RecordEvidence(record) => Some((
                record.target.id.clone(),
                record.kind.clone(),
                record.source.clone(),
                record.reference.clone(),
            )),
            _ => None,
        };
        self.current = Current {
            decided: matches!(&envelope.payload, Command::MoveStatus(_)),
            decided_on,
            touched,
            observation,
        };
        Ok(())
    }

    fn coordinates(&self, inner: &MemoryBackend, id: &EntityId) -> Option<(String, String)> {
        let relative = match self.file_for(id) {
            Some((_, relative)) => relative,
            None => Self::artifact_for(inner, id).ok().flatten()?.1,
        };
        let stem = relative.strip_suffix(".md").unwrap_or(&relative);
        stem.split_once('/')
            .map(|(directory, name)| (directory.to_owned(), name.to_owned()))
    }

    fn placements(
        &mut self,
        store: &MarkdownProvider,
        inner: &MemoryBackend,
        result: &CommandResult,
    ) -> Result<Vec<Placement>, CommandError> {
        let mut placements = Vec::new();
        for reference in &result.affected {
            if let Some(placement) = self.place(store, inner, &reference.id)? {
                placements.push(placement);
            }
        }
        for id in std::mem::take(&mut self.current.touched) {
            if let Some(placement) = self.place(store, inner, &id)? {
                placements.push(placement);
            }
        }
        if let Some((target, kind, source, reference)) = self.current.observation.take() {
            if let Some(placement) =
                self.observe(store, inner, &target, &kind, &source, reference.as_deref())?
            {
                placements.push(placement);
            }
        }
        Ok(placements)
    }

    /// Nothing beside the documents: a plan's relations live in frontmatter, its observations in
    /// the journal as events, and its audit trail is the journal — which is the one this store had
    /// before there was a contract, and the one `protocol artifact history` reads.
    fn records(
        &mut self,
        _store: &MarkdownProvider,
        _inner: &MemoryBackend,
        _before: &Snapshot,
        _key: &IdempotencyKey,
    ) -> Result<Vec<Record>, CommandError> {
        Ok(Vec::new())
    }
}

/// Which change the journal should record.
///
/// **Which change it actually was.** `Change::Related` existed and was never emitted, so an edge was
/// journalled as `body_replaced` — a record describing something that did not happen, in the one
/// place a reader goes to find out what did.
fn change_for(
    creating: bool,
    existing: &PlanningDocument,
    updated: &PlanningDocument,
    decided_on: journal::Provenance,
) -> journal::Change {
    if creating {
        return journal::Change::Created {
            status: updated.frontmatter.status.clone(),
        };
    }
    if existing.frontmatter.status != updated.frontmatter.status {
        return journal::Change::Moved {
            from: existing.frontmatter.status.clone(),
            to: updated.frontmatter.status.clone(),
            decided_on,
        };
    }
    match updated
        .frontmatter
        .relations
        .iter()
        .find(|edge| !existing.frontmatter.relations.contains(edge))
    {
        Some(added) => journal::Change::Related {
            relation: added.kind,
            target: added.target.to_string(),
        },
        None => journal::Change::BodyReplaced,
    }
}

/// Copies the frontmatter fields an entity body carries, and only those.
///
/// Everything else in the frontmatter — relations, `extra` — is left alone, because the body says
/// nothing about them and "absent from the body" is not the same claim as "removed".
fn apply_body(frontmatter: &mut PlanningFrontmatter, body: &Node) {
    let Node::Map(fields) = body else {
        return;
    };
    if let Some(Node::Text(status)) = fields.get(aep_backend_memory::command::STATUS_KEY) {
        if let Ok(parsed) = status.parse() {
            frontmatter.status = parsed;
        }
    }
    for (key, slot) in [
        ("title", &mut frontmatter.title),
        ("summary", &mut frontmatter.summary),
        ("owner", &mut frontmatter.owner),
    ] {
        if let Some(Node::Text(value)) = fields.get(key) {
            *slot = Some(value.clone());
        }
    }
    // Tags, same rule as everything else here: present replaces, absent leaves alone. A command
    // that says nothing about tags is not a command that removed them.
    if let Some(Node::Seq(tags)) = fields.get("tags") {
        frontmatter.tags = tags
            .iter()
            .filter_map(|tag| match tag {
                Node::Text(text) => Some(text.clone()),
                _ => None,
            })
            .collect();
    }
}
