---
title: CLI reference
sidebar_position: 1
description: Every subcommand of the reference CLI, grouped by surface — protocol, planning, adoption, driver, evidence, entity, ESS, infrastructure, trace, contract and evaluation — with exit codes.
---

# CLI reference

The reference CLI is `protocol`, built with `cargo build --release -p protocol-cli` and left at
`target/release/protocol`. `--help` on any subcommand carries the full flag list — this page is the
map.

Most verbs take `--format text|yaml|json`, with `text` the default: refusals, decisions and
evaluations all serialise. The exceptions are named in their own sections, and they are exceptions
because the thing being rendered is not a report — a graph has `dot` and `mermaid`, a drawing has
`svg` and `png`, and the three verbs that mint evidence default to `yaml` because that is what
`protocol evaluate --evidence` reads back.

General conventions: exit `0` is success, exit `1` is a refusal or invalid input, and errors
accumulate — a run reports every problem it found, not the first. Two exceptions are deliberate and
stated where they apply. A verb that *reports* rather than gates exits `0` whatever it found: that
covers `protocol evaluate` on a blocked execution and the four `infra` report verbs. And the two
verbs that judge an implementation or a run use a third code for *unverified* — `ess conform run`
and `trace check`.

## Protocol surface

| Command | Does |
|---|---|
| `protocol validate [--root .] [--artifacts m.yaml]` | checks a document tree structurally and semantically — including that every rule could actually fire |
| `protocol resolve --task task.yaml [--root .]` | resolves a task into an execution plan: workflow, principles in force, capabilities, obligations |
| `protocol inspect [reference]` | shows what a protocol, principle, workflow or profile declares — `aep/1`, `test-driven`, `development.standard` |
| `protocol evaluate --task … [--artifacts …] [--evidence e.yaml]… [--advance]` | evaluates an execution: what is owed, what is permitted, what is missing; `--advance` also attempts transitions |
| `protocol explain --task … --action production.write` | explains one decision — allowed or denied, by which rule, and what would unlock it |
| `protocol schema [name]` | prints engineering-protocols' generated JSON Schemas, or one by file stem |
| `protocol schema validate <paths>… [--schemas dir] [--format text\|yaml\|json]` | discovers the project registry from `project.yaml`, selects each JSON instance's contract by `schema` → `$id`, and validates offline; `--schemas` is the fixture/non-project override |
| `protocol schema typescript <schema-id> --root Name [--out file] [--check] [--schemas dir]` | deterministically projects structural TypeScript from one registry schema selected by `$id`; `--check` detects drift without writing |
| `protocol conformance [--level core\|audited\|full] [--suite name] [--inject fault]` | checks a storage backend against the AEP contract suites (16 suites at `full`, 14 at `audited`, 7 at `core`); `--inject` breaks one property on purpose to show the responsible suite fails |

Inside a project — a directory holding `.engineering/` — `resolve`, `evaluate` and `explain` take
their `--root`, `--task` and `--artifacts` from `.engineering/project.yaml`, so the three long paths
collapse to the verb. `explain --action` exits `1` when the answer is *denied*, which is what lets a
harness ask before it acts.

## Planning surface

`protocol artifact` reads and writes the markdown planning store: one artifact per file, YAML
frontmatter, free markdown body, under `<project>/.engineering/planning/` unless `--store` says
otherwise. The consequence for a person is the reason it is markdown and not a database: the diff of
a status move is one line, and `git log` already knows who made it.

Every verb here takes `--store` and `--root` (the document tree the lifecycles and templates come
from, default `.`).

| Command | Does |
|---|---|
| `protocol artifact new <kind> <name> --title … [--summary …] [--owner …] [--tag …] [--relate rel:id]` | writes one file, at the path the id determines; refuses to overwrite an existing one |
| `protocol artifact move <id> --to <status>` | moves it if the kind's lifecycle permits, and on a refusal names every status it could have moved to instead — or, when the rung is on the ladder but its evidence has not been recorded, says **which kind** is missing and how many. See [Lifecycles, decided as data](../concepts/lifecycles.md) |
| `protocol artifact relate <id> <relation> <target>` | adds one edge |
| `protocol artifact body <id> --from <path\|->` | replaces the complete markdown body while preserving CLI-owned frontmatter; changed bytes bump one revision, identical bytes do nothing |
| `protocol artifact show <id> [--format text\|yaml\|json]` | one artifact, printed: id, kind, status, title, summary, owner, tags, relations and revision, then the markdown body **verbatim**. The verb for an id in hand — `list` prints the whole plan, `explain` answers what made a status happen and `body` writes. An id the plan does not hold is refused, naming it |
| `protocol artifact list [--kind …] [--status …]` | the plan, one line per artifact |
| `protocol artifact board [--kind …]` | the same plan as status columns |
| `protocol artifact graph [--format dot\|json]` | the plan's graph — `dot` for `dot -Tsvg`, `json` for a consumer that would otherwise parse a diagram |
| `protocol artifact history <id> [--format text\|yaml\|json]` | what happened to one artifact, oldest first, out of the store's append-only journal: creations, moves, and every evidence record. A corrupt journal line is skipped **and counted**, never silently dropped |
| `protocol artifact explain <id> [--format text\|yaml\|json]` | what made this what it is: per status it reached, the move and every evidence record admitted since the previous one — each named against the revision the artifact was at when it was admitted, so a later edit cannot re-date an old record onto the new text. A status reached on nobody's record is marked rather than left blank. Not `protocol explain`, which is how a policy decided |
| `protocol artifact evidence <id> --kind <k> --source <s> [--ref <url>] [--at <iso8601>]` | records an observation about an artifact, so a later move can be decided on it. `--at` defaults to now, read at the edge |
| `protocol artifact validate` | every file, every edge, every status, accumulated into one list: a file where its id does not put it, an edge pointing at nothing, a cycle, a duplicate id, a status the lifecycle does not have. Over a markdown plan it reads the event log too: a document edited outside a command is **drift**, and a `revision:` higher than any event for it records is a **forged revision** — a number no write produced, reported and never refused |
| `protocol artifact kinds` | the 26 artifact kinds, marking which are planning rather than output |
| `protocol artifact relations` | the 13 relations, with what each edge means |
| `protocol artifact lifecycle <kind>` | where a kind starts, and what may follow what |

`new`, `move`, `relate` and `body` write without an `--out`, unlike `ess generate` and `ess synthesize`.
The difference is that they write exactly one file, at a path the id determines, inside a directory
somebody opted into — and an item you did not want is removed with `rm`.

**Every write is journalled with an actor, and the caller says who.** `AEP_ACTOR` declares it —
`human:<name>`, `agent:<name>`, `service:<name>` or `system` — and a value that does not parse is
refused, naming the variable, rather than quietly replaced by yours; unset, the write is
`human:$USER` as before. `protocol drive` sets it to `agent:<execution id>` on every process it
starts for a step, so a `command` step's `protocol artifact move` reads back as the run's own act
in `protocol artifact history` rather than as the operator's. Nothing here *verifies* an
identity — it is a declaration, exactly as strong as the rest of the provenance model.

## Specification surface

One verb, and it answers one question: **is the specification this task is being held to satisfied
by what this run observed?** It reads the planning store and a run's snapshot, decides every
requirement, and writes the `specification` evidence record `protocol evaluate --evidence` accepts —
the record `spec-driven` reads as `specification.satisfied` before a task may complete.

| Command | Does |
|---|---|
| `protocol specification evidence [--store .engineering/planning] [--task <file>] [--snapshot <file>] [--artifact <id>] [--out <file>] [--format text\|yaml\|json]` | decides the specification of this task's work, requirement by requirement, and writes the record naming what is unmet |

**A requirement is a list item under a `Requirements` or `Acceptance` heading, and it is satisfied
when the predicate it names in backticks is observed `True`.** Nothing in a markdown artifact marks
a requirement, so the verb defines one, and the definition has to be one you can satisfy on purpose
and cannot satisfy by accident: a requirement naming no predicate is reported **unmet**, and a
ticked checkbox is deliberately not the rule — the party that writes the specification is the party
being checked. `False` and `Unknown` both fail to satisfy and are reported apart, because nobody
looked is not the same finding as it is broken. Without `--snapshot` every requirement reads
`Unknown`, which is a legitimate question of its own: *is this written so that anything could ever
decide it?*

**Which specification is the guard's own question.** Omit `--artifact` and the verb selects an
approved `specification` whose `specifies` edge lands on the work the task declares — the rule
`spec-driven.before_implementation` states, evaluated by the engine's own function, so the verb
cannot decide a document the guard it serves would refuse. The task is `--task <file>`, or the one
`project.yaml` names when the flag is absent; with neither in reach the selection is unbound and
falls back to the store's one in-force specification. A driven step writes `--task {task}`, which
the driver expands to the document *that run* was started from.

`--artifact` names *which* specification, never *whether* the binding applies: an id that does not
specify this task's work is refused. It does lift the status half, so a `draft` can be asked whether
it states anything a fact could decide.

**A refusal names both ends and which document it read**, because the wrong task is the failure a
reader cannot otherwise see:

```text
2 specifications in .engineering/planning are this task's — specification:billing,
specification:billing-v2 — so a step here would establish something about one of several
documents. this task's work is story:billing, task:BILLING-1 (from
.engineering/task-billing.yaml). `--artifact` names one exactly; it does not lift the binding
```

It exits `0` whatever the verdict — an unsatisfied specification is exactly what the record is for —
and writes nothing at all when it cannot tell which specification the run is about. A driver reads
that as *nothing observed*, and the run stops at the guard rather than moving on a record about
somebody else's story.

## Adoption surface

`protocol reverse` points the tooling at a repository that already exists and was not written with
any of this in mind. Three of its four verbs **write nothing** — the consequence for a person
evaluating the tool is that you can run it against your own repository before deciding anything,
and the worst case is a report you disagree with.

| Command | Does |
|---|---|
| `protocol reverse scan [root] [--format text\|yaml\|json]` | reads a repository and reports what it already says about itself — headings, declared toolchains, gates, test layout — as an `aep.reverse-scan/1` bundle. **Writes nothing** |
| `protocol reverse history [root] [--recent 500] [--top 15] [--format …]` | reads what the repository's own git history says: who touches what, which areas are dormant, where change concentrates. **Writes nothing** |
| `protocol reverse openapi <path> --domain <name> [--out …]` | drafts an `ess/1` domain from an OpenAPI document that already exists; standard output when `--out` is absent |
| `protocol reverse init --protocols <path-or-git-locator> --profile <profile> [--root .] [--protocol adp/1] [--summary …] [--no-verify]` | writes the `project.yaml` that makes a repository an adopting project. This is the one that writes, and it resolves the protocol source first unless `--no-verify` says not to |

`--protocols` takes a path or a pinned `git+…#<40-hex>` locator: a governing document tree that
could move under you is a gate whose meaning changes without a commit in your repository.

See [Adopting a repository that already exists](https://github.com/beyond10x/engineering-protocols/blob/main/docs/guide/adopting.md)
for the walkthrough.

## Driver surface

`protocol drive` walks a workflow: it makes the engine's calls in order, runs the three kinds of
step that touch the world — a program, a model, a person — and records what it did. It evaluates no
gate itself, because a driver that could evaluate a gate would be a second protocol implementation
with none of the conformance suites behind it.

| Command | Does |
|---|---|
| `protocol drive run [--map <file-or-id>] [--pause-on-approval] [--approver agent:<name>] [--max-iterations 25] [--take-lock] [--allow-evidence-gap]` | starts a new run of a task, allocating a run id such as `AUTH-142/3` |
| `protocol drive status [--run <id>]` | what the store's last run is doing, and who holds the lock |
| `protocol drive resume <run> [--pause-on-approval] [--approver agent:<name>] [--max-iterations 25] [--take-lock]` | continues a run that stopped, re-taking the store lock |

All three discover `--project`, `--root`, `--task` and `--store` from the project when omitted, and
take `--plugin-dir` (repeatable; `AEP_DRIVE_PLUGIN_DIR` supplies it when the flag is absent) to load
a harness plugin into every `llm` step's session. `--pause-on-approval` runs until the first thing a
person owes, then persists and exits `0`; the resume walks on from the step after it. What answered
the `operator` step is read on that resume from the run's own record: a granted `approval` a
person recorded while the run was stopped always counts, and `--approver agent:<name>` admits one
named agent's recorded approval as well — never the run's own actor, which is refused by name. The
cursor then says who answered (`protocol drive status`: `answered …`). With an approver named, a
resume that finds no admissible approval stops again and says who would be admissible; with none
named, a resume that finds nothing walks on as it always did and the report says the record holds
nobody's answer. `run` and `resume` exit `0` when the run completes or stops awaiting an operator,
and `1` otherwise.

What a run writes beside its cursor, in `.engineering/runs/<run>/`: `launch.json`, how the run was
started — which is what makes the printed `resume with: protocol drive resume <run>` line a line
that works, since `resume` fills in `--map`, `--task`, `--pause-on-approval` and `--plugin-dir` from
it and a flag typed on the resume still wins; `commands.jsonl`, one line per `command` step attempt
naming the program the map wrote, the program that was spawned and which of the two it was; and a
`step-context.json` per `llm` step. `--max-iterations` bounds the call, not the run's lifetime, so a
resume gets the budget the operator typed.

Four refusals and fallbacks to know before the first paid run. A `command` step whose program is
`protocol` runs the driver's own binary, whatever is first on `PATH`; where the driver cannot name
its own binary and the `protocol` on `PATH` is another version, `run` and `resume` refuse before
allocating a run id, naming both versions. A run whose `llm` sessions could not reach the `protocol`
CLI is refused before anything is spent — the child environment is constructed, and its `PATH` is
`$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin`. With no `--plugin-dir` and no
`AEP_DRIVE_PLUGIN_DIR`, `<project>/integrations/claude-code` is loaded when it exists. And every
`llm` step is told which task the run drives, before the map's own words.

An `llm` step names its harness. `harness: claude-code` — the default when a step is silent — is
launched through `metaharness run claude`; `harness: b10x` is launched through `metaharness run
b10x`, the beyond10x loop, given the state's program allow-list, the driver's own policy as the
loop's `--hooks`, the `protocol` binary as its `--driver`, and the same `--plugin-dir` the vendor
arm is given. The shipped `development/default` map drives all six of its `llm` steps on `b10x`;
`development/checks` says nothing and so drives Claude Code.

| Option | Does |
|---|---|
| `--b10x-endpoint <url>`, `--b10x-model <model>`, `--b10x-wire openai-responses\|anthropic-messages` | where a `harness: b10x` step's loop is pointed and which model API it speaks there; the loop picks no model of its own |
| `--b10x-api-key` | send `OPENAI_API_KEY` to that endpoint; off by default, because a gateway that authenticates nobody is the case a driven run starts in |
| `--b10x-oauth-token-file <file> [--b10x-oauth-token-pointer <ptr>]` | a subscription token for the b10x arm instead of an API key; the path travels into an argv, and the token enters neither this process nor metaharness |
| `--b10x-cgroup-root <dir>` | a delegated cgroup subtree, so a confined `b10x` step may execute its suite; turns on `--substrate-embedded` with it |
| `--claude-endpoint <url>`, `--claude-model <model>` | point a `harness: claude-code` step at the same gateway, so a comparison of the two arms differs by harness and not by model; metaharness is passed `--credentials none` with it |
| `--allow-evidence-gap` | start even though the map cannot produce an evidence kind the plan will demand — an economic pre-flight, not a protocol rule |

`protocol workflow render` draws the same thing for a reader: the states down the page, the guards
beside the arrows, and — with `--run` or `--state` — where a run is, where it has been, what it
produced and why it stopped. It evaluates nothing; every overlay was decided by the engine and read
out of a run directory.

| Command | Does |
|---|---|
| `protocol workflow render --id adp/default [--root .] [--format svg\|html\|png\|tui] [--out f]` | the workflow, as a standalone SVG, a self-contained HTML page, a raster image by way of `rsvg-convert`, or one terminal frame |
| `protocol workflow render --id … --run AUTH-142/3 [--project …] [--watch]` | the same figure with a driver run drawn over it; `--watch` redraws as the run advances, and is `--format tui` with `--run` only |
| `protocol workflow render --id … --state snapshot.yaml` | an engine snapshot drawn over it instead |

Without `--out`, everything but `png` goes to standard output.

Two more verbs read the same documents. `protocol workflow instruct` writes a workflow out as
instructions in words, for a reader with no canvas: the states as things you may not enter yet, the
guards as what opens each move, and the principles that time obligations against the phases those
states declare, joined to the states each lands on. `protocol workflow flow` projects a workflow into
the document the b10x harness walks natively (`b10x-harness workflow run --flow`). It is an honest
projection and not an equivalence: that notation is a DAG of sub-trees and this graph goes backwards,
so a retreat becomes a group that repeats, terminal states are dropped because nothing runs in them,
and **no guard travels at all** — the governor stays a program the loop asks at every section
boundary, not a field in the document. What it answers for free, before anything is paid to run, is
whether the shape fits.

| Command | Does |
|---|---|
| `protocol workflow instruct [--id adp/default] [--root .] [--out f]` | the workflow as instructions; without `--id`, every workflow the tree declares, into a directory |
| `protocol workflow flow --id adp/default [--root .] [--map <file-or-id>] [--max-attempts 3] [--out f]` | the projection; with `--map`, each node carries what a harness does in that state — an `llm` step as its prompt, context, write scope and harness, a `command` step as its argv and the evidence it establishes, an `operator` step as what it asks for — and a state with several steps becomes a group chained in the map's order |

Without `--map` the nodes carry the state's summary and nothing a harness could run, which is enough
to answer whether the shape fits and not enough to run. The header names the map and the pin it was
written against; a map pinned to another version of the workflow is refused before anything is
written, in the words `drive run` refuses it in. `--max-attempts` is a number because the notation
wants one: the workflow bounds a retreat with the engine's iteration budget, which is not in the
document.

## Evidence surface

The observation half of evidence horizons. Neither verb writes anything, neither resolves a plan and
neither decides a gate: they report what a document says about when somebody last looked.

| Command | Does |
|---|---|
| `protocol evidence scan <paths>… [--at 2026-09-01] [--warn-days n] [--strict] [--fail-on-expired]` | reads human-written markdown for dated claims and reports coverage beside the classification; a directory is read one level deep for `*.md` |
| `protocol evidence inspect <files>… [--at …] [--horizon 7d]` | reads the evidence document `protocol evaluate --evidence` submits and reports, per record, when somebody last looked |

`scan` classifies each record `ok`, `expiring`, `expired` or `malformed`, and closes with a coverage
line — occurrences found, records parsed, and how many it could not read:

```text
43 occurrence(s), 43 record(s), 0 unparsed — 27 ok, 6 expiring, 10 expired, 8 malformed (at 2026-09-01)
```

That line is the point. A scanner over human-written documents needs a coverage claim of its own,
because an annotation that is present, correct, legible to a human and invisible to the gate is the
one failure a clean report cannot show.

The two exit flags on `scan` answer different questions and are separate for that reason. `--strict`
fails when the parser found fewer records than there are annotation-shaped occurrences — *is the
gate blind?* `--fail-on-expired` fails when a record is past its horizon — *is the claim stale?* An
expired record is a normal finding; a corpus with none is a corpus nobody has kept.

`inspect`'s `--horizon` is report-only: a what-if applied to a printed table. It reaches no
requirement and no evaluation, and nothing it prints can extend the life of a record. The horizon
that decides a gate is declared on a requirement, in a reviewed document. `inspect` exits `1` on a
record whose observation time is in the future — the same refusal the engine applies, available
before anything is submitted.

## Entity surface

These seed an **in-memory** backend from `--artifacts` (an artifact manifest) or `--planning` (a
markdown planning store) and then answer; one of the two is required. Nothing is durable, and what
`history` shows is this run's seeding — every entity is at revision 1.

| Command | Answers |
|---|---|
| `protocol entity list <--artifacts m.yaml\|--planning dir> [--type aep.design/v1]` | every entity the source seeds, with type, locator, revision |
| `protocol entity get <source> <locator-or-id>` | one entity; exit 1 when nothing matches |
| `protocol entity history <source> <ref>` | revision records, oldest first |
| `protocol entity relations <source> <ref> [--incoming]` | what an entity points at, or what points at it |
| `protocol audit <source> [--correlation …] [--entity …] [--rejected]` | the audit trail, oldest first; `--rejected` shows only refused attempts |
| `protocol describe <source> <entity-type>` | what a type *is*: mutable or not, which commands may target it, which relations it may have |

`--organisation` (default `local`) and `--space` (default `manifest`) set the namespace the seeded
locators live under.

## ESS surface

All take `--path <file-or-dir>` (default `.`) unless noted.

| Command | Does |
|---|---|
| `protocol ess validate` | parses and checks a specification, naming every problem in one run |
| `protocol ess compile` | resolves every reference into the normalized IR |
| `protocol ess inspect <name> [--kind domain\|type\|command\|event\|error\|binding\|component]` | one declaration, resolved |
| `protocol ess graph [--format dot\|mermaid\|json\|yaml]` | the actor/command/event graph |
| `protocol ess generate --kind docs\|schema\|openapi\|asyncapi [--out dir]` | the projections; without `--out`, a listing only |
| `protocol ess synthesize [--target rust\|go\|web] [--out dir]` | the synthesis plan and one emitted tree |
| `protocol ess conform synthesize [--out dir]` | the conformance suite the specification obliges; `--format json` carries the suite document itself |
| `protocol ess conform run --target <name> [--suite suite.json] [--inject fault] [--untraced]` | runs the suite against a compiled-in reference implementation |
| `protocol ess conform evidence --target <name> [--observed-at date] [--out f]` | runs the suite and mints the AEP evidence record in the same process |
| `protocol ess diff --from <path> --to <path> [--format text\|json]` | the semantic delta between two revisions of one specification |
| `protocol ess impact --from <path> --to <path> [--suite suite.json] [--generated dir]` | what the delta invalidates: scenarios owed again, artifacts owed regeneration, each with its dependency path |

`--target` names a reference implementation this binary was compiled with — `billing` or
`oracle-fixture`. It cannot reach yours: a conformance target is a Rust trait, and nothing here
speaks to an implementation over a socket. To hold your own system to a specification, depend on
`ess-conformance`, implement the trait, and run the committed `suites/generated/<system>/suite.json`
against it — the same document this verb writes.

`ess conform run` exit codes differ from the general convention, because "wrong" and "unverified"
are different findings:

| Exit | Meaning |
|---|---|
| `0` | every scenario passed |
| `1` | the implementation contradicted the specification, or a scenario the specification requires is one the target cannot expose |
| `3` | nothing contradicted it, and at least one scenario could not be executed |

`ess conform evidence` exits `0` whenever a record was produced, **including for a failing run** —
the verdict is in the record, and the engine is what decides on it. Its `--observed-at` exists so a
committed record can be regenerated byte for byte; it defaults to now, which is the truth.

## Infrastructure surface

Inputs are files written by an external scanner; no verb reaches a cluster.

| Command | Does |
|---|---|
| `protocol infra validate --path <bundle>` | checks an `infra-observation/1` bundle |
| `protocol infra compile --path <bundle> [--out f]` | compiles it to the content-addressed `infra-ir/1` document |
| `protocol infra inspect --path <ir> [--properties]` | per-object and per-workload facts |
| `protocol infra graph --path <ir> [--namespace n] [--format mermaid\|json\|html]` | the typed dependency graph, with the evidence on every edge |
| `protocol infra diagnose --path <ir> [--min-severity info\|warning\|error] [--candidates] [--directions]` | twenty coded findings (`INFRA-DIAG-001`…`020`), invariant candidates, ranked directions — a report, never a gate |
| `protocol infra view --path <ir> [--namespace n] [--out f]` | writes the self-contained HTML component page and opens it in a browser; the one verb here that spawns another program |
| `protocol infra simulate --spec expected.yaml --path <bundle\|ir>` | evaluates a desired state against a snapshot: `ok` / `gap` / `unk` per expectation |
| `protocol infra diff --from <ir> --to <ir>` | what moved between two scans of one cluster, over declared state |
| `protocol infra project --spec expected.yaml --path <bundle\|ir> --out <dir>` | writes the patch tree that would close the gaps, plus `OBLIGATIONS.md` and `SUMMARY.md`; applies nothing |

`diagnose`, `simulate`, `project` and `diff` exit `0` whatever they found, and take
`--format text|json` only. A cluster with sixteen decisions owed has been successfully diagnosed,
simulated and projected; drift is a report too. Exit `1` here means an input that could not be read
— or, for `diff`, the one refusal: two snapshots of different clusters. `view` takes no `--format`
at all: it has one output, and its purpose is to open it.

## Trace surface

Inputs are transcripts a harness already wrote; no verb runs an agent, calls a model or reaches a
network. All three take `--format text|json`, except `trace evidence`, which writes the record and
so takes the shared `text|yaml|json` with `yaml` the default.

| Command | Does |
|---|---|
| `protocol trace inspect --transcript <file>` | the transcript's census from the typed event IR: event families, per-tool traffic in both directions, per-step `gen`/`exec` timing |
| `protocol trace check --spec <file> --transcript <file> [--redact] [--advisory <id>]` | judges the run against a `trace-spec/1` document: `ok` / `gap` / `unk` per expectation, every verdict citing event indices — exit 0 conformant, 1 contradicted, 3 unknown |
| `protocol trace evidence --spec <file> --transcript <file> [--advisory <id>] [--observed-at date] [--out <file>]` | mints the verdict as a `trace_conformance` evidence record (producer `trace-checker`, digest pair binding it to one transcript and one spec) that `protocol evaluate --evidence` accepts |

`--redact` cites event indices and digests only — no command strings, no file paths, no text. It is
opt-in, and the un-redacted rendering carries a footer naming what it contains, so pasting a report
somewhere public is a decision rather than an accident.

`--advisory <id>` downgrades one named expectation for this run: still evaluated, still printed,
gating nothing, and every downgraded id named in the report. An id the specification does not
declare is a usage error, not a silent no-op. In an evidence record, `trace_conformance.passed`
ignores the downgrade, because a flag the caller passed must not satisfy a requirement the protocol
asked for.

## Contract surface

The consumer/provider contract — *does the published interface still behave as its consumers were
told?* — and specifically a record an outside contract runner printed about one. Not
`protocol conformance`, which asks whether a storage backend implements `aep-contract`; the two
share the word and nothing else.

| Command | Does |
|---|---|
| `protocol contract evidence --record <file> --observed-at <date> [--out <file>]` | reads a `contract_result` record a contract runner emitted and writes the AEP evidence document it implies (producer `contract-runner`, the record's bytes digested into the provenance) that `protocol evaluate --evidence` accepts |

The record is one JSON object in the shape `aep-domain` defines —
`{kind, checked, failed, breaking_changes, provider, consumer}` — which is what
`metaharness conformance <kind> --contract` prints. Redirect it to a file and hand the file over;
`--record` takes a path rather than standard input so that the bytes the provenance digest names
exist somewhere a later reader can go and check.

`--observed-at` is required, unlike `protocol trace evidence`'s. That verb runs its check in its own
process and may stamp its own clock; this one is handed a record made elsewhere, possibly last week,
and the record carries no time of its own — so a default of *now* would claim a freshness nobody
observed.

Two records are refused, each naming why, and both refusals are about a record that says nothing
rather than a record that says something bad:

* `checked: 0` — a run that checked nothing also has zero failures. Minting it would discharge the
  `contract_result` obligation the `contract-testing` principle places on a task while two of that
  principle's three predicates passed vacuously.
* `breaking_changes` greater than `failed` — a breaking change is one of the failures, so the pair
  describes no run.

A record reporting failures is written down and exits `0`. The verdict belongs in the record, and
`protocol evaluate` is what decides on it.

## Evaluation surface

How well a harness follows these workflows, under four treatments — `raw` instructions, the shipped
`plugin`, a `driven` run whose every tool call is answered at a seam, and a `native` run whose
published toolset *is* the policy. `metaharness` is a tool here, the way `git` is: found on `PATH`,
and a machine without it is told so by name and exits `2` rather than reddening a gate.

| Command | Does |
|---|---|
| `protocol eval matrix <runs>… [--format text\|json] [--out <file>]` | assembles the outcome matrix from `*.manifest.yaml` / `*.report.json` pairs: per harness × arm × workflow and per expectation, how many facts held, how many were contradicted, and how many nobody could find out |
| `protocol eval run --arm raw\|plugin\|driven\|native --harness … --case … --out <dir> --observed-at <date> [--stream <file>] [--budget-usd <usd>]` | runs one arm of one case and leaves the documents `eval matrix` reads; `--stream` ingests a run somebody already recorded and spends nothing |

`eval matrix` exits `0` whenever a matrix was assembled, whatever it says: a matrix is a report, and
an exit code that moved with the counts would be the single number it refuses to compute — there is
no score, no ranking and no percentage in the output. Nothing spawns without `METAHARNESS_LIVE=1`
and `--budget-usd`. Arms `driven` and `native` are not launched from here and the refusal says what
launches each: `protocol drive run` and `b10x-harness`.

### Reading a native cell

The arm word is the enforcement model, and there is no column for it. So a clean store-integrity
row — no `store_broken`, `census.denied = 0` — does not mean the same thing in every row:

| arm | what a clean row means |
|---|---|
| `raw` | **compliance.** Nothing on that arm was in a position to refuse |
| `plugin` | compliance, except where the vendor hook saw the call — a refusal there is the hook's |
| `driven` | **enforced.** The call was put to the driver and answered before it ran, and the refusal is in the run's own record |
| `native` | **compliance, or not observable — never enforced**, unless the run carried a `scope:` or a loop hook that could refuse |

`denied: 0` is *nobody asked me*, not *nothing was refused*; the driver already prints those as two
different sentences, and only one of them is about the run. Why this is a reading rule and not a
column: [the native arm and store integrity](https://github.com/beyond10x/engineering-protocols/blob/main/docs/design/native-arm-store-integrity-design-v0.1.md).

## Repository automation (`cargo xtask`)

For contributors to the repository itself; each `--check` variant fails on any byte of drift.

| Command | Regenerates |
|---|---|
| `cargo xtask schema [--check]` | `schemas/generated/` from the Rust types |
| `cargo xtask generate [--check]` | `generated/` — the projections of the example specifications |
| `cargo xtask suite [--check]` | `suites/generated/` — the conformance suites |
| `cargo xtask synth [--check]` | `generated/rust\|go\|web/` — the synthesized trees, then builds them and runs the dual-target demonstration |
| `cargo xtask infra [--check]` | the example cluster's committed IR, simulation, drift and projection |
| `cargo xtask status [--check]` | `docs/status.md` — the delivered-waves record, from the repository's tags |
| `cargo xtask fmt [--check]` | formatting, scoped to workspace members |
