---
format: aep.planning-md/1
id: story:planning-skill-says-what-the-cli-does
kind: story
status: implemented
title: The planning skill says what the CLI does, in six places it did not
summary: 'Six passages of skills/planning that cost seven sessions a wrong turn: moves-are-proposals, 7 of 18 verbs listed, evidence never named, derived_from, typo-by-hand, a blocker kind that may not exist.'
tags:
- plugin
relations:
- decomposes: epic:self-evaluation
- serves: vision:O3
- informed_by: task:decomposes-edge-examples
revision: 5
---
# Story: The planning skill says what the CLI does, in six places it did not

## Outcome

Six passages of `skills/planning/SKILL.md` and its references match the CLI and the operator's standing
rules. Each one below cost at least one session a wrong turn.

## Context

Audit of 2026-08-30 (`SYNTHESIS.md` PL-2), seven sessions. Verified on disk at `85c3e91`:

| # | passage | defect | evidence |
|---|---|---|---|
| a | `SKILL.md:129` "Status moves … are **proposals** until confirmed" | the operator rejected the behaviour it produces, four times | `431986de#6960` "is it implemented or not? if yes, why do you ask me?"; `9da4f51c` 19/19 stories left stale; `57338989#130`; `2e81f991#399` |
| b | `SKILL.md:23-29` discovery table lists 7 of 18 `artifact` verbs | `show`, `history`, `explain`, `evidence`, `validate` absent | `11727595` #5; `ed007513` reached for `show` 18× before it existed; `9da4f51c` 35 raw store reads |
| c | `artifact evidence` is named 0 times | evidence gates every status move; the model guessed a kind and was refused | `11727595#3407` (`measurement`); `431986de#6957`; `e70b8018 s1#694`; `ed007513#1199` (`--reference` typo, 4 stories closed on an assertion) |
| d | `derived_from` at `SKILL.md:195,200`, `agents/decomposer.md:49`, `agents/reverse-engineer.md:52`, `references/store-conventions.md:103` | 39 of 39 stories use `decomposes` | `task:decomposes-edge-examples` |
| e | `references/store-conventions.md:70` "correcting a typo by hand is harmless" | contradicts guardrail 2 | fixed on `impl/skill-text-cannot-instruct-a-direct-store-write` (`f07db4f`), not on `main` |
| f | guardrail 6 assumes the store declares a blocker kind | `kinds` lists none; `blocked` answers "nothing is blocked" | `431986de#7024` "what are you talking about blockers"; `11727595#3566`; `fcf5873a#361` |

## Acceptance

- (a) says: a move is made when the store holds the evidence the rung requires; the coordinator asks only
  when the evidence is missing and names what is missing.
- (b) lists every read verb `protocol artifact --help` prints, in the table's question/answer form.
- (c) names `protocol artifact evidence <id> --kind <k> --source <s> [--ref …]`, says the kind list is
  closed and how to read it (`protocol artifact evidence --help`), and that `move` finds recorded
  evidence without being told.
- (d) every example edge reads `decomposes:epic:…`.
- (e) the line is gone from `main`. The correction is *no verb changes a title after creation*
  (`replace_body` writes only the body, `crates/protocol-cli/src/planning.rs:1952`); `body` is not
  the fix, `set` (`story:body-edits-have-a-verb`) is.
- (f) guardrail 6 opens with "if `protocol artifact kinds` lists no blocker kind, file the blocker as a
  `decision-blocker` where the ladder exists, otherwise record it in the story's body and say so".
- The skill's `description` names adoption ("adopt", "migrate", "replace the track plugin", a repository
  with no `.engineering/`) so § 5 is in context before a store is hand-populated
  (`e70b8018 s1#91-#196`: 13 artifacts written 3 min before the skill loaded).

## Out of Scope

The CLI-side fix for (f) — `story:blocker-kinds-are-discoverable`.
