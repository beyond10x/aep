---
format: aep.planning-md/1
id: story:writer-control-socket-path-budget
kind: story
status: draft
title: The writer-control holder exits silently when TMPDIR leaves its socket no room
summary: 45 bytes of TMPDIR, measured; over it the holder dies at custody and says nothing a caller reads
owner: aep
revision: 2
---
## Outcome

`aep plan store writer-control hold` either takes custody or says, in a message a caller can read,
that it cannot — it never exits silently. The 45-byte budget its socket path leaves for `TMPDIR` is
either removed, or stated where a caller meets it and checked where it is violated.

## Why

Measured 2026-09-21 on `integrate/ess-evolution-public-control-20260918` at 051a19346, while
running the repository's own 16-step gate.

`control_directory` (`crates/edge/aep-cli/src/store_command/writer_control.rs:277-289`) puts the
holder's socket at

```
$TMPDIR/aep-writer-control-<31 hex>/hold.sock
```

The name is `format!("aep-writer-control-{}", sha256_hex)` truncated to 51 characters, so the path
after `TMPDIR` is a fixed **62 bytes**. `sun_path` holds 107 characters and a NUL, which leaves
`TMPDIR` **45 bytes**. Bisected on one case, everything else equal:

| `TMPDIR` length | `public_already_idle_custody_applies_retries_and_rebuilds_without_stop_witness` |
| --- | --- |
| 45 | ok, 2.36 s |
| 46 | FAILED, 0.03 s |

What that costs, and why it was not obvious:

- The gate's own `TMPDIR` was 114 bytes (an evidence directory under the wave), so **31 of the 32
  `store_writer_control` cases failed** — the whole lane, `0 passed; 32 failed` in 1.5 s — while
  every other lane in the workspace was green (755 passed). A first correction to 58 bytes,
  chosen as "short", failed identically: 58 is still over 45.
- Parallelism is **not** involved. The same lane at the same default thread count under a 40-byte
  `TMPDIR` is `32 passed; 0 failed` in 172 s.
- The failure surfaces as `store_writer_control.rs:145` — `assert!(line.contains("Writer control
  held"), "{line}")` — with `line` **empty**. The holder prints its first two lines, then exits at
  the moment custody is taken. Its stderr is piped and never read, so whatever it says is dropped
  and the operator sees a bare assertion on an empty string.
- The 2026-09-19 gate passed this lane under `TMPDIR=/var/tmp/aep-validate-v2-fix` (28 bytes) with
  9 cases; the lane now has 32. Nothing about the budget changed — only the chance of meeting it.

- 2026-09-21, the same wave, twice more, in scripts rather than in the gate: `evidence/gate.sh`
  set `TMPDIR` to a 114-byte evidence directory and `evidence/c-rehearsal.sh` set it to `<copy
  root>/tmp`, 56 bytes. In both, every governed hold (apply, rebuild, retry) died at custody with an
  empty line and no message, and each was first read as a product failure or a rehearsal failure.
  Both scripts now derive `TMPDIR` as `$HOME/.cache/<prefix>/<8 hex of the root>` and exit 7 before
  the first hold if that path is over 45 bytes. The point for this story: the failure is
  indistinguishable from a real refusal until someone measures the path length.

## Acceptance

- Red first: a case that runs the holder under a `TMPDIR` over the budget and asserts the refusal
  is a message naming the path and the limit, not an exit. It fails before the change.
- The holder does not exit silently. Whichever way the budget is handled, a caller that cannot bind
  reads why on stderr **and** the failure is distinguishable from a clean exit.
- Either the budget is removed — an abstract socket, a shorter directory name, or a path that does
  not go through `TMPDIR` — or it is documented at `writer-control hold` in
  `website/docs/reference/cli.md` and checked before the first line is printed.
- The repository's own gate does not depend on the caller's `TMPDIR` being short by luck.
