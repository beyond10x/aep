//! The planning store as a contract implementation.
//!
//! [`crate::store::MarkdownStore`] says what the files contain. This says what the *contract* means
//! over them: [`MarkdownBackend`] implements [`CommandService`] and [`QueryService`], so a plan held
//! as markdown answers the same questions, and refuses the same writes, as any other backend.
//!
//! # One adapter, one provider, one projection
//!
//! Since wave G (`docs/plan/store-waves-f-g-h.md`), `MarkdownBackend` is
//! `aep_backend_entity::EntityBackend<MarkdownProvider, MarkdownProjection>` behind the same
//! constructor and the same surface. The adapter applies every command in `aep-backend-memory`,
//! asks the projection where the result lands, seals the event and commits it with the document in
//! one step; [`crate::provider::MarkdownProvider`] writes the file and appends the event to the
//! journal; [`crate::projection::MarkdownProjection`] keeps what is plan-shaped — the prose, the
//! edges in frontmatter, the ladder, the journal's own vocabulary. What used to be this module's
//! `persist`, its latch and its `journal::append` — a second hand-written durability layer beside
//! the SQLite backend's — is gone, not kept beside.
//!
//! # What this closes
//!
//! Deviation **D-P1**: the store wrote through its own `create`/`update` rather than through the
//! contract's one write path, so the sixteen `aep-conformance` suites did not run against it — and
//! "there is a durable backend" was a claim the suites did not support. It is closed, and the scan
//! in `tests/conformance.rs` holds it closed: the one write out of this crate is the provider's
//! `commit`, reached only by the adapter.
//!
//! # The contract logic is not written twice
//!
//! Every command is handed to `aep-backend-memory`, and the adapter adds durability around it. That
//! is deliberate: idempotency, revision conflicts, "a refusal still leaves an audit record",
//! "nothing is ever physically deleted" — each is a decision with a wrong version that looks right,
//! and two implementations of them drift in exactly the ways a suite run months apart discovers.
//!
//! # The order, and the window it leaves
//!
//! A command is applied in memory and **then** written to disk, because what must be written is the
//! *result* of applying it. If the write fails, the adapter **latches** — every later call, reads
//! included, refuses with the same fault rather than serving state that is not durable. Closing the
//! window properly needs a durable intent log, which is the shape `P6` is for.

use std::path::Path;

use aep_backend_entity::EntityBackend;
use aep_backend_memory::MemoryBackend;
use aep_contract::command::{CommandEnvelope, CommandResult, CommandService};
use aep_contract::error::{CommandError, QueryError};
use aep_contract::query::{
    AuditQuery, EntityEnvelope, EntityQuery, Page, QueryService, Relation, RelationQuery,
    RevisionRecord,
};
use aep_contract::registry::TypeDescriptor;
use aep_contract::QueryConsistency;
use aep_domain::audit::AuditRecord;
use aep_domain::command::Command;
use aep_domain::entity::{ActorRef, EntityId, EntityLocator, EntityRef, EntityType};
use aep_domain::time::Timestamp;
use aep_domain::workspace::MemberName;

use crate::projection::MarkdownProjection;
use crate::provider::MarkdownProvider;

/// Where a plan seeded from markdown lives, by convention.
pub const ORGANISATION: &str = "planning";
/// The space it lives in.
pub const SPACE: &str = "store";

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

/// A planning store that answers as a backend.
///
/// A newtype over the adapter rather than an alias, because `open` is an inherent constructor and
/// an inherent method can only be written in the crate that owns the type. Every trait method
/// forwards.
#[derive(Debug)]
pub struct MarkdownBackend(EntityBackend<MarkdownProvider, MarkdownProjection>);

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
        lifecycles: aep_domain::artifact::LifecycleRegistry,
    ) -> Result<Self, CommandError> {
        let provider = MarkdownProvider::open(root.as_ref());
        let projection = MarkdownProjection::new(members, at, actor, lifecycles);
        Ok(Self(EntityBackend::shaped(provider, projection)?))
    }

    /// The fault that made this backend untrustworthy, if one has happened.
    ///
    /// Once set, every call refuses with it. See the module note on the write window.
    pub fn latched(&self) -> Option<String> {
        self.0.latched()
    }

    /// How many entities the plan holds.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// `true` when the plan holds nothing.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The backend the contract logic lives in, for a caller that needs to look.
    pub const fn inner(&self) -> &MemoryBackend {
        self.0.inner()
    }

    /// Entities this backend accepted and could not write to a file.
    ///
    /// **Not empty is not a bug, and not a secret either.** A conformance suite creates entities
    /// under its own organisation and space; they are real to the contract and this store has no
    /// document shape for them. What matters is that a caller can ask, rather than discovering it
    /// when a restart loses them.
    pub fn unprojected(&self) -> Vec<EntityId> {
        self.0.with_projection(MarkdownProjection::unprojected)
    }

    /// The adapter this is an instantiation of, for a caller that wants the generic surface.
    pub const fn as_entity_backend(&self) -> &EntityBackend<MarkdownProvider, MarkdownProjection> {
        &self.0
    }
}

impl CommandService for MarkdownBackend {
    type Command = Command;

    async fn execute(
        &self,
        envelope: CommandEnvelope<Self::Command>,
    ) -> Result<CommandResult, CommandError> {
        self.0.execute(envelope).await
    }
}

impl QueryService for MarkdownBackend {
    type AuditRecord = AuditRecord;

    async fn get(
        &self,
        reference: &EntityRef,
        consistency: QueryConsistency,
    ) -> Result<EntityEnvelope, QueryError> {
        self.0.get(reference, consistency).await
    }

    async fn resolve(&self, locator: &EntityLocator) -> Result<EntityId, QueryError> {
        self.0.resolve(locator).await
    }

    async fn query(&self, query: &EntityQuery) -> Result<Page<EntityEnvelope>, QueryError> {
        self.0.query(query).await
    }

    async fn relations(&self, query: &RelationQuery) -> Result<Page<Relation>, QueryError> {
        self.0.relations(query).await
    }

    async fn history(&self, reference: &EntityRef) -> Result<Vec<RevisionRecord>, QueryError> {
        self.0.history(reference).await
    }

    async fn audit(&self, query: &AuditQuery) -> Result<Page<Self::AuditRecord>, QueryError> {
        self.0.audit(query).await
    }

    async fn describe_type(&self, entity_type: &EntityType) -> Result<TypeDescriptor, QueryError> {
        self.0.describe_type(entity_type).await
    }
}
