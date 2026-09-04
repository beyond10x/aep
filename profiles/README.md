# Profiles

A profile bundles a protocol version, a workflow, a set of principles, a capability policy and a
completion condition, so a task selects one line instead of enumerating three dozen rules.

`release.source` proves that one annotated release came from the default branch after its repository
gate and retains human authority over publication. `release.dependency-chain` governs the consumer
side of a typed dependency graph: prepare one atomic provider update, gate, review, merge, request a
separate release approval when required, then verify downstream convergence.

`development.fast`, `development.standard` and `development.critical` are three points on one scale:
the latter two extend the former, and extension can only make completion harder.

`development.driven` sits beside that scale rather than on it. It extends `development.standard`
with exactly one capability — `command.execute` — and is for runs under `aep drive`, where the
plugin's `driven-surface` hook holds a model's shell to `aep plan artifact …` and `aep observe trace
…`. The grant exists because the planning store's whole vocabulary is CLI verbs and a driven step
that cannot run one cannot create the artifact its transition is guarded on. Do not choose it for
interactive work, and do not choose it under a harness that cannot constrain a shell to a named
surface: the profile is the outer bound and the hook is the inner one, and without the hook there is
only the outer bound. The reasoning, and what it does not claim, is in the document's own header.
