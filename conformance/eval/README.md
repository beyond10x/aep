# The eval-case corpus

One directory per case. A case is three things and no others:

| Part | File | What it is |
|---|---|---|
| the task statement | `task:` in `case.yaml` | what an agent is asked to do, in the words it would be asked in |
| the expectations | `expectations.trace.yaml` | a `trace-spec/1` document — what the run must have looked like |
| the transcript | `transcript.jsonl` | a committed run, replayed through the checker in the gate |

`crates/edge/aep-cli/tests/eval_corpus.rs` enumerates this directory and replays every case. Nothing
registers a case anywhere: a new directory with a `case.yaml` in it **is** a new case, and one whose
transcript stops satisfying its own document turns `task check` red naming the expectation that
stopped holding.

## Why the corpus lives here

`conformance/` is where this repository keeps the material that decides whether something conforms —
`fixtures/`, `scenarios/` and `expected/` for a backend, `trace/` for the three shipped trace
specifications. An eval case is the same class of object one domain further out: a fixture, a
specification and the verdict they jointly produce. It is not a protocol document kind — `protocol
validate` does not read this tree — so it does not belong beside `workflows/` or `drivers/`, and it
is not generated, so it does not belong under `generated/` or `suites/`.

## The verdict a case declares

`case.yaml` carries `verdict:`, and it is the whole point of the corpus rather than an afterthought.

* **`held`** — the check must exit 0 with no gap and no unknown. Every row decided, every row passed.
* **`violated`** — the check must report a gap on **exactly** the expectation ids listed under
  `violated:`, and nothing else may gap and nothing may be undecidable.

A violation case is not a broken case. It is the control: a corpus of honest runs measures whether
the documents can be satisfied, and says nothing at all about whether they can be *failed*. The test
refuses a violation case whose gapping set differs from what it declared — in either direction — so
repairing the transcript to make the row pass is as red as breaking a row that was passing. That is
what makes the bound discriminating rather than decorative.

Both development cases are judged by **one** document. `development-tests-after-the-code` points its
`expectations:` at its sibling's file rather than carrying a copy, so the two runs meet literally the
same rows and the only difference between them is what the agent did.

## The transcripts are synthesized, and that is stated rather than implied

Every transcript here is written by hand against the `metaharness.event/1` event stream — the same
construction, and for the same reason, as `crates/observe/trace-spec/tests/fixtures/metaharness-driven-*.jsonl`,
whose module documentation makes the argument at length. They are **structurally faithful and not
observed**: a number in one of these files is a number this corpus chose, so a failing assertion here
is a change in the checker or in a document, never a finding about a harness.

Two consequences worth writing down:

* **The seam format, not a vendor's.** `metaharness.event/1` is the one reader that serves every
  harness metaharness drives, so a case written against it is checkable for a Claude Code arm and a
  Codex arm alike. A case recorded in `claude-code/stream-json` would be a case about one vendor.
  `read_any` detects the format from the transcript's own first line, so a real recorded run replaces
  a synthesized fixture **in place** — same directory, same document, no test change.
* **No case here is evidence that an arm behaves this way.** The corpus states what a case *is*, and
  the live run states what happened. Until a paid run is committed beside one of these, a green
  corpus means the bounds are satisfiable and self-consistent, and means nothing about any model.

## What counts as a write, and what a Codex run cannot witness

A case that asserts *bytes reached this file* scopes to a **set** of tool names, not one:

```yaml
expect:
  order:
    first:  {tools: [Edit, NotebookEdit, Write], args: {file_path: {glob: "*/tests/*"}}}
    before: {tools: [Edit, NotebookEdit, Write], args: {file_path: {glob: "*/src/*"}}}
```

The list is `crates/edge/aep-cli/src/drive.rs`'s — exactly the three verbs it renders to the
`repository.write` capability — so this repository's own driver stays the authority on what a write
is and a fourth verb is added in one place.

**This is a widening of what can witness a claim, never of the claim.** The first live pilot bought
the rule: a run asked to write a test before the code did precisely that with `Edit`, on files that
already existed, and a selector naming `Write` alone reported `never_occurred` — the checker
shrugging at work that had visibly happened. `Edit` before `Edit` is the same ordering assertion
`Write` before `Write` was. Dropping the tool scope entirely and matching `file_path` alone would
have been a real weakening, because `Read` carries a `file_path` too and *read the test first* is not
*wrote the test first*. Both directions are mutation-tested in
`crates/observe/trace-spec/tests/write_selectors.rs`.

**A forbidding row is not the same set as a witnessing row.** `no-artifact-file-was-rewritten-whole`
names `[NotebookEdit, Write]` and deliberately leaves `Edit` out, because the store's rule denies a
whole-file replacement outright and permits a targeted body edit — the skill's second guardrail asks
for exactly that. That half is the step map's `scope:` (`write: denied` or `partial-only` over
`.engineering/planning/**`), which both arms are held to. Whether a given `Edit` crossed the `---`
fence is a judgement about `old_string`, not about a path, and no matcher here expresses it; that
half is `store_integrity` in the driver, and `protocol plan artifact validate` afterwards. Copy the
*reasoning*, not the list, when you write a new row.

**Codex observability, stated rather than discovered.** On the seam a Codex write travels as
`apply_patch`, whose input is a patch envelope under `command` — the path lives inside the patch text
(`*** Add File: <path>`), and there is no `file_path` argument at all. Codex shell calls arrive as
`exec` with an empty `input`. So a path-scoped row answers `unk` against a Codex transcript, which is
the honest verdict — nobody found out — and not a failed run. It is deliberately **not** papered over
by adding `apply_patch` to the set: the tool would match, the `file_path` matcher would not, and the
row would still not decide while now *looking* cross-harness. A selector cannot say *`file_path` here
or `command` there* — design decision D2 keeps boolean combinators out of the matcher language — so
closing this is a Codex-side case with its own rows, not a wider set.

## A recorded id is not renamed with the plugin

`agentplugins@a2077d2` renamed the plugins by product and verb — `aep-planning` → `aep-plan`, `adp`
→ `aep-drive`, `ess-schema` → `ess-specify`. A case's `task:` and its expectation rows were **not**
rewritten wherever the run they describe was recorded before that: `transcript.jsonl` is evidence,
the ids in it are the ids that session was actually offered, and a row renamed away from them stops
matching and reports a gap — which reads as a finding about the run and is a finding about nothing.
Each such line carries `# recorded-under-this-name` and the document says why at its head. A case
written from here on names the current plugin; a case whose transcript is re-recorded renames with
it, in the same commit as the recording.

## Adding a case

```console
$ mkdir conformance/eval/<slug>
$ $EDITOR conformance/eval/<slug>/case.yaml
$ $EDITOR conformance/eval/<slug>/expectations.trace.yaml
$ $EDITOR conformance/eval/<slug>/transcript.jsonl
$ cargo test -p aep-cli --test eval_corpus
```

`case.yaml` must declare a `workflow:` that a document under `workflows/` declares, and `states:` that
that workflow declares — both refused by name, so a case cannot drift away from the machine it claims
to be about. `integrations/workflow-coverage.yaml` is the map of which plugin surface teaches those
states; a case whose states are all in a gap there is a case measuring what nothing teaches, which is
a legitimate and deliberately available thing to write.
