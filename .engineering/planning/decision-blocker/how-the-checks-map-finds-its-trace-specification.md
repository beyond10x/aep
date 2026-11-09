---
format: aep.planning-md/3
id: decision-blocker:how-the-checks-map-finds-its-trace-specification
kind: decision-blocker
status: open
title: Nobody has decided how the checks map's trace step finds its specification outside this repository
relations:
- blocks: story:checks-map-trace-spec-path
revision: 1
---
# Decision blocker: Nobody has decided how the checks map's trace step finds its specification outside this repository

The defect is still present at `824869fec`: `drivers/development/checks.yaml:293-305` passes `--spec conformance/trace/expectations.driven-step.trace.yaml` as a relative path, a `command` step runs in the adopting project (`crates/edge/aep-cli/src/drive.rs:1832`), and `expand` (`drive.rs:141-178`) fills `run_directory`, `task` and `transcript` only.

The story says: "Acceptance: Not yet decided; the fix is one of: a placeholder the map can use for the map's or the protocol root's directory, `aep observe trace evidence` accepting a specification by a name it resolves from the protocol root, or the trace specification shipped beside the map. Each is a design decision for the orchestrator or operator". The map has `llm` steps, so `aep` refuses it before this step; the path is reached only through `metaharness aep drive`, which this repository cannot run, so whichever is chosen is also a contract with that repository.

Cleared by: the orchestrator's or operator's choice among the three, written into the story's acceptance.

Filed 2026-11-09 by the coordinator of `epic:zero-open-issues-2026-11-09`, in a non-interactive run; the story stays `draft` and is not in the epic.
