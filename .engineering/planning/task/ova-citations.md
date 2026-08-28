---
format: aep.planning-md/1
id: task:ova-citations
kind: task
status: implemented
title: 'Invited at and Decided by: every verdict has a path behind it'
summary: 'R6 and R7: each Invited at cites a corpus path with a line number and a verbatim fragment that still occurs there; each Decided by resolves to a file in this tree.'
owner: protocol
tags:
- adoption
- checks
relations:
- decomposes: specification:open-vocabulary-audit
- derived_from: task:w4-2-open-vocabulary-audit
- depends_on: task:ova-checks-runner
- depends_on: task:ova-table-shape
revision: 6
---
# Task: every verdict has a path behind it

## What

**R6–R7.** Two columns, both resolved against the tree rather than read:

- `Invited at` holds a corpus path with a line number and a verbatim quoted fragment from that file.
  The fragment is the drift detector: a guide that stops inviting the declaration turns the row red,
  which is the signal the next round needs.
- `Decided by` holds a path in this repository that settles the verdict — a document key for an open
  row (`protocols/aep/1.yaml:capabilities`), a `file:line` for a closed one
  (`crates/aep-domain/src/artifact.rs:707`).

An `open` row is therefore exactly as falsifiable as a closed one.

## Why

This is the invariant *no cell in `Decided by` may be prose* made mechanical. A verdict with no path
behind it cannot be entered, and a citation that has rotted announces itself instead of quietly
becoming a claim about a file that no longer says it.

## Done When

| # | Acceptance |
|---|---|
| I1 | Every `Invited at` cell parses as a path, a line number and a quoted fragment. A cell missing any of the three is red and named. |
| I2 | Each such path is a member of the R1 corpus, re-derived from the globs rather than from the audit's own list. |
| I3 | Each quoted fragment occurs verbatim in the file it cites. |
| I4 | Each cited line number is within that file's length. |
| I5 | Every `Decided by` cell is a path in this repository in one of two forms: `<file>:<key>` or `<file>:<line>`. |
| I6 | For the `<file>:<line>` form, the file exists and has at least that many lines. |
| I7 | For the `<file>:<key>` form, the file exists and the key occurs in it as a declaration, not merely as a substring of prose. |
| I8 | No `Decided by` cell is prose: a cell that does not resolve to an existing path under either form is red, naming the cell. |
| I9 | Deleting a cited fragment from its corpus file on a scratch copy turns I3 red, naming the row. (Acceptance criterion 5.) |
| I10 | Every row in the table is covered — the count of cells checked equals the row count, so a row cannot be skipped by being unparseable. |
| I11 | Each quoted fragment occurs **at the line the cell cites**, not merely somewhere in the file. Added by the adversarial pass. |
| I12 | For a **closed** row citing a `crates/` `file:line`, that line is an item declaration, never a use site. Added by the adversarial pass. |
| I13 | For an **open** row citing a `crates/` `file:line`, that line is the variant admitting a value of the adopter's own — not the enum head. Added by the adversarial pass. |

## What the adversarial pass found

I3 and I6 resolved the *path* halves of both grammars and left the numbers decorative. Three
mutations ran green against the suite before I11–I13 existed:

| Mutation | What the reader would have got |
|---|---|
| a line inserted at the top of `website/docs/reference/vocabulary.md` | eight rows citing a line that is no longer the one quoted — the ordinary consequence of somebody adding a paragraph |
| a `Decided by` moved from the enum to a line that calls it | a closed verdict cited to `RelationKind::parse(…)`, which settles nothing without reading `RelationKind` |
| an `open` verdict moved onto the enum head | a link landing on a ten-variant enum, from which the honest conclusion is *closed* |

Two of the three were **live**, not hypothetical. `Relation names a relations document may use`
decided at `crates/protocol-cli/src/planning.rs:395`, a call site; `Test suite names in a tests fact
path` decided at `crates/aep-domain/src/evidence.rs:38`, the head of `pub enum TestSuite`, when the
line that makes it open is `Named(String)` at 60. Both cells were repointed.

## Notes

- I10 is the guard against the shape this check would otherwise take: a loop that silently ignores
  rows it cannot parse, reporting green because it examined nothing.
- The fragment is compared as a fixed string, not a pattern. Quoting a fragment containing a pipe is
  therefore the one thing that would need escaping, and the way to avoid it is to quote a shorter
  fragment.
- I7 needs the key to be a column-zero `key:` line for a protocol document. A key that appears only
  inside a sentence is not what decided the verdict.

## Verifier

`.engineering/checks/check-citations.sh`. I1–I13 are its rows.

I9 copies the tree under `${TMPDIR:-$HOME/.cache/claude-tmp}`, deletes one cited fragment, and
requires the check to exit non-zero there. `task:ova-mutation-proof` runs the same mutation as part
of the four; the duplication is deliberate, because this check must be able to demonstrate its own
discrimination without depending on the task that audits all four.
