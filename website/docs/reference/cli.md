---
title: CLI reference
sidebar_position: 1
description: Every subcommand of the AEP reference CLI, grouped by protocol, planning, adoption, driver, evidence, entity, trace, contract and evaluation surfaces.
---

# CLI reference

The canonical CLI is `aep`; `protocol` is an exact compatibility alias. Building
`protocol-cli` leaves both under `target/release/`. Current examples use `aep`; existing automation
may keep `protocol` because both names produce identical standard output, standard error and exit
status. `--help` on any subcommand carries the full flag list — this page is the map.

Most verbs take `--format text|yaml|json`, with `text` the default: refusals, decisions and
evaluations all serialise. The exceptions are named in their own sections. Workflow drawings have
`svg` and `png`, and the verbs that mint evidence default to `yaml` because that is what
`aep evaluate --evidence` reads back.

General conventions: exit `0` is success, exit `1` is a refusal or invalid input, and errors
accumulate — a run reports every problem it found, not the first. Reporting verbs exit `0` whenever
they produced their report. `trace check` uses a third code for unverified input.

## Protocol surface

| Command | Does |
|---|---|
| `aep validate [--root .] [--artifacts m.yaml]` | checks a document tree structurally and semantically — including that every rule could actually fire |
| `aep resolve --task task.yaml [--root .]` | resolves a task into an execution plan: workflow, principles in force, capabilities, obligations |
| `aep inspect [reference]` | shows what a protocol, principle, workflow or profile declares — `aep/1`, `test-driven`, `development.standard` |
| `aep evaluate --task … [--artifacts …] [--evidence e.yaml]… [--advance]` | evaluates an execution: what is owed, what is permitted, what is missing; `--advance` also attempts transitions |
| `aep explain --task … --action production.write` | explains one decision — allowed or denied, by which rule, and what would unlock it |
| `aep schema [name]` | prints AEP's generated JSON Schemas, or one by file stem |
| `aep conformance [--level core\|audited\|full] [--suite name] [--inject fault]` | checks a storage backend against the AEP contract suites (16 suites at `full`, 14 at `audited`, 7 at `core`); `--inject` breaks one property on purpose to show the responsible suite fails |

Inside a project — a directory holding `.engineering/` — `resolve`, `evaluate` and `explain` take
their `--root`, `--task` and `--artifacts` from `.engineering/project.yaml`, so the three long paths
collapse to the verb. `explain --action` exits `1` when the answer is *denied*, which is what lets a
harness ask before it acts.

## Planning surface

`aep artifact` reads and writes the markdown planning store: one artifact per file, YAML
frontmatter, free markdown body, under `<project>/.engineering/planning/` unless `--store` says
otherwise. The consequence for a person is the reason it is markdown and not a database: the diff of
a status move is one line, and `git log` already knows who made it.

Every verb here takes `--store` and `--root` (the document tree the lifecycles and templates come
from, default `.`). `list --format json` and `show --format json` always carry `relations` — `[]`
when the artifact has none — so a `jq` shape needs no branch for an artifact without edges;
`graph --format json` does not yet.

| Command | Does |
|---|---|
| `aep artifact new <kind> <name> --title … [--summary …] [--owner …] [--tag …] [--relate rel:id] [--from <path\|->] [--withholds <evidence-kind>]` | writes one file, at the path the id determines, with the body from `--from` or else the kind's template; refuses to overwrite an existing one. `--from` is the only way a body reaches an immutable kind such as `review-result`, which refuses `body`. `--withholds` records the evidence kind this artifact is stopping anybody from producing, and is only meaningful beside `--relate blocks:<id>` |
| `aep artifact move <id> --to <status> [--via] [--evidence <kind>=<count>] [--at <instant>]` | moves it if the kind's lifecycle permits, and on a refusal names every status it could have moved to instead — or, when the rung is on the ladder but its evidence has not been recorded, says **which kind** is missing and how many. It also runs the graph rules on the store the move *would* leave and refuses one that would add a finding, printing that finding's own text and hint, so `move` and `validate` cannot disagree about one document. `--via` walks the ladder's intermediate rungs, journalling each hop; a rung with a `requires:` or a `when:` stops the walk, evidence or no evidence. See [Lifecycles, decided as data](../concepts/lifecycles.md) |
| `aep artifact relate <id> <relation> <target>` *or* `relate <id> <relation>:<target>` | adds one edge, spelled either way — the second is the form `new --relate` takes, split at the first colon — and journalled identically whichever was typed. `blocks` is the one the listings read: while the artifact declaring it is short of the end of its own ladder, everything it points at is marked `blocked` |
| `aep artifact body <id> --from <path\|-> [--append] [--section "<heading>"]` | writes the markdown body while preserving CLI-owned frontmatter, as one `update` in the journal. Bare, `--from` is the whole body; `--append` adds it to the end; `--section "<heading>"` replaces the prose under that `##` heading, or adds the section at the end when the document has none. Changed bytes bump one revision, identical bytes do nothing, and a `--from` that holds nothing but whitespace is refused naming the flag |
| `aep artifact set <id> [--title …] [--summary …] [--owner …] [--tag …] [--untag …]` | changes one frontmatter field through the same command path `body` uses, so the reason every hand edit of a planning document happened is gone. Values may begin with `-`. `status`, `revision`, `id` and `kind` are accepted and **refused by name**: a status is `move`'s and carries a lifecycle decision, a revision is the store's own count of its writes, and an id and a kind are identity, fixed at `new` |
| `aep artifact show <id> [--format text\|yaml\|json]` | one artifact, printed: id, kind, status, title, summary, owner, tags, relations, `withholds` and revision, then the markdown body **verbatim**. The verb for an id in hand — `list` prints the whole plan, `explain` answers what made a status happen and `body` writes. An id the plan does not hold is refused, naming it. `--body-only` prints the body bytes and nothing else — what `body --from` would write straight back — and is refused with `--format yaml\|json`, which would wrap them |
| `aep artifact list [--kind …] [--status …]` | the plan, one line per artifact. An artifact a blocker still stops carries `blocked: <type>` after its title, and every row of `--format json` carries a `blocked_by` list — empty where nothing stops it, never absent |
| `aep artifact board [--kind …]` | the same plan as status columns, with `[blocked: <type>]` on the card. The marker rides on the card and not on a column of its own: a blocked story is still `active`, which is exactly the complaint |
| `aep artifact blocked [--type <type>]` | what is stopped, by what type, and on which single item — **grouped by the blocker**, so five stories waiting on one decision are one row with five lines under it rather than five conversations. A `blocks` edge counts until the artifact declaring it reaches the end of its own ladder, so `move <blocker> --to cleared` is how something is unblocked and the journal keeps the record that it was stuck. Always exits 0: it is a report. When no lifecycle in force declares a blocker kind at all, it says so and points at `aep artifact kinds` rather than answering `nothing is blocked`, which reads as good news about a mechanism the store does not have |
| `aep artifact graph [--format dot\|json]` | the plan's graph — `dot` for `dot -Tsvg`, `json` for a consumer that would otherwise parse a diagram |
| `aep artifact history <id> [--format text\|yaml\|json]` | what happened to one artifact, oldest first, out of the store's append-only journal: creations, moves, and every evidence record. A corrupt journal line is skipped **and counted**, never silently dropped |
| `aep artifact explain <id> [--format text\|yaml\|json]` | what made this what it is. Every blocker still in force is named first — with the evidence kind it is withholding, when it names one, which is the answer to *why is there no record* — then, per status it reached, the move and every evidence record admitted since the previous one — each named against the revision the artifact was at when it was admitted, so a later edit cannot re-date an old record onto the new text. A status reached on nobody's record is marked rather than left blank. Not `aep explain`, which is how a policy decided. It ends with one line per legal next rung — `next: <status> needs <n> <kind> record(s); held: <m>` — so a rung's price is read rather than learnt by being refused |
| `aep artifact evidence <id> --kind <k> --source <s> [--ref <url>] [--at <iso8601>]` | records an observation about an artifact, so a later move can be decided on it. `--at` defaults to now, read at the edge |
| `aep artifact validate [--strict]` | every file, every edge, every status, accumulated into one list: a file where its id does not put it, an edge pointing at nothing, a cycle, a duplicate id, a status the lifecycle does not have. Over a markdown plan it reads the event log too: a document edited outside a command is **drift**, and a `revision:` higher than any event for it records is a **forged revision** — a number no write produced, reported and never refused. Three classes are reported and not failed on — a status closed on an assertion, a document predating the event log, drift — because each is legal in a store people are working in; `--strict` turns each into exit 1 for the caller that wants it, prints the same lines and names which class decided |
| `aep artifact kinds` | what can be created: the compiled vocabulary, marked planning or output, plus every kind the document tree declares a lifecycle for, plus one row for the open `<type>-blocker` family — which no list can enumerate, because the type is whatever would clear the blockage |
| `aep artifact relations` | the 14 relations, with what each edge means |
| `aep artifact lifecycle <kind>` | where a kind starts, and what may follow what |
| `aep serve [--store …] [--root …] [--port 8899] [--read-only]` | the plan in a browser: status columns, one artifact's fields and body, and the rungs it may take next with what each costs. A rung whose price is unmet is drawn with the price on it rather than left to be refused. Answers the same facts `board`, `show` and `explain` print, and takes a status move back through the same decision `move` makes — including the graph rules on the store the move *would* leave. **Binds `127.0.0.1` and there is no flag that widens it**; reaching it from another machine is `ssh -L`. The URL it prints carries a token for the run, and a request without that token is refused — which is what stops another page in the same browser writing to the store. That is not authentication, and the module says so. `--read-only` answers reads and refuses every transition by name |

`new`, `move`, `relate` and `body` write without an `--out`; they are explicit planning-store mutations.
The difference is that they write exactly one file, at a path the id determines, inside a directory
somebody opted into — and an item you did not want is removed with `rm`.

**Every write is journalled with an actor, and the caller says who.** `AEP_ACTOR` declares it —
`human:<name>`, `agent:<name>`, `service:<name>` or `system` — and a value that does not parse is
refused, naming the variable, rather than quietly replaced by yours; unset, the write is
`human:$USER` as before. `aep drive` sets it to `agent:<execution id>` on every process it
starts for a step, so a `command` step's `aep artifact move` reads back as the run's own act
in `aep artifact history` rather than as the operator's. Nothing here *verifies* an
identity — it is a declaration, exactly as strong as the rest of the provenance model.

**A hybrid plan has two verbs of its own.** `store: hybrid` in `project.yaml` keeps the plan in
markdown *and* in a replica under a declared policy. A write one side took and the other did not is
a **divergence** — recorded, never silently merged, because the two sides disagreeing is a fact
somebody has to see.

| Command | Does |
|---|---|
| `aep artifact divergences [--store …] [--root .] [--format text\|yaml\|json]` | the divergences a hybrid plan has recorded: writes one side took and the other did not. Only a `store: hybrid` plan has any, and the exit code says whether anything is outstanding |
| `aep artifact catch-up [--store …] [--root .] [--format …]` | replays those divergences at the side that has not seen them — the runtime's catch-up (`store-v0.1.md` R-108). What the authority holds **now** is replayed, nothing is merged, and a replica that moved on its own stays outstanding for a person |

## Workspace surface

`aep workspace` answers across the repositories `.engineering/workspace.yaml` names and pins, so
a plan spanning four repositories is one question rather than four. The consequence for a person is
that a member nobody has checked out is a **normal** condition that says so, rather than an error
that stops the answer.

| Command | Does |
|---|---|
| `aep workspace members [--root .] [--fetch] [--format text\|yaml\|json]` | the members, where each one resolves to, and whether its store is there. `--fetch` materializes a pinned Git member instead of reporting it unresolved, and is **off by default**: it is the one thing here that reaches a network, and a read-only report should not do that because somebody typed `members` |
| `aep workspace list [--root .] [--kind …] [--status …] [--member …] [--format …]` | the plan across every member, one line per artifact |
| `aep workspace crossings [--root .] [--strict] [--format …]` | every relation that crosses a member boundary, and whether its target is there. `--strict` exits 1 when one does not resolve — a gate for a workspace whose members are all present, and not the default, because an unresolved crossing into a member you have not checked out is not a defect |
| `aep workspace show <reference> [--root .] [--format …]` | where one reference points, and what to type when more than one member holds it. `kind:name`, or `member/kind:name` to say which member |

## Property surface

One verb, and it writes rather than decides — the same split as `aep trace evidence`.

| Command | Does |
|---|---|
| `aep property evidence [--out …] [--format …]` | runs the properties and writes the `property_test_result` document a run reads; standard output when `--out` is absent. Exits `0` whatever the properties said, because the verdict belongs in the record and the engine is what decides on it. A caller who wants the verdict as an exit code is asking for a test runner, and `cargo test` is one |

## Specification surface

One verb, and it answers one question: **is the specification this task is being held to satisfied
by what this run observed?** It reads the planning store and a run's snapshot, decides every
requirement, and writes the `specification` evidence record `aep evaluate --evidence` accepts —
the record `spec-driven` reads as `specification.satisfied` before a task may complete.

| Command | Does |
|---|---|
| `aep specification evidence [--store .engineering/planning] [--task <file>] [--snapshot <file>] [--artifact <id>] [--out <file>] [--format text\|yaml\|json]` | decides the specification of this task's work, requirement by requirement, and writes the record naming what is unmet |

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

`aep reverse` points the tooling at a repository that already exists and was not written with
any of this in mind. Three of its four verbs **write nothing** — the consequence for a person
evaluating the tool is that you can run it against your own repository before deciding anything,
and the worst case is a report you disagree with.

| Command | Does |
|---|---|
| `aep reverse scan [root] [--format text\|yaml\|json]` | reads a repository and reports what it already says about itself — headings, declared toolchains, gates, test layout — as an `aep.reverse-scan/1` bundle. **Writes nothing** |
| `aep reverse history [root] [--recent 500] [--top 15] [--format …]` | reads what the repository's own git history says: who touches what, which areas are dormant, where change concentrates. **Writes nothing** |
| `aep reverse tickets --provider <name> [--repository .] [--top 100] [--format …]` | joins the tracker keys in the history and in the plan's prose to the references the store holds: what is recorded, what an `artifact set --ref` would record, and which keys no artifact names. **Writes nothing** |
| `aep reverse openapi <path> --domain <name> [--out …]` | drafts an `ess/1` domain from an OpenAPI document that already exists; standard output when `--out` is absent |
| `aep reverse init --protocols <path-or-git-locator> --profile <profile> [--root .] [--protocol adp/1] [--summary …] [--no-verify]` | writes the `project.yaml` that makes a repository an adopting project. This is the one that writes, and it resolves the protocol source first unless `--no-verify` says not to |

`--protocols` takes a path or a pinned `git+…#<40-hex>` locator: a governing document tree that
could move under you is a gate whose meaning changes without a commit in your repository.

See [Adopting a repository that already exists](https://github.com/beyond10x/aep/blob/main/docs/guide/adopting.md)
for the walkthrough.

## Driver surface

`aep drive` walks a workflow: it makes the engine's calls in order, runs the three kinds of
step that touch the world — a program, a model, a person — and records what it did. It evaluates no
gate itself, because a driver that could evaluate a gate would be a second protocol implementation
with none of the conformance suites behind it.

| Command | Does |
|---|---|
| `aep drive run [--map <file-or-id>] [--budget-usd <usd> --assume-usd-per-run <usd>] [--pause-on-approval] [--approver agent:<name>] [--max-iterations 25] [--take-lock] [--allow-evidence-gap]` | starts a new run of a task, allocating a run id such as `AUTH-142/3`; a map with an `llm` step requires both cost flags and `METAHARNESS_LIVE=1` |
| `aep drive status [--run <id>]` | what the store's last run is doing, and who holds the lock |
| `aep drive transition [--run <id>]` | answers a native loop's `transition` hook from the engine: the loop's JSON on stdin; exit `0` proceeds, `2` refuses with `{"reason": …}`; writes nothing |
| `aep drive resume <run> [--budget-usd <usd>] [--pause-on-approval] [--approver agent:<name>] [--max-iterations 25] [--take-lock]` | continues a run that stopped, re-taking the store lock; the optional budget may narrow, never raise, the launch cap |

All three discover `--project`, `--root`, `--task` and `--store` from the project when omitted, and
take `--plugin-dir` (repeatable; `AEP_DRIVE_PLUGIN_DIR` supplies it when the flag is absent) to load
a harness plugin into every `llm` step's session. `--pause-on-approval` runs until the first thing a
person owes, then persists and exits `0`; the resume walks on from the step after it. What answered
the `operator` step is read on that resume from the run's own record: a granted `approval` a
person recorded while the run was stopped always counts, and `--approver agent:<name>` admits one
named agent's recorded approval as well — never the run's own actor, which is refused by name. The
cursor then says who answered (`aep drive status`: `answered …`). With an approver named, a
resume that finds no admissible approval stops again and says who would be admissible; with none
named, a resume that finds nothing walks on as it always did and the report says the record holds
nobody's answer. `run` and `resume` exit `0` when the run completes or stops awaiting an operator,
and `1` otherwise.

What a run writes beside its cursor, in `.engineering/runs/<run>/`: `launch.json`, how the run was
started — which is what makes the printed `resume with: aep drive resume <run>` line a line
that works, since `resume` fills in `--map`, `--task`, `--pause-on-approval` and `--plugin-dir` from
it and a flag typed on the resume still wins; `spend.json`, the exact integer-microdollar
reservations made before model sessions; `commands.jsonl`, one line per `command` step attempt
naming the program the map wrote, the program that was spawned and which of the two it was; and a
`step-context.json` per `llm` step. `--max-iterations` bounds the call, not the run's lifetime, so a
resume gets the budget the operator typed.

A map with an `llm` step is a paid run even when this machine or a later pre-flight would prevent
the first launch. It is refused before a lock or run id unless `METAHARNESS_LIVE=1`,
`--budget-usd <usd>` and `--assume-usd-per-run <usd>` are all explicit. Dollar text is converted
exactly to integer millionths; exponent notation, negative values and more than six fractional
digits are refused rather than rounded. Immediately before each metaharness spawn the assumed
charge is durably reserved in `spend.json`; if the next reservation would cross the cap, the run
stops with `budget-exhausted` and no process is spawned. A resume remembers both figures. It may
lower the cap with `--budget-usd`, and may neither raise it nor change the per-launch assumption.
An older paid run with no remembered cost terms is not resumable, because a new cap cannot bound
sessions that already ran. A command-only map needs none of these flags and writes no spend ledger.

Four refusals and fallbacks to know before the first paid run. A `command` step whose program is
`protocol` runs the driver's own binary, whatever is first on `PATH`; where the driver cannot name
its own binary and the `protocol` on `PATH` is another version, `run` and `resume` refuse before
allocating a run id, naming both versions. A run whose `llm` sessions could not reach the `protocol`
CLI is refused before anything is spent — the child environment is constructed, and its `PATH` is
`$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin`. With no `--plugin-dir` and no
`AEP_DRIVE_PLUGIN_DIR`, no plugin is loaded; AEP never guesses a repository-local marketplace
path. Every `llm` step is told which task the run drives before the map's own words.

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

`aep workflow render` draws the same thing for a reader: the states down the page, the guards
beside the arrows, and — with `--run` or `--state` — where a run is, where it has been, what it
produced and why it stopped. It evaluates nothing; every overlay was decided by the engine and read
out of a run directory.

| Command | Does |
|---|---|
| `aep workflow render --id adp/default [--root .] [--format svg\|html\|png\|tui] [--out f]` | the workflow, as a standalone SVG, a self-contained HTML page, a raster image by way of `rsvg-convert`, or one terminal frame |
| `aep workflow render --id … --run AUTH-142/3 [--project …] [--watch]` | the same figure with a driver run drawn over it; `--watch` redraws as the run advances, and is `--format tui` with `--run` only |
| `aep workflow render --id … --state snapshot.yaml` | an engine snapshot drawn over it instead |

Without `--out`, everything but `png` goes to standard output.

Two more verbs read the same documents. `aep workflow instruct` writes a workflow out as
instructions in words, for a reader with no canvas: the states as things you may not enter yet, the
guards as what opens each move, and the principles that time obligations against the phases those
states declare, joined to the states each lands on. `aep workflow flow` projects a workflow into
the document the b10x harness walks natively (`b10x-harness workflow run --flow`). It is an honest
projection and not an equivalence: that notation is a DAG of sub-trees and this graph goes backwards,
so a retreat becomes a group that repeats, terminal states are dropped because nothing runs in them,
and **no guard travels at all** — the governor stays a program the loop asks at every section
boundary, not a field in the document. What it answers for free, before anything is paid to run, is
whether the shape fits.

| Command | Does |
|---|---|
| `aep workflow instruct [--id adp/default] [--root .] [--out f]` | the workflow as instructions; without `--id`, every workflow the tree declares, into a directory |
| `aep workflow flow --id adp/default [--root .] [--map <file-or-id>] [--max-attempts 3] [--out f]` | the projection; with `--map`, each node carries what a harness does in that state — an `llm` step as its prompt, context, write scope and harness, a `command` step as its argv and the evidence it establishes, an `operator` step as what it asks for. Every state is a group named for it — its steps chained in the map's order, or one node when the map gave it one step or none — so a loop that asks its `transition` hook at every section boundary asks it at every state |

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
| `aep evidence scan <paths>… [--at 2026-09-01] [--warn-days n] [--strict] [--fail-on-expired]` | reads human-written markdown for dated claims and reports coverage beside the classification; a directory is read one level deep for `*.md` |
| `aep evidence inspect <files>… [--at …] [--horizon 7d]` | reads the evidence document `aep evaluate --evidence` submits and reports, per record, when somebody last looked |

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
record whose observation time is in the future, naming the file and the record's position in it —
the engine's own comparison, available before anything is submitted, so `inspect` and
`aep evaluate --evidence` answer identically about one file. A calendar date is refused only
once that day has begun in no timezone (its midnight at UTC+14); an epoch value is compared exactly.
`--at` is the one place the two verbs differ: it pins the comparison to the **end** of the named day
instead of the wall clock, so reading a record on the day it was written keeps working.

## Entity surface

These seed an **in-memory** backend from `--artifacts` (an artifact manifest) or `--planning` (a
markdown planning store) and then answer; one of the two is required. Nothing is durable, and what
`history` shows is this run's seeding — every entity is at revision 1.

| Command | Answers |
|---|---|
| `aep entity list <--artifacts m.yaml\|--planning dir> [--type aep.design/v1]` | every entity the source seeds, with type, locator, revision |
| `aep entity get <source> <locator-or-id>` | one entity; exit 1 when nothing matches |
| `aep entity history <source> <ref>` | revision records, oldest first |
| `aep entity relations <source> <ref> [--incoming]` | what an entity points at, or what points at it |
| `aep audit <source> [--correlation …] [--entity …] [--rejected]` | the audit trail, oldest first; `--rejected` shows only refused attempts |
| `aep describe <source> <entity-type>` | what a type *is*: mutable or not, which commands may target it, which relations it may have |

`--organisation` (default `local`) and `--space` (default `manifest`) set the namespace the seeded
locators live under.

## Trace surface

Inputs are transcripts a harness already wrote; no verb runs an agent, calls a model or reaches a
network. All three take `--format text|json`, except `trace evidence`, which writes the record and
so takes the shared `text|yaml|json` with `yaml` the default.

| Command | Does |
|---|---|
| `aep trace inspect --transcript <file>` | the transcript's census from the typed event IR: event families, per-tool traffic in both directions, per-step `gen`/`exec` timing |
| `aep trace check --spec <file> --transcript <file> [--redact] [--advisory <id>]` | judges the run against a `trace-spec/1` document: `ok` / `gap` / `unk` per expectation, every verdict citing event indices — exit 0 conformant, 1 contradicted, 3 unknown |
| `aep trace evidence --spec <file> --transcript <file> [--advisory <id>] [--observed-at date] [--out <file>]` | mints the verdict as a `trace_conformance` evidence record (producer `trace-checker`, digest pair binding it to one transcript and one spec) that `aep evaluate --evidence` accepts |

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
`aep conformance`, which asks whether a storage backend implements `aep-contract`; the two
share the word and nothing else.

| Command | Does |
|---|---|
| `aep contract evidence --record <file> --observed-at <date> [--out <file>]` | reads a `contract_result` record a contract runner emitted and writes the AEP evidence document it implies (producer `contract-runner`, the record's bytes digested into the provenance) that `aep evaluate --evidence` accepts |

The record is one JSON object in the shape `aep-domain` defines —
`{kind, checked, failed, breaking_changes, provider, consumer}` — which is what
`metaharness conformance <kind> --contract` prints. Redirect it to a file and hand the file over;
`--record` takes a path rather than standard input so that the bytes the provenance digest names
exist somewhere a later reader can go and check.

`--observed-at` is required, unlike `aep trace evidence`'s. That verb runs its check in its own
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
`aep evaluate` is what decides on it.

## Evaluation surface

How well a harness follows these workflows, under four treatments — `raw` instructions, an
operator-selected `plugin`, a `driven` run whose every tool call is answered at a seam, and a `native` run whose
published toolset *is* the policy. `metaharness` is a tool here, the way `git` is: found on `PATH`,
and a machine without it is told so by name and exits `2` rather than reddening a gate.

| Command | Does |
|---|---|
| `aep eval matrix <runs>… [--format text\|json] [--out <file>]` | assembles the outcome matrix from `*.manifest.yaml` / `*.report.json` pairs: per harness × arm × workflow and per expectation, how many facts held, how many were contradicted, and how many nobody could find out |
| `aep eval run --arm raw\|plugin\|driven\|native --harness … --case … --out <dir> --observed-at <date> [--plugin-dir <dir>] [--stream <file>] [--budget-usd <usd>]` | runs one arm of one case and leaves the documents `eval matrix` reads; the plugin arm requires an external plugin directory; `--stream` ingests a recorded run and spends nothing |

`eval matrix` exits `0` whenever a matrix was assembled, whatever it says: a matrix is a report, and
an exit code that moved with the counts would be the single number it refuses to compute — there is
no score, no ranking and no percentage in the output. Nothing spawns without `METAHARNESS_LIVE=1`
and `--budget-usd`. Arms `driven` and `native` are not launched from here and the refusal says what
launches each: `aep drive run` and `b10x-harness`.

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
column: [the native arm and store integrity](https://github.com/beyond10x/aep/blob/main/docs/design/native-arm-store-integrity-design-v0.1.md).

## Repository automation (`cargo xtask`)

For contributors to the repository itself; each `--check` variant fails on any byte of drift.

| Command | Regenerates |
|---|---|
| `cargo xtask schema [--check]` | `schemas/generated/` from the Rust types |
| `cargo xtask status [--check]` | `docs/status.md` — the delivered-waves record, from the repository's tags; the gate-step list in `AGENTS.md` § Gate and the website's currency line, both from `Taskfile.yml`'s `check:` block |
| `cargo xtask fmt [--check]` | formatting, scoped to workspace members |
| `cargo xtask release` | nothing — reports, one line each, whether the newest release was cut completely: workspace version, `CHANGELOG.md` heading, tag pushed to `origin`, GitHub Release present, and a `test_result` in the planning store naming the tag's commit. Reaches the network, so `task release-check` rather than a gate step |
