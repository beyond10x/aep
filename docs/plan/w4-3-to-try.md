# W4-3 — the third governed run, ready to start

> **Status: prepared, not started, 2026-08-28.** Everything the run needs is committed at `0.32.0`.
> It is not started because it spends money and stops at a person — decision **D1** of
> [`next-ten-steps.md`](next-ten-steps.md), whose default is *prepare and stop here*.

## What it is

`story:governed-dogfood-run`'s third attempt, and the **first whose product is Rust** — so the first
that walks `drivers/development/default.yaml` (the cargo map) rather than `development/checks`.

| | |
|---|---|
| story driven | `story:operator-resume-ux`, re-scoped 2026-08-28 |
| task document | `.engineering/task-w4-3.yaml` |
| map | `drivers/development/default.yaml` |
| profile | `development.driven` |
| the work | one real defect — `--take-lock` builds a `StolenLock`, prints it and never persists it, so `protocol drive status` has a printer for a field that is always `None` and **the theft is not in the record** — plus the holder's cursor state in the lock refusal, and three assertions |
| blast radius | `crates/aep-driver/` and `crates/protocol-cli/src/drive.rs` |

The candidate changed on the day: this plan named `story:retry-budgets`, and the audit found it
shipped and closed it. `story:operator-resume-ux` is what now fits `story:governed-dogfood-run`'s own
stated default — *mechanical acceptance, blast radius one crate*.

## What was cleared for it

| was in the way | now |
|---|---|
| the coverage pre-flight refused, naming `contract_result`, `property_test_result`, `verification`, `specification` | names **none**; the run starts with no `--allow-evidence-gap` (`story:evidence-producers-for-the-driven-map`) |
| `protocol drive` refused the store — *cannot be trusted* — over one declared cross-repository edge | the driver reads the workspace manifest the way `validate` does |
| the two previous runs' step maps could not mint what their plans demanded | four producers, each `producer: verifier`, minted by a `command` step the driver ran |

Both earlier runs are recorded and neither reached `complete`: `W4-1/1` blocked in
`establish_verifiers` ($15.42), `W4-2/1` in `adversarial_verify` ($31.46).

## To try

1. Check the pre-flight still passes, free of charge — nothing runs, no model is called:

   ```console
   cd ~/beyond10x/aep
   cargo build -p protocol-cli --bin protocol
   ./target/debug/protocol drive run \
     --task .engineering/task-w4-3.yaml \
     --map drivers/development/default.yaml \
     --pause-on-approval --max-iterations 0
   rm -rf .engineering/runs/W4-3        # the probe's directory
   ```

   Expect `run W4-3/1`, `status budget-exhausted`, `state receive`, and **no** mention of
   `--allow-evidence-gap`.

2. Start the real run. It calls a model per `llm` step and stops at the first thing only a person can
   answer — approving the specification, in `establish_verifiers`:

   ```console
   ./target/debug/protocol drive run \
     --task .engineering/task-w4-3.yaml \
     --map drivers/development/default.yaml \
     --pause-on-approval
   ```

3. Answer the approval, then resume:

   ```console
   ./target/debug/protocol drive status
   ./target/debug/protocol drive resume
   ```

4. Whatever happens is the result. **A run that wedges is a recorded result** — where it stopped,
   what the cursor said, and which decision was wrong. Quietly retrying until it works does not close
   `story:governed-dogfood-run`, and the two runs before this one are on
   [`harness-wave-4-governed-dogfood.md`](harness-wave-4-governed-dogfood.md) precisely because they
   stopped short.

## Known risk, before you spend anything

`protocol specification evidence` discovery: in this store two specifications are approved and
neither relates to the task's `derived_from`, so that step refuses and submits nothing rather than
guessing — correct, and it means the run may block at the specification guard until the run's own
specification is the one in force. The join is `task.artifacts.derived_from` against the
specification's relations; if it blocks there, look at that first rather than at the map.

## After it

Release **0.33.0**, and `epic:reference-driver`'s *Done When* — a real task driven end to end with
its records admitted — is finally answerable either way.
