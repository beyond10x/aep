---
format: aep.planning-md/3
id: story:driver-run-state-integrity
kind: story
status: implemented
title: A resumed run sees one committed generation and no silent repeat
summary: Commit snapshot and cursor together and expose uncertain attempts for explicit resolution.
relations:
- derived_from: epic:architecture-hardening
- serves: vision:O6
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-08-30T22:08:51Z", actor: "human:operator", revision: 2, imported: true}
- {from: "proposed", to: "active", at: "2026-08-30T22:08:51Z", actor: "human:operator", revision: 3, imported: true}
- {from: "active", to: "implemented", at: "2026-08-30T23:50:49Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, imported: true}
---
## Finding

`crates/aep-driver/src/run.rs` writes snapshot and cursor independently. A crash can expose a mixed pair, and a crash after external execution but before persistence silently repeats the step.

## Acceptance

A run publishes hash-verified snapshot/cursor generations through one atomic current pointer. A valid legacy pair migrates before execution; a missing or mismatched pair is refused. The cursor persists an attempt id before dispatch. Resume with an unresolved attempt refuses unless `--retry-in-flight` repeats the same id or `--record-in-flight-no-verdict` records uncertainty. Circuit-breaker state survives resume. Crash-boundary tests prove no mixed pair or unapproved repeat.

## Scope

- `crates/aep-driver/`, `crates/aep-driver-spec/` and `crates/protocol-cli/src/drive.rs` — cited.
- renderer run-file discovery — inferred from `RunDirectory` callers; confirm before editing.
