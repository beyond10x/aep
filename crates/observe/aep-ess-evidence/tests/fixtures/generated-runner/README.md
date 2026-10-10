# Generated runner reports

A report/2 the Go test package ESS generates writes under `producer_profile: go-scenario-status/2`,
the profile ESS 0.56.0 and later give their generated Go and TypeScript runners. Each suite is the
exact one the run executed: its SHA-256 is the report's `suite.digest`.

Written by `ess 0.57.0` from this repository's own specification `ess/` as
story:evidence-admits-generated-runner-profile left it (19 scenarios, 68 refusals), run from the
repository root; no byte was edited afterwards. The suite `--target ir` writes is byte-identical to
the `essconform/suite.json` that `--target go` writes beside the runner.

| File | Command |
| --- | --- |
| `suite-34-aep.json` | `ess verify conform synthesize --path ess --target ir --out suite-34-aep.json` (ordinary `ess-conformance/34`, no `coverage` block) |
| `report-34-aep-go.json` | `ess verify conform synthesize --path ess --target go --out <dir>`, then `ESS_REPORT_FORMAT=2 ESS_REPORT_OUT=<file> go test ./...` in `<dir>` with the target below |
| `suite-35-aep.json` | the same `synthesize --target ir` with `--suite-format 5` (coverage `ess-conformance/35`) |
| `report-35-aep-go.json` | the same `go test` run against the package `synthesize --target go --suite-format 5` writes |

The target is one Go test beside the generated `essconform` package, calling
`essconform.Run(t, …)`. Its `ExecuteCommand` returns `essconform.ErrUnsupported` for every
`aep.review.*` command and an ordinary error for every other command; every other method returns
`essconform.ErrUnsupported`. So each run counts 8 error and 11 unsupported scenarios, two categories
`go-scenario-status/1` cannot carry, and its `execution_status` is `failed` (unsupported makes
execution fail). `go test` exits nonzero, as ESS documents for a run with unsupported or error
results; the report is written regardless.

The consumer report that prompted the story had this shape: format `ess-conformance-report/2`,
`producer_profile: go-scenario-status/2`, policy `complete-selection/1`, and a suite from
`ess verify conform synthesize --target ir` whose digest equals the report's `suite.digest`.
