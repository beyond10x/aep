# Passkeys, planned in the repository

The same work as [`examples/development-passkeys`](../development-passkeys), planned the other way
round. This one keeps its plan **here**, as markdown under `.engineering/planning/`:

```text
.engineering/
├── project.yaml
└── planning/
    ├── initiative/passwordless-authentication.md
    ├── epic/passkey-sign-in.md
    ├── story/passkey-registration.md
    ├── story/passkey-login.md
    ├── story/passkey-recovery.md
    ├── task/webauthn-ceremony.md
    └── task/assertion-verification.md
```

One artifact per file, YAML frontmatter the tooling reads, and a markdown body it never interprets.

## Try it

```console
protocol artifact list     --store .engineering/planning --root ../..
protocol artifact board    --store .engineering/planning --root ../..
protocol artifact graph    --store .engineering/planning --root ../.. | dot -Tsvg > plan.svg
protocol artifact validate --store .engineering/planning --root ../..
```

`--root` points at the document tree the lifecycles come from — this repository. Inside a project
whose `project.yaml` names its own protocol tree, neither flag is needed: `--store` defaults to
`<project>/.engineering/planning`.

The entity surface reads this store as readily as it reads a manifest:

```console
protocol entity list --planning .engineering/planning
```

## The same plan, kept in SQLite

`.engineering/project.sqlite.yaml` is this project with one line changed:

```yaml
store:
  sqlite: plan.sqlite3
```

Copy it over `project.yaml` and every verb above opens `.engineering/plan.sqlite3` instead of the
documents — same commands, same output, same history. The store is a line in the project file, not a
different tool; `docs/guide/backend.md` § *Choosing the store* has the three forms.

## The same plan, kept twice

`.engineering/project.hybrid.yaml` keeps the markdown **and** a SQLite replica, under the four words
a hybrid store cannot work without:

```yaml
store:
  hybrid:
    authority: local
    read: local-first
    on_unreachable: refuse
    on_divergence: record
    local: markdown
    replica: { sqlite: replica.sqlite3 }
```

Every verb writes both. When the replica would not take a write, `protocol artifact divergences`
says so and which side is authoritative; `protocol artifact catch-up` replays it. The record lives in
`planning/divergences.jsonl` between commands.

## The contrast with `development-passkeys` is the point

`examples/development-passkeys` keeps the *same* stories in Linear and points at them from
`artifacts.yaml`:

```yaml
- id: story:AUTH-141
  kind: story
  location:
    provider: linear
    reference: AUTH-141
```

Neither example is the recommended one. They are the two arrangements the protocol supports, and
they are both here so that neither reads as an accident:

| | `development-passkeys` | `planning-passkeys` |
|---|---|---|
| where the plan lives | Linear | this repository |
| how AEP sees it | `artifacts.yaml`, `location: {provider, reference}` | `.engineering/planning/*.md`, `location: <path>` |
| what moves a status | Linear's UI | `protocol artifact move`, refused against the kind's lifecycle |
| history | Linear's | `git log` |
| what it costs | AEP cannot check the plan's own contents | the plan is one more thing in the repository to review |

The graph is the same shape either way, which is what
[`ArtifactLocation`](../../crates/aep-domain/src/artifact.rs) exists to make true: location is
metadata, and only the graph is normative. A team can start in one arrangement and move to the
other without the protocol noticing.

## What this fixture is used by

`crates/protocol-cli/tests/planning_cli.rs` drives the real binary against it: that the store
validates clean, that `list --format json` is byte-identical across two runs, and that
`protocol entity list --planning` counts what is here. `crates/protocol-cli/tests/store_selection.rs`
seeds the seven artifacts into a markdown copy, into the SQLite variant and into the hybrid variant,
runs every `protocol artifact` verb over all three, each as its own process, asserting the output is
the same — and makes the hybrid's replica refuse a write, to list the divergence and catch it up.
