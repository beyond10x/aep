---
format: aep.planning-md/3
id: story:a-transition-executor-reads-on-every-path
kind: story
status: rejected
title: A transition's executor reads on every path of a current aep
tags:
- planning-store
relations:
- serves: vision:O2
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T16:38:54Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "rejected", at: "2026-10-30T16:38:54Z", actor: "human:timo", revision: 3}
---
# Story: A transition's `executor` reads on every path of a current aep

## Outcome

A story moved with `--executor` (a consumer writes `executor: "agent:<name>"` into the transition when
it takes a story) is read by every planning verb of the current aep release, so a repository whose CI
validates its store with a current aep accepts the consumer's write.

## Context

A downstream repository's CI job "planning validate" failed with `transitions[1]: unknown field
executor` on a pull request whose store write came from a consumer taking a story. That job built aep
at `18a18a3f`. `git describe --tags 18a18a3f` prints `0.63.1-13-g18a18a3f1`. The field was added by
`267e4f5b2` ("feat: a transition records who executed the move and its correlation", 2026-09-28),
first released in 0.65.0 (`git describe --tags --contains 267e4f5b2` prints `0.65.0~2^2`). The
repository's CI pin was moved by hand afterwards and the check went green.

## Finding

Runtime probe, aep 0.78.0, on a scratch copy of a one-story store: `aep plan artifact new story
probe-executor`, then `aep plan artifact move story:probe-executor --to proposed --executor agent:takt
--correlation probe-1` wrote

    - {from: "draft", to: "proposed", at: "2026-10-30T16:35:00Z", actor: "human:timo", revision: 2, executor: "agent:takt", correlation: "probe-1"}

and each of these then exited 0 on that store: `aep plan artifact validate`, `show --format json`,
`list --format json`, `graph --format json`, `blocked`, `waves --kind story --status proposed --format
json`, `findings`, `aep plan audit --planning .engineering/planning`, `aep plan entity list
--planning .engineering/planning`, and a further `move --to draft` followed by `validate`.

So the rejection is an older pin's (before 0.65.0), not a defect of the current aep. No code change.
Which aep a consumer's write must suit is the consumer's to decide.

## Acceptance

The finding above is recorded; the story is closed as rejected with no code change.
