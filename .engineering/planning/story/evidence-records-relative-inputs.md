---
format: aep.planning-md/3
id: story:evidence-records-relative-inputs
kind: story
status: implemented
title: Evidence records keep no absolute local path, and a record that does not count says why
relations:
- serves: vision:O2
- informed_by: story:evidence-admits-generated-runner-profile
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T20:13:50Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T20:13:50Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-14T00:45:54Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
# Story: Evidence records keep no absolute local path, and a record that does not count says why

## Outcome

A record that `aep plan artifact evidence --from <report>` writes names its inputs by path relative
to the repository root, beside each input's digest. It never names them by the path as typed. An
input outside the repository root is recorded by its file name and digest only.

After recording, the command says, for the record just written, whether it counts toward a move. If
it does not, the command says why: for example, `does not count: failed, 3 of 40 scenario(s)
failed`, or `does not count: spec_digest differs from model_digest`. The line `on hand:
ess_conformance_v2=0` no longer appears without a reason beside it.

## Context

A consumer recorded an `ess_conformance_v2` report against a store with aep 0.71.2. The committed
record kept `report_input` and `suite_input` as absolute local paths. Gates' personal-paths check
refuses those paths, and they mean nothing on another machine.

Read on `main` (8ef7565e2):

- `crates/edge/aep-cli/src/planning.rs:7156` writes `report_input` and `suite_input` as
  `path.display()`.
- `:7275` does the same for the coverage record.
- `reference` is the typed path for all three ESS kinds: `:7167`, `:7252`, `:7286`.
- The `on hand` line prints counts only (`:7391-7395`). On a specification with a `model_digest`,
  a failed or other-digest record does not count. This is intended
  (`crates/edge/aep-cli/src/planning/conformance_count.rs:1-25`). The reason is computed there
  (`Discounted.reason`), but this command never prints it.

## Acceptance

- Spec first: `ess/domains/evidence.yaml` states the rule as an error, and `ess specify validate`
  passes with the newest `ess`. The rule: a recorded input path is relative to the repository root
  and carries a digest, or is a file name with a digest when the input lies outside the root.
  `cargo xtask ess` regenerates `generated/ess/`, and `ess-gate` passes.
- `--from <absolute path inside the repo> --suite <absolute path inside the repo>` records
  `report_input`, `suite_input` and `reference` as repository-relative paths, each with a `sha256:`
  digest.
- An input outside the repository root is recorded as its file name and digest. No component of
  its directory is kept.
- The same holds for `ess_conformance` (report/1) and `ess_conformance_coverage_v1` records.
- A failed report recorded against an `executable-system-specification` with a `model_digest`
  prints `does not count:` and the `Discounted` reason. A passed one prints `counts`.
- Records already in stores are read as before, absolute paths included. Nothing rewrites them.

## Out of Scope

`--ref` and `--source` text given by the caller on other kinds. They are the caller's words, not
paths the command resolved. The Gates scan is what holds them.
