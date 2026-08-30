---
format: aep.planning-md/1
id: story:unreadable-lock-refuses-its-own-escape-hatch
kind: story
status: draft
title: A lock file nobody can read refuses the command that exists to remove it
relations:
- decomposes: epic:reference-driver
- informed_by: story:protocol-drive-verb
revision: 1
---
# Story: A lock file nobody can read refuses the command that exists to remove it

## Outcome

An operator who finds `protocol drive` refused by a damaged lock file runs the command the refusal
names and gets their repository back — instead of being handed the same parse error by every verb
they try, including the one documented as the way out.

## Context

Found by the adversarial pass on `story:protocol-drive-verb`, 2026-08-30, by writing an empty
`lock.json` and running three verbs. All three answer identically:

```
error: reading .../.engineering/runs/lock.json: EOF while parsing a value at line 1 column 0
```

`run`, `run --take-lock` **and** `status`. `read_lock` (`crates/protocol-cli/src/drive.rs:1418`)
propagates the serde error with `?`, and `take_lock` (`:1444-1452`) calls it **before** it consults
`force` — so `--take-lock`, the route the refusal itself advertises, is refused by the thing it
exists to override. There is no verb that clears the state and nothing tells the operator to delete
the file.

**This repository has already solved this exact problem once, for the neighbouring file.**
`a_holder_cursor_that_will_not_parse_is_a_refusal_and_never_a_crash` holds it for `cursor.json`. The
lock is the file *every* verb reads, and it got the weaker treatment.

**How the unreadable file gets there without anybody doing anything wrong.** `take_lock` calls
`create_new` and then `write_all(body)?`. A failure between the two leaves a zero-byte `lock.json`
and **no `HeldLock` value in existence**, so the `Drop` guard added under `story:protocol-drive-verb`
cannot fire. The verb writes the file that wedges it. That half is fixed under that story; this one
is the half that makes the residue unrecoverable.

## Acceptance

- An unreadable `lock.json` — empty, truncated, or not JSON — produces a **refusal that names the
  file and says how to clear it**, not a serde diagnostic. Asserted for `run`, `run --take-lock` and
  `status` separately, because all three read it and all three are wedged today.
- `--take-lock` consults `force` **before** parsing, so a damaged lock cannot refuse the flag whose
  purpose is to override a lock.
- `status` reports a damaged lock as damaged and still prints everything it can read, rather than
  failing whole.
- The red case that exists is
  `adversary_a_lock_file_that_will_not_parse_is_a_refusal_and_never_a_parse_error`
  (`crates/protocol-cli/tests/drive_cli.rs:2621`), delivered red on purpose by the wave of
  2026-08-30 and merged in that state.

## Out of Scope

The lock's *scope* — whether it is per store or per project — which is `decision-blocker:store-lock-scope`.
A damaged lock is wedging whatever scope it turns out to have.

## Why It Is Not In `story:protocol-drive-verb`

It was found in that unit's final correction round and touches three call sites the story does not
otherwise open. Folding it in would have closed a story on a surface nobody planned to review.
