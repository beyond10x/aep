---
format: aep.planning-md/1
id: story:metaharness-executor
kind: story
status: implemented
title: 'The metaharness executor: one policy, one enforcer'
relations:
- decomposes: epic:metaharness-migration
- supersedes: story:plugin-enforcement-hooks
revision: 6
---
# Story: The metaharness executor — one policy, one enforcer

## Outcome

An operator who drives a step with `harness: metaharness` gets a session whose per-state surface is
enforced by the metaharness seam itself, so a run can no longer look clean while running unenforced
— the failure mode run `W4-2` paid for eight sessions to expose when a resume forgot `--plugin-dir`.

## Context

Accepted directly by the operator, 2026-08-22 ("CONTINUE with meta-harness impl and integration"),
against §§ 10.1–10.3 of the metaharness protocol design
(`beyond10x/metaharness`, `docs/design/metaharness-protocol-v0.1.md`). The integration is over the
**binary seam**: this repository is public and metaharness is not, so no Cargo dependency crosses —
the shared artifact is the sealed `metaharness.frame/1` document (metaharness amendment a5), and
the working directory travels as the a6 `--cwd` declaration. metaharness commits `9dd3e61`,
`edacc3a`, `c27bdef` are the other half.

## Acceptance

- `run_llm` selects a second executor by the existing harness-name seam (§ 4.9 point 3); the
  default `claude-code` path is byte-for-byte unchanged. **Met**: `drive.rs`, gate exit 0.
- The step's surface travels as a sealed frame document whose digest metaharness verifies without
  this repository linking its crates. **Met and cross-verified**: an externally sealed document
  passed resolution against the real binary and failed only on the deliberately missing prompt; a
  corrupted byte was refused naming both digests.
- The operation rendering mirrors `allowed_tools` decision-for-decision, including `subagent.spawn`
  never being offered. **Met**: unit tests beside the `allowed_tools` ones.
- Denials arrive in the event stream the executor writes as the transcript, not in a side-channel
  log a forgotten flag can silence. **Met by construction**: `--decisions frame`; the plugin's
  hooks no-op without the step-context environment metaharness's H3 scrub drops.

## What the frame is pinned by, and what it is not

Added 2026-08-23. The sealed frame document is the one artifact that crosses to metaharness, and
until now nothing compared the two ends of it: both repositories hold the vocabulary and neither
holds the other's code, so a drift in the minter or in the reader stayed silent until a driven run
died at its first step with a refusal and a spent session. The acceptance line above says
*cross-verified* on the strength of one manual check against the real binary on 2026-08-22; a manual
check that nothing re-runs is not a pin.

`crates/protocol-cli/tests/metaharness_frame_contract.rs` transcribes `Frame::parse_document` out of
`metaharness-protocol/src/frame.rs` — tag, then shape, then digest, in that order, with a `Refusal`
per `FrameDocError` variant — and reads the committed golden
`crates/protocol-cli/fixtures/metaharness-frame-canonical.json` with it. No Cargo dependency crosses,
which is the boundary this story was accepted on; the transcription names its source at every rule it
copies.

Pinned, one test per refusal the consumer has a name for:

| consumer refusal | test | the mutation |
|---|---|---|
| accepted | `the_minted_golden_is_accepted_by_the_rules_that_would_refuse_it` | none — the golden as the driver mints it, its digest re-derived from the bytes |
| `UnknownFormat` | `a_frame_document_without_a_tag_it_knows_is_refused_as_untagged` | `format` removed; then set to `metaharness.frame/2` |
| `Invalid` | `a_frame_document_that_is_not_a_frames_shape_is_refused_as_misshapen` | `step.attempt` removed; `workflow` replaced by `3`; an operation outside the closed vocabulary appended |
| `DigestMismatch` | `a_single_flipped_byte_in_a_minted_frame_breaks_the_digest` | one obligation's `red` → `fed`, nothing else touched |
| `DigestMismatch` | `a_frame_document_that_was_never_sealed_is_refused_by_the_same_check` | the digest replaced by sixty-four zeroes |
| `NotJson` | `bytes_that_are_not_a_json_object_are_refused_before_the_tag_is_looked_for` | the document truncated at half its length; and a JSON array |

Pinned about the minter itself: the golden is the bytes `write_frame_document` would put on disk and
not a hand-typed copy (`the_committed_golden_is_the_document_the_driver_would_write`, beside the
minter), two mints agree byte for byte, the tag matches a literal transcribed from `frame.rs` rather
than this crate's own constant, the field set is exactly the struct's eleven plus the tag, and the
operations are strictly ascending by wire name.

Not pinned, deliberately:

- **Unreadable.** A missing file, a partial write, a permission error — an I/O condition of the
  caller's filesystem, not a property of any document, so there is no document a minter could
  produce that has it. Stated rather than tested. The readable-but-wrong end of the same class is
  tested.
- **That the transcription still matches the consumer.** It is a second implementation and agrees
  with `frame.rs` as read on 2026-08-23; nothing here forces it to keep agreeing. **The golden is
  what closes that**, and closing it is a metaharness-side wave: replay those exact bytes through
  the real `Frame::parse_document`, and the two sides then disagree loudly or not at all. This is
  EP's half.
- **serde's normalisation of a nested unknown field.** The minter cannot produce one, and both
  reasons are asserted instead of assumed — its field set is the struct's, and its operation set is
  emitted already sorted and deduplicated.
- **The prompt, the argv and the per-call decisions.** The seam's other half, with tests of their
  own; this section is about the document.

## Out of Scope

- `--decisions ask` with `Engine::authorize` called per call inside the driver — the full § 10.1
  shape, and with it the hooks' per-argument narrowing (one program, two verbs, no pipes). Frame
  mode admits or refuses whole operations; this is stated in the executor's own doc comment.
- Any change to the eval scripts (§ 10.2) or a driven run over the real step maps (§ 10.3).
- A paid live run through the new executor. Nothing here spends money; the cross-check used
  refusal paths only.

## Open Questions

- ~~When the executor moves to `ask` mode, where does the tool-name → `ActionRequest` translation
  live — the executor, or a table beside `allowed_tools`?~~ **Answered 2026-08-22: a table beside
  `allowed_tools`.** `action_for` in `crates/protocol-cli/src/drive.rs` renders one call as the
  action it is, immediately below the function that renders a capability set as tool names — the
  two are the same seam read in opposite directions, and splitting them would put half of adapter
  point 2 in the executor. It decides nothing: which capability an action needs stays
  `Action::required_capability`'s. Two offered tools render to nothing on purpose — `Skill`, which
  takes no action, and `WebSearch`, which names no URL a `NetworkRequest` could honestly carry —
  and for those the engine is not consulted.
