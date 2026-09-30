# Current coverage suites

Written by `ess 0.48.0` from ESS's own test specifications at ESS `origin/main` `1bd946d6b3`; no
byte was edited afterwards.

| File | Command |
| --- | --- |
| `suite-27.json` | `ess verify conform synthesize --path crates/generate/ess-synth/tests/fixtures/declared-behaviour --suite-format 5 --out suite-27.json` (writes `ess-conformance/27`: command steps carry `caller`; one in-scope refusal, so ESS exits 1 after writing) |
| `results-27.json` | an `ess-conformance-results/1` document passing every scenario key of `suite-27.json` (`jq`) |
| `report-27-external.json` | `ess verify conform report --suite suite-27.json --results results-27.json --implementation example-ticket-service --runner example-runner@1.0.0 --report-out report-27-external.json` |
| `report-27-run.json` | `ess verify conform run --target interpreted --path <same specification> --suite suite-27.json --report-format 2 --report-out report-27-run.json --allow-incomplete` (ESS's own run) |
| `ids-27-selected.json` | the first three scenario keys of `suite-27.json` (`jq`) |
| `input-27-selected.json` | `ess verify conform select --suite suite-27.json --ids ids-27-selected.json --out input-27-selected.json` (an explicit `/27` child with `suite-27.json` as its parent) |
| `results-27-selected.json` | as `results-27.json`, for the selected child |
| `report-27-selected-external.json` | `ess verify conform report --suite input-27-selected.json --results results-27-selected.json --implementation example-ticket-service --runner example-runner@1.0.0 --report-out report-27-selected-external.json` |
| `suite-31.json` | `ess verify conform synthesize --path crates/verify/ess-conformance/tests/fixtures/delivery-context.yaml --suite-format 5 --out suite-31.json` (writes `ess-conformance/31`) |
| `suite-27-aggregate.json` | `ess verify conform synthesize --path crates/verify/ess-conformance/tests/fixtures/aggregate-optional-fields.yaml --suite-format 5 --out suite-27-aggregate.json` (`<view>/aggregate` scenarios) |
| `suite-17-aggregate.json` | the same for `crates/verify/ess-conformance/tests/fixtures/aggregate-views.yaml` (writes `ess-conformance/17`; in-scope refusals, so ESS exits 1 after writing) |
| `suite-27-retry.json` | the same for `crates/specify/ess-compiler/tests/fixtures/bounded-retry.yaml` (a `<binding>/binding/final-failure` scenario) |
| `results-<name>.json`, `report-<name>-external.json` for these three | as for `suite-27.json`, with `--implementation example-service` |
| `suite-7-accessor-refusal.json` | `ess verify conform synthesize --path crates/generate/ess-synth/tests/fixtures/bounded-accessor.yaml --suite-format 5` (writes `ess-conformance/7` with an `ESS-SYNTH-015` refusal) |
| `aggregate-refusals.spec.yaml` | `crates/verify/ess-conformance/tests/fixtures/aggregate-views.yaml` with its `views:` replaced by the `ByChannel` and `Either` views of ESS's `aggregate_views.rs` tests |
| `suite-17-aggregate-refusals.json` | `ess verify conform synthesize --path aggregate-refusals.spec.yaml --suite-format 5` (writes `ess-conformance/17` with `ESS-SYNTH-016` and `ESS-SYNTH-017` refusals) |
| `authored-037-send-failed.scenario.yaml` | the `SEND_FAILED` act of ESS's `authored_external_answer.rs` without its `outcome:` line |
| `suite-5-authored-refusal.json` | `ess verify conform synthesize --path examples/billing --scenarios <directory holding authored-037-send-failed.scenario.yaml as send-failed.yaml> --suite-format 5` (writes `ess-conformance/5` with an `ESS-AUTHOR-037` refusal) |
| `results-<name>.json`, `report-<name>-external.json` for these three suites | as for `suite-27.json`, with `--implementation example-service` |
| `results-31.json` | as `results-27.json`, for `suite-31.json` |
| `report-31-external.json` | `ess verify conform report --suite suite-31.json --results results-31.json --implementation example-service --runner example-runner@1.0.0 --report-out report-31-external.json` |
