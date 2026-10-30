# Suites ESS 0.77.0 writes

ESS 0.70.0 added the suite pair /58 and /59 (`exact_decimal`): a `Decimal` in an exact observer, the
complete subject a refusal leaves unchanged or a retained command result. 0.77.0 writes and reads
majors 1–59. No byte below was edited after `ess 0.77.0` wrote it, and `synthesize` wrote the same
bytes on a second run.

`wrong-state-decimal.spec.yaml` is ESS's own case `WRONG_STATE_DECIMAL` in
`crates/verify/ess-conformance/tests/suite.rs` at ESS tag `0.77.0`, with `{format}` replaced by
`ess/32`. Its wrong-state refusal snapshots a complete subject whose `price` is a `Decimal`, so ESS
writes the ordinary major /58 and the coverage major /59. These four files were written from this
directory:

| File | Command |
| --- | --- |
| `suite-58.json` | `ess verify conform synthesize --path wrong-state-decimal.spec.yaml --target ir --out suite-58.json` (`ess-conformance/58`, top-level keys `events`, `provenance`, `scenarios`; 5 scenarios, 0 refusals) |
| `report-58.json` | `ess verify conform run --path wrong-state-decimal.spec.yaml --suite suite-58.json --target interpreted --report-format 2 --report-out report-58.json` (5 passed) |
| `suite-59.json` | the same `synthesize` with `--suite-format 5` (`ess-conformance/59`, adds `coverage`) |
| `report-59.json` | the same `run` over `suite-59.json` (5 passed, conformance `passed`) |

The seed pair was written from the root of an ESS source tree at tag `0.77.0`
(`git archive 0.77.0`), over ESS's own fixture `crates/edge/ess-cli/tests/fixtures/synthesis-seeds`
(`$M` below), and copied here:

| File | Command |
| --- | --- |
| `suite-42-seeds.json` | `ess verify conform synthesize --path $M/model --out suite-42-seeds.json --synthesis-seed $M/seeds/max.yaml at-max --synthesis-seed $M/seeds/below.yaml below-max` (`ess-conformance/42` with a `synthesis_seeds` provenance record; 8 scenarios, 0 refusals) |
| `report-42-seeds.json` | `ess verify conform run --path $M/model --suite suite-42-seeds.json --target interpreted --report-format 2 --report-out report-42-seeds.json` (8 passed) |

`ess 0.77.0` runs `suite-42-seeds.json` relabelled `ess-conformance/58` (8 passed), refuses it
relabelled `/40` as `UnsupportedVocabulary` at `$suite.provenance.synthesis_seeds`, and relabelled
`/60` as `UnsupportedSuiteVersion` ("execution readers admit suite majors 1–59").
