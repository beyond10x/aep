---
format: aep.planning-md/2
id: story:workspace-index-derived-from-member-logs
kind: story
status: draft
title: A workspace index answers across repositories and names the commit each fact was read at
revision: 1
---
## Problem

Links between planning artifacts in different repositories have no query surface: each
repository's store is read alone, and `plan workspace` walks each member store in turn. Moving the
authority into a shared database would make cross-repository queries easy but would lose what the
Git-held store gives: each artifact state is pinned to a commit, branches carry their own plan, and
a pull request reviews plan changes with code.

## Outcome

A workspace index that is derived and rebuildable, never an authority.

- Each repository's tree store stays the authority, in Git.
- An index (SQLite on one machine; Postgres where several machines share it) ingests each member's
  log keyed by (repository, commit, event), so every indexed fact names the commit it was read at.
- Cross-repository relations and queries read the index; a stale or lost index is rebuilt from the
  member repositories and changes no authority.

## Acceptance

- Deleting the index and rebuilding it from the same member commits yields the same query answers.
- A query answer names the repository and commit each artifact state was read at.
- An index behind a member's head reports itself stale rather than answering from the old commit.
