---
format: aep.planning-md/1
id: task:ova-scan-declarations
kind: task
status: implemented
title: 'scan-declarations.sh: the candidate surfaces the tree declares'
summary: 'The derivation: sorted, byte-identical between runs, tree-only, emitting every top-level vocabulary key under protocols/ and every adopter-writable document family under artifacts/.'
owner: protocol
tags:
- adoption
- checks
relations:
- decomposes: specification:open-vocabulary-audit
- derived_from: task:w4-2-open-vocabulary-audit
revision: 4
---
# Task: the derivation, and its determinism

## What

**R12.** `.engineering/checks/scan-declarations.sh`: emits, one per line and sorted, the declaration
surfaces the tree declares **in documents** — every top-level vocabulary key under
`protocols/*/*.yaml`, and every adopter-writable document family under `artifacts/` (`kinds/`,
`lifecycles/`, `relations/`, `templates/`).

It reads the tree, makes no network call, writes nothing, and two runs against an unchanged tree
produce byte-identical output.

It does **not** write the audit. It emits candidates; a person reads the corpus and writes the rows.

## Why

Without the scan, R13's completeness check has nothing to compare the table against, and the next
round is a re-read rather than a diff. Determinism is not tidiness here: a scan whose order varies
turns the completeness check into noise, and a noisy check is one a future round switches off.

## Done When

| # | Acceptance |
|---|---|
| S1 | `bash .engineering/checks/scan-declarations.sh` from the repository root exits 0 and prints at least one candidate per line, with no blank lines and no diagnostics on stdout. |
| S2 | The output equals its own `sort -u` — sorted, no duplicates. |
| S3 | Two consecutive runs against an unchanged tree are byte-identical, compared with `cmp`. (Acceptance criterion 8.) |
| S4 | Every top-level vocabulary key of `protocols/aep/1.yaml` appears, asserted by name: `capabilities`, `evidence_kinds`, `verifiers`, `artifact_kinds`, `phases`, `observables`, `scales`. |
| S5 | Every top-level vocabulary key `protocols/adp/1.yaml` extends appears, asserted by name against that file rather than from a list typed into the check. |
| S6 | The four `artifacts/` families appear: `kinds`, `lifecycles`, `relations`, `templates`. |
| S7 | Adding a top-level key to a **copy** of a protocol document makes a new candidate appear; removing one makes it disappear. Shown both ways against the copy, with the real tree unchanged. |
| S8 | `git status --porcelain` is identical before and after a run — the scan writes nothing into the tree. |
| S9 | The run completes with `curl`, `wget`, `nc`, `jq`, `yq`, `python` and `node` shadowed by stubs that exit 127. |

## Notes

- S5 is the guard against the scan being a list of keys somebody typed: it re-derives from the
  document rather than asserting a constant. A constant here would go stale the first time a
  protocol document gains a key, which is the exact event the scan exists to catch.
- No `yq`. The workspace's constraint is `bash`, `git`, `protocol` only; top-level keys in these
  documents are column-zero `key:` lines, which shell can decide.
- The scan cannot find a closed surface — a closed surface is precisely one with no document key.
  That limit is stated in the audit by `task:ova-scan-loop`, not here.

## Verifier

`.engineering/checks/check-scan.sh`. S1–S9 are its rows.

S7 copies `protocols/` into `${TMPDIR:-$HOME/.cache/claude-tmp}` and runs the scan against the copy
via its root argument; the check fails if the scan cannot be pointed at a root, because a scan that
can only read the live tree cannot be tested without mutating it.
