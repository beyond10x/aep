# Zero open issues, 2026-11-09: wave plan and wave record

The plan and the record of the waves that deliver `epic:zero-open-issues-2026-11-09`: GitHub issues
#152 and #153 and the eight defects still present in the planning store. The run is
non-interactive; each wave's approval is an `approval-record` tagged `non-interactive`. No release.

Integration branch: `wave/zero-open-issues-2026-11-09`, forked from `824869fec`. One pull request
from it carries every wave. Every checkout is a managed worktree (`worktree create`); each unit
builds into `target/` inside its own tree. Paths are recorded by managed tree id; the coordinator's
scratch record maps ids to paths.

## The plan

Derived with `aep plan artifact waves --kind story --status active` (aep 0.79.0) after the ten
stories were accepted and the parallel-safety critic's round-2 scope corrections were written. The
verb's answer, verbatim:

```
wave 1
  story:a-directory-and-a-file-inside-it-read-as-disjoint (inferred)
  story:agents-dependency-rule-matches-the-manifests
  story:checks-map-evidence-producers
  story:discarded-protocol-source-tree-is-removed (inferred)
wave 2
  story:legacy-ess-report-calendar-refusal (inferred)
  story:published-pattern-residue
wave 3
  story:relate-and-unrelate-write-the-revision-they-print (inferred)
wave 4
  story:relations-order-is-the-same-in-every-store (inferred)
wave 5
  story:an-artifact-id-can-be-renamed (inferred)
wave 6
  story:history-shows-a-removed-edge (inferred)
collision: story:a-directory-and-a-file-inside-it-read-as-disjoint story:an-artifact-id-can-be-renamed crates/edge/aep-cli/src/planning.rs (inferred)
collision: story:a-directory-and-a-file-inside-it-read-as-disjoint story:history-shows-a-removed-edge crates/edge/aep-cli/src/planning.rs (inferred)
collision: story:a-directory-and-a-file-inside-it-read-as-disjoint story:legacy-ess-report-calendar-refusal crates/edge/aep-cli/src/planning.rs (inferred)
collision: story:a-directory-and-a-file-inside-it-read-as-disjoint story:published-pattern-residue crates/govern/aep-domain/src/artifact.rs
collision: story:a-directory-and-a-file-inside-it-read-as-disjoint story:relate-and-unrelate-write-the-revision-they-print crates/edge/aep-cli/src/planning.rs (inferred)
collision: story:a-directory-and-a-file-inside-it-read-as-disjoint story:relations-order-is-the-same-in-every-store crates/edge/aep-cli/src/planning.rs (inferred)
collision: story:an-artifact-id-can-be-renamed story:history-shows-a-removed-edge crates/edge/aep-cli/src/planning.rs (inferred)
collision: story:an-artifact-id-can-be-renamed story:legacy-ess-report-calendar-refusal crates/edge/aep-cli/src/planning.rs
collision: story:an-artifact-id-can-be-renamed story:relate-and-unrelate-write-the-revision-they-print crates/edge/aep-cli/src/planning.rs
collision: story:an-artifact-id-can-be-renamed story:relate-and-unrelate-write-the-revision-they-print crates/edge/aep-cli/src/planning/git_record.rs (inferred)
collision: story:an-artifact-id-can-be-renamed story:relations-order-is-the-same-in-every-store crates/edge/aep-cli/src/planning.rs (inferred)
collision: story:history-shows-a-removed-edge story:legacy-ess-report-calendar-refusal crates/edge/aep-cli/src/planning.rs (inferred)
collision: story:history-shows-a-removed-edge story:relate-and-unrelate-write-the-revision-they-print crates/edge/aep-cli/src/planning.rs (inferred)
collision: story:history-shows-a-removed-edge story:relate-and-unrelate-write-the-revision-they-print crates/plan/aep-backend-markdown/src/journal.rs (inferred)
collision: story:history-shows-a-removed-edge story:relations-order-is-the-same-in-every-store crates/edge/aep-cli/src/planning.rs (inferred)
collision: story:history-shows-a-removed-edge story:relations-order-is-the-same-in-every-store crates/plan/aep-backend-entity/src/lib.rs (inferred)
collision: story:legacy-ess-report-calendar-refusal story:relate-and-unrelate-write-the-revision-they-print crates/edge/aep-cli/src/planning.rs
collision: story:legacy-ess-report-calendar-refusal story:relations-order-is-the-same-in-every-store crates/edge/aep-cli/src/planning.rs (inferred)
collision: story:relate-and-unrelate-write-the-revision-they-print story:relations-order-is-the-same-in-every-store crates/edge/aep-cli/src/planning.rs (inferred)
collision: story:relate-and-unrelate-write-the-revision-they-print story:relations-order-is-the-same-in-every-store docs/design/git-native-planning-store-v0.1.md (inferred)
6 wave(s), 20 collision(s), 0 unassessed
```

| wave | story | objective |
|---|---|---|
| 1 | `story:a-directory-and-a-file-inside-it-read-as-disjoint` | O3 |
| 1 | `story:agents-dependency-rule-matches-the-manifests` | O2 |
| 1 | `story:checks-map-evidence-producers` | O3 |
| 1 | `story:discarded-protocol-source-tree-is-removed` | O2 |
| 2 | `story:legacy-ess-report-calendar-refusal` | O2 |
| 2 | `story:published-pattern-residue` | O2 |
| 3 | `story:relate-and-unrelate-write-the-revision-they-print` (#153) | O2 |
| 4 | `story:relations-order-is-the-same-in-every-store` | O2 |
| 5 | `story:an-artifact-id-can-be-renamed` (#152) | O2 |
| 6 | `story:history-shows-a-removed-edge` | O2 |

Units run one at a time (one sub-agent at once, by the operator's budget), so the waves order the
merges rather than the concurrency. `CHANGELOG.md` is the coordinator's file and in no unit's scope.

## Commits the approvals authorise

One commit per unit on its `impl/<slug>` branch, one merge per unit into the integration branch,
the coordinator's store and changelog commits on the integration branch, the push of the
integration branch and one pull request into `main`, merged once the gate is green. No tag, no
version bump, no release.

## Units

| wave | unit | managed tree | branch | head | stage |
|---|---|---|---|---|---|

## Backlog filed from findings

None yet.
