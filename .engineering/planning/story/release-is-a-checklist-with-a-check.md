---
format: aep.planning-md/3
id: story:release-is-a-checklist-with-a-check
kind: story
status: implemented
title: A release is a checklist, and one command says whether it was followed
summary: The operator had to define release; eleven tags shipped with red CI; two tags never pushed. A checklist and task release-check.
tags:
- process
- release
relations:
- decomposes: epic:declared-configuration-invariants
- serves: vision:O6
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-08-30T15:48:11Z", actor: "agent:claude-audit", revision: 2, imported: true}
- {from: "proposed", to: "active", at: "2026-08-30T15:48:12Z", actor: "agent:claude-audit", revision: 3, imported: true}
- {from: "active", to: "implemented", at: "2026-08-30T15:48:12Z", actor: "agent:claude-audit", revision: 4, decided_on: {"recorded":{"test_result":1}}, imported: true}
---
# Story: A release is a checklist, and one command says whether it was followed

## Outcome

"Cut a release" means one list of steps, and `task release-check` reports which were done.

## Context

- `4d4c15a4#400-#401,#587`: the operator had to define it — "cutting a release means changelog update,
  commit new tag, + gh release" — after the coordinator conflated release with merge-to-main.
- `CHANGELOG.md:75`: "`release` may not be automated: it is a written procedure nothing enforces, and it has
  already slipped once".
- `9da4f51c#2662`: eleven releases tagged with red CI; `ed007513#2625`: `0.27.1`/`0.27.2` tagged locally,
  never pushed; `#2678`: a GitHub Release created by hand.

## Acceptance

- `AGENTS.md` § Releases is a numbered checklist; `skills/wave/SKILL.md` *Close it* links it and says
  "release is not part of a wave".
- `task release-check` verifies: newest tag == `[workspace.package] version` == top `CHANGELOG.md` heading;
  the tag is pushed; a GitHub Release exists for it; the gate's `test_result` for the tag's commit is in the
  store. Each missing item is one line.

## Out of Scope

Running the release from `task`.
