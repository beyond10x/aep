# Downstream fixture — `entity-runtime`'s reading of our ladders

Copied verbatim, not adapted. These eleven files are `entity-runtime`'s `examples/aep/*.yaml`: one
entity definition per lifecycle document this repository ships, written by that repository as
phase 1 of its `docs/design/engineering-protocols-adoption-v0.1.md`. They are committed here rather
than read from a sibling checkout so
[`entity_runtime_equivalence.rs`](../../entity_runtime_equivalence.rs) says the same thing on a
machine that has only this repository — the same reasoning their own
`crates/entity-yaml/tests/fixtures/aep-lifecycles/PIN.md` gives for holding a copy of ours.

| | |
|---|---|
| source | `github.com/beyond10x/entity-runtime`, `examples/aep/*.yaml` |
| pinned tag | `0.13.0` |
| pinned commit | `ddee747` — the commit tag `0.13.0` points at, 2026-08-28 |
| last upstream change to these files | `1bfad9f`, 2026-08-25 — `chore(release): 0.5.2`. Unchanged across eight releases since |
| copied | 2026-08-28, for `story:entity-runtime-mapping` |
| licence | Apache-2.0, the same as this repository |

```
ebf56a61e54f6ad5d6c09e209d49bb41fefc33763343e69e707c2a0bb3dac3cf  architecture-decision-record.yaml
46da77df22bade3f9d585962df28cb92315f3cfff91d1eb2f0b0272ccbe01402  blocker.yaml
f0264371d11499ccaf2816f4fc863ad61addc9991714d594cb5b0c0f4cc15326  design.yaml
77bc44c92a68524517eae254b56cae2f6c3af3fbd7c640394185e13adeec2ef6  epic.yaml
357c466174e43c484cf48c62b03c7d7ee5bc3a375fcfda4ee695cd0bfe6f07a5  initiative.yaml
4decdd4769cce5b6c5c4b0bc611b559127026961db0e179ff1f274616618dfc6  obligation.yaml
b535ff52fbdd985dba758c62d8b79bfcc0267a41e108b13a7e2b9cf5f870b667  review-result.yaml
941fc4d1519ed7cd2faf3001171314621c8a57a22dcfbba66dc0670f9f6d209c  specification.yaml
6ae756a540203d70845c7fa5f09a76886c2e06088eef2077119a259eb5ba7c4c  story.yaml
84c9e0f31954eda03fbf93660e9a57f188556c28f54565691cb1cb252ce3f6eb  task.yaml
cbd6ca4c54a0cadecebcc8949b225b2512f254adeaf159179d679a2899f80976  vision.yaml
```

Recompute with `sha256sum *.yaml` in this directory;
`the_pinned_copy_is_the_bytes_this_pin_records` holds the block against the files in both
directions, so a file that changes without its sum changing is refused and so is a sum with no file.

## Which kinds are here, and which are not

Eleven of the twelve ladders under `artifacts/lifecycles/`. **`outbound-claim` is not here**, and
that is the one direction the equivalence deliberately does not close: it landed after the copy
`entity-runtime` holds of us (`bba1a15`, `4d331a0`, both after their pinned `3de6e07`), so no
definition for it exists there yet. `the_only_ladder_without_a_definition_is_the_one_named_here`
names it as a constant, so a *twelfth* ladder growing here without a definition there fails rather
than being silently skipped.

`blocker`, `obligation` and `vision` **are** here: all three are ours and shipped
(`6409587`, `ac30a24`, `57b8d2b`), and `entity-runtime` expresses all three.

## What the pin does not do

It holds the copy honest, not current. It says nothing about whether `entity-runtime`'s `main` still
carries these bytes — answering that means cloning their repository, and no step of `task check`
reaches the network. Their side has the mirror-image gap and solves it with a scheduled workflow
(`entity-runtime/.github/workflows/upstream-pin.yml`); ours is refreshed by hand when the tag in
`Cargo.toml` moves, which is a reviewable line rather than a silent drift.

Refreshing: copy the files again, update the tag, the commit and the sums, and run
`cargo test -p aep-backend-markdown --test entity_runtime_equivalence`. A refresh that makes the
test fail is the point of the fixture — it means one of the two readings moved and the other did
not, which is a fact somebody has to decide about.
