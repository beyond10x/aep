---
format: aep.planning-md/1
id: story:an-llm-step-cannot-declare-its-tools
kind: story
status: draft
title: An llm step cannot declare the tools it needs
relations:
- serves: vision:O3
revision: 1
---
## What happens

A driven `llm` step prints, before it runs:

```text
note: state `extract` admits tools the session was not offered — Edit, Glob, Grep, NotebookEdit,
Read, Skill, Write. The step map and this harness disagree about the tool set; the model will be
refused by the vendor rather than by the policy, and the turn is spent either way.
note: state `extract` put 6 tool call(s) to the driver and 1 were refused.
```

The step declares what it may touch, and nothing else:

```yaml
- kind: llm
  skills: [extractor]
  context: [raw/delta.json]
  scope:
    - paths: ["**"]
      write: denied
```

There is no key for *which tools this step needs*. `scope` bounds paths, `skills` names guidance,
`context` names files; the tool set is the harness's, and the driver can only notice afterwards
that the two disagree. Measured 2026-09-04 with `aep` 0.50.0.

## Why the note is not enough

The step in question reads one declared context file. `Read` is the tool it cannot work without,
and `Read` is in the list of tools it was *not* offered — so the note is reporting that the session
was denied the one capability its own `context:` implies. It ran anyway, and one of its six tool
calls was refused.

A note is the right severity for a mismatch nobody can fix. It is the wrong one when the map has no
way to state the requirement: the reader is told the two disagree and given nothing to change.

## Shape

- An `llm` step declares the tools it needs, and the driver refuses the run when the harness cannot
  offer them — before the turn is spent, not after.
- `context:` implies read access to what it names. A step that declares a context file and is not
  offered a way to read it is a contradiction the loader can catch.
- Where a harness offers more than the step declares, the extra is denied rather than noted: a step
  map that cannot narrow the tool set is not bounding anything, and `scope: write: denied` reads as
  if it were.
