# Open vocabulary: what you may declare, and what is fixed in the engine

For every declaration this project's documentation invites you to write, one answer to one question:
**can I put my own value here?**

The answer is `open` when you can introduce a new value without modifying a file under `crates/` in
this repository — writing a document is enough. Everything else is `closed`, and a closed answer
means the value space is a Rust item you would have to patch and rebuild to extend.

A closed vocabulary is not a defect. `evidence_kinds` is closed on purpose: it is the seam whose
semantics are guaranteed, and a requirement for `test_result` has to mean the same thing in your tree
as in this one. What this table exists to stop is a *different* thing — a closed vocabulary nobody
wrote down as closed, which you discover by writing a tree and hitting a validation error the guide
gave you no reason to expect. Every closed row here therefore carries what the closure buys and where
you can read the argument for it; the rows that carry neither name the story that owes you one.

## The corpus

The table covers the declarations invited by the published documentation, and nothing else. An
internal document key no guide mentions is not an adopter-facing declaration, and collecting those
would turn this into a list of the repository's YAML keys.

The corpus is exactly what these three globs produce — `docs/guide/*.md`, `website/docs/**/*.md` and
`docs/plan/document-authoring-brief.md` — which is 34 files:

- `docs/guide/README.md`
- `docs/guide/adopting.md`
- `docs/guide/backend.md`
- `docs/guide/harness.md`
- `docs/guide/open-vocabulary.md`
- `docs/guide/specification.md`
- `docs/plan/document-authoring-brief.md`
- `website/docs/concepts/aep.md`
- `website/docs/concepts/design-principles.md`
- `website/docs/concepts/ess.md`
- `website/docs/concepts/evidence.md`
- `website/docs/concepts/lifecycles.md`
- `website/docs/concepts/overview.md`
- `website/docs/examples/governed-task.md`
- `website/docs/examples/specification-to-contracts.md`
- `website/docs/getting-started.md`
- `website/docs/guides/check-a-transcript.md`
- `website/docs/guides/check-infrastructure.md`
- `website/docs/guides/generate-artifacts.md`
- `website/docs/guides/govern-a-task.md`
- `website/docs/guides/integrate-a-harness.md`
- `website/docs/guides/synthesize.md`
- `website/docs/guides/track-change.md`
- `website/docs/guides/verify-conformance.md`
- `website/docs/guides/write-a-principle.md`
- `website/docs/guides/write-a-specification.md`
- `website/docs/index.md`
- `website/docs/reference/cli.md`
- `website/docs/reference/documents.md`
- `website/docs/reference/glossary.md`
- `website/docs/reference/vocabulary.md`
- `website/docs/status/limitations.md`
- `website/docs/status/roadmap.md`
- `website/docs/status/where-this-stands.md`

The list is checked against the globs on every run, in both directions. A guide added later turns
this page red rather than leaving it quietly out of date.

## One row per layer

Two declarations in this table are open in a document and closed in the value that document may
carry, and each of them gets **two rows** rather than one qualified verdict. A single averaged
verdict would be the sentence you believed right up until the validation error.

The worked case **was** artifact status, and it is now the worked case for something else: what
happens when a table like this one is acted on. Its two rows read:

- **A status ladder under `artifacts/lifecycles/`** — open. You declare which rungs a kind of yours
  may hold, in a document, in your own tree.
- **Artifact status values a lifecycle document may name** — **open since 2026-08-25**. It was
  closed: every rung had to be a variant of `ArtifactStatus`, and there was no `statuses:` key
  anywhere under `protocols/` to add one from. An adopter needed `correction-owed` — sent, known
  wrong, audience not yet told — and a ten-variant enum decided that their process could not say it.

  What the closure bought was *a status name means the same rung to every tool*. That guarantee is
  still bought, by the **ladder** instead of by the type: a status you name is accepted only if the
  lifecycle document for that kind declares it, so the vocabulary is open to authors and still
  closed to typos. `protocol artifact move --to correction-owed` refuses with *"its lifecycle
  declares draft, proposed, rejected, active, implemented, archived"* until the rung is in the
  document, and works the moment it is.

  Two things it deliberately does **not** buy. An invented rung is never `approved` and never
  `retired` — this repository cannot know what a rung it has never seen means, and reading an
  unknown name as *agreed and relied on* is the one mistake an open vocabulary must not make. And a
  descriptor document under `adp-`/`aop-domain` still takes named statuses only, because it has no
  ladder in scope to check an invented one against.

Both layers are open now. The pair is kept as two rows anyway, because the *reason* they are open
differs — one is a document you write, the other is a value that document may carry, and a single
averaged verdict would tell you neither.

The rows **A relation document under `artifacts/relations/`** and
**Relation names a relations document may use** are the same shape, and the pair was the one still
owed an explanation. It has one now, and the answer is **not** the same as artifact status:

- The store layer is open: you write `artifacts/relations/relations.yaml` in your own tree.
- The value layer stays **closed, deliberately**, and this is the guarantee. A relation name is
  something the engine *acts on*. `supersedes` gates a status — an artifact marked `superseded` must
  have a successor declaring it. `reviews` is mandatory on a `review-result`. Cycles are checked once
  per relation kind. None of that can move into a document, because a document can declare what an
  edge *means* and not what the engine *does* about it.

That is the difference from artifact status, where the guarantee — *a status name means the same
rung to every tool* — could move to the ladder, and did. There is nowhere for a relation's semantics
to go. An open relation vocabulary would let you write an edge that looks like it should mean
something and that no rule will ever read, which is a worse failure than being told `frobnicates` is
not a relation.

What was actually wrong here was smaller and is fixed: the type had **thirteen** relations and the
document **twelve**. `delivers` was in the binary and in no row, so the engine accepted an edge this
file said did not exist. It has a row now.

What remains genuinely advisory is the `source`/`target` pairings — the file says so in its own
header, and nothing in `crates/` reads them. That is a different gap from this one and is not closed
by this decision.

The reason is written where an adopter reads it — [`adopting.md`](adopting.md#relation-names) — and
not only in the cell below, and it says how to ask: a kind that is not on the list of thirteen is a
change to the engine's graph rules, so it arrives as a story in this repository's planning store
naming the rule the new edge should get. That is what moves this row from unsettled to settled.
`story:ova-relation-vocabulary` asked for a stated guarantee or a recorded decision to open; this is
the guarantee.

## Predicate operators, the other row that was owed one

`Predicate operators in mapping form` was the weaker of the two cases. Not only was no reason
written for adopters — no guarantee was claimed for the closure either, and unlike every other row
in this table there is no document key anywhere under `protocols/` an adopter could extend even in
their own tree.

The set stays closed, for the same class of reason as relations rather than for convenience. An
operator is the step the engine *performs*, and it performs it three-valued: `gte` on `risk: medium`
is decided by the `scales:` the protocol declares rather than by string ordering, and `exists` is
the one operator allowed to read an unobserved fact without collapsing it to False. A
document-declared operator would arrive with a name and no answer to either question, and *Unknown
is not False* is the rule the evaluator is built to keep. The ten are `eq ne lt lte gt gte any_of
none_of exists truthy`, published with that reason in
[`adopting.md`](adopting.md#predicate-operators).

Worth stating in the form an adopter meets it: a substring match, a set intersection and a regular
expression are **not** expressible, and that is a boundary rather than an omission. Needing one is a
change to the evaluator and a story in this store, not a key you can add to your own tree.

## The table

| Declaration | Invited at | Verdict | Decided by | Guarantee | Reason for adopters at | Follow-up |
|---|---|---|---|---|---|---|
| A protocol document's `capabilities:` block | `website/docs/reference/vocabulary.md:15` — "Nothing else may appear in any" | open | `protocols/aep/1.yaml:capabilities` | — | — | — |
| A protocol document's `approval_floor:` block | `docs/plan/document-authoring-brief.md:26` — "may never appear in a" | open | `protocols/aep/1.yaml:approval_floor` | — | — | — |
| A protocol document's `evidence_kinds:` block | `website/docs/reference/vocabulary.md:39` — "adds two more, each minted by the verb that ran the check" | open | `protocols/aep/1.yaml:evidence_kinds` | — | — | — |
| A protocol document's `verifiers:` block | `website/docs/reference/vocabulary.md:57` — "the producers of the two evidence kinds it" | open | `protocols/aep/1.yaml:verifiers` | — | — | — |
| A protocol document's `artifact_kinds:` block | `docs/plan/document-authoring-brief.md:47` — "is satisfied by any design subkind" | open | `protocols/aep/1.yaml:artifact_kinds` | — | — | — |
| A protocol document's `phases:` block | `website/docs/reference/vocabulary.md:92` — "decomposition verification-setup adversarial-verification" | open | `protocols/aep/1.yaml:phases` | — | — | — |
| A protocol document's `observables:` block | `website/docs/reference/vocabulary.md:98` — "A predicate may only read these" | open | `protocols/aep/1.yaml:observables` | — | — | — |
| A protocol document's `scales:` block | `website/docs/reference/vocabulary.md:114` — "on non-numeric values" | open | `protocols/aep/1.yaml:scales` | — | — | — |
| An artifact kind document under `artifacts/kinds/` | `website/docs/reference/documents.md:230` — "one per kind, beside" | open | `artifacts/kinds/design.yaml:required_sections` | — | — | — |
| A status ladder under `artifacts/lifecycles/` | `docs/guide/adopting.md:94` — "what statuses each artifact kind may hold" | open | `artifacts/lifecycles/story.yaml:transitions` | — | — | — |
| A relation document under `artifacts/relations/` | `website/docs/reference/documents.md:231` — "`artifacts/relations/` and" | open | `artifacts/relations/relations.yaml:relations` | — | — | — |
| An artifact template under `artifacts/templates/` | `website/docs/reference/documents.md:231` — "and `artifacts/templates/`" | open | `artifacts/templates/story.md:1` | — | — | — |
| Artifact status values a lifecycle document may name | `website/docs/reference/vocabulary.md:72` — "draft proposed in_review approved accepted rejected active implemented superseded" | open | `crates/aep-domain/src/artifact.rs:741` — the `Other(String)` variant, gated by the kind's ladder at `crates/protocol-cli/src/planning.rs:parse_status_in` | — | — | — |
| Relation names a relations document may use | `website/docs/reference/documents.md:244` — "artifact must have a successor declaring" | closed | `crates/aep-domain/src/artifact.rs:1107` | a relation name is something the engine *acts on*, not only records: `supersedes` gates a status, `reviews` is mandatory on a review-result, and cycles are checked per relation kind. An invented name would be an edge that looks like it should mean something and that no rule will ever read | docs/guide/adopting.md#relation-names | — |
| Capability value names the engine accepts | `docs/plan/document-authoring-brief.md:13` — "nothing else may be mentioned in any" | closed | `crates/aep-domain/src/capability.rs:144` | a capability name resolves to the same authorisation decision in every harness, which is what lets a profile be read by one and enforced by another | website/docs/reference/vocabulary.md#capabilities | — |
| Evidence kind names the engine accepts | `docs/plan/document-authoring-brief.md:29` — "**Evidence kinds**:" | closed | `crates/aep-domain/src/evidence.rs:1189` | an evidence kind carries fixed semantics and a fixed set of verifiers that may establish it, so a requirement for one cannot be satisfied by a record that means something else | website/docs/reference/vocabulary.md#evidence-kinds | — |
| Predicate operators in mapping form | `website/docs/reference/vocabulary.md:186` — "Operators in mapping form" | closed | `crates/aep-domain/src/predicate.rs:132` | an operator is the step the engine *performs*, and it performs it three-valued: `gte` is decided by the `scales:` the protocol declares rather than by string ordering, and `exists` is the one operator that may read an unobserved fact without collapsing it to False. A document-declared operator would arrive with a name and no answer to either question | docs/guide/adopting.md#predicate-operators | — |
| Test suite names in a tests fact path | `website/docs/reference/vocabulary.md:125` — "unit integration contract property regression mutation fuzz differential e2e smoke" | open | `crates/aep-domain/src/evidence.rs:60` | — | — | — |

An `open` row's last three cells are an em dash, always — never blank. A blank cell reads as *not
applicable* to one person and *not filled in yet* to the next, and both of them stop asking.

## What the derivation cannot find

Twelve of these rows were derived: a script walks `protocols/*/*.yaml` for top-level vocabulary keys
and `artifacts/` for adopter-writable document families, and every candidate it emits must have a row
here or the check goes red. That half is mechanical and it stays true as the tree changes.

The other six were found by reading, and no script could have found them. **The derivation cannot discover a closed surface**,
because a closed surface is precisely one with *no document key* to find: `ArtifactStatus` is closed
exactly because nothing under `protocols/` declares `statuses:`. If
you take the completeness check for proof that this table is complete, it has misled you. What holds
the read rows honest is the quoted fragment in each `Invited at` cell — it is resolved against the
file it cites on every run, so a guide that stops inviting a declaration turns its row red instead of
leaving a row here about a page nobody writes any more.

## How this round was produced

Round taken at commit `23a213f`, against the corpus rule above.

1. The corpus: `docs/guide/*.md`, `website/docs/**/*.md` and
   `docs/plan/document-authoring-brief.md`.
2. The derivation: `bash .engineering/checks/scan-declarations.sh`, run from the repository root. It
   reads the tree and nothing else, and two runs against an unchanged tree are byte-identical.
3. The reading pass over the corpus, which produced 6 rows the derivation cannot reach — every one of
   them a value space fixed in the engine, plus the suite names, which turned out to be open.
4. The verdicts, each attached to a path in this tree. No cell in `Decided by` is prose: a verdict
   that cannot be attached to a file was not entered.

This round was a diff and not a rewrite, which is the property the list above exists to buy. What it
found, in the order the suite reported it:

- one corpus file the list had not caught up with, `website/docs/concepts/lifecycles.md`, and the
  stated count that goes with it;
- five citations into the vocabulary reference that had drifted by fourteen lines and two into the
  artifact type that had drifted by twelve — every one of them still resolving *somewhere* in its
  file, which is why the line is checked and not only the fragment;
- the two rows that carried no reason an adopter could read. Both are settled above, and neither by
  opening the vocabulary: in both cases the guarantee was there to be written down and had not been.

The reading pass over the corpus produced **no new rows** this round, and refusing the two candidates
it did consider is the part worth recording:

- *the spellings a predicate may read* — closed, and already argued in the vocabulary reference, but
  there is no single item to attach the verdict to, because four separate types project facts. A
  verdict with no path is one this table does not enter.
- *the producer of an evidence record* — what an adopter writes there is a verifier class, and
  verifier classes are declared in a protocol document. That is the open row this table already
  carries, not a second one.

The project directory is the third instance the adopter's report named, and it is deliberately **not**
a row. It is a path rather than a vocabulary a document declares; it is set by AEP_PROJECT_DIR, read
once per process, and covered by `crates/aep-engine/tests/project_directory_env.rs`; and it stopped
being the compile-time constant the report found. A row would have to call it open on the strength
of an environment variable, and every other `Decided by` cell in this table names the declaration
that decides the verdict — for an open row, the variant that admits a value of the adopter's own.
There is no such line here, so the finding is recorded in prose instead of asserted in a cell that
would mean something different from every cell beside it.

The next round is a diff, not a rewrite. Re-run `bash .engineering/checks/run.sh` and it says which
citations no longer resolve, which candidates are new, and which closed rows are still unexplained.

Nine deliberate mutations are applied to a copy of the tree on every run, to show the suite
discriminates rather than merely agrees:

- deleting the table row that carries a derived candidate, which must orphan that candidate;
- downgrading a settled row's guarantee to `none`, which must demand a follow-up;
- pointing a follow-up at an id that is not in the planning store, on a row first downgraded to
  need one;
- deleting a quoted fragment from the corpus file the row cites;
- inserting a line above a cited fragment, which leaves the row's line number pointing past it;
- renaming the heading a reason's anchor names, which leaves the link resolving to nothing;
- repointing a reason at this page, so that the row cites its own cell as the place you read why;
  this one is expected to redden *two* checks — the row loses its reason and is left with no
  follow-up — and both are named rather than tolerated;
- repointing a verdict from the declaration that settles it to a use site that does not;
- repointing an `open` verdict onto the enum head, where a reader would read it as closed.

Each is applied alone, to a fresh copy, and a mutation that reddens nothing is itself a failure —
because a suite that cannot be made to fail is not evidence that anything passed. Two of them are
applied to state this round had to **construct**: once every closed row carries a reason, there is
no unsettled row left lying around to repoint, and a mutation that can no longer be applied is a
proof that quietly stopped running. Both now downgrade a settled row first and mutate that.

The last five are here because they were once invisible. The suite ran green under every one of
them, which is worth saying plainly: the first four showed it catches what it was written to catch,
and said nothing about the rest. A citation whose line number is off by one still reads as a
citation, and it is the ordinary consequence of somebody adding a paragraph to a page this table
quotes.

Two of them found a live row rather than a hypothetical one. The relation row settled its verdict at
a line that merely *called* the parser, and the suite row pointed at `pub enum TestSuite {` — a
ten-variant enum a reader would follow the link and conclude was closed, when the line that makes it
open is the `Named(String)` variant twenty-two lines below. Both cells now name the line that
actually decides.
