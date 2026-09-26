# Wave — the two open GitHub issues (#37, #38)

> **Status: approved 2026-09-25 by the operator in session**, written against `9ea1f28e1a`
> (`origin/main`). Stage 1 waived by the operator's request; this page is kept current through
> stage 2. **Close differs from the skill default:** the integration branch is reviewed with the
> operator, then delivered as **one pull request** — it is not merged into `main` by the wave.

## 1. Pre-flight — measured 2026-09-25

| fact | evidence |
|---|---|
| root filesystem 70 G free of 848 G, 92 % | `df -h /` after creating the unit trees |
| 15 managed trees from earlier sessions standing, none eligible for cleanup | `worktree gc --dry-run`: 4 `worktree-dirty`, 6 `no-remote-recovery-proof`; left standing |
| `aep --version` | `aep 0.59.3` |
| `.engineering/.aep-planning-writer*.lock` already gitignored | `.gitignore`; the files still block worktree cleanup (#37) |

## 2. The units

| unit | story | issue | serves | surface | branch |
|---|---|---|---|---|---|
| u37 | `story:planning-writer-fence-leaves-no-lock-file` | #37 | O2 *(edge added today)* | `crates/edge/aep-cli/src/planning_writer_fence.rs`, one new test file under `crates/edge/aep-cli/tests/` | `impl/fence-leaves-no-lock-file` |
| u38 | `story:review-findings-accept-prose-and-json` | #38 | O2 | `crates/plan/aep-backend-markdown/src/findings.rs`, `crates/edge/aep-cli/src/planning.rs`, `crates/edge/aep-cli/tests/planning_cli.rs`, `website/docs/reference/cli.md` | `impl/review-findings-prose-json` |

No file is shared between the units. `CHANGELOG.md` is the coordinator's. Selection path: pairwise
reading by the coordinator; `aep plan artifact waves` was not run. u37 scope `cited` (story body);
u38 scope `cited`, confidence medium (coordinator `rg`, no scoper agent).

Dispatch types: `aep:implementor`, then `aep:adversary`, per unit.

## 3. Unit records

| unit | managed worktree id | build directory | scratch root | stage |
|---|---|---|---|---|
| u37 | `wave-20260925-u37` | `b10x-target/aep-wave-20260925-u37` under the user cache (deleted) | `aep-wave-20260925/u37-scratch` under the user cache | merged `ff1f8e4ba8` (unit `bd4243711d`) |
| u38 | `wave-20260925-u38` | `b10x-target/aep-wave-20260925-u38` under the user cache (deleted) | `aep-wave-20260925/u38-scratch` under the user cache | merged `503a5bc82b` (unit `1a6a27f83b`) |
| integration | `wave-20260925-issues-37-38` | `b10x-target/aep-wave-20260925-int` under the user cache (deleted) | — | gated; awaiting operator review |

## 5. What happened

| unit | adversary pass 1 | correction | second pass |
|---|---|---|---|
| u37 | 2 introduced (legacy in-tree fence not contended; read-only `.git` refuses) | legacy names locked when they already exist, never created; read-only case stated | none — coordinator applied the implementor's lint-only patch to the adversary file and re-ran fmt, clippy and the adversary lane (6 passed) |
| u38 | 4 introduced, 1 pre-existing (control characters, parser positions, escaped values, nested fences) | all five fixed | none — coordinator verified no assertion was dropped; free disk was under the 10G floor |

Gate on the integration branch, one exit status per step: 16 steps, `test` 2274 passed / 0 failed.
`postgres-check` exit 0 is a skip (`ENTITY_POSTGRES_URL unset`). `audit-check` first exited 201
because the `protocol` binary on `PATH` was 0.59.2; re-run with the tree's own 0.59.3 binary first
on `PATH`: 61 pass, 0 fail.

Environment findings, not defects of this wave: `store_writer_control` fails 31/32 under a private
`TMPDIR` below the user cache (passes under the default one); `planning_cli` scratch names are fixed,
so two concurrent runs collide.

## 4. Commits this wave makes

One commit per unit, the merges into `integrate/wave-20260925-issues-37-38`, this opening store
commit and one closing store commit. No merge into `main`, no tag, no release. The pull request is
opened only after the operator has reviewed the integration branch.
