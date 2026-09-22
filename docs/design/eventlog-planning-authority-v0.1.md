# Eventlog planning authority and explicit legacy migration v0.1

Status: accepted implementation direction under ESS evolution revision1 and Atlas ADR0050,
2026-09-15. One owning story: eventlog-planning-authority-migration. Implementation and real
cutovers remain unproved. This page fixes the required behavior; concrete encoding/API additions
must receive their exact typed design and compatibility fixtures before implementation.

## Authority and compatibility

The new planning authority is ER recorded execution over the qualified Eventlog file provider.
Markdown under `.engineering/planning/` is a tracked deterministic projection. The authority is
tracked under `.engineering/state/`, including Eventlog's existing `manifest.json`, `events.jsonl`,
blobs and every durable recovery/privacy/adapter fact the provider requires. The existing manifest
belongs to eventlog-file/1; AEP must not add fields to it or reinterpret its commit point. Migration
facts belong to typed AEP records via the adapter, with record contents in blobs. Only disposable
caches and runtime-local operational lock files may be excluded from version control.

Introduce `aep.project/2` for this authority and its default. Preserve aep.project/1 parsing and
meaning for inspection, migration and explicit legacy operation. A version1 file never silently
opens an Eventlog store. Version2 resolves omitted planning-store configuration to Eventlog file
authority under `state/` and the tracked `planning/` projection; an explicit form must name that
same authority/projection role. Hybrid is not a version2 runtime selection. The existing project
`state: state.yaml` refers to workflow state and remains separate; do not repurpose that key.

An old reader must refuse version2 before opening or mutating a store. Retained `aep` and `protocol`
operations have identical bytes and exits. An explicit legacy `--store` path cannot accidentally
turn a version2 projection into a writable Markdown authority. Driver locking, plan validation,
queries, mutations, evidence, history and conformance all use the selected authority. No new
parallel planning engine or direct lifecycle-status setter is introduced.

## Preserve the complete available source

Inspect legacy Markdown, SQLite, PostgreSQL and both declared sides of every supported hybrid
policy. The declared authority is a fact to verify, not permission to ignore conflicting replicas.
Report unreachable/partial sources distinctly; applying a migration without a complete admitted
source inventory refuses. A divergent hybrid or inconsistent document/journal pair refuses until
an explicit separately recorded source correction resolves it. Never choose the newer timestamp,
larger revision or preferred file as an inferred merge policy.

Preserve artifact identities and namespaces, exact revisions/statuses, relations and their order,
scope, external references, metadata, Markdown bodies, evidence observations, provenance, audit and
idempotency facts actually available. Inventory auxiliary backend entities as well as rendered
documents: query-visible equivalence is insufficient if relations, applied commands or audit facts
are dropped. Retain original journal records/bytes and their known source order as legacy evidence.
The current Markdown journal has both old Change entries and sealed bare DomainEvents; neither
becomes a fabricated complete DecisionRecord. Exact input bytes remain available beside normalized
typed source facts wherever legacy parsing loses representation detail.

Use explicit ER import anchors for incomplete genesis. Preserve complete recorded envelopes when
available, reserve their original global IDs, and expose unavailable original receipts/global order
as unavailable. Available bare events without envelope IDs remain bare evidence. No timestamp sort,
new ID, generated creation command, new recorded-at time or guessed definition fills a historical
gap. Post-import suffix verification proves only its stated boundary. A complete-history claim
requires complete original evidence and replay; importing a current document proves no genesis.

Snapshot and import coordinates have the typed design home `specs/planning-migration/`. These are
new authored migration decisions, not inferred legacy facts. The coordinate model is deliberately
not the serialized schema of full inventory, ER records or provider manifests. Actual Rust types
remain the source of generated AEP schemas. No ESS library dependency enters AEP.

## Inspect, dry-run, apply, verify and rebuild

Add `aep plan store` operations for read-only inspection, migration dry-run, migration apply,
verification and projection rebuild. Their machine-readable results must be closed versioned
Rust-owned values, with actual source identities, evidence boundaries and refusal codes. Final
flags/encodings are bound before implementation; this page does not claim installed AEP has them.
Inspection and dry-run never mutate source, destination, projection, lock authority or config.
Dry-run gives a stable source snapshot identity and exact proposed target/equivalence report;
apply requires that inspected source identity and refuses stale or substituted input.

Apply uses this sequence:

1. Establish exclusive writer authority over the selected source and the project's store selector.
   Capture/revalidate its concrete identity and resolve or explicitly refuse pending legacy intents.
2. Stage a fresh destination at a separate bounded path. Preserve source evidence and checkpoint
   state through the recorded import contract; never replay old business commands as new commands.
3. Reopen and verify the staged authority, all referenced blobs, exact enumerated state/history,
   relations/evidence/audit/idempotency facts, and deterministic Markdown projection equivalence.
   Check complete counts and identities, not only sample documents or a successful open.
4. Under the still-held writer fence, recheck source identity and project-config bytes against the
   inspected snapshot. Any movement refuses switching; the source remains authoritative.
5. Publish the verified destination and atomically replace the project selector with version2,
   binding the exact destination identity. Persist enough durable phase evidence to recover an
   interrupted publish/switch without guessing which history won. Sync files and parent directories
   where the provider's durability contract requires them.
6. Reopen through ordinary project discovery and verify that it selects the exact new authority.
   Keep a read-only legacy recovery copy and explicit cutover receipt. Release fences only after
   the selector and authoritative receipt agree; report a recoverable uncertain phase honestly.

Source fencing is an implementation obligation, not a claim about today's in-process Mutex.
Current Markdown/EntityBackend locking does not establish a cross-process migration fence. Add a
shared admitted fence protocol to all relevant new-build legacy writers and migration paths, with
source/config identity checked after lock acquisition. Exercise a paused writer and an apply race.
An older process that ignores that protocol cannot be called fenced: require it to be quiescent
or an actually enforced source-specific exclusion, and refuse claimed online cutover otherwise.
SQL snapshots alone do not fence later writes. Test the actual transaction/table or external
writer exclusion used by each admitted SQL source. Do not infer quiescence from one clean read.

Before switch, a failed migration leaves the old source/config authoritative and the stage can be
inspected/resumed or discarded only by its exact identity. After switch, especially after any new
command commits, recovery follows Eventlog authority; never reactivate a stale old writer. An
apply retry recovers the original matching migration receipt/phase rather than importing twice.
Malformed, truncated, divergent or substituted stage/receipt/source data refuses without choosing
a branch. Recovery must distinguish a staged destination from the selected active destination.

## Projection and committed failure

Every command commits its complete decision/observation and receipt once through ER/Eventlog,
then materializes Markdown. A projection failure after commit returns a structured committed
outcome containing the original durable receipt and projection error. It is not rollback, absence
or permission to repeat the command with a fresh ID. Retrying the same identity recovers the same
receipt; repair/rebuild reads committed authority and does not execute a command again.

Record or derive a projection watermark tied to exact authority coordinates. Detect direct edits,
missing/extra projections and stale/corrupt projected content. Ordinary mutations refuse unresolved
drift instead of accepting it as source. Read-only inspect/verify reports both authoritative facts
and drift without silently serving edited documents as current authority. Rebuild stages/validates
the full deterministic projection, replaces only owned projection paths, preserves foreign files
with explicit conflicts, and returns the covered authority/watermark. Concurrent commits must be
fenced or make the rebuild visibly stale; they cannot yield a false current result.

Projection ownership comes from exact captured Markdown bytes or an ownership inventory whose
snapshot and digest match authority-held watermark evidence. Parseability alone never makes a
current path replaceable. On Unix, the inventory and recovery equality include all permission
bits, including setuid, setgid and sticky; projected files use deterministic mode `0644`.
This corrects the intended `aep.planning-projection-inventory/1` digest without changing its typed
wire envelope or schema. Watermarks produced by the incomplete byte-only implementation are not
compatible: verification and recovery refuse them instead of treating a mode-blind digest as
current.

## Acceptance and the six real cutovers

The one owner story includes the new backend/config/commands and operational acceptance. Subtasks
may separate implementation surfaces but cannot become competing migration owners. Eventlog,
ER, Service SDK, ESS, Connectors v2 and AEP are migrated in that order, using the qualified new
executable explicitly; AEP migrates its own planning store last. Rehearse each exact source snapshot
before its real cutover. Preserve each repository's single complete planning lineage and writer.

After every real switch verify complete state/history, queries, an actual governed mutation, restart,
deleted-projection rebuild, drift refusal and original-receipt recovery. Record source/config/
binary digests, boundaries, destination identity and actual test outputs. Track all durable files;
do not hide them with ignore rules to obtain a clean gate. Retain legacy recovery copies without
leaving an enabled competing authority.

Required tests include every legacy backend and hybrid policy shape, old/new config readers,
complete versus incomplete history, zero-event decisions and observations, concurrent writes,
unchanged retries and changed-content conflicts, every interruption boundary, source/config/stage
substitution, blob corruption, projection deletion/drift, committed projection failure and safe
rebuild. Mutations removing source recheck, conflict detection or original-receipt reuse must fail
their named cases. PostgreSQL runs against a disposable real server and cannot silently skip.
Run actual AEP task check, affected dependency/MSRV/alias/schema guards and independent review.
Eventlog-backed runtime closure moves to Rust1.91; independent pure libraries retain their minima.
No publication, release or deployment is part of local migration acceptance.
