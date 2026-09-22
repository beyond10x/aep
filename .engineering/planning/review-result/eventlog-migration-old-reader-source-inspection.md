---
format: aep.planning-md/1
id: review-result:eventlog-migration-old-reader-source-inspection
kind: review-result
status: active
title: 'Coordinator inspection: explicit legacy store paths bypass project version refusal'
relations:
- reviews: story:eventlog-planning-authority-migration
revision: 1
---
# Coordinator inspection of explicit-store old-reader writes

Subject: story:eventlog-planning-authority-migration and existing planning source at
4eb999e0ae3cc77d1c387152e23a85ad4eae86dc. This is coordinator source inspection plus a direct
installed-CLI fixture, not an independent implementation attack or a passing migration test.

## Measured behavior

The installed protocol 0.55.0 executable was run against a disposable synthetic project declaring
version aep.project/2 and an initially empty Markdown planning directory. Project-discovered
`aep plan artifact list` refused with unsupported_protocol_version and exited 1. The same command
with `--store .engineering/planning` exited 0. An explicit-store `artifact new story` then created
the synthetic draft and journal and exited 0. Project config before and after compared unchanged.
No real planning authority was migrated or altered by this probe.

Direct logs, actual exits and hashes are retained at
local-evidence:ess-evolution/waves/0005-aep-migration/analysis-scratch/old-reader-probe/.
The source reaches this behavior in crates/edge/aep-cli/src/planning.rs:140: an explicit store
returns the Markdown plan before project validation. Legacy Markdown collection skips dot entries
and recursively loads visible Markdown files; frontmatter format validation is exact but permits
extra metadata keys. A new hidden marker or extra key alone therefore cannot prove old refusal.

## Consequence and required correction

The accepted migration direction already requires old-reader refusal and forbids explicit paths
from turning a version-2 projection into writable authority. Changing project.version alone cannot
meet that requirement. The concrete companion must select and test projection identification for
empty and populated stores, all explicit/project-discovered mutation routes and both binary names,
and retain proved old-writer quiescence or enforced exclusion during and after cutover.

No particular new projection descriptor/format/fence is selected by this inspection. The finding
is open until the chosen contract and its executable compatibility cases close the measured path.
It does not justify replacing Eventlog authority or omitting any legacy backend or real cutover.

| File and line | Verdict | Origin | Finding |
| --- | --- | --- | --- |
| crates/edge/aep-cli/src/planning.rs:140 | NEEDS-CHANGE | pre-existing | An explicit --store path bypasses project-version validation and permits an old CLI to write Markdown under a version-2 project. |

```findings
- file: crates/edge/aep-cli/src/planning.rs
  line: 140
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: An explicit --store path bypasses project-version validation and permits an old CLI to write Markdown under a version-2 project.
```
