---
format: aep.planning-md/1
id: task:aep-pin-verify-once-vector
kind: task
status: implemented
title: Pin AEP to the verify-once Eventlog provider and the runtime that carries it
summary: 'ten rev values: eventlog-* to db608cd, entity-* to e535aded; Cargo.lock only'
owner: aep
relations:
- decomposes: story:eventlog-planning-authority-migration
- serves: vision:O2
revision: 5
---
## Done when

The AEP workspace builds and its 16-step gate passes against the qualified vector this wave
produced: Eventlog at `db608cd4152e6a2370e8ef0e3a562d7a829cf34a` (replacing
`f802eb8b01b44ba04a93394b20f0c07391f7757a`) and Entity Runtime at
`e535aded9970d41ba16349e9eccf110df29d08c3` (replacing `8b1757365f628338cf697f53deb9ce76acd25a99`),
with no source change and no dependency change other than the revs and the lines `Cargo.lock`
rewrites for them.

## Why

The AEP CLI issues about one Eventlog transaction per artifact per command, and the v1 file
provider re-verified the whole store on each one: one `plan artifact list` over the 64-artifact
migrated planning store made 218 transactions, 227,452 blob opens, read 2,863 MB and took 79.7 s.
`story:file-eventlog-verifies-once-per-open` fixes that in the provider, and
`task:eventlog-provider-pin-verify-once` carries it into Entity Runtime. AEP pins both directly, so
neither reaches the qualified binary until this task moves its ten `rev` values.

Both revs exist on no remote. Cargo resolves them from `~/.cargo/git/db/eventlog-3661f9771fa6366c`
and `~/.cargo/git/db/entity-runtime-98446cf7d1f27fe1` / `-fb33c5846d594547`, into which they have
been fetched; every cargo invocation here runs `--offline`.

## Acceptance

- Every `rev` in the workspace naming `github.com/beyond10x/eventlog` reads
  `db608cd4152e6a2370e8ef0e3a562d7a829cf34a`, and every `rev` naming
  `github.com/beyond10x/entity-runtime` reads `97d6edfb5cccdadbe142399dc31da7d5d851a529`. Counted,
  not read: no other commit of either URL survives anywhere in the workspace or in `Cargo.lock`.
- **The Entity Runtime commit is `97d6edfb`, not `e535aded`.** Independent pass 1 found this
  acceptance still naming `e535aded`, which pins eventlog at `c698923`: satisfying it literally
  recreates the two-`eventlog-core` state the bullet above forbids, because AEP depends on
  `entity-*` and on `eventlog-*` directly and the `entity-*` crates depend on `eventlog-*` directly
  too. `97d6edfb` is `e535aded` plus the second ER pin bump to `db608cd`. The code was right and
  this record was stale (review-result:aep-pin-vector-review-1, N3).
- `Cargo.lock`: only the source lines of the eventlog and entity packages change. Any other package
  cargo rewrites is reverted, and `cargo metadata --locked --offline` still exits 0 under the
  default toolchain and 1.91.0.
- `evidence/gate.sh` on the integration tree: **16 of 16 steps exit 0**, with a disposable
  PostgreSQL, per-step exit codes recorded, and `TMPDIR` inside the 45-byte writer-control socket
  budget (story:writer-control-socket-path-budget).
- No source file changes. The CHANGELOG carries one Unreleased entry naming both revs and what they
  change for a caller of the AEP CLI. **Any number it quotes is a number measured after the pin**,
  not one carried over from the v1 provider's cost (same review, N2).
- One independent verification pass over the pin diff, covering both repositories.

## Scope

cited: Cargo.toml, Cargo.lock, CHANGELOG.md.
