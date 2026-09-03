---
format: aep.planning-md/1
id: story:redaction-removes-the-operator-not-two-variables
kind: story
status: implemented
title: What `--redact` removes is the operator, not two environment variables
relations:
- decomposes: epic:self-evaluation
- serves: vision:O3
revision: 4
---
# Story: what `--redact` removes is the operator, not two environment variables

## Outcome

A stream a `--redact` run wrote carries no spelling of the person who ran it, and a stream an older
build wrote can be finished without re-recording it.

## Context

`--redact` read `$HOME`, `$USER` and `$LOGNAME`. That is the operator as the shell knows them, and it
is not what a stream carries when a recorded run commits inside its own fixture: `git log` prints an
author, and an author is a person's real name and their address.

The golden-path recording of 2026-09-03 went to disk redacted and still carried the operator's name
four times — twice as a commit author in `git log` output, and twice inside a
`git -c user.name="…"` call the agent made after reading the value out of `git config`. Neither
spelling of `$HOME` reaches either, and neither does `$USER`.

The recording had already cost $22.53 when this was found, so a fix that only helped the next run
would have meant paying again.

## Acceptance

- `user.name` and `user.email` are read the way the child would read them, and removed with the same
  word-boundary rule the user name uses.
- A machine with no git, or a directory that is not a repository, has nothing to remove and its
  stream is byte-identical.
- `protocol trace redact --transcript <path> [--out <path>]` applies the removal to a stream on disk.
- It is idempotent, so a stream an older `--redact` wrote can be finished in place.
- It re-digests nothing: a manifest's `transcript_digest` names the bytes its own run wrote.

## Out of Scope

- Removing a name that is not the operator's. A transcript quoting a third party is a different
  problem, and one this cannot solve by reading the environment.
