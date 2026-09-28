---
title: Scope and waves
sidebar_position: 7
description: A story records the surfaces it lands on. waves derives which stories can be implemented at the same time, and prints every collision and every story nobody assessed.
---

# Scope and waves

Choosing several stories to implement at the same time, for example one coding agent per story, is
a claim that they touch different parts of the code. AEP makes that claim checkable. Each story
records its **scope**, the paths it lands on, and `waves` works out the groupings from the scopes
and the `depends_on` edges.

## Scope

```shell-session
$ aep plan artifact scope story:save-card --add src/checkout.rs --add src/vault.rs
story:save-card scope set (revision 2) at …/planning/story/save-card.md
  + src/checkout.rs (cited)
  + src/vault.rs (cited)
$ aep plan artifact scope story:refund-guest --add src/checkout.rs --inferred
story:refund-guest scope set (revision 2) at …/planning/story/refund-guest.md
  + src/checkout.rs (inferred)
```

Each entry is a path plus a confidence:

- `cited`: somebody read the path in the story, a diff or a file they opened.
- `inferred`: it was worked out and not read anywhere. Pass `--inferred` to mark every `--add` in
  one command this way.

The file records it as a typed list:

```yaml
scope:
- confidence: cited
  path: src/checkout.rs
- confidence: cited
  path: src/vault.rs
```

Scope is a **story** field. Other kinds are refused, because a task inherits the scope of the story
it decomposes. Nothing is normalized: `src` and `src/checkout.rs` are two different surfaces, and
collisions are reported at whatever granularity you declared.

## Waves

```shell-session
$ aep plan artifact waves
wave 1
  story:guest-receipt
  story:save-card
wave 2
  story:refund-guest (inferred)
collision: story:refund-guest story:save-card src/checkout.rs (inferred)
unassessed: story:pay-by-card
2 wave(s), 1 collision(s), 1 unassessed
```

The rules:

- Inside one wave, no two stories share a scope path.
- A story is never in the same wave as anything it `depends_on`, and never in an earlier one.
- Every pair that shares a path is printed as a `collision`. An inferred entry **counts** as a
  collision, because placing two stories together on the strength of a guess is exactly the mistake
  this command exists to catch.
- A story with no scope is printed under `unassessed` and is **never placed**. An unassessed story
  would otherwise look exactly like a safe one.
- A `depends_on` cycle is printed with its ids, and the command exits `2`. Otherwise it exits `0`,
  because *these two cannot run together* is an answer, not an error.

`--kind` (default `story`) and `--status` narrow the selection. A `depends_on` edge that leaves the
selection is ignored.

`waves` reads and prints. It does not start anything. Choosing a wave, and deciding that a reported
collision is acceptable anyway, stay with the person running the work. See
[Run waves](../guides/run-waves.md).
