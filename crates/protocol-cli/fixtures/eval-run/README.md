# Streams the eval runner is proven against, and the matrix they assemble into

Three `metaharness.event/1` event streams and the two renderings of the matrix
`crates/protocol-cli/tests/eval_dry_run.rs` assembles from them. Nothing here was recorded from a
live run, and the rest of this file is what that means.

| file | what it is |
|---|---|
| `claude-plugin-attested.jsonl` | arm **b** on Claude Code: an observe-mode session with the injected plugin attested in `session.started`, **source and digest** |
| `claude-driven-attested.jsonl` | arm **c** on Claude Code: a step walk, the same plugin attested, written by `protocol drive run` rather than by this verb |
| `codex-plugin-attested.jsonl` | arm **b** on Codex, on a wire whose terminal event prices nothing |
| `dry-run.matrix.json`, `dry-run.matrix.txt` | what those three plus one corpus transcript assemble into, asserted byte for byte |

Arm **a** needs no file here: the eval corpus's own
`conformance/eval/development-tests-after-the-code/transcript.jsonl` attests no plugin, which is
exactly what arm a is, and it is ingested unchanged.

## Crossing #4, and which half of it this is

`--plugin-dir` copies a plugin into metaharness's hermetic scratch home and attests what it
installed. `session.started` carries **two** plugin lists and they answer different questions:

| row | whose | written when |
|---|---|---|
| top-level `plugins` | the **vendor's** own init list, echoed | Claude Code writes one; Codex writes `null`, because its vendor states nothing and metaharness will not mint a field it did not receive (a9 discipline, the rule that leaves `thinking_tokens` null rather than zero) |
| `hermetic.installed_plugins` | the **instrument's** record of what *it* injected | always, on every adapter |

Crossing #4 is the second row, and it is the one this repository reads:

```json
"hermetic":{"decisions":"observe",
            "installed_plugins":[{"name":"aep",
                                  "source":"aep@integrations/claude-code",
                                  "installed_at":"/plugins/aep",
                                  "loaded_by":"metaharness",
                                  "digest":"7258e0…"}]}
```

**This reader read the vendor echo until the first live pilot run**, where it cost two refusals on a
Codex arm-a run that was perfectly well formed — `plugins: null` beside `installed_plugins: []`. The
boundary refusing to guess was right; the field it was reading was wrong. What makes the instrument's
row correct is not that it is populated more often: the question a manifest asks is *what was this
run given*, and only one of the two rows answers it from something that knows.
`the_digest_is_read_from_the_instruments_row_and_not_from_the_vendors_echo` asserts it by taking the
vendor echo away entirely and requiring the manifest to be unchanged.

`protocol eval run` writes that digest into the run manifest's `plugin_digest` **verbatim** — it
does not hash the directory on disk, because a digest computed here would attest bytes the session
never saw, and an edited plugin would then be indistinguishable from the shipped one.
`the_plugin_digest_in_the_manifest_is_the_one_the_session_attested_byte_for_byte` reads the expected
value out of the fixture rather than repeating it, so the two cannot drift apart.

The other half of this crossing is metaharness's `c1-plugin-injection` conformance vector, which
produces the attestation these files reproduce. **Until that side replays these exact bytes, this is
one implementation agreeing with a transcription of another** — the same posture, and the same
sentence, as `crates/protocol-cli/fixtures/metaharness-frame-canonical.json` and the frame contract
test beside it. Vocabulary crosses that boundary; a dependency never does.

Two refusals are the load-bearing half of reading it, and both are about the experiment rather than
about a document:

* arm `plugin` over a stream attesting **no** plugin — the treated arm without its treatment
  (`EVAL-STREAM-006`);
* arm `raw` over a stream attesting **one** — the control arm with the treatment (`EVAL-STREAM-007`).

A plugin attested with no `digest` is refused too (`EVAL-STREAM-008`), which is why the corpus's own
`development-honest/transcript.jsonl` cannot stand in for arm b: it predates the attestation and
names a plugin without saying which bytes it was.

## How the three streams were derived, exactly

All three come from `conformance/eval/development-honest/transcript.jsonl`, and the derivation is
mechanical:

| stream | derivation |
|---|---|
| `claude-plugin-attested.jsonl` | `step.entered` and `step.left` removed and `seq` renumbered from 1 — an observe-mode arm-a/b session is not a step walk and mints no frame — `run` and `session_id` changed, `hermetic.decisions: observe`, and `hermetic.installed_plugins` given one entry mirroring the vendor echo plus a `digest` |
| `claude-driven-attested.jsonl` | the step events **kept**, `run` and `session_id` changed, `hermetic.decisions: ask` (which is what `protocol drive` passes), and the same instrument row |
| `codex-plugin-attested.jsonl` | as `claude-plugin-attested.jsonl`, then `adapter`, `harness_version` and the offered surface changed to Codex's; **`model: null`**, which is what the live pilot run showed Codex's wire actually states; and the a9 keys that wire does not fill written as explicit `null`: `total_cost_usd`, `thinking_tokens`, `iterations`, `speed`, `service_tier`, `cost_usd` |

The two corpus transcripts the runner ingests gained the instrument's row too:
`development-tests-after-the-code` an empty one (nothing was injected, which is what arm a is), and
`development-honest` one mirroring its vendor echo — **without** a digest, because that recording
predates the attestation. That is not a gap to repair: it is the honest fixture for
`EVAL-STREAM-008`, a plugin whose bytes nobody can name.

The two plugin digests are the ones `crates/protocol-cli/fixtures/eval-matrix/runs/` already uses for
the Claude Code and Codex plugins, so the two fixture sets name the same two plugins.

**Nothing here is invented.** `digest` is written because R2.6 specifies it, and every other added
key — `hermetic.installed_plugins`, `hermetic.decisions`, `model: null` — is copied from the shape of
the first live pilot run, which is the only reason they are here at all: before it, this file said
observe mode's own attestation was *not* written into these fixtures because the spelling had never
been seen and a fabricated key would be a crossing nobody agreed to. It has now been seen. Nothing in
the manifest assembler reads `hermetic.decisions`, so it is fidelity rather than function.

## What a failure here means

A change in this repository's code or documents, and never a finding about a model. These are
**structurally faithful and not observed**: every number in them is a number this fixture set chose,
on the same reasoning `crates/trace-spec/tests/fixtures/`' module documentation gives at length.

When a paid sweep produces real streams they replace these **in place** — same directory, same verb,
same flags. `dry-run.matrix.json` is regenerated with

```console
protocol eval run --case <case> --arm <arm> --harness <harness> \
    --stream <stream> --observed-at 2026-08-23 --out <dir> --redact
protocol eval matrix <dir> --format json > crates/protocol-cli/fixtures/eval-run/dry-run.matrix.json
protocol eval matrix <dir>                > crates/protocol-cli/fixtures/eval-run/dry-run.matrix.txt
```

once per run, in the order `REPLAYS` declares in `crates/protocol-cli/tests/eval_dry_run.rs`.
