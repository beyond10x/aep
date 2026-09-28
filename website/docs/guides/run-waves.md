---
title: Run waves
sidebar_position: 4
description: Scope stories, derive which can be implemented at the same time, work one wave, and close it on evidence.
---

# Run waves

A **wave** is a set of stories that can be implemented at the same time without two of them landing
on the same file. This guide goes from a backlog of proposed stories to one closed wave. How
[waves are derived](../concepts/waves.md) is covered separately.

## 1. Scope every candidate

`waves` places only stories that declare a scope. For each story, record the paths it will change,
marking guesses as guesses:

```bash
aep plan artifact scope story:save-card --add src/checkout.rs --add src/vault.rs
aep plan artifact scope story:guest-receipt --add src/receipt.rs
aep plan artifact scope story:refund-guest --add src/checkout.rs --inferred
```

A `## Scope` section in the story body is what a person reads. The `scope` field is what `waves`
computes from. Keep both.

Record ordering that is not about files as `depends_on`:

```bash
aep plan artifact relate story:refund-guest depends_on story:guest-receipt
```

## 2. Derive the waves

```shell-session
$ aep plan artifact waves --status proposed
```

Read the output in this order:

1. **`unassessed:`** These stories have no scope, so they are never placed. Scope them, or leave
   them out on purpose.
2. **`collision:`** These pairs share a path. An `(inferred)` collision is still a collision. You can
   decide to run a pair together anyway, but make that decision explicitly.
3. **`wave 1`**: the stories that can start now.

A `depends_on` cycle makes the command exit `2` and prints the cycle. Break it before you go on.

## 3. Start the wave

Move each story in wave 1 to `active`, so the board shows what is in flight:

```bash
aep plan artifact move story:guest-receipt --to active --via
aep plan artifact move story:save-card --to active --via
```

Work each story in its own branch or worktree. Planning writes are safe across linked worktrees of
one repository: the writer lock lives in the shared Git directory, and each story's writes touch only
that story's file.

## 4. Close each story on evidence

When a story's checks pass, record the result against that story and move it. The `story` lifecycle
refuses `implemented` without a `test_result`:

```bash
aep plan artifact evidence story:guest-receipt --kind test_result \
  --source "cargo test -p receipt" --ref "$CI_RUN_URL"
aep plan artifact move story:guest-receipt --to implemented
```

If the story was reviewed, record the review and its outcome as in
[Review with findings](./review-with-findings.md).

## 5. Merge and check

Merge the story branches. Plan changes on different stories are different files, so they merge
without conflict. Then run:

```bash
aep plan artifact validate
aep plan artifact waves --status proposed     # the next wave
```

## With agents

The same loop works when each story goes to a coding agent. The
[agent plugins](https://beyond10x.github.io/agentplugins/) ship a wave workflow that proposes a
wave, dispatches one implementer per story into its own worktree, sends each result to an
adversarial reviewer, and merges what passes. AEP itself only answers what can run together and
what each story still needs. Choosing the wave stays with the operator.
