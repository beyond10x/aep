---
format: aep.planning-md/3
id: approval-record:store-pack-stories-accepted
kind: approval-record
status: draft
title: 'Accepted: the store-pack stories, on the operator''s sentence of 2026-11-10'
tags:
- non-interactive
relations:
- decides: story:lifecycle-declares-closed-states
- decides: story:pack-format-and-overlay-read
- decides: story:pack-and-unpack-verbs
- decides: story:validate-checks-packs
- decides: story:pack-end-to-end-and-docs
revision: 1
---
# Approval record: the store-pack stories, accepted on the operator's sentence

## Which stop

Acceptance of the five stories of `epic:store-pack-closed-artifacts` (moving them to the accepted
status the story lifecycle names). Reason `non-interactive`: the coordinating session runs as a
sub-agent; the operator's words reached it through the orchestrating session, and no operator turn
exists inside this run.

## The operator's words, verbatim (2026-11-10)

> you shall take the aep request, its important

That sentence is the acceptance. Nothing else is accepted by it: the finding draft
`story:review-results-stay-active-after-their-subject-closes` is not, and stays `draft`.

## The request it accepts (from the operator's structure review of a downstream group; numbers measured there, not re-verified here)

> A Git-native store (aep.project/5) has no way to pack or prune closed artifacts, and it dominates a
> repository's file count.
> - aep 0.79.0, repository ai-agent-platform/ai-agent-platform at origin/main: `.engineering/planning`
>   1883 markdown files: story 1347 (605 implemented, 302 archived, 5 rejected, 370 draft, 53 active,
>   12 proposed); review-result 460 (217 archived, 243 active). `.engineering/evidence`: 2305 JSON
>   files in 714 story directories, largest record 2631 bytes (one file per evidence record is what
>   `plan store migrate git` produces). Together 4188 of 5860 tracked files (71 %). Across the group's
>   9 repositories: 2612 planning markdown of 3872 markdown files, and 3282 evidence JSON files.
> - `aep plan --help` and `aep plan artifact --help` list no pack, prune, compact or archive-out verb.
>   `move --to archived` keeps the file. `store migrate git` on a /5 store only drops duplicate
>   transitions.
> - Wanted: closed work (implemented, archived, rejected, plus its evidence and archived
>   review-results) out of the working tree, or packed into few files, with `validate`, `history` and
>   `explain` still answering. NOT wanted: deleting records.
> - Second observation, unchecked: in the `ui` repository of that group 68 of 68 review-results are
>   `active` while 154 of 253 stories are implemented. Unknown whether a lifecycle step is missing or
>   the wave flow skips it.

## What was decided in the operator's absence

The coordinating session moves `story:lifecycle-declares-closed-states`,
`story:pack-format-and-overlay-read`, `story:pack-and-unpack-verbs`, `story:validate-checks-packs`
and `story:pack-end-to-end-and-docs` from `draft` through `proposed` to `active`, after the plan-critic
round recorded against them. The store-version decision
(`architecture-decision-record:packed-store-is-aep-project-6`) is not accepted by this record; it
stays `proposed` for the operator.

## What the operator would have been asked

"Accept these five stories as the decomposition of the request?" — answered in advance by the
sentence above.
