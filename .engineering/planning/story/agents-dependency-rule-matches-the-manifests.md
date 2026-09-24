---
format: aep.planning-md/2
id: story:agents-dependency-rule-matches-the-manifests
kind: story
status: draft
title: AGENTS.md's dependency-comment rule matches what the manifests do
scope:
- confidence: cited
  path: AGENTS.md
revision: 2
---
## Outcome

`AGENTS.md` states the dependency-comment rule the repository's manifests follow. Either the rule
says which entries need an explanation beside them (and the manifests match it), or the manifests
gain the explanations the rule demands. A reviewer citing the line and a contributor reading the
manifests reach the same answer.

## Why

`AGENTS.md:212` at 1a5ceda4: "Prefer no new dependency. Explain every necessary dependency beside
its manifest entry."

Counted by the wave sub-operator on 2026-09-21 across `crates/*/*/Cargo.toml`:

| dev-dependency entries | with a comment immediately above | bare |
| --- | --- | --- |
| 33 | 7 | 26 |

The bare entries include workspace-internal dev-dependencies such as `aep-driver`'s `aep-project`,
`aep-schema`, `serde_yaml` and `trace-spec`. So the manifests follow the rule for 21 % of
dev-dependencies and, for workspace-internal dev-dependencies, in effect never.

How it surfaced: unit 7 of wave-validate-v2-20260920 (story:eventlog-store-hydrates-from-one-snapshot)
added `aep-domain` as a dev-dependency of `aep-backend-eventlog`. The implementor wrote a comment,
the sub-operator removed it under the operator's global rule that manifests carry no explanatory
comments, and review pass 2 flagged the bare line under `AGENTS.md:212`. The finding was recorded
no-op on the count above and the line stays bare (review-result:one-snapshot-hydrate-review-2).

The operator's global rule and this line also contradict each other for regular dependencies; the
operator owns that reconciliation. This story is only about the repository's own line matching the
repository's own manifests.

## Acceptance

- `AGENTS.md` says which dependency entries carry an explanation beside them (for example: a new
  external dependency, never a workspace-internal one) and where the reasoning for the rest lives
  (the commit message). The sentence is checkable against the manifests.
- A count like the table above, taken after the change, matches the sentence.
- No manifest comment is added or removed by this story beyond what the sentence requires.
