---
format: aep.planning-md/1
id: story:driver-run-state-integrity
kind: story
status: implemented
title: A resumed run sees one committed generation and no silent repeat
summary: Commit snapshot and cursor together and expose uncertain attempts for explicit resolution.
relations:
- derived_from: epic:architecture-hardening
- serves: vision:O6
revision: 4
---
## Finding

`crates/aep-driver/src/run.rs` writes snapshot and cursor independently. A crash can expose a mixed pair, and a crash after external execution but before persistence silently repeats the step.

## Acceptance

A run publishes hash-verified snapshot/cursor generations through one atomic current pointer. A valid legacy pair migrates before execution; a missing or mismatched pair is refused. The cursor persists an attempt id before dispatch. Resume with an unresolved attempt refuses unless `--retry-in-flight` repeats the same id or `--record-in-flight-no-verdict` records uncertainty. Circuit-breaker state survives resume. Crash-boundary tests prove no mixed pair or unapproved repeat.

## Scope

- `crates/aep-driver/`, `crates/aep-driver-spec/` and `crates/protocol-cli/src/drive.rs` — cited.
- renderer run-file discovery — inferred from `RunDirectory` callers; confirm before editing.
