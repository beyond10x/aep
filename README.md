# AEP

The Agentic Engineering Protocol is a typed, portable, machine-executable specification for how
agent-performed engineering work is governed and proven complete.

Prose can tell an agent to write tests first, obtain approval before production changes, and verify
its work. AEP represents those requirements as validated data. The model reasons; a deterministic
engine decides what the recorded facts permit.

`aep` is the canonical command. `protocol` is an exact compatibility alias: retained operations
produce the same standard output, standard error, and exit status through either name.

```console
$ aep explain --task examples/development-passkeys/task.yaml --action production.write
production.write denied
  operation: change production state
  reason:    principle approval-gates rule production-write-requires-approval
  missing:   approval for capability production.write
  state:     receive
```

## What AEP owns

AEP governs artifacts, planning, workflows, evidence, permissions, approvals, audit, and
completion. Its generic planning substrate is shared by two profiles:

- ADP applies AEP to software development: specification, decomposition, design, tests,
  implementation, and review.
- AOP applies AEP to operational planning, controlled change, verification, rollback, and
  incidents.

Other workflows remain named profiles until they establish distinct semantics.

The workspace includes:

| Component | Responsibility |
|---|---|
| `aep-domain`, `adp-domain`, `aop-domain` | protocol and profile vocabularies |
| `aep-engine` | deterministic resolution, evaluation, and transitions |
| `aep-contract` | storage-independent commands and queries |
| `aep-backend-*` | memory, markdown, SQLite, PostgreSQL, entity, and hybrid backends |
| `aep-driver`, `aep-driver-spec` | reference workflow driver and step maps |
| `trace-domain`, `trace-spec` | typed transcript normalization and conformance checking |
| `aep-conformance` | backend contract suites |
| `aep-schema` | standalone schemas for AEP documents |
| `aep-ess-evidence` | optional conversion of a standalone ESS report into AEP evidence |
| `protocol-cli` | the canonical `aep` command and `protocol` alias |

The document trees under `protocols/`, `principles/`, `workflows/`, `profiles/`, `artifacts/`, and
`drivers/` are data. Teams may vendor them and add their own validated definitions.

## Repository boundaries

AEP is intentionally separate from two sibling projects:

- [ESS](https://github.com/beyond10x/ess) specifies, imports, compiles, analyzes, and projects
  executable system descriptions. ESS has no dependency on AEP. It publishes a standalone
  conformance report; the optional `aep-ess-evidence` adapter translates that report without core
  AEP compiling against ESS modeling types.
- [agentplugins](https://github.com/beyond10x/agentplugins) is the curated `beyond10x`
  marketplace. AEP does not bundle harness-specific skills or agents. `aep eval run` and
  `aep drive run` accept explicit plugin directories at their execution boundaries. See
  [Agent plugins](#agent-plugins) for how to install them.

The reference driver is not an LLM orchestration framework. It proves the protocol contract has a
caller. AEP chooses no credentials, model, endpoint, marketplace, or plugin installation.

The workspace depends on [entity-runtime](https://github.com/beyond10x/entity-runtime) for its
IO-free entity kernel and provider foundations. The dependency arrow points from AEP to Entity
Runtime, never the reverse.

## Agent plugins

This repository carries no plugin source and no marketplace manifest, so it is not a marketplace
source. The Claude Code and Codex plugins live in the sibling public repository
[beyond10x/agentplugins](https://github.com/beyond10x/agentplugins), which publishes the `beyond10x`
marketplace. Its install page is <https://beyond10x.github.io/agentplugins/>.

In Claude Code, add the marketplace and install a plugin from it:

```text
/plugin marketplace add beyond10x/agentplugins
/plugin install aep-planning@beyond10x
```

`adp@beyond10x` and `ess-schema@beyond10x` install the same way. In Codex, add the same GitHub
repository as a marketplace from the Plugins surface and select the plugin there. The install page
carries the current plugin list and how to pin a release tag.

Nothing here chooses a plugin for you: `aep eval run --arm plugin` requires an explicit
`--plugin-dir`, and `aep drive run` accepts repeatable `--plugin-dir` values and the
`AEP_DRIVE_PLUGIN_DIR` fallback. Neither guesses a path under this checkout.

## Evidence and completion

AEP treats evidence as recorded facts with provenance rather than assertions in prose:

- Red-before-green can be expressed as an ordering predicate over evidence sequence numbers.
- Independent verification requires a producer other than the author.
- Approval binds to the artifact revision that was reviewed.
- A stale observation becomes unknown again when its horizon expires; it does not become false.
- Capabilities default to deny, and a denial cannot be granted back by a later document.
- Refused transitions change nothing and remain in the audit record.

`aep trace check` turns a normalized agent transcript into a typed conformance report. ESS
conformance can enter the same evidence system only through the optional report adapter.

## Start here

| Goal | Documentation |
|---|---|
| adopt AEP in an existing repository | [`docs/guide/adopting.md`](docs/guide/adopting.md) |
| integrate an agent harness | [`docs/guide/harness.md`](docs/guide/harness.md) |
| install the Claude Code or Codex plugins | [beyond10x/agentplugins](https://beyond10x.github.io/agentplugins/) |
| choose or implement a backend | [`docs/guide/backend.md`](docs/guide/backend.md) |
| understand open vocabulary rules | [`docs/guide/open-vocabulary.md`](docs/guide/open-vocabulary.md) |
| inspect delivered releases | [`docs/status.md`](docs/status.md) |
| inspect accepted and proposed work | [`.engineering/planning/`](.engineering/planning/) |
| understand repository constraints | [`AGENTS.md`](AGENTS.md) |

## Build and verify

The workspace requires Rust 1.85 or newer and [go-task](https://taskfile.dev). The documentation
site additionally requires Node.

```console
task check
```

The gate performs formatting, generated-status, planning-store, audit, version, dependency,
duplicate-guard, changelog-claim, Clippy, test, CLI-documentation, PostgreSQL, rustdoc, schema,
MSRV, and website checks. It invokes no model and spends no money. The named PostgreSQL check uses
only the server explicitly selected through `ENTITY_POSTGRES_URL`; when none is configured it reports
that the integration did not run.

Install both command names from this checkout with:

```console
task install
```

## License

Apache-2.0. See [`LICENSE`](LICENSE).

<!-- b10x-docs:start -->
## Documentation

[AEP documentation](https://beyond10x.github.io/docs/aep/) · [Start](https://beyond10x.github.io/) · [Ecosystem](https://beyond10x.github.io/ecosystem/) · [Impact](https://beyond10x.github.io/changes/) · [Releases](https://beyond10x.github.io/releases/)
<!-- b10x-docs:end -->
