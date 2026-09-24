---
format: aep.planning-md/1
id: story:evidence-is-an-observation-on-a-tree-store
kind: story
status: draft
title: Evidence on a tree store is an observation that neither advances the revision nor forks the artifact
relations:
- decomposes: epic:planning-on-entity-runtime
- serves: vision:O2
revision: 1
---
## Outcome

Evidence on a tree store is an Entity Runtime observation at the artifact's current revision:
it does not advance the revision, and evidence on one branch with a move on another does not
fork the artifact (design § 4.1, review N7).

## Why it is not built yet

Entity Runtime 0.21.0 has `BatchAction::Observe` and `Store::observe`, and an observation leaves
the revision unchanged, but `walk_branches` (`entity-store/src/asynchronous/verify.rs`) counts
every record without a successor as a head, observations included. Measured: an observation on
one branch and a move on another report the artifact as forked after the merge. The Entity
Runtime change that makes only decisions heads is in progress; this story follows its release.

Switching also changes what `move`, `explain` and `history` read, since evidence counts come
from the packed `document` events today.
