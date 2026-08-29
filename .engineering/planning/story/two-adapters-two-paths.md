---
format: aep.planning-md/1
id: story:two-adapters-two-paths
kind: story
status: draft
title: Two adapters on one wire disagree about the path, and the flag documents one of them
relations:
- decomposes: epic:cross-harness-portability
revision: 2
---
# Story: Two adapters on one wire disagree about the path, and the flag documents one of them

## Outcome

An operator who points a driven run at a gateway reaches it. Today one adapter reaches it and
another gets a 404, and the flag's own help text describes the one that works.

## Context

`metaharness run --model-endpoint <BASE_URL>` is documented as *"a model gateway to point the
harness at, as the gateway's **root** URL (no `/v1`)"*, and states what each adapter does under it:
*"Claude Code speaks Anthropic messages at `{root}/v1/messages`, codex the OpenAI Responses wire at
`{root}/v1/responses`."*

The b10x adapter appends **`/responses`**, not `/v1/responses`. Measured 2026-08-29 against the dev
cluster's `b10x-llmgw`, which serves `/v1/responses`:

| given | reached | result |
|---|---|---|
| `--model-endpoint http://127.0.0.1:18080` | `…/responses` | `model wire refused: openai-responses answered 404 Not Found` |
| `--model-endpoint http://127.0.0.1:18080/v1` | `…/v1/responses` | `session.ended is_error=false`, the model answered |

Confirmed independently: `curl -X POST …:18080/v1/responses` returns 200 and `…/responses` returns
404, so the gateway is unambiguous and the disagreement is entirely between the two adapters.

**Why it cost more than a flag.** The failure arrives as `NO_TERMINAL_RECORD` carrying *model wire
refused*, which is the same shape as an endpoint that is down, an endpoint that is starting, and an
endpoint that is not there — so the first two sessions of a driven run were spent on it while the
cause looked environmental. It sat behind
`story:cold-start-outlives-the-deadline` for half an hour: the endpoint really was cold, and the 404
really was a path, and each looked like the other.

The workaround is to pass a root with `/v1` on it, which the flag's help says not to do. That works
and is exactly the kind of thing nobody remembers a month later.

## Acceptance

- One rule, stated once, and both adapters follow it — whichever rule it is. `{root}` + the
  adapter's own dialect path is the design; what must not survive is two adapters composing
  different prefixes from the same word.
- The flag's help describes what every adapter does, not what one of them does. Today it names
  Claude Code and codex and is silent about b10x, which is how the difference stayed invisible.
- A 404 from a model endpoint says the URL it asked for. *"openai-responses answered 404 Not
  Found"* without the path is a refusal a reader cannot act on — the same message would appear for
  a gateway that is missing, misconfigured or addressed wrongly.
- A pinned vector covers it, so the composition is asserted rather than remembered: given a root,
  each adapter's request URL is what the design says.

## Out of Scope

- Changing what `b10x-llmgw` serves. It is OpenAI-shaped at `/v1/...`, which is the convention the
  wire is named after.
- Anything about the cold start. Different story, and they only look alike from the outside.

## Open Questions

**Which prefix is right?** Decides: whoever owns the model adapter design. Default if nobody
answers: **`{root}/v1/responses`** — it matches the codex adapter, the flag's documented behaviour
and the convention every OpenAI-shaped gateway follows, and it makes the b10x adapter the one that
changes.

**Should a root ending in `/v1` be accepted anyway?** Decides: same owner. Default: **yes, and
normalised** — an operator who pastes a URL that already works with `curl` should not be punished
for it, and a doubled `/v1/v1` is a refusal that names itself.
