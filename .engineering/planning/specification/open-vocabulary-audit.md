---
format: aep.planning-md/1
id: specification:open-vocabulary-audit
kind: specification
status: approved
title: Every adopter-facing declaration, in one table a check can decide
summary: 'Required behaviour of the open/closed audit: a declared corpus, one table with a verdict, a guarantee and a stated-where per closed row, a follow-up artifact for every unjustified closure, and a derivation command that makes the next round a diff.'
owner: protocol
tags:
- adoption
- protocol
relations:
- specifies: story:open-vocabulary-audit
- derived_from: task:w4-2-open-vocabulary-audit
revision: 4
---
# Specification: every adopter-facing declaration, in one table a check can decide

One audit document under `docs/`, and a check suite that decides it. The table answers *"can I put
my own value here?"* for each declaration the published documentation invites, and every cell that
carries a verdict, a guarantee, a citation or a follow-up is resolved against the tree by a check —
not read for plausibility by a person.

The thing this specification is trying to make impossible is the failure the story was opened for:
a closed vocabulary that nobody wrote down as closed. So the checks are built to go **red on drift**
in both directions — a guide that stops inviting a declaration, and a closed row whose justification
was never written.

## Context

`story:open-vocabulary-audit` (under `epic:adopter-feedback-round-1`) answers item B of an early
adopter's round-1 review: *things the docs invite an adopter to declare keep turning out to be fixed
in the engine*. Three instances, all found by writing a tree rather than by reading the guide.

The tree already makes the open/closed distinction mechanical, which is what lets this be checked
rather than argued:

| | Where the vocabulary lives | Consequence for an adopter |
|---|---|---|
| Open | a key in a protocol document — `protocols/aep/1.yaml` declares `capabilities:` (l.14), `evidence_kinds:` (l.41), `verifiers:` (l.57), `artifact_kinds:` (l.70), `phases:` (l.99), `observables:` (l.111), `scales:` (l.144), and `protocols/adp/1.yaml` extends five of them (l.11-40) | writes an extension document |
| Closed | a Rust item — `crates/govern/aep-domain/src/artifact.rs:707` `pub enum ArtifactStatus`, and no `statuses:` key exists in any file under `protocols/` | must patch and rebuild this repository |

That is B1 stated as a fact about two paths. The same test separates the other candidates:
`artifacts/lifecycles/*.yaml` lets an adopter declare a kind's ladder in a document, but every rung
it names must be a variant of that closed enum — so the store layer is open and the value layer is
not, and a single verdict for "artifact status" would be wrong in one of the two directions.

The story is equally clear that the output is **not** "open everything": `evidence_kinds` being
closed is *correct*, because it is the seam whose semantics are guaranteed. So a closed verdict is
not a defect. A closed verdict with nothing behind it is.

`docs/guide/README.md:23-28` is where an adopter is routed into the guides; `website/docs/` is the
published site built from `website/sidebars.ts`. Between them they are the corpus R1 fixes.

Task `W4-2` drives this under `drivers/development/checks.yaml`, whose verifier is
`bash .engineering/checks/run.sh` — the file R14 specifies, red in `establish_verifiers` and green
at the end.

## Requirements

### Deliverables

| Path | What it is |
|---|---|
| `docs/guide/open-vocabulary.md` | the audit: the corpus, the table, and how it was produced |
| `docs/guide/README.md` | one added row in its *Which guide* table, pointing at the audit |
| `.engineering/checks/scan-declarations.sh` | the derivation: emits the candidate surfaces the tree declares |
| `.engineering/checks/run.sh` | the runner — every check, one row each, honest exit code |
| `.engineering/checks/check-*.sh` | one check per decomposed unit, named for the unit it decides |

Names are this specification's. A later state may move one only by recording why.

**R0.** The audit lives at `docs/guide/open-vocabulary.md` and `docs/guide/README.md` links to it.
This resolves the story's open question by its own stated default — *the guide*, because the reader
is an adopter deciding what they may declare. `website/` is outside the task's declared surface, so
the published site does not carry it in this run; that consequence is stated in Out of Scope, not
left to be discovered.

### The corpus

**R1.** The audit names its corpus explicitly, as a list of paths, and the corpus is exactly:

- `docs/guide/*.md` — 5 files, the adopter's guide;
- `website/docs/**/*.md` — 26 files, the published site;
- `docs/plan/document-authoring-brief.md` — 1 file, which `docs/guide/README.md:30-32` sends the
  adopter to for "every capability, evidence kind, fact path and predicate operator".

32 files. The count is in the audit and a check asserts the list in the audit equals the set the
glob produces — so a guide added later makes the audit red rather than silently out of date.

**R2.** Every path the audit names as corpus exists. A corpus entry that does not resolve is a
failed check, not a stale line.

### The table

**R3.** The audit contains exactly one markdown table whose header row is, in this order:

| Declaration | Invited at | Verdict | Decided by | Guarantee | Reason for adopters at | Follow-up |

Column names are the specification's, because the checks parse by header.

**R4.** `Verdict` holds exactly one of two values: `open` or `closed`. There is no third value and
no hedge. **Open** means: *an adopter can introduce a new value without modifying a file under
`crates/` in this repository.* Everything else is `closed`.

**R5.** A declaration that is open at one layer and closed at another gets **two rows**, one per
layer, each with its own verdict — never one row with a qualified verdict. The artifact-status case
in Context is the worked example, and the rule exists because a single averaged verdict is exactly
the sentence an adopter would have believed.

**R6.** `Invited at` holds a corpus path with a line number and a verbatim quoted fragment from that
file. Checks assert: the path is in the R1 corpus, **and** the quoted fragment occurs in that file.
The fragment is the drift detector — a guide that stops inviting the declaration turns the row red,
which is the signal the next round needs.

**R7.** `Decided by` holds a path in this repository that settles the verdict: a document key for an
open row (`protocols/aep/1.yaml:capabilities`), a `file:line` for a closed one
(`crates/govern/aep-domain/src/artifact.rs:707`). Checks assert the file part exists, and for a `file:line`
form that the file has at least that many lines. An `open` row is therefore as falsifiable as a
closed one; a verdict with no path behind it cannot be entered.

**R8.** For an `open` row, `Guarantee`, `Reason for adopters at` and `Follow-up` each hold `—`.
Checks assert this. A blank cell must never be readable as either "not applicable" or "not filled
in yet".

**R9.** For a `closed` row:

- `Guarantee` states the semantics the closure buys, or the literal `none`;
- `Reason for adopters at` holds a corpus path (optionally with an anchor) where that reason is
  written for adopters, or the literal `none`;
- a closed row is **settled** when `Guarantee` is not `none` **and** `Reason for adopters at`
  resolves to a corpus file. Otherwise it is **unsettled**.

Checks assert every `Reason for adopters at` value that is not `none` is a path in the R1 corpus and
exists. This is the story's second acceptance bullet made decidable: the reason has to be somewhere
an adopter reads, not only in the audit's own cell.

**R10.** Every **unsettled** closed row names a planning artifact id in `Follow-up` — a `story:` or
an `architecture-decision-record:` recording that it stays closed. A check asserts each such id
resolves in `protocol artifact list --format json`. A settled row holds `—`.

This is the story's third bullet: *a closed vocabulary with no stated reason does not survive the
audit unremarked*. The follow-up artifacts are created in this run, in the planning store; the
work they name is not done in this run.

**R11.** The table has at least one row with `open` and at least one with `closed`, and a row count
of at least the candidate count R12 emits. The floor exists because every quantified requirement
above is true of the empty table.

### The derivation, and what it can and cannot find

**R12.** `.engineering/checks/scan-declarations.sh` emits, one per line and sorted, the declaration
surfaces the tree declares in documents: every top-level vocabulary key under `protocols/*/*.yaml`,
and every adopter-writable document family under `artifacts/` (`kinds/`, `lifecycles/`,
`relations/`, `templates/`). It reads the tree only, makes no network call, and two runs against an
unchanged tree produce byte-identical output.

**R13.** Two checks close the loop between the scan and the table:

| | Rule | What it catches |
|---|---|---|
| Completeness | every candidate `scan-declarations.sh` emits has a row in the table | a document-declared vocabulary the audit forgot |
| Provenance | every row the scan does **not** emit satisfies R6 — a corpus citation that resolves | a row invented rather than found |

The scan cannot discover a *closed* surface, because a closed surface is precisely one with no
document key to find. Those rows are produced by reading the corpus, and R6 is what holds them
honest. The audit says this limit in its own words; a reader who takes the completeness check for
proof of completeness has been misled by it.

### Repeatability

**R14.** The audit carries a section stating how it was produced: the corpus rule (R1), the command
`bash .engineering/checks/scan-declarations.sh`, the commit the round was taken at, and the reading
pass that produced the non-scanned rows. A check asserts the section exists, names the scan script
by path, and that the script runs and exits 0.

**R15.** The next round is a diff: re-running the scan and re-running the checks tells the reader
which rows moved, which citations no longer resolve, and which candidates are new. Nothing in the
audit requires rewriting to run it again.

### The runner

**R16.** `bash .engineering/checks/run.sh`, run from the repository root, runs every check, prints
one row per check naming the unit it decides, prints its table **on every path including failure**,
and exits non-zero while any check fails. It makes no network call and reaches no API. The model is
`integrations/claude-code/eval/checks/run-checks.sh`.

**R17.** A check whose script is missing is a **failed row**, never a skipped one, and never a
silent success.

**R18.** No check reads this specification, the task document, or any planning artifact body. Checks
read the audit, the corpus, the tree, and `protocol artifact list` output. A check that greps this
file asserts that a sentence is still written here, which is the failure mode the story exists to
remove.

## Constraints

- **Writes are confined to `docs/` and `.engineering/`.** Nothing under `crates/`, `website/`,
  `integrations/`, `drivers/`, or the workspace `Cargo.toml` is modified — the task's constraint,
  restated as a checkable one (Acceptance criterion 7).
- **`integrations/claude-code/eval/checks/run-checks.sh` is read as a model and not edited.**
- **No network, no API, no money.** Every check is hermetic; the suite runs offline.
- **Never `/tmp`** for scratch. `${TMPDIR:-$HOME/.cache/claude-tmp}`, as the model runner does.
- **Three programs only**: `bash`, `git`, `protocol` — the set `drivers/development/checks.yaml`
  already requires on `PATH`. No check introduces a fourth dependency (no `yq`, no `jq` unless it is
  already required by the model runner).
- **The audit is a document, not a generated file.** `scan-declarations.sh` emits candidates; it does
  not write `docs/guide/open-vocabulary.md`. A generated table would make R6's citations unverifiable
  by construction.

## Out of Scope

- **Opening any vocabulary.** The story says so: each one that should open is its own story with its
  own migration question. This run produces the verdict, the list, and the follow-up artifacts.
- **Doing the follow-up work.** R10 requires the artifacts to exist and resolve. Their bodies state
  the question; nobody answers it here.
- **The published site.** `website/docs/` is outside the task's declared surface, so the audit is not
  copied there and `website/sidebars.ts` gains no entry. Consequence, stated plainly: an adopter
  reading only the published site does not see this table until that follow-up lands. It is named in
  Open Questions.
- **A `Taskfile.yml` target.** The root Taskfile is outside the declared surface; the suite is
  invoked as `bash .engineering/checks/run.sh`, which is what the step map runs.
- **Auditing anything the corpus does not invite.** An internal document key no guide mentions is not
  an adopter-facing declaration, and adding it would make the table a list of the repository's YAML
  keys rather than an answer to the adopter's question.
- **Judging whether a guarantee is a good one.** A check decides that a guarantee is stated and where
  its reason is written. Whether the trade is right is a person's call, and it is what the review
  state is for.

## Invariants

- **A closed vocabulary is not a defect; an unexplained one is.** No check treats `closed` as
  failing. The failing condition is R10's — unsettled and unremarked.
- **Every verdict has a path behind it.** No cell in the `Decided by` column may be prose. If a
  verdict cannot be attached to a file in this tree, it is not entered.
- **A vacuous check is a failed check.** Every quantified assertion has a floor that stops it being
  true of the empty table (R11), and every citation check resolves something that can actually
  disappear (R6, R7, R9, R10).
- **The scan is deterministic.** Same tree, same bytes. A scan that varies between runs turns R13's
  completeness check into noise and the next round into a rewrite.
- **One row per layer.** A verdict is never averaged across the layer that is open and the layer that
  is not (R5).
- **The runner prints its table on every path.** No check aborts the script before the report.

## Acceptance Criteria

Demonstrated by running the suite, and by four deliberate mutations that must each turn it red.

1. `bash .engineering/checks/run.sh` from the repository root exits 0 and prints one row per check,
   each naming the unit it decides.
2. Deleting from the table a row for a candidate `scan-declarations.sh` emits turns the completeness
   check red.
3. Changing a settled closed row's `Guarantee` cell to `none` while leaving `Follow-up` as `—` turns
   a check red.
4. Pointing a `Follow-up` cell at an artifact id that is not in the store turns a check red.
5. Deleting the quoted fragment named in a row's `Invited at` cell from the corpus file it cites
   turns a check red.
6. `protocol validate --root .` exits 0 and `protocol artifact validate` exits 0.
7. `git status --porcelain` in this repository shows changed paths only under `docs/` and
   `.engineering/`.
8. `bash .engineering/checks/scan-declarations.sh` run twice produces identical output, and neither
   run touches the network.

Criteria 2 through 5 are the ones that matter. 1, 6 and 7 show the suite runs inside its surface;
only a mutation shows it discriminates.

## Open Questions

**Does the audit belong on the published site as well as in `docs/guide/`?**
Decides: protocol owner. Default if nobody answers: **yes, as a follow-up story**, not in this run —
`website/` is outside the task's declared surface, and copying it there without the sidebar entry
would publish an unreachable page. Recorded because the story's own audience argument points at the
published site.

**Is `docs/plan/document-authoring-brief.md` part of the corpus?**
Decides: protocol owner. Default: **yes** — `docs/guide/README.md:30-32` sends the adopter there for
the full vocabulary, so a declaration it invites is adopter-facing whatever directory it sits in.

**Are `website/docs/status/*.md` part of the corpus?**
Decides: protocol owner. Default: **yes** — `limitations.md` carries the most adopter-declaration
language of any file in the tree, and a limitation that names a closed vocabulary is exactly the
kind of row this audit exists to collect.

**Should the completeness check also fail on a scan candidate that is new since the last round?**
Decides: implementer, at the design state. Default: **no** — a new candidate with no row already
fails R13's completeness rule, and a separate "new since last round" check would need a stored
baseline this run has nowhere to keep.
