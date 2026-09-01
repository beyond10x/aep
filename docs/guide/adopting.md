# Adopting the protocol in your repository

For a team that already has rules — in a wiki, a `CONTRIBUTING.md`, or in one senior engineer's head
— and wants them enforced instead of restated. This is the narrative version of the
[document authoring brief](../plan/document-authoring-brief.md); the brief holds the reference tables
(every capability, evidence kind, fact path, predicate operator) and this page holds the order to do
things in.

Every command below assumes `B=target/debug/protocol` after `cargo build -p protocol-cli`, run from
the root of the tree being checked.

## What you bring

Three things, and only the first two are yours to write.

| You provide | What it is | Smallest useful form |
|---|---|---|
| `task.yaml` | What is being done, and which profile governs it | 5 lines |
| `artifacts.yaml` | Where the specs, designs, ADRs and reviews are — references, not copies | 1 entry |
| a profile name | Which bundle of rules applies | one of `development.fast` / `.standard` / `.critical` |

Everything else — the workflow, the principles in force, what the agent may do, what counts as
finished — is derived from the profile. Restating any of it in the task is how it drifts.

A task that resolves:

```yaml
id: BILL-88
kind: feature
objective: move-invoices-to-partitioned-table
protocol: adp/1
profile: acme.service
manifest: artifacts.yaml
constraints:
  facts:
    # Declared here because nothing can observe it. A principle reads it to decide whether it applies.
    change.database_schema: true
    change.public_contract: false
    change.architectural: false
```

And the manifest it points at:

```yaml
version: aep.artifacts/1

artifacts:
  - id: spec:invoice-partitioning
    kind: specification
    status: approved
    location:
      path: docs/specs/invoice-partitioning.md

  - id: plan:invoice-partitioning
    kind: migration-plan
    status: approved
    location:
      path: docs/migrations/0007-partition-invoices.md
    relations:
      - derived_from: spec:invoice-partitioning
```

The manifest holds no copies. The design stays in `docs/`, the PRD stays in the planning tool, and
neither has to move for the protocol to reason about them.

## Where the documents live

The loader walks six directories under one root, recursively, skipping anything dot-prefixed:
`protocols/`, `principles/`, `workflows/`, `profiles/`, `artifacts/lifecycles/` and `drivers/`. That
root is a **document tree**, and everything below is about which tree your documents are in.

```text
your-repo/
  protocols/                        vendored, unchanged — the declared vocabulary
  workflows/                        vendored, plus any state machine of your own
  principles/
    upstream/                       vendored
    migration-has-a-way-back.yaml   yours
  profiles/
    upstream/                       vendored
    acme-service.yaml               yours
  artifacts/lifecycles/             vendored
  drivers/                          step maps, only if you drive runs with `protocol drive`
```

Vendor by submodule, subtree or copy — the loader cares about content, not provenance. Documents are
indexed by the `id` declared *inside* the file, never by path, so a subdirectory layout of your
choosing costs nothing.

| Lives in this repository | Lives in yours |
|---|---|
| `protocols/` — the vocabulary. Extending it is a protocol change, not a configuration change | your principles |
| `principles/`, `workflows/`, `profiles/` — the defaults, worth reading before replacing | your profiles, usually `extends:` one of theirs |
| `artifacts/lifecycles/` — what statuses each artifact kind may hold | your `task.yaml` and `artifacts.yaml`, per task |

One consequence worth knowing before you need it: **a kind's last hyphen segment names its parent**,
so a family of kinds of yours shares one ladder. That is how the typed blocker works — a
`credential-blocker`, a `decision-blocker` and your own `procurement-blocker` are all governed by
`artifacts/lifecycles/blocker.yaml`, and a new type of blocker costs a name in a document and no
change to any crate. `artifacts/kinds/blocker.yaml` writes down six types with what clears each,
and nothing checks a type against that list; see
[blocker types](../../website/docs/reference/vocabulary.md#blocker-types).

## Two shapes of adoption, and the one thing that decides between them

You can point at somebody else's tree instead of owning one. A project does that with a small file,
`.engineering/project.yaml`, which every command finds by walking up from where it is run:

```yaml
protocol: adp/1
profile: acme.service
protocols: git+ssh://git@github.com/beyond10x/aep.git#0123456789abcdef0123456789abcdef01234567
schemas: schemas
```

Write it with `protocol reverse init` rather than by hand:

```console
$ $B reverse init \
    --protocols git+ssh://git@github.com/beyond10x/aep.git#0123456789abcdef0123456789abcdef01234567 \
    --profile acme.service
.engineering/project.yaml written
  protocol source resolves to /home/you/.cache/aep/0123456789ab…
  profile acme.service
```

It refuses an unpinned source before writing anything, resolves the tree so the failure surfaces
here rather than at the first command that needed it, and leaves no file behind when it does not
resolve — a half-adopted repository whose every later command fails on the same unreadable tree is
worse than one that was never adopted, because it looks adopted. `--no-verify` writes the file
without the resolution step, for an offline machine or a source that is not reachable yet.

The suffix is the full commit id, not a branch or tag. On first use the engine fetches that immutable
revision into `AEP_CACHE_DIR`, then `XDG_CACHE_HOME/aep`, or the conventional user
cache. Later commands verify and read the cached checkout, so the project file contains no
machine-local or cross-repository path and an already materialized source works offline. A local path
is still valid when a repository owns its tree or a fixture supplies one; relative paths are resolved
from `.engineering/`.

**An absolute path is refused, and this is the one rule about the project file worth knowing before
you write it.** `protocols: /opt/aep` names a place on one machine. The file is
committed, so every other machine that clones the repository reads a path that is not there, and CI
reads one that has never been there:

```console
$ $B resolve
error: [type_mismatch] project.protocols: `/opt/aep` is an absolute path (it is
  rooted at the filesystem root) (hint: use a path relative to the .engineering directory, or a
  pinned git+ssh://, git+https://, or git+file:// locator …)
```

The refusal is in the reader rather than in `reverse init`, so a file hand-edited afterwards fails
the same way, and it covers `~`, a drive letter and a UNC path as well as a leading `/`. It is
checked by spelling and not by the platform's own notion of absolute, so one project file gets one
verdict everywhere. Every other path in the file — `artifacts`, `task`, `principles`, `profiles`,
`schemas` — was already held to this rule; as of 2026-08-25 `protocols` is too.

Two ways to name a tree that is not inside your repository: a relative path that climbs out of it,
or a pinned locator. Prefer the locator for anything other people will clone.

`store:` names where the plan is kept — `markdown` (the default, one document per artifact under
`.engineering/planning/`), `sqlite: <file>`, `postgres: <url>`, or `hybrid:` (markdown and a replica,
under four declared words) — and every `protocol artifact` verb opens through it; a relative file is
relative to `.engineering/`. See [`backend.md` § Choosing the store](backend.md#choosing-the-store).

`schemas:` names the project's own JSON Schema registry, also relative to `.engineering/`; it
defaults to `schemas`, so `.engineering/schemas/` needs no explicit entry. The path locates the
registry while each schema's absolute `$id` identifies its contract. `protocol schema validate`
and `protocol schema typescript` discover this same registry from the project file.

(The directory is called `.engineering` by default. If that name is taken, or your team calls it
something else, set `AEP_PROJECT_DIR=.workflow` and discovery looks for that instead — read once per
process, so it cannot change mid-run.)

**Pointing at a tree merges two of the six document kinds, and only two.** Documents in
`.engineering/principles/` and `.engineering/profiles/` are loaded *over* the tree's. Nothing else
is: a workflow, a protocol, a lifecycle or a driver step map placed under `.engineering/` is not
read at all, and the failure surfaces where the workflow is named rather than where the file sits:

```console
$ $B resolve
error: 1 document problem(s):
  - [unknown_workflow] workflow knowledge/curation: no workflow document declares
    `knowledge/curation` (hint: available: adp/default, incident/standard,
    migration/forward-only, release/progressive)
```

The file is right there in `.engineering/workflows/`. It was never read.

| What you want to add | Enough to point at a tree? |
|---|---|
| A principle of your own | **Yes** — `.engineering/principles/` |
| A profile of your own, usually `extends:` one of theirs | **Yes** — `.engineering/profiles/` |
| A state machine your work actually has — different states, different phases | No. Own a tree |
| A lifecycle for an artifact kind of your own | No. Own a tree |
| A protocol declaring facts, capabilities or evidence kinds nobody upstream declared | No. Own a tree |

So: **a new state machine means owning a tree.** Assemble the upstream documents and local additions
as one governed tree, and point the project at it. The shortest local form is `protocols: .`, which
makes the project directory itself the tree:

```yaml
protocol: adp/1
profile: acme.knowledge
protocols: .                # the tree is .engineering/ itself
principles: local/principles
profiles: local/profiles
```

```text
your-repo/.engineering/
  project.yaml
  protocols/      principles/      workflows/      profiles/      artifacts/lifecycles/
  ^ one tree, yours, with the upstream documents vendored into it and yours beside them
```

The last two lines of that `project.yaml` are load-bearing and easy to miss. `principles:` and
`profiles:` default to `principles/` and `profiles/` under `.engineering/` — which, once the tree
*is* `.engineering/`, are directories the tree loader has already read. Every file in them would be
loaded twice and refused as a duplicate id:

```console
$ $B resolve
error: 6 document problem(s):
  - .engineering/profiles/development-standard.yaml: [duplicate_principle] profile
    development.standard: a second profile document declares the id `development.standard`
```

Point the two merge paths at directories the tree does not contain — they need not exist — and the
project-local merge stays out of the way, because with your own tree you no longer need it.

## The plan the repository already has

A repository that is old enough to want rules is old enough to have a plan nobody wrote down: a
roadmap in the README, a suite switched off in CI two quarters ago, a `FIXME` that is really a story.
`protocol reverse scan` finds those and reports them with the `path:line` each was read from. It
interprets nothing and writes nothing.

```console
$ $B reverse scan --format json > bundle.json
```

The split is deliberate. Deciding whether four roadmap stages are one initiative or four is
judgement, and judgement belongs to whoever — or whatever — is doing the planning. What belongs to a
program is finding the evidence and saying it the same way twice: the scan has no clock, no network
and no `read_dir` order dependence, so two runs over one tree produce identical bytes and a bundle
can be committed, diffed and cited by an artifact that outlives the session that wrote it.

An artifact drafted from a bundle entry should carry that entry's `path:line` in its body. A plan
invented from a plausible reading of a codebase is indistinguishable, later, from one somebody
agreed to, and it is the worse of the two because nobody can check it.

What a scan cannot find is what nobody wrote down — a convention that lives in review comments, the
reason a module exists. Those are the questions to take to a person, not gaps to fill in.

### And what the history says

```console
$ $B reverse history
```

A scan reads the tree as it stands, so it can report that a suite is switched off and never that it
has been off for two and a half years. The second is what anybody acts on, and it is the one thing a
working tree cannot tell you:

```text
stated expiry: 4
  2025-12-12 443d9034  fix: skip TestDial for sip driver for now until we found why it constantly fails

line ages: 168
  2023-06-30  <path>:62  feat(test): extend tests
```

Every marked line and every disabled test, dated from the commit that wrote it and reported oldest
first — so a flat list of 156 equal items becomes a ranked one. Beside it: the commits that undid
something, the commits whose message hedged, the files the work keeps returning to, the files nothing
recent has touched, and the tracker keys the messages mention (which is often how you discover the
team migrated tracker two years ago and half the code still cites the old one).

Dates are quoted from commits and never compared against today, so a fixed `HEAD` gives fixed bytes
and a bundle stays true after it is committed. The verb needs a Git working tree and says so in one
sentence when there is none — `reverse scan` needs none and still works.

## Choosing a profile

Measured on the [worked example](../../examples/development-passkeys/) task, same artifacts, only the
profile changed:

| | `development.fast` | `development.standard` | `development.critical` |
|---|---:|---:|---:|
| principles in force | 5 | 9 | 15 |
| obligations | 6 | 10 | 17 |
| completion checks | 14 | 24 | 45 |
| distinct evidence kinds owed | 5 | 7 | 7 |

The evidence *kinds* barely grow between standard and critical — what grows is the number of runs and
who has to sign. Read the cost as what a person has to do that they were not doing before:

| Profile | Choose it when | What it costs you |
|---|---|---|
| `development.fast` | Blast radius contained, contract surface private: internal tooling, scripts, a spike | A spec, a failing test first, static analysis, provenance. It cannot request a review or an approval, so a human is never in the loop |
| `development.standard` | Anything with an external consumer, persisted data, or a customer-visible path. The default | Adds contract tests and a property suite, and the ability — with it, the obligation — to ask a human |
| `development.critical` | A silent defect is worse than a late delivery: auth, money, migrations, crypto | Adds a mutation run, a differential run against the implementation being replaced, an invariant check, an adversarial verification, contracts on the code, an approved design related to the specification, a **fresh human review of that design**, and — where the project has an executable system specification — conformance to it |

The last line of `development.critical` is the one that bites. Under it, an approval of version 3 of a
design stops satisfying the review requirement once the design reaches version 7:

```console
$ $B evaluate --task /path/to/critical-task.yaml \
    --artifacts examples/development-passkeys/artifacts.yaml \
    --evidence examples/development-passkeys/evidence/04-review.yaml | grep -A 1 review
  ✗ review of a design is approved (by a person)                  [completion]
      the approved review of design:passkeys-auth was given against a different version
```

Someone approved a design they saw. Version 7 is a different design and their name is not on it.

## Writing a principle of your own

A real rule: **a change that rewrites persisted data needs an approved migration plan before the code
is written, and something other than the agent has to have run the recovery path before it is
finished.**

Four decisions, in this order.

| Decision | Question it answers | In the document |
|---|---|---|
| Applicability | When is this rule even about this task? | `applies_when:` |
| Timing | At which point does it bite? | `before_<phase>:` keys under `requires:` |
| Obligation | What must be true or exist? | `predicates:`, `artifacts:` |
| Evidence | Who is allowed to say so? | `evidence:` with `independent: true` |

`principles/migration-has-a-way-back.yaml`:

```yaml
id: migration-has-a-way-back
version: 1
title: A schema migration has a way back
summary: >-
  A change that rewrites persisted data is not finished until something other than the agent has run
  the down path. Without it the recovery plan gets written during the incident, by whoever is awake.
applies_when:
  # A fact nothing can observe, so the task declares it. The alternative — applying the rule to every
  # task — gets it removed within a month.
  change.database_schema: true
requires:
  before_implementation:
    artifacts:
      - kind: migration-plan
        status: approved
  before_completion:
    predicates:
      - verification.recovery.passed
    evidence:
      # The agent's own report of a successful rollback rehearsal does not satisfy this.
      - kind: verification
        independent: true
```

And a profile that puts it in force:

```yaml
id: acme.service
version: 1
title: Acme service development
summary: Standard development, plus Acme's migration rule.
protocol: adp/1
extends: development.standard
principles:
  - migration-has-a-way-back
```

`applies_when` is doing real work. Two tasks, same profile:

```console
$ $B resolve --root . --task task.yaml | grep -E '^(task|principles|obligations)'
task        BILL-88 (feature)
principles  spec-driven, test-driven, static-analysis, least-privilege, provenance-tracking, contract-testing, property-based-testing, approval-gates, reversible-changes, migration-has-a-way-back
obligations 12
$ $B resolve --root . --task task-rename-a-button.yaml | grep -E '^(task|principles|obligations)'
task        BILL-89 (feature)
principles  spec-driven, test-driven, static-analysis, least-privilege, provenance-tracking, contract-testing, property-based-testing, approval-gates, reversible-changes
obligations 10
```

The rule is absent from the second task rather than present and vacuously satisfied, so nobody reads
a green report and wonders which of the ticks meant anything.

And the timing is not decorative. With the migration plan in the manifest, a failing test carries
BILL-88 into implementation. Delete the plan from the manifest and the same evidence stops one state
short:

```console
$ $B evaluate --root . --task task.yaml --artifacts artifacts-without-the-plan.yaml \
    --evidence red-test.yaml --advance | grep -E '^(state|transitions)|migration-plan'
state       establish_verifiers (Establish verifiers)
transitions
      ? artifact migration-plan (approved) — no migration-plan artifact is declared [principle migration-has-a-way-back]
```

The block names the principle. Somebody can go and read it and either write the plan or argue with
the rule — both better outcomes than an agent quietly writing the migration.

## A rule with a clock on it

A rehearsed rollback from six months ago is not a rehearsed rollback. Put a `horizon` on the
evidence requirement and the engine stops taking the old record's word for it:

```yaml
    evidence:
      - kind: verification
        independent: true
        horizon: 90          # whole days
```

Every evidence record already carries `observed_at` — the day somebody looked, written by whoever
submitted it, never stamped by the engine, and refused outright if it is in the future. A horizon is
what that date is measured against:

```console
$ $B evaluate --root . --task task.yaml --artifacts artifacts.yaml --evidence recovery-rehearsal.yaml
  ? evidence verification (independent) within 90d                [principle migration-has-a-way-back]
      the last observation was on 2023-11-13, the horizon is 90d, and it lapsed on 2024-02-11
```

**`?`, not `✗`, and the distinction is the whole point.** An expired observation is not a wrong
answer, it is an old one, so the task is blocked with *nobody knows* rather than accused of a
failure that never happened. The lapsed record's facts are withheld too, so a guard reading
`tests.unit.failed == 0` off a stale suite refuses rather than passing on it. `evidence.lapsed` is
the count of records in that state, and it sits beside `evidence.missing` because *nobody produced
it* and *somebody did and nobody has looked since* are different problems with different owners.

Two things a horizon cannot do, worth knowing before you set one:

* **It is a volatility guess, not a guarantee.** A seven-day claim can be false on day five and the
  gate will say it is fine. What follows is that *shortening* a horizon has to stay cheap and normal
  — if the only way to say "this moves faster than I thought" is prose, nobody says it.
* **There is no way to extend one.** The horizon lives on the requirement, in a reviewed document,
  and no command mutates it. The only refresh is to observe again and write a new date, which is
  deliberate: if extending were as easy to call as re-checking, it is the one that would get called
  by whoever is trying to get a gate green.

**Write the date your writers write.** `observed_at: 2026-08-30` is a *day*, not an instant, and a
store east of Greenwich writes tomorrow's UTC date for the last hours of every UTC day. A date is
refused as future only once it has begun in **no** timezone — its midnight at UTC+14 — so a correct
local date is accepted wherever it was written. Write epoch milliseconds only when you mean an
instant: that spelling is compared exactly, and the two spellings of one midnight can get opposite
answers from the same clock. A record that is genuinely in the future is still refused, and the
refusal names the file, the record's position in it and the date as written, while the rest of the
document is submitted.

`protocol evidence inspect <file>` reads the dates back out of a record without submitting anything,
and `protocol evidence scan <dir>` does the same for claims written into markdown by hand — with a
coverage line comparing annotation-shaped occurrences against records the parser actually produced,
because a scanner over human-written documents that quietly stops seeing half of them reports green
either way. [`examples/evidence-horizons-corpus/`](../../examples/evidence-horizons-corpus/) is the
regression corpus behind that, contributed by an adopter who had been keeping such claims by hand.

## The failure worth learning first

A predicate may only read facts the protocol declares observable. Suppose the rule had reached for
`migration.rollback_tested`, which reads perfectly well in English:

```console
$ $B validate --root .
47 file(s): 3 protocol(s), 23 principle(s), 4 workflow(s), 7 profile(s), 8 lifecycle(s), 2 step map(s)
1 problem(s):
  - [unobservable_fact] principle migration-has-a-way-back.obligations.migration-has-a-way-back/before-completion: `migration.rollback_tested` is not declared observable by protocol adp/1 (hint: declared families: ess_conformance.**, trace_conformance.**, mutation.**, differential.**, invariant.**, clean_room.**, build.**, types.**, task.**, change.**, risk, severity, state.**, workflow.**, principle.**, evidence.**, required_evidence.**, tests.**, test.**, unit_tests.**, contract_tests.**, regression_suite.**, static_analysis.**, contracts.**, property_test.**, coverage.**, specification.**, diff.**, source_diff.**, artifact.**, review.**, verification.**, approval.**, approvals.**, deployment.**, metric.**, service.**)
$ echo $?
1
```

`migration.**` exists — but only under `aop/1`, and this principle is in force under `adp/1`. The
error lists every family that *is* available, which is usually enough to find the right spelling
(`verification.recovery.passed`, here).

There is a second, quieter version of the same mistake. A fact in a declared family but a spelling
nothing projects passes validation and then never becomes true:

```console
$ $B validate --root .
47 file(s): 3 protocol(s), 23 principle(s), 4 workflow(s), 7 profile(s), 8 lifecycle(s), 2 step map(s)
valid
$ $B evaluate --root . --task task.yaml | grep passsed
  ? verification.recovery.passsed                                 [principle migration-has-a-way-back]
      unobserved: verification.recovery.passsed
```

A `?` that never becomes `✓` is a task nobody can finish, and it looks like a stuck agent rather than
a typo. Section 2 of the [authoring brief](../plan/document-authoring-brief.md#2-facts-the-engine-actually-projects)
lists the spellings the engine actually projects — check a new predicate against it before writing the
rest of the rule.

## Keeping the documents honest

Two commands, and they catch different things. `validate` reads the tree on its own; `resolve` also
needs a task, because some checks only mean anything once a profile and a workflow are paired.

| Caught by | Refusal | Because otherwise |
|---|---|---|
| `validate` | a predicate reading a fact the protocol does not declare | the rule can never be satisfied, and nobody finds out until a task hangs |
| `validate` | a workflow state that cannot be reached | `[unreachable_state]` — the state is decoration |
| `validate` | a non-terminal state with no way out | `[dead_end_state]` — execution wedges there |
| `validate` | a rollback policy with no precondition | `[incomplete_rollback_policy]` — "rolled back" describes a wish |
| `resolve` | an obligation timed against a phase no state declares | `[unknown_phase]` — the rule is not strict, it is absent |
| `resolve` | a task needing a capability the resolved policy denies | `[capability_conflict]` — the agent would find out mid-task |

Both exit 1, and both accumulate: a document with four broken references reports four errors, not the
first one. Add them to CI as their own step, so a broken rule reads as a broken rule rather than as
one failed assertion inside a test log:

```yaml
- name: Documents
  run: |
    cargo run -p protocol-cli -- validate --root .
    cargo run -p protocol-cli -- resolve --root . --task examples/typical-task.yaml
```

Keep at least one representative task per profile you ship. `validate` alone will not tell you that a
principle is timed against a phase your workflow does not have.

This repository does the same thing from the other side: `crates/aep-engine/tests/documents.rs` loads
the tree, asserts it has no failures, and resolves a task against every profile — so a principle that
could never fire cannot be committed. Its own gate is `task check`, and a step whose
toolchain is missing fails and names it rather than skipping.

## What is fixed in the engine, and why

Most of what this page invites you to declare is open: artifact kinds, phases, verifiers,
capabilities, evidence kinds, observables and the statuses a lifecycle names are all read from
documents, and [`open-vocabulary.md`](open-vocabulary.md) carries the table of which is which and
why. Two of them are not open, and both refusals arrive from `validate` as a message rather than in
a design review, so the reason is worth having before you write the document.

### Relation names

An `artifacts/relations/` document declares which pairings your tree means — that part is yours. The
names an edge may carry are the engine's, and there are fourteen:

```text
informed_by  derived_from  decomposes  specifies  designs  implements  decides
reviews  verifies  blocks  depends_on  supersedes  delivers  serves
```

A relation name is something the engine *acts on*, not only something it records. `supersedes` gates
a status: an artifact that reads `superseded` must have a successor declaring it. `reviews` is
mandatory on a `review-result`. `serves` names an objective — a `vision` artifact — and once a store
declares one, every `proposed`, `approved` or `active` story or task must serve one (`validate`
says which do not). `decomposes` is the edge the artifact tree is built from, and
`depends_on` is the one coverage and validation walk. Cycles are checked once per kind. None of that
can move into a document, because a document can say what an edge *means* and not what the engine
*does* about it — so an adopter-declared name would be an edge that looks like it should mean
something and that no rule will ever read. Being told `frobnicates` is not a relation is the better
failure of the two.

This is the opposite answer from artifact statuses, and the difference is where the guarantee could
live. *A status name means the same rung to every tool* survived being moved into the kind's
lifecycle ladder, so the status vocabulary opened and the guarantee stayed bought. A relation's
semantics have nowhere to go.

If you need a kind that is not on that list, it is a change to the engine and its graph rules rather
than a document you can write: ask for it with a story in this repository's planning store, naming
the rule you want the engine to apply to the new edge.

### Predicate operators

`eq ne lt lte gt gte any_of none_of exists truthy` — the mapping form takes these ten and no others.
The parser accepts aliases for most of them (`equals` and `==`, `not_equals` and `!=`, `in` and
`one_of`, `not_in`, `defined`), and it refuses an unknown operator by name rather than reading it as
false.

An operator is the step the engine *performs*, and it has to perform it three-valued. `gte` on
`risk: medium` is decided by the `scales:` the protocol declares, not by string ordering; `exists`
is the one operator that may read an unobserved fact without collapsing it to False. An operator
declared in a document would arrive with a name and no answer to either question — the engine would
know what to call it and not what it means when nobody has looked, and *Unknown is not False* is the
rule this evaluator is built to keep.

So the set is a boundary and not an omission. A comparison it does not carry — a substring match, a
set intersection, a regular expression — is a change to the evaluator, and asking for one is a story
in this repository's planning store rather than a key in your own tree.

## Next

* [`harness.md`](harness.md) — wiring an agent to the engine so these rules actually govern it.
* [`backend.md`](backend.md) — storing the entities the manifest points at.
* [`open-vocabulary.md`](open-vocabulary.md) — for each thing this page invited you to declare, whether the vocabulary is open or fixed in the engine, and what a closure buys.
