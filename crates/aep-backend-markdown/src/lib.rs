//! A markdown planning store: one artifact per file, YAML frontmatter, git as the log.
//!
//! This is the first durable store in the repository. Until now the only implementation of the
//! interaction contract was `aep-backend-memory`, which forgets everything when the process
//! exits — good enough to prove the contract is implementable, useless for keeping a plan.
//!
//! # What it is for
//!
//! An agent that has to plan work needs somewhere to put epics, stories and tasks that survives
//! the session, that a human can read and edit in an ordinary editor, and whose history is already
//! reviewable. A directory of markdown files in the repository is all three: the diff of a status
//! move is one line, `git log` says who moved it and when, and nobody needs a database or a
//! credential to read the plan.
//!
//! ```text
//! .engineering/planning/
//! ├── initiative/passwordless.md
//! ├── epic/passkeys.md
//! ├── story/passkey-login.md
//! └── task/webauthn-ceremony.md
//! ```
//!
//! # Where this sits relative to the contract
//!
//! This crate is the store, a **provider** and a **projection** — and the backend is the one
//! adapter over both. [`provider::MarkdownProvider`] is the documents as an `entity_store::Store`:
//! frontmatter as instance, body as a field, `journal.jsonl` as the event log, held to
//! `entity-runtime`'s own provider suite. [`projection::MarkdownProjection`] is the plan's shape for
//! `aep_backend_entity::EntityBackend` — where an entity lands, what a document keeps that an
//! entity does not carry, which ladder a status is checked against. [`backend::MarkdownBackend`]
//! is `EntityBackend<MarkdownProvider, MarkdownProjection>` behind the same constructor as before;
//! the sixteen `aep-conformance` suites run against it, and they are shown to fail it under
//! injected faults — a suite that has never failed is not evidence that it can.
//!
//! **Neither the contract logic nor the durability logic is written twice.** Every command is
//! handed to `aep-backend-memory`; the adapter seals the event and commits it with the document;
//! the provider writes. Idempotency, revision conflicts, "a refusal still leaves an audit record",
//! "nothing is ever physically deleted", the latch — each is a decision whose wrong version looks
//! right, and two implementations drift in exactly the ways a suite run months apart discovers.
//! Until wave G this crate carried the second one.
//!
//! **Deviation D-P1 is closed** (2026-08-26). It existed because the CLI wrote through this crate's
//! [`create`](store::MarkdownStore::create) and [`update`](store::MarkdownStore::update) rather than
//! through a command — and it stayed open because the vocabulary was missing two words: a planning
//! store's ladders are data with an open status vocabulary, and an evidence record is the input to
//! the gated move. `aep.status.move/v1` and `aep.evidence.record/v1` are those words.
//!
//! Consequently there is **no delete**, on any type here. Removing a plan item is a status move to
//! `archived`, which is invariant 16 spelled for a file tree: an epic that was abandoned is a fact
//! about how the work went, and a store that can drop it silently is a store whose history lies.
//!
//! # The frontmatter is this backend's format, not the protocol's
//!
//! `aep-domain` gains nothing from this crate — no type, no field, no format constant. The
//! frontmatter key set, the `aep.planning-md/1` version, the `---` fences and the file layout are
//! all private to this backend, and the crate's job is to map them onto the domain types that
//! already exist: [`Artifact`](aep_domain::artifact::Artifact),
//! [`ArtifactId`](aep_domain::artifact::ArtifactId),
//! [`ArtifactKind`](aep_domain::artifact::ArtifactKind),
//! [`ArtifactStatus`](aep_domain::artifact::ArtifactStatus),
//! [`ArtifactRelation`](aep_domain::artifact::ArtifactRelation) and
//! [`ArtifactLifecycle`](aep_domain::artifact::ArtifactLifecycle).
//!
//! That boundary is the same one [`ArtifactLocation`](aep_domain::artifact::ArtifactLocation)
//! draws: location is metadata, the graph is normative. A second backend keeping the same
//! artifacts as rows in Postgres or as issues in Linear must be able to exist without teaching the
//! protocol anything about YAML fences — and it cannot if the protocol crate carries one storage
//! format's spelling.
//!
//! # Reading a document a person wrote
//!
//! [`claim`] is the second thing this crate does, and it reads the other kind of markdown: not a
//! document this backend rendered, but one a person wrote, carrying the adopter's dated-claim
//! convention (`Verify: 2026-08-30 — … (horizon: 7d)`). It reports its own coverage — occurrences
//! seen against records produced — because a scanner over human-written documents that cannot say
//! what it missed is a gate that goes quiet instead of failing. See
//! `docs/design/evidence-horizons-design-v0.1.md` § 6.
//!
//! # No clock, no randomness
//!
//! Nothing here reads the wall clock or an RNG, so rendering a document twice produces the same
//! bytes and a test does not have to freeze time. In particular a document records **no
//! `created_at`**: git already knows who wrote the file and when, to the commit, and a timestamp
//! the tooling maintains beside it is a second answer that goes stale the first time somebody
//! rebases. The same reasoning fixes the CLI's seeding clock to `Timestamp::EPOCH`.

pub mod assembly;
pub mod backend;
pub mod claim;
pub mod document;
pub mod drift;
pub mod frontmatter;
pub mod journal;
pub mod kernel;
pub mod projection;
pub mod provider;
pub mod store;

pub use claim::{
    horizon_growth, raw_occurrences, scan, scan_at, ClaimRecord, ClaimRejection,
    ClaimRejectionReason, ClaimScan, ClaimState, HorizonGrowth, DEFAULT_HORIZON_DAYS,
};
pub use document::{MoveRefusal, PlanningDocument, PlanningDocumentError};
pub use frontmatter::{PlanningFrontmatter, RawPlanningFrontmatter, PLANNING_FORMAT};
pub use store::{MarkdownStore, StoreError, StoreFailure, StoreReport, StoredDocument};
