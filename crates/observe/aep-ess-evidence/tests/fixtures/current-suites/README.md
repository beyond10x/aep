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

## ESS 0.55.0

Written by `ess 0.55.0` from ESS's own specifications at ESS tag `0.55.0` (`1132a87a7`), run from
that checkout's root with `--out` naming this directory; no byte was edited afterwards. ESS 0.55.0
writes every fresh suite at `ess-conformance/34` or above (`ConformanceSuite::select_fresh_format`
sets `scenario_initial_state: empty`), so each coverage suite here is `/35` or later.

| File | Command |
| --- | --- |
| `suite-35-view-grants.json` | `ess verify conform synthesize --path docs/design/view-grants.example.yaml --suite-format 5 --out suite-35-view-grants.json` (writes `ess-conformance/35`: `<view>/grant/read/denied`, `<view>/grant/read/admitted/<actor>` and `<command>/grant/admitted/<actor>`) |
| `suite-35-gatepass.json` | `ess verify conform synthesize --path examples/gatepass --suite-format 5 --out suite-35-gatepass.json` (writes `ess-conformance/35` with three `<command>/grant/denied` scenarios; five in-scope `ESS-SYNTH-011` refusals, so ESS exits 1 after writing) |
| `suite-35-disclosure.json` | `ess verify conform synthesize --path crates/verify/ess-conformance/tests/fixtures/one-time-coverage/model.yaml --suite-format 5 --out suite-35-disclosure.json` (writes `ess-conformance/35` with four disclosure cells) |
| `suite-37-refusal-policy.json` | `ess verify conform synthesize --path crates/verify/ess-conformance/tests/fixtures/refusal-policy.yaml --suite-format 5 --out suite-37-refusal-policy.json` (writes `ess-conformance/37` with five `<binding>/binding/refusal/<outcome>` scenarios) |
| `suite-37-binding-condition.json` | `ess verify conform synthesize --path crates/verify/ess-conformance/tests/fixtures/binding-condition.yaml --suite-format 5 --out suite-37-binding-condition.json` (writes `ess-conformance/37` with the `condition-false` and `condition-absent` aspects; one in-scope `ESS-SYNTH-010` refusal, so ESS exits 1 after writing) |
| `suite-39-conditional-measures.json` | `ess verify conform synthesize --path crates/verify/ess-conformance/tests/fixtures/conditional-aggregate-measures.yaml --suite-format 5 --out suite-39-conditional-measures.json` (writes `ess-conformance/39`) |
| `expression-a2.spec.yaml` | the `MODEL` constant of ESS's `crates/verify/ess-conformance/tests/expression_a2.rs`, verbatim |
| `suite-41-expression.json` | `ess verify conform synthesize --path <this directory>/expression-a2.spec.yaml --suite-format 5 --out suite-41-expression.json` (writes `ess-conformance/41`) |
| `suite-43-seeds.json` | `ess verify conform synthesize --path crates/edge/ess-cli/tests/fixtures/synthesis-seeds/model --suite-format 5 --out suite-43-seeds.json --synthesis-seed crates/edge/ess-cli/tests/fixtures/synthesis-seeds/seeds/max.yaml at-max --synthesis-seed crates/edge/ess-cli/tests/fixtures/synthesis-seeds/seeds/below.yaml below-max` (writes `ess-conformance/43` with a `synthesis_seeds` provenance record) |
| `suite-45-event-multiplicity.json` | `ess verify conform synthesize --path crates/verify/ess-conformance/tests/fixtures/event-multiplicity.yaml --suite-format 5 --out suite-45-event-multiplicity.json` (writes `ess-conformance/45`) |
| `results-<name>.json` for these nine suites | an `ess-conformance-results/1` document passing every scenario key of `suite-<name>.json` (`jq -c '{format:"ess-conformance-results/1",completed_at:1790000000000,results:[.scenarios\|keys[]\|{scenario_id:.,status:"passed"}]}'`) |
| `report-<name>-external.json` for these nine suites | `ess verify conform report --suite suite-<name>.json --results results-<name>.json --implementation example-service --runner example-runner@1.0.0 --report-out report-<name>-external.json` |
| `ids-43-selected.json` | the keys `counter.model.Authorize/outcome/authorized` and `counter.model.Create/outcome/created` of `suite-43-seeds.json` (`jq`): one seeded scenario kept, the other moved outside by the filter |
| `input-43-selected.json` | `ess verify conform select --suite suite-43-seeds.json --ids ids-43-selected.json --out input-43-selected.json` (an explicit `/43` child with `suite-43-seeds.json` as its parent) |
| `results-43-selected.json` | as the results above, for the selected child's scenario keys |
| `report-43-selected-external.json` | `ess verify conform report --suite input-43-selected.json --results results-43-selected.json --implementation example-service --runner example-runner@1.0.0 --report-out report-43-selected-external.json` |
