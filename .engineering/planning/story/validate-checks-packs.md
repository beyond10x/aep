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
- depends_on: story:lifecycle-declares-closed-states
revision: 2
---
# Story: `validate` holds every pack to the bytes it replaced

## Outcome

`aep plan artifact validate` checks every pack: its format and name, each record's digest, path
safety, that no path is held twice with different bytes, that each packed record equals the last
committed blob at its original path, that a packed artifact is at a closed status of its kind and
packed evidence belongs to the artifact its directory names, that immutability and redaction replay
hold for packed review-results and evidence, that no copy re-exposes a redacted literal, and that the
store version agrees with whether packs exist.

## Context

`architecture-design:store-pack`, section *What `validate` checks about packs*, items 1–8. Git
history reading is `crates/edge/aep-cli/src/planning/git_record.rs` `committed_versions`, whose walk
today ends at a deletion; for a packed path it continues past the commit that moved the file into a
pack. Closed statuses come from `story:lifecycle-declares-closed-states`. Fixtures write their packs
with the pack module of `story:pack-format-and-overlay-read` (its `render`), not with the `pack`
verb, so this story does not wait for `story:pack-and-unpack-verbs`.

## Acceptance

`validate` answers `valid` on a fixture Git store whose closed artifacts and evidence were moved into
a committed pack unchanged, and refuses each corruption below with its own stable code, exit 1,
naming the pack file and the record path where one exists. Checked by CLI tests, one per case:

- a pack that does not parse (bad header, truncated record), and a pack whose file name is not its
  content digest: refused (format);
- a record byte changed in a pack: refused (digest);
- a record changed and its header digest recomputed, the pack renamed to match: refused (provenance:
  differs from the last committed blob at the record's original path before the commit that added
  the pack);
- record paths `planning/../x.md`, `/etc/x`, `evidence/a\b/c.json`: refused (path);
- the same path loose and packed with different bytes, and in two packs with different bytes:
  refused, naming both places; identical bytes in both places: admitted;
- a packed artifact whose status is not closed for its kind, and a packed evidence record whose
  `artifact` is not the one its directory names: refused;
- a pack in an `aep.project/5` store, and an `aep.project/6` store with no pack: refused (version;
  this case names `project.yaml`, there is no record path);
- an evidence file that had two different committed versions while loose (the first committed
  version differs from the last) and was then packed: refused (immutability), as it is today while
  loose — the history walk crosses the pack commit;
- a packed review-result with a redaction record: `valid` (replay over the first committed version
  reproduces the packed text); the same store with a packed or loose copy that still contains the
  redacted literal: refused, and the output does not contain the literal.

## Shared surfaces

- `CHANGELOG.md` (`## [Unreleased]` / `### Added`, one line for the pack checks).

## Out of scope

- Writing packs: `story:pack-and-unpack-verbs`.
