---
format: aep.planning-md/1
id: story:one-cli-verb-surface
kind: story
status: implemented
title: The verbs answer across members, and a refusal says which member refused
summary: list, board, graph and validate over the assembled graph. A refusal that does not name the member is a refusal somebody has to go and reproduce by hand.
relations:
- decomposes: epic:one-cli-many-repositories
- depends_on: story:cross-repo-relations
revision: 4
---
# Story: The verbs answer across members, and a refusal says which member refused

## Outcome

An operator gets one answer for the whole workspace, and when something is wrong they are told
*which repository* it is wrong in — rather than having to reproduce the refusal by hand in each one.

## Context

The assembly is only worth building if the verbs use it. The requirement that makes the difference
between a usable command and a frustrating one is attribution on refusals and on partial results.

## Acceptance

`protocol workspace list` (filterable by `--kind`, `--status`, `--member`), `crossings`, `show` and
`members` all answer over the assembled graph, in text and JSON; every refusal names the member that
refused; **`members` exits `0` with an unresolved member listed as unresolved**, because a command
that failed because a colleague's repository is missing from your disk is a command nobody could
use; and a member that failed to load is reported with its failures rather than skipped.

## Out of Scope

Any write verb across members, and `board`/`graph` rendering over the assembly — `list`,
`crossings`, `show` and `members` are the surface this story delivers.

## Open Questions

Whether `board` should render across members is open; it is a rendering decision, and nobody has
asked for it yet.
