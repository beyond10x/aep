---
format: aep.planning-md/3
id: story:admit-er-direct-return-evidence
kind: story
status: active
title: Admit exact direct-return ESS conformance evidence for ER
relations:
- serves: vision:O2
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/edge/aep-cli
- confidence: cited
  path: crates/govern/aep-domain
- confidence: cited
  path: crates/observe/aep-ess-evidence
- confidence: cited
  path: docs
- confidence: cited
  path: schemas/generated
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T07:38:44Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-09-28T07:38:45Z", actor: "human:timo", revision: 7}
---
## Acceptance

The ER library specification's exact passing ESS report and inventory-bearing suite are admitted as typed conformance evidence while malformed, contradictory, unknown-field, wrong-digest, and incomplete reports remain refused, allowing ER to adopt its executable contract and release on the current Git-native planning store.

## Authorized scope

The operator requested completing ER's mainline integration, moving its own plan to the current planning store, cleanup and release. This is the narrow AEP boundary fix previously recorded as ER's `blocker:er-ess-suite27-evidence`. ESS 0.38.0 has since allocated suite/26–27 to other constructs, so the direct-return extension must use a fresh version before final admission is wired. Coordinate the exact new versions with that extension; never relabel existing suite evidence.

## Implementation contract

Keep AEP core independent of compiled ESS modeling crates. Extend the closed optional aep-ess-evidence reader and narrow AEP count/coverage value types as needed. Preserve exact original suite association, model digest, scenario identity/counts, inventory and parent-lineage guards. Use the real ER report as an integration fixture and mutation tests for refusals. Update schemas only through xtask and retain source/design traceability. Preserve legacy admissions and their tests.

## Scope

Cited: crates/observe/aep-ess-evidence, crates/govern/aep-domain/src/ess_conformance*, related schema projections and CLI evidence tests. Inferred: documentation and changelog entries for the newly admitted formats. No general ESS dependency or broad store behavior change.
