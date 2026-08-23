# Workflows

State machines work moves through, one document each. States declare *phases*; principles time their
obligations against phases, which is what lets one principle apply to a development workflow, a
release workflow and an incident workflow without being rewritten.

Validated by `RawWorkflow` → `Workflow`, which rejects unreachable states, dead ends, transitions to
states that do not exist, and rollback declared on an irreversible state.

Each one is also rendered as instructions — the states, what opens each move, and the principles that
time obligations against the phases those states declare — under
[`generated/instructions/`](../generated/instructions/). Those documents are generated; edit the
workflow and run `protocol workflow instruct --out generated/instructions`.
