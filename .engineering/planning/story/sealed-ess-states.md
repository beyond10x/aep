---
format: aep.planning-md/3
id: story:sealed-ess-states
kind: story
status: implemented
title: Validated ESS and compiled IR cannot be forged
summary: Seal public fields and make compilation begin with complete validation.
relations:
- derived_from: epic:architecture-hardening
- serves: vision:O2
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-08-30T22:08:50Z", actor: "human:operator", revision: 2, imported: true}
- {from: "proposed", to: "active", at: "2026-08-30T22:08:50Z", actor: "human:operator", revision: 3, imported: true}
- {from: "active", to: "implemented", at: "2026-08-30T23:50:45Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, imported: true}
---
## Finding

Validated `Specification` and compiled `EssIr` expose public fields, and compiler entry points do not first invoke complete specification validation. Rust callers can construct invalid states that later phases treat as authoritative.

## Acceptance

Both types expose documented read-only accessors and no unrestricted construction surface. `RawSpecFile` conversion remains the specification entrance and the compiler remains the IR entrance. Every compiler entry point accumulates validation diagnostics before resolution. External direct construction is pinned impossible, existing consumers use accessors, and canonical serialization errors are explicit rather than hashed as empty bytes.

## Scope

- `crates/ess-domain/` and `crates/ess-compiler/` — cited.
- ESS provenance, graph and diff consumers — inferred from direct field reads; confirm with `rg`.
