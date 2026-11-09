---
format: aep.planning-md/3
id: story:agents-dependency-rule-matches-the-manifests
kind: story
status: active
title: AGENTS.md's dependency-comment rule matches what the manifests do
relations:
- decomposes: epic:zero-open-issues-2026-11-09
- serves: vision:O2
scope:
- confidence: cited
  path: AGENTS.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T22:07:44Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-09T22:07:44Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":1}}}
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

- `AGENTS.md` § Conventions states, in place of "justify each one beside its manifest entry", that
  a new dependency is justified in the commit message that adds it, and that a manifest entry needs
  no comment beside it. After the change, `grep -n "beside its manifest entry" AGENTS.md` prints
  nothing.
- The rule then holds for every entry counted below (it demands nothing of any of them), where the
  0.79.0 rule held for 54 of 211.
- No manifest comment is added or removed by this story: `git diff --stat` over `Cargo.toml` files
  is empty.

Counted at triage (`824869fec`, a `#` on the entry's line or the line above, over
`crates/*/*/Cargo.toml`, `xtask/Cargo.toml` and the root `Cargo.toml`): `[dependencies]` 144
entries, 46 commented; `[dev-dependencies]` 31, 7 commented; root `[workspace.dependencies]` 36, 1
commented.

Decision taken at revision (coordinator): the commit message carries the reason, because the
operator's standing rule is that manifest files carry no explanatory comments, and the manifests
already follow that for 157 of 211 entries.

## Triage 2026-11-09

Still present at `824869fec`. The rule is now `AGENTS.md:185`: "Prefer no new dependency; justify each one beside its manifest entry." Counted across `crates/*/*/Cargo.toml` and `xtask/Cargo.toml` (an entry counts as commented when a `#` is on its line or the line above): `[dependencies]` 144 entries, 46 commented; `[dev-dependencies]` 31, 7 commented; root `[workspace.dependencies]` 36, 1 commented.
