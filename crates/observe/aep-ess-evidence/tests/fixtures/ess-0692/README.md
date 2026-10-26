# Suites ESS 0.69.2 writes

`navigate-absent-reference.spec.yaml` is ESS's own fixture
`crates/verify/ess-conformance/tests/fixtures/navigate-absent-reference.yaml` at ESS tag `0.69.2`,
copied unchanged. Its two views read through a reference, so ESS writes the `<view>/reference/<field>`
form and the suite-level `events` catalog, which select the ordinary major /54 and the coverage
major /55. Every other file was written by `ess 0.69.2` from this directory; no byte was edited
afterwards, and `synthesize` wrote the same bytes on a second run.

| File | Command |
| --- | --- |
| `suite-54.json` | `ess verify conform synthesize --path navigate-absent-reference.spec.yaml --target ir --out suite-54.json` (`ess-conformance/54`, top-level keys `events`, `provenance`, `scenarios`; 8 scenarios, 0 refusals) |
| `report-54.json` | `ess verify conform run --path navigate-absent-reference.spec.yaml --suite suite-54.json --target interpreted --report-format 2 --report-out report-54.json` (8 passed) |
| `suite-55.json` | the same `synthesize` with `--suite-format 5` (`ess-conformance/55`, adds `coverage`) |
| `report-55.json` | the same `run` over `suite-55.json` (8 passed, conformance `passed`) |
| `input-55-selected.json` | `ess verify conform select --suite suite-55.json --ids ids-55-selected.json --out input-55-selected.json` (a /55 child of two scenarios; ESS drops the catalog entry the survivors no longer need and re-spells their `absent_events`) |
| `report-55-selected.json` | `ess verify conform run --path navigate-absent-reference.spec.yaml --suite-input input-55-selected.json --target interpreted --report-format 2 --report-out report-55-selected.json` (2 passed) |
