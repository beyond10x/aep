---
title: Validate the plan in CI
sidebar_position: 6
description: Run aep plan artifact validate as a required check on every pull request, with a pinned aep and an optional strict mode.
---

# Validate the plan in CI

`aep plan artifact validate` reads every artifact and evidence file and exits `1` on any problem, so
it is already a gate. Run it on every pull request that touches `.engineering/`, and make the check
required.

## What to pin

| What | Why | How |
|---|---|---|
| the `aep` binary | a newer or older `aep` can read the store differently | download one release archive and check its checksum |
| the governing documents | the lifecycles decide which statuses are legal | `protocols:` in `project.yaml` is a path in the repository or a Git locator pinned to a commit |

A pinned Git source is fetched into a cache on first use (`AEP_CACHE_DIR` moves the cache). The job
therefore needs network access to that repository, or a vendored tree.

## A GitHub Actions job

{/* generated:release-pin:begin version=0.64.0 — kept by `cargo xtask status` */}
```yaml
name: Planning store
on:
  pull_request:
  push:
    branches: [main]
permissions:
  contents: read
jobs:
  validate:
    runs-on: ubuntu-latest
    env:
      AEP_VERSION: "0.64.0"
    steps:
      - uses: actions/checkout@v4
      - name: Install aep
        run: |
          base=https://github.com/beyond10x/aep/releases/download/$AEP_VERSION
          archive=aep-$AEP_VERSION-x86_64-unknown-linux-gnu.tar.gz
          curl -sSLO "$base/$archive"
          curl -sSLO "$base/SHA256SUMS"
          sha256sum -c --ignore-missing SHA256SUMS
          tar xzf "$archive"
          echo "$PWD/aep-$AEP_VERSION-x86_64-unknown-linux-gnu" >> "$GITHUB_PATH"
      - name: Validate the planning store
        run: aep plan artifact validate
```
{/* generated:release-pin:end */}

Pin the checkout action to a commit if your policy requires it. The job needs no credentials: it
reads files and writes nothing.

## Strict mode

`validate` separates problems (exit `1`) from findings it only reports:

| Reported, exit `0` | Why it is allowed by default |
|---|---|
| a status reached on an assertion (`move --evidence …`) rather than a recorded file | somebody may close a story on the day a runner is down |
| a `review-result` whose findings are prose only | a review written as prose is still a review |
| a review older than `--outcome-within` days (default 14) with no `review_outcome` | outstanding work, not a broken store |

`aep plan artifact validate --strict` exits `1` on any of these, and names which class decided.
Use it where the plan must hold only recorded evidence, for example on `main`. The review age is the
one line that depends on when the job runs.

## Also worth running

| Command | Gate on |
|---|---|
| `aep doctor` | exits `1` on any `fail`: a project file that does not parse, a protocol source that does not resolve, a store `validate` would reject |
| `aep plan workspace crossings --strict` | every edge into another repository of a workspace resolves; only meaningful when every member is checked out |
| `aep plan artifact waves` | exits `2` on a `depends_on` cycle |

## Locally, before you push

The same command works as a pre-push check:

```bash
aep plan artifact validate || exit 1
```
