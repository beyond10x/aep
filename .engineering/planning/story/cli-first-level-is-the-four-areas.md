---
format: aep.planning-md/1
id: story:cli-first-level-is-the-four-areas
kind: story
status: active
title: aep --help shows the four areas and doctor; every flat verb stays as a hidden alias
summary: Group the 23 verbs under govern, plan, drive, observe (+ doctor) with hidden flat aliases, a clap-tree test, and grouped spellings in the driver documents and docs.
relations:
- serves: vision:O2
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/edge/aep-cli/src/app.rs
- confidence: cited
  path: crates/edge/aep-cli/tests
- confidence: cited
  path: docs/guide
- confidence: cited
  path: drivers
- confidence: cited
  path: generated/instructions
- confidence: cited
  path: principles
- confidence: cited
  path: profiles
- confidence: cited
  path: website/docs
revision: 4
---
# Story: `aep --help` shows the four areas and `doctor`; every flat verb stays as a hidden alias

## Context

`aep` has 23 top-level verbs. The crates were grouped under `crates/{govern,plan,drive,observe,
profile,edge}` in 0.51.0; the command surface should say the same thing. External call sites of
the flat spellings (2026-09-04): agentplugins 275, metaharness 138, atlas 36, harness 15,
aep-service 5, plus 865 in this repository's own docs, drivers, fixtures and tests; `artifact`
alone is 572 of them.

Target:

```
aep
├── govern     validate · resolve · inspect · evaluate · explain · describe · schema · workflow {render,instruct,flow}
├── plan       artifact {…} · serve · entity {…} · audit · workspace · conformance · reverse {…}
├── drive      run · status · resume · transition · eval {matrix,run}
├── observe    trace {check,inspect,evidence,redact} · contract · property · specification · evidence {scan,inspect}
└── doctor
```

Rules: group names are the area directory names; every existing flat spelling keeps working and
prints the same bytes through a hidden top-level alias; no deprecation line on stdout or stderr
(recorded transcripts, `protocol drive transition` callers and the byte-equivalence fixtures must
see nothing new); `protocol` shares the clap tree so invariant 10 holds by construction. Alias
removal is a later decision, not this story.

## Acceptance

`aep --help` lists exactly five commands; every leaf command is reachable by its grouped path and
by its flat alias with byte-identical stdout, stderr and exit status, asserted by a test that
enumerates the clap tree; `command_equivalence.rs` covers grouped and flat spellings for both
binaries; `task check` exits 0 with `docs-check` regenerating `website/docs/reference/cli.md`;
the driver step maps (`drivers/development/*.yaml`, regenerated `generated/instructions/**`),
`principles/**`, `profiles/**`, README, AGENTS.md and the guides use the grouped spellings.

## Notes

Recorded transcripts and their narration keep flat spellings. The 865 in-repo sites split into
executable documents (step maps, principles, profiles, Taskfile, scripts, tests: grouped) and
prose (grouped). Call sites in other repositories are separate stories after the release.
