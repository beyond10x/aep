# Ordinary suites

An ordinary suite is an even `ess-conformance/<N>` major from /6 on: no `coverage` block, and a
report/2 over it says `coverage: {knowledge: unknown}`. ESS 0.56.0 and later write one by default.

Written by `ess 0.57.0` from this repository's own specification `ess/` at commit `ecf4976da`
(before the `aep.evidence` domain), run from the repository root; no byte was edited afterwards.
`ess verify conform synthesize` wrote the same bytes on a second run.

| File | Command |
| --- | --- |
| `suite-34-aep.json` | `ess verify conform synthesize --path ess --target ir --out suite-34-aep.json` (writes `ess-conformance/34`, top-level keys `provenance` and `scenarios` only; 11 scenarios, 61 refusals) |
| `report-34-aep.json` | `ess verify conform run --path ess --suite suite-34-aep.json --target interpreted --report-format 2 --report-out report-34-aep.json` (5 passed, 6 unsupported, so `execution_status` and `conformance_status` are `failed`) |
