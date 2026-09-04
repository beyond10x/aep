---
format: aep.planning-md/1
id: story:resume-names-a-flag-it-does-not-have
kind: story
status: implemented
title: drive resume names --restart, which it does not accept
relations:
- serves: vision:O3
revision: 4
---
## What happens

`drive resume` refuses after a document in the tree changes, and names a flag it does not accept:

```console
$ aep drive resume SYNC-ROUND/1 --root <tree> --pause-on-approval --budget-usd 5
error: the snapshot was written by engine 0.52.0 and this driver links engine 0.53.0; the routes
out are `--restart`, which allocates a new run id and re-observes the evidence, or reverting the
document that moved

$ aep drive resume SYNC-ROUND/1 --root <tree> --pause-on-approval --budget-usd 5 --restart
error: unexpected argument '--restart' found
Usage: protocol drive resume --root <ROOT> --pause-on-approval --budget-usd <USD> <RUN>
```

`drive --help` lists `run`, `status`, `resume`, `transition` and `eval`; none of them takes
`--restart`. The only route is `drive run`, which allocates a new run id — which is what the
message describes, under a name nothing answers to.

Measured 2026-09-04 with `aep` 0.50.0 against a driven round whose workflow document had been
edited between the run and the resume.

## Why it costs more than a wrong word

The reader has just been refused and is being told what to type. A named flag reads as the
supported route, so the next thing they do is type it and get a second refusal — this time a clap
usage error with no explanation, which looks like their mistake rather than the message's. Two
refusals to learn one fact.

The wording also describes the situation as an engine version difference (`0.52.0` against
`0.53.0`) when what moved was a document in the tree. Both halves of the sentence point away from
what happened.

## Shape

- The message names a route the CLI accepts. Either `resume` gains `--restart`, or the message says
  `aep drive run` and what that costs (a new run id, evidence re-observed).
- Where the cause is a changed document rather than a changed engine, say which document.
- A test that every flag named in a refusal is a flag the named command parses. One test covers the
  whole surface and this class of defect does not come back.
