---
format: aep.planning-md/1
id: story:entity-runtime-mapping
kind: story
status: implemented
title: Another repository has already expressed our eight ladders as data, and asks for a verdict
summary: entity-runtime carries artifacts/lifecycles/*.yaml as eight entity definitions with an equivalence test pinned at 79b641c. Phase 0 of its adoption design asks whether the reading is faithful, and whether entity-core becomes an ordinary dependency here.
owner: protocol
tags:
- adoption
- lifecycle
- protocol
revision: 9
---
# Story: Another repository has already expressed our eight ladders as data, and asks for a verdict

## Outcome

The protocol owner has said **accepted**, **accepted in part** or **refused** to one specific
outside reading of `artifacts/lifecycles/*.yaml`, and has said separately whether taking
`entity-core` as an ordinary Cargo dependency is worth pursuing. Either answer ends the state this
story exists to end: another repository holding a pinned copy of our documents and deciding alone
what to do when they move.

## Context

### Four rows of this register are, in substance, *a Rust enum cannot say this*

This is the half of `story:open-vocabulary-audit`'s question that the audit's own table cannot
reach. `docs/guide/open-vocabulary.md:63-80` splits artifact status into two rows on purpose — the
ladder document is open, the values it may name are closed — and the closed row's guarantee cell
(`docs/guide/open-vocabulary.md:99`) reads *"a status name means the same rung to every tool that
reads the artifact graph, so a lifecycle written elsewhere can be compared with one written here"*.
A lifecycle written elsewhere now exists, and the comparison is committed, so that guarantee is a
tested claim rather than a stated one. The row below is the first evidence for it.

| our row | what it cannot say | what a data-driven kernel changes |
|---|---|---|
| `docs/plan/gap-register.md:70` | `correction-owed`; `expired`/`failed`/`blocked` flatten onto rungs that mean something else. `ArtifactStatus` is a ten-variant enum, `crates/aep-domain/src/artifact.rs:707` | a rung is a line in a YAML file. Their `story:aep-open-status-vocabulary` is this row and nothing else, and cites it by number |
| `docs/plan/gap-register.md:73` → `story:blocker-relation` | a blocker typed by what clears it | an `enum` field with declared values is data there today (`entity-runtime/examples/order.yaml:18-19`) |
| `docs/plan/gap-register.md:73` → `story:decision-with-default` | a decision with a required `default` and a required `expires` | the `default` half is data; the expiry half is not — the row below |
| `docs/plan/gap-register.md:73` → `story:time-based-transitions` | a transition the clock can trigger | **not data there either.** Their kernel refuses a clock and has no `$now` (`entity-runtime/docs/requirements.md` R-55, R-62). What it offers is a comparison against an argument the shell supplies — which is the shape our own story already demands: *"the clock is read at the edge and passed in"* |
| `docs/plan/gap-register.md:39` | a story's `implemented` is a claim nothing checks. `move_status` consults `LifecycleRegistry::for_kind`, falls back to `ArtifactLifecycle::permissive()`, and reads nothing else (`crates/aep-backend-markdown/src/document.rs:115-142`) | a `preconditions:` block is data (`entity-runtime/examples/order.yaml:96`), and as of their `ae4e040` the rule can say `unknown`: a comparison over a fact nobody recorded refuses as `PreconditionUnobservable` naming every address it could not read, while `exists` stays two-valued. Their `Truth` is ported name for name from our `aep-domain::predicate::Truth`. Invariant 5 is satisfied |
| `story:open-vocabulary-audit` | the meta-defect itself | — |

**The honest count is three, not four.** Three of those capabilities are expressible as data in
the kernel as it stands today — the third became so when they shipped three-valued rules, which
they built because *our* invariant 5 demanded it and which was the only blocker they had that was
ours. What is not data there is the clock: `story:time-based-transitions` needs a `$now` their
kernel refuses to have, and the expiry half of `story:decision-with-default` with it. Both reduce
to a comparison against an argument the shell supplies, which is the shape our own story already
demands. The asymmetry that survives the correction is still the argument:
`story:decision-with-default`, `story:time-based-transitions`, `story:blocker-relation` and
`story:outbound-claims-and-status-vocabulary` are four Rust changes here and, on the far side of a
clock read at the edge, four YAML edits there.

**One register row was dropped from this table because it is stale.**
`docs/plan/gap-register.md:77` says *"`parent()` is defined over built-in variants only, so custom
kinds cannot share a ladder"*. It no longer is: `crates/aep-domain/src/artifact.rs:517` reads
`Self::Other(name) => Self::suffix_parent(name)`, and `for_kind` resolves a custom kind through that
chain — asserted at `crates/aep-domain/src/artifact.rs:2142-2153` for `observation-log` and
`weekly-digest`. That row is owed a *closed by code* line under this page's own rule; writing it is
not this story's job, and it is named here so the absence is on the record.

### What exists on the other side, stated as fact

`entity-runtime` (`github.com/beyond10x/entity-runtime`, `0.2.1` at `Cargo.toml:10`, Apache-2.0 at
`Cargo.toml:12`) carries all eight of our `artifacts/lifecycles/*.yaml` as entity definitions under
`examples/aep/`.

| | states | operations | edges |
|---|---|---|---|
| `initiative`, `epic`, `story`, `task` | 6 | 6 | 9 each |
| `design`, `specification` | 7 | 7 | 12 each |
| `architecture-decision-record` | 4 | 3 | 3 |
| `review-result` | 2 | 1 | 1 |

Sixty-four edges in total, which is what our own documents declare: summing the `transitions` maps
in `artifacts/lifecycles/*.yaml` at this commit gives 3 + 12 + 9 + 9 + 1 + 12 + 9 + 9 = 64.

`entity-runtime/crates/entity-yaml/tests/aep_lifecycles.rs` holds the equivalence: eleven tests
comparing each definition's `(from, to)` edge set, its declared states and its initial state against
our `transitions` maps, read from a fixture committed at pin `79b641c` with a sha256 per file
(`crates/entity-yaml/tests/fixtures/aep-lifecycles/PIN.md`). Their README states the comparison runs
in both directions — an edge invented there fails, an edge our ladder grows and theirs does not
express fails too — and that it was verified by breaking it; that last claim is theirs, not
re-derived here. The test is inside their gate: `task check` runs `test`, which is
`cargo test --workspace --locked` (`entity-runtime/Taskfile.yml:26-29`).

**Nothing here changed to make that true, and no dependency exists in either direction.** No
`Cargo.toml` in this workspace names `entity-core`; none in theirs names a crate of ours.

### The unreviewed half: the verbs are their invention

The equivalence test pins states, initial state and edges. It **cannot** pin operation *names*,
because there is nothing here to pin them against: our lifecycle documents declare target states only
(`artifacts/lifecycles/story.yaml` `transitions:`), and `protocol artifact move --to <TO> <ID>` names
the target state, never a verb. Their own design's phase-1 row claims a `(from, operation, to)`
comparison; the test compares `(from, to)` — `type Edge = (String, String)` at
`crates/entity-yaml/tests/aep_lifecycles.rs:19`. The operation name is carried in the map's value and
checked only for one thing: that no two operations declare the same edge.

So eleven verb names entered the mapping unreviewed:

| kinds | verbs |
|---|---|
| `initiative`, `epic`, `story`, `task` | `propose`, `return`, `activate`, `reject`, `implement`, `archive` |
| `design`, `specification` | `submit`, `request_changes`, `approve`, `reject`, `implement`, `supersede`, `archive` |
| `architecture-decision-record` | `accept`, `reject`, `supersede` |
| `review-result` | `archive` |

The **schema** half is pinned against nothing at all. `title`, `summary`, `owner`, `tags` and `body`
as `json` are their reading of our frontmatter; `artifacts/kinds/*.yaml` and its `required_sections`
are not modelled, and their README says so.

### The collision with `story:journal-backed-store`

`story:journal-backed-store` (D-P3, `docs/plan/gap-register.md:37`) reroutes the markdown store's two
write functions through `CommandService` envelopes and makes the journal the history. The adoption
proposed here reroutes the same store's *verdicts* through a kernel. Both cut the same seam in
`aep-backend-markdown`, and whichever ships first without the other in view builds it twice — the
second one arrives as a rewrite of the first. **This is one decision, not two designs**, and the
sequence matters more than either answer.

### A finding from using our own tooling

`protocol artifact` has `relate` and no `unrelate` (`protocol artifact --help`: `new`, `move`,
`relate`, `body`, `list`, `board`, `graph`, `validate`, `kinds`, `relations`, `lifecycle`). A wrong
edge can be added and never removed — only annotated with a corrected one pointing the other way,
which leaves both in the graph. That was hit for real while building the store on the other side.
Invariant 16 (*nothing is physically deleted*) is the reason and is a good one; the consequence for
an author is that the graph accumulates edges nobody meant, and `graph` cannot distinguish them from
the intended ones. Worth a register row: a mistyped relation is not a superseded fact, and the
vocabulary has no way to say so.

## Verdict — 2026-08-28

**Accepted in part.** An adopter reading either repository can from today rely on this: the eleven
ladders this repository publishes and `entity-runtime` re-expresses agree — same states, same
starting rung, same edges — and **each repository now holds a pinned copy of the other's documents
and fails its own gate when the two drift apart**, so the agreement is a tested claim rather than a
stated one. What nobody may rely on is a **verb**. `propose`, `activate`, `implement` and the eight
other operation names in their `examples/aep/` are that repository's invention, this verdict endorses
none of them, and no surface published here names one.

The mechanism is a second equivalence test, on this side of the boundary.
`crates/aep-backend-markdown/tests/entity_runtime_equivalence.rs` reads their `examples/aep/*.yaml`
from a committed fixture pinned at their tag `0.13.0` (`ddee747`), with a sha256 per file held in
both directions by `the_pinned_copy_is_the_bytes_this_pin_records`, and compares it against
`artifacts/lifecycles/*.yaml` read through `ArtifactLifecycle` — the type `Document::move_status`
actually decides moves with, so this compares what the code reads and not merely what the YAML says.
Six tests over eleven kinds:
`each_definition_declares_exactly_the_states_our_transitions_map_declares`,
`each_definition_starts_where_our_ladder_starts`, and — the claim the verdict rests on —
`each_definition_yields_exactly_the_edges_our_transitions_map_yields`, which compares **77 edges**
in both directions and asserts that total, so a ladder cannot quietly leave both sides at once.
Verified by breaking it: moving `story`'s `activate` from `proposed` to `draft` in the fixture fails
naming the kind and both halves — *in our ladder, not expressed there: `[("proposed", "active")]` /
expressed there, not in our ladder: `[("draft", "active")]`* — with the pin check failing beside it.

The twelfth ladder is `outbound-claim`, and it is **not** compared: it landed here (`bba1a15`,
`4d331a0`) after the commit their fixture pins, so no definition for it exists there yet. It is
named as a constant in the test rather than filtered out by absence, so a *thirteenth* ladder growing
here without a counterpart fails by name — which is the failure their own fixture once had, staying
green about eight ladders while nine existed.

### The verbs: refused, and the refusal costs nothing

There is nothing here to pin a verb against. Our lifecycle documents declare target statuses only,
and `protocol artifact move --to <TO>` names a status, never a verb; their own test compares
`(from, to)` and carries the operation name solely to detect two operations claiming one edge. So the
eleven names entered the mapping unreviewed and stay unreviewed. **Phase 2 already took this line
without being told to**: the operations `crates/aep-backend-markdown/src/kernel.rs` builds are named
for their *target status*, so the kernel decides our moves without a verb of theirs reaching our
wire (`entity-runtime/docs/design/engineering-protocols-adoption-v0.1.md` § 3, *Phase 2, as built*).
Any later phase that puts a verb on a published surface needs its own decision — `move --to
implemented` and `execute --operation implement` are different things to have promised.

### The sequencing question, answered by history — and not the way this story predicted

The story's default was *the journal first*. **It went the other way, and the fear was unfounded.**
The kernel-decided move landed first: `f20c9d6`, 2026-08-25 14:44, released as 0.13.0 —
`PlanningDocument::move_status` asks `crate::kernel::permits_transition`, held identical to the
lookup it replaced over 800 ordered status pairs by
`crates/aep-backend-markdown/tests/kernel_equivalence.rs` (`docs/plan/gap-register.md:39`). The
journal landed **behind** it the same day (`ab48bc8`, 23:54, released in 0.19.0), and
`story:journal-backed-store`'s own acceptance — the `CommandService` envelopes and the sixteen
conformance suites — behind that on 2026-08-26, shipping in 0.27.0 as wave D
(`docs/plan/store-waves-f-g-h.md:8`; `docs/plan/gap-register.md:37`).

What this story predicted was that whichever shipped first would be built twice. It was not. The
envelope work added `MoveStatus`, a command that *applies* a decision and does not take one — the
ladder still decides, through the kernel, before the command is issued, because "a backend that
re-decided it would be a second protocol" (`story:journal-backed-store`). `kernel.rs` has been
touched three times since and every one added a rung guard rather than unpicking the seam:
`a193caa` (evidence), `c0ee6e8` (dates), `6069178` (project addressing). **The sequencing question
is closed as moot**: both seams are cut, in the order this story argued against, and the cost it
priced was never paid.

### The schema half: no verdict now, and that is the answer

Default taken. `title`, `summary`, `owner`, `tags` and `body: json` are their reading of our
frontmatter and claim nothing about `artifacts/kinds/*.yaml`; `required_sections` is not modelled,
their README says so, and the test above compares no field. An unmodelled `body: json` cannot be
wrong about our kinds, so there is nothing yet to accept or refuse. When `required_sections` becomes
fields over there, that is a new reading of a document of ours and it earns its own verdict.

### A mistyped relation gets no way out

Default taken: **no `unrelate`, and no soft-delete flag either.** Invariant 16 — *nothing is
physically deleted* — wins, and the consequence for an author is concrete: a wrong edge is
permanent, `protocol artifact graph` renders it exactly as it renders an intended one, and the only
correction available is a *second* edge annotating the first, which leaves both. That is not
hypothetical — `entity-runtime`'s own store still carries `story:aep-lifecycles-as-definitions
depends_on story:aep-mapping-review`, decided backwards on 2026-08-25, with the correction
`story:aep-mapping-review informed_by story:aep-lifecycles-as-definitions` written beside it rather
than instead of it. Both edges are in the graph and `graph` renders them alike.

**And three times in this repository's own store**, each corrected in prose because prose is the
only correction there is: `story:governed-dogfood-run` carries a stale
`depends_on: story:driven-eval-acceptance` and says so (`:76-78`); `story:driven-eval-acceptance`
names the same edge from the other end and points at this very row (`:63-65`); and
`story:postgres-backend` carries `depends_on` against a superseded story, tracked as D-H3
(`docs/plan/store-waves-f-g-h.md:204`). Four instances is not an edge case. What is owed is a **vocabulary, not a verb**: *this edge was a
mistake* is a different fact from `supersedes`, which says the target was once right. The register
row this story asked for is written — `docs/plan/gap-register.md` § *Open, from 2026-08-28 — using
our own tooling from another repository* — so an author is told before making the mistake rather
than by making it.

### The dependency: not a decision to take, a fact to record

For a person, this is what it already means: `protocol artifact move` refuses **in-process**, so the
refusal names the rule and the unresolved reference instead of arriving as a string somebody
re-parses, and anyone who clones this repository and runs `cargo test` gets it without installing
anything of theirs. The mechanism: this workspace takes **five** `entity-runtime` crates —
`entity-core`, `entity-store`, `entity-sqlite`, `entity-postgres`, `entity-remote` — declared once
in `Cargo.toml:112-116`, all at git **tag `0.13.0`**, with `cargo xtask deps` (the gate's
`dep-check`) failing if the lockfile ever resolves two pins or two versions (`AGENTS.md`
§ *Dependencies*). The arrow is one-way and permanent: no manifest in `entity-runtime` may name a
crate of ours, at any version, ever
(`atlas/architecture/adr/0002-the-entity-runtime-dependency-arrow.md`).

That ADR's own § *Taken, 2026-08-25* records that its step 3 — **this verdict** — was skipped and
phase 2 was built anyway on the operator's instruction. This section is that step, arriving late,
and it changes nothing that shipped: it says the evidence holds, names what was never endorsed, and
leaves the removal cost where the ADR left it (*"deleting one module and one manifest line"*) in
case a later reading of these ladders does not.

### What this story got wrong, and how it aged

Kept rather than edited away, because the *Context* above is the record of what was believed on the
day it was written. Corrected here:

| the story says | true on 2026-08-28 |
|---|---|
| `entity-runtime` is `0.2.1` | `0.13.0` (`ddee747`), seventeen releases later |
| our eight ladders, 64 edges | **twelve** ladders here, eleven of them expressed there, **77** edges compared |
| their fixture is pinned at `79b641c` | `3de6e07`; the eleven shared files still hash to what their `PIN.md` records, so their copy of us is current |
| *"no dependency exists in either direction"* | false since 0.13.0 — five crates, one tag, one way |
| `docs/plan/gap-register.md:77` is the stale `parent()` row | that row is now at `:102`, was already closed by code, and its citations were re-read and corrected on 2026-08-28. A2 is shut: `crates/aep-domain/src/artifact.rs:529` |
| the schema half "is pinned against nothing at all" | still true, and it is why the schema half gets no verdict |

## Acceptance

- A verdict on `entity-runtime/docs/design/engineering-protocols-adoption-v0.1.md` — accepted,
  accepted in part, or refused — with the reason, recorded here or on a plan page. A refusal closes
  this story exactly as an acceptance does.
- The verb question is answered: whether operations are wanted at all, and if so whose names.
- The `story:journal-backed-store` sequencing is decided before either side starts building.
- `docs/plan/gap-register.md:77` gets its *closed by code* line, or the row is corrected to say what
  of A1/A2/A3 is still open.

## Out of Scope

Taking the dependency. `atlas/architecture/adr/0002-the-entity-runtime-dependency-arrow.md` records
the arrow as accepted — this repository takes `entity-core`, `entity-runtime` takes nothing back at
any version, ever — and its own step 4 says the `Cargo.toml` line does not enter until after this
verdict. Nothing has been built on either side that assumes a yes.

Also out of scope: changing any lifecycle document, opening the status vocabulary, and modelling
`required_sections`. Each is its own story.

## Open Questions

**Do we want operations at all, and if so whose names?** Decides: protocol owner. Default if nobody
answers: **the mapping is accepted for states and edges and explicitly *not* for verbs** — the eleven
names above stay theirs and unendorsed, and any phase that puts a verb on our wire needs its own
decision, because `move --to implemented` and `execute --operation implement` are different published
surfaces.

**Which seam is cut first, `story:journal-backed-store` or a kernel-decided move?** Decides: store
owner. Default if nobody answers: **the journal first**, because it is ours end to end and closes a
deviation we already recorded; a kernel-decided move then lands behind an existing envelope rather
than beside one.

**Does the schema half need a verdict now, or only when `required_sections` is modelled?** Decides:
protocol owner. Default if nobody answers: **only then** — an unmodelled `body: json` claims nothing
about our kinds, so there is nothing yet to be wrong about.

**Does a mistyped relation get a way out?** Decides: store owner. Default if nobody answers: **no,
and the reason is written down** — invariant 16 wins, and the register row this story asks for says
so where an author will read it rather than leaving it to be discovered by making the mistake.
