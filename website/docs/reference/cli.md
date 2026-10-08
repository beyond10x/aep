---
title: CLI reference
sidebar_position: 1
description: Every aep command, grouped under the four areas of its first level — govern, plan, drive and observe — plus doctor, with the flags each one accepts.
---

# CLI reference

`aep --help` lists five commands: four areas and `doctor`. Each table below lists every leaf command
in an area with the flags it accepts. `aep <command> --help` prints the full description of every
flag. This page is checked against the CLI in the repository's gate, so a command that exists and
is missing here fails the build.

| Area | For |
|---|---|
| `aep govern` | what the rule documents say, and what they decide for a task |
| `aep plan` | the work that exists, the store that holds it, and adoption |
| `aep drive` | walking a workflow run, and evaluating recorded runs |
| `aep observe` | what actually happened, checked and turned into evidence |
| `aep doctor` | whether this checkout is in a state the other commands will accept |

## Conventions

- **Output.** Most commands take `--format text|yaml|json`, and `text` is the default. The exceptions
  are named in their rows. Show `text` to people and give JSON to programs.
- **Exit codes.** `0` success, `1` refused or invalid. Report commands (`blocked`, `findings`,
  `review-value`, `waves`, `eval matrix`) exit `0` whenever they produced their report. `waves` exits
  `2` on a dependency cycle, and `observe trace check` exits `3` when the verdict is unknown.
- **Errors accumulate.** A command reports every problem it found, each with a stable code, not
  just the first.
- **Discovery.** Inside a project (a directory with `.engineering/project.yaml`, found upwards from
  the working directory), `--store`, `--root`, `--task` and `--artifacts` default to what the
  project names.
- **Older spellings.** Every flat spelling from before the areas existed still works and is hidden
  from `--help`: `aep artifact list` is `aep plan artifact list`, and `aep eval` is `aep drive eval`.

### Environment

| Variable | Effect |
|---|---|
| `AEP_ACTOR` | who a planning write is recorded as: `human:<name>`, `agent:<name>`, `service:<name>` or `system`. Unset means `human:$USER`, and an unparseable value is refused |
| `AEP_PROJECT_DIR` | renames the project directory (default `.engineering`) |
| `AEP_CACHE_DIR` | where pinned Git protocol sources are materialized (default `~/.cache/aep`) |
| `AEP_DRIVE_PLUGIN_DIR` | the plugin directory `drive` and `doctor` use when `--plugin-dir` is absent |

## Plan: artifacts

`aep plan artifact` reads and writes the planning store. Every verb takes `--store <dir>` (default
`<project>/.engineering/planning`), `--root <tree>` (default: the project's `protocols` source) and
`--format`. Writes change one file each: see [the planning store](../concepts/planning-store.md). A
`--store` directory outside a project is opened as a Git-native store with its evidence under
`<dir>/evidence`; one that still holds an `aep.project/1` `journal.jsonl` is refused.

### Write

| Command | Does |
|---|---|
| `aep plan artifact new <kind> <name> --title … [--summary …] [--owner …] [--tag …]… [--ref <provider:key>]… [--relate <relation:id>]… [--from <path\|->] [--findings <path\|->] [--prose-only <reason>] [--withholds <evidence-kind>]` | creates one artifact at the path its id determines; the body is `--from`, or the kind's template. `--findings` takes a `review-result`'s findings as a JSON array and writes them as a `findings` block. In a project whose `project.yaml` sets `findings_required_since`, a `review-result` with no block is refused unless `--prose-only <reason>` records why; the flag works in every store and is refused on another kind, beside a block, or with a blank reason. Refuses an existing id |
| `aep plan artifact move <id> --to <status> [--via] [--evidence <kind=count>]… [--at <instant>] [--executor <actor>] [--correlation <id>]` | moves it if the lifecycle permits and the rung's price is met; a refusal lists every legal status, or names the missing evidence. `--via` walks unguarded intermediate rungs, one transition each. `--evidence` asserts a count instead of reading records. `--at` pins the instant a `when:` rung is judged against. `--executor` records what ran the move when it is not the actor; `--correlation` records the activity it belongs to |
| `aep plan artifact body <id> --from <path\|-> [--append \| --section <heading>]` | replaces the body, appends to it, or replaces the prose under one `##` heading |
| `aep plan artifact set <id> [--title …] [--summary …] [--owner …] [--tag …]… [--untag …]… [--ref …]… [--unref …]… [--model-digest <hex>]` | changes front-matter fields; refuses `status`, `revision`, `id` and `kind` by name. `--model-digest` only on an executable system specification |
| `aep plan artifact relate <id> <relation> <target>`, or `relate <id> <relation>:<target>` | adds one edge; refuses a target the store does not hold |
| `aep plan artifact unrelate <id> <relation> <target>`, or `unrelate <id> <relation>:<target>` | removes exactly that edge; refuses an edge the artifact does not declare, listing the ones it does, and a `reviews` edge a recorded `review_outcome` relies on |
| `aep plan artifact scope <id> [--add <path>]… [--remove <path>]… [--inferred]` | records the paths a story lands on, each `cited` or (with `--inferred`) `inferred`; stories only |
| `aep plan artifact evidence <id> --kind <kind> --source <source> [--ref <reference>] [--at <instant>]` | records one observation about the artifact as an [evidence file](./evidence-file.md) |
| `aep plan artifact evidence <id> --kind review_outcome --review <review-result-id> --outcome no-op\|fixed\|escalated` | records what became of a review, on the reviewed artifact |
| `aep plan artifact evidence <id> --from <report> [--suite <suite.json> \| --suite-input <input.json>] [--ref …]` | reads kind, source and completion time out of an ESS conformance report (`ess-conformance-report/1`, or `/2` with its exact suite or suite input); refuses a report of no scenarios |

### Read

| Command | Shows |
|---|---|
| `aep plan artifact list [--kind …] [--status …] [--ref <provider:key>]` | one line per artifact; a blocked one carries `blocked: <type>`. JSON rows always carry `relations` and `blocked_by` |
| `aep plan artifact board [--kind …] [--format text\|yaml\|json\|markdown]` | status columns; `markdown` renders a page with each column's description |
| `aep plan artifact show <id> [--body-only]` | fields, scope, relations, findings and outcomes, then the body verbatim; `--body-only` prints the body bytes alone |
| `aep plan artifact blocked [--type <type>]` | what is stopped, grouped by the blocker |
| `aep plan artifact graph [--format dot\|mermaid\|json]` | the artifact graph; `dot` is the default |
| `aep plan artifact history <id>` | moves and evidence records, oldest first |
| `aep plan artifact explain <id>` | blockers in force, then per status reached: the move and the evidence admitted before it; ends with the price of each legal next rung |
| `aep plan artifact waves [--kind story] [--status …]` | which stories can be implemented at once, every collision, and every unassessed story. See [scope and waves](../concepts/waves.md) |
| `aep plan artifact findings <id> [--from <review-result-id>] [--to <review-result-id>]` | `carried`, `new` and `resolved` between two reviews of an artifact |
| `aep plan artifact review-value [--since <YYYY-MM-DD>]` | per reviewer: reviews, findings, outcomes and recorded cost; no score |
| `aep plan artifact validate [--strict] [--outcome-within <days>]` | every file, edge, status and transition list; exits `1` on a problem. Reports assertions, prose-only reviews and reviews with no outcome after `--outcome-within` days (default 14); `--strict` fails on those too. With `findings_required_since` set, a review with no `findings` block is a problem unless it was recorded `--prose-only`, a review carrying a block supersedes it, or the store recorded it (the commit that added it, in a Git-native store) before the date; the exempt ones are listed with their standing and `--strict` does not refuse them |
| `aep plan artifact kinds` | the kinds that can be created, marked planning or output |
| `aep plan artifact relations` | the relation vocabulary |
| `aep plan artifact lifecycle <kind>` | where a kind starts and what may follow what |

## Plan: store, browser, workspace

| Command | Does |
|---|---|
| `aep plan store migrate git [--engineering <dir>] [--dry-run \| --verify] [--protocols <source> --profile <profile> [--protocol adp/1]] [--planning-scope <name>]` | rewrites an `aep.project/1` store, or a planning directory with no `project.yaml`, as `aep.project/5`: the one verb that still reads a `/1` journal, which every other verb refuses. Refuses a dirty `.engineering` and any document that disagrees with its journal. `planning_scope` is `--planning-scope`, else derived from the `origin` remote, the primary checkout or the project directory, and the output names which. See [Migrate an older store](../guides/migrate-an-older-store.md) |
| `aep plan serve [--port 8899] [--read-only]` | the plan in a browser: board, artifact, next rungs with their price, and moves through the same decision `move` makes. Binds `127.0.0.1` only; the printed URL carries a per-run token. `--port 0` takes any free port |
| `aep plan workspace members [--fetch]` | the repositories `.engineering/workspace.yaml` names and whether each store is present; `--fetch` materializes pinned Git members |
| `aep plan workspace list [--kind …] [--status …] [--member …]` | the plan across every member |
| `aep plan workspace crossings [--strict]` | every edge that crosses a member boundary and whether it resolves; `--strict` exits `1` on an unresolved one |
| `aep plan workspace show <reference>` | where `kind:name` or `member/kind:name` points |

## Plan: adoption

`reverse` reads an existing repository. Every verb except `init` writes nothing.

| Command | Does |
|---|---|
| `aep plan reverse init --protocols <path-or-git-locator> --profile <profile> [--root .] [--protocol adp/1] [--summary …] [--no-verify] [--planning-scope <name>]` | writes an `aep.project/5` `project.yaml`, with `planning_scope` derived as `migrate git` derives it. Resolves the protocol source first unless `--no-verify`. Refuses a `.engineering/planning` that already holds a plan and names `aep plan store migrate git` |
| `aep plan reverse scan [root]` | what the repository says about itself: headings, toolchains, gates, test layout, as an `aep.reverse-scan/1` bundle |
| `aep plan reverse history [root] [--recent 500] [--top 15]` | what its Git history says: who touches what, dormant areas, where change concentrates |
| `aep plan reverse tickets --provider <name> [--repository .] [--top 100]` | tracker keys in history and in the plan's prose, joined to the references the store holds |
| `aep plan reverse openapi <path> --domain <name> [--out <file>]` | drafts an ESS domain from an OpenAPI document, with relations it can read from `$ref`s and `<x>_id` fields; standard output without `--out` |

## Plan: backends and entities

| Command | Does |
|---|---|
| `aep plan conformance [--level core\|audited\|full] [--suite <name>] [--inject <fault>] [--backend memory\|markdown\|sqlite\|postgres\|project] [--store …]` | holds a storage backend to the AEP contract suites; `--inject` breaks one property to show which suite catches it. The suites write, so a durable backend with no `--store` gets a scratch one |
| `aep plan entity list <--artifacts <manifest> \| --planning <dir>> [--type <entity-type>]` | seeds an in-memory backend and lists its entities |
| `aep plan entity get <--artifacts … \| --planning …> <reference>` | one entity |
| `aep plan entity history <--artifacts … \| --planning …> <reference>` | its revision records |
| `aep plan entity relations <--artifacts … \| --planning …> <reference> [--incoming]` | what it points at, or what points at it |
| `aep plan audit <--artifacts … \| --planning …> [--correlation …] [--entity …] [--rejected]` | the seeded backend's audit trail; `--rejected` shows refused attempts only |

The entity commands also take `--organisation` (default `local`) and `--space` (default
`manifest`). Nothing they seed is durable.

## Govern

These commands read the document tree and decide. Inside a project, `--root`, `--task` and
`--artifacts` come from `project.yaml`.

| Command | Does |
|---|---|
| `aep govern validate [--root .] [--artifacts <manifest>] [--evidence <out>]` | checks a document tree structurally and semantically, including that every rule could fire; `--evidence` also writes the result as a `verification` record |
| `aep govern resolve [--task …] [--artifacts …] [--evidence …]… [--state <snapshot>] [--advance]` | resolves a task into a plan: workflow, principles, capabilities, obligations |
| `aep govern evaluate [--task …] [--artifacts …] [--evidence …]… [--state …] [--advance]` | what is owed, what is permitted and what is missing; `--advance` takes every transition the evidence allows |
| `aep govern explain --action <capability> [--task …] [--artifacts …] [--evidence …]…` | one decision and the rule behind it; exits `1` when the answer is *denied* |
| `aep govern inspect [reference] [--root .]` | what a protocol, principle, workflow or profile declares, such as `aep/1` or `development.standard` |
| `aep govern describe <--artifacts … \| --planning …> <entity-type>` | what an entity type is, whether it may change, and what may target it |
| `aep govern schema [name]` | lists AEP's generated JSON Schemas, or prints one by name |
| `aep govern workflow render --id <workflow> [--root .] [--format svg\|html\|mermaid\|png\|tui] [--run <run-id> [--watch] \| --state <snapshot>] [--project …] [--out <file>]` | draws a workflow, optionally with a run or snapshot over it; `png` needs `rsvg-convert` and `--out` |
| `aep govern workflow instruct [--id <workflow>] [--map <map>] [--root .] [--out <path>]` | the workflow written out as instructions; without `--id`, every workflow into a directory |
| `aep govern workflow flow --id <workflow> [--map <map>] [--max-attempts 3] [--root .] [--out <file>]` | projects a workflow into the flow document a native agent loop walks; no guard travels, and the loop asks `aep drive transition` |

## Drive

`aep drive` walks **command and operator** steps of a step map, asking the engine before every
transition. A map with a model (`llm`) step is refused before a run is allocated, and the refusal
names `metaharness aep drive`, which runs model sessions over the same governor. `run`, `status`,
`resume` and `transition` take `--project`, `--root`, `--task`, `--store`, `--map` and
`--plugin-dir`, each discovered from the project when absent.

| Command | Does |
|---|---|
| `aep drive run [--map <file-or-id>] [--pause-on-approval] [--approver agent:<name>] [--max-iterations 25] [--take-lock] [--allow-evidence-gap] [--budget-usd <usd> --assume-usd-per-run <usd>]` | starts a run of the project's task, allocating an id such as `AUTH-142/3`; exits `0` when it completes or stops for an operator |
| `aep drive status [--run <id>]` | what the last run, or the named one, is doing, and who holds the lock |
| `aep drive resume <run> [--pause-on-approval] [--approver …] [--max-iterations 25] [--budget-usd <usd>] [--take-lock] [--retry-in-flight <attempt> \| --record-in-flight-no-verdict]` | continues a stopped run; a budget may only narrow. The two in-flight flags resolve an outside attempt whose result is uncertain |
| `aep drive transition [--run <id>]` | answers one `transition` hook document from a native loop on standard input: exit `0` proceeds, `2` refuses with a reason; writes nothing |
| `aep drive eval matrix <runs>… [--out <file>]` | assembles the outcome matrix from run manifests and reports: held, contradicted and unknown counts per cell; no score |
| `aep drive eval run --arm raw\|plugin\|driven\|native --harness claude\|codex\|b10x --out <dir> --observed-at <date> (--case <dir>… \| --workflow <id> [--corpus <dir>]) [--stream <file>] [--plugin-dir <dir>] [--plugin <repo@name@pin>]… [--model <model>] [--redact] [--cwd <dir>] [--budget-usd <usd>] [--assume-usd-per-run <usd>] [--instructions <dir>]` | with `--stream`, ingests a recorded run and spends nothing, exiting `0` conformant, `1` contradicted or `3` undecided. Without it, AEP refuses and names `metaharness aep drive eval run`. The `plugin` arm needs a plugin named explicitly |

A run writes its records under `.engineering/runs/<run>/`. Sessions for a model step are started by
Metaharness, which needs `METAHARNESS_LIVE=1`, `--budget-usd` and `--assume-usd-per-run`, and
reserves each session's assumed cost before spawning it. See
[Integrate an agent harness](../guides/integrate-a-harness.md).

## Observe

| Command | Does |
|---|---|
| `aep observe trace inspect --transcript <file>` | the transcript's census: event families, per-tool traffic, per-step timing |
| `aep observe trace check --spec <file> --transcript <file> [--redact] [--advisory <expectation-id>]…` | judges a run against a `trace-spec/1` document, citing event indices; exits `0` conformant, `1` contradicted, `3` unknown. `--format text\|json` |
| `aep observe trace evidence --spec <file> --transcript <file> [--advisory …]… [--observed-at <date>] [--out <file>]` | writes the verdict as a `trace_conformance` evidence record from the `trace-checker` verifier; exits `0` whatever the verdict |
| `aep observe trace redact --transcript <file> [--out <file>]` | removes the operator's home path, user name and Git identity from a recorded stream |
| `aep observe contract evidence --record <file> --observed-at <date> [--out <file>]` | turns a contract runner's `contract_result` record into an evidence document; refuses a record that checked nothing |
| `aep observe property evidence [--out <file>]` | runs the properties and writes a `property_test_result` document |
| `aep observe specification evidence [--store …] [--task <file>] [--snapshot <file>] [--artifact <id>] [--out <file>]` | decides the task's approved specification requirement by requirement, and writes a `specification` record |
| `aep observe evidence scan <paths>… [--at <date>] [--warn-days <n>] [--strict] [--fail-on-expired]` | reads Markdown for dated claims and reports `ok`, `expiring`, `expired` and `malformed`, with a coverage line; `--strict` fails on a coverage gap, `--fail-on-expired` on a stale claim |
| `aep observe evidence inspect <files>… [--at <date>] [--horizon <days>]` | the age of every record in an evidence document; exits `1` on a record observed in the future |

Evidence written by `observe` is the engine's format, read by `aep govern evaluate --evidence`. See
[Evidence](../concepts/evidence.md) and [Check a transcript](../guides/check-a-transcript.md).

## Doctor

| Command | Does |
|---|---|
| `aep doctor [--root .] [--plugin-dir <path>]… [--format text\|json]` | one `ok`, `warn` or `fail` line per check: binary version, project file, protocol source, planning store (by `validate`'s own rules), plugin directories, and the newest release tag reachable from `HEAD`. Exits `1` on any `fail`. Fixes nothing, reads no clock and opens no connection |

## Removed commands

These were removed in `0.62.0` together with event-log stores. The migration path is in
[Migrate an older store](../guides/migrate-an-older-store.md).

| Removed | Instead |
|---|---|
| `aep plan store inspect`, `migrate` (event-log forms), `verify`, `rebuild`, `writer-control`, `init-tree`, `export`, `install-hooks` | `aep plan store migrate git`; `aep plan artifact validate` |
| `aep plan artifact resolve`, `aep plan artifact render` | a Git merge; the artifact files are the store |
| `aep plan artifact validate --against <revision>` | `aep plan artifact validate` |
| `aep plan conformance --backend eventlog` | the other backends |

## Repository automation

For contributors to this repository only:

| Command | Does |
|---|---|
| `cargo xtask schema [--check]` | regenerates `schemas/generated/` from the Rust types |
| `cargo xtask status [--check]` | regenerates the release and gate stamps in `docs/status.md`, `AGENTS.md`, the status page and the landing page |
| `cargo xtask fmt [--check]` | formats the workspace members |
| `cargo xtask release` | reports whether the newest release was cut completely; reaches the network |
