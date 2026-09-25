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
| u37 | `wave-20260925-u37` | `b10x-target/aep-wave-20260925-u37` under the user cache | `aep-wave-20260925/u37-scratch` under the user cache | dispatched |
| u38 | `wave-20260925-u38` | `b10x-target/aep-wave-20260925-u38` under the user cache | `aep-wave-20260925/u38-scratch` under the user cache | dispatched |
| integration | `wave-20260925-issues-37-38` | `b10x-target/aep-wave-20260925-int` under the user cache | — | open |

## 4. Commits this wave makes

One commit per unit, the merges into `integrate/wave-20260925-issues-37-38`, this opening store
commit and one closing store commit. No merge into `main`, no tag, no release. The pull request is
opened only after the operator has reviewed the integration branch.
