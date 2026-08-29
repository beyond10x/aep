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
revision: 2
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
- **`protocol specification evidence` choosing which specification a run is held to.** It still picks
  *the* approved `specification` artifact in the store and refuses when there is more than one
  (`crates/protocol-cli/src/specification.rs`). That refusal is now stricter than the guard it serves
  and could read the same binding; it is a separate change with its own refusals to get right.
- **A `ValidationCode`.** The malformed form is refused at the parse stage, which is where invariant
  2 puts it, and there is no `ParseError` → `ValidationError` bridge in the workspace. The stable
  identity is the variant plus its `kind`, matched as such and never by message text.

## Implementation notes (2026-08-29)

**The declaration.** `relation: {kind: specifies, target: task}`. `target` is a new closed vocabulary
on `RelationRequirement` with one member, beside the existing `target_kind`
(`crates/aep-domain/src/requirement.rs`, `RelationTarget`). A vocabulary rather than a `for_task:
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
