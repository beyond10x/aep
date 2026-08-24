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
`docs/plan/document-authoring-brief.md` — which is 33 files:

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

The worked case is artifact status, and its two rows read:

- **A status ladder under `artifacts/lifecycles/`** — open. You declare which rungs a kind of yours
  may hold, in a document, in your own tree.
- **Artifact status values a lifecycle document may name** — closed. Every rung you name has to be a
  variant of `ArtifactStatus`, and there is no `statuses:` key anywhere under `protocols/` to add one
  from.

The store layer is open and the value layer is not, and you need both sentences to plan a migration.
One averaged verdict would have given you neither.

The rows **A relation document under `artifacts/relations/`** and **Relation names a relations
document may use** are the same shape, and that pair is the one still owed an explanation.

## The table

| Declaration | Invited at | Verdict | Decided by | Guarantee | Reason for adopters at | Follow-up |
|---|---|---|---|---|---|---|
| A protocol document's `capabilities:` block | `website/docs/reference/vocabulary.md:15` — "Nothing else may appear in any" | open | `protocols/aep/1.yaml:capabilities` | — | — | — |
| A protocol document's `approval_floor:` block | `docs/plan/document-authoring-brief.md:26` — "may never appear in a" | open | `protocols/aep/1.yaml:approval_floor` | — | — | — |
| A protocol document's `evidence_kinds:` block | `website/docs/reference/vocabulary.md:39` — "adds two more, each minted by the verb that ran the check" | open | `protocols/aep/1.yaml:evidence_kinds` | — | — | — |
| A protocol document's `verifiers:` block | `website/docs/reference/vocabulary.md:57` — "the producers of the two evidence kinds it" | open | `protocols/aep/1.yaml:verifiers` | — | — | — |
| A protocol document's `artifact_kinds:` block | `docs/plan/document-authoring-brief.md:47` — "is satisfied by any design subkind" | open | `protocols/aep/1.yaml:artifact_kinds` | — | — | — |
| A protocol document's `phases:` block | `website/docs/reference/vocabulary.md:78` — "decomposition verification-setup adversarial-verification" | open | `protocols/aep/1.yaml:phases` | — | — | — |
| A protocol document's `observables:` block | `website/docs/reference/vocabulary.md:84` — "A predicate may only read these" | open | `protocols/aep/1.yaml:observables` | — | — | — |
| A protocol document's `scales:` block | `website/docs/reference/vocabulary.md:100` — "on non-numeric values" | open | `protocols/aep/1.yaml:scales` | — | — | — |
| An artifact kind document under `artifacts/kinds/` | `website/docs/reference/documents.md:210` — "one per kind, beside" | open | `artifacts/kinds/design.yaml:required_sections` | — | — | — |
| A status ladder under `artifacts/lifecycles/` | `docs/guide/adopting.md:94` — "what statuses each artifact kind may hold" | open | `artifacts/lifecycles/story.yaml:transitions` | — | — | — |
| A relation document under `artifacts/relations/` | `website/docs/reference/documents.md:211` — "`artifacts/relations/` and" | open | `artifacts/relations/relations.yaml:relations` | — | — | — |
| An artifact template under `artifacts/templates/` | `website/docs/reference/documents.md:211` — "and `artifacts/templates/`" | open | `artifacts/templates/story.md:1` | — | — | — |
| Artifact status values a lifecycle document may name | `website/docs/reference/vocabulary.md:72` — "draft proposed in_review approved accepted rejected active implemented superseded" | closed | `crates/aep-domain/src/artifact.rs:707` | a status name means the same rung to every tool that reads the artifact graph, so a lifecycle written elsewhere can be compared with one written here | website/docs/status/limitations.md | — |
| Relation names a relations document may use | `website/docs/reference/documents.md:224` — "artifact must have a successor declaring" | closed | `crates/aep-domain/src/artifact.rs:987` | none | none | story:ova-relation-vocabulary |
| Capability value names the engine accepts | `docs/plan/document-authoring-brief.md:13` — "nothing else may be mentioned in any" | closed | `crates/aep-domain/src/capability.rs:144` | a capability name resolves to the same authorisation decision in every harness, which is what lets a profile be read by one and enforced by another | website/docs/reference/vocabulary.md#capabilities | — |
| Evidence kind names the engine accepts | `docs/plan/document-authoring-brief.md:29` — "**Evidence kinds**:" | closed | `crates/aep-domain/src/evidence.rs:1181` | an evidence kind carries fixed semantics and a fixed set of verifiers that may establish it, so a requirement for one cannot be satisfied by a record that means something else | website/docs/reference/vocabulary.md#evidence-kinds | — |
| Predicate operators in mapping form | `website/docs/reference/vocabulary.md:172` — "Operators in mapping form" | closed | `crates/aep-domain/src/predicate.rs:132` | none | none | story:ova-predicate-operator-vocabulary |
| Test suite names in a tests fact path | `website/docs/reference/vocabulary.md:111` — "unit integration contract property regression mutation fuzz differential e2e smoke" | open | `crates/aep-domain/src/evidence.rs:60` | — | — | — |

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

Round taken at commit `14a87ee`, against the corpus rule above.

1. The corpus: `docs/guide/*.md`, `website/docs/**/*.md` and
   `docs/plan/document-authoring-brief.md`.
2. The derivation: `bash .engineering/checks/scan-declarations.sh`, run from the repository root. It
   reads the tree and nothing else, and two runs against an unchanged tree are byte-identical.
3. The reading pass over the corpus, which produced 6 rows the derivation cannot reach — every one of
   them a value space fixed in the engine, plus the suite names, which turned out to be open.
4. The verdicts, each attached to a path in this tree. No cell in `Decided by` is prose: a verdict
   that cannot be attached to a file was not entered.

The next round is a diff, not a rewrite. Re-run `bash .engineering/checks/run.sh` and it says which
citations no longer resolve, which candidates are new, and which closed rows are still unexplained.

Nine deliberate mutations are applied to a copy of the tree on every run, to show the suite
discriminates rather than merely agrees:

- deleting the table row that carries a derived candidate, which must orphan that candidate;
- downgrading a settled row's guarantee to `none`, which must demand a follow-up;
- pointing a follow-up at an id that is not in the planning store;
- deleting a quoted fragment from the corpus file the row cites;
- inserting a line above a cited fragment, which leaves the row's line number pointing past it;
- renaming the heading a reason's anchor names, which leaves the link resolving to nothing;
- repointing a reason at this page, so that the row cites its own cell as the place you read why;
- repointing a verdict from the declaration that settles it to a use site that does not;
- repointing an `open` verdict onto the enum head, where a reader would read it as closed.

Each is applied alone, to a fresh copy, and a mutation that reddens nothing is itself a failure —
because a suite that cannot be made to fail is not evidence that anything passed.

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
