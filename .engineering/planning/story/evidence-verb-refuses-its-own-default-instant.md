---
format: aep.planning-md/1
id: story:evidence-verb-refuses-its-own-default-instant
kind: story
status: draft
title: protocol artifact evidence refuses the instant it defaults to
summary: Without --at the verb stamps now_at_the_edge() as an ISO datetime and instant() reads only YYYY-MM-DD or epoch millis, so every undated evidence record is refused in 0.27.3; the flag that exists to replace move --evidence cannot be used as documented.
relations:
- decomposes: epic:evidence-gated-completion
revision: 3
---
# Story: `protocol artifact evidence` refuses the instant it defaults to

## Outcome

`protocol artifact evidence <id> --kind test_result --source "task check"` records an observation
dated now, as its help says it does.

## Context

Found 2026-08-28 while closing wave F: every `protocol artifact evidence` call without `--at` failed
with *"`2026-08-28T14:54:06Z` is not an instant this build can read"*.

- `crates/protocol-cli/src/planning.rs`, `fn instant`: reads `CivilDate::parse` (`YYYY-MM-DD`) or
  epoch milliseconds, and nothing else.
- The same file, the `evidence` verb: `at.map_or_else(now_at_the_edge, str::to_owned)`, and
  `now_at_the_edge()` renders a full ISO-8601 datetime.

So the default is a value the parser refuses, and the verb the help text calls *"the alternative and
the better one"* to `move --evidence` cannot be used as documented. `--at 2026-08-28` works, which
is how wave F's five records were written.

## Acceptance

- `instant` reads what `now_at_the_edge` writes — a full ISO-8601 UTC datetime, to the second —
  beside the two forms it reads today; `Timestamp::iso_8601` (added in wave F) is the spelling to
  round-trip.
- A CLI test records evidence with no `--at` and the journal entry carries today's instant.
- A CLI test that asserts the *refusal* for an unreadable instant still passes, naming the text.

## Out of Scope

Sub-second precision; a timezone other than UTC.

## Open Questions

None.
