---
format: aep.planning-md/1
id: review-result:adversary-renames-round-2
kind: review-result
status: active
title: 'Adversary, round 2: profile/CLI crate renames and plugin-name references'
relations:
- reviews: story:profile-and-cli-crates-named-after-aep
- reviews: story:plugin-names-follow-agentplugins
revision: 1
---
# Adversary, round 2 — story:profile-and-cli-crates-named-after-aep and story:plugin-names-follow-agentplugins (commits 228706f, 0a58a66)

Verdict: NEEDS-CHANGE. Cases executed 2128 → 2131, red 2. Origin: introduced 5 / pre-existing 4.
Agent: `adp:adversary` (opus). Cases added (kept): `xtask/tests/live_acceptance_names_a_package_this_workspace_builds.rs`,
`crates/edge/aep-cli/tests/guide_console_blocks_are_what_the_command_prints.rs`.

Attacked, did not break: `aep trace check` over 12 spec × recording pairs against the 39c66e7 and
9607ea6 specs, byte-identical; all 9 `# recorded-under-this-name` markers name an id their recording
contains; guard mutations (a) package name left behind and (b) retired area-qualified path both fire,
(c) missing file inside a real crate does not (directory-level rule, by construction); `cargo
metadata` 23 packages with the new names only; `aep --help` and `protocol --help` byte-equal, both
`protocol 0.50.0`; every `-p <package>` and `cargo install --path` in tracked non-dated files
resolves incl. `release.yml:56,104`, `Taskfile.yml:122,147,200`, `drivers/development/default.yaml:266`;
no live scope names an absent (area, crate); an agentplugins case with `subject.skills:
[ess-specify:specify]` reaches EVAL-RUN-002 (EVAL-RUN-018 is behind `live()` and could not be driven
without spending); every install line matches agentplugins a2077d2; `xtask status/guards/deps/
version/claims/schema/notes` and `artifact validate` exit 0.

```findings
- file: .engineering/planning/story/implementor-and-adversary-agents.md
  line: 79
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    a draft story's `## Acceptance` is `cargo test -p protocol-cli --test workflow_coverage`, which
    the rename made answer "package ID specification `protocol-cli` did not match any packages",
    so the story's own completion predicate cannot be evaluated.
- file: website/docs/guides/check-a-transcript.md
  line: 131
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    the unit added a sentence certifying the page's console blocks as "quoted as it was printed",
    and all four `trace check`/`trace inspect` blocks differ from what the command prints.
- file: website/docs/guides/check-a-transcript.md
  line: 141
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: >-
    the quoted reports carry a stale transcript digest, a stale spec digest, `41 ok` against 43
    rows, and `Bash(command ~ "aep artifact")` where the recording holds `protocol artifact`.
- file: xtask/tests/crate_paths_are_area_qualified.rs
  line: 48
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    the exclusion table excuses `.engineering/planning/` on the ground that `scope:` is asserted
    by the pre-move rule, but that rule reads only pre-move spellings, so a live scope naming
    `crates/edge/protocol-cli/...` is guarded by nothing.
- file: crates/edge/aep-cli/src/eval.rs
  line: 2602
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: >-
    `RawCase`'s doc cites `tests/eval_corpus.rs` as the denier that owns the case shape, and that
    reader is `deny_unknown_fields` with no `subject` field, so it rejects the block EVAL-RUN-018
    keys on and denies nothing inside it.
- file: crates/observe/trace-spec/tests/write_selectors.rs
  line: 7
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the narration of the 2026-08-23 pilot's `git status` was rewritten to `crates/edge/aep-cli/...`,
    a path that did not exist on that date, against the commit's own rule that recorded narration
    stays as written.
- file: crates/edge/aep-cli/tests/planning_cli.rs
  line: 223
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    25 scratch-directory names that never meant the plugin were swept from `aep-planning-*` to
    `aep-plan-*`; harmless and collision-free, but it is a blanket substitution in a commit arguing
    that a name is rewritten only where it means the plugin.
- file: website/docs/reference/cli.md
  line: 510
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: >-
    the two `--plugin-dir ../agentplugins/aep-plan` examples omit the `plugins/` segment that
    agentplugins actually uses and that the repository's three other examples spell.
- file: crates/edge/aep-cli/src/eval.rs
  line: 3567
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: >-
    `preflight_child_path` returns the first refusal rather than accumulating, so EVAL-RUN-017
    masks EVAL-RUN-018 and an operator with both faults pays two live round trips.
```
