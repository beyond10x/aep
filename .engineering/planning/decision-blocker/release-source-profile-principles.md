---
format: aep.planning-md/3
id: decision-blocker:release-source-profile-principles
kind: decision-blocker
status: cleared
title: What the source release profile requires before complete
relations:
- blocks: story:release-step-map
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-15T09:13:18Z", actor: "human:timo", revision: 3}
---
# Decision: what the source release profile requires before `complete`

`story:release-step-map`'s fixture run reaches `verify` and stays blocked there, whatever the map
does. `profiles/release-source.yaml` (`release.source`) lists the principle `verify-after-action`,
which has no applicability condition and asks for `service.health == healthy`,
`verification.recovery.passed` and an independent `health_observation`: production-health
observations a source release has no service for. `provenance-tracking` asks for one `diff` record
for every task kind except `incident`.

Options:

- **A. Drop `verify-after-action` from `release.source`** (its completion already requires
  `verification.source-release.passed`), and let the release map mint the `diff` from a command
  step (the release commit's diff against the previous tag). Cost: a change to a shipped profile,
  so a minor release and a changelog line; consumers on `release.source` stop being asked for
  health evidence they cannot give.
- **B. Make `verify-after-action` applicable only where a service exists** (an applicability
  condition on the principle). Cost: a principle-model change, ESS first, every profile that lists
  it re-read.
- **C. Keep the profile; the release map stops at `verify`** and the epic's release Done When is
  narrowed. Cost: no release reaches `complete` under `aep drive run`.

Recommendation: A. `story:release-step-map` waits on this decision; the wave's other units go on.

## Decided

Option A. `release.source` drops `verify-after-action`; its completion still requires `verification.source-release.passed`, so a source release loses no check it can show. The release map mints the `diff` record from a command step. It ships in the next minor release with a changelog line.
