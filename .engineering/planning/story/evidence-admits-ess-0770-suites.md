---
format: aep.planning-md/3
id: story:evidence-admits-ess-0770-suites
kind: story
status: active
title: The ESS pin moves to 0.77.0, and evidence admits the suite majors it writes
tags:
- ess
relations:
- serves: vision:O2
scope:
- confidence: cited
  path: .github/workflows/ci.yml
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/govern/aep-domain/src/ess_conformance_coverage/values.rs
- confidence: cited
  path: crates/govern/aep-domain/src/ess_conformance_v2.rs
- confidence: cited
  path: crates/observe/aep-ess-evidence
- confidence: cited
  path: ess/ess-inputs.yaml
- confidence: cited
  path: generated/ess
- confidence: cited
  path: xtask/tests/ess_gate.rs
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T16:38:55Z", actor: "human:timo", revision: 10}
- {from: "proposed", to: "active", at: "2026-10-30T16:38:55Z", actor: "human:timo", revision: 11}
---
# Story: The ESS pin moves to 0.77.0, and evidence admits the suite majors it writes

## Outcome

aep's ESS pin is the newest ESS release, 0.77.0 (released 2026-10-29), and `aep plan artifact
evidence <id> --from <report/2> --suite <suite>` records a report against every suite major ESS
0.77.0 writes, instead of refusing a major added after /57 as `UnsupportedSuiteVersion`. A major
newer than the ones this build lists is still refused by name.

## Context

The operator's rule (2026-10-29): a repository's `ess` pin moves to the newest ESS release when a
change touches the repository anyway, in the same wave. This wave touches aep. The pin is 0.69.2
(`.github/workflows/ci.yml` `ESS_VERSION`, `ess/ess-inputs.yaml`, `xtask/tests/ess_gate.rs`), held
there by `story:evidence-admits-ess-0692-suites`, whose acceptance 6 makes `ess-gate` fail when the
pin moves past the release the admitted lists were transcribed from. ESS 0.77.0's `CHANGELOG.md`
names `ess-conformance/58`, a major after the /57 aep admits today (read from the ESS tag; the full
set of majors 0.77.0 writes and reads is for the implementor to establish from its readers).

## Acceptance

Following `story:evidence-admits-ess-0692-suites`:

1. `ESS_VERSION` and `ESS_SHA256` in `.github/workflows/ci.yml` are 0.77.0's (the
   `x86_64-unknown-linux-gnu` line of the release's `SHA256SUMS`); `ess/ess-inputs.yaml` and
   `xtask/tests/ess_gate.rs` name 0.77.0; `cargo xtask ess` with ESS 0.77.0 rewrites `generated/ess/`
   and the result is committed; `ess-gate` passes.
2. The admitted ordinary and coverage suite major lists extend to every major ESS 0.77.0 writes and
   reads, and name 0.77.0 as the release they were transcribed from.
3. A committed suite of the newest ordinary major and of the newest coverage major, each with its
   report/2, written by released ess 0.77.0 with a note naming the commands, is recorded by the CLI
   (exit 0, the report's counts).
4. The next major after the newest admitted is refused by name as `UnsupportedSuiteVersion`.
5. Each vocabulary ESS added in the new majors that AEP reads is gated by major: present under a
   lower major, refused as `UnsupportedVocabulary`; at or above its major, admitted.
6. `CHANGELOG.md` gains an Unreleased entry.

## Out of scope

- Moving the ESS plugin or the machine's `ess` CLI.
- If ESS 0.77.0 changed the meaning of an existing scope in a way the readers cannot admit by an
  explicit major list, this unit leaves the wave and the finding is filed as its own story.
