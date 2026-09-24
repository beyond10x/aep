---
format: aep.planning-md/2
id: dependency-blocker:metaharness-source-receipt
kind: dependency-blocker
status: open
title: Metaharness runtime prerequisite lacks verified source evidence
relations:
- blocks: migration-plan:foundation-source-pins-20260909
revision: 1
---
# Missing runtime prerequisite

`crates/edge/aep-cli/src/drive.rs:2020` spawns the command constructed for an
LLM step. `drive.rs:3665` names the ambient `metaharness` executable, and
`drive.rs:4004` checks its availability. `crates/edge/aep-cli/src/eval.rs:1616`
also declares Metaharness as the executable used by live evaluation.

This is a runtime tool dependency even though the ordinary gate uses fixtures
and the planning workspace describes the tool as not being a dependency.
The source composition supplied verified Entity Runtime, Docs System, ESS
and Eventlog revisions, but no verified Metaharness revision or supported
exact executable-source selector.

Atlas must reconcile the prerequisite closure, establish an acyclic graph,
and provide verified Metaharness source evidence and a supported exact selector
before this consumer can be integrated. Do not start another provider chain,
accept an ambient binary as a pin, or infer that a passing gate or admission
check settles the missing input.

The local complete gate passed and candidate
`ddd69bd5c0407d1e2c8cf8c87a4a7e32f919f657` was published to a task branch.
Atlas dependency admission passed for that candidate, but main remains at
`35b5c9949d7b9e1caa3a5e5de7502af42c2dbc5e`. These results do not clear this
newly discovered blocker. The candidate is retained for the reconciled run.
