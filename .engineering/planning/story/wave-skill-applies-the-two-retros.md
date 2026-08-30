---
format: aep.planning-md/1
id: story:wave-skill-applies-the-two-retros
kind: story
status: implemented
title: The wave skill applies the two retros written on 2026-08-30
summary: Eighteen changes named by the harness wave-3 retro and the substrate byte-plane retro of 2026-08-30; zero of them in skills/wave at 85c3e91.
tags:
- plugin
- wave
relations:
- decomposes: epic:self-evaluation
- serves: vision:O3
- informed_by: story:wave-skill-defects-found-by-running-it
revision: 4
---
# Story: The wave skill applies the two retros written on 2026-08-30

## Outcome

Eighteen changes named by `docs/reviews/2026-08-30-wave-3-retro.md` (harness wave 3) and
`~/.cache/substrate-wave/wave-retro-2026-08-30.md` (substrate wave) are in `skills/wave/SKILL.md`,
`skills/wave/references/branch-and-merge.md` and a new `references/unit-brief.md`. Neither retro had
reached the skill at `85c3e91` (`grep -c 'scratch\|tokens' skills/wave/SKILL.md` → 0).

## Context

`story:wave-skill-defects-found-by-running-it` holds the twelve defects of the first two waves, all
landed. These are the next layer, from waves 3 (harness), byte-plane (substrate) and what-the-last-wave-found
(this repository). The substrate retro ends "say the word and I'll open it as a PR there" (`6f1be0d9#138`);
nobody did.

| # | change | evidence |
|---|---|---|
| 1 | write the wave page even when approval is pre-given; the page holds per unit branch, head, worktree, build dir, scratch dir, stage, and the commits authorised; stage 2 updates it. `SKILL.md:165-168` welds the page to the stop | SUBRETRO §2; `6f1be0d9` compacted twice mid-wave with the wave's state only in context |
| 2 | a unit's record is a triple worktree / build dir / scratch dir; the coordinator assigns scratch under the wave root; cleanup removes all three | SUBRETRO §3; RETRO3 P8 (579 MB found by self-report); `4d4c15a4#537` (`.wave/` 83 MB); `9c286ad7` implementor wrote 15 files to `/tmp` |
| 3 | the close step rewrites each unit's `## Scope` from the implementor's confirmation table through `protocol artifact body` | SUBRETRO §4 (2 of 5 inferred lines wrong, still in the store) |
| 4 | one brief per unit written to a file with the repo-invariant block; corrections carry deltas | SUBRETRO §6 (~240 retyped lines, no omission detector); RETRO3 P5 |
| 5 | by-function split of a shared file when no disjoint pair exists; `git merge-tree --write-tree` dry run before the first merge | SUBRETRO §7; `114c2340#205` U2 edited U4's file |
| 6 | the attack budget records findings per pass and regressions, and names who verifies the final correction | SUBRETRO §9, addendum 2; RETRO3 P9; `114c2340` U2 4→9, U4 3→4 |
| 7 | free disk is re-read when each unit returns | SUBRETRO §10 (84 G → 62 G in one wave) |
| 8 | the closing report records per-agent tokens, tool uses, duration | SUBRETRO §1; `6f1be0d9#178` 2,025,848 tokens with nothing merged |
| 9 | a wave of one is legal | `e70b8018 s1#1512` "a wave of one is not a wave" → operator `s1#1536` |
| 10 | teardown globs `wt/*` (`SKILL.md:395,403,409`); this repository uses `impl/*`/`wave/*` | `9c286ad7#155` 8 merged branches left standing; `114c2340#86` |
| 11 | `references/branch-and-merge.md:80` "usually outside it" → inside the worktree, `AGENTS.md:493-502` | `2e81f991#199` cost one gate run |
| 12 | "same implementor" routing needs a fallback: agents vanish after `/compact` | `9c286ad7#244` |
| 13 | pre-flight names N by model budget and records worktree locations | `4d4c15a4#215` N=6 → HTTP 429, 4 agents killed, 47 min; `4d4c15a4#274` 5 worktrees deleted mid-wave |
| 14 | a fixed machine-readable header on sub-agent reports | RETRO3 P10; `114c2340#226` 278k-token report |
| 15 | the coordinator's opening commit passes the gate before dispatch | RETRO3 P3; `114c2340#138` |
| 16 | an implementor may return a patch for a file it does not own instead of editing it | RETRO3 P4 |
| 17 | a sanctioned path for the dirty-tree pre-flight override | `2e81f991#127` |
| 18 | `git merge -F -` does not read stdin; the diff base is `A...B` | RETRO3 P13/P14; `114c2340#188` |

## Acceptance

- Each numbered change is present in the named file, and a reader can find it by the number.
- `skills/wave/SKILL.md` names its own version (see `story:plugin-and-skills-carry-a-version`).
- No rule about how to talk to the operator is added; `3a91a8b` removed those on purpose.

## Out of Scope

A `wave` artifact kind (`story:wave-as-a-surface`).
