---
title: Harnesses and confinement
description: How Claude Code, Codex, and the b10x native loop are launched, governed, confined, and evidenced.
---

# Harnesses and confinement

The three harnesses are comparison arms, not interchangeable binaries. Claude Code and Codex own
vendor loops that metaharness drives. `b10x-harness` owns a direct-provider loop that metaharness
observes. AEP owns the workflow and capability decisions; substrate owns process
confinement.

| Property | Claude Code | Codex | b10x-harness |
|---|---|---|---|
| Loop owner | vendor binary | vendor binary | b10x native loop |
| Metaharness adapter | `metaharness run claude` | `metaharness run codex` | `metaharness run b10x` |
| `aep drive` selector | `claude-code` or `metaharness` | not yet selectable | `b10x` |
| Per-call policy | metaharness asks the driver before a mediated call | metaharness asks the driver before a mediated call | observe-only; the loop publishes its admitted tools and runs its own approval gate |
| Workspace presented to the child | metaharness scratch by default; an operator-named tree weakens hermetic claims | metaharness scratch by default; an operator-named tree weakens hermetic claims | operator-named workspace, optionally adopted by substrate |
| Inner process confinement | vendor sandbox facts plus metaharness launch attestation; no substrate envelope in the current CLI path | vendor sandbox facts plus metaharness launch attestation; no substrate envelope in the current CLI path | substrate socket or embedded driver; no substrate means read-only local catalogue and no process tool |
| Process write surface | whatever the vendor launch and metaharness frame jointly admit | whatever the vendor launch and metaharness frame jointly admit | read-only by default; repeat `--process-write-subtree DIR` for exact writable directories |
| Process network | recorded from the launch/vendor evidence available to the adapter | recorded from the launch/vendor evidence available to the adapter | substrate requests and measures no network for confined execution |
| Environment | constructed child environment | constructed child environment | substrate receives a constructed environment for confined execution |
| Resource evidence | adapter and transcript evidence; absence stays unknown | adapter and transcript evidence; absence stays unknown | substrate resource-usage measurement is requested; unavailable evidence stays unavailable |
| Credentials | explicit metaharness custody/copy or loopback declaration | explicit metaharness custody/copy or loopback declaration | caller-named key or token source read by the provider wire; no ambient fallback |
| Agent-visible execution context | task prefix says metaharness-driven, inner harness, workspace mode, hermetic mode, decision mode, and current substrate-envelope status | same | machine-trust context layer says native loop, direct/metaharness launch, substrate mode, exact process write access, network/environment/resource posture |

“No substrate envelope in the current CLI path” is deliberate wording. Metaharness has a real
substrate-backed `ProcessEnvelope` implementation for embedders and tests, but the Claude and Codex
CLI adapters do not silently route through it: their credential and model-proxy apertures still
need an explicit composition. An absent measurement is unknown, never proof that the machine had no
mount, network, environment, or resource surface.

## Local run sequence

From an AEP checkout with sibling `metaharness` and `harness` checkouts:

```shell-session
$ cargo install --locked --path crates/edge/aep-cli
$ cargo install --locked --path ../metaharness/crates/metaharness-cli
$ cargo install --locked --path ../harness/crates/harness-cli
$ aep drive status
```

The free native comparison performs all setup and confinement checks, then stops before a model:

```shell-session
$ cargo run --locked --manifest-path ../metaharness/Cargo.toml \
    -p metaharness-aep-eval -- native \
    --ep-repo "$PWD" --harness-repo ../harness
```

For a governed live run, choose a step map and one of the two selectors `aep drive` currently
supports:

```shell-session
$ METAHARNESS_LIVE=1 aep drive run --project . --map development/default \
    --plugin-dir /path/to/agentplugins/plugins/aep-plan --pause-on-approval \
    --budget-usd 10 --assume-usd-per-run 1
```

Set an `llm` step's `harness: b10x` to select the native arm and supply its explicitly named
endpoint, model, wire, credential and substrate options through the drive surface. There is no paid
default: an LLM-bearing map is refused without the live opt-in and both reservation values.

## Reading the evidence

Keep these claims separate when comparing results:

- A metaharness launch attestation says what the outer observer imposed or could not impose.
- A harness transcript says what the inner loop offered, attempted, denied, spent, and completed.
- A substrate measurement says what the host actually applied to mounts, environment, network, and
  resources.
- A missing field means nobody reported it. It must not be converted into an empty set, zero, or a
  successful confinement claim.

The comparison is useful only while all three layers remain attributable. Prefixing a vendor task
or adding the b10x machine context tells the agent which layer it is inside; it does not grant a
capability or replace the recorded evidence.
