# Designs

Every design here is a dated record: its body says what was decided or proposed on its date and is
not rewritten. The status column is read from each file's own `Status` line and from the later
designs and plan pages that accepted or superseded it. Invariant numbers inside a design are
`AGENTS.md`'s as of that design's date.

| Design | Status | Source of the status |
|---|---|---|
| [`consolidated-design-v0.2.md`](consolidated-design-v0.2.md) | accepted; normative for protocol semantics | `AGENTS.md` § *Normative documents*; `reconciliation-v0.2.md` |
| [`reconciliation-v0.2.md`](reconciliation-v0.2.md) | accepted; record of the v0.2 reconciliation and its deviations | its header, 2026-08-19 |
| [`harness-planning-and-driver-design-v0.1.md`](harness-planning-and-driver-design-v0.1.md) | accepted: Phase 1 by harness wave 1, Phase 2 built as the reference driver in wave 3 | its header; `docs/plan/archive/harness-wave-2-driver-decision.md` |
| [`transcript-conformance-design-v0.1.md`](transcript-conformance-design-v0.1.md) | accepted for implementation (its header still reads proposed) | `docs/plan/archive/trace-wave-1-transcript-checker.md`, 2026-08-21 |
| [`evidence-horizons-design-v0.1.md`](evidence-horizons-design-v0.1.md) | implemented (its header still reads proposed) | `docs/plan/gap-register.md` § *Closed by code, 2026-08-21 — evidence horizons* |
| [`workflow-declared-context-and-write-scope-v0.1.md`](workflow-declared-context-and-write-scope-v0.1.md) | accepted 2026-08-24; milestones 4–6 not started | its header |
| [`story-completion-evidence-design-v0.1.md`](story-completion-evidence-design-v0.1.md) | accepted in part, 2026-08-28: § 10.1 shipped, § 10.2 sequenced, § 10.3 refused | its header |
| [`fact-scoped-applicability-design-v0.1.md`](fact-scoped-applicability-design-v0.1.md) | proposed | its header |
| [`native-arm-store-integrity-design-v0.1.md`](native-arm-store-integrity-design-v0.1.md) | proposed, 2026-08-29 | its header |
| [`aep-service-wire-v0.1.md`](aep-service-wire-v0.1.md) | proposed | its header |
| [`ess-conformance-v2-evidence.md`](ess-conformance-v2-evidence.md) | accepted 2026-09-06 for the count stage | its header |
| [`ess-conformance-coverage-evidence.md`](ess-conformance-coverage-evidence.md) | accepted 2026-09-06 | its header |
| [`eventlog-planning-authority-v0.1.md`](eventlog-planning-authority-v0.1.md) | superseded for the planning store: first by `planning-on-entity-runtime-v0.1.md`, then by `git-native-planning-store-v0.1.md` | `planning-on-entity-runtime-v0.1.md` header |
| [`planning-store-selection-and-commands-v0.1.md`](planning-store-selection-and-commands-v0.1.md) | superseded for the planning store by `git-native-planning-store-v0.1.md`; its commands were removed in 0.62.0 | `CHANGELOG.md` 0.62.0 |
| [`planning-raw-capture-v0.1.md`](planning-raw-capture-v0.1.md) | superseded for the planning store by `git-native-planning-store-v0.1.md` | `CHANGELOG.md` 0.62.0 |
| [`planning-on-entity-runtime-v0.1.md`](planning-on-entity-runtime-v0.1.md) | superseded for storage by `git-native-planning-store-v0.1.md`; its typed lifecycles (`entity-core` decides every move) are kept | `git-native-planning-store-v0.1.md` header |
| [`planning-content-blobs-v0.1.md`](planning-content-blobs-v0.1.md) | superseded for the planning store by `git-native-planning-store-v0.1.md` (`aep.project/4` is refused since 0.62.0) | `CHANGELOG.md` 0.62.0 |
| [`git-native-planning-store-v0.1.md`](git-native-planning-store-v0.1.md) | accepted 2026-09-28; implemented in 0.62.0 | its header |
| [`archive/design-draft-v0.1.md`](archive/design-draft-v0.1.md) | superseded by `consolidated-design-v0.2.md` | `reconciliation-v0.2.md` |
| [`archive/artifact-model-extension-v0.1.md`](archive/artifact-model-extension-v0.1.md) | superseded by `consolidated-design-v0.2.md` | `reconciliation-v0.2.md` |

A design is proposed until a plan page or planning artifact accepts it (`AGENTS.md`).
