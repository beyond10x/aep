# AGENTS.md — AEP

The contract for changing this repository. Read it before changing anything. What AEP is and how
to use it is in [`README.md`](README.md) and the [documentation](https://beyond10x.github.io/docs/aep/).
Organization-wide rules live in `atlas/AGENTS.md`; a change to bytes another repository verifies is
a coordinated migration with an Atlas ADR.

## Serves

The objectives are those of `atlas/ROADMAP.md`.

- **O2 — decisions as data, with evidence.** Planning status, capability and completion are decided
  by the engine from validated documents and recorded evidence.
- **O3 — any harness, observed and compared.** The driver and transcript checking hold any harness
  to the same step map and trace contract.

A change that advances neither is a question for the operator, not an inferred task.

## Normative documents

| Subject | Authority |
|---|---|
| protocol semantics | `docs/design/consolidated-design-v0.2.md` and `docs/design/reconciliation-v0.2.md` |
| planning store (`aep.project/5`) | `docs/design/git-native-planning-store-v0.1.md` |
| artifact completion evidence | § 10.1 of `docs/design/story-completion-evidence-design-v0.1.md` |
| transcript checking | `docs/design/transcript-conformance-design-v0.1.md` |
| driver and harness boundary | `docs/design/harness-planning-and-driver-design-v0.1.md` |
| open work and acceptance | `.engineering/planning/`, plus the live pages listed in `docs/plan/README.md` |
| delivered releases | `docs/status.md`, generated from annotated tags by `cargo xtask status` |
| the `aep plan reverse init` and `aep plan store migrate git` surface | the ESS specification `ess/`, projected into `generated/ess/` by `cargo xtask ess` |

Every design's status is in `docs/design/README.md`. A design is proposed until a plan or planning
artifact accepts it; a later accepted decision overrides earlier prose. Pages under
`docs/plan/archive/`, `docs/design/archive/` and `docs/reviews/` are dated records: do not rewrite
them.

## Repository boundaries

### Areas

Every crate lives at `crates/<area>/<crate>`: `govern` (domain, engine), `plan` (contract,
conformance, client, backends), `drive` (driver, step maps, rendering), `observe` (trace checking,
ESS evidence adapter), `profile` (development and operations vocabulary), `edge` (schemas,
filesystem and Git acquisition, the `aep` CLI). `xtask` is the build tool and has no area.

A crate compiles against its own area and the ones under it: `edge` → `{profile, drive, observe}` →
`{govern, plan}` → `aep-domain`. The one compiled exception is `aep-engine` → `aep_contract::command`
(`crates/govern/aep-engine/src/trail.rs`). Test-only dependencies are outside the rule. Enforced by
`xtask`'s `layout_tests` (run by `test`) and `xtask/tests/crate_paths_are_area_qualified.rs`.

The engine decides from caller-supplied documents and evidence; only `edge` crates do IO.

### Entity Runtime

AEP depends on `entity-runtime` (`entity-core` decides every lifecycle move); no Entity Runtime
manifest names an AEP crate. Every `entity-*` crate is pinned once, at one revision, in the root
`Cargo.toml`. `dep-check` (`cargo xtask deps`) refuses two versions or two pins of an `entity-*`
crate, any compiled `ess-*` modeling crate, and any event-log crate (`eventlog-*`,
`entity-eventlog`). Changing the dependency direction is a coordinated migration.

### ESS

ESS is a sibling repository with no AEP dependency. Core AEP must not compile against ESS modeling
types (`dep-check`). Only `aep-ess-evidence` reads the standalone ESS conformance report, and it
refuses unknown fields and contradictory totals (its tests, under `test`).

### Agent plugins, Harness and Metaharness

Skills, agents and marketplace manifests live in the sibling `agentplugins` repository; this
repository carries none. AEP has no dependency on Metaharness or Harness. Model execution, native
hooks and live plugin evaluation belong to `metaharness aep drive`; an `aep` invocation of a
model-backed step map refuses before allocating a run and names that command. `aep drive eval run
--stream` spends nothing. Plugin inputs are explicit directories or exact marketplace pins; never
install a plugin or guess a path under this checkout.

### Public source

Public source and public technical history only. No private paths, private identities,
credentials, signing keys, private policy values or a public denylist. Enforced by the Gates hooks
(see *Releases and commits*).

## Invariants

Each invariant names what enforces it. A rule without a check is not an invariant. Cite an
invariant by its name, never by its number: the numbers move when a rule is added or retired, and
`guard-check` refuses a numbered citation outside dated records.

1. **Rust types are the source of truth.** `schemas/generated/` is written only by
   `cargo xtask schema`; `schema-check` refuses changed and orphaned files.
2. **Parse, then validate.** Raw document types deserialize; validated types are built only by
   validation, and closed formats refuse unknown fields. `tests/invariants.rs` in `aep-domain` and
   `aep-driver-spec` (`test`).
3. **Validation accumulates.** Independent defects are reported together with stable codes and
   paths; tests assert variants or codes, never only `is_err()` (`test`).
4. **Decisions are deterministic.** Ordered collections, caller-supplied time; no ambient clock,
   randomness, filesystem, environment or network in deterministic cores. `tests/determinism.rs`
   banned-token scans in `aep-domain`, `aep-client`, `aep-driver-spec`, `aep-driver`, `aep-render`
   (`test`).
5. **Unknown differs from false.** A missing observation cannot satisfy a predicate and is not
   rewritten as a contradiction (`test`).
6. **Capability decisions default to deny.** A denial cannot be granted back by a later layer;
   resolution and explain tests cover conflicts and provenance (`test`).
7. **Refusals change nothing.** A failed command or move writes nothing. The `aep-conformance`
   suites, `aep-backend-entity/tests/atomic_commands.rs` and
   `aep-backend-memory/tests/failure_atomicity.rs` (`test`).
8. **Audit is append-only.** A move appends one entry to the artifact's `transitions`; archive and
   supersede replace deletion. `aep plan artifact validate` refuses transitions that are not
   continuous, do not end in the artifact's status or, for moves made in this layout, do not follow
   its lifecycle (`plan-check`); the conformance `audit` and `immutability` suites (`test`).
9. **Planning status is decided as data.** `entity-core` evaluates validated lifecycles; AEP has no
   generic status setter and one write path. `aep-contract/tests/write_surface.rs` (`test`).
10. **The ESS adapter is optional and narrow.** No core manifest depends on an ESS crate
    (`dep-check`, adapter tests).
11. **Plugin authority is explicit.** No repository-local fallback chooses a plugin; launch records
    keep the operator-supplied directories (`aep-cli` drive tests, `test`).
12. **Public APIs are documented and unsafe is forbidden.** Workspace lints, raised to errors by
    `clippy` and `doc-check`; every member opts into workspace lints.
13. **The gate is offline except by an opted-in name.** No check calls a model or spends money.
    `postgres-check` reaches only `ENTITY_POSTGRES_URL` when set and prints that it skipped
    otherwise. Cargo and npm may fill their caches on a cold machine.
14. **A guard is mutation-tested before it is trusted.** Break the guarded condition, observe the
    named failure, restore it, and run the passing test. Reviewed, not automated.

## Gate

```console
task check
```

Read the command's own exit status; never pipe the authoritative run through a command whose exit
status replaces it. CI and the release workflow delegate to `task check` (`status-check` refuses
drift). A change under `website/` is exercised by the `website` step.

Run the gate that decides a merge or a release the way CI builds: with the Cargo target directory
inside the checkout (the default `target/`, or `CARGO_TARGET_DIR=$PWD/target`). CI's fixtures then
live inside the repository's Git work tree, and a test whose answer depends on that passes outside
it and fails in CI; it happened twice in 0.64.x (a crate-path scan over untracked evidence, and
history checks over a fixture store under `target/`).

<!-- generated:gate-steps:begin — do not edit; run `cargo xtask status` -->
`task check` runs **17 steps**, in this order: `fmt-check`, `status-check`, `plan-check`, `audit-check`, `version-check`, `dep-check`, `guard-check`, `claim-check`, `clippy`, `test`, `docs-check`, `postgres-check`, `doc-check`, `schema-check`, `ess-gate`, `msrv`, `website`.
<!-- generated:gate-steps:end -->

What the other steps refuse: `status-check` a stale generated region (this list, `docs/status.md`,
the website's currency line); `version-check` a workspace version that differs from the newest
tag; `guard-check` a test body duplicated across crates, or a comment or document citing an
invariant by number; `claim-check` a released `### Fixed` entry naming something absent at the
previous release; `docs-check` a CLI verb missing from `website/docs/reference/cli.md`;
`ess-gate` an `ess/` specification that does not validate under its pinned `ess`, a projection
under `generated/ess/` that differs from a fresh one (`cargo xtask ess` rewrites them), and open
questions or synthesis counts other than the ones `xtask/tests/ess_gate.rs` records. Prose
states no count of tests; the gate output is the only place that count belongs.

`audit-check` and `plan-check` run the `.engineering/checks` suite and the planning validator
through `aep`; the checks use `AEP_BIN` when it is set and executable. Set it to a build of this
tree (`AEP_BIN=<target>/release/aep task check`) so an older installed `aep` is never what runs.

## Planning store

The plan is `.engineering/planning/`, an `aep.project/5` store: one Markdown file per artifact
(`aep.planning-md/3`) is the authority, moves are recorded in its `transitions` front matter, each
evidence record is one immutable file under `.engineering/evidence/`, and Git is the history.

1. Write only through the CLI: `aep plan artifact new | relate | unrelate | body | set | scope |
   move | evidence`. Each write changes one artifact file, plus one evidence file for `evidence`.
2. Never hand-edit machine-owned front matter (`format`, `id`, `kind`, `status`, `revision`,
   `relations`, `transitions`) and never edit or delete an evidence file. Retire an artifact with
   `move --to archived`, never `rm`.
3. A status move is a claim about project state. Propose it unless the operator asked for that
   exact move.
4. A refusal is an answer: relay the legal moves it prints; do not route around it.
5. After a batch of writes, run `aep plan artifact validate` and relay its output verbatim.
   `plan-check` runs the same validator.
6. When the operator asked for planning work, an already-satisfied or invalid request still gets an
   artifact recording the finding.

## Conventions

- Anything executable added here is Rust, with `clap` derive for command lines. Existing shell
  checkers are legacy: changing their behaviour means replacing them.
- Tests are named for behaviour and assert the reason for failure.
- Comments explain why; public docs explain what a type is for.
- Prefer no new dependency; justify each one beside its manifest entry. `aep-domain` takes no IO
  dependency.
- `CHANGELOG.md` gains an Unreleased entry for every user-visible change. Rationale goes in the
  commit message or `docs/design/`, not the changelog.
- Preserve unrelated work in dirty trees. Use a dedicated worktree; a shared Cargo target is fine
  for builds, never for generated source — run generators (`cargo xtask status`, `cargo xtask
  schema`) only in the worktree that owns their output.
- When another agent is integrating this repository, stop at a clean handoff. Do not merge,
  rebase, publish or rewrite shared state without explicit authority.

## Releases and commits

Every commit, push, tag and GitHub write is `b10x-bot[bot]`'s: `b10x-gates bot -- <git command>`
for Git, `b10x-gates api` for REST writes. Install the hooks with
`b10x-gates --repository beyond10x/aep install`; they scan the index, names, messages, metadata,
tags and every outgoing commit, and the shared Gates check is required before a merge. Private
policies and signing keys stay outside this repository. Commit titles use `feat:`, `fix:`, `docs:`, `refactor:`, `test:` or
`chore:`, then a blank line and a body; ticket references go in a trailing `Refs:` line.

Never update a pull request's branch through GitHub (`PUT …/pulls/<n>/update-branch` or the
"Update branch" button). It writes a GitHub-committed "Merge branch 'main'" commit that is not the
merge commit of any pull request, and once it reaches `main` Gates refuses every later bot push
with `merged pull request missing or ambiguous` until the repository's baseline in the private
policy is advanced (done for 412a2f2 and 938320e). To bring a branch up to date, create a new
branch from `origin/main` and replay its changes as a new bot commit (`git cherry-pick
--no-commit`, then `b10x-gates bot -- commit`); the bot runs only commit, tag, push and fetch.

Tags are bare semantic versions (`0.64.0`). A release:

1. On a `release/<version>` branch from current `origin/main`: set the workspace version in
   `Cargo.toml` (and `Cargo.lock`), and turn `## [Unreleased]` into `## [<version>] — <date>`.
2. Commit `chore: release <version>` and create the annotated tag locally:
   `b10x-gates bot -- tag -a <version> -m "aep <version>"`.
3. Run `cargo xtask status` (it derives `docs/status.md` and the website currency from the tag),
   fold its output into the release commit, and re-create the tag on that commit.
4. Full gate on that tree: `AEP_BIN=<target>/release/aep task check`, exit 0.
5. Push the branch and the tag, open the release PR through `b10x-gates api`, and merge it through
   the bot with a merge commit (never a squash) once the required checks are green, so the tagged
   commit is on `origin/main`. If a fix lands on the branch first, move the tag to the branch
   head.
6. Verify the `Release` workflow run for the tag succeeded and the GitHub Release carries the four
   `aep-<version>-<target>.tar.gz` archives and `SHA256SUMS`.
7. Record the release on a `plan/release-<version>-record` branch:
   `aep plan artifact evidence epic:planning-on-entity-runtime --kind test_result --source "release
   <version>: tag commit <sha>, Release run <id>; task check exit 0" --ref <Release run URL>`,
   committed first; then run `cargo xtask status` and commit its output. The pins move only once a
   commit after the release exists, so a `status` run before the record commit changes nothing and
   `status-check` fails on the PR (0.65.0's record PR did). Merge it the same way.
8. `cargo xtask release` (`task release-check`) prints 6/6 `ok`: version, tag on `origin/main`,
   changelog heading, pushed tag, GitHub Release, and a planning-store `test_result` naming the
   tag's commit. Only then is the release done; see *Release completion* below.

<!-- b10x-docs-operations:start -->
## Public documentation operations

This repository owns the public source and presentation allowlist in `b10x.docs.yaml`. The generated credential-free `.github/workflows/b10x-docs-bundle.yml` passively packages only those declared files for the exact successful `main` commit; it must never run repository code. The generated `.github/workflows/b10x-docs-check.yml` runs the publisher's per-source checks on every pull request and main push, with read-only contents and no credentials; it is deliberately separate from the shared gate, which runs on `pull_request_target` with a secret and never reads candidate source. Atlas selects the latest successful bundle with every other catalog source, and Website plus Docs System own rendering, shared components, search, and feeds. Do not add a standalone docs deployer or put App credentials in this public repository. If Atlas catalogs a former Pages workflow, that file remains repository-owned validation: preserve its bespoke checks while keeping exact read-only permissions, an unconditional pull-request trigger, and no deployment primitives. Project Pages at `/aep/` is only the generated stable redirect façade in `.github/workflows/b10x-docs-pages.yml`; content-only publication never rebuilds it.

From the complete organization workspace, verify the contract with a clean Atlas checkout at the current remote `main`. Set `B10X_ATLAS_CHECKOUT` to a managed Atlas worktree when the primary checkout is dirty or stale; never infer command availability from the primary alone.

```bash
atlas_checkout="${B10X_ATLAS_CHECKOUT:-atlas}"
atlas_head="$(git -C "$atlas_checkout" rev-parse HEAD)"
atlas_main="$(git -C "$atlas_checkout" ls-remote origin refs/heads/main | awk '{print $1}')"
test -z "$(git -C "$atlas_checkout" status --porcelain)"
test "$atlas_head" = "$atlas_main"
cargo run --manifest-path "$atlas_checkout/Cargo.toml" --locked -q -- \
  --store "$atlas_checkout/catalog/store" docs reconcile --workspace . --check
```

Keep internal plans, stories, ADRs, decisions, worklogs, security material, and research out of the public allowlist unless a repository authority explicitly declares them public.
<!-- b10x-docs-operations:end -->

<!-- b10x-release-operations:start -->
## Release completion

An ordinary release completes after this repository's exact tag, required source checks,
published release and required artifacts are verified. A pushed tag with unfinished checks or
uploads is queued; report it as released only after those requirements succeed.

Atlas reconciliation and public documentation publication run asynchronously. Do not wait for
Atlas or Website, update Website source locks or bootstrap snapshots, promote consumer pins,
release shared docs tooling, or redeploy documentation façades as part of an ordinary source
release. Report documentation as pending unless its publication was actually verified. A background
documentation failure does not invalidate a successful source release.

Keep this repository's provenance, correctness, security, compatibility and artifact verification
requirements. Shared rendering, routing or delivery-control changes still require their relevant
integration gates. A release request does not authorize deployment or downstream releases.
Repositories without a release unit retain their existing publication policy. This completion
boundary supersedes older instructions that attach synchronous documentation ceremony to each
source release.
<!-- b10x-release-operations:end -->
