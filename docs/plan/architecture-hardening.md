# Architecture hardening wave

> **Status: accepted by the operator on 2026-08-30 and picked up on 2026-08-31.** The work order is
> the architecture review findings recorded in the eight stories under
> `epic:architecture-hardening`. The wave starts from `09ca7548430efe822f2ec665cf60eef993df2cb5`;
> concurrent changes in the main checkout are deliberately excluded and are merged into this wave
> only after they are committed.

## Goal

Close the review's failure-atomicity, durable concurrency, type-boundary, persistence-integrity,
source-immutability, dependency-layering, status-drift and pagination findings without weakening the
protocol's evidence or determinism rules. The prerequisite is an additive atomic-batch provider
contract released by `entity-runtime` as `0.14.0`; this repository consumes that one tag and is
landed without a release.

## Wave record

The operator explicitly required an isolated worktree because the main checkout carries concurrent
work. Pre-flight reported these paths there; the wave neither stashes, commits nor edits them:

```text
CHANGELOG.md
Cargo.lock
Cargo.toml
examples/billing-conformance/evidence/06-conformance-faulty.yaml
examples/billing-conformance/evidence/06-conformance.yaml
```

The integration checkout is `../aep-wt-architecture-hardening`, branch
`wave/architecture-hardening`. The upstream checkout is `../entity-runtime-wt-atomic-batch`, branch
`impl/atomic-batch-store`. Unit worktrees are created one at a time under
`../aep-wt-<story-slug>` because this session implements serially; each still
forks from the same wave-base commit and uses its own in-tree `target/`. Scratch lives below each
unit worktree's `.wave-scratch/<story-slug>/` and is removed with that worktree.

| unit | branch | objective | implementation boundary |
|---|---|---|---|
| command failure atomicity | `impl/command-failure-atomicity` | O2 | candidate memory state; one rejection audit |
| durable command batches | `impl/durable-command-batches` | O2 | upstream batch SPI; transactional adapter; authority-backed Hybrid |
| sealed ESS states | `impl/sealed-ess-states` | O2 | private validated/IR fields; compiler validation |
| eval record integrity | `impl/eval-record-integrity` | O3 | required identity/digest/expectation fields |
| driver run-state integrity | `impl/driver-run-state-integrity` | O6 | committed generations and explicit in-flight resolution |
| immutable project loader | `impl/immutable-project-loader` | O2 | loader edge crate and verified Git snapshots |
| authoritative gate status | `impl/authoritative-gate-status` | O6 | one executable gate and generated current-state regions |
| pagination cursors | `impl/pagination-cursors` | O2 | parsed cursor offsets across every query surface |

The opening `chore(store):` commit carries this selection and activates the stories. Each unit has
one implementation commit and merges serially with `--no-ff`. One complete gate run against the
merged integration commit supplies the evidence for every story; a closing `chore(store):` commit
records it, after which the wave merges to `main`. No tag, push or aep release is
authorised by this wave.

## Contract decisions

1. `entity-store` adds `AtomicCommit` and the additive `AtomicBatchStore::commit_batch` extension
   trait. Entries execute in order against transaction-local state; any error rolls the entire
   batch back. Memory, SQLite and Postgres implement it and share conformance tests.
2. The AEP entity adapter stages memory and projection state, derives expectations from the
   pre-command view, commits placements and records once, and publishes the staged state only after
   durable success. Hybrid commands and contract reads use its transactional authority; the replica
   remains an explicitly divergent projection.
3. Validated `Specification` and compiled `EssIr` fields become private. Read-only accessors replace
   direct construction, and all compiler entry points perform complete validation first.
4. Eval records require their identity, both digests and non-empty expectations. Driver snapshots
   and cursors remain separate owned documents but are exposed through one hash-verified generation.
   A persisted in-flight attempt makes resume refuse until the operator retries the same attempt id
   or records a no-verdict outcome.
5. Filesystem, environment, schema and Git acquisition move from `aep-engine` to a new edge crate.
   Pinned Git sources are materialised from verified commit objects into read-only, manifest-checked
   snapshots; symlinks and credential-bearing URLs are refused.
6. CI invokes `task check`; generated status regions replace hand-maintained current-state claims.
7. Every query parses and applies its existing `offset-<n>` cursor after deterministic filtering and
   ordering and returns the next real offset only when more matches remain.

## Verification

Each unit adds a load-bearing regression plus a one-line mutation proof. The merged wave runs focused
crate tests throughout, the runtime's complete gate before its release, dependency pin validation
after adoption, and finally every aep gate step with its own captured exit status
and output. The closing record distinguishes executed and explicitly skipped steps and cites the
exact merge commit judged.
