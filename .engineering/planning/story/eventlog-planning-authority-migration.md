---
format: aep.planning-md/1
id: story:eventlog-planning-authority-migration
kind: story
status: active
title: Migrate all participating planning stores to recorded Eventlog file authority
owner: aep
relations:
- serves: vision:O2
- informed_by: story:store-selection-in-project-yaml
- informed_by: story:markdown-documents-as-a-store
scope:
- confidence: cited
  path: .engineering/planning/
- confidence: cited
  path: .engineering/project.yaml
- confidence: cited
  path: .engineering/state/
- confidence: cited
  path: .github/workflows/
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/edge/aep-cli/src/
- confidence: cited
  path: crates/edge/aep-cli/tests/
- confidence: cited
  path: crates/edge/aep-project/
- confidence: cited
  path: crates/edge/aep-schema/Cargo.toml
- confidence: cited
  path: crates/edge/aep-schema/src/schema.rs
- confidence: cited
  path: crates/govern/aep-domain/src/project.rs
- confidence: cited
  path: crates/plan/aep-backend-entity/
- confidence: cited
  path: crates/plan/aep-backend-eventlog/
- confidence: cited
  path: crates/plan/aep-backend-hybrid/
- confidence: cited
  path: crates/plan/aep-backend-markdown/
- confidence: cited
  path: crates/plan/aep-backend-postgres/
- confidence: cited
  path: crates/plan/aep-backend-sqlite/
- confidence: cited
  path: crates/plan/aep-contract/Cargo.toml
- confidence: cited
  path: crates/plan/aep-contract/src/lib.rs
- confidence: inferred
  path: crates/plan/aep-contract/src/migration/
- confidence: cited
  path: crates/plan/aep-planning-migration/
- confidence: cited
  path: docs/design/eventlog-planning-authority-v0.1.md
- confidence: inferred
  path: docs/design/planning-raw-capture-v0.1.md
- confidence: inferred
  path: docs/design/planning-store-selection-and-commands-v0.1.md
- confidence: inferred
  path: docs/guide/
- confidence: inferred
  path: schemas/generated/
- confidence: cited
  path: specs/planning-migration/
- confidence: inferred
  path: website/docs/
- confidence: cited
  path: xtask/
revision: 61
---
## Outcome

AEP owns exactly one migration from every supported legacy planning backend to qualified ER over
Eventlog file authority, with tracked Markdown projections and all six participating real stores
actually cut over and verified. The source plan/history remains auditable through explicit import
boundaries, durable receipts and safe recovery.

## Authority and typed home

Approved plan ess-evolution-20260915 revision1, SHA256
7579145c3de5a1c6f8088fd7fb804d29dac8903ec505048f3ce595c45023b787; Atlas ADR0050.
Accepted direction: docs/design/eventlog-planning-authority-v0.1.md. Typed snapshot, intent and
boundary coordinates: specs/planning-migration/, validated before this story was filed. These
are authored new coordinates, not fabricated legacy facts or replacement ER/provider schemas.
Final new CLI/format encodings require concrete Rust-owned schemas/fixtures before implementation.

## Acceptance

- Add the Eventlog file planning backend using complete async ER recorded execution and its
  explicit compatibility bridge where retained sync APIs require it. Preserve zero-event decisions,
  non-revision-advancing observations, global IDs, transaction-local checks and atomic multi-entity
  batches. Do not build an independent planning lifecycle interpreter or bypass the complete record.
- Introduce aep.project/2 for the new authority/default, preserving v1 meanings/readers for legacy
  operation/inspection/migration. Old readers refuse v2 before writes. Hybrid is absent from the
  new runtime config; explicit path overrides cannot silently make projections authoritative.
- Track .engineering/state/manifest.json, events.jsonl, blobs and all durable provider/adapter
  recovery metadata; preserve eventlog-file/1 manifest bytes/ownership. Track Markdown projections
  under .engineering/planning/. Exclude only disposable/runtime-local operational state.
- Add aep plan store inspect, migration dry-run/apply, verify and projection-rebuild operations with
  exact source/destination/config identities, closed versioned reports and named refusals. Inspect
  and dry-run do not mutate. Both binary names retain exact output/exit equivalence.
- Import Markdown, SQLite, PostgreSQL and every supported hybrid configuration. Preserve identities,
  revisions, body bytes, ordered relations, scopes/references, evidence/provenance/audit/idempotency
  and all available history, including auxiliary backend entities. Retain original source evidence
  and explicit imported boundaries where complete decisions, genesis, receipts or global order are
  unavailable. Do not invent records, IDs, definitions, chronology or fresh commands for old history.
- Refuse contradictory documents/journals, divergent hybrid sides, unreachable required authority,
  unsupported fencing, changed inspected sources, wrong stages and incomplete equivalence proofs.
  Never choose a winner by timestamp, maximum revision or preferred backend.
- Apply under a proved source/config writer fence: capture, stage, reopen/equivalence-verify,
  recheck source/config, durable publish and atomic selector switch. Test real concurrent/paused
  writers. Existing in-process locks or SQL snapshots do not prove cross-process fencing. Older
  unfenced writers require quiescence or an actually enforced source-specific exclusion.
- Exercise every interrupted phase and exact-identity apply retry. Before switch retain old authority;
  after switch recover the selected Eventlog authority, especially after new commands commit.
  Keep read-only legacy recovery copies without an enabled competing writer.
- Return the original committed durable receipt when projection fails after commit. Same-identity
  retry recovers it without duplicate execution. Detect direct edits, deleted/extra/stale files and
  refuse unresolved drift on mutation. Rebuild from a proved authority snapshot, preserve foreign
  files through explicit conflicts, and never replay the business command to repair projection.
- Rehearse preserved source snapshots, then actually migrate Eventlog -> Entity Runtime -> Service
  SDK -> ESS -> Connectors v2 -> AEP, using the qualified new executable explicitly and AEP last.
  After each cutover verify exact complete history/state, queries, a governed mutation, restart,
  deleted-projection rebuild, drift refusal and safe receipt recovery; retain the exact evidence.
- Keep one full planning lineage and writer per repository. Source/target comparisons enumerate
  all identities/counts and semantic facts, not a sample or only rendered documents. Preserve dirty
  primary work separately; no journal concatenation and no disguised blanket baseline refresh.
- Retain decisive red tests and causal mutations for source/config recheck, divergence, dropped
  history/evidence, projection failure/retry and receipt reuse. Run real disposable PostgreSQL lanes,
  independent review and actual task check including dependency/alias/schema/MSRV guards. Raise only
  the Eventlog-backed runtime closure to Rust1.91; retain and test pure-package existing minima.

## Scope

- crates/plan/aep-backend-eventlog/ — inferred; the new recorded backend/projection adapter.
- crates/plan/aep-backend-entity/ — cited; current generic provider projection/persistence seam.
- crates/plan/aep-backend-markdown/ — cited; exact legacy documents/journal/pending-intent inventory.
- crates/plan/aep-backend-sqlite/, aep-backend-postgres/, aep-backend-hybrid/ — cited; legacy source
  inventories, consistent snapshots and writer-fence admission, preserving existing contracts.
- crates/edge/aep-cli/src/ — cited; one project-selected planning entry path, store verbs and aliases.
- crates/edge/aep-cli/tests/ — cited; real command/backend equivalence, migration/recovery/receipt tests.
- crates/govern/aep-domain/src/project.rs and crates/edge/aep-project/ — cited; versioned config,
  discovery/path/source resolution and exclusive selector switching.
- Cargo.toml, Cargo.lock, affected crate manifests and xtask/ — cited; coordinated ER/Eventlog pins,
  layouts/schema generation and bounded compiler-minimum guards, not an ESS modeling dependency.
- Taskfile.yml and .github/workflows/ — cited; actual gates, provider admission and pure/runtime minima.
- docs/design/eventlog-planning-authority-v0.1.md and specs/planning-migration/ — cited; root-owned
  binding direction and validated coordinate model; evolve exact implementation decisions explicitly.
- docs/guide/, website/docs/, schemas/generated/ and CHANGELOG.md — inferred; accepted new commands,
  generated Rust-owned schemas and honest delivery record; no release claim before actual acceptance.
- .engineering/project.yaml, .engineering/state/ and .engineering/planning/ in the six named managed
  repository authorities — cited by the approved real-cutover requirement; AEP alone mutates plan
  artifacts and all operational cutovers wait for the qualified executable and provider foundation.
- Confidence: high on required behavior; exact new adapter/module/encoding mechanisms require source
  scoping and review before implementation. Would collide with planning backend/CLI/config/ER pin work.

## Readiness and ownership

The first ER async contract/reference-provider/ports/executor unit is locally integrated at
17da35a7b2d5ad0edfebecb9d770c0ffd558b368 and its actual composed task check passed. The qualified
Eventlog adapter and explicit bridge remain unimplemented; remaining provider qualification is
still open. Keep dependency-blocker:recorded-eventlog-runtime open until those exact requirements
are proved. A passed first unit is not evidence that the adapter or bridge exists.

Read-only backend capture analysis is complete for legacy Markdown, SQLite, PostgreSQL and all
48 admitted hybrid configurations. It identifies non-mutating capture, complete raw/typed source
preservation, relation/history admission and source-wide writer exclusion as concrete design work.
The exact command/config/projection/cutover companion remains to be selected and reviewed before
implementation. Coordinator review-result:eventlog-migration-old-reader-source-inspection records
the actual explicit-store write bypass that version-2 project validation alone cannot close.

These dependencies prevent implementation/cutover readiness; they do not justify replacing the
approved backend with a weaker synchronous or unrecorded shortcut. Preserve this single owner while
filing bounded subordinate implementation tasks when ready. No competing per-repository migration
stories are permitted; each of the six actual cutovers is evidence under this owner.

## Completion boundary

Local acceptance only. No publication, release, deployment or legacy-history rewrite is authorized
by this story. A successful backend test alone does not complete the story: all six actual store
cutovers and their required acceptance/gates remain part of its outcome.

## Command and selector companion proposal

The proposed docs/design/planning-store-selection-and-commands-v0.1.md fixes version-preserving
raw project readers, explicit Eventlog authority/projection roles, a required existing authority
identity and new-build explicit legacy-path resolution. It names inspect, migrate dry-run/apply,
verify and rebuild inputs and failure/receipt boundaries. It does not claim installed commands or
solve old-binary exclusion with a selector, marker or advisory lock. No force/assumed-quiescence
path is admitted. The exact source capture/hash, relation/hybrid mapping, durable phases and
source-specific enforced fence remain to be designed under this owner before implementation.
The adapter's final position/receipt types and report schemas must be bound before code dispatch.
This is a proposal for technical review; the runtime dependency blocker stays open.

## Physical capture and identity preservation decisions

Fresh source inspection confirms that both pinned SQL providers store document as TEXT, including
PostgreSQL; its GIN expression casts that text without replacing the source bytes. Capture exact
physical instances, events, history and legacy_origins rows, plus PostgreSQL provider_sequences,
with their original coordinates and exact retrieved document bytes before separate typed parsing.
The known instance kinds include removed aep.relation records and aep.audit/aep.applied facts.
Current logical hydration skips removed relations (aep-backend-entity/src/lib.rs:910), so its view
cannot supply complete migration evidence. Unknown kinds/schema, malformed rows and contradictory
embedded coordinates refuse admission rather than disappear from the captured set.

Markdown capture retains complete document and journal bytes separately from parsed facts, including
relative coordinates, original line order, blank lines and present-empty versus absent files.
Pending batch intents prevent admitted capture and inspect must not recover them. The normal
provider's filter_map journal read and per-document rename path do not establish complete capture
or source-wide writer exclusion (aep-backend-markdown/src/provider.rs:251-284; store.rs:254-299).

Preserve original SQL relation IDs, including removed records. Markdown relation ordinals remain
source coordinates; any target runtime identity minted for them is a new explicit migration mapping
fact, never an invented original ID. Hybrid active-edge correspondence must be unique and exact;
ambiguous duplicate matches refuse. Retain each backend's available history separately, including
SQL-only complete/auxiliary evidence, without inventing a shared prefix or cross-subject chronology.
Complete current semantic equality does not replace exact raw preservation or missing original
records/definitions/receipts. Config bytes remain bound by digest without copying private inputs
into tracked migration content or diagnostic output.

Exact closed wire encoding/hash vectors and a proved source-specific writer fence remain to be
selected and reviewed. New-build locks, same-user permissions and an SQL snapshot alone cannot
establish old-writer exclusion across switch and queued-writer resumption. No assumption-only
cutover is admitted. These decisions remain under this single owner and do not clear the runtime
dependency blocker or promote migration to implementation readiness.

## Legacy writer control decision

The existing accepted design permits actual supervised offline quiescence or enforced source-specific
write denial for older binaries (docs/design/eventlog-planning-authority-v0.1.md:90-99). Preserve both
routes in the final closed migration contract. Permanent filesystem/role denial is one implementation
choice, not a prerequisite imposed on every supervised offline cutover.

Cooperative new-build locking is separate and begins before backend opening, because normal legacy
open/read paths can create schema or recover pending batches. It does not prove that an older binary
took the lock. A version2 selector, retirement marker, stable scan, in-process mutex or ordinary SQL
snapshot does not establish old-writer exclusion. The explicit-store bypass remains observable at
crates/edge/aep-cli/src/planning.rs:135-145.

Supervised quiescence must actually control the declared ordinary writer fleet: stop and reap its
processes, inhibit registered schedulers/queues/restarts, and verify that no associated backend
operation remains active or queued. Control survives migration-process death, is freshly revalidated
through capture/recheck/publication/switch/recovery, and permanently retires the old writer route
after the new selector and receipt agree. Pre-switch restoration is an explicit verified abort only.
This bounded operational contract does not promise resistance to a deliberate out-of-band superuser
or malicious same-user override. A process census or user assertion alone remains insufficient.

Markdown control covers document/journal/pending-batch writes, temporary-file rename and selector
publication. SQLite additionally drains already-open connections and queued IMMEDIATE writers.
PostgreSQL requires server-side session/transaction drain, including external pools or separately
controlled clients; stopping one client is insufficient. Hybrid control covers both sides, every
reconciler and the divergence sidecar, independent of its selected authority policy.

The source-denial alternative must cover the same exact source/config/selector scope and existing as
well as future writer operations. Its held state survives migration-process death, revalidates at
each phase and cannot restore a stale writer after switch. Neither alternative is a deserialized
attestation or user-implemented success callback. Both need a concrete reviewed IO-edge provider;
AEP's deterministic core remains free of process, filesystem and network authority.

Current AEP has no such admitted control provider or complete fleet/session inventory. Keep named
WriterExclusionUnavailable refusal until the selected repository's actual mechanism is implemented
and causally exercised. The final design still owes exact capability/phase encodings and provider
selection. Do not add force/assume-quiescent success, call a mock provider production-qualified, or
treat this scoped decision as actual cutover readiness. No live service, role, process or source has
been changed by this planning work; the one migration owner and full acceptance remain intact.

## Capture and recovery contract scoping

Completed source-backed scoping of closed Markdown/SQLite/PostgreSQL/Hybrid capture records, the
five physical SQL row families, raw versus semantic digest domains, all48hybrid configurations,
identity/history mapping and twelve durable migration phases. These are binding-companion inputs,
not accepted serialized types, implemented commands or a completed migration.

Provider-neutral persisted values belong in aep-contract; parsed PlanningDocument/Entry and ER
views stay backend-local and convert explicitly. The Markdown backend already depends on the
contract, so a reverse dependency cannot host the capture types. Migration journal admission must
preserve every byte and reject duplicate/unknown fields before typed conversion, without using
normal readers that discard records. Exact optional-field compatibility and lossless JSON-number
handling remain to be bound before implementation.

Digest framing distinguishes framed bytes, raw32-byte SHA-256 and canonical textual digest; minted
Markdown relation IDs use raw digest hex with retained source mapping, never a fabricated old ID.
Exact independent byte vectors, semantic DTOs, phase/result layouts and provider APIs still require
the concrete reviewed companion. The operational writer fleet/provider remains unmapped; the
existing WriterExclusionUnavailable boundary and qualified runtime prerequisite remain open.

Source reports retained under local-evidence:ess-evolution/waves/0005-aep-migration/analysis-scratch:
capture-phase-contract-scope-result.md (f2b625f32299f13884073d19df14200f6c842762539364add5360c7e22967d4c),
capture-type-boundary-addendum.md (9aaba142e6a6a7f08d8d23d16d4e413f7937f2086b8c8524377964928aecda83),
and capture-coordinator-adoption.md. Preserve original reports; do not treat this scoping as an
independent acceptance review. All six real cutovers remain under this owner and are unperformed.

## Raw capture and digest companion proposal

Proposed docs/design/planning-raw-capture-v0.1.md consolidates the authored raw capture inputs into
one exact Rust-owned aep.raw-capture/1 transport: source/selector presence, complete/unstable/refused
observations, retained partial raw evidence, complete legacy row families and typed catalog records,
closed null-free envelopes, canonical ordering and framed digest inputs. It preserves raw documents
before separate semantic parsing and never derives operational writer exclusion from a hash or tag.

Root corrected the authored drafts' unrepresentable pre-resolution failures by making the outer
selector/config coordinates explicitly present or missing; completed transcript construction unwraps
the required present selector/config without changing the proposed complete fixture parts. Refused
records retain actual raw bytes and SQL storage classes. Diagnostics carry named codes/coordinates,
not arbitrary source-derived text. Exact backend catalog queries and admission comparisons remain
required separately; a known typed variant is not proof of its claimed catalog meaning.

This is a proposal awaiting independent technical review, not an accepted format or an implemented
capture. Digest constants are authored fixtures: no Rust fixture execution has been observed.
The pure transport may proceed after its exact design review independently of operational writer
facts and the recorded runtime dependency. It retains Rust1.85; full provider, phase, semantic,
report, adapter and six-cutover acceptance still belong to this single owner.

Scope additions: docs/design/planning-raw-capture-v0.1.md (inferred new companion),
crates/plan/aep-contract/Cargo.toml and src/lib.rs (cited dependency/export edge), and
crates/edge/aep-schema/Cargo.toml and src/schema.rs (cited schema roster/wiring).
The existing migration module, generated schema, lockfile and xtask scope remains applicable.

Authored input reports under local-evidence:ess-evolution/waves/0005-aep-migration/analysis-scratch:
raw-capture-contract-draft.md c9f6156a1f013c5862aefe7a3aabfd3c7c1ba3b6d9b6b838a10c96f6c18f795e;
raw-capture-contract-clarifications.md 09acb32c2ced25af6f9505e479eb73e5016bac76edd65253af757041026263bd.
These immutable authored inputs are not independent review reports; the consolidated repository
proposal supersedes them where it makes an explicit different decision.

## Raw capture first-review corrections

review-result:raw-capture-design-pass-1 records the independent needs-revision report unchanged,
SHA45396152351630bb0e5b158d001786cdc74d098ff9e09f42fa34259653654f10. Its four blockers and one
warning all caused concrete revisions to docs/design/planning-raw-capture-v0.1.md:

- Fixed ordered method phase rosters retain every successful scan, SQL snapshot and partial failed
  phase; subsequent phases are explicitly NotAttempted. Preflight refusals invent no acquisition.
- NotAttempted is separate from a queried empty list or queried absent scalar. Catalog/row-family
  order and the failed-prefix/unattempted-suffix rules prevent fabricated empty data or failures.
- Partial catalog families and per-table definition/column/key/check reads preserve all obtained
  facts, raw rejected cells and terminal coordinates; SqlSchema is assembled only after complete
  admitted catalog observations with exact correspondence.
- Unix bytes and Windows UTF-16 relative paths now have explicit host-independent lexical rules,
  including separators, NUL, prefixes, surrogate validity and reserved component behavior. Raw
  invalid units remain in refused evidence without normalization or loss.
- Unstable is restricted to the exact two-scan or bracketed-hybrid method. Its changed coordinates
  must equal the canonical difference of the compared local/divergence images. Complete capture
  must equal reconstruction from its complete phase evidence; equal images cannot claim Unstable.

Revised proposal SHA e23b99297e2aeb1b650d28a06db1a23a1598b46f12084280771cfa63f1dea47a.
The complete raw snapshot transcript and its proposed fixture constants are unchanged; phase
evidence is separately retained and hashed. No Rust implementation, fixture execution, acquisition
or operational control is claimed. The second and final technical review remains to be performed.
Full semantic/runtime/provider/phase-import/report and six-cutover acceptance is unchanged.

## Raw capture final-review correction and bounded implementation decision

The final independent report is recorded unchanged as review-result:raw-capture-design-pass-2,
SHAa987b686a97b8a529e343707f2ce8543bc93712bae46e82d6fef7959c172f0d3. It resolves all five prior
findings and introduces one blocker: the revised hybrid representation lost the resolved policy
needed to reconstruct its capture. Findings comparison reports zero carried, one new, five resolved.

Root corrected that exact omission: SourceCoordinateV1::Hybrid now contains required explicit
PresenceV1<HybridPolicyWordsV1>, after divergence_file. Complete/Unstable require Present admitted
words; all hybrid acquisition phases require it. Refused observations retain any obtained policy.
Complete reconstructs its policy from that coordinate and refuses contradictions; the source
transcript binds the policy presence/value. Missing policy supports only preflight refusal, never
fabricated defaults. All48 combinations, wrong/missing policy and post-snapshot failure retention
are mandatory pure tests. The Markdown transcript fixture is unchanged.

The coordinator accepts this corrected design for the bounded pure value/schema implementation
within the already approved initiative. This is a coordinator implementation decision, not a new
independent approval; the final reviewer verdict remains needs-revision. No third design review,
fabricated test result, operational admission or runtime-blocker clearance is claimed. Independent
source examinations and actual tests/gates remain required before source acceptance.

The pure task adds only contract values/validation/transcripts, generated schemas and tests, with
Rust1.85 preserved. Catalog acquisition/openers, writer controls, semantic mappings, durable migration
phases, reports, qualified ER/Eventlog runtime and all six real cutovers remain unimplemented owner
requirements. No competing migration owner is created and the full outcome remains intact.

## Full-gate scope correction for the planned backend

The pure capture full gate stopped on the draft scope naming
crates/plan/aep-backend-eventlog/, because the planned new crate does not yet exist.
The nonexistent path is removed from machine scope through the CLI. The proposed backend and
all migration acceptance remain unchanged. Its creation changes the already-scoped Cargo.toml
and Cargo.lock; those exact workspace surfaces reserve the change until the crate exists and
its concrete path can be added. Existing backend, CLI, contract and design scopes are retained.
No empty crate placeholder or guard waiver is introduced. The two source reviews remain closed;
the affected scope guard and the complete repository gate must pass before pure capture acceptance.

## Current completion and remaining whole migration delivery

Pure raw-capture values/validation/transcripts/schema are accepted and integrated in this selected owner lineage at715e52a88df9abcfcdd35752c09ab64b1e5a3fb3. Full taskcheck including actualPostgreSQL/MSRV/site exited0; bothsource examinationsclosed. Exact local-evidence:ess-evolution/waves/0005-aep-migration/raw-capture-full-gate/acceptance.md. Historical statements above describing this pure implementation as absent are superseded only for that bounded deliverable.

M3 remains incomplete: acquisition from all legacy backends, semantic/history mapping, the Eventlog planning backend and tracked projection, exclusive v2selection, inspect/dry-run/apply/verify/rebuild, durable phases and receipt-preserving recovery still require their complete source/check delivery. The command/config companion exists as a proposal and the phase/provider contract remains to be bound to actual accepted adapter APIs before implementation dispatch; this is the existing owner requirement, not new scope. A transport schema is not physical acquisition or operational control.

Runtime readiness: ERasync executoraccepted; SQLandcaptureaccepted; administrationcandidate43ceaa09 has fullprovider checks but independent source reviewwasinterrupted under recordedplatform restriction. ERadaptercandidate implementation runs final fullchecks against that frozen candidate. It is not yet accepted/integrated and dependency-blocker:recorded-eventlog-runtime remainsopen. No equivalentreviewretry or newreviewunit.

Writer fleet question remainsunanswered. Actualcompletefleet/start-stop-restart controls are required to select and verify source-specific exclusion or supervisedquiescence; localcopies are notthefleet. This blocks productionapplyqualification andsixrealcutovers, not read-onlydesign/source preparation or isolated tests. No assumedquiescence or mock-as-productioncontrol.

Nextwhole delivery contract remainsunderthissingleowner: completecommand/config/phase/provider specification using actualadapter receipts, then allremainingM3source and originalfullgates, followedby6orderedCacceptances. Stop onlyat named deliverable/gates or concreteexternalfacts; no perstoreowner/cellmicrotickets. Each operationalcutover remains separately fenced and evidenced. No realstore or primarylineage changed.

## Concrete command companion readiness

The existing planning-store-selection-and-commands design now distinguishes accepted pure capture at 715e52a88df9abcfcdd35752c09ab64b1e5a3fb3 from missing physical acquisition, semantic/history mapping, commands and durable phases. Its completed source examinations are not reopened. Concrete candidate ER open/provision/import/snapshot/rebuild/bridge interfaces are identified; their source preparation can proceed independently while final provider and adapter qualification remain required before integration.

Missing real writer control blocks apply qualification and real cutovers, not read-only inventory/acquisition/dry-run, exact mapping or phase-logic implementation against an explicit interface. Interface tests cannot qualify a real fence. No second migration story, new product scope, cutover or accepted runtime pin is created by this clarification. Complete remaining implementation stays owned here; the existing writer inventory question is still unanswered.

## Command-phase contract preparation

Approved requirement: M3, existing story:eventlog-planning-authority-migration. Existing raw capture715e52a8 is accepted; it does not define physical acquisition, semantic import, phase recovery or command results. Root has expanded the existing proposed command companion with source-cited acquisition boundaries, actual adapter import semantics, an eight-phase restart table, projection-after-selector ordering, exact identity retry rules and five closed result families. Diagnostic ApplyPhase vocabulary validates and compiles with installed ESS (exit0); no executable migration or qualified writer control is claimed.

Source evidence: aep-backend-sqlite constructors open mutating legacy stores; PostgreSQL constructors prepare schema; aep-backend-entity hydrate skips removed relations. Existing capture roster/schema contract is reused. Candidate adapter AsyncImportedAnchorWriter requires Imported origin and empty new suffix; LegacyEvidence preserves available envelopes/decisions/events without invented receipt coordinates. Provider completeness must be obtained from its port, not a forged marker.

Critical recovery consequence: before selector switch the old source remains untouched and authoritative; after exact intended v2 selection, repair projection from Eventlog and never reactivate the legacy source. Complete requires ordinary discovery verification plus immutable original receipt. A crash requires reacquisition of actual writer control, not reliance on a persisted held-fence assertion.

Remaining original deliverable: implement whole new backend/config/commands, exact Rust types/schemas/catalog acquisition and all acceptance cases; complete original reviews and gates. B-ADMIN-REVIEW and qualified adapter dependencies block integration. B-WRITER blocks real apply qualification/cutovers, not source work. This page creates no new migration owner or review round; its status remains proposed.

- docs/design/planning-store-selection-and-commands-v0.1.md SHA256 1420d2e69541cd45e0ba5932ff86c9502dd87a8f95ac1d6851412eea0ce7fe27
- specs/planning-migration/domains/migration.yaml SHA256 aac2ed5844677602649b56a53b0d71bc1d9b6725dedf2e3ae219adafbe37ee34

## Final contract review and complete implementation dispatch

Final whole contract pass2of2 is CLOSED NEEDS CHANGE, report840efbd59e5dade1b2ac927344f1230af1c8bba105c10306e35a90fba03a382a. F1/F2/F3/F5 resolved. F4-R1 requires exact unavailable-order envelopes and immutable original-ID reservations inside closed provider-complete boundary values; external recovery copies/locators alone are insufficient. F6-R1 requires ordered receipts/results, deterministic child identities and exact partial-commit retry semantics for retained move --via, new --relate and every equivalent ordinary route. Both remain explicit corrections under this owner; no third whole contract review.

The complete remaining M3 source assignment now includes those two corrections in the existing companion/model before implementing their semantics, then full Eventlog file backend, project/2 selection, acquisition/history/imports, five command families, durable phases, projection/drift/rebuild, ordinary mutation/driver/served outcomes and original required gates. Preserve retained independent multi-command behavior; do not silently replace it by atomic semantics. No per-store/finding owners or narrower refusal substitute. Exact contract: local-evidence:ess-evolution/waves/0005-aep-migration/complete-command-implementation-brief.md.

Candidate-facing source may use the consistent exact ER/adapter9769cc59 pin containing accepted core250f699. This is not qualified dependency admission. B-ADMIN/native stages/B-WRITER remain blockers to qualified executable and real store apply/cutovers. No provider administration changes or denied review retry. Pure minima stay1.85; affected Eventlog runtime1.91. All actual disposable-backend, interruption, idempotency, multi-step retry, canonical result/schema/alias, full AEP task check and applicable site checks remain mandatory.

## Complete snapshot and projection publication boundary

Approved requirement M3 requires complete authority capture plus drift detection and receipt-preserving projection recovery. The previous watermark wording left an impossible self-reference if a watermark hashes the current complete snapshot containing itself. Existing pure capture and earlier model checks do not prove publication behavior. Root resolved this within the existing owner and fixed acceptance contract: retain every subject in the complete transcript; an immutable one-creation watermark names exact prior complete snapshot S. Verify stable current C twice, validate exact watermark W/receipt, remove only that new W subject to reconstruct S, and require its complete hash to match. Every other subject, earlier watermark and suffix stays included; extra business/control/metadata writes refuse stale. Recovery after W commits reuses W and its original receipt without executing a business command again. No generic historical-prefix selection or metadata filtering.

Binding companion SHA256 cc5ff48fa4419fe7e5c576e57820208789ca757602c329ec2461f7fabe952fab. The bounded deliverable is this publication/verify/retry path plus decisive unchanged-success, crash-after-W, same-identity retry, intervening-write and altered-watermark controls; stop when these original projection requirements pass as part of the complete command deliverable. No new task, third contract review, qualified runtime claim or real-store cutover. Exact native/provider and actual writer-exclusion gates remain. The two now-existing new crates have been added to machine-readable scope; earlier absent-path scope workaround is superseded only for those concrete paths.

## Implementation and focused retry evidence

Five-package locked/offline check0017 exited0 for domain, contract, Eventlog backend, planning migration and CLI after the original F6 ordering correction and ordinary command identity wiring. Focused controls0022 exited0: projection failure after child0 stops child1; same reservation recovers the durable prefix without executing child0 again; a committed uncertain child retries its immutable child identity and recovers the identical actual Eventlog provider receipt, with authoritative business revision1. Root inspected the real provider setup and receipt assertions.

These are focused coordinator controls. Projection publication/snapshot values are injected in0022; this is not actual staged filesystem publication, watermark verification, generated CLI acceptance or writer-fence proof. The existing projection test now asserts the exact ordered trace: repair publication, child1 execution, child1 publication, terminal-result publication. Restored focused run0023 exited0; root inspected its terminal evidence. Complete operational command/backend/rebuild/inspect/hybrid equivalence and all original disposable provider/interruption/full gates remain under this same owner. No final F6 or M3 acceptance, qualified runtime integration or cutover is claimed.

Evidence directory local-evidence:ess-evolution/waves/0005-aep-migration/complete-command-implementation/ retains0017 and0022 commands/logs/exits and earlier failures.0017 log SHA256 61b8279bd56f73eb4eae77640aedf575b21b6dbdc0f43f501cbd586afe6853bd;0022 log SHA256 f967583669885f293ab3e9fa9764dafd43b1826a0d2d483536a5131ba23b77fd. Existing B-ADMIN/native-provider qualification and real B-WRITER restrictions remain unchanged; independent implementation continues.

Ordered control0023 log SHA256 30bb6d78bcc7ab511d8100c72c11aff2e85e93cab345145940df8039281ad3c0. This supersedes the missing-order-assertion note without broadening the proof to real projection publication.

## Provider-owned destination identity

Approved requirement M3 requires a fresh staged authority and exact durable restart, not an operator-prepared fixture. The current CLI freezes a caller-supplied physical stream identity before creation (store_command.rs apply path), while prepare_file/provision_file correctly expose/validate the provider-minted identity only after creation. Existing capture/compile/coordinator controls cannot prove this impossible fresh-stage path.

Root resolved the request/observation boundary in the existing command companion, section Provider-owned identity during fresh migration, SHA256 f852a9a08cee949a25e3ac1b94aa5175d23ca8d5f4e2a854bba1ea2f12e17d11. Fresh mode is explicit --authority-new with authored scope/tenant. Persist the complete immutable request under actual writer control before effects; within Prepared to DestinationProvisioned, create/reopen only that owned stage, obtain its actual identity, provision the adapter with it, and durably bind authority/receipt/exact final selector bytes. Every later phase/retry uses this binding. Dry-run stays read-only and reports an explicitly unassigned physical identity. Exact-existing mode cannot request creation under a guessed stream identity. No ER/provider API, extra preparation command or denied administration work is added.

Changed intent/dry-run/ownership/phase/pointer envelopes use explicit version2 replacements with a concrete closed field catalog and old-reader rejection before readers; existing v1 canonical bytes/readers, captured data, ordinary mutation and selector2 semantics stay protected. Eight logical phases remain. Bounded acceptance: fresh no-destination dry-run; actual empty-stage apply/import/select/reopen; interruptions around creation and binding persistence reusing identity/receipt; changed-request/foreign-stage/substitution/binding/selector refusal; format compatibility. These stop with the original complete M3 deliverable and gates, not a new owner or third contract review. B-ADMIN/native-provider and actual writer-fleet restrictions still gate qualification and real cutovers.

## Shared writer fence and complete history acceptance checkpoint

## Shared writer fence and complete history acceptance checkpoint

The original author contract remains active and incomplete. Shared new-build planning_writer_fence now routes actual explicit CLI, served and driver writes through the same held cross-process lock; actual separate-process commands0071/0074/0077 passed. This proves exclusion for the new executable only. External old writers and fleet supervision remain unknown B-WRITER, so real cutovers remain unqualified. No injected guard or disposable fixture supplies that authority.

Exact mixed Markdown journal bytes and order0082, SQL recorded-envelope bytes/global-ID roster/missing-blob and altered-byte refusal/recovery-copy deletion0083 passed. Unavailable-order complete envelopes0089 use the typed mapper seam and actual provider boundary evidence; current supported SQL acquisition always establishes subject order, and no nonempty CLI route to that optional seam is claimed. Full aep-cli0096 passed after missing-store fence acquisition was corrected to retain touch-free semantic refusal (0095,20cases).

Root supplied task-owned PostgreSQL fixture for the original required actualPG acquisition/backend/full gate. Exact owner/container identity and private env path at ~/beyond10x/.ess-evolution/verification/m3-disposable-postgres-20260916/receipt.json; worker acknowledged lifetime custody and exact guarded teardown. No external database or credentials borrowed. Complete PG/command/compatibility/full gates and immutable source/evidence report still required before author closure. Final contract review2 remains closed; no new unit or review-budget reset.

## Complete command implementation handoff

The complete backend and migration-command author assignment is closed. Local candidate a80a95f4b3ae7712708438ccbd8dc3f95dbe1bbb contains61 product/design/spec/generated paths atop715e52a88df9abcfcdd35752c09ab64b1e5a3fb3; the root planning lineage is preserved separately.

Full actual-PostgreSQL task check0137 exited0, including strict workspace checks, all tests, schema/rustdoc, pure Rust1.85, runtime1.91 and site build. Exact owned PostgreSQL fixture teardown and absence were verified. Author report digest eddb9e7d00afabbd082fc5910ff3572dafa715c8eb336d3688782b7c3b56e875; source manifest e5eac67d743d8867cb516f4a1bbefc6b892c885a935f25cc7c46ec5b2c215fd9;455 evidence files independently rehashed.

The common commit hook identified inherited mutable action references in the changed CI workflow. Coordinator corrected only those five references to resolved exact commits; the existing authoritative CI/release contract passed. Corrected61-path manifest3067161425e723d1f1f0f56ecc17b007b038b8930df2fd66c83636edbe1d4e65; signed common check and verify exited0. Gate0137 predates this workflow-only correction: the final integrated full gate remains required, rather than being claimed for a different exact tree.

The original implementation source examination remains pending; the two command-contract reviews remain closed. Public real-fleet apply/rebuild still selects UnavailableWriterControl until admitted fleet control exists. Cooperative new-build locks do not establish exclusion of old or external writers. Provider administration/native qualification remains external. Unavailable-order acceptance is mapper-level; current frozen physical acquisitions supply known order. No participating store cutover was performed. The full approved cutover scope remains authorized subject to its recorded gates; no renewed scope approval is requested.

## Original implementation source examination

Original complete implementation source examination pass1 CLOSED NEEDS-CHANGE at a80a95f4b3ae7712708438ccbd8dc3f95dbe1bbb. Reviewer report SHA2560d911c41a391f3ed555a895625e4ad1375b4e6ff07af8907c865f1db23b01cd6: local-evidence:ess-evolution/waves/0005-aep-migration/complete-source-review-1/report.md. Three tests-only regression cases were written but not executed because no compilation lane was granted. Reviewer lease released; no live processes remain.

Findings cover explicit v2 projection selection as Markdown authority, project/explicit-store lock aliasing, physical SQL schema predicates not actually captured/validated, and the known unconditional public UnavailableWriterControl selection. Unavailable-order support remains mapper-only with no admitted physical producer; report marks that limitation INFEASIBLE, not an executed failure. Root independently reproduced the original alias defect with the retained executable and disposable fixture; no reviewer execution is claimed. Root owns all finite correction, checks and integration within the original two-pass limit. Denied administration review remains separate and is not retried.

## Cooperative alias correction in progress

Root correction tree aep/ess-evolution-migration-correction-20260917 at a80a95f4 now holds seventeen source/test files, UNCOMPILED and unintegrated. Exact receipt ~/beyond10x/.ess-evolution/waves/0005-aep-migration/complete-source-review-1/coordinator-correction/receipt-postgres-prefix.json; diff SHA a71ad31d5c4ee8a6034efda76cb98b9988d1adfe050afa1c131080f22f9e967f. Selected-file rustfmt1.91.1 and diff checks exited0. Initial recursive format1 on unchanged planning/waves.rs remains retained; no runtime/full-gate pass is claimed.

Within the original M3 capture requirement (planning-raw-capture-v0.1.md:117-164,265-270), PostgreSQL now streams ordered catalog families, separates primary/unique/check reads, retains actual rejected catalog/data cells and completed prefixes, and stops later reads. Data row failures retain tableoid/ctid, and reads name the admitted namespace explicitly. Standalone and hybrid captures wrap unresolved or later SQL failures in the existing refused envelope, preserving earlier hybrid observations. SQLite/filesystem/projection/fence corrections remain. No new wire fields, format, baseline, product requirement, task or review round.

The existing pure DTO and successful-capture results were insufficient to prove physical failure retention. Added regression sources exercise a deferred unique key after completed primary-key capture; negative numeric rows across four data families with actual physical locators and unchanged source rows; foreign-view definition retention; unresolved standalone endpoint; and the public hybrid unresolved-replica prefix. ALL new and prior correction regressions are UNEXECUTED. PostgreSQL SQL execution and compilation remain unverified; selected formatting does not qualify this change.

Earlier refused-only validator corrections preserve invalid path units with explicit refusal and unresolved PostgreSQL/hybrid coordinates until observation, as required by the same original design. Present observed SQL sources cannot be hidden. Complete/Unstable path/source/digest requirements remain unchanged. The original final source review and required gates still apply.

Next source work within this owner: finish the public operational WriterControl path, assessing existing enforceable local controls. Selecting/activating it for the first Eventlog store still requires that store's actual writers/controller/start-stop/restart-exclusion facts; copies are not writers and absence is not exclusion. Storage custody remains UNASSIGNED; builds, fixture execution and cleanup are stopped. When a lane is granted, compile and execute the complete correction cases and required M3 gates, then the original final review and qualified integration. Stopping condition remains accepted public migration commands and the approved six real cutovers, subject to unchanged M1/M2 and writer-control gates; no cutover is claimed.

Source-specific control assessment (2026-09-17): the current cooperative PlanningWriterFence coordinates new AEP processes but excludes neither legacy writers nor restart. Eventlog FileTenantCapture acquires its strict writer lock inside each capture and drops it when that capture returns (eventlog-file/src/capture.rs:40-46,126-142); it exposes no held command-wide guard and cannot qualify apply/rebuild as currently exposed. No actual first-store supervisor/controller is identified. An additional generic selection wrapper around UnavailableWriterControl would not close this gap, so none was added. Timo has been asked which actors can write/restore this Eventlog source and which existing controller can stop/drain them and inhibit restart. The selected controller-specific implementation remains unfinished; no process absence, ownership lease, JSON assertion, new infrastructure or gate waiver substitutes. While that concrete input is pending, root corrects the independently actionable original M2 source-identity finding.

The original seventeen-file correction is now composed into the existing AEP SQLite compatibility tree ess-evolution-aep-sqlite-compatibility-20260916. Source hashes match the prepared composition; CHANGELOG retains both correction and dependency notes. No compilation, runtime verification or integration is claimed. Public writer control remains unfinished pending the first-store actor/controller facts already requested. Receipt: ~/beyond10x/.ess-evolution/waves/0012-connectors-adoption/source-composition/receipt.json. Storage custody UNASSIGNED; existing build and cleanup stops remain. Original completion contract and review limits unchanged.

## Execute existing physical-capture correction acceptance

The original17-file physical-capture/selector/fence correction has been source-only since00:15Z. Root rehashed all17 paths against receipt-postgres-prefix.json with no mismatch. Its original review1 identifies actual public authority-selection/lock-alias and physical SQL catalog admission defects; prior author gates predate the correction. Existing owner remains incomplete, with public operational WriterControl and qualified dependencies separately unresolved.

Root now assigns sdk_final_obligation_correction the bounded original correction-acceptance contract at local-evidence waves/0005-aep-migration/correction-acceptance/brief.md. Previous M2 final review assignment is CLOSED; this is separately justified M3 executable acceptance, not an extended review. Required outcome: original three reviewer regressions red/green, actual filesystem/SQLite/PostgreSQL retained-source and catalog cases, affected public-command/acquisition suites, full AEP task check including site/schema/minima, exact source/check report. The17original correction paths plus directly necessary demonstrated compile fixes are the scope; retain exact recorded provisional dependencies. No generic writer controller, first-store assumption, extra review or denied-work retry.

The M2 correction has finished all required builds and released fixture use; the M3 worker receives that sequential lane after root handoff, alongside existing S8. Owned PG remains root custody and may be explicitly reused instead of reprovisioned; no source or recovery data is cleanup-eligible merely from this handoff. Stop after the fixed acceptance deliverable, then close; root owns later source composition, final review readiness and qualified migration. No M3 completion/cutover is claimed, and final review2 will not consume its budget while public implementation remains unfinished.

## Original correction acceptance and concrete public-control boundary

2026-09-17T05:09Z: original three reviewer cases executed red against the original candidate and green against the correction. The affected contract/migration/CLI suite exited zero, but the later environment correction below invalidates its claimed live-PostgreSQL scope. SQL expression column aliases were made unique after a genuine earlier failure; values, order and original assertions stayed unchanged. Actual PostgreSQL restoration remains to be executed with the fixture exported. The selected-zero exploratory command08 is not acceptance evidence; exact09 selected and passed the original failure case. The only other change to the17-file handoff is a crate-level doc comment required by strict lint. The17-path manifest is frozen at SHA5f1a5b92ec8d3b798f243903d36c4907044442d84ca688aea5dafd286de5583c. Full AEP task check is running; no final correction acceptance yet.

Root composed these exact two changes into the existing AEP SQLite compatibility companion, preserving all other prior source and dependencies. Exact local-evidence waves/0012-connectors-adoption/source-composition/migration-catalog-aliases/receipt.json; diff check0. Composition is source-only; coherent qualified pins/locks and final-vector acceptance remain.

Current source narrows the public-control work: run_with_control already dispatches the complete command path; apply/resume/rebuild already acquire guards, and durable phases recheck/retire. Public run still always selects UnavailableWriterControl. Remaining product code is a concrete operational controller and its public selection, not another generic guard-wiring framework. Choosing that adapter requires the actual first Eventlog store's other writers and existing stop/drain/restart-exclusion control. That narrowly scoped factual question was presented directly to Timo again at05:06Z. No quiescence, inventory, approval or controller is inferred. Existing root/worker source work continues; original final review2 remains held until public implementation is ready. No third review or denied administration retry.

## Correction: later PostgreSQL green evidence skipped its fixture

The worker discovered that sourcing the private fixture assignments did not export ENTITY_POSTGRES_URL into subsequent child Cargo/Task processes. Full gate20 explicitly reported postgres-check skipped. Logs09/10/16 and20 therefore do not establish live-PostgreSQL acceptance; selected test names and zero exits cannot override their skip branches. Prior claims of actual-PG green acceptance for these commands are withdrawn. The original log07 PostgreSQL failure remains evidence of the duplicate-column defect; non-database results retain only their actual scope. No correction acceptance or complete M3 is claimed.

The worker now exports the owned fixture environment, verified child presence without exposing credentials, and connected to that exact disposable fixture with SELECT1. Required selected PostgreSQL cases will run with visible diagnostics, followed by the complete authored task check with exported environment. Gate20 may finish minima/site for diagnostic value but is not final acceptance. Root holds further source composition until final verified source/evidence, corrects the handoff, and preserves all failed/skipped logs. No new prerequisite or test count substitute.

## Final correction acceptance and checkpoint closure

2026-09-17T05:40Z: the original physical capture/selector/fence correction assignment is CLOSED after root verification of all17 final source paths, original red/green regressions, affected actual-PostgreSQL run21 and full actual-PostgreSQL task check22 including minima/schema/site. Earlier08/09/10/16/20 PostgreSQL claims remain withdrawn. Exact receipt: waves/0005-aep-migration/correction-acceptance/coordinator-acceptance.json; source manifest SHA d3d2e3e9e8bf0a1f3df32aeeaf76a7d3c8ce2644ee076510f330b8f1c09872f1. No further worker scope.

Root composed the ten final changed source paths into the existing AEP SQLite companion, preserving its composed changelog and dependency files; source-composition/migration-final-acceptance/receipt.json. Qualified integration is still open. Public operational control/first-store exclusion facts and unavailable-order physical/history reachability remain original M3 blockers; original final review2 stays held until public implementation is complete. No cutover, additional review, denied-work retry or gate waiver.

Checkpoint cleanup retired only released reproducible compiler/site caches:10,154,549,248allocatedbytes; sources, original tests, numbered evidence and mixed fixture scratch retained. Root disposable PG container stopped/removed and absence verified; bind data retained and no database bytes counted. storage-20260917-m3-correction-cache/ receipts.

## Remaining original legacy-evidence reader correction

Original capture/selector/fence correction acceptance is CLOSED and preserved. A distinct finite correction now addresses the existing unavailable-order reader promise (accepted migration-command design695-710; original review finding durable.rs394). Existing boundary joins/collision checks do not expose exact bytes through an ordinary reader. Scope is backend reader and existing boundary acceptance, preserving explicit imported/unavailable evidence and ordered HistoryProvider semantics; no new source backend or fabricated physical unordered input. Supported physical sources already supply admitted order; their unreachable unavailable path is not a demand to invent another source. Root confirmed current source before dispatch. Contract: waves/0005-aep-migration/legacy-evidence-reader-correction/brief.md. Worker starts source-only; a concrete API/format choice is reported before any new public wire/command contract. Existing M3 final review budget is unchanged, public writer-control facts remain external, and no complete migration/cutover is claimed.

## Legacy-evidence reader acceptance and checkpoint cleanup — 2026-09-17T06:16Z

The separately bounded reader correction is CLOSED. Root verified all19 source hashes and26 numbered evidence files, inspected the public backend lookup and complete boundary join, and composed exactly three changed source/design paths into the existing AEP SQLite companion while preserving its dependency files and changelog. Receipt: ~/beyond10x/.ess-evolution/waves/0005-aep-migration/legacy-evidence-reader-correction/coordinator-acceptance.json. Both RecordedCommit and RecordedObservation lookup/reopen/provenance regressions pass; affected all-target tests, default strict Clippy and formatting pass. This is bounded source acceptance, not the final full M3 gate or live PostgreSQL acceptance of the new reader. Prior full gate22 retains its exact older-source scope.

Worker lease ended; root retired only target/debug (2,193,989,632 allocated bytes), reverified all19 source hashes and unchanged diff, and retained target/tmp plus fixture scratch and every numbered log. Receipt: ~/beyond10x/.ess-evolution/storage-20260917-m3-reader-cache/receipt.json. No whole tree removed.

Next is coherent Eventlog/ER dependency composition and final full AEP gate. Concrete public writer-control adapter/selection, original final review and qualified integration remain; first-store exclusion facts are still required for activation. No extra reader assignment, review round or cutover is inferred.

## Final dependency composition and reader fixture correction — 2026-09-17T06:38Z

Root now composes checked local Eventlog6d5e249b83552b3417ce5b0c883999c95d304db4 and ER6a960962dd660d1e828797b7eb9c1db879ccad96 in the AEP SQLite companion. ER full actual-PG/runtime1.91, pure1.85 and site gates passed with exact13source hashes; Eventlog both required gates passed. Source URLs remain upstream Git identities; package-scoped Cargo lock resolution retains unrelated registry pins. These remain provisional for qualified M1/M2 integration.

The first composed full AEP gate reached the expanded reader regression and failed because its test selected an arbitrary first coordinate after adding a second envelope. Coordinate hash ordering is not record identity. Root corrected only the test selector to the known original observation ID and made its wrong-subject mutation select that same coordinate. The original exact-byte/provenance/refusal assertions are preserved; product source is unchanged. Focused case passed, and corrected full gate has passed that case plus actual PostgreSQL migration while later suites continue. Failure, exact delta and21-path corrected source preflight are retained under ~/beyond10x/.ess-evolution/waves/0012-connectors-adoption/sqlite-compatibility/aep-acceptance/. This is a demonstrated integration test gap in original §4 acceptance, not another prerequisite, worker extension or review round. Stop at passing complete gate and exact local freeze; the public control/final review/provider qualification blockers remain explicit.

## Composed source full acceptance — 2026-09-17T06:49Z

AEP2fc4035a0c749aa2a38d04243a15bebed3de149f is locally frozen after corrected complete task check0: actual PostgreSQL capture/CLI/backend, all workspace tests, default strict/docs/schema, pure1.85/runtime1.91 and website passed. Root reverified21 source hashes; common signed check/verify0. Receipt: ~/beyond10x/.ess-evolution/waves/0012-connectors-adoption/sqlite-compatibility/aep-acceptance/freeze.json. Initial test-selector failure stays retained; no product assertion relaxed. Tracked source clean; two untracked planning-writer lock files retained.

The Eventlog→ER→AEP SQLite compatibility companion is complete as local source acceptance, and Connectors now selects these exact pins. This does not complete M3: public operational writer control, original final review and qualified M1/M2 integration remain. No real-store cutover or publication. Root reclaimed5,614,059,520bytes of exact idle AEP compiler/site outputs and verified source/status unchanged; PG06c655878e0b stopped/removed/absence verified, bind data/env/evidence retained. M7 receives the freed bounded lane under its original contract.

## Complete public operational control after provider acceptance

# Complete public writer control within original M3

Owner: existing AEP story:eventlog-planning-authority-migration, canonical planning tree ess-evolution-aep-migration-20260915. Root is sole planning writer and final integrator. One Sol/high implementor under the recorded quota fallback, no children. This is the next ordered original product outcome after M1 and M2 are accepted locally, not an extension of the closed native worker or a new review project.

## Requirement and demonstrated missing implementation

Approved plan §4/M3 requires usable migration commands under actual writer exclusion and preserved authority/history. Existing public `crates/edge/aep-cli/src/store_command.rs::run` always selects `UnavailableWriterControl`; injected acceptance cannot establish a usable public command. Existing durable.rs already wires acquire, recheck, recovery and retirement. Reuse it, including AuthorityWriterControl for rebuild; do not build another guard framework. The cooperative project/explicit-path key mismatch and legacy reader/capture corrections are already accepted in this composed source; do not reopen them.

Timo's recorded model: the only writers are agent sessions he starts on this machine, no supervisor. He kills applicable writer processes and does not restart them until the new selector is verified. This identifies the operational authority and removes the former inventory question. It does NOT assert present quiescence. Bind the smallest concrete edge adapter and public selection to that model. Do not invent a supervisor, installed service, generic control platform or inventory of source copies. Existing operator decisions: ../../operator-decisions-20260918-critical-path.md. Read the binding command design and preserved writer-quiescence clarification for guarantees, distinguishing its old proposed supervisor illustration from today's actual operator model.

## Bounded deliverable

Implement the public operational path, required direct command acceptance, and concise adopter/design documentation in the existing source. The held capability binds actual selected source/configuration, migration or authority identity, observed stop/drain and continuing operator restart-exclusion custody. Recheck real current control before effects and reacquire after interruption. Preserve retirement and original receipt behavior. Missing, unsupported, stale, substituted or lost control must refuse; no `--force`, `--assume-quiescent`, caller assertion alone, deserialized claim of an old held guard, process absence alone or cooperative lock alone may establish exclusion. Existing trust boundary is ordinary operator-controlled writers, not an invented malicious same-user adversary.

First resolve the concrete interaction against those existing requirements and report it briefly to root before introducing a public format or extra command. This is implementation coordination inside this contract, not a new design/review assignment. If the supplied model cannot establish a particular invariant, name that invariant and the smallest missing fact/control; do not expand scope or silently weaken acceptance. Code and disposable tests can proceed without a real cutover or new operator approval.

Use actual public executable tests for absent/live/stopped control, wrong source/config/migration binding, lost control, interruption/reacquisition, successful apply/original retry/verify/rebuild and legacy-source retirement. Preserve original review regressions and complete imported evidence. Reuse existing fixture/helpers and native ports; no fake production provider or new general evidence engine. Required full AEP task check with actual PostgreSQL, existing minima/schema/site/common gates remain. Root supplies the disposable PG fixture when ready for final acceptance. No current real-store activation is permitted by this assignment.

## Source, execution and stop

Reuse managed `~/.local/state/worktree/trees/b10x/aep/ess-evolution-aep-sqlite-compatibility-20260916`, branch integrate/ess-evolution-public-control-20260918, base2fc4035a. Root has changed only Cargo.toml/Cargo.lock to accepted ER8b1757365f628338cf697f53deb9ce76acd25a99 and Eventlogf802eb8b01b44ba04a93394b20f0c07391f7757a. Preserve those changes and the unrelated existing untracked planning lock files. No new worktree. Acquire/heartbeat/release own lease ess-evolution-public-control-sol-20260918; root leaves source exclusively to this worker until freeze.

Write only relevant edge CLI source/tests, the smallest necessary existing migration port correction if demonstrably required, command design/adopter documentation/CHANGELOG, and derived schema only if a persisted public type changes. Existing machine scope covers these paths. No planning edits, baseline/accounting changes, M7/S9 work, commits/publication or independent reviewer dispatch by worker. Root handles final dependencies, bot commit, original remaining M3 source pass, acceptance and integration.

One build lane, maximum2 Cargo jobs, one test thread, no incremental/debug output, locked/offline dependencies, local target and TMPDIR outside Git. Global8GiB free hard stop. Inspect existing target scratch before cleanup; it includes retained noncompiler material. Do not delete whole target or managed tree. Put raw commands/exits/source manifest and one final report under `.ess-evolution/waves/0005-aep-migration/public-control-20260918/`.

Stopping condition: public control implementation and specified required author checks are complete on frozen source, or a concrete contract/execution blocker is demonstrated. Return the complete source/check/custody report, release own lease and CLOSE. No automatic follow-up extension. Root completes original final source review, composed gate acceptance and integration as M3; author closure alone does not close the milestone. Next real-store activation separately requires Timo's actual first-store stop/drain/no-restart custody.

## Current migration acceptance

M1 provider and M2 ER remain accepted locally. AEP632d7a3bd retains full gate and recovery acceptance for its observed-process writer mode. A subsequent fresh copied-store test passed import/verify/retry/history but demonstrated that a true already-idle source is refused because writer-pid is required. Current complete M3 usability qualification is open until the bounded idle-holder correction and composed acceptance pass.

The operator has now supplied the missing operational fact: no other writers exist and none will start until selector verification. The earlier request for writer identities is resolved; do not ask for a process that does not exist. Root selects this first control alternative: explicit operator assertion plus fresh live foreground custody, source/selector/authority checks and generation-bound revalidation. No synthetic observed-stop witness or silent process-absence inference. Source-denial was tested as an optional additional guard: ordinary writes refuse and capture identity is unchanged, but read-only publication cleanup is unqualified. It is not added as a completion prerequisite for the chosen operator-controlled alternative.

One bounded implementor owns existing CLI writer-control source, public tests and current design/reference/changelog. Its public red/green test demonstrates idle initial apply, matching retry, fresh reacquisition after selection, absent stop evidence/resume refusal, binding mismatches and authority rebuild. Complete affected/fmt/strict Clippy/minima/actual-PostgreSQL/schema/site task check and frozen handoff are still due. Original reviews remain closed; no new tree, accounting, diagnostic or review-budget reset.

Root owns bot/common integration, corrected idle-mode copy acceptance and then real Eventlog activation under the supplied commitment. Original source/recovery archive/history preflight is unchanged. No real cutover has happened. Complete Eventlog acceptance before ER, Service SDK, ESS, Connectors and AEP. The owner remains active until all real cutovers finish. Full scope/baseline/gates preserved; publication/release/deployment remain separate. Resource custody and receipts reside under idle-writer-control-20260919.

# Existing M3 history preservation correction

Owner: one Sol/high implementor, no children. Existing parent: AEP story:eventlog-planning-authority-migration. This is a separately bounded correction after public_writer_control CLOSED GREEN, not an extension of that assignment or a new review.

Approved requirement: ESS docs/design/ess-evolution/migration.md lines35–38 requires identity/revision/relation/body/evidence/history migration and preservation of available history. Source-verified gap: mapping.rs markdown_runtime_boundaries supplies empty anchor evidence; markdown_journal_items preserves Entry/DomainEvent bytes in auxiliary subjects; EventlogPlanningStore::events only returns anchor/suffix events; ordinary planning.rs history_from_the_contract/entries_from_the_contract reads only those events. ImportedLegacyEvidence presently exposes only rostered decision/observation envelopes. Existing public-control fixture has a document but no journal, so its passing gate cannot prove this requirement. Root's real-sized disposable rehearsal is running separately; do not touch it.

Deliverable: first demonstrate the ordinary public history loss in a small public-command migration test, preserving red output; if reproduction does not show loss, stop and report evidence rather than invent work. Then implement the smallest correction making original available history readable through ordinary history/explain after migration, preserving original provenance/order and appending new recorded suffix once. Cover relevant existing Markdown Entry and DomainEvent forms and SQL retained histories using existing suites. No manufactured genesis, decisions, receipts, inferred ordering, format weakening or store-file edits. Reuse validated retained boundary bytes; do not build a second persistence architecture.

Source tree: ~/.local/state/worktree/trees/b10x/aep/ess-evolution-aep-sqlite-compatibility-20260916, branch integrate/ess-evolution-public-control-20260918, base2fc4035a plus frozen public-control and qualified-pin changes. Preserve those changes and unrelated untracked planning locks. Read AGENTS.md and implementor charter ~/.codex/plugins/cache/beyond10x/aep-drive/0.9.3/agents/implementor.md. Use worktree skill, acquire own lease, release it on closure. No new tree, commit, publication, planning writes or review.

Scope: crates/edge/aep-cli/src/planning.rs and existing CLI migration/history tests; crates/plan/aep-backend-eventlog/src/lib.rs and its existing tests; crates/plan/aep-planning-migration existing tests only unless demonstrated mapping defect requires a minimal correction; existing command design and CHANGELOG. Outside that scope propose a patch to root. Do not change writer-control semantics or dependency pins.

Evidence/scratch: ~/beyond10x/.ess-evolution/waves/0005-aep-migration/history-preservation-20260919/. All logs and report there. Source-only review remains original final M3 pass, dispatched by root after this deliverable; no additional review budget.

Acceptance and stopping: retained red public-command regression, green original/new affected suites with source hashes, formatter/strict Clippy, complete task check with actual PostgreSQL and minimum/site steps. Stop at final frozen source/report and release own lease. No automatic follow-up. Root owns final source review, commit/common checks, integrated acceptance and cleanup. If prerequisite proof needs wider work, stop with the concrete contract gap.

Capacity: sole build lane, max2 Cargo jobs/one Rust test thread, debug0/incremental0, same existing target (do not set CARGO_TARGET_DIR), TMPDIR outside Git. No cleanup by worker. Root's already-running disposable apply/holder uses the old compiled inode and is separate from new tests. Check disk before substantial allocation; stop below8GiB free. Root-owned PG17.6 env is ~/beyond10x/.ess-evolution/verification/public-control-postgres-20260918/postgres.env: source/export without printing values. Root keeps fixture alive through your gate.

## Already idle writer custody correction

Operator confirmed there are no other writers to the selected Eventlog planning store and supplies continuing no-restart until selector verification. Fresh disposable-copy test proved the current public holder exits at required writer-pid when the source is already idle; its simulated observed-process path passed migration, verify, identical retry and preserved history. This is a concrete M3 public usability gap, not a need for further operator inventory.

One bounded implementor owns already-idle public holder support for initial apply, selected retry and rebuild, preserving fresh affirmative custody, current selector/source/authority checks, live generation challenges and old observed-stop witness semantics. No empty observed-stop evidence or inferred absence is admissible. Cited scope is existing CLI writer_control module, public/alias tests, current command design, CLI reference and Unreleased changelog; no dependencies, formats or migration engine changes. Required completion is red/green public regression, complete affected and full actual-PostgreSQL/minima/schema/site gates, frozen source and released lease. Root owns local integration, actual fixture and disposable source-denial procedure validation before real activation. Full owner remains active for all store cutovers. Original reviews stay closed; no new accounting or diagnostic project.
