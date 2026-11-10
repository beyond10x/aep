---
format: aep.planning-md/3
id: story:validate-checks-packs
kind: story
status: draft
title: validate holds every pack to the bytes it replaced
relations:
- decomposes: epic:store-pack-closed-artifacts
- informed_by: architecture-design:store-pack
- serves: vision:O2
- depends_on: story:pack-format-and-overlay-read
revision: 1
---
# Story: `validate` holds every pack to the bytes it replaced

## Outcome

`aep plan artifact validate` checks every pack: its format, each record's digest, path safety, that
no path is held twice with different bytes, that each packed record equals the last committed blob
at its original path, that redaction replay and immutability hold for packed review-results and
evidence, that no copy re-exposes a redacted literal, and that the store version agrees with whether
packs exist.

## Context

`architecture-design:store-pack`, section *What `validate` checks about packs*, items 1–8. Git
history reading is `crates/edge/aep-cli/src/planning/git_record.rs` `committed_versions`, whose walk
today ends at a deletion; for a packed path it continues past the commit that moved the file into a
pack.

## Acceptance

Each case on a fixture Git store holding a committed pack; each refusal exits 1 with a stable code
naming the pack and the path. Checked by CLI tests asserting the code.

1. A record byte changed in a pack: refused (digest).
2. A record changed and its header digest recomputed: refused (provenance — differs from the last
   committed blob at its original path).
3. A record path `planning/../x.md`, `/etc/x`, `evidence/a\b/c.json`: refused (path).
4. The same path loose and packed with different bytes, or in two packs with different bytes:
   refused, naming both places; identical bytes: admitted.
5. `aep.project/5` with a pack, `aep.project/6` without one: refused (version).
6. A packed review-result with a redaction record: valid; the same with a packed or loose copy that
   still contains the redacted literal: refused, without printing the literal.
7. A packed evidence file whose bytes differ from its committed version: refused (immutability).
8. The fixture store packed and committed, unmodified: `valid`.

## Out of scope

- Writing packs: `story:pack-and-unpack-verbs`.
