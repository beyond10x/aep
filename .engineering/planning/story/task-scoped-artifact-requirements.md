---
format: aep.planning-md/1
id: story:task-scoped-artifact-requirements
kind: story
status: draft
title: A rule about this task's specification stops accepting somebody else's
summary: before_implementation asked for any approved specification in the store; it now binds to the work the task declares.
owner: protocol
tags:
- driver
- protocol
relations:
- decomposes: epic:reference-driver
scope:
- confidence: cited
  path: crates/drive/aep-driver-spec
- confidence: cited
  path: crates/edge/aep-cli
- confidence: cited
  path: crates/govern/aep-domain
- confidence: cited
  path: crates/govern/aep-engine
- confidence: cited
  path: drivers/development/default.yaml
revision: 13
---
# Story: A rule about this task's specification stops accepting somebody else's

## Outcome

A run cannot start writing code on the strength of a specification that was never about it. The
person hitting this sees the difference in one line: the guard before `implement` now reads
*artifact specification (approved) which specifies this task*, and names what the task said it was
about when it refuses.

## Context

**`spec-driven` asked for `kind: specification, status: approved` and nothing else, which is a query
over the whole artifact store.** `ArtifactRequirement::matches` counted every artifact of the kind in
the graph, so in any store holding more than one piece of work the rule was satisfied by a
specification of a different story. The principle's own header says an approved specification must
exist before implementation *of this work*; the rule as written had no way to say "this work".

Run `NATIVE-1/1` (2026-08-29) moved `establish_verifiers -> implement` holding **zero** approvals of
its own. Its worktree's store held two approved specifications —
`specification:adopter-schema-contract-tooling` and `specification:open-vocabulary-audit` — belonging
to two other stories. Nothing was wrong with either document. The guard read them as this task's
because it could not read them as anything else.

`clean-room` carried the same declaration, and there it matters more: that principle withdraws
`network.read` precisely so the original cannot be fetched and the specification is the only
permitted source. Somebody else's specification is not a source for this work.

`RelationRequirement` could already say *a relation to a kind* — `{kind: designs, target_kind:
specification}` — and never *a relation to the thing being run*. `RequirementContext` exposed facts,
the artifact graph, the evidence log and the clock; nothing on it answered *what is this task about*.

## Acceptance

- An artifact requirement can bind to the execution's own task, and a principle that binds is
  satisfied only by an artifact whose edge lands on the work the task declares.
- Another story's approved specification, in the same store, does **not** satisfy a bound
  requirement — and the fixture proving it holds that specification, approved, matching every other
  clause of the rule.
- An unmet bound requirement reads `Unknown`, never `False`: this task's specification has not been
  written yet, and waiting is what produces one.
- The binding fails closed. A context that cannot say what its task is about satisfies no bound
  requirement; the opposite polarity would be the defect restored as a default.
- The edge the binding reads is the edge a driven run already writes. The `specify` state's artifact
  carries `specifies: story:<the task's story>`, and the task document carries
  `derived_from: story:<that story>` — no run and no example changes to satisfy the rule.
- A declaration naming a binding nothing binds to is refused where the document is parsed, and the
  refusal is recognisable by its variant and its `kind` rather than by its sentence.
- The requirement still renders as one line, and the generated instruction documents carry it.

## Out of Scope

- **Reaching more than one edge.** `design-by-contract` and `invariant-checking` ask for an approved
  `design`, and a design's edge lands on the *specification* (`designs: spec:…`), which is two hops
  from the work a task declares. A one-hop binding would refuse this repository's own worked example
  (`examples/development-passkeys/`). Binding those two needs a transitive reach, which is a
  different rule and a different story.
- **Binding `incremental-decomposition` and `preserve-evidence`.** Both ask for an artifact kind
  (`acceptance-criteria`, `incident-report`) that no run and no example relates to a task's work at
  all, so a binding would refuse every task rather than the wrong ones. `preserve-evidence`'s own
  comment names this gap — *"cannot say which artifact"* — and it stays named rather than closed on a
  relation nobody writes.
- **`protocol specification evidence` choosing which specification a run is held to.** Carried as a
  follow-up rather than done in the same change, because it has its own refusals to get right — and
  **now done**: see *The follow-up, done 2026-08-29* below.
- **A `ValidationCode`.** The malformed form is refused at the parse stage, which is where invariant
  2 puts it, and there is no `ParseError` → `ValidationError` bridge in the workspace. The stable
  identity is the variant plus its `kind`, matched as such and never by message text.

## Implementation notes (2026-08-29)

**The declaration.** `relation: {kind: specifies, target: task}`. `target` is a new closed vocabulary
on `RelationRequirement` with one member, beside the existing `target_kind`
(`crates/govern/aep-domain/src/requirement.rs`, `RelationTarget`). A vocabulary rather than a `for_task:
true` flag because it answers the question `target_kind` already asks — *which thing is at the other
end* — and two ways to constrain one edge is how the two come to disagree. It composes:
`{kind: specifies, target_kind: story, target: task}` is *specifies a story of this task*, and **one**
edge has to satisfy both halves, or a specification reaching somebody else's story and this task's
epic would count.

Deliberately not spelled `target_kind: task`, which is the different question *any artifact of kind
`task`, whosever it is*.

**What "this task" resolves to.** `RequirementContext::task_artifacts`, defaulted to empty so every
context that predates the binding evaluates as it did — and so an empty answer permits nothing.
`Execution` fills it from the task document's `derived_from`, plus the task's own id as
`task:<id>`. `context:` is deliberately excluded: it holds artifacts that *constrain* the work rather
than the work itself, and counting it would let a task admit another story's specification by listing
that story as reading material.

**What changed for a reader.** `ArtifactRequirement::matches` takes the task's declared work as a
third argument, so a caller that cannot supply it cannot silently get the old answer. The unmet row
gained one clause — what the task declares — because *two approved specifications are present and the
rule is still unmet* reads as an engine defect until the row says whose they are.

**Not bound, and why the rest of the file says so.** Only `spec-driven` and `clean-room` changed;
`design-by-contract`, `invariant-checking`, `incremental-decomposition`, `preserve-evidence` and
`ess-conformance` are argued in *Out of Scope* above rather than left silent, because the next author
will otherwise close them the same way and refuse the worked example.

## The follow-up, done 2026-08-29

**What was wrong.** Binding the guard left `protocol specification evidence` **looser than the guard
it serves**: it selected any in-force specification in the store, so a run could write a
`specification` record about a document `before_implementation` would refuse — and
`specification.satisfied`, the fact `spec-driven` reads before completion, would be a verdict about
somebody else's story. A tool that is looser than its own gate is worse than one that is stricter:
the record looks like evidence and is about the wrong work.

**One rule, shared by construction.** The set is `Task::declared_work`
(`crates/govern/aep-domain/src/task.rs`) — `derived_from` plus the task's own id as `task:<id>`, `context:`
still excluded. It was the body of `Execution::task_artifacts`; the engine now calls it and so does
the verb, so *whose specification is this* has one answer and one place to change. The match is
`ArtifactRequirement::matches`, the engine's own function, over the requirement
`{kind: specification, status: approved, relation: {kind: specifies, target: task}}` — the same
value `spec-driven.before_implementation` and `clean-room` state, held to those two files by a test
that parses them and compares (`the_rule_this_verb_selects_by_is_the_one_the_shipped_principles_declare`).

**How the verb knows the task.** `--task <file>`, which already existed, or the task `project.yaml`
names. With neither — a store handed to the verb from outside a project — the selection is unbound
and behaves as it always did, because refusing there would refuse a person asking a legitimate
question about a store they are standing outside of.

**`--artifact` was not made an escape hatch.** It names *which* specification, never *whether* the
binding applies: an id that does not specify this task's work is refused, saying so. What it does
lift is the status half, so a `draft` can still be asked whether it states anything decidable.

**What a refusal now says.** Both ends, the way the engine's own unmet row says them: what is
declared (`specification:passkeys (approved), specification:sessions (approved)`) and what the task
said it was about (`this task's work is story:billing, task:BILLING-1`), plus which task document
that came from — because the wrong task document is the failure a reader cannot otherwise see.

**Done 2026-08-29, and the original finding is kept as the record of why.** It read: *a `command`
step's map cannot say `{task}` (`CommandStep::PLACEHOLDERS` is `run_directory` and `transcript`), so
a run driven with `protocol drive run --task <a path that is not the project's>` reaches this verb
through discovery and binds to the project's task instead. It fails closed and says which document
it read, which is visible rather than silent; a `{task}` placeholder is the fix and is a step-map
vocabulary change.* It was, and that is all it was: `--task` already existed on the verb, so the
consumer was only ever waiting for a way to be told.

**`{task}` is the third and last name in the closed vocabulary**
(`crates/drive/aep-driver-spec/src/map.rs`, `CommandStep::PLACEHOLDERS`). The driver expands it to the
**absolute** path of the task document the run was started from — the one `--task` named, or the one
discovery found when no flag did. Absolute because a `command` step is spawned with the project
directory as its working directory while `--task` is relative to wherever the operator typed it, and
a placeholder that expanded to a path the child cannot open would be the same class of defect one
layer down. Nothing about it is decidable at load, so a `{task}` in a run started from no document
at all is D5's `Unknown` — the same shape as a `{transcript}` in a run where no `llm` step has run.

**A resume expands what the run was started from, not what it can work out today.** The value is
written into the run's `launch.json` beside `--map` and the b10x options, for their reason: a resume
that resolved the document again would resolve it against its own working directory and against
whatever `project.yaml` says now, so one run's steps could name two documents with nothing in the
record saying which was meant. A flag still wins, as it does everywhere else in `remembering`.

**`drivers/development/default.yaml` passes `--task {task}`**, so the shipped map binds explicitly
and discovery is what answers a person running the verb by hand outside a run.

Three tests hold it, each verified by the mutation it exists to catch:

- `the_task_document_can_be_named_and_a_misspelling_is_offered_all_three_names`
  (`crates/drive/aep-driver-spec/src/map.rs`) — the vocabulary, and the hint that has to grow with it, read
  out of `PLACEHOLDERS` rather than spelled again.
- `the_task_placeholder_is_the_document_this_run_was_started_from`
  (`crates/edge/aep-cli/src/drive.rs`) — the expansion, alone and inside a word, and the refusal for
  a run whose task was never read out of a file.
- `a_command_step_binds_the_specification_verb_to_the_task_the_run_was_started_from`
  (`crates/edge/aep-cli/tests/drive_cli.rs`) — a driven project holding two stories, two approved
  specifications and two tasks, asserting **both** halves: the map without the placeholder writes a
  record about the *project's* story, and the map with it writes one about the task the run was
  started from. Without the first half this would pass in any store with one specification in it.
