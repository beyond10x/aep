---
format: aep.planning-md/1
id: story:native-arm-needs-a-window-that-fits
kind: story
status: draft
title: The native arm needs a model whose window holds a driven state
relations:
- decomposes: epic:cross-harness-portability
revision: 2
---
# Story: The native arm needs a model whose window holds a driven state

## Outcome

The harness comparison is run against a model that can finish a state, so a difference between the
two arms is a difference between the harnesses.

## Context

**Measured 2026-08-29, and it ends the b10x arm as configured.** `protocol drive` reached the native
loop, confined it, gave it exec and a working CLI — and `receive`, the *smallest* state in the map,
died at:

```
400 Bad Request: This model's maximum context length is 32768 tokens.
However, you requested 0 output tokens and your prompt contains at least 32769 input tokens
```

43 tool calls into the first state. `b10x-llmgw` serves one model, `qwen3.8-27b`, with
`max_model_len = 32768`.

For scale: the Claude arm's `specify` spent **92 turns** reading a crate to write a specification of
17 requirements. A driven state accumulates the task document, the story, the state's requirements,
the tools' results and the conversation; 32k is not a small budget for a chat and is not enough for
this.

**Nothing about that is a fact about the harness**, which is what makes it worth recording rather
than fixing in place. The loop published its surface correctly, executed correctly, and refused
nothing it should have admitted. It ran out of window.

**It was foreseeable and was foreseen.** The risk was written down before the first fix — *"32k
context … this model may simply not fit the task, and that would be a finding about the model, not
our harness"* — and then two hours went on plumbing without measuring it. One request sizing the
smallest state's prompt against the window would have closed it in a minute.

## Acceptance

- The comparison runs against a model whose context window holds the largest state of
  `drivers/development/default.yaml`, and the run records which model and which window.
- **Before any paid comparison**, the prompt size of the smallest and largest states is measured
  against the window and written down. A cell that cannot start is not a cell that scored zero.
- Where the two arms cannot share a model, the report says so and compares nothing — a table whose
  arms ran different models measures the models.
- A `400` from a context limit is distinguished, in the run's record, from a wire refusal. Today
  both arrive as `NO_TERMINAL_RECORD` carrying *model wire refused*, which is how it took 43 calls
  and a retry to see.

## Out of Scope

- Making the driven states smaller to fit a 32k model. The states are what the workflow demands, and
  trimming them to suit an endpoint would change what is being measured.
- Anything about `b10x-llmgw`'s model choice. Which model that gateway serves is the endpoint
  owner's decision.

## Open Questions

**Which model?** Decides: whoever holds the eval budget. Default if nobody answers: **the same model
both arms run**, whatever it is — a shared endpoint is what makes the comparison a comparison, and
`--claude-endpoint`/`--claude-model` exist for exactly that.

**Is a driven run's context growth bounded at all?** Decides: driver owner. Default: **unmeasured
and worth measuring** — the Claude arm never hit a limit, which means nothing about where the limit
is.
