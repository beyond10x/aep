# Declared scope, prompt cache, and what arm `native` actually proves — 2026-08-24

> **Repositories touched:** `aep`, `harness`, `metaharness`.
> **Status:** four live runs of `eval-case/development-default` on arm `native`, all against
> `https://chatgpt.com/backend-api/codex`, model `gpt-5.6-sol`, rate card of 2026-08-24.
> **Relationship to design:** this is the measurement § 9 of
> [`workflow-declared-context-and-write-scope-v0.1`](../design/workflow-declared-context-and-write-scope-v0.1.md)
> asked for, plus two defects it found on the way.

## 1. The four runs

| run | scope | context | compaction | held / violated | turns | calls | refusals | cache | cost |
|---|---|---|---|---|---|---|---|---|---|
| 1 | none | none | as shipped | 10 / 1 | 24 | 52 | 4 | 89% | $0.6572 |
| 2 | stated | `SKILL.md` | as shipped | **11 / 0** | 24 | 43 | 0 | 78% | $1.1853 |
| 3 | **silent** | `SKILL.md` | as shipped | 10 / 1 † | 40 | 63 | **5** | 82% | $1.4818 |
| 4 | stated | `SKILL.md` | **fixed** | **11 / 0** | 28 | 49 | 2 | 86% | **$0.9877** |

† run 3's single contradiction is `terminal-record-clean`: it reached `--max-turns 40` and stopped.
`no-artifact-file-was-rewritten-whole` **held** — see § 3, this is the important row in the table.

Transcript digests, for anyone re-scoring: run 1 `sha256:082ddb08…`, run 2 `sha256:56b9654b…`,
run 3 `sha256:f8e606ab…`, run 4 `sha256:4e8a1932…`. Spec `sha256:44d8f5a3…` throughout.

`claude+plugin` on the same case, for reference: 11 / 0, ~19 turns, $0.5216.

## 2. What the scope bought

Run 1 contradicted `no-artifact-file-was-rewritten-whole` by writing three artifact files whole. It
also spent ten of fifty-two calls discovering the CLI.

Declaring the scope and the context in `drivers/development/default.yaml` closed both:

| | run 1 | run 4 |
|---|---|---|
| `file.write` under `.engineering/planning/**` | 3 | **0** |
| discovery calls (`--help`, `kinds`, `relations`) | 10 | 3 |
| held | 10 | **11** |

The preloaded `SKILL.md` costs about **$0.02 per run** — 2k tokens replayed over ~24 turns, almost
all of it cache-read at $0.40/Mtok. The design's § 6 worry about a "permanent tax" was overstated by
roughly twenty times, and the bound it argued for (named files, never a glob) is worth keeping for
reproducibility rather than for cost.

## 3. Denied, not merely avoided

Run 2 satisfied the store rule with **zero refusals**: the instruction stated the scope, so the model
never attempted a whole-file write. That is the cheap outcome and the right default, but on its own it
proves the *prose* worked, which is exactly what `claude+plugin`'s 157-line skill already did.

Run 3 is the control. `--scope-announce silent` binds the tools to the scope and says nothing about it
in the instruction. The result:

* **5 refusals fired**, and every one of them came from the toolset, not from a seam — arm `native`
  runs `Seam::None` and adjudicates nothing.
* `no-artifact-file-was-rewritten-whole` **held anyway**. The model was stopped, not warned.

That is the claim arm `native` exists to make: *the published toolset is the policy*. It is also
measurably more expensive — run 3 spent 63 calls and never finished inside 40 turns — which is why a
real run states its scope. Run 4, with the rule stated, still logged 2 refusals: stating it makes the
model mostly right, and the tool is what makes it always right.

## 4. Two defects the runs found

### 4.1 Compaction could not reach its target, so it fired again and again

`harness-loop` elides old tool-result payloads when the conversation passes 192 kB. Compaction
rewrites the prefix, and the prefix is what the prompt cache is keyed on: **the turn after a
compaction pays full rate for everything.**

The floor was a count — the newest six results were never elided. Six results can be 130 kB. So:

| run 3 compaction | freed | left | bound |
|---|---|---|---|
| 1 | 111,586 | 96,648 | 196,608 |
| 2 | 45,860 | **177,915** | 196,608 |
| 3 | 14,801 | **196,331** | 196,608 |
| 4 | 70,192 | 128,524 | 196,608 |

Compactions 2 and 3 left the conversation a few hundred bytes under the bound, so the next result
crossed it again — turns 38 and 39 compacted back to back, each one a full uncached replay. In run 2
the same mechanism cost 43,203 and 58,448 uncached tokens on two turns: about **$0.39 of a $1.19 run**.

Two changes:

* `KEPT_TOOL_RESULTS: usize = 6` became `KEPT_RESULT_BYTES: usize = 48 * 1024`. A count cannot bound
  bytes when one result can be 64 kB; a size can, so the floor can never sit above the target.
* `COMPACTED_TARGET_BYTES: usize = 96 * 1024` — compaction now elides to a low-water mark rather than
  stopping the moment it fits, so the rewrite is rare and deep instead of frequent and shallow.

Run 4: **one** compaction, which freed 101,238 bytes and left 98,641. Cost fell 17% and the cache
recovered from 78% to 86%.

### 4.2 A confined read had no ceiling at all

`ConfinedOperations::file_read` ignored `max_bytes` and always answered `truncated: false`. The note
above it said a truncation on that side "could not be reported as one" — which was wrong: the whole
text is in hand, so the total and the fact of truncation are both known.

Run 2, turn 10 read three files; turn 11's replay grew by **24,630 tokens**, which is what pushed the
conversation past its bound in the first place. The confined provider now bounds at 64 kB — the same
figure the unconfined one uses — and reports the real size and `truncated: true`.

## 5. What is still open

* The step map's `scope:` and `context:` were **hand-translated** into `--write-scope` and `--context`
  flags for these runs. Until something compiles them, the declaration in
  `drivers/development/default.yaml` is documentation rather than the source of truth.
* `store_integrity` in `crates/protocol-cli/src/drive.rs` still holds the path half of the same rule,
  in Claude Code's tool names. Two copies of one rule disagree the first time one moves.
* `no-artifact-file-was-rewritten-whole` now passes by construction on this arm. A control that
  cannot fail has stopped testing anything; it needs a sibling that asserts the **denial**.
* Vendor arms carry no scope at all. `Frame.subjects` exists and is sealed; nothing compiles the step
  map into it yet, so `claude+plugin` is still bounded only by prose it happens to load.
* Run 3 reached `--max-turns 40` without finishing, under the unfixed compaction. Whether it finishes
  with the fix is unmeasured.
* A mid-stream transport drop ends a run: one attempt at run 4 died on turn 2 with
  `reading the event stream: request or response body error`, and the loop does not retry once text
  has been emitted. Cost of that attempt: $0.01. Re-running was the whole remedy.

## 6. How to reproduce

```
metaharness run b10x \
  --model-endpoint https://chatgpt.com/backend-api/codex --model gpt-5.6-sol \
  --credentials api-key --prices rates.json --cwd <ws_*> \
  --substrate-embedded --cgroup-root /sys/fs/cgroup<delegated slice> \
  --toolchain rust \
  --allow-program /usr/bin/cargo --allow-program /usr/bin/git \
  --allow-program /workspace/target/debug/protocol \
  --context <ws>/integrations/claude-code/skills/planning/SKILL.md \
  --write-scope '.engineering/planning/**=partial-only' \
  --write-scope 'crates/**=allowed' --write-scope 'docs/**=allowed' \
  --write-scope 'conformance/**=allowed' --write-scope 'drivers/**=allowed' \
  --write-scope '**=denied' \
  --max-turns 40 -p "$(cat task.txt)"
```

Two things are easy to get wrong. The cgroup root must be a **delegated** subtree — outside
`systemd-run --user --scope --property="Delegate=cpu memory pids"` substrate reports no exec facts,
no `run` entry is published, and the run silently cannot build or test. And `--scope-announce silent`
is an experiment control, not a mode: a real run states its scope, because being refused costs a call.

Scoring:

```
protocol trace check \
  --spec conformance/eval/development-honest/expectations.trace.yaml \
  --transcript <run>.jsonl
```
