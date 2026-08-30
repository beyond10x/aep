---
format: aep.planning-md/1
id: story:plugin-and-skills-carry-a-version
kind: story
status: implemented
title: The plugin and its skills carry a version a session can quote
summary: plugin.json read 0.1.0 across 9 commits and 968 lines; a skill edited mid-wave halted the wave and /reload-skills said no changes. Six sources.
tags:
- plugin
- wave
relations:
- decomposes: epic:self-evaluation
- serves: vision:O3
- informed_by: story:wave-skill-defects-found-by-running-it
revision: 4
---
# Story: The plugin and its skills carry a version a session can quote

## Outcome

A coordinator can say which plugin text it is running, and a stale load is a refusal rather than a
surprise. `integrations/claude-code/.claude-plugin/plugin.json` changes version whenever anything under
`integrations/claude-code/` changes, every skill states the version it belongs to, and the wave skill's
stage-1 report prints it.

## Context

Audit of 2026-08-30 (`~/.cache/ep-session-audit/SYNTHESIS.md` PL-1), six independent sources:

- `plugin.json` read `0.1.0` across 9 plugin commits and 968 changed lines since the installed commit
  `422966a` (`git diff --stat 422966a..HEAD -- integrations/claude-code`).
- The marketplace install is disabled (`~/.claude/settings.json:47`); every skill load in every session
  came from the checkout via `--plugin-dir` (27 `planning`, 9 `wave` loads).
- Session `11727595`: installed cache carried a 203-line planning skill, "Five guardrails", 3 agents;
  the tree had 241 lines, "Six", 6 agents.
- Session `e70b8018` (substrate): 13 h on a stale snapshot, 23 of 24 dispatches were `general-purpose`
  because `wave`/`implementor`/`adversary`/`story-scoper` did not exist in the loaded copy.
- Session `cc946bc3#515`: `Agent type 'engineering-protocols:implementor' not found` — a charter written
  in the session was undispatchable until `/reload-plugins`.
- Session `114c2340#130` and `docs/reviews/2026-08-30-wave-3-retro.md:12-28`: `3d86d5b` landed 18 min
  into a running wave; the loaded copy lacked the commit-authorisation section; `/reload-skills` said
  "no changes"; the wave halted at the merge boundary after 5 commits.

## Acceptance

- A gate step (`plugin-check`, `cargo xtask plugin`) fails when `integrations/claude-code/**` differs
  from the newest release tag and `plugin.json`'s `version` does not, and when any `SKILL.md`'s stated
  version differs from `plugin.json`'s.
- Each `SKILL.md` opens with a line naming its version; `skills/wave/SKILL.md` stage 1 tells the
  coordinator to print that line in the proposal.
- `skills/wave/SKILL.md` says: a charter or skill edited during a session is not what the session runs
  until `/reload-plugins`; reload before the next dispatch, and name the version you reloaded to.
- `integrations/claude-code/README.md` § Install states which of the two load paths is supported and
  what a stale marketplace install looks like.

## Out of Scope

Publishing releases to the marketplace; a hash-based check that reaches into Claude Code's loader.
