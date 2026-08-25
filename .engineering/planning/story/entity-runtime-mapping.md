---
format: aep.planning-md/1
id: story:entity-runtime-mapping
kind: story
status: draft
title: Another repository has already expressed our eight ladders as data, and asks for a verdict
summary: entity-runtime carries artifacts/lifecycles/*.yaml as eight entity definitions with an equivalence test pinned at 79b641c. Phase 0 of its adoption design asks whether the reading is faithful, and whether entity-core becomes an ordinary dependency here.
owner: protocol
tags:
- adoption
- lifecycle
- protocol
revision: 3
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
