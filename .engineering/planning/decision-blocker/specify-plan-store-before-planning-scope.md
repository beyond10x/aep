---
format: aep.planning-md/3
id: decision-blocker:specify-plan-store-before-planning-scope
kind: decision-blocker
status: cleared
title: Specify the plan store surface before changing planning_scope
relations:
- blocks: story:planning-scope-comes-from-the-repository
revision: 2
transitions:
- {from: "open", to: "cleared", at: "2026-10-08T18:28:23Z", actor: "human:timo", revision: 2}
---
# Decision: specify the plan store surface before changing `planning_scope`

## Question

`story:planning-scope-comes-from-the-repository` adds a `--planning-scope` flag and changes how the
`planning_scope` default is derived. The repository had no ESS specification. Build without one,
build without the flag, or specify the surface first?

## Options

| option | does | cost |
|---|---|---|
| A | build the flag and the derivation without a specification | the behaviour stays unspecified |
| B | build the derivation only, no flag | no override when `origin` is absent or wrong; still unspecified |
| C | retrofit an ESS specification of `aep plan store migrate git`, `aep plan reverse init` and the `planning_scope` default first, then build against it | one more unit in the wave |

## Decided

C, inside the same wave: the specification unit runs first; if ESS cannot express the flag or the
default, the wave stops there and reports what it refused.
