---
title: Where this stands
sidebar_position: 1
description: The generated release and gate stamp, with links to authoritative planning state.
---

# Where this stands

{/* generated:currency:begin — do not edit; run `cargo xtask status` */}
Current as of the tag `0.53.0` (2026-09-04).

The repository's gate, `task check`, runs **16 steps** — `fmt-check`, `status-check`, `plan-check`, `audit-check`, `version-check`, `dep-check`, `guard-check`, `claim-check`, `clippy`, `test`, `docs-check`, `postgres-check`, `doc-check`, `schema-check`, `msrv` and `website`.
{/* generated:currency:end */}

The stamp above is generated from the annotated tags and `Taskfile.yml`; `cargo xtask status
--check` fails if it drifts. CI provisions the declared toolchains and invokes that same `task
check` entry point, rather than maintaining another list of checks.

Current state is split across sources that can answer it without copying it into this page:

- [Delivered waves](https://github.com/beyond10x/aep/blob/main/docs/status.md) are
  derived from annotated tags.
- [Open gaps](https://github.com/beyond10x/aep/blob/main/docs/plan/gap-register.md)
  name what remains and what closes it.
- [The planning store](https://github.com/beyond10x/aep/tree/main/.engineering/planning)
  carries accepted work and its lifecycle state.
- [The vision](https://github.com/beyond10x/aep/blob/main/docs/VISION.md) carries
  the proposal boundary; a proposal is not a work order.

Run `task check` for the current verification result. The gate prints its own suite and test counts;
this page deliberately does not preserve a second copy.
