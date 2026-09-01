# Session audit, 2026-08-30 — what 32 Claude sessions and 15 review documents said about the plugin and the CLI

Branch `wave/session-audit-2026-08-30`, worktree `~/.cache/ep-wt-audit/tree`, base `main` at `85c3e91`.
Source: `~/.cache/ep-session-audit/SYNTHESIS.md` (per-source findings under `findings/`, 11 files,
387 findings; every citation is `<session-id>#<render turn>` from `render.py`).

## What was read

| source | count |
|---|---|
| session transcripts under `~/.claude/projects/-home-operator-beyond10x*` | 32 files, 15 with plugin or `protocol` use |
| sub-agent transcripts inside those sessions | 300+ |
| `docs/reviews/*.md` | 14, plus the substrate wave retro in `~/.cache/substrate-wave/` |
| planning store, for what was already tracked | 167 artifacts, 64 relevant, 41 open |

## Stories filed (22), and what happened to each on this branch

| story | state after this branch | where the change is |
|---|---|---|
| `plugin-and-skills-carry-a-version` | implemented | `plugin.json` 0.2.0; `**Skill version**` line in 3 skills; `cargo xtask plugin` / `plugin-check`; README § Install |
| `plugin-roster-is-data` | implemented | `integrations/claude-code/roster.json`, checked by `plugin-check` |
| `planning-skill-says-what-the-cli-does` | implemented | `skills/planning/SKILL.md` § 2, § 3 guardrail 6, § 4; `references/store-conventions.md:70`; `decomposes:` everywhere |
| `wave-skill-applies-the-two-retros` | implemented | `skills/wave/SKILL.md` (+212/−21), `references/branch-and-merge.md`, new `references/unit-brief.md` |
| `adversary-names-whose-defect-it-is` | implemented | `agents/adversary.md` (+96/−10), README row |
| `implementor-counts-what-ran` | implemented | `agents/implementor.md` (+104/−6) |
| `skill-loads-before-the-store-is-touched` | implemented | planning skill `description`; wave skill dispatch line |
| `fan-out-survives-a-rate-limit` | implemented | `skills/wave/SKILL.md` pre-flight and 429 recovery |
| `one-session-per-checkout` | implemented | `AGENTS.md` § Branches and waves; wave skill pre-flight |
| `release-is-a-checklist-with-a-check` | implemented | `AGENTS.md` § Releases; `task install`, `task release-check` (`cargo xtask release`) |
| `agents-md-is-generated-where-it-counts` | implemented | `AGENTS.md` § Gate generated region, owned by `cargo xtask status` |
| `gate-runs-in-a-worktree` | implemented | `AGENTS.md` § Gate (wrapper exit, no lib target); `-buildvcs=false` had landed in `fab1d73` |
| `body-edits-have-a-verb` | implemented | `body --append` / `--section`, `show --body-only`, `set` |
| `validate-strict-refuses-what-it-reports` | implemented | `validate --strict`; `move` runs the would-be graph; empty `--from` refused |
| `refusals-name-what-to-do-next` | implemented | kernel panic → `Verdict::Undecidable`; refusal text names the verb; `explain` `next:` lines; nearest evidence kinds |
| `blocker-kinds-are-discoverable` | implemented | `kinds` lists declared ladders and the `<type>-blocker` family; `blocked` says when none exists |
| `one-spelling-for-an-edge` | active — `relate rel:id` landed; `unrelate` needs `aep-backend-markdown` first | body names the three edits |
| `cli-ergonomics-round-2` | active — items 1, 3 (`list`/`show`), 5 landed | body names the rest |
| `a-stale-binary-refuses-itself` | draft — `task install` landed; journal version stamping not | — |
| `drive-watch-is-a-verb` | draft | — |
| `gate-lanes-count-what-ran` | draft | — |
| `journal-carries-digests-not-bodies` | draft | — |

## Commits this branch authorises

None yet. Everything is uncommitted in the worktree; the operator decides the commit shape. The
convention (`AGENTS.md` § Branches and waves) is one `chore(store):` opening commit carrying the 22
stories, the code and plugin changes, and one `chore(store):` closing commit carrying the gate record.

## Found on the way, filed rather than fixed

- `set --untag` that empties the tag set makes `validate` report drift: `instance_of`
  (`crates/aep-backend-markdown/src/provider.rs:197`) omits empty `tags`/`relations` from the event,
  so *emptied* is indistinguishable from *never set*. Recorded in `story:body-edits-have-a-verb`'s
  scope for the next pass.
- `task release-check` against `0.33.0`: four `ok`, one `MISSING` — no `test_result` in the store
  names the tag's commit `304d198`.
