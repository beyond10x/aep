# ESS 0.55.0 suites the grant-id fixtures do not carry

Written by `ess 0.55.0`, run from this directory; no byte was edited afterwards.

| File | Source or command |
| --- | --- |
| `one-time-actors.spec.yaml` | ESS's `docs/design/one-time-response-values.example.yaml` at tag `0.55.0`, with the `Read` command, two actors, the `Record` entity and the `Records` view appended, as ESS's `one_time_generation.rs` test `every_declared_followup_and_actor_has_a_cell_or_a_named_refusal` composes them, actors renamed `Owner` and `Guest` |
| `desk-grants.spec.yaml` | ESS's `docs/design/view-grants.example.yaml` at tag `0.55.0`, with a third actor, `desk.tickets.Visitor`, granted only the `Titles` view |
| `desk-scenarios/*.yaml` | three authored acts: a visitor refused `OpenTicket` (accepted), a clerk expected refused a command it holds (`ESS-AUTHOR-038`), a refusal naming no actor (`ESS-AUTHOR-039`) |
| `suite-35-one-time-actors.json` | `ess verify conform synthesize --path one-time-actors.spec.yaml --suite-format 5 --out suite-35-one-time-actors.json` (twelve disclosure cells, every aspect, each sent as an actor) |
| `suite-35-desk-authored.json` | `ess verify conform synthesize --path desk-grants.spec.yaml --scenarios desk-scenarios --suite-format 5 --out suite-35-desk-authored.json` (exits 1 after writing: two in-scope authored refusals) |
| `suite-35-desk-component.json` | `ess verify conform synthesize --path desk-grants.spec.yaml --component desk-service --suite-format 5 --out suite-35-desk-component.json` |
| `results-35-<name>.json` | `jq -c '{format:"ess-conformance-results/1",completed_at:1790000000000,results:[.scenarios\|keys[]\|{scenario_id:.,status:"passed"}]}' suite-35-<name>.json` |
| `report-35-<name>-external.json` | `ess verify conform report --suite suite-35-<name>.json --results results-35-<name>.json --implementation example-service --runner example-runner@1.0.0 --report-out report-35-<name>-external.json` |
| `report-35-one-time-actors-run.json` | `ess verify conform run --target interpreted --path one-time-actors.spec.yaml --suite suite-35-one-time-actors.json --report-format 2 --report-out report-35-one-time-actors-run.json` |
| `report-35-desk-authored-run.json` | `ess verify conform run --target interpreted --path desk-grants.spec.yaml --suite suite-35-desk-authored.json --report-format 2 --report-out report-35-desk-authored-run.json` |
| `ids-35-one-time-actors-selected.json` | `jq -c '[.scenarios\|keys[]\|select(test("denied/\|read/credentials\|outcome/read"))]' suite-35-one-time-actors.json` |
| `input-35-one-time-actors-selected.json` | `ess verify conform select --suite suite-35-one-time-actors.json --ids ids-35-one-time-actors-selected.json --out input-35-one-time-actors-selected.json` |
| `results-35-one-time-actors-selected.json` | the results command above, over `jq -r .suite_json input-35-one-time-actors-selected.json` |
| `report-35-one-time-actors-selected-external.json` | `ess verify conform report --suite input-35-one-time-actors-selected.json --results results-35-one-time-actors-selected.json --implementation example-service --runner example-runner@1.0.0 --report-out report-35-one-time-actors-selected-external.json` |
