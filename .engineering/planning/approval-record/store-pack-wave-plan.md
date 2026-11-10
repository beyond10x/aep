---
format: aep.planning-md/3
id: approval-record:store-pack-wave-plan
kind: approval-record
status: draft
title: Wave plan for the store-pack stories, run with no operator turn
tags:
- non-interactive
relations:
- decides: story:lifecycle-declares-closed-states
- decides: story:pack-format-and-overlay-read
- decides: story:pack-and-unpack-verbs
- decides: story:validate-checks-packs
- decides: story:pack-end-to-end-and-docs
revision: 1
---
# Approval record: the store-pack wave plan, run with no operator turn

## Which stop

The stage-1 wave proposal of `aep:implementing` 0.26.0 (wave mode), for the five accepted stories of
`epic:store-pack-closed-artifacts`. Reason `non-interactive`: the coordinator is a sub-agent; the
operator's instruction ("Implement via aep:implementing wave mode", relayed by the orchestrating
session on 2026-11-10) reached it before the proposal, and no operator turn exists inside the run.

## The plan, derived from the store

`aep plan artifact waves --kind story --status active`, after the scopers' typed entries were written
and `CHANGELOG.md` (coordinator-held, below) and two inferred entries the coordinator holds units off
(`aep-backend-markdown/src/pack.rs` and `store_command/migrate_git.rs` for the pack verbs,
`website/docs/reference/cli.md` for validate) were removed from the units' scopes. Verbatim:

```text
wave 1
  story:lifecycle-declares-closed-states (inferred)
wave 2
  story:pack-format-and-overlay-read (inferred)
wave 3
  story:pack-and-unpack-verbs (inferred)
  story:validate-checks-packs (inferred)
wave 4
  story:pack-end-to-end-and-docs (inferred)
collision: story:lifecycle-declares-closed-states story:pack-format-and-overlay-read crates/edge/aep-cli/src/planning.rs
collision: story:lifecycle-declares-closed-states story:validate-checks-packs crates/edge/aep-cli/src/planning.rs
collision: story:pack-format-and-overlay-read story:validate-checks-packs crates/edge/aep-cli/src/planning.rs
4 wave(s), 3 collision(s), 0 unassessed
```

Check path: the verb (aep 0.79.0). Every story's scope is `inferred` overall (each has cited and
inferred entries; the `## Scope` sections mark each line).

## Decision taken in the operator's absence

The verb's waves 1 and 2 run together as one wave under the split-by-function fallback of
`references/wave.md`, because the only collision between them is `crates/edge/aep-cli/src/planning.rs`
and the two stories touch disjoint symbols there: `story:lifecycle-declares-closed-states` owns
`fn lifecycle` and `struct Lifecycle`; `story:pack-format-and-overlay-read` owns `evidence_problems`,
the `records` closure of `redact_literal` and the error mapping of `create_through_a_command`. Each
brief declares its symbols, each report pastes its hunk headers, and `git merge-tree` runs on the two
branches before the first merges. This is a deviation from the verb's placement, recorded here.
Waves 3 and 4 run as the verb placed them.

| run | stories | objective |
|---|---|---|
| A | `story:lifecycle-declares-closed-states`, `story:pack-format-and-overlay-read` | O2 |
| B | `story:pack-and-unpack-verbs`, `story:validate-checks-packs` | O2 |
| C | `story:pack-end-to-end-and-docs` | O2 |

Shared files held by the coordinator: `CHANGELOG.md` (each story's `[Unreleased]` line is written by
the coordinator at merge, from the unit's report) and the store.

Agents: `aep:story-scoper` (done, opus), `aep:implementor` (sonnet; a unit failing twice on sonnet is
redone once on opus), `aep:adversary` on every unit (opus), one `aep:security-reviewer` pass over the
feature (opus). At most five sub-agents at once.

## Commits this record authorises

One or more unit commits per story on its `impl/<story>` branch; the merges of each unit into
`feat/store-pack-closed-artifacts`; the coordinator's changelog and store commits (review-results,
review outcomes, scope corrections, evidence, the moves to `implemented`); a merge of `origin/main`
into the feature branch before the final gate; one push of the feature branch and one pull request
to `main`, both through `b10x-gates` as the bot. Not authorised: merging the pull request, a tag, a
version bump, a release.

## What the operator would have been asked

"Run this wave plan, with waves 1 and 2 together under the split-by-function fallback?"
