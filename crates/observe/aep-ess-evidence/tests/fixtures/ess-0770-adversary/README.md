# Seed-bearing suites ESS 0.77.0 writes natively at /58 and /59

`../ess-0770/` admits a seed record at /58 and /59 only by relabelling the /42 and /43 seed suites.
These files are what `ess 0.77.0` writes itself when a seeded model also carries a `Decimal` in an
exact observer, so its synthesis raises the seed suite to /58 (ordinary) and /59 (coverage). No
byte of the JSON files was edited after `ess 0.77.0` wrote it, and `synthesize` wrote the same
bytes on a second run.

`model/` and `seeds/` are ESS's own fixture `crates/edge/ess-cli/tests/fixtures/synthesis-seeds`
at ESS tag `0.77.0`, changed in two ways: the `Counter` entity and the `Counters` view gain a
`price: Decimal` field (set to `1.5` by `Create`, `2.50` in both seed rows), and the entity gains a
terminal `Closed` state with a `Close` command whose `wrong-state` refusal makes ESS snapshot the
complete subject, which carries the `Decimal`.

Written from this directory:

| File | Command |
| --- | --- |
| `suite-58-seeds.json` | `ess verify conform synthesize --path model --out suite-58-seeds.json --synthesis-seed seeds/max.yaml at-max --synthesis-seed seeds/below.yaml below-max` (`ess-conformance/58` with a `synthesis_seeds` record of 2 applications; 13 scenarios, 0 refusals) |
| `suite-59-seeds.json` | the same with `--suite-format 5` (`ess-conformance/59`, adds `coverage`) |
| `report-58-seeds.json` | `ess verify conform run --path model --suite suite-58-seeds.json --target interpreted --report-format 2 --report-out report-58-seeds.json` (13 passed) |
| `report-59-seeds.json` | the same `run` over `suite-59-seeds.json` (13 passed) |
