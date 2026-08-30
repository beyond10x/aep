# Wave — what the last wave found

> **Status: proposed 2026-08-30**, written against `3d86d5b`. Not dispatched. Stage 1 of the `wave`
> skill ends here and waits for the operator.

**Goal: close four defects this repository found by running itself, none of which any unit was in a
position to fix at the time. Three came out of the wave of 2026-08-30 — its adversarial pass, its
implementor's refusal to ship a pattern that only looked stricter, and three prose claims it
disproved on the way past. The fourth is the ninth defect of `story:adopter-schema-contract-tooling`,
the only one of nine that never got a test.**

Four surfaces, no two on one file, merged onto one branch and closed by one gate run.

## 1. Where this stands — measured 2026-08-30

| fact | evidence |
|---|---|
| root filesystem **74 G free of 848 G, at 91 %** | `df -h .` |
| this tree's `target/` **31 G** | `du -sh target` |
| `git worktree list` — **one entry**, the main tree | no previous wave's trees are standing |
| working tree **clean** at the start of selection | `git status --short` before the scope writes below |
| **9.0 G of build directories standing from another repository's waves** | `~/.cache/harness-wave3/target` 6.2 G, `~/.cache/harness-wt-gate-target` 2.3 G, `~/.cache/harness-wt-install-target` 479 M — all three contain `b10x-harness` artefacts, not this workspace's. Left standing: they are not this repository's to delete |
| **eight merged branches standing** — six `impl/*`, two `wave/*` | `git merge-base --is-ancestor <b> main` succeeds for all eight. Both previous waves predate the cleanup step |
| `sccache` at `/usr/bin/sccache`, wired at `~/.cargo/config.toml:60` | `RUSTC_WRAPPER` unset in the shell; the cargo config carries it |
| a package-scoped build in a fresh worktree: **30 s wall, 1.6 G target** | measured 2026-08-30 for the previous wave on this tree at `a8b139b` — `docs/plan/wave-the-driver-owes-only-tests.md:26`. **Re-measured in stage 2 before N is fixed** |
| **so N = 4 costs ~6.4 G** | 4 × 1.6 G against 74 G free |
| build directory placement: **inside the worktree** | `AGENTS.md:493-502`. The skill's own reference still says *"usually outside it"* — see § 6 |

## 2. The units

Every scope below was derived 2026-08-30 by `story-scoper`, one agent per candidate, and written
into the story's own body. Every one is **high** confidence.

| # | story | epic | serves | surface | branch |
|---|---|---|---|---|---|
| 1 | `story:unreadable-lock-refuses-its-own-escape-hatch` | `reference-driver` | **O3** *(edge added today)* | `crates/protocol-cli/src/drive.rs`, `tests/drive_cli.rs` | `impl/unreadable-lock-refuses-its-own-escape-hatch` |
| 2 | `story:workflow-id-pattern-numeric-tail` | `reference-driver` | **O3** *(edge added today)* | `crates/aep-domain/src/ids.rs`, `crates/aep-driver-spec/src/pin.rs` + `tests/published_pattern_evaluated.rs`, `schemas/generated/*.json` (4) | `impl/workflow-id-pattern-numeric-tail` |
| 3 | `story:prose-that-the-tree-contradicts` | `reference-driver` | **O3** *(edge added today)* | `crates/aep-engine/src/load.rs`, `crates/aep-engine/tests/adopting_guide.rs`, `crates/aep-driver/tests/shell_echo.rs`, `drivers/development/default.yaml`, `conformance/eval/development-honest/expectations.trace.yaml`, 2 docs | `impl/prose-that-the-tree-contradicts` |
| 4 | `story:skill-text-cannot-instruct-a-direct-store-write` | `adopter-feedback-round-1` | **O2** *(already declared)* | one **new** file under `crates/protocol-cli/tests/` | `impl/skill-text-cannot-instruct-a-direct-store-write` |

Integration branch: `wave/what-the-last-wave-found`, forked from `main` at `3d86d5b`.

## 3. Overlap, per pair — by file, not by package

| pair | packages | verdict |
|---|---|---|
| 1 × 2 | `protocol-cli` vs `aep-domain` + `aep-driver-spec` | **disjoint**. 2 rebuilds 1 through the dependency graph; no file is shared |
| 1 × 3 | `protocol-cli` vs `aep-engine` + `aep-driver` | **disjoint by file**, with one caveat below |
| 1 × 4 | **both `protocol-cli`** | **disjoint by file.** 1 edits `src/drive.rs` and `tests/drive_cli.rs`; 4 adds a new file under `tests/` and touches neither `src/` nor `Cargo.toml` (no `[[test]]` targets are declared, so the file is auto-discovered) |
| 2 × 3 | `aep-domain` + `aep-driver-spec` vs `aep-engine` + `aep-driver` | **disjoint.** Only 2 regenerates `schemas/generated/`, and `xtask schema` rewrites that directory as a set — a second unit doing so would be a whole-directory collision. None does |
| 2 × 4 | — | **disjoint** |
| 3 × 4 | — | **disjoint by file**, with one caveat below |

**Caveat on 1 × 3 and 3 × 4 — a read is not a write, and this is where it could still bite.**
Unit 3 edits `#` comments in two *shipped* artefacts: `drivers/development/default.yaml:11` and
`conformance/eval/development-honest/expectations.trace.yaml:125`. `crates/protocol-cli/src/drive.rs`,
`crates/aep-driver/tests/coverage.rs` and `crates/protocol-cli/tests/eval_*` **read** both files. No
merge conflict is possible — nothing else writes them — but a byte-comparing assertion in
`protocol-cli` would go red on the integration branch and not in unit 3's own package gate. Named
here so the first red on the integration branch is recognised rather than investigated.

**Unit 4's collision is semantic, not textual.** Its new guard reads every `SKILL.md` under
`integrations/`. No unit in this wave edits skill prose, so nothing in the wave can trip it. A later
wave that rewrites a skill can.

**One file every unit would touch, and no unit will:** `CHANGELOG.md`. The coordinator writes it.
Implementors are told not to.

## 4. What each unit owes

**1 — `story:unreadable-lock-refuses-its-own-escape-hatch`.** `read_lock`
(`crates/protocol-cli/src/drive.rs:1448`) propagates the serde error with `?`, and `take_lock`
(`:1462`) calls it at `:1510` — **before** it consults `force` at `:1512`. So `--take-lock`, the
route the refusal itself advertises, is refused by the thing that exists to override it. Four call
sites are affected, not the three the acceptance names: `start` (`:754`), `resume` (`:889`) and
`status` (`:950`) all read the lock.

**The red case is not on `main`.** `adversary_a_lock_file_that_will_not_parse_is_a_refusal_and_never_a_parse_error`
lives only on `impl/protocol-drive-verb`, at `crates/protocol-cli/tests/drive_cli.rs:2695`. `b216ce7`
removed it from `main` deliberately and its message names this story as the owner. The implementor
restores it from that branch rather than looking for it in place.

**Why this one first:** `story:protocol-drive-verb` is `active` and `depends_on` it. Nothing else in
the backlog is on an active story's critical path.

**2 — `story:workflow-id-pattern-numeric-tail`.** `WorkflowId::PATTERN` (`crates/aep-domain/src/ids.rs:235`)
is looser than `WorkflowId::new`: the constructor refuses an id whose last `.`- or `/`-separated
component is a bare integer (`ids.rs:97-108`), the published pattern does not, so `adp/2/1`,
`adp.2/1` and `adp/22/1` pass the schema and fail to load. `ProfileId` (`:227`) shares the rule.

The replacement pattern body **already exists and is corpus-checked** —
`the_numeric_tail_rule_is_expressible_as_a_pattern`, `crates/aep-driver-spec/tests/published_pattern_evaluated.rs:537`.

Two corrections to the story, both in its Scope now: `PinnedWorkflowRef::PATTERN`
(`crates/aep-driver-spec/src/pin.rs:73`) is a hand-written literal, **not** composed from
`aep-domain`, so *one line in `aep-domain`* understates the change; and four generated schemas carry
the definitions, not the one the acceptance names.

**3 — `story:prose-that-the-tree-contradicts`.** Three claims, all re-verified unchanged at `3d86d5b`.
- `crates/aep-engine/src/load.rs:29` says the `drivers` row is last because *"the workflows are
  filled in by the row above this one"*. The row above is `artifacts/lifecycles` (`:27`); `workflows`
  is `:25`. `:30-32` then explains that `Registry::validate` runs after the whole tree is read, so
  the ordering is not what makes cross-validation work — the comment argues against itself.
- *"No development profile grants `command.execute`"* is false and has been since
  `profiles/development-driven.yaml:78` shipped. `story:driver-router` corrected the two copies
  inside its own surface; the story enumerates the survivors and the Scope section re-resolved each
  one at `3d86d5b`. Three of them are shipped artefacts rather than prose — a test doc, the shipped
  driver map, a shipped conformance fixture. **The story says *seven copies* and accounts for seven;
  `git grep "no development profile"` outside the planning store returns ten hits, some of them
  already corrected inline. The count is the implementor's to settle, and the survivors are what
  matters.**
- `VENDORED` (`crates/aep-engine/tests/adopting_guide.rs:19-27`) either derives from the loader's
  table or says in one line why it does not. **Which of the two is an open decision** — deriving adds
  a third copy of the same source parser, because integration tests are separate binaries. The
  implementor decides and states the reason; it is not a question for the operator.

**4 — `story:skill-text-cannot-instruct-a-direct-store-write`.** Nothing in this repository reads a
`SKILL.md`'s content; the only code that touches one joins its path and asserts it exists
(`crates/protocol-cli/src/drive.rs:7838`). The prohibition on hand-editing planning artifacts is
prose guarded by nothing — the same shape as the defect it was written to fix. One new test under
`crates/protocol-cli/tests/`, modelled on `workflow_coverage.rs`, enumerating the five shipped
`SKILL.md` files and refusing text that instructs a direct store write. The shipped skills must pass
unmodified.

Two open forks the implementor closes, not the operator: whether `integrations/claude-code/agents/*.md`
(six files) and `skills/*/references/*.md` (two files) are also scanned, and whether the pattern set
is a `const` in the test file or a committed document under `integrations/`.

## 5. Considered and left out

| story | why |
|---|---|
| `story:a-story-records-where-it-lands` | scoped, **medium** confidence. Two unresolved forks decide whether it is one crate or four: which population the report covers (the acceptance says *non-draft*, 73 stories; the rationale describes the *draft* set, 44), and whether `validate` must genuinely load `artifacts/kinds/story.yaml` — which does not exist, and which **nothing in the tree reads**: `crates/aep-engine/src/load.rs:22-34` has no `artifacts/kinds` row and `required_sections` appears in no `.rs` file. On the deep path it edits `load.rs`, which is unit 3's file |
| `story:evidence-subject-binding` | scoped 2026-08-30, high confidence for the remaining increment (`crates/aep-domain/src/requirement.rs:377-382`, gap-register finding **F26**) — but the fix is an undecided fork between a behaviour change and a parse-time refusal, and F26 explicitly declines to require `subject` alongside `horizon`. A wave unit that has to pick a semantics for evidence gating is a decision, not an implementation |
| `story:recurrence-key` | its own Scope section says outright it is **not ready to dispatch**: no referent for *where the workflow's other outputs are declared*, an undecided store, and no incident instances in this tree to roll up |
| `story:advisory-enforcement-tier` | `docs/plan/gap-register.md` records it **closed by code 2026-08-26 — the protocol layer has the tier**, with four tests named, while the story is still `draft`. Verify-and-close, not implement |
| `story:completion-needs-evidence` | its own body marks two acceptance lines **Shipped** and carries a *Verdict — accepted in part, 2026-08-28*. What remains is a decision that was deliberately taken (report, do not refuse), not code |
| `story:wave-skill-defects-found-by-running-it` | its surface is the wave skill, which this wave is running. Most of its nine defects were landed by `ee78c9f`…`3d86d5b` and the story has not been moved |
| everything under `epic:cross-harness-portability`, `story:three-arm-pilot-2`, `story:agent-eval-cases`, `story:governed-dogfood-run`, `story:partial-edits-…` | each needs a paid run, a second harness or a live endpoint. Not implementable in this tree tonight |
| `story:two-adapters-two-paths` | the defect is in `metaharness`, a different repository |
| `story:decision-with-default`, `story:provenance-scale`, `story:governed-dogfood-run` | `depends_on` a story that is not terminal |
| `story:fanout-promote`, `story:claim-retirement`, `story:streaming-checker`, `story:per-record-horizons`, `story:task-scoped-artifact-requirements` | in-tree and real, but each is a mechanism rather than a defect. Blast radius well past one package |

## 6. Found during selection, outside every unit's surface

Filed rather than fixed. None is in scope for this wave.

1. **`AGENTS.md:384` says *"Twenty steps in `Taskfile.yml`"*; the `check:` task lists twenty-one.**
   Counted from `Taskfile.yml`: `fmt-check`, `status-check`, `plan-check`, `audit-check`,
   `version-check`, `dep-check`, `guard-check`, `claim-check`, `clippy`, `test`, `docs-check`,
   `postgres-check`, `doc-check`, `schema-check`, `generate-check`, `suite-check`, `infra-check`,
   `synth-check`, `lab-check`, `msrv`, `website`. Exactly the class
   `story:prose-that-the-tree-contradicts` exists for, and exactly what *Prose states no count of the
   gate's own suites or tests* (`AGENTS.md:94-96`) forbids one line higher up.
2. **The wave skill's cleanup step cannot delete this repository's own branches.** `SKILL.md:395`,
   `:403` and `:409` glob `wt/*`; this repository's convention is `impl/<story-slug>` and
   `wave/<name>` (`AGENTS.md:806-833`, and the skill's own
   `references/branch-and-merge.md:7-11`). Eight merged branches are standing because of it.
3. **`references/branch-and-merge.md:80` still says a build directory goes *"usually outside it"*.**
   `AGENTS.md:493-502` says inside, and `story:wave-skill-defects-found-by-running-it` § 9 records
   that the fix reverses the skill's wording. `SKILL.md` was updated by `ee78c9f`; the reference file
   was not.
4. **9.0 G of another repository's wave build directories in `~/.cache`**, dated 2026-08-30, every
   worktree they belonged to already gone. Named in § 1; the operator's to delete.
5. **Three story bodies carried stale `file:line` citations** — `ids.rs:191` (actual `:235`),
   `drive.rs:7747` (actual `:7838`), `drive.rs:1418`/`:1444-1452` (actual `:1448`/`:1462-1512`).
   Corrected in the Scope sections written today.

## 7. What approval authorises

Exactly these commits, and nothing else:

1. one opening `chore(store):` commit — the four Scope sections written today, the four `serves`
   edges, and the four `draft -> proposed -> active` moves;
2. one commit per unit, on its own `impl/*` branch (four);
3. the merges of those four branches into `wave/what-the-last-wave-found`, plus any
   `Merge main into wave/…` if the base moves;
4. one closing `chore(store):` commit — the gate's per-step exit statuses, the evidence recorded
   against the merge commit, and anything filed rather than fixed;
5. the merge of `wave/what-the-last-wave-found` into `main`.

**Not** a push, **not** a tag, **not** a release, **not** a `CHANGELOG` cut for a version, **not**
any story outside the four. The grant ends when this wave closes.
