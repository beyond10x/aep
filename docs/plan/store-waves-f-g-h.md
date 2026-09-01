# Store waves F, G, H — the storage layer is `entity-runtime`'s

> **Status: accepted 2026-08-28 by the operator, with every § 4 default taken.** Written the same
> day against `aep` 0.27.3 (`82a80e5`) and `entity-runtime` 0.9.1 (`dc5b25a`).
> Every fact in § 1 was re-read from those two trees on that date; nothing below it is a fact until
> a story ships. Wave F is in progress; the stories carry the record.
>
> Predecessors: **wave D** (`story:journal-backed-store`, P3) and **wave E**
> (`story:sqlite-backend-adapter`, P4) shipped in 0.27.0 and had no plan page — their record is the
> two stories and the 0.27.0 changelog section. This page exists so the next three do not repeat
> that: a wave whose acceptance is written down before the code is a wave a review can refute.
>
> Design behind the backend sequence: `harness-planning-and-driver-design-v0.1.md` § 2.8 (P1–P6).
> Design behind the seam: `entity-runtime/docs/design/store-v0.1.md`. The dependency arrow:
> `atlas/architecture/adr/0002` — this repository takes from `entity-runtime` and gives nothing back.

**Goal: after wave H this repository contains exactly one storage adapter and one storage provider
of its own, and everything else that keeps a plan durable is `entity-runtime`'s and is tested there.**
Postgres, hybrid and any store the runtime grows later arrive here as a *type*, not as a crate of
logic; the plan's own markdown store stops being a special case and answers the same provider suite
as SQLite; history is read from an event log rather than from a journal this repository parses by
hand; and a harness can ask the contract which statuses a story may hold instead of reading the
lifecycle document itself.

## 1. Where this stands — verified 2026-08-28

| fact | evidence |
|---|---|
| Two versions of `entity-core` are compiled into this workspace | `cargo tree -i entity-core` → *"specification is ambiguous: entity-core@0.5.2, entity-core@0.8.0"*; `crates/aep-backend-markdown/Cargo.toml:31` pins `0.5.2`, `crates/aep-backend-sqlite/Cargo.toml:21-23` pin `0.8.0`; `entity-runtime` is at `0.9.1` (`Cargo.toml:15`) |
| `AGENTS.md` says there is one `entity-runtime` dependency | `AGENTS.md:528` — *"the eleventh is `entity-core` in `aep-backend-markdown`, the only dependency"*. There are three crates from that repository in two of ours |
| `SqliteBackend` writes **no events** into the store | `crates/aep-backend-sqlite/src/lib.rs:189-192` — `Decision { instance, events: Vec::new() }`. `entity-sqlite` writes instance and events in one transaction (R-83); we hand it nothing to write |
| `SqliteBackend` cannot read a database back | `lib.rs:60-76` — `written` set; *"Hydration is P5; point this at an empty database until then"*. Relations, audit and history live only in the per-process `MemoryBackend` |
| `SqliteBackend` has no CLI surface | `crates/protocol-cli/src/planning.rs:501-503` opens `MarkdownBackend` only; `BackendArgs` (`main.rs:200-219`) has no backend selector; `protocol conformance` is hard-coded to `MemoryBackend` (`story:conformance-verb-takes-a-backend`) |
| `MarkdownBackend` is a second hand-written durability layer | `crates/aep-backend-markdown/src/backend.rs` — its own `persist`, latch, `journal::append`; 883 lines beside `store.rs` 706 and `journal.rs` 404. Both it and `SqliteBackend` wrap `MemoryBackend` and add durability separately |
| `history`, `audit`, `describe_type` are answered by the in-memory backend in both durable backends | `backend.rs:870-882`, `lib.rs:311-335`. `protocol artifact history` reads `journal.jsonl` by a separate path |
| D-P5 open: `TypeDescriptor::lifecycle` is `None` everywhere | `crates/aep-contract/src/registry.rs:78`; `journal-backed-store` acceptance line struck through |
| The markdown write path is fixed | `store.rs:271` pid+counter temp name, `store.rs:289` `sync_all` — the defect P3 found is closed; G1 keeps this shape |
| `entity-store`'s `Store` has no enumeration | `entity-runtime/crates/entity-store/src/lib.rs:147-190` — `load`, `revision_of`, `events`, `commit`. Nothing lists what a store holds, so nothing can hydrate from one |
| `entity-remote`'s `Hybrid` composes **any two** stores | `crates/entity-remote/src/hybrid.rs:168,175,411` — `Hybrid<L: Store, R: Store>`. A hybrid of two local stores needs no network |
| `DomainEvent` records what changed and not what it was decided on | `entity-runtime/crates/entity-core/src/runtime.rs:51-78` — `entity, version, id, revision, type, from_state, to_state, changed`. The evidence a move rested on has no field |
| `entity-runtime` opens no socket and its gate reaches no network | `crates/entity-remote/src/lib.rs:6-16`; `Taskfile.yml:5-16` |
| The planning store validates, and four implemented stories reached their rung on an **asserted** provenance | `protocol artifact validate` (0.27.3) → *valid*, with `guard-tests`, `journal-reconciliation`, `changelog-claims-are-checked`, `sqlite-backend-adapter` listed as *closed on an assertion*. A 0.26.0 build reports one `undeclared_reference` instead — the cross-repository edge from `story:assemble-across-sources` — which is the tool, not the store; ER's `plan-check` pins the minimum for this reason |

## 2. The end state, in six lines

1. **One adapter.** `aep-backend-entity::EntityBackend<S: entity_store::Store>` — the contract over
   any provider. `SqliteBackend`, `MarkdownBackend`, `PostgresBackend`, `HybridBackend` are
   instantiations.
2. **One provider of our own.** `MarkdownProvider: entity_store::Store` — a document's frontmatter
   is the instance, its body a field, `journal.jsonl` the event log. It passes `entity-runtime`'s
   provider suite and the `Broken` check, then our sixteen suites through the adapter.
3. **Events cross the seam.** Every accepted command becomes `DomainEvent`s in an `Envelope`, written
   with the instance in one `commit`. A refused command writes nothing (R-84).
4. **History is the event log.** `history`, `audit`, `protocol artifact history` and
   journal-reconciliation read `EventProvider::events`. D-P3 closes in full.
5. **`project.yaml` names the store.** `store: markdown | sqlite: <path> | hybrid: {…}`; every verb
   opens through it; a hybrid's four policy words are typed there with no default (R-106).
6. **`describe_type` reports the ladder** the kernel decides with. D-P5 closes.

Invariant 14 (*exactly one write path*) is what makes 1 and 2 a narrowing rather than a rewrite:
`crates/aep-contract/tests/write_surface.rs` refuses a second trait, and the adapter is not one — it
is the existing traits over a provider.

## 3. The three waves

| wave | name | stories | what a person gets |
|---|---|---|---|
| **F** | one adapter, one pin | F1–F5 | a SQLite plan you can close and reopen; one `entity-runtime` version; `protocol conformance --backend sqlite` from the command line |
| **G** | the plan's own store is a provider | G1–G4 | `protocol artifact history` answered from an event log; a hand-edited or deleted document reported as drift; ~1 500 lines of hand-written durability gone |
| **H** | every store the runtime has, and the ladder the store reports | H1–H4 | `store:` in `project.yaml`; Postgres and a markdown+SQLite hybrid as types; a harness can ask which statuses a story may hold |

Each wave is one release here and, where it has an `entity-runtime` story, one release there first —
the pin (F1) is what makes the order visible.

### Wave F — one adapter, one pin

| # | story | repository | after |
|---|---|---|---|
| F1 | `story:one-entity-runtime-pin` | EP | — |
| F2 | `story:one-adapter-over-any-store` | EP | F1 |
| F3 | `story:events-reach-the-store` | EP | F2 |
| F4 | `story:sqlite-hydrates-on-open` | EP | F3, ER `story:store-enumeration` |
| F5 | `story:conformance-verb-takes-a-backend` *(exists, draft)* | EP | F2 |
| — | `story:store-enumeration` | ER | — |

**F1** ends the ambiguous `cargo tree`. One tag for `entity-core`, `entity-store`, `entity-sqlite`;
a `dep-check` gate step that fails when any `entity-*` crate appears at two versions; `AGENTS.md`
§ *Dependencies* names three crates and the bundled SQLite they bring. Small, and first, because F2
cannot be written against two kernels.

**F2** extracts what `SqliteBackend` already is — `MemoryBackend` + latch + `persist` over a `Store`
— into `EntityBackend<S>`. `SqliteBackend` becomes `EntityBackend<SqliteStore>`; the sixteen suites
and the faulty-backend guard pass unchanged. The markdown backend is **not** touched in F: it has
plan-shaped projection work (templates, relations into frontmatter, `unprojected`) that G owns.

**F3** puts the audit record into the store. Today `persist` commits `events: Vec::new()`, so
`entity-sqlite`'s one-transaction guarantee protects an instance and an empty list. Each accepted
command's `AuditRecord` becomes one `DomainEvent` per affected entity, sealed with a `Recording`
(`recorded_at`, `correlation` = the command's correlation, `causation` = the command id, `actor`).
Acceptance is read through a **second** handle, as P4 did: `events(entity, id)` returns one event per
accepted command and zero for a refused one.

**F4** makes open read the database back. It needs one thing the runtime does not have —
`store-enumeration` — and one decision here: identities. `MemoryBackend` mints from a per-process
counter, which is why the foreign-row refusal exists. Hydration installs the **stored** identity, and
the refusal retires because it no longer protects anything. Acceptance: a second process against a
populated file sees every entity, relation and audit record the first wrote, with the same ids;
`protocol artifact history` over a SQLite store equals the same command over the markdown store of
the same plan.

**F5** is the existing story, unchanged: `--backend memory|markdown|sqlite --store <path>`, default
`memory`, and the report names what it ran against.

### Wave G — the plan's own store is a provider

| # | story | repository | after |
|---|---|---|---|
| G1 | `story:markdown-documents-as-a-store` | EP | F2 |
| G2 | `story:markdown-backend-is-the-adapter` | EP | G1, F3 |
| G3 | `story:history-from-the-event-log` | EP | G2, ER `story:events-carry-what-they-were-decided-on` |
| G4 | `story:out-of-band-edit-is-drift` | EP | G3 |
| — | `story:events-carry-what-they-were-decided-on` | ER | — |

**G1** is the load-bearing story of the three waves. `MarkdownProvider` implements
`StateProvider`, `EventProvider` and `Store` over `.engineering/planning/`: `load` parses a
document's frontmatter into an `EntityInstance` (fields = frontmatter, `lifecycle_state` = `status`,
`revision` = `revision`, body = a `body` field); `events` reads `journal.jsonl` filtered to the
instance; `commit` checks `Expect` against the file's revision, writes the document with the existing
temp-name + `sync_all` path, appends the sealed events, in that order, and states which failure leaves
what. It runs `entity_store::conformance` — the runtime's own suite — plus `a_broken_provider_is_caught`
against a deliberately wrong copy of itself. This is the first time the markdown files are held to
a suite written by somebody who has never seen them.

**G2** replaces `MarkdownBackend`'s body with `EntityBackend<MarkdownProvider>` plus the projection
hooks the plan shape needs: the kind's template body on create, relations into frontmatter,
`unprojected` for entities addressed elsewhere. `persist`, the latch and `journal::append` in
`backend.rs` are deleted, not kept beside. The D-P1 scans (`the_only_write_path_out_of_this_crate_is_a_command`,
`no_planning_verb_writes_to_the_store_except_through_a_command`) still hold. Acceptance includes a
**golden test over this repository's own store**: `list`, `board`, `graph`, `validate` and `history`
byte-identical before and after, because the point of the wave is that nobody can tell.

**G3** closes D-P3 in full. `history()` and `audit()` in the adapter read `EventProvider::events`,
so they answer the same from a fresh process; `protocol artifact history` and
`journal-reconciliation` read the same log. The provenance a move rested on —
`{"recorded": {"test_result": 1}}` today — needs a field on the event; that is the runtime story
beside it, and until it ships the provenance travels in the envelope's `causation` and the story
says so.

**G4** spends what G1 and G3 bought. R-89 says an event records the state before and after and the
fields written, so a document whose frontmatter does not match its own last event is **drift**, and a
document with events and no file is a **deletion**. `protocol artifact validate` reports both, per
document, without preventing either — D-P2 and D-P4 close *by detection*, and this page records that
prevention was considered and refused: a hook can be bypassed by `Bash`, and a check that runs in the
gate cannot.

### Wave H — every store the runtime has, and the ladder the store reports

| # | story | repository | after |
|---|---|---|---|
| H1 | `story:store-selection-in-project-yaml` | EP | F4, G2 |
| H2 | `story:describe-type-reports-the-ladder` | EP | F2 |
| H3 | `story:postgres-backend` *(exists, draft; re-scoped)* | EP | F3, ER `story:postgres-provider` |
| H4 | `story:hybrid-backend` *(exists, draft; re-scoped)* | EP | G1, H1 |
| — | `story:postgres-provider` | ER | — |

**H1** adds `store:` to `aep.project/1`. `markdown` (the default, so no existing project changes
meaning), `sqlite: <path>`, `hybrid: {authority, read, on_unreachable, on_divergence, local, replica}`.
Every `protocol artifact` verb opens through it; `--store` remains the path override. A hybrid block
missing any of its four words is refused at validation — the runtime's R-106, enforced at our edge.

**H2** fills `TypeDescriptor::lifecycle` from the same `EntityDefinition` that
`aep-backend-markdown::kernel` builds to decide a move, in the adapter, so all backends report it.
Acceptance: `describe_type(story)` lists exactly the states and transitions
`protocol artifact lifecycle story` prints, pinned by an equivalence test over every kind in the
store. D-P5 closes.

**H3** becomes `EntityBackend<PostgresStore>` — a few lines here once the runtime has the provider.
What stays ours: the CI service, the sixteen suites against it, and the two-writers acceptance
(one accepted, one `RevisionConflict` naming the revision it lost to). The runtime's provider carries
R-103 (one transaction) and its own broken-provider check.

**H4** is the story P6 always was, with its open question answered by what now exists. The
composite is `Hybrid<MarkdownProvider, SqliteStore>` — the plan in markdown for pull requests and in
SQLite for tooling, both local, no network. The atomicity guarantee is **not** chosen here: it is
the runtime's declared policy (`store-v0.1.md` § 10 — authority, read path, unreachable, divergence,
recorded rather than swallowed, `catch_up` merges nothing), and the story's first acceptance line
(*"written down first, as a decision with its rejected alternatives"*) is met by citing it. Two
verbs are ours: `protocol artifact divergences` and `protocol artifact catch-up`, over
`Hybrid::divergences()` and `Hybrid::catch_up()`.

## 4. Decisions this plan takes

Each with the default taken if nobody answers. Silence does not block the work.

| # | question | default |
|---|---|---|
| D-F1 | Hydration identities: how does a second process get the first process's `EntityId`s? | The adapter derives the identity from the stored instance id, the way `seed` already derives command ids and idempotency keys from artifact ids; `MemoryBackend` gains an "install with this identity" path used only by hydration |
| D-F2 | Which `entity-runtime` tag does F1 pin? | The newest release on the day F1 lands — `0.9.1` today. Not a floating `main`: the pin is the reversible half of ADR 0002 |
| D-G1 | Does `MarkdownProvider` live here or in `entity-runtime`? | **Here.** A frontmatter-document provider is generic enough for the runtime, but ADR 0002 says nothing of ours appears there, and a provider shaped by one adopter's files is exactly the thing the arrow rule exists to keep out. Revisit only if a second adopter asks |
| D-G2 | Where does a move's evidence provenance live on the event? | An `args` (or `decided_on`) map on `DomainEvent`, added by the runtime (`story:events-carry-what-they-were-decided-on`). Until then, in the envelope's `causation`, labelled |
| D-G3 | Prevent out-of-band edits, or detect them? | **Detect.** `protocol artifact validate` reports drift per document; no hook, no lock |
| D-H1 | Does the runtime's gate reach a Postgres? | **Opt-in.** `entity-postgres` tests run when `ENTITY_POSTGRES_URL` is set, in a CI job with a service container; `task check` without it stays green and **prints that the step was skipped** — no silent cap |
| D-H2 | Which two stores does the first hybrid compose? | `MarkdownProvider` + `SqliteStore`, both local. A remote replica waits for a `Transport` somebody ships, which the runtime has decided not to |
| D-H3 | `story:postgres-backend` carries `depends_on: story:sqlite-backend` (superseded) and `story:hybrid-backend` carries `depends_on: story:postgres-backend` (no longer true) | The store has `relate` and no `unrelate`. The correct edges are added beside the stale ones and each story body names which is which; a `supersedes` edge from `sqlite-backend-adapter` to `sqlite-backend` records the replacement |

## 5. Not in these waves, and where each lives

| item | why not here | where |
|---|---|---|
| a real `Transport` (HTTP, NATS) | the runtime refuses to ship one; a server is a product | `entity-runtime` `story:remote-provider` § *Out of Scope* |
| `story:entity-runtime-mapping` — the verb vocabulary verdict | vocabulary, not storage; it gates nothing in F–H because operations stay named for their target status | that story |
| `story:generator-version-stamp` | release mechanics, not storage | that story |
| the four lifecycle concepts (`decision-with-default`, `time-based-transitions`, `blocker-relation`, `outbound-claims-and-status-vocabulary`) | ladder semantics; three of four are already YAML in `artifacts/lifecycles/`; the clock question is `aep-domain`'s, not the store's | `epic:adopter-feedback-round-1` |
| `story:completion-audit-join` | it is *what G3 enables*, from the evidence side; it stays under `epic:evidence-gated-completion` and gains `depends_on story:history-from-the-event-log` | that story |
| `docs/roadmap.md` in `entity-runtime` § 1 stating a false "blocking fact" | housekeeping, flagged by their own `next-wave-the-shell.md`; a task there, not a story here | `entity-runtime` `task:roadmap-page-is-current` |

## 6. The three refusals every wave keeps

1. **Nothing of ours enters `entity-runtime`.** Three stories are proposed there; each is a kernel or
   SPI capability with no `aep` in it (`atlas/architecture/adr/0002`).
2. **`entity-core` stays IO-free.** F–H touch `entity-store` and its providers; the purity scan over
   `entity-core` is unaffected and is the runtime's to keep.
3. **One write path.** No new trait, no `PlanningStore`; `write_surface.rs` decides. The adapter is
   the existing `CommandService`/`QueryService` over a `Store`, and the scans that pin D-P1 must pass
   before and after G2 without being edited.

## 7. How each wave is accepted

The same way D and E were, with the page added: the stories reach `implemented` through
`protocol artifact move` on **recorded** evidence; the wave's release carries the changelog section;
two independent reviewers read the released commit; findings are fixed in the next patch release
with the corrections named. `docs/status.md` grows one row per wave via `cargo xtask status`.
