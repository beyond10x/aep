---
format: aep.planning-md/1
id: story:the-plan-in-a-browser
kind: story
status: implemented
title: The plan is a shape, so there is a verb that draws it
relations:
- serves: vision:O2
revision: 8
---
# Story: The plan is a shape, so there is a verb that draws it

## Outcome

`protocol serve` answers a browser with the plan's status columns, one artifact's fields and body,
and the rungs it may take next — and takes a status move back through the same decision
`protocol artifact move` makes.

## Context

Every read verb answers one question at a time into a terminal. A board is a shape, so triage — see
what is where, advance or close what you do not want — was the thing this CLI was worst at, and the
answer was a person reading `artifact board` and typing `artifact move` per row.

**The decision had to be shared, not reimplemented.** `Command::MoveStatus` *applies* a decision and
does not take one — `crates/aep-backend-memory/src/command.rs:306-309` says so: *"A backend that
re-decided it here would be a second protocol, which is the thing the whole arrangement exists to
avoid."* So a server calling `CommandService::execute` directly would have moved artifacts past
every ladder in the store. `move_status` was split instead: `decide_and_move` holds every rule and
prints nothing, and both the verb and the server call it. The store-wide re-validation comes with it,
so a story with no `serves` edge is refused in the browser with the store's own
`[empty_declaration]` finding, exactly as the terminal refuses it.

**A refusal became a value.** `MoveRefusal` carries `legal: BTreeSet<ArtifactStatus>` and now
serialises, so a caller that is not a terminal renders the statuses the ladder *would* have permitted
as buttons rather than parsing a sentence.

## Acceptance

- The board, one artifact's detail, and the rungs it may take next with what each costs, over HTTP.
- A transition from the page goes through the same decision the CLI makes, graph rules included.
- A refusal answers 409 with the typed refusal, including every legal target.
- No new dependency.

## Out of Scope

- **Body editing, evidence recording, creation and relating.** Transitions only.
- **A `--bind` flag.** `127.0.0.1` is hard-coded; widening it is a source change and a review.
- **Rendering markdown.** The body is shown as bytes, which is what `show` prints.

## Not established

**Delivered on a plan rather than on this story**, which was written afterwards so the release had a
subject to record evidence against. The order is the wrong way round and is recorded rather than
tidied away.

**A terminal rung is one click with no warning.** `archived` ends an epic's ladder and no verb undoes
it; during the first session using the page an epic was archived by accident and had to be recovered
from git, which worked only because the click was uncommitted. The page draws a terminal rung exactly
like any other. Open.

## Release-gate socket finding — 2026-08-31

The 0.35.0 release gate repeatedly reported Linux `ConnectionReset` in the read-only socket test.
Half-closing the synthetic client's write side did not make the runner's close behavior portable;
the strict follow-up proved the final failing run reset before any response byte arrived.

The client no longer treats TCP closure as HTTP framing. It reads headers, requires
`Content-Length`, then reads exactly that many body bytes. It therefore returns as soon as one
complete answer arrives and still refuses missing or truncated framing. Ten repeated socket-suite
runs pass locally. The required guard mutation increased the server's declared length by one byte
and the focused board test failed with `UnexpectedEof`, naming the incomplete body. The release gate
remains the independent runner proof.
