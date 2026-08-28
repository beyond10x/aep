---
format: aep.planning-md/1
id: task:ova-mutation-proof
kind: task
status: implemented
title: Four deliberate mutations, each turning the suite red
summary: 'Acceptance 2-5: a deleted candidate row, a guarantee downgraded to none, a follow-up pointing at nothing, and a deleted quoted fragment, each shown to turn a named check red and then restored.'
owner: protocol
tags:
- adoption
- checks
relations:
- decomposes: specification:open-vocabulary-audit
- derived_from: task:w4-2-open-vocabulary-audit
- depends_on: task:ova-checks-runner
- depends_on: task:ova-scan-loop
- depends_on: task:ova-closed-cells
- depends_on: task:ova-followups
- depends_on: task:ova-citations
revision: 9
---
# Task: nine deliberate mutations, each turning the suite red

The frontmatter still says *four*; it is the CLI's field and there is no verb to retitle an
artifact, so it is left for the operator rather than hand-edited. Mutations 5–9 were added at
`adversarial_verify`.

## What

Acceptance criteria 2 through 5, and five more. Each mutation is applied **alone** to a copy of the
tree, and each is required to turn a **named** check red:

| # | Mutation | Must redden |
|---|---|---|
| 1 | delete from the table a row for a candidate `scan-declarations.sh` emits | `scan-loop` |
| 2 | set a settled closed row's `Guarantee` to `none`, leaving `Follow-up` as `—` | `followups` |
| 3 | point a `Follow-up` cell at an artifact id that is not in the store | `followups` |
| 4 | delete the quoted fragment named in a row's `Invited at` from the corpus file it cites | `citations` |
| 5 | insert a line above a cited fragment, leaving the row's line number pointing past it | `citations` |
| 6 | rename the heading a `Reason for adopters at` anchor names | `closed-cells` |
| 7 | repoint a closed row's reason at the audit itself | `closed-cells` |
| 8 | repoint a closed verdict from the declaration that settles it to a use site | `citations` |
| 9 | repoint an open verdict onto the enum head, where a reader reads it as closed | `citations` |

The whole thing runs against a copy under `${TMPDIR:-$HOME/.cache/claude-tmp}`. The real tree is not
touched.

## Why

The specification says it plainly: criteria 2 through 5 are the ones that matter. Criteria 1, 6 and 7
show the suite runs inside its surface; **only a mutation shows it discriminates.** A suite of twelve
green checks that would stay green under a broken audit has measured nothing, and there is no way to
tell that by reading it.

This is also the unit that catches a check going red for the wrong reason — a mutation that reddens
some other check has demonstrated a coupling nobody intended.

## Done When

| # | Acceptance |
|---|---|
| M1 | The check operates on a copy of the tree; `git status --porcelain` in the real repository is byte-identical before and after it runs. |
| M2 | With no mutation applied, the copy runs the full suite to exit 0 — so M3–M6 are attributable to the mutation and not to the copying. |
| M3 | Mutation 1 makes `check-completeness.sh` exit non-zero, and its output names the orphaned candidate. |
| M4 | Mutation 2 makes `check-followups.sh` exit non-zero, and its output names the row that became unsettled. |
| M5 | Mutation 3 makes `check-followups.sh` exit non-zero, and its output names the unresolvable id. |
| M6 | Mutation 4 makes `check-citations.sh` exit non-zero, and its output names the row whose fragment vanished. |
| M7 | Each mutation is applied to a fresh copy, alone. A mutation that reddens a check other than the one named is a failed row, reported with both check names. |
| M8 | A mutation that reddens **no** check is a failed row naming the mutation — the case this task exists for. |
| M9 | The nine mutations are described in the audit's own method section, so a future round can re-run them by hand, and the count stated there equals the count that runs. |
| M10 | Mutation 5 — a line inserted above a cited fragment — reddens `citations`, naming the row whose line number went stale. Added by the adversarial pass. |
| M11 | Mutation 6 — the heading a reason's anchor names, renamed — reddens `closed-cells`. Added by the adversarial pass. |
| M12 | Mutation 7 — a reason repointed at the audit itself — reddens `closed-cells`, naming the row. Added by the adversarial pass. |
| M13 | Mutation 8 — a closed verdict repointed from the declaration to a use site — reddens `citations`, naming the line. Added by the adversarial pass. |
| M14 | Mutation 9 — an open verdict repointed onto the enum head — reddens `citations`. Added by the adversarial pass. |

## What the adversarial pass found

Nothing here was broken. What was missing is the thing this task exists to supply: mutations 1–4 are
the specification's own acceptance criteria, so they showed the suite discriminates on exactly the
four cases somebody had already thought of.

Five more were written and applied by hand to a copy first. **The suite exited 0 under every one of
them.** They are now mutations 5–9, and M8 — *a mutation that reddens no check is a failed row* —
would have caught all five had they been asked earlier. Two of them (8 and 9) reddened a live row
rather than a hypothetical one; see `task:ova-citations`.

## Notes

- M7's cross-check is the part that is easy to leave out and expensive to lack. "The suite went red"
  is not the assertion; "this check went red, and only this one" is.
- Mutation 2 is stated in the specification as reddening "a check"; it is `check-followups.sh`,
  because downgrading `Guarantee` to `none` moves the row into the unsettled partition and an
  unsettled row with `—` in `Follow-up` violates R10. `check-closed-cells.sh` stays green on it, and
  M7 asserts that too.
- Copying the tree means copying `.git` or working without it. `check-surface.sh`'s git rows are
  excluded from the copy's run; that exclusion is stated in the check's header rather than silently
  applied.
- Runs last in `run.sh`'s order, since it invokes the other checks.

## Verifier

`.engineering/checks/check-mutation-proof.sh`. M1–M14 are its rows.

It is the only check that runs other checks, and the only one that writes outside the repository. If
it cannot make a copy — no writable `${TMPDIR:-$HOME/.cache/claude-tmp}` — that is a failed row, not
a skip.
