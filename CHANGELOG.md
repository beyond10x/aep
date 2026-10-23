# Changelog

Notable changes to AEP. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html), where a **major**
version is a breaking change to a protocol's semantics, not merely to a Rust API.

One line per change. The reasoning lives in the commit messages and in `docs/design/`.

## [Unreleased]

### Added

- `aep drive resume <run> --evidence <file>` submits the file's records (the document `aep govern evaluate --evidence` reads) to the run before its first step, and `DriverOptions::evidence` carries them; a record the engine refuses stops the resume before anything is persisted.
- `EvidenceSubmission` implements `PartialEq` and `Eq`.

### Fixed

- A run stopped at an `operator` step under `--approver` can be answered: the pause text names `aep drive resume <run> --evidence <file>`, which writes the approval into the run. It named `aep govern evaluate --evidence --state <run>/snapshot.json`, which restores a snapshot, evaluates it and writes nothing back, so every resume stopped again with "nothing was recorded while the run was stopped".
- `aep govern evaluate --state` with `--evidence` says on stderr that it writes nothing back and names the resume.
- The refusal a driven step prints for an approval read out of a record names `aep drive resume --evidence` as the route.

## [0.77.0] - 2026-10-23

### Added

- An `llm` step may declare `commands:`, the argv prefixes its shell admits; `driven_surface`, which a harness consults per shell call, holds the step to one simple invocation of a declared prefix, and the refusal names the declared surface.
- The loader refuses a `commands:` prefix with no words, a blank word, a program written as a path, or a program that runs another program: the shells, `eval`, `exec`, `source`, the wrappers (`env`, `xargs`, `sudo`, `timeout`, `nohup`, `find`, …), the script interpreters and `npx` (`CommandPrefix::PROGRAM_RUNNERS`).
- `driven_surface` refuses `cargo --config` and `rg --pre` under every surface, because each runs a program the call does not name.
- `StepContext` carries the running step's declared `commands`, empty for a step that declares none.

### Changed

- `drivers/development/checks.yaml` declares `[aep, plan, artifact]` for `receive`, `specify`, `decompose`, `establish_verifiers` and `adversarial_verify`, and the gate, the generators (`task check|fmt|schema|ess|status`) and the working-tree `cargo` verbs (`build`, `check`, `clippy`, `doc`, `fmt`, `test`) for `implement`; its `scope:` now denies writes under `drivers/**`, `Taskfile.yml`, `.cargo/**` and `rust-toolchain*`.
- A step that declares no `commands:` keeps the surface it had (`aep plan artifact …`, `aep observe trace …` and the read-only programs) and the map digest it had, so a run under such a map resumes after the upgrade.

## [0.76.0] - 2026-10-21

### Fixed

- A repository protocol source loads on macOS; publishing its snapshot failed with `Permission denied (os error 13)`.

## [0.75.0] - 2026-10-21

### Added

- `aep plan artifact redact` replaces every case-insensitive occurrence of `--literal` with `--with` in an artifact.
- `redact` covers title, summary and body, works on any kind in a Git-native store and requires `--reason`.
- `redact` writes one `"change": "redaction"` evidence record with the literal's SHA-256; the literal is stored nowhere.
- `redact` refuses, writing nothing, an absent literal, a replacement or reason holding it, and SQL stores.
- New command `aep.entity.redact/v1` (`redact-entity`); the memory backend applies it to immutable kinds too.

### Changed

- `aep plan artifact validate` admits an edited `review-result` when its redaction records replay to the current text.
- **Breaking:** A store holding a redaction record needs this release to validate; older versions reject the record.

## [0.74.0] - 2026-10-16

### Added

- `drivers/releases/source.yaml` (`releases/source`, for `release/source/1`) cuts a release with `aep drive run`.
- `aep plan artifact spend` prints per task kind the median `task_spend` figures against baseline and target.
- `spend` takes `--kind`, `--since <YYYY-MM-DD>` and `--baseline <n>`, and writes nothing.
- `.engineering/project.yaml` gains an optional `spend.<kind>.<figure>: { baseline, target }`.
- A step map state may declare `concurrent: true` to run consecutive `command` steps at once.
- `aep drive resume --retry-in-flight <id>` reruns an interrupted concurrent batch under the attempts already charged.
- Workflow `workflows/bugfix/standard.yaml` (`bugfix/standard/1`) with step map `drivers/bugfix/standard.yaml`.
- The bugfix run takes `--input repro=<command>`, `logs`, `traces`, `artifact` and `gate` inputs.
- The bugfix workflow is rendered as `generated/instructions/bugfix/standard.md`.

### Changed

- The `release.source` profile no longer requires the `verify-after-action` principle.

### Fixed

- A calendar date (`findings_required_since`, `--since`) with a signed part such as `2026-+1-+2` is now refused.

## [0.73.0] - 2026-10-15

### Added

- A `kind: command` step may declare `expect: failure`, making a nonzero exit the passing verdict.
- `aep drive run --input <name>=<value>` (repeatable) passes named inputs, used as `{input.<name>}` in a step map.
- A command step may declare `output: <file>` to write its standard output into the run directory.
- An `llm` step's `context` may name `{run_directory}/<file>` to hand that file to the session.
- The ESS specification `ess/domains/drive.yaml` models `expect`, `output` and run inputs.
- `aep plan artifact evidence <id> --kind task_spend --from <file>` records what a task spent as `task_spend` evidence.
- `task_spend` reads `stream-json` or `metaharness.event/1` transcripts, `eval.run-manifest/1` or a run's `cursor.json`.
- A spend figure the source does not state is printed `unknown`, never `0`.
- A spend source outside the repository is refused with `SpendSourceOutsideRepository`.
- An unrecognised spend source is refused with `SpendSourceUnreadable`; no `--from` with `SpendSourceDigestMissing`.

## [0.72.1] - 2026-10-14

### Changed

- The `judgement:` requirement moves to a new step-map format, `aep.driver-steps/2`; the loader reads `/1` and `/2`.
- Both shipped maps under `drivers/development/` are `/2`, and the published `driver-steps` schema matches.

### Fixed

- A store pinning a protocol tree older than 0.72.0 validates again: `aep.driver-steps/1` makes `judgement:` optional.

## [0.72.0] - 2026-10-14

### Added

- `aep drive run`, `aep drive resume` and `aep drive status` print `llm steps  <declared> declared, <run> run`.

### Changed

- **Breaking:** Every `kind: llm` step of an `aep.driver-steps/1` map must carry `judgement:` or the map is refused.
- The published `driver-steps` schema refuses the blank, empty or zero values the loader refuses.

## [0.71.3] - 2026-10-13

### Fixed

- `aep plan artifact evidence --from <report>` records report and suite paths relative to the repository root.
- An input outside the repository is recorded by file name and digest; default reference `<name>@sha256:<digest>`.
- After recording a conformance report, the command prints `counts` or `does not count:` with the reason.

## [0.71.2] - 2026-10-10

### Fixed

- `aep plan artifact evidence --from <report/2> --suite <suite>` accepts producer profile `go-scenario-status/2`.

## [0.71.1] - 2026-10-10

### Changed

- `aep plan store migrate git` on an `aep.project/5` store drops repeated identical transitions instead of refusing.

### Fixed

- `aep plan artifact evidence --from <report/2> --suite <suite>` records a report over an ordinary suite.
- An unknown suite version is refused as `UnsupportedSuiteVersion`, naming it.
- `aep plan store migrate git` carries a move the `aep.project/1` journal held twice only once.

## [0.71.0] - 2026-10-09

### Added

- `project.yaml` takes an optional `findings_required_since: YYYY-MM-DD` requiring a `findings` block in new reviews.
- `aep plan artifact validate` reports a review without a `findings` block recorded on or after that date.
- `aep plan artifact new --prose-only <reason>` records a `review-result` without a `findings` block on purpose.

### Changed

- `aep plan artifact validate` treats a review it cannot date as undated instead of dating it by the clock.
- `aep plan artifact validate --format json|yaml` lists reviews without findings under `findings_standing`.

## [0.70.0] - 2026-10-08

### Added

- `aep plan store migrate git` and `aep plan reverse init` take `--planning-scope <name>`.

### Fixed

- In a linked worktree, `migrate git` and `reverse init` no longer write the worktree's directory name as scope.
- The scope now comes from `--planning-scope`, the `origin` remote, the primary checkout or the store's directory.
- `aep plan store migrate git --engineering .engineering` no longer fails with "cannot make an empty path absolute".
- A `planning_scope` refusal now says "a non-whitespace character" instead of "a non-whitespace byte".

## [0.69.1] - 2026-10-08

### Fixed

- `aep plan artifact relate`, `new --relate` and `unrelate` handle edges to another workspace member's artifact.
- A target naming a member the workspace does not declare is refused, naming `workspace.yaml`.

## [0.69.0] - 2026-10-07

### Added

- `aep plan artifact evidence --suite` admits `scenario_initial_state` and `synthesis_seeds` provenance from ESS 0.55.0.
- `aep plan artifact evidence --suite` admits authored refusals `ESS-AUTHOR-038` to `ESS-AUTHOR-041`.

### Fixed

- `aep plan artifact evidence --suite` admits coverage suites `ess-conformance/35` through `/45` from ESS 0.55.0.
- Grant, one-time disclosure and binding scenario keys are no longer refused as `MalformedScenarioId`.
- `aep govern resolve`, `evaluate` and `explain` with `--task` load the project's configured `protocols` source.
- `aep plan store migrate git --verify` compares every migrated evidence record field by field, not only counts.

## [0.68.0] - 2026-09-30

### Added

- A lifecycle `requires:` line may name `or: [<kind>, …]`, alternative evidence kinds that pay for the rung.

### Fixed

- An `executable-system-specification` reaches `conforming` on report/2 evidence, not only on `ess_conformance`.
- A specification's conformance record counts only when it passed and its `spec_digest` equals `model_digest`.

## [0.67.0] - 2026-09-30

### Changed

- `aep plan artifact evidence --from <report> --suite <suite>` admits coverage suites `ess-conformance/5` to `/33`.
- An ordinary suite major or one newer than `/33` is refused with `UnsupportedSuiteVersion`.
- Coverage evidence reads current ESS scenario ids and refusal codes, refusing too-new ones as `UnsupportedVocabulary`.

## [0.66.0] - 2026-09-30

### Added

- `aep plan artifact evidence --from <report> --suite <suite>` admits profile `external-scenario-status/1`.
- The recorded evidence keeps the exact profile, runner included, so supplied results never read as an ESS run.

## [0.65.0] - 2026-09-28

### Changed

- The README and website name the current release; `cargo xtask status` rewrites the release-pin regions.
- **Breaking:** SQLite plans are selected by `store: { sqlite: { path: <file> } }` in `aep.project/5`.
- **Breaking:** PostgreSQL plans are selected by `store: { postgres: { url: <url> } }` in `aep.project/5`.
- `--store <dir>` with no project around it opens a Git-native store with evidence under `<dir>/evidence`.
- `aep plan store migrate git` imports a transition to the current status for an artifact the journal never moved.

### Removed

- **Breaking:** The hybrid planning backend (`aep-backend-hybrid`, `store: hybrid`) is removed.
- **Breaking:** `aep plan artifact divergences`, `aep plan artifact catch-up` and `--backend hybrid` are removed.
- **Breaking:** Every verb refuses an `aep.project/1` store, naming `aep plan store migrate git --verify`.
- **Breaking:** `validate --format json` drops `pre_provider` and `chain_*`; `AEP_NO_UPGRADE_NOTICE` is gone.

### Fixed

- Loading a verified Git-pinned protocol snapshot opens no directory.
- `aep plan artifact validate` on an `aep.project/5` store accepts a migrated artifact first moved from a later status.
- `validate --outcome-within` dates a review in an `aep.project/5` store from the commit that added it.
- `aep plan artifact validate` no longer warns about a `review-result` whose `findings` block is `[]`.

## [0.64.0] - 2026-09-28

### Changed

- Evidence the CLI mints names its verifier tool `aep` and records the grouped command path.

### Removed

- **Breaking:** The `protocol` alias binary is removed; use `aep`, including in `aep drive` step maps.

### Fixed

- A stamped Git-pinned protocol snapshot is trusted without opening a directory; a format-1 stamp is replaced.
- `aep plan artifact validate` on an `aep.project/5` store refuses a hand-edited `status` on a never-moved artifact.
- `validate` refuses a changed committed evidence file and an edited committed `review-result` inside a Git work tree.

## [0.63.1] - 2026-09-28

### Fixed

- A pinned Git protocol snapshot is verified in full once per pin, not once per process, using a stamp file.

## [0.63.0] - 2026-09-28

### Added

- `aep plan store migrate git [--engineering <dir>] [--dry-run | --verify]` migrates `aep.project/1` stores.
- The migration writes `aep.planning-md/3` documents, evidence files and an `aep.project/5` `project.yaml`.
- Commands opening an `aep.project/1` store print a migration notice; `AEP_NO_UPGRADE_NOTICE=1` hides it.
- `aep doctor` reports an `aep.project/1` store as `warn`.

### Changed

- `aep plan reverse init` writes an `aep.project/5` project and refuses a `.engineering/planning` that holds a plan.

## [0.62.0] - 2026-09-28

### Added

- `aep.project/5`: a Git-native planning store (`store: { git: {} }`) with artifacts as `aep.planning-md/3` files.
- A move appends to the artifact's `transitions`; each evidence record is one file under `.engineering/evidence/`.
- `validate` checks that transitions are continuous, end in the status and follow the lifecycle.

### Changed

- Opening a Markdown or Git-native plan is linear in its size.

### Removed

- **Breaking:** Event-log stores (`aep.project/2`, `/3`, `/4`) are refused by every planning verb.
- **Breaking:** Every `aep plan store` verb is removed, including `inspect`, `verify`, `rebuild` and `export`.
- **Breaking:** `aep plan artifact resolve`, `aep plan artifact render` and `validate --against` are removed.
- **Breaking:** `--command-identity`, the move's `command_identity` and `mutation` fields and `--backend eventlog` go.
- **Breaking:** The planning-migration, projection, ownership and raw-capture schemas under `schemas/generated/` go.
- **Breaking:** The `aep-backend-eventlog` and `aep-planning-migration` crates are removed.

### Fixed

- A command verifies a pinned Git protocol snapshot once instead of once per resolution.

## [0.61.1] - 2026-09-27

### Changed

- Entity Runtime is pinned at 0.25.0 and Eventlog at 0.6.0; new tree stores use `eventlog-tree/2`.
- `eventlog-tree/1` stores read and write as before; `aep plan store migrate content` keeps the source layout.

## [0.61.0] - 2026-09-27

### Added

- `aep.project/4`: a tree store that writes each large value once to a content-addressed blob directory.
- The blob directory is set by `store.eventlog.blobs`, default `blobs`.
- `aep plan store migrate content --engineering <dir>` migrates an `aep.project/3` store to `aep.project/4`.
- `aep plan store migrate content --keep-source` keeps the old tree beside the new one.
- `aep plan artifact validate` on an `aep.project/4` store checks blob hashes (C1) and home paths in blobs (S9).

## [0.60.1] - 2026-09-27

### Changed

- Entity Runtime is pinned at 0.24.2 and Eventlog at 0.5.0; planning stores read and write as before.

## [0.60.0] - 2026-09-26

### Added

- `aep plan artifact new review-result … --findings <file|->` takes the findings as a JSON array.

### Fixed

- Planning writer fence files live under `<git-common-dir>/aep/` inside Git instead of in the working tree.
- A malformed `findings` block is refused at one quoted body line, with a hint naming JSON.
- A refused `render`, `store rebuild`, write or `validate` no longer leaves a stage directory behind.
- A stage directory left by a killed process is removed by the next publication once its owner is gone.

## [0.59.3] - 2026-09-25

### Fixed

- `aep --version`, `--help` and usage errors print `aep` instead of the alias name `protocol`.
- Refusing an empty `findings` block now names `[]` as the way to record a review with no findings.

## [0.59.2] - 2026-09-24

### Fixed

- `render` over a store an older release published last no longer leaves a stage directory behind.

## [0.59.1] - 2026-09-24

### Fixed

- `aep plan artifact render` re-renders a tree store an older release published instead of `ProjectionDrift`.

## [0.59.0] - 2026-09-24

### Added

- A tree store (`aep.project/3`) renders its projection as `format: aep.planning-md/2`.
- `aep plan store export --fixup <file>` applies a JSON object of literal replacements to every exported string.

### Changed

- On a tree store, `aep plan artifact evidence` records an observation instead of an `edit`, so branches merge.
- Entity Runtime is pinned at 0.22.0; Eventlog stays at 0.4.0.

### Fixed

- `aep plan store export` also rewrites home paths and fixup literals inside hex-encoded legacy bytes.
- The export comparison replaces inside strings, and a difference names its JSON pointer.
- Two concurrent `aep plan artifact validate` runs on one tree store no longer break each other.
- `aep plan artifact validate --against <revision>` skips V2 when the revision holds no tree store.

## [0.58.0] - 2026-09-24

### Added

- A tree store reads lifecycles from the project's `protocols` source and types every kind AEP names.
- `aep plan store install-hooks` writes a pre-push hook that runs `validate` for a push touching `.engineering/`.
- `aep.project/3` keeps the plan as typed Entity Runtime entities on a mergeable `eventlog-tree` store.
- New tree-store verbs `aep plan store init-tree`, `aep plan artifact resolve` and `aep plan artifact render`.
- `aep plan artifact validate` on a tree store checks V1–V5, S3, S5 and S9, also against `--against <revision>`.
- `aep plan store export --engineering <dir> --into <dir> --map <file>` copies an `aep.project/2` store to a tree store.

### Changed

- Entity Runtime is pinned at 0.21.0 and Eventlog at 0.4.0.
- An Eventlog planning write opens its authority once per command instead of once per read and write.

## [0.57.0] - 2026-09-22

### Added

- `plan store writer-control hold` binds an operator custody session to one migration or projection rebuild.
- Migration apply, retry and rebuild require live writer control; `--already-idle` affirms an idle writer set.
- The raw capture document schema is in the generated schema roster.

### Changed

- Entity Runtime is pinned at 0.19.0 and Eventlog at 0.3.0, both released commits.
- The Eventlog file provider verifies history once at open; later transactions re-check only what changed.
- `aep_backend_eventlog::open_with_snapshot` and `FileProjectionPublisher::with_snapshot` take a `HeldCapture`.
- `FileProjectionPublisher::publish_current` refuses on a publisher built by `with_snapshot`.
- A relation qualified with this store's own member name is a local edge again on every read path.
- `aep workspace crossings` no longer lists an edge from a member to itself.
- Migration receipts count `inventory.workspace_crossings` separately from `inventory.relations` on every path.
- A mapper refusal is reported at the source document's coordinate instead of the `--project` selector.

### Fixed

- `plan store migrate apply` imports in time linear in store size, with three captures whatever its size.
- `plan store migrate apply` publishes its projection from the capture it already took.
- `aep plan store migrate dry-run` and `apply` read the members declared in `.engineering/workspace.yaml`.
- A `.engineering/workspace.yaml` that does not parse is refused instead of read as declaring nothing.
- Opening an Eventlog plan takes one capture instead of one per record.
- `plan artifact validate` on an Eventlog plan reads drift from the authority's watermark, not `journal.jsonl`.
- On an Eventlog plan, evidence, review outcomes, `review-value` and `findings` are read from the authority.
- `plan artifact unrelate <review-result> reviews <artifact>` is refused while a `review_outcome` names the review.
- Migration discovers foreign destination paths before writing and refuses link substitutions during recovery.
- Migrated Markdown history stays visible through history, explanation and evidence reads.
- SQLite planning storage uses the bundled `rusqlite` 0.40.2 line shared with Entity Runtime and Eventlog.
- PostgreSQL, SQLite and filesystem captures retain completed observations when a later read fails.
- SQL migration admits physical catalog declarations before decoding provider rows.
- Explicit `--store` refuses an Eventlog project's derived planning projection.
- Project-selected and explicit `--store` writers contend on the same Markdown-store fence.
- Raw capture validation checks present sources on preflight refusals and accumulates capture defects.
- Shared Gates 0.1.1 rejects non-automation commit authors before scanning or reusing signed evidence.
- Harness integration, concepts and transcript guides route model-backed runs through `metaharness aep drive`.

## [0.56.0] - 2026-09-18

### Added

- A markdown plan's `journal.jsonl` is a hash chain; `aep plan artifact validate` names an edited record.
- A journal written before this is reported as not covered; the next appended record seals the older lines.

### Fixed

- Shared Gates 0.1.1 rejects non-automation commit authors before scanning or reusing signed evidence.
- Harness integration, concepts and transcript guides route model-backed runs through `metaharness aep drive`.

## [0.55.0] - 2026-09-10

### Added

- Pinned public Gates security and privacy checks, signed local evidence and coordinated hooks.
- Optional `ess_conformance_coverage_v1` evidence admits exact suite/5 lineage and complete selected coverage.
- Optional `ess-conformance-report/2` and suite readers with the opt-in `adp-ess-conformance/1` protocol.
- The opt-in `development.ess-conformance-v2` profile requires independent task expectations.
- Planning `--from` accepts report version 2 with its original suite through `--suite`.

### Changed

- **Breaking:** Model-backed driving moves to `metaharness aep drive`; AEP refuses such invocations and names it.
- Entity Runtime is pinned to a verified revision whose replay refuses imported AEP audit events.
- Direct evidence recording returns a checked result and takes an explicit reading instant.
- Source releases keep correctness and artifact checks while documentation publishes independently.

### Fixed

- Execution hosts can name the runnable resume command in refusal messages.

## [0.54.0] - 2026-09-04

### Changed

- `govern workflow render --format mermaid` writes a Mermaid diagram with guards, terminal states and `requires`.
- `govern workflow instruct --map <step-map>` names the steps that run in each state; `--map` needs `--id`.
- `aep-render` takes the step map as `steps::StepsView`.
- A refused `drive resume` names `drive run` instead of the nonexistent `--restart`.

## [0.53.0] - 2026-09-04

### Added

- `aep plan artifact unrelate <id> <relation> <target>` removes exactly the named edge.
- `unrelate` refuses an edge that is not there, naming the ones that are.
- `history` shows a removed edge as `no longer <relation> <target>`.

### Fixed

- `relate` re-making an edge that was taken back is no longer skipped as a replay in SQLite and Postgres stores.

## [0.52.0] - 2026-09-04

### Changed

- `aep --help` now groups commands under `govern`, `plan`, `drive`, `observe` and `doctor`.
- The old top-level verbs stay as hidden aliases with identical output and exit status; only `Usage:` lines differ.
- `validate`, `resolve`, `inspect`, `evaluate`, `explain`, `describe`, `schema`, `workflow` move under `aep govern`.
- `artifact`, `serve`, `entity`, `audit`, `workspace`, `conformance` and `reverse` move under `aep plan`.
- `eval` moves to `aep drive eval`; `aep drive` and `aep doctor` are unchanged.
- `trace`, `contract`, `property`, `specification` and `evidence` move under `aep observe`.
- A driven `llm` step's `Bash` grant admits both the flat and the grouped `protocol` spellings.
- Step maps, driver prompts, guides, the CLI reference, `README.md` and `AGENTS.md` use the grouped spellings.

### Fixed

- `aep eval run --stream` exits with the verdict it prints: `0` conformant, `1` contradicted, `3` undecided.
- `aep eval run`'s preflight reports every fault of the child's `PATH` at once (`EVAL-RUN-017`, `EVAL-RUN-018`).
- `trace evidence` always names the tool `protocol` in `provenance.command`, whichever binary ran; now tested.

## [0.51.0] - 2026-09-04

### Changed

- Crates moved to `crates/<area>/<crate>` under `govern`, `plan`, `drive`, `observe`, `profile` and `edge`.
- **Breaking:** crate `adp-domain` is renamed `aep-profile-development`, `aop-domain` is `aep-profile-operations`.
- **Breaking:** crate `protocol-cli` is renamed `aep-cli`; use `cargo install --path crates/edge/aep-cli`.
- The binaries `aep` and `protocol` and the wire ids `adp/1`, `aop/1` and `adp/default` are unchanged.
- References follow the renamed agent plugins: `aep-planning` to `aep-plan`, `adp` to `aep-drive`.
- References follow the renamed plugin `ess-schema` to `ess-specify`, whose skill is `specify`.
- `EVAL-RUN-018` fires on both the `ess-schema:` and `ess-specify:` skill prefixes, for one release.
- A `conformance/eval` case may declare `subject.skills`; unknown fields inside it are still refused.
- A live artifact's `scope:` is checked for an area-qualified path naming a crate that does not exist.
- The *Check a transcript* guide's console output is regenerated and held by a test.

## [0.50.0] - 2026-09-03

### Added

- Lifecycle for `executable-system-specification`: `draft`, `validated`, `conforming`, `superseded`, `archived`.
- Reaching `conforming` requires one `ess_conformance` record.
- `aep artifact set --model-digest` writes `model_digest` on a specification artifact; other kinds refuse the key.
- `cargo xtask release` checks that the tag's commit is on `origin/main`.

### Changed

- Tag `0.48.0` is not on `main`; its lifecycle and model digest content ships in this release.

### Fixed

- `aep artifact set --model-digest` now keeps the digest; it reported success and wrote nothing.
- `cargo xtask release` no longer accepts a tag on a branch that never reached `main`.
- The Design principles stale-approval example now uses the fields AEP actually checks.

## [0.49.0] - 2026-09-03

### Fixed

- `eval run` and `trace redact` read the git identity in the run's directory as well as their own.
- A one-word git `user.name` is redacted only from eight characters up, so `root` or `CI` is left alone.
- Redacting an operator named `user` is idempotent; a second pass no longer produces `<<user>>`.

## [0.47.0] - 2026-09-03

### Added

- `protocol trace redact --transcript <path> [--out <path>]` applies the redaction to a stream on disk, idempotently.
- `trace redact` does not re-digest; a manifest's `transcript_digest` still names the bytes its run wrote.

### Fixed

- `--redact` also removes the operator's git name and email, not only `$HOME`, `$USER` and `$LOGNAME`.

## [0.46.0] - 2026-09-03

### Changed

- The Design principles guide is a failure-first, runnable walkthrough of the passkey example.

### Fixed

- `aep eval run` no longer double-counts cost, usage and duration when one session writes two terminal records.

## [0.45.0] - 2026-09-03

### Added

- `aep eval run` refuses a live spawn when the child's `aep` on `PATH` has another version (`EVAL-RUN-017`).
- `task install` refreshes `~/.local/bin/aep` and `~/.local/bin/protocol` after the cargo install.
- A case whose `subject.skills` names `ess-schema:*` requires `ess` on the child's `PATH` (`EVAL-RUN-018`).
- `aep eval run --budget-usd` also caps each Claude session through `--max-budget-usd`; needs metaharness 0.6.1.
- `aep eval run --model <MODEL>` is forwarded verbatim to `metaharness run claude --model`.
- Run manifests record `model_requested` beside `model` when the run asked for a model.
- `--model` on `codex` or `b10x` is refused (`EVAL-RUN-016`).
- `aep eval run --plugin <repo>@<name>@<version-or-commit>`, repeatable, forwards a pinned marketplace plugin.
- `--plugin` and `--plugin-dir` combine, and arm `plugin` is satisfied by either (`EVAL-RUN-012` if neither).
- Run manifests name each declared marketplace plugin in a new `plugins` list beside `plugin_digest`.
- `--plugin` is refused unpinned (`EVAL-RUN-013`), on `codex`/`b10x` (`EVAL-RUN-014`) and on arm `raw` (`EVAL-RUN-015`).
- A declared plugin that the attestation does not list is refused (`EVAL-STREAM-013`).

### Changed

- `aep trace check` decides a `tool.absent` expectation only over a closed stream, and answers `unk` otherwise.
- A `metaharness.event/1` stream is closed by a last-line `stream.closed` event or `stream_complete: true`.
- A Claude Code `stream-json` transcript is closed by a final `result` record; a Codex rollout never is.
- `session.ended` does not close a `metaharness.event/1` stream.
- Unread events no longer block a decided `tool.absent`; they still leave counts such as `tool.called` at `unk`.
- `aep eval run --redact` also redacts the stream: the home directory becomes `~`, the user name `<user>`.
- Home paths are replaced longest first, and user names only at word boundaries.
- The manifest's `transcript_digest` is taken over the redacted bytes.
- `--stream --redact` writes the redacted stream into `--out` as `<run>.events.jsonl`.
- Repository names, commit ids and branch names are not redacted.

### Fixed

- The 0.43.0 rendered-plan change summary fits the `b10x-change/v1` limit for documentation snapshots.

## [0.44.0] - 2026-09-03

### Added

- A `review-result` body may carry a fenced `findings` YAML block; `aep artifact new` refuses a malformed one by line.
- Finding fields are `file`, `line`, `category`, `severity`, `verdict`, `origin` and `message`.
- `aep artifact show --format json` returns `findings`; `validate` reports a `review-result` with prose-only findings.
- `aep artifact findings <artifact-id>` sorts two reviews' findings into `carried`, `new` and `resolved`.
- Evidence kind `review_outcome`, recorded with `--review <review-result-id> --outcome no-op|fixed|escalated`.
- `aep artifact show <review-result-id>` prints its outcomes; `validate --outcome-within <days>` reports open reviews.
- `aep artifact review-value [--since <YYYY-MM-DD>]` prints per-reviewer reviews, findings, outcomes and cost.
- Stories carry `scope:` entries of a `path` and a `confidence` of `cited` or `inferred`.
- `aep artifact scope <id> --add <path> [--inferred]` and `--remove <path>` edit a story's scope.
- `aep artifact waves` derives parallel waves from `scope` and `depends_on`, printing collisions and `unassessed`.
- `aep artifact waves` exits `2` on a `depends_on` cycle; `--kind` defaults to `story` and `--status` narrows.
- `aep artifact validate` reports in-progress stories that declare no `scope`, without failing.
- `aep reverse openapi` drafts `relations:` with `kind: references` from `$ref` properties and `<x>_id`/`<x>Id` fields.
- Drafted relations carry `UNMAPPED: ownership` or `UNMAPPED: cardinality` where the contract is silent.

### Changed

- Releases publish `aep` and `protocol` archives for x86-64 and ARM64 Linux and macOS with `SHA256SUMS`.

### Fixed

- `aep artifact new --ref` now writes the reference to the document; it was dropped since 0.41.0.

## [0.43.0] - 2026-09-03

### Added

- `aep artifact board --format markdown` renders the board as a page, linking references through `providers:`.
- `aep artifact graph --format mermaid` writes a `flowchart LR`.
- A lifecycle document may carry per-rung `descriptions:`; filled in for `story`, `task` and `design`.
- `aep artifact evidence <id> --from <report>` records an `ess-conformance-report/1`, dated by its `completed_at`.
- `--from <report>` refuses a report with no scenarios, no `spec_digest`, or of another format.

## [0.42.0] - 2026-09-02

### Added

- `aep doctor` reports `ok`/`warn`/`fail` for the binary, project file, protocol source, store, plugin dirs and tag.
- `aep doctor` exits `1` on any `fail`; it fixes nothing and fetches nothing.
- Epic, story and specification templates seed an `## Ambiguities` section.
- An ambiguity is `inferable` or `requires-stakeholder-input`; the latter is raised as a `decision-blocker`.

### Changed

- The README and docs name `beyond10x/agentplugins` as the source of the Claude Code and Codex plugins.

## [0.41.0] - 2026-09-02

### Added

- Planning artifacts carry `refs:` to the same work in an external system, such as `jira:EX-1`.
- `aep artifact new --ref`, `aep artifact set --ref/--unref` and `aep artifact list --ref <provider>:<key>` manage refs.
- `.engineering/project.yaml` takes a `providers:` map of URL patterns with `{key}`; `aep artifact show` links refs.
- `aep reverse tickets` reports tracker keys in history and proposes `artifact set --ref` lines; it writes nothing.
- `aep artifact list --format json` and `board --format json` always carry `refs`.

## [0.40.3] - 2026-09-01

### Fixed

- AEP's profiles, driver prompts, templates, examples and docs invoke `aep`; `protocol` stays an exact alias.

## [0.40.2] - 2026-09-01

### Changed

- Guides and command references use `aep` examples; `protocol` remains an exact compatibility alias.
- Entity Runtime is pinned at `0.17.5`.

## [0.40.1] - 2026-09-01

### Fixed

- Entity Runtime is repinned to `0.17.4` so fresh builds resolve.

## [0.40.0] - 2026-09-01

### Added

- `aep` is the canonical command; `protocol` remains an alias with identical stdout, stderr and exit status.
- Optional crate `aep-ess-evidence` converts an ESS conformance report into `ess_conformance` evidence.
- Profiles `release.source` and `release.dependency-chain` govern source releases and dependency promotion.
- Harness-integration docs cover `protocol drive` setup and a Claude Code, Codex and b10x feature matrix.

### Changed

- Workspace dependencies are refreshed, including `sha2 0.11` and `jsonschema 0.52`.

## [0.39.2] - 2026-08-31

### Fixed

- CI installs Go 1.25.10, the minimum the pinned go-task 3.52.0 requires.

## [0.39.1] - 2026-08-31

### Fixed

- CI and Pages use the Node 24 versions of GitHub's checkout, Go and Node setup actions.

## [0.39.0] - 2026-08-31

### Fixed

- Driven vendor steps enforce the step map's `scope:` rules (`allowed`, `partial-only`, `denied`) on file subjects.
- The pinned b10x adapter contract result consumes the five-vector harness 0.9.1 contract.

## [0.38.1] - 2026-08-31

### Added

- `aep_project::load_bundle` is public; it validates a definition tree and prints its source-byte digest.

## [0.38.0] - 2026-08-31

### Added

- `aep-client` publishes the service route catalog and JSON Schema for every HTTP request and response document.

## [0.37.1] - 2026-08-31

### Fixed

- Driven b10x steps declare metaharness's `observe` decision mode instead of inheriting `frame`.

## [0.37.0] - 2026-08-31

### Added

- `protocol drive run` runs model steps only with `METAHARNESS_LIVE=1`, `--budget-usd` and `--assume-usd-per-run`.
- `drive run` reserves budget before each spawn and stops with `budget-exhausted`; a resume never raises the cap.

## [0.36.4] - 2026-08-31

### Fixed

- A race in the `protocol serve` test client no longer resets the first HTTP request.

## [0.36.3] - 2026-08-31

### Fixed

- PostgreSQL test-schema cleanup is serialized so parallel backend tests no longer race.

## [0.36.2] - 2026-08-31

### Fixed

- Entity Runtime 0.17.3 lets fresh PostgreSQL command sessions take advisory locks and run containment queries.

## [0.36.1] - 2026-08-31

### Fixed

- Fresh PostgreSQL command sessions keep a pinned lifecycle registry while using provider-reserved identity ranges.

## [0.36.0] - 2026-08-31

### Added

- The query contract exposes bounded paginated history beside complete version-1 history.
- The official client prefers wire-v2 history pages and falls back to version 1 when v2 is not served.
- PostgreSQL offers a fresh command-session backend that locks only one command's records.
- Startup definition trees can be validated and pinned by a deterministic SHA-256 digest.

### Changed

- Central-service retries compare canonical caller-controlled command intent.
- Entity Runtime is pinned at 0.17.1.

## [0.35.0] - 2026-08-31

### Added

- `aep-client` publishes the `application/vnd.aep.service+json;version=1` wire and an official client.
- The client implements `CommandService` and `QueryService` with injected transport and credential ports.
- Request documents omit actor and time, refuse unknown members, and keep bearer credentials out of diagnostics.

### Changed

- **Breaking:** `Specification` and `EssIr` expose read-only accessors instead of public fields.
- Project discovery, schema parsing and Git acquisition move to the `aep-project` crate.
- Pinned Git sources use a bare object cache and read-only snapshots revalidated before reuse.
- CI runs `task check` once; `cargo xtask status --check` guards the generated status regions.

### Fixed

- `protocol serve` completes each loopback response before closing, so answers are no longer discarded.
- A refused memory-backed command leaves no partial mutation and records one rejection audit.
- Durable contract commands commit all records through one `entity-runtime` atomic batch.
- Entity, relation and audit query cursors (`offset-<n>`) advance correctly and stop when no matches remain.
- The eval matrix reader refuses records missing specification ids, digests or expectations (`EVAL-RECORD-*`).
- A resumed driven run publishes snapshot and cursor together and refuses partial or tampered pairs.
- A resume past an uncertain attempt needs `--retry-in-flight` or `--record-in-flight-no-verdict`.

## [0.34.0] - 2026-08-31

### Added

- `protocol serve` shows the board, artifacts and next rungs in a browser and moves artifacts like `artifact move`.
- `protocol serve` binds `127.0.0.1` only, requires the printed URL's token, and offers `--read-only`.
- The Claude Code plugin manifest carries version `0.2.0`, and every `SKILL.md` states it.
- `cargo xtask plugin` fails on plugin changes without a version bump or a stale `integrations/claude-code/roster.json`.
- `task release-check` checks version, `CHANGELOG.md` heading, tag on `origin`, GitHub Release and a gate record.
- `AGENTS.md` § Gate carries a step list generated by `cargo xtask status` and checked by `status-check`.
- `trace-spec/1` expectation kinds `usage.trend` and `usage.share` assert how per-request usage moved.
- `usage.trend` answers `gap` on a wrong-way pair, `ok` when it moved right, and `unk` when it never moved.
- `on_unknown: gap` makes a flat `usage.trend` fail; unknown fields and impossible share bounds are refused.
- The Claude Code plugin ships a `wave` skill; `AGENTS.md` § Branches and waves documents the convention.
- The plugin ships a read-only `story-scoper` agent that returns a `## Scope` section marked cited or inferred.
- The Claude Code plugin ships `implementor` and `adversary` agents.
- `AGENTS.md`, `README.md` and the plugin README say what `integrations/` is for.
- The `task docs-check` gate step requires a `website/docs/reference/cli.md` entry for every CLI verb.
- `protocol artifact relate <id> <relation>:<target>` is accepted beside `relate <id> <relation> <target>`.
- `protocol artifact body <id> --from <path|-> --append` appends; `--section "<heading>"` replaces one section.
- `protocol artifact show <id> --body-only` prints the body bytes only.
- `protocol artifact set <id>` changes `--title`, `--summary`, `--owner`, `--tag` and `--untag`.
- `protocol artifact set` refuses `--status`, `--revision`, `--id` and `--kind` by name.
- `protocol artifact validate --strict` exits 1 on assertion-only statuses, pre-log documents and log drift.
- `protocol artifact move` refuses a move that would introduce a new validation finding.
- `protocol artifact body --from` refuses an empty body.
- `protocol artifact move --to <status> --via` walks the unguarded rungs, journalling each.
- `protocol artifact explain` prints what each legal next rung costs and what is held.

### Changed

- Thirteen published JSON Schemas refuse identifiers their constructors refuse and bound length with `maxLength`.
- Shipped skill documents no longer suggest editing planning artifacts by hand.
- The `wave` skill keeps a wave page per unit, a unit brief file, and writes learned `## Scope` back.
- The adversary's findings carry an `origin` column, and it attacks the unit's own added contracts first.
- The implementor reports executed test counts per lane and fixes the class of a finding.
- The planning skill matches the CLI's behaviour and loads on adoption.
- `protocol artifact kinds` lists kinds the document tree declares and the `<type>-blocker` family.
- `protocol artifact blocked` says when the store has no blocker kind.
- `--title` and `--summary` accept values starting with `-` on `new` and `set`.
- `relations` is always a list in `list --format json` and `show --format json`.
- Evidence-kind refusals suggest `health_observation` and `artifact`.
- The website's release chip, status tag and gate step list are generated and checked by `status-check`.

### Fixed

- `protocol artifact move` refuses a lifecycle the pinned kernel cannot hold instead of panicking.
- Evidence-gate refusals name the `protocol artifact evidence` command to run, and `--at <iso8601>` for dated rungs.
- The landing page's status panel drops hand-written test counts, a stale tag and a false durable-backend claim.

## [0.33.0] - 2026-08-30

### Added

- `regex:` matchers now run in a `trace-spec/1` document; a `regex:` searches the field, a `glob:` still anchors.
- `TRACE-SPEC-008` now fires at validation on a pattern the engine cannot compile, quoting the engine's error.
- `{regex: ".*"}` is refused under `text.matches`, as `{glob: "*"}` already was.
- Blocker kinds are typed by what clears them (`credential-blocker`, `decision-blocker`, ...); the type list is open.
- `artifacts/kinds/blocker.yaml` declares six starting blocker types and what clears each.
- `protocol artifact list` and `board` mark blocked artifacts `blocked: <type>`; `--format json` adds `blocked_by`.
- New `protocol artifact blocked [--type <type>]` lists stopped items grouped by blocker.
- A blocker stops blocking when moved to the end of its ladder (`protocol artifact move <blocker> --to cleared`).
- New optional `withholds:` key names the evidence kind a blocker withholds; `protocol artifact explain` prints it.
- `validate` refuses `withholds` on an artifact that blocks nothing or with an unknown evidence kind.
- New `serves` relation points a story or task at a `vision` objective; `validate` refuses `serves` into other kinds.
- With a `vision` in the store, `validate` requires every `proposed`, `approved` or `active` story or task to serve one.
- New `protocol drive transition` answers a native flow's section-boundary hook; exit 0 proceeds, 2 refuses.
- `protocol drive transition --run <id>` decides on that run's snapshot; the verb writes nothing and takes no lock.
- `--take-lock` on `protocol drive run` and `resume` records the lock it took as `took_lock_from`.
- A refusal over a live lock names the holder's state from its `cursor.json`, or says `state unknown`.
- `protocol artifact validate` reports a `revision:` no logged write produced as `forged` (JSON list) and exits 1.
- New `{task}` placeholder in `command` steps is the run's task path; `drivers/development/default.yaml` uses it.
- `AEP_ACTOR` sets the actor store writes are journalled with; an unparseable value is refused.
- `protocol drive` sets `AEP_ACTOR` to `agent:<execution id>` for every step process it starts.
- `protocol drive run --pause-on-approval --approver agent:<name>` admits that agent's approval at `operator` steps.
- The cursor records who answered an `operator` step; self-approval and approvals by unnamed agents are refused.
- Run directories record the program each `command` step spawned in `commands.jsonl` and at the top of each step `.log`.
- `protocol drive run` and `resume` refuse before allocating a run id if steps would get a different `protocol` version.
- `network.read` is scoped by audience: `network.read:private` and `network.read:public`; unscoped still means both.
- **Breaking:** `aep/1` floors `network.read:private`, so `allow: [network.read]` alone no longer resolves.
- The driver offers no `WebFetch` or `WebSearch` to a profile that may not read privately.
- `--claude-endpoint` and `--claude-model` point `claude-code` steps at a gateway, like the `--b10x-*` flags.
- New `protocol artifact show <id>` prints one artifact's frontmatter fields and body, over every store type.
- `task audit-check` is a gate step, re-resolving every `file:line` that `docs/guide/open-vocabulary.md` cites.

### Changed

- **Breaking:** `protocol workflow flow` emits every state as a group of `<state>-1`, `<state>-2`, ... nodes.
- `protocol drive transition` is now consulted on both sides of every state of a native walk.
- A driven `llm` step's writes are decided by the step's `scope:` in the step map: `denied`, `partial-only`, `allowed`.
- **Breaking:** a step map that declares no `scope:` restricts no writes; both shipped maps now declare one.
- `protocol drive hook` now answers only the rule refusing an `Edit` across a planning document's closing `---`.
- The approval floor accepts a broad grant when the floored entry is explicitly denied; an approval gate still is not.
- The `unknown capability` refusal now also lists `network.read[:public|private]`.
- `protocol eval` documents per arm what a clean store-integrity cell means; text output notes it under `native` cells.
- A metacharacter inside quotes is no longer refused as composition; `$(` and backticks still are in double quotes.
- States that admit reading also allow `grep`, `rg`, `ls`, `cat`, `head`, `tail` and `wc` in the shell.

### Fixed

- A bare calendar date in `observed_at` is compared at UTC+14, so dates written east of UTC are no longer future.
- `protocol evaluate` refuses only the future-dated record, naming file and position, and submits the others.
- `protocol evidence inspect` applies the same date comparison as `protocol evaluate`.
- A `review-result` can be retired with `move --to archived`; `body` on it stays refused.
- `protocol artifact new … --from <path|->` writes the body with the record, for every kind.
- `protocol artifact --help` says to retire an item with `move --to archived`, not delete it with `rm`.
- `protocol workflow flow --map` now carries each state's steps into its nodes; without `--map` output is unchanged.
- `validate` no longer reports revision drift after a logged write that changed nothing, such as an empty `body`.
- `protocol eval` lists all four arms, including `native`, when refusing an unknown arm.
- **Breaking:** `spec-driven` and `clean-room` require an approved specification that `specifies` the task's own work.
- **Breaking:** `protocol specification evidence` selects only a spec that `specifies` the task's `derived_from`.
- A run that passed an `operator` step with no recorded answer now says so in its report.
- `command` steps naming `protocol` run the driver's own binary, not the first `protocol` on `PATH`.
- Driven native-loop steps get a program allowlist, so they can start the `protocol` CLI.
- The tool audit no longer reports every catalogue tool missing on a b10x-loop run.
- `conformance/trace/expectations.trace.yaml` now also gates that a `protocol artifact new` call succeeded.
- The `protocol drive resume` line a stopped run prints now works: `resume` reuses the run's launch flags.
- `--max-iterations` on a resume bounds that call, not the run's lifetime iteration count.
- `protocol drive` refuses a run whose sessions' `PATH` cannot reach the `protocol` CLI, naming the install.
- A driven run without `--plugin-dir` falls back to the project's `integrations/claude-code` plugin.
- A driven session's prompt now names the task the run drives.
- The audit's own checks read the store with the tree's build, not the `protocol` on `PATH`.

## [0.32.1] - 2026-08-28

### Fixed

- `protocol drive run` checks evidence coverage before checking whether `metaharness` is on `PATH`.
- The `drive_cli` tests pass without `metaharness`, which had kept 0.32.0's GitHub Release from publishing.

## [0.32.0] - 2026-08-28

### Added

- `protocol drive run --map drivers/development/default.yaml` produces every evidence kind a `feature` task demands.
- Three new producer verbs mint that evidence with `producer: verifier`; no `--allow-evidence-gap` is needed.
- A test holds `entity-runtime`'s eleven re-expressed ladders equal to AEP's, edge for edge in both directions.
- `task plan-check` (`protocol artifact validate` on the repository's own store) is a step of `task check`.
- New `protocol artifact explain <id>` shows per status the move, its evidence and the revision each record was at.

### Changed

- Story completion still requires a `test_result`; a `trace_conformance` requirement is accepted but not enforced.

### Removed

- **Breaking:** generated files drop `compiler_version`, `generator_version`, `synthesizer_version`, `planner_version`.
- Records carrying those fields still validate; the published schema keeps them declared but nothing writes them.
- **Breaking:** `task plugin-eval` and `task driven-eval` are removed; the eval now lives in metaharness.

### Fixed

- `protocol drive` accepts a cross-repository relation declared in `.engineering/workspace.yaml`, as `validate` does.
- `protocol artifact validate` finds the workspace manifest from any directory inside a project.
- Over SQLite and Postgres, `protocol artifact history` again prints the `(on asserted evidence)` marker.
- `synth-check` works in a `git worktree`.

## [0.31.0] - 2026-08-28

### Changed

- `protocol artifact relate` and `new --relate` no longer bump the source document's revision on any store.

### Fixed

- `protocol artifact evidence` accepts its own default `YYYY-MM-DDTHH:MM:SSZ` instant, so `--at` is not required.
- Over SQLite and Postgres, evidence on hand is counted from the entity's events, so evidence-gated moves work.

## [0.30.0] - 2026-08-28

### Added

- `project.yaml` gains `store:`: `markdown` (default), `sqlite: <path>`, `postgres: <url>` or `hybrid: {...}`.
- Every `protocol artifact` verb, `protocol drive` and `protocol conformance --backend project` open the declared store.
- `protocol validate` also validates the discovered `project.yaml`; `--store <dir>` remains the markdown override.
- `protocol artifact history` prints the same history over SQLite and Postgres plans, from the event log.
- `examples/planning-passkeys/.engineering/project.sqlite.yaml` is an example project on SQLite.
- Relations and recorded evidence now write an observation event at the entity's unchanged revision.
- The `entity-runtime` pin moved to 0.13.0.
- `store: hybrid` keeps the plan in markdown plus a SQLite or Postgres replica, via the new `aep-backend-hybrid` crate.
- A hybrid store types `authority`, `read`, `on_unreachable` and `on_divergence` in `project.yaml`, none defaulted.
- New `protocol artifact divergences` lists what one side missed (exit 1 while any remain); `catch-up` replays it.
- `protocol conformance --backend hybrid` runs the suites against the hybrid store.
- `examples/planning-passkeys/.engineering/project.hybrid.yaml` is an example hybrid project.

### Changed

- The ladder bridge `kernel` moved to `aep-backend-entity`; `aep-backend-markdown` re-exports it under the same path.
- `Identity::with_lifecycles` gives a SQLite plan its ladders.

### Fixed

- A plan seeded into SQLite keeps its tags (`seed::from_manifest`).
- `describe_type` reports each planning kind's lifecycle: initial status, statuses, edges and evidence requirements.
- New `aep-backend-postgres` crate implements the contract over PostgreSQL; `postgres-check` skips without a URL.
- `protocol conformance --backend postgres --store <url>` runs the suites against a PostgreSQL server.
- The `entity-runtime` pin moved to 0.12.1.

## [0.29.0] - 2026-08-28

### Added

- `MarkdownProvider` implements the `entity-store` provider traits over `.engineering/planning/` and `journal.jsonl`.
- `history()` and `audit()` on every `EntityBackend` read the provider's event log, so a second process sees them.
- `protocol artifact validate` reports drifted and deleted documents against the event log and exits 1.

### Changed

- `MarkdownBackend` is now `EntityBackend<MarkdownProvider, MarkdownProjection>`; documents and output are unchanged.
- `EntityBackend<S, P = Identity>` gains a projection type parameter.
- The `entity-runtime` pin moved to 0.11.0; every event the adapter writes carries `args`.

## [0.28.0] - 2026-08-28

### Added

- New `aep-backend-entity` crate: `EntityBackend<S>` implements the contract over any `entity_store::Store`.
- A SQLite plan file stores one event per affected entity with each accepted command.
- The `entity-runtime` pin moved to 0.10.0; `EntityBackend::over` is now fallible.
- `protocol conformance --backend memory|markdown|sqlite [--store <path>]`; output names the backend it ran against.
- `SqliteBackend::open` hydrates a populated SQLite plan, so a second process reads what the first wrote.

### Changed

- `SqliteBackend` is now `EntityBackend<SqliteStore>` with the same public API.
- One `entity-runtime` (0.9.1) across the workspace; a new `dep-check` gate step fails if versions split.
- `DomainEvent` records `from_state`, `to_state` and `changed`; `entity_core::rehydrate` folds an instance from events.

## [0.27.3] - 2026-08-26

### Fixed

- `status --check` and `version-check` count only tags reachable from `HEAD`, so pushing two tags at once passes.
- 0.27.1's GitHub Release is now published from its changelog section.

## [0.27.2] - 2026-08-26

### Changed

- `README.md`, `docs/status.md` and `where-this-stands.md` state that the storage contract has three implementors.
- `docs/guide/backend.md` describes what the two durable backends do.
- `website/docs/status/limitations.md` lists the current storage limitations.
- `harness-planning-and-driver-design-v0.1.md` marks D-P1 closed and D-P5 open.

## [0.27.1] - 2026-08-26

### Fixed

- The lab's bridge copy is synced with `generated/web/billing/bridge.js`; `lab-check` is now a `task check` step.

## [0.27.0] - 2026-08-26

### Added

- `MarkdownBackend` implements `CommandService` and `QueryService`; the sixteen `aep-conformance` suites run against it.
- New `aep-backend-sqlite` crate: a single-file SQLite backend over `entity-runtime`'s transactional store.
- New commands `aep.status.move/v1` and `aep.evidence.record/v1`.
- `eval.matrix/1` adds an `advisory` column, omitted when zero.
- New gate steps `cargo xtask guards`, `cargo xtask claims` and `cargo xtask version`.
- `protocol artifact validate` reconciles the journal with the files and reports statuses reached on asserted evidence.

### Changed

- `AGENTS.md`, `docs/plan/gap-register.md` and the markdown backend's docs now say D-P1 is closed.

### Fixed

- The store refuses a `status` change on an `UpdateEntity` that the ladder does not allow.
- `MoveStatus` respects immutability, so a review result can no longer be edited, and enforces `expected_revision`.
- A relation is no longer journalled as `Change::BodyReplaced`; repeated `--relate` and replays bump no revision.
- `protocol artifact new --relate` resolves targets before writing anything.
- The latch now covers reads in both durable backends.
- `protocol artifact evidence --at` is honoured again.
- Seeding accepts a store holding a cross-repository relation.

## [0.26.0] - 2026-08-26

### Added

- `ArtifactGraph::build_in_workspace` and `StoreReport::graph_in_workspace` build a graph aware of workspace members.

### Fixed

- Corrected 0.25.0 notes: cycle detection over the combined workspace graph did not exist; it ships in this release.
- Corrected 0.25.0 notes: no shipped workspace member holds `story:passkey-login`; that example was wrong.
- Corrected 0.25.0 notes: a member listed by `members` is reported as `absent`, not `unresolved`.
- `protocol workspace crossings` now finds cross-member cycles and exits 1 on a cycle whatever `--strict` says.
- An unresolved crossing adds no edge to cycle detection.
- A `member/` prefix on a relation target no longer disables cycle detection inside a single store.
- The dangling-reference check is no longer relaxed for any target containing `/`; only declared members are crossings.
- A target naming an undeclared member is checked as a dangling edge; `build` and `graph` declare no members.
- `protocol workspace show` tells an unreadable member from a missing artifact and lists the members it could not read.
- A member sourced from a pinned git revision is no longer dropped from the assembly.
- The unresolved-member list reaches every `protocol workspace` verb that can report it.
- The documentation of `parse::workspace` describes the workspace file instead of copying `parse::project`.

## [0.25.0] - 2026-08-26

### Added

- `.engineering/workspace.yaml` (`version: aep.workspace/1`) names member repositories, each with a `source`.
- A workspace `source` takes the `protocols:` locator; absolute paths and unpinned git locators are refused.
- A generated JSON Schema for the workspace file.
- `protocol workspace` with `list`, `crossings`, `show` and `members` reads the plan across every member.
- `members` exits `0` and lists a member nobody has checked out instead of failing.
- An unqualified reference held by more than one member is `Ambiguous`: every holder is listed, no document returned.
- Membership is carried beside the artifact id; no id is rewritten when a store is read through a workspace.
- A member that fails to load appears as an empty member with its failures attached, never skipped.
- A `blocks`, `depends_on` or `derived_from` relation may target another member; an unresolvable target is a typed fact.

### Changed

- Every `protocol workspace` verb only reads; nothing writes to another member's store.

## [0.24.0] - 2026-08-26

### Added

- `.github/workflows/release.yml` cuts a GitHub Release on a version tag after running `ci.yml`; no binaries attached.
- `cargo xtask notes <version>` prints one release's CHANGELOG section, reflowed for GFM release notes.
- `cargo xtask notes --self-test` checks the reflow against eight Markdown shapes; the release workflow runs it first.

### Changed

- Website: new page on lifecycles decided as data; CLI reference now covers `protocol reverse`.
- Website: CLI reference adds `artifact history` and `artifact evidence`; status, roadmap and limitations updated.
- Website: the vocabulary page says artifact kinds and statuses are open to authors and `evidence_kinds` is closed.
- Website `/releases` has one post per release tag; backported posts are marked as written retrospectively.
- Release posts carry `release_tag` and `release_commit` front matter and sort in release order by filename.
- Drifted counts corrected in `AGENTS.md`, `README.md`, `docs/status.md`, `docs/guide/adopting.md` and website pages.
- Documented the boundary with `metaharness`: harness transcript readers and paid eval runs belong there.

### Fixed

- The delivered-waves CI check fetches all tags explicitly when run on a tag ref, and prints how many tags it sees.
- The release-notes reflow keeps a line ending in two spaces as a line break.

## [0.23.2] - 2026-08-26

### Changed

- `task check` now runs the `msrv` and `website` jobs, so it covers everything CI runs.

### Fixed

- The MSRV 1.85 CI job passes again: idna_adapter is pinned to 1.1.0, so no `icu` crate needing rustc 1.88 is pulled in.
- The Website CI job passes again: a link from `website/docs/` into the repository tree broke the Docusaurus build.

## [0.23.1] - 2026-08-26

### Changed

- `aep-backend-markdown` pins `entity-core` 0.5.2 instead of 0.5.0; the linked source is unchanged.

### Fixed

- The `outbound-claim` lifecycle starts at the built-in `draft` instead of `drafted`.

## [0.23.0] - 2026-08-26

### Added

- New `outbound-claim` artifact kind for a claim sent outside the organisation, defined in YAML with no Rust change.
- An `outbound-claim` cannot move back from `sent`; a wrong claim moves forward to `correction-owed` or `standing`.
- `correction-owed` is a non-terminal rung, never `is_approved`, so it shows as a `board` column.
- Reaching `cleared` needs one approval record; reaching `corrected` two in total (`requires:` counts all records).

## [0.22.0] - 2026-08-26

### Added

- `trace_spec::codex` reads codex-cli session rollout JSONL into the same `TraceIr` the Claude adapter produces.
- The Codex reader leaves `tools` and `is_error` as `None` and `operations` empty instead of inferring them.
- The Codex reader refuses, by line number, a rollout record that begins mid-line instead of recovering it.

### Fixed

- `protocol ess impact` no longer counts files the `--from` model derives nothing for as owed; they print apart.
- The generated `horizon` schema accepts an integer up to 3650 or a duration string such as `7d`, as the parser does.
- Documented scenario and claim counts now match the counts their gates print.

## [0.21.0] - 2026-08-26

### Added

- A task may declare `subject`; evidence about a different subject is refused before it is recorded, naming both.
- Once a task declares `subject`, evidence naming no subject is refused too; a task without one is unchanged.
- `task.subject` is a readable fact, so a rule may condition on it.
- `requires:` accepts `advisory` entries, evaluated and reported but never blocking.
- An `advisory` entry must name `owner` and `exit_criterion`, or it is refused at parse time.
- `RequirementReport::advisory_gaps()` counts advisory failures; `unmet()` excludes them.
- A `command` step may set `circuit_breaker: <n>` with `depends_on`; after n failures its steps are skipped.
- A step skipped by a circuit breaker is recorded as skipped, never as failed.
- A breaker with no `depends_on`, an empty dependency name or a threshold of `0` is refused at load.
- `protocol drive` writes a `trace-spec/1` document per `llm` step with a `tool.absent` row per refused operation.
- Refusal rows use neutral operation names and `on_unknown: gap`; a state that refuses nothing writes no document.

## [0.20.0] - 2026-08-26

### Added

- `protocol artifact evidence <id> --kind <kind> --source <source> --ref <url>` records evidence about one artifact.
- `protocol artifact move` finds evidence recorded for that artifact; evidence about another artifact does not count.
- `move --evidence` still works; such a move is labelled as decided on asserted evidence in output and history.

### Changed

- The journal's `moved` entry carries `decided_on`; entries written by 0.19.0 read back with an empty account.

## [0.19.0] - 2026-08-25

### Added

- The planning store keeps a journal: every create, move, relate and body write appends who, when, revision and change.
- `protocol artifact history <id>` prints an artifact's journal entries.
- The journal is append-only; an unparsable line is skipped and the number of skipped lines is printed.

## [0.18.0] - 2026-08-25

### Added

- New `blocker` kind; `credential-blocker`, `person-blocker`, `decision-blocker` and any `*-blocker` share its ladder.
- `cleared` is terminal for a blocker; being blocked again is a new blocker.

### Changed

- The relation vocabulary stays closed: relation names are acted on by the engine, not only recorded.

### Fixed

- `artifacts/relations/relations.yaml` now has a row for `delivers`, which the engine already accepted.

## [0.17.0] - 2026-08-25

### Added

- New `obligation` kind in `artifacts/lifecycles/obligation.yaml`: `open` to `met` or `slipped`, `slipped` to `met`.
- `slipped` opens only after the obligation's `due` date has passed.
- No ladder can wait on an obligation, so an obligation never blocks other work.

### Fixed

- A refusal about a kind beginning with a vowel says "an", not "a obligation".

## [0.16.0] - 2026-08-25

### Added

- Lifecycle documents accept `when:` with `after: <field>`; the rung stays shut until the artifact's own date passes.
- `protocol artifact move --at <instant>` sets the instant a dated rung is judged at; default: system clock.
- A dated move is unobservable when no instant, no recorded date, or an unreadable instant such as `yesterday` is given.
- `after` is strict: the rung opens after the date, not at it.

### Changed

- **Breaking:** `RefusalReason` renames `EvidenceUnobserved`/`EvidenceInsufficient` to `Unobservable`/`NotEarned`.
- **Breaking:** `move_status` also takes the instant to judge a dated rung against.
- `entity-core` moves to release tag 0.5.0.

## [0.15.0] - 2026-08-25

### Added

- Lifecycle documents accept `requires:`, naming the evidence kind and count a status needs.
- `artifacts/lifecycles/story.yaml` requires at least one `test_result` to reach `implemented`.
- `protocol artifact move --evidence <kind>=<n>` presents evidence; refusals tell nothing presented from too little.

### Changed

- **Breaking:** `PlanningDocument::move_status` takes the evidence on hand and returns `Box<MoveRefusal>`.
- **Breaking:** `MoveRefusal` carries a `reason`.
- `entity-core` moves to release tag 0.4.0.

## [0.14.0] - 2026-08-25

### Added

- `protocol reverse init` writes `.engineering/project.yaml` and checks the source resolves (`--no-verify` skips).
- `reverse init` refuses an unpinned `git+` source or an absolute path, and leaves no file if resolution fails.
- `protocol reverse scan` emits a deterministic `aep.reverse-scan/1` bundle of repository facts, each with `path:line`.
- `protocol reverse openapi` drafts an `ess/1` domain from an OpenAPI document, marking open choices `UNMAPPED:`.
- `protocol reverse history` reports `line_ages`, `stated_expiry`, `reverted`, `churn`, `dormant`, `tickets` from git.
- `reverse history` reads no clock, records author counts not names, and warns on stderr about a shallow clone.
- `reverse scan` reports `disabled_tests`, with each test's stated reason and whether anything can still enable it.
- `artifacts/lifecycles/vision.yaml` gives `vision` a ladder: `specification`'s without `implemented`.
- `integrations/claude-code/agents/reverse-engineer.md` drafts a first plan from `reverse scan`, citing `path:line`.

### Changed

- **Breaking:** `protocols:` in `.engineering/project.yaml` refuses an absolute path; use relative or pinned `git+`.
- Absolute paths (`/`, `~`, drive letter, UNC) are refused the same way on every platform, for every path field.

## [0.13.0] - 2026-08-25

### Added

- `.engineering/project.yaml` declares a relative `schemas` registry path, defaulting to `.engineering/schemas`.
- `protocol schema validate` validates JSON instances offline against the registry, matching `schema` to a `$id`.
- `protocol schema typescript` produces a deterministic TypeScript projection of a registry schema.
- `protocol artifact body <id> --from <path|->` replaces an artifact's markdown body and keeps CLI-owned frontmatter.

### Changed

- Artifact statuses are open: a lifecycle document may declare any rung, not only an `ArtifactStatus` variant.
- A status is accepted for a write only if the kind's lifecycle declares it; `artifact list --status` accepts any name.
- A rung unknown to the code is never `is_approved` or `is_retired`.
- Descriptor documents under `adp-`/`aop-domain` still accept named statuses only.
- **Breaking:** `ArtifactStatus` is no longer `Copy`, `as_str` borrows, `ArtifactLifecycle::permits` takes references.
- The three generated schemas publish a status as a patterned string instead of a closed `oneOf`.
- The status ladder is decided as data by `entity-core`; every move gets the same verdict as before.
- `aep-backend-markdown` depends on `entity-core`, pinned by git revision.
- Single-writer store: direct `Edit`, `Write` or `NotebookEdit` under `.engineering/planning/**` is denied.
- The Codex and Claude integrations ship a `schema-contracts` skill.

## [0.12.0] - 2026-08-24

### Added

- An `llm` step in `aep.driver-steps/1` accepts `context:` (files handed to the run) and `scope:` (write rules).
- A `scope:` rule sets `write:` to `allowed`, `partial-only` or `denied`; first match wins and the last must name `**`.
- `protocol eval run --case <dir> --arm <arm> --harness <harness> --out <dir>` runs one eval arm through `metaharness`.
- `protocol eval run` exits 2 when `metaharness` is not installed.
- `protocol eval run` spawns nothing without `METAHARNESS_LIVE=1` and `--budget-usd`; cap checked before each run.
- `--assume-usd-per-run` (default `$0.25`) is charged for a run whose stream states no cost.
- `--arm driven` reads an existing stream via `--stream <file>`; `--stream` re-ingests a run without spending.
- `protocol eval run` requires `--observed-at`.
- The run manifest is read from `session.started`; a missing field is refused as `EVAL-STREAM-003` to `EVAL-STREAM-012`.
- Arm `plugin` without an attested plugin, arm `raw` with one, or a plugin without digest is refused.
- `task check` runs the three-arm eval pipeline over committed streams and compares the matrix byte for byte.
- `integrations/workflow-coverage.yaml` maps each workflow state to the skills and agents that teach it, or to a gap.
- The gate fails when a workflow or state has no entry in `integrations/workflow-coverage.yaml`.
- `conformance/eval/` holds eval cases (task, `trace-spec/1` expectations, transcript); the gate replays every one.
- `protocol workflow instruct --id <id>` renders a workflow as prose; without `--id` it writes all to `--out DIR`.
- Rendered instructions are committed under `generated/instructions/` and held byte-identical by the gate.
- `protocol eval matrix` tabulates checked runs as held, contradicted and unobservable, per arm, harness and workflow.
- `eval.run-manifest/1` records arm, harness, workflow, case, model, harness version, plugin digest and optional cost.
- `protocol eval matrix` always exits `0` and refuses bad input with `EVAL-MANIFEST-*` and `EVAL-PAIR-*` codes.
- Canonical `metaharness.frame/1` committed at `crates/protocol-cli/fixtures/metaharness-frame-canonical.json`.
- `protocol drive run` refuses a step map that cannot produce evidence the plan needs; `--allow-evidence-gap` overrides.
- A driven step's tool calls go through `Engine::authorize`; an engine denial overrides the driver's allow.
- Tool decisions land in the execution and in `snapshot.json`; a `tool.decided` reason names the refusing layer.
- `protocol trace check|inspect|evidence` reads `metaharness.event/1` streams, picking the reader from the first line.
- The event-stream reader reads `skill.completed`, `tokens.thinking`, `iterations`, `speed`, per-model `cost.total`.
- A `null` value for any of those fields reads as `unk`.
- `harness: metaharness` on an `llm` step spawns through `metaharness run claude` with a sealed `metaharness.frame/1`.
- New expectation kind `env.mcp_servers` bounds the MCP servers in a session; a missing field reads `unk`.
- Eval sessions launch with `--strict-mcp-config`, and `env.mcp_servers` gates at `{at_most: 0}`.
- New step map `drivers/development/checks.yaml` for work accepted by checks; `--map` picks between fitting maps.
- A `command` step accepts `evidence.record: <path>` and submits the document a verifier wrote.
- Step `run` words and `record:` expand `{run_directory}` and `{transcript}`; an unknown placeholder is refused at load.
- Codex planning instructions ship at `integrations/codex/`; `task codex-eval` checks them offline.
- `protocol contract evidence --record <file> --observed-at <date>` turns a contract runner's record into evidence.
- `protocol contract evidence --record -` reads the record from standard input.
- `protocol contract evidence` refuses a record with `checked: 0` or with `breaking_changes` above `failed`.

### Changed

- Every `llm` step in `drivers/development/default.yaml` names the planning skill as `context:` and declares `scope:`.
- `principles/development/contract-testing.yaml` requires `contracts.breaking_changes == 0` before review as well.
- `protocol contract evidence` refuses a record that omits or nulls `checked`, `failed` or `breaking_changes`.
- `conformance/README.md` documents all five tenants, including `trace/` and `eval/`.
- `protocol drive run` and `protocol drive resume` refuse at launch when `metaharness` is not installed.
- Every `llm` step streams through `metaharness run claude --decisions ask`; the driver decides each tool request.
- Eval logic, transcripts and results moved to metaharness `evals/aep/`; trace expectations to `conformance/trace/`.

### Removed

- `integrations/claude-code/hooks/` is deleted; the Claude plugin is skills and agents.
- `hook-decisions.jsonl`, the step-context file, the settings file and `claude_argv` no longer exist.

### Fixed

- A selector whose `subject:` was written but could not be read is now refused instead of deciding nothing.
- A `trace-spec/1` call selector takes a set, `tools: [Edit, NotebookEdit, Write]`; `tool:` beside `tools:` is refused.
- An empty tool set, a blank tool name or a repeated one in a selector is refused.
- An eval run is charged the cost its stream stated, rounded to the millionth, not the `--assume-usd-per-run` estimate.
- A stated cost that cannot be converted is refused as `EVAL-STREAM-011` and no manifest is written.
- `protocol eval run` prints each run's charged cost and whether it was stated.
- The plugin digest is read from the instrument's installed-plugins record, not the vendor's plugin list.
- A run manifest may record `model` as an explicit `null` when the harness named none; an omitted key is still refused.
- `integrations/codex/` is restored: the Codex planning plugin stays in this repository.

## [0.11.0-ground-truth-and-docs] - 2026-08-22

### Changed

- `llm` step context gains `reaching`, the unmet requirements on each way out (additive to `aep.drive-step-context/1`).
- An `operator` step pauses once; `protocol drive resume` continues from the next step and the exit guard decides.
- The driver launches model sessions with `--strict-mcp-config`, so a driven session gets no MCP servers.
- Public docs, guides and website pages updated to match the current CLI, evidence formats and driver.
- `aep-backend-markdown` scans backticked mid-line annotations and ignores annotations inside fenced code blocks.

### Fixed

- `protocol evidence inspect` no longer refuses a record observed on the reference date; only a later date is future.

## [0.10.0-horizons-dogfood-lab] - 2026-08-21

### Added

- **Breaking:** evidence records require a caller-supplied `observed_at`; `produced_at` stays the engine's entry time.
- Evidence whose `observed_at` is in the future is refused (`observation_in_future`).
- An evidence requirement may declare a `horizon` such as `3d`; past it the requirement reads `Unknown` and moves fail.
- Facts from a lapsed record are withheld under the strictest horizon the plan declares for its kind.
- `evidence.lapsed` reports stale evidence separately from `evidence.missing`.
- Re-submitting an identical record does not restore a lapsed requirement; only a newer observation time does.
- `aep-backend-markdown` scans one-line dated-claim annotations and reports per-file coverage and divergence.
- New verbs `protocol evidence scan` and `protocol evidence inspect`.
- `--observed-at` on `ess conform evidence` and `trace evidence`.
- The website `/lab` runs the billing specification's browser realization `billing_web_realized.wasm` live.
- `task lab` builds the lab module; it is not committed and the page says when it is missing.
- `AEP_PROJECT_DIR` renames the project directory (default `.engineering`); it is read once per process.

### Changed

- `docs/guide/adopting.md` states that the project-local merge covers only principles and profiles.

### Fixed

- A lifecycle document may omit `kind:` and becomes the tree's fallback lifecycle; a second fallback is refused.
- A custom kind's parent is the kind its last hyphen segment names (`observation-log` is a `log`); aliases excluded.
- `on_failure` refuses parameters its action does not take, naming every invented key; the JSON Schema matches.

## [0.9.0-harness-waves-2-3] - 2026-08-21

### Added

- `protocol drive run` drives a workflow: runs each state's steps, submits verifier evidence, asks the engine to move.
- Step maps `drivers/<family>/<name>.yaml` are a new document kind and must pin a workflow version.
- Step kinds: `command` (records `producer: verifier`), `operator` (pauses, exits 0), `llm` (cannot carry evidence).
- Runs persist under `.engineering/runs/<task>/<n>/` with engine snapshot, cursor, step logs and transcripts.
- A blocked run prints the engine's refusal verbatim; a crashed step submits no evidence.
- One run per store, guarded by a lock naming its holder; `--resume` refuses if workflow, step map or engine changed.
- Plugin `PreToolUse` hook `store-integrity`, always on, denies hand-edits of machine-owned planning frontmatter.
- Plugin hook `driven-surface` limits a driven shell to single `protocol artifact` or `protocol trace` invocations.
- Both hooks fail closed without `jq` or `python3` and log each decision to the run's `hook-decisions.jsonl`.
- Profile `development.driven`: `development.standard` plus `command.execute`, for runs under `protocol drive` only.
- Eval `integrations/claude-code/eval/run-driven.sh` drives a real model through allowed and refused steps.
- `protocol workflow render` draws a workflow as `svg`, `html`, `png` (via `rsvg-convert`) or `tui`.
- `protocol workflow render --run` overlays a run's path, visits, evidence and verbatim refusals; `--watch` redraws.
- `protocol workflow render` output is byte-stable for the same workflow and run.

## [0.8.0-harness-wave-1-trace-wave-1] - 2026-08-21

### Added

- `protocol trace check` judges a harness transcript (Claude Code `stream-json`) against a `trace-spec/1` document.
- `trace-spec/1` has 49 expectation kinds: completion, tool calls, ordering, tokens, cost, cache, rate limits, timing.
- `protocol trace check` verdicts are `ok`, `gap` or `unk` (exit 0/1/3), each citing transcript event indices.
- `protocol trace inspect` prints the transcript census: event families, per-tool traffic and per-step timing.
- The plugin eval's assertions moved to `integrations/claude-code/eval/expectations.trace.yaml`.
- `protocol trace evidence` turns a passing check into an `Evidence::TraceConformance` record.
- `protocol artifact` stores epics, stories, tasks and initiatives under `.engineering/planning/<kind>/<slug>.md`.
- `protocol artifact` verbs: `new`, `move`, `relate`, `list`, `board`, `graph`, `validate`, `kinds`, `relations`.
- `protocol artifact lifecycle` prints a kind's lifecycle.
- A refused `protocol artifact move` names the statuses reachable from the current one.
- `protocol artifact validate` reports every problem, not the first, and exits 1.
- Artifact ids are declared as `<kind>:<slug>`, must match the file path, and are never allocated by a counter.
- Planning file format `aep.planning-md/1` with `schemas/generated/planning-document.schema.json`.
- The `format:` line in a planning file is optional; unknown frontmatter keys are preserved.
- Lifecycles for `epic`, `task` and `initiative`, mirroring the `story` ladder.
- Claude Code plugin at `integrations/claude-code/`: a `planning` skill and agents `decomposer` and `plan-reviewer`.
- Plugin eval `integrations/claude-code/eval/run.sh` runs a hermetic headless session and checks the resulting store.
- Plugin eval reports carry run metrics and an advisory adversarial review.

### Changed

- `docs/VISION.md` brings a reference driver into scope; gates are still evaluated only by the engine.

## [0.7.1-infra-waves-1-4] - 2026-08-21

### Added

- `protocol infra project --spec <file> --path <bundle|ir> --out <dir>` writes patch files that close gaps.
- `protocol infra project` writes values only from a gap or a `remedy:`; undecidable values become obligations.
- `protocol infra project` patches the observed objects, using strategic merge patches for container changes.
- `protocol infra project` also closes gaps its own patches induce, such as a missing disruption budget.
- `protocol infra project` writes `OBLIGATIONS.md` and `SUMMARY.md`; a contradicted expectation is refused.
- `infra-spec/1` expectations may carry a `remedy:`; it never changes a verdict.
- New refusals `INFRA-SPEC-009` (remedy its kind cannot write) and `INFRA-SPEC-010` (empty or ill-typed remedy).
- `examples/k3d-dev-cluster/projection/`: a committed patch tree, drift-checked by `cargo xtask infra --check`.
- `protocol infra simulate --spec <file> --path <bundle|ir>` checks an `infra-spec/1` desired state.
- `protocol infra simulate` answers `ok`, `gap` or `unk` with a reason; a scope selecting nothing is `unk`.
- `infra-spec/1` has twelve expectation kinds and scopes of cluster, namespace or label set.
- `protocol infra simulate` exits 0 whatever the verdicts; exit 1 means the input could not be simulated.
- `protocol infra diff --from <ir> --to <ir>` reports sixteen typed changes in declared state between two scans.
- `protocol infra diff` names changed configuration keys, never values, and refuses scans from different contexts.
- `examples/k3d-dev-cluster/` adds `expected.yaml`, `observation.drifted.json`, `simulation.json` and `drift.json`.

### Changed

- `cargo xtask infra` writes three documents and `task infra-check` checks all three; the CI job is renamed.

## [0.7.0-ess-wave-7] - 2026-08-21

### Added

- `examples/gatepass/` specification; `protocol ess synthesize` emits Rust and Go HTTP servers with one surface.
- Components may declare `reached_by: network` (default `in_process`); only then is an HTTP server emitted.
- A network component serves each view at `GET /{domain}/views/{view}`; one with no command or view is refused.
- Synthesized servers serve `GET /openapi.json` and `GET /docs` from the committed contract and docs.
- Synthesized servers share one route and status mapping in `ess_gen::http` and answer 400, 404, 405 and 501.
- Synthesized servers take no dependencies: `std::net::TcpListener` in Rust, `net/http` in Go.
- Realizations in `examples/gatepass-realization/` and `examples/gatepass-go-realization/`.
- A server's startup record separates model-derived fields from a `runtime` member (language, address, port).
- `protocol ess synthesize --target web` emits a `WebAssembly` bridge and page under `generated/web/`.
- The web page builds itself at load time from the module's `catalog.json`.
- The web target needs no `wasm-bindgen` or third-party crate; `examples/billing-web/` links a realization.
- The web target lists its limitations in `TARGET.md` and refuses the network transport.
- `protocol ess synthesize --target go` emits a standard-library-only Go module under `generated/go/`.
- The Go target encodes unions as sealed interfaces and lifecycles as per-state types; `TARGET.md` lists weakenings.
- The Go target refuses `Map<Bytes, _>` and colliding obligation method names as target-stage refusals.
- `protocol ess diff` compares entities, commands, views and bindings with 74 new typed change kinds.
- `protocol ess diff` compares predicates by canonical equality; `ess-diff/1` is unchanged.
- `protocol infra view --path <bundle|ir> [--namespace <ns>]` writes and opens an HTML component view.
- `protocol infra graph --format html` prints the HTML component view to stdout.
- Infra observation reads replicasets, jobs, cronjobs, pod disruption budgets and autoscalers, each optional.
- Pod ownership follows observed `ownerReferences`, falling back to the `pod-template-hash` label.
- Diagnosis rules `INFRA-DIAG-015` to `INFRA-DIAG-020` and invariant candidates `INFRA-PROP-001` to `INFRA-PROP-003`.
- Infra analysis adds a severity-ranked directions summary and `infra_analyze::render_html`.
- Generated artifacts carry a `contract_digest` of their model slice beside `source_digest`.
- **Breaking:** a conformance suite document requires `contract_digest`; regenerate suites from before this release.
- **Breaking:** `protocol ess impact` emits `ess-impact/2`; `--suite` is optional and `--generated <dir>` is new.
- `protocol ess impact` lists owed generated artifacts and refuses a suite whose contract digest does not match.
- `protocol infra graph` builds a typed dependency graph of the observed cluster as `json` or `mermaid`.
- `protocol infra diagnose` reports findings under `INFRA-DIAG-001` to `INFRA-DIAG-014`; exits 0.
- `protocol infra diagnose --min-severity` filters the listing and keeps the totals.
- `graph`, `diagnose` and `inspect --properties` accept a compiled `infra-ir/1` document, re-verified on read.
- `protocol infra inspect --properties` prints per-workload replicas, parsed images and resource envelopes.
- Infra observation records a waiting container's reason and a claim's phase.
- `protocol infra validate`, `compile` and `inspect` turn an `infra-observation/1` bundle into `infra-ir/1`.
- A bundle with a plain-string secret value is refused (`INFRA-SECRET-001`) without echoing it.
- Configmap values are digested at validation; keys survive, content does not.
- `examples/k3d-dev-cluster/`: a trimmed observation and its compiled IR, drift-checked by `task infra-check`.

### Changed

- **Breaking:** the `infra-ir/1` model grew, so IRs compiled before this release carry different digests.
- `task check` needs the `wasm32-unknown-unknown` target and Node beside the Go toolchain.
- `generate-check`, `suite-check` and `synth-check` report a stale contract digest as its own finding.

## [0.6.1-ess-wave-6.5] - 2026-08-21

### Added

- The build fails when an engine constructs an evidence payload outside its test code.
- The build fails on a clock, RNG or unordered map in `aep-domain` or `ess-gen`.
- The build fails on a new public trait method beside `CommandService::execute` and the seven queries.
- Property tests (`proptest`, fixed seed) check the Kleene laws of `Truth` and adversarial specification compilation.
- `PROPTEST_CASES` widens a local property-test run.
- `payload:` on a command outcome maps an emitted event's fields onto the command's input or a literal.
- New diagnostic `ESS-COMMAND-003` refuses a `payload:` field the event does not carry.
- Synthesis asserts declared event payload values; an unmapped field is checked for presence and type only.
- New scenario family `<type>/invariant/at/<view>/<field>` checks a value object's invariants at each view position.
- A value-object invariant with no witness is refused as `ESS-SYNTH-013`.
- `ess impact` invalidates the whole suite (`Invalidation::Whole`) when an uncompared construct family changed.

### Changed

- The model digest is the full 64-hex-character SHA-256 in provenance headers, projections and suites.
- `SpecDigest` still parses a 16-character digest; a comparison against it fails and names both digests.

## [0.6.0-ess-wave-6] - 2026-08-20

### Added

- `protocol ess synthesize --path <spec> [--out <dir>] [--target rust]` writes a synthesised workspace.
- A `SynthesisPlan` gives every capability one disposition: generated, obligation or refused, each with its reason.
- The Rust emitter writes a zero-dependency workspace with newtypes, enums, outcome enums, row types and typestate.
- A lifecycle transition the specification refuses is a method that does not exist in the generated typestate.
- `PLAN.md` and `plan.json` are written inside the generated workspace.
- Synthesis emits one crate per component with typed command handlers, view queries and an event outbox.
- A generated system crate wires bindings over an in-process `at_least_once` transport.
- A binding whose command zero or several components accept is refused.
- Every obligation is a typed stub returning `UnmetObligation` instead of panicking.
- `generated/rust/billing/` is committed; `cargo xtask synth` regenerates it.
- `cargo xtask synth --check` fails on drift and runs `cargo check` in each workspace; it runs in `task check` and CI.
- `examples/billing-realization` provides hand-written implementations of the billing workspace's obligations.
- `synth-check` runs the committed billing suite against the synthesised workspace and against a faulty variant.
- The linker takes exactly one implementation per obligation; a missing or ambiguous one is a reported error.
- The generated system crate records `BindingInvocation` per binding call and gains `redeliver`.

### Fixed

- A binding's failure policy applies only to the declared failure outcome; an `UnmetObligation` propagates from `pump`.

## [0.5.0-ess-wave-5] - 2026-08-20

### Added

- `protocol ess impact --from <dir> --to <dir> --suite <file>` lists the scenarios a change invalidates.
- Each impact carries the dependency path that produced it, one hop per line.
- `ess impact` only narrows: a system header change or an unknown dependency invalidates everything.
- `ess impact` refuses a suite whose digest does not match the earlier revision.
- `protocol ess diff --from <dir> --to <dir>` reports typed semantic changes between two specification revisions.
- `ess diff` ignores file renames, reordering, comments and written-out defaults.
- `ess diff` compares the system header, types, events, errors, actors and components.
- A grant or enum variant added is marked as widening, one removed as narrowing.

## [0.4.0-ess-wave-4] - 2026-08-20

### Added

- A specification generates conformance scenarios for outcomes, external branches, lifecycles, invariants and bindings.
- The generated suite is checked against ten injected faults, each with a blast-radius allowance.
- `wrong_state: true` with `error:` declares the error for a command attempted in the wrong lifecycle state.
- The `wrong_state` branch appears in OpenAPI as `409`.
- Scenarios assert that events of branches not taken were not emitted.
- A read-your-writes check fails when the command returned no consistency token.
- Event payloads are checked for the presence and type of every declared field.
- A view assertion names the instance the scenario acted on.
- `protocol ess conform` adds `synthesize` (write a suite) and `run` (execute one).
- `protocol ess conform run` exits `0` conformant, `1` contradicted, `3` when the run could not be carried out.
- Generated suites are committed under `suites/generated/` and drift-checked in the gate and in CI.
- The suite index lists every construct that got no scenario, with the reason.
- The `ConformanceTarget` trait runs a generated suite against an implementation with byte-identical reports.
- A scenario status `unsupported`; a required scenario ending in it fails conformance.
- Binding scenarios check the mapping per field, duplicate delivery under `at_least_once` and the escalation event.
- `on_failure: drop` produces a recorded refusal instead of a scenario.
- `protocol ess graph --format mermaid`; `dot`, `json` and `yaml` too, `--format text` remains an alias of `dot`.

### Changed

- **Breaking:** an outcome with `creates:`, `moves:` or `updates:` must name `instance:`, or it is refused.
- For `creates:`, `instance:` names a field of the emitted event rather than of the input.
- An artifact evidence record carries its kind as `artifact_kind` on the wire.

### Fixed

- An artifact evidence record could not be read from a document and failed with `missing field 'kind'`.
- `protocol ess graph` and the documentation page now draw the same graph, including actors and grants.

## [0.3.3-ess-wave-3.5] - 2026-08-20

### Added

- A command outcome declares the entity it acts on with `creates:`, `moves:` or `updates:`.
- **Breaking:** a lifecycle transition no outcome takes is refused as `missing_causation`.
- The published schemas accept every alias the parsers accept, such as `component:`, `id:`, `type:` and `require:`.
- Conformance evidence names its specification by model digest and satisfies only that revision.
- A specification artifact with no recorded model digest leaves its conformance requirement owed.
- `ess-conformance` decides a guard against a candidate command input, including an unevaluable outcome.
- **Breaking:** `on_failure: escalate` requires `escalate:` with `emits:` naming a declared event.
- A property-test result carries a `seed` string that reproduces it.

### Changed

- **Breaking:** `version: 4294967296` is refused instead of saturating to `4294967295`.
- **Breaking:** a YAML mapping key written twice is refused in every document, not only in specifications.
- **Breaking:** a number JSON cannot round-trip, such as `1e400` or `.nan`, is refused.
- A type or predicate nested deeper than 32 levels is refused instead of overflowing the stack.

### Fixed

- A recorded refused approval no longer authorises the action it refused.
- A capability a principle denies can no longer be downgraded to requiring an approval.
- An approval floor on `deployment.create:production` now catches a profile granting `deployment.create`.
- The audit trail refuses a record that claims a refusal and lists changed rows.
- A validated type can no longer be built from a document by `Deserialize`.

## [0.3.2-ess-wave-3] - 2026-08-20

### Added

- `generated/` holds Markdown, JSON Schemas, OpenAPI 3.1 and AsyncAPI 3.0 projected from `examples/billing/`.
- Every generated artifact records its specification, model digest and tool versions, also as `x-ess-provenance`.
- A named type keeps its own definition and reference in every projection.
- A command maps to `POST /{domain wire name}/commands/{command wire name}`, stated in the generated document.
- An AsyncAPI channel uses the event's `naming.wire` or qualified name; `x-ess-address-source` says which.
- Response status follows the outcome: `202` taken, `422` refused by input, `502` refused externally.
- A command invoked by an `at_least_once` binding requires an `Idempotency-Key` header.
- A binding's `delivery` and `on_failure` appear in the publisher's and subscriber's AsyncAPI documents.
- `task generate` regenerates projections byte-identically; `task generate-check` fails on drift in CI.
- `cargo xtask generate --check` fails on orphaned artifacts; `cargo xtask generate` removes them.
- `protocol ess generate --kind docs|schema|openapi|asyncapi` lists artifacts, or writes them with `--out`.
- `protocol ess generate --format json|yaml` carries artifact contents.
- Generated pages show entities with lifecycle diagrams, views with consistency, and actors with their commands.
- Every projection publishes the same schema for the same construct.
- **Breaking:** AsyncAPI payloads enforce `additionalProperties: false` and type patterns; messages may now fail.
- An operation lists its permitted actors in `x-ess-may-invoke`; no security scheme is generated.
- The compiled IR carries entities, views and actors in full.

## [0.3.1-ess-wave-2] - 2026-08-20

### Added

- A specification declares components, bindings and topology above its domains.
- A binding requires `delivery:` and `on_failure:`; `drop` must be written out.
- A binding's mapping between two bounded contexts is typechecked.
- `conversions:` declares an intended, directional type crossing and requires `because:`.
- `ess-compiler` resolves references into a normalized IR with coded `file:line` diagnostics.
- `protocol ess compile`, `ess inspect` and `ess graph` (DOT output).
- `ess inspect` resolves a name in seven namespaces and refuses an ambiguous one.
- Compiling the same source twice is byte-identical.

### Changed

- New validation codes `misspelled_reference` and `unsupported_construct`, distinct from `unsupported_format_version`.

### Fixed

- A type recursive through a union is accepted when one variant is buildable; the refusal names the unmet requirement.
- A mapping key written twice is refused with the key and line instead of silently kept.
- A binding mapping now reports an input mapped twice.

## [0.3.0-ess-wave-1] - 2026-08-20

### Added

- `ess-domain` models an Executable System Specification: domains, entities, commands, events, views, actors, types.
- `protocol ess validate --path <file-or-directory>` reports every problem with a code and location.
- `examples/billing/` is the normative example and is checked to exercise every construct.
- Commands declare outcomes; `external: <the cause>` marks a branch decided outside the input.
- A view declares its consistency.
- A declaration is addressable as `ep://acme/billing/ess-command/billing.invoice.CreateInvoice`.
- `schemas/generated/ess.schema.json` validates a specification in an editor.
- `docs/guide/specification.md` and `docs/VISION.md`.
- `docs/design/ess-implementor-design-v0.1.md` and `docs/design/ess-review-v0.1.md`.
- `ArtifactKind::ExecutableSystemSpecification` and `EvidenceKind::EssConformance`.
- The `ess-conformance` principle requires `independent: true` evidence from a `conformance-runner`.

### Changed

- Validation reports nine specific codes, such as `undeclared_reference`, in specifications and protocol documents.
- The published schemas accept what the parsers accept and are checked against every shipped document.
- **Breaking:** the versions `v01` and `ess/01` are refused.

### Fixed

- The published schema accepts `version: v3`.
- An error's payload types and an event's duplicate fields are checked.
- Refused: undeclared header domain, ownerless actor, mutually dependent or uninhabited types.
- Refused: a view filter on an unknown lifecycle state, a union with no tag field.
- A misspelt key in a type declaration such as `invarants:` is a parse error with a line number.
- A type's invariants are checked as predicates over its own fields.
- A field name must be a valid identifier.
- An entity invariant may read the identity field.
- A field may not shadow the identity's name.
- A state whose only transition is a self-loop is a dead end.
- A domain file accepts `naming:`.
- A malformed header no longer hides the reference errors under it.
- `protocol ess validate` names the file of a problem and refuses a directory that is not a specification.
- `protocol ess validate` reads each file once when a symlink points back up the tree.
- `cargo xtask schema --check` fails on a schema nothing generates any more.

## [0.2.1] - 2026-08-20

### Added

- `.engineering/project.yaml` names the protocol, profile and protocol tree of a project.
- `protocol resolve` and `protocol evaluate` run without arguments inside a project.
- `.engineering/principles/` and `.engineering/profiles/` merge over the protocol tree's.
- `protocol resolve` and `protocol evaluate` report where their inputs came from.

### Fixed

- Profiles written against `adp/1` or `aop/1` inherit the approval floor and cannot grant `production.write`.
- The CLI no longer panics when its output pipe closes.

## [0.2.0-wave-3] - 2026-08-20

### Added

- `aep-conformance`: sixteen black-box suites a backend runs against itself; failures name the property.
- Conformance levels `core`, `audited` and `full`.
- `FaultyBackend` breaks one property at a time; `Fault::caught_by()` names the suite that catches it.
- `protocol conformance --level core|audited|full [--suite <name>] [--inject <fault>]`.
- `adp-domain` with `adp.specification/v1`, `adp.test-plan/v1`, `adp.acceptance-criteria/v1` and `adp.change/v1`.
- `adp-domain` commands `adp.story.start/v1`, `adp.story.complete/v1` and `adp.test-plan.record/v1`.
- `adp.specification.satisfy/v1` refuses a specification declared satisfied by no evidence.
- `aop-domain` with `aop.incident/v1`, `aop.runbook/v1`, `aop.release/v1` and their commands.
- `aop.release.promote/v1` to production without an approval is refused.
- `docs/guide/` covers adopting the protocol, wiring a harness and implementing a backend.

### Changed

- **Breaking:** the in-memory backend refuses an update to an immutable type; archiving stays available.

## [0.2.0-wave-2] - 2026-08-20

### Added

- Identity types `EntityId`, `EntityLocator`, `EntityType` and `EntityRevision`.
- `ActorRef` (`human:`, `agent:`, `service:`, `system`); commands carry both an actor and an executor.
- `aep-contract` defines `CommandService`, `QueryService`, command envelopes and consistency tokens.
- `aep-contract` defines a typed failure taxonomy and `TypeDescriptor`.
- `aep-domain::command` with six generic and three domain commands.
- `aep-domain::domain_event`, a versioned event vocabulary with a `Custom` variant.
- `aep-domain::audit` records actor, executor, decisions, changes and rejected attempts.
- `aep-backend-memory`, an in-memory implementation of both contract surfaces.
- `aep-engine::trail` turns protocol decisions into audit records.
- Evidence may be submitted as an entity reference.
- `RelationKind::Delivers` and `ArtifactKind::entity_type()`.
- `protocol entity list|get|history|relations`, `protocol audit` and `protocol describe <type>`.

### Changed

- Nine new `ValidationCode`s, such as `self_reference` and `missing_causation`, replace `unknown_state` for those cases.
- Minimum supported Rust version is 1.85.
- A protocol may declare an approval floor; `aep/1` declares `production.write` and `deployment.create:production`.

## [0.2.0-wave-1] - 2026-08-20

### Added

- `aep-engine` resolves a task against a document tree into an execution plan.
- `aep-engine` evaluates what is owed, which actions and transitions are permitted, and whether a task is complete.
- `ProtocolEngine` trait with deterministic transitions and an injected `Clock`.
- 42 documents: `aep/1`, `adp/1`, `aop/1`, principles, workflows, profiles, lifecycles and templates.
- `protocol` CLI with `validate`, `resolve`, `inspect`, `evaluate`, `explain` and `schema`.
- `--format text|yaml|json` on the `protocol` CLI.
- Worked example `examples/development-passkeys/`.
- A protocol approval floor: capabilities no profile can grant outright.
- `Action::ProductionMutate` for production changes that are not deployments.
- GitHub Actions CI mirroring `task check`.

### Changed

- **Breaking:** evidence files spell the envelope's subject `about`, not `subject`.
- **Breaking:** `protocol evaluate` exits `0` whenever it produced a report, including a blocked execution.

### Fixed

- Missing evidence no longer counts requirements of conditional rules that did not apply.
- The approval floor is violated by any overlapping grant, not only an exact one.
- A task may name the base protocol its profile refines, such as `aep/1` with an `adp/1` profile.

## [0.1.0] - 2026-08-19

### Added

- `aep-domain`, the source-of-truth model of the protocol.
- `aep-schema` reads documents and generates JSON Schema for six document and four interchange types.
- `xtask schema [--check]` generates schemas from the Rust types and checks them in CI.
- Repository scaffolding: workspace, `Taskfile.yml` gate, Apache-2.0 licence, `AGENTS.md`.

[Unreleased]: https://github.com/beyond10x/aep/compare/0.77.0...HEAD
[0.77.0]: https://github.com/beyond10x/aep/compare/0.76.0...0.77.0
[0.76.0]: https://github.com/beyond10x/aep/compare/0.75.0...0.76.0
[0.75.0]: https://github.com/beyond10x/aep/compare/0.74.0...0.75.0
[0.74.0]: https://github.com/beyond10x/aep/compare/0.73.0...0.74.0
[0.73.0]: https://github.com/beyond10x/aep/compare/0.72.1...0.73.0
[0.72.1]: https://github.com/beyond10x/aep/compare/0.72.0...0.72.1
[0.72.0]: https://github.com/beyond10x/aep/compare/0.71.3...0.72.0
[0.71.3]: https://github.com/beyond10x/aep/compare/0.71.2...0.71.3
[0.71.2]: https://github.com/beyond10x/aep/compare/0.71.1...0.71.2
[0.71.1]: https://github.com/beyond10x/aep/compare/0.71.0...0.71.1
[0.71.0]: https://github.com/beyond10x/aep/compare/0.70.0...0.71.0
[0.70.0]: https://github.com/beyond10x/aep/compare/0.69.1...0.70.0
[0.69.1]: https://github.com/beyond10x/aep/compare/0.69.0...0.69.1
[0.69.0]: https://github.com/beyond10x/aep/compare/0.68.0...0.69.0
[0.68.0]: https://github.com/beyond10x/aep/compare/0.67.0...0.68.0
[0.67.0]: https://github.com/beyond10x/aep/compare/0.66.0...0.67.0
[0.66.0]: https://github.com/beyond10x/aep/compare/0.65.0...0.66.0
[0.65.0]: https://github.com/beyond10x/aep/compare/0.64.0...0.65.0
[0.64.0]: https://github.com/beyond10x/aep/compare/0.63.1...0.64.0
[0.63.1]: https://github.com/beyond10x/aep/compare/0.63.0...0.63.1
[0.63.0]: https://github.com/beyond10x/aep/compare/0.62.0...0.63.0
[0.62.0]: https://github.com/beyond10x/aep/compare/0.61.1...0.62.0
[0.61.1]: https://github.com/beyond10x/aep/compare/0.61.0...0.61.1
[0.61.0]: https://github.com/beyond10x/aep/compare/0.60.1...0.61.0
[0.60.1]: https://github.com/beyond10x/aep/compare/0.60.0...0.60.1
[0.60.0]: https://github.com/beyond10x/aep/compare/0.59.3...0.60.0
[0.59.3]: https://github.com/beyond10x/aep/compare/0.59.2...0.59.3
[0.59.2]: https://github.com/beyond10x/aep/compare/0.59.1...0.59.2
[0.59.1]: https://github.com/beyond10x/aep/compare/0.59.0...0.59.1
[0.59.0]: https://github.com/beyond10x/aep/compare/0.58.0...0.59.0
[0.58.0]: https://github.com/beyond10x/aep/compare/0.57.0...0.58.0
[0.57.0]: https://github.com/beyond10x/aep/compare/0.56.0...0.57.0
[0.56.0]: https://github.com/beyond10x/aep/compare/0.55.0...0.56.0
[0.55.0]: https://github.com/beyond10x/aep/compare/0.54.0...0.55.0
[0.54.0]: https://github.com/beyond10x/aep/compare/0.53.0...0.54.0
[0.53.0]: https://github.com/beyond10x/aep/compare/0.52.0...0.53.0
[0.52.0]: https://github.com/beyond10x/aep/compare/0.51.0...0.52.0
[0.51.0]: https://github.com/beyond10x/aep/compare/0.50.0...0.51.0
[0.50.0]: https://github.com/beyond10x/aep/compare/0.49.0...0.50.0
[0.49.0]: https://github.com/beyond10x/aep/compare/0.47.0...0.49.0
[0.47.0]: https://github.com/beyond10x/aep/compare/0.46.0...0.47.0
[0.46.0]: https://github.com/beyond10x/aep/compare/0.45.0...0.46.0
[0.45.0]: https://github.com/beyond10x/aep/compare/0.44.0...0.45.0
[0.44.0]: https://github.com/beyond10x/aep/compare/0.43.0...0.44.0
[0.43.0]: https://github.com/beyond10x/aep/compare/0.42.0...0.43.0
[0.42.0]: https://github.com/beyond10x/aep/compare/0.41.0...0.42.0
[0.41.0]: https://github.com/beyond10x/aep/compare/0.40.3...0.41.0
[0.40.3]: https://github.com/beyond10x/aep/compare/0.40.2...0.40.3
[0.40.2]: https://github.com/beyond10x/aep/compare/0.40.1...0.40.2
[0.40.1]: https://github.com/beyond10x/aep/compare/0.40.0...0.40.1
[0.40.0]: https://github.com/beyond10x/aep/compare/0.39.2...0.40.0
[0.39.2]: https://github.com/beyond10x/aep/compare/0.39.1...0.39.2
[0.39.1]: https://github.com/beyond10x/aep/compare/0.39.0...0.39.1
[0.39.0]: https://github.com/beyond10x/aep/compare/0.38.1...0.39.0
[0.38.1]: https://github.com/beyond10x/aep/compare/0.38.0...0.38.1
[0.38.0]: https://github.com/beyond10x/aep/compare/0.37.1...0.38.0
[0.37.1]: https://github.com/beyond10x/aep/compare/0.37.0...0.37.1
[0.37.0]: https://github.com/beyond10x/aep/compare/0.36.4...0.37.0
[0.36.4]: https://github.com/beyond10x/aep/compare/0.36.3...0.36.4
[0.36.3]: https://github.com/beyond10x/aep/compare/0.36.2...0.36.3
[0.36.2]: https://github.com/beyond10x/aep/compare/0.36.1...0.36.2
[0.36.1]: https://github.com/beyond10x/aep/compare/0.36.0...0.36.1
[0.36.0]: https://github.com/beyond10x/aep/compare/0.35.0...0.36.0
[0.35.0]: https://github.com/beyond10x/aep/compare/0.34.0...0.35.0
[0.34.0]: https://github.com/beyond10x/aep/compare/0.33.0...0.34.0
[0.33.0]: https://github.com/beyond10x/aep/compare/0.32.1...0.33.0
[0.32.1]: https://github.com/beyond10x/aep/compare/0.32.0...0.32.1
[0.32.0]: https://github.com/beyond10x/aep/compare/0.31.0...0.32.0
[0.31.0]: https://github.com/beyond10x/aep/compare/0.30.0...0.31.0
[0.30.0]: https://github.com/beyond10x/aep/compare/0.29.0...0.30.0
[0.29.0]: https://github.com/beyond10x/aep/compare/0.28.0...0.29.0
[0.28.0]: https://github.com/beyond10x/aep/compare/0.27.3...0.28.0
[0.27.3]: https://github.com/beyond10x/aep/compare/0.27.2...0.27.3
[0.27.2]: https://github.com/beyond10x/aep/compare/0.27.1...0.27.2
[0.27.1]: https://github.com/beyond10x/aep/compare/0.27.0...0.27.1
[0.27.0]: https://github.com/beyond10x/aep/compare/0.26.0...0.27.0
[0.26.0]: https://github.com/beyond10x/aep/compare/0.25.0...0.26.0
[0.25.0]: https://github.com/beyond10x/aep/compare/0.24.0...0.25.0
[0.24.0]: https://github.com/beyond10x/aep/compare/0.23.2...0.24.0
[0.23.2]: https://github.com/beyond10x/aep/compare/0.23.1...0.23.2
[0.23.1]: https://github.com/beyond10x/aep/compare/0.23.0...0.23.1
[0.23.0]: https://github.com/beyond10x/aep/compare/0.22.0...0.23.0
[0.22.0]: https://github.com/beyond10x/aep/compare/0.21.0...0.22.0
[0.21.0]: https://github.com/beyond10x/aep/compare/0.20.0...0.21.0
[0.20.0]: https://github.com/beyond10x/aep/compare/0.19.0...0.20.0
[0.19.0]: https://github.com/beyond10x/aep/compare/0.18.0...0.19.0
[0.18.0]: https://github.com/beyond10x/aep/compare/0.17.0...0.18.0
[0.17.0]: https://github.com/beyond10x/aep/compare/0.16.0...0.17.0
[0.16.0]: https://github.com/beyond10x/aep/compare/0.15.0...0.16.0
[0.15.0]: https://github.com/beyond10x/aep/compare/0.14.0...0.15.0
[0.14.0]: https://github.com/beyond10x/aep/compare/0.13.0...0.14.0
[0.13.0]: https://github.com/beyond10x/aep/compare/0.12.0...0.13.0
[0.12.0]: https://github.com/beyond10x/aep/compare/0.11.0-ground-truth-and-docs...0.12.0
[0.11.0-ground-truth-and-docs]: https://github.com/beyond10x/aep/compare/0.10.0-horizons-dogfood-lab...0.11.0-ground-truth-and-docs
[0.10.0-horizons-dogfood-lab]: https://github.com/beyond10x/aep/compare/0.9.0-harness-waves-2-3...0.10.0-horizons-dogfood-lab
[0.9.0-harness-waves-2-3]: https://github.com/beyond10x/aep/compare/0.8.0-harness-wave-1-trace-wave-1...0.9.0-harness-waves-2-3
[0.8.0-harness-wave-1-trace-wave-1]: https://github.com/beyond10x/aep/compare/0.7.1-infra-waves-1-4...0.8.0-harness-wave-1-trace-wave-1
[0.7.1-infra-waves-1-4]: https://github.com/beyond10x/aep/compare/0.7.0-ess-wave-7...0.7.1-infra-waves-1-4
[0.7.0-ess-wave-7]: https://github.com/beyond10x/aep/compare/0.6.1-ess-wave-6.5...0.7.0-ess-wave-7
[0.6.1-ess-wave-6.5]: https://github.com/beyond10x/aep/compare/0.6.0-ess-wave-6...0.6.1-ess-wave-6.5
[0.6.0-ess-wave-6]: https://github.com/beyond10x/aep/compare/0.5.0-ess-wave-5...0.6.0-ess-wave-6
[0.5.0-ess-wave-5]: https://github.com/beyond10x/aep/compare/0.4.0-ess-wave-4...0.5.0-ess-wave-5
[0.4.0-ess-wave-4]: https://github.com/beyond10x/aep/compare/0.3.3-ess-wave-3.5...0.4.0-ess-wave-4
[0.3.3-ess-wave-3.5]: https://github.com/beyond10x/aep/compare/0.3.2-ess-wave-3...0.3.3-ess-wave-3.5
[0.3.2-ess-wave-3]: https://github.com/beyond10x/aep/compare/0.3.1-ess-wave-2...0.3.2-ess-wave-3
[0.3.1-ess-wave-2]: https://github.com/beyond10x/aep/compare/0.3.0-ess-wave-1...0.3.1-ess-wave-2
[0.3.0-ess-wave-1]: https://github.com/beyond10x/aep/compare/0.2.1...0.3.0-ess-wave-1
[0.2.1]: https://github.com/beyond10x/aep/compare/0.2.0-wave-3...0.2.1
[0.2.0-wave-3]: https://github.com/beyond10x/aep/compare/0.2.0-wave-2...0.2.0-wave-3
[0.2.0-wave-2]: https://github.com/beyond10x/aep/compare/0.2.0-wave-1...0.2.0-wave-2
[0.2.0-wave-1]: https://github.com/beyond10x/aep/compare/0.1.0...0.2.0-wave-1
[0.1.0]: https://github.com/beyond10x/aep/releases/tag/0.1.0
