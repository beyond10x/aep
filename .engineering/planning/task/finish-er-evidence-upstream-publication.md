---
format: aep.planning-md/3
id: task:finish-er-evidence-upstream-publication
kind: task
status: draft
title: Finish publication of the reviewed ER evidence integration
relations:
- decomposes: story:bind-current-coverage-to-specification-lifecycle
revision: 1
---
The implementation is published at 18a18a3f1cfa110dc5c9a675b3e9956f74c29bb7 and passed full local and remote checks. ER pins that revision and its actual specification is conforming. ER PR47 is merged at 72455539c0756290d03fd5ef28b30512a729c457.

The later documentation-only integration candidate 8913a69427e31bd9e78a29a79137d9aaaad639e3 passed the full local gate and signed common checks, but Gates publication refuses upstream ancestry introduced by PR64. Complete this task only after governed delivery accepts the ancestry, the exact final candidate passes CI, and PR61 is merged. Do not bypass the refusal or rewrite upstream history. The managed archive retains the unpublished integration and this planning record for recovery.
