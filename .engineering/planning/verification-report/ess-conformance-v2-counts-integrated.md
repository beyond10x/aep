---
format: aep.planning-md/1
id: verification-report:ess-conformance-v2-counts-integrated
kind: verification-report
status: draft
title: Paired count reader integrated verification
relations:
- verifies: story:admit-ess-conformance-v2-counts
revision: 1
---
## Source and completed checks

Clean integration `012a183c1228538c86be26a91e46d982b01c970a` contains reader implementation `fc58d0fb365f04f2c92e0a7cc7a278f85b55ec8e` and first-review tests `a5ad722399cfc17beef84208b44c9c98b2a51c70`. All sixteen declared gate commands returned zero. The PostgreSQL command explicitly skipped because `ENTITY_POSTGRES_URL` was unset; no live database evidence is claimed. The full workspace suite passed 2,209 cases, zero failed or ignored, across 147 runner summaries. MSRV 1.85 and the actual Website build passed.

## Implementation and independent review

The implementation passed 1,313 assigned package cases and retains 32 measured guard mutations with original/red/restored/green evidence. It generated exactly ten schemas and six instruction files through their canonical writers. Both default and arbitrary-precision feature lanes preserve legacy time semantics separately from exact new integer tokens. Schema closure and policy/profile inventory corrections are retained with their original failed runs.

First independent implementation review added seven cases and executed all 1,320 assigned cases with zero failures or ignored cases. Its immutable report is `review-result:ess-conformance-v2-counts-adversary-pass-1`, SHA-256 `a76352d1f461e387f8b229e41f31c33ef841f3be423ed4b4518c7d6295924bd6`; original test construction and diagnostic-wording mistakes remain separated from semantic results. It returned no findings. Root verified all three test hashes and formatting, committed them through organization tooling, and merged only the green unit.

The pure optional reader admits original report/2 and suite/1–4 pairs, retains exact source and numeric identity, and re-admits at mutation/restore boundaries. Independent expectations and shared same-record qualification prevent combining unrelated facts. Count-stage coverage remains unknown and never qualifies complete conformance. Trusted custom Rust readers/contexts remain explicit extension code. Default policies, frozen legacy meanings and ESS dependency direction remain unchanged. No suite/5, detailed run/2 reader, ESS writer or external installed binary upgrade is claimed.

## Gate environment correction

The first complete suite ran 1,841 cases before stopping at an existing aep-project test: 1,840 passed, one failed, none ignored. Its fixture assumed `std::env::temp_dir()` had no project ancestor, but the assigned TMPDIR was inside this managed AEP checkout. No production or test source was changed. Root reassigned gate TMPDIR/GOTMPDIR to `~/.cache/ess-review/2026-09-06-resume/aep-gate-tmp`; the exact previously failing case passed, followed by the full workspace suite and every remaining gate command. The first nine passing step records are retained; failed `results.json`, original logs, focused correction and final `resolved-results.json` all remain available.

The installed older CLI correctly refused the additive evidence vocabulary before opening the planning store. Completion records use this frozen source's freshly built `target/debug/aep`; the installed binary was not replaced. This is observed compatibility behavior, not a reason to reinterpret the new vocabulary for an old engine.

## Individual gate results

```json
{
  "subject": "012a183c1228538c86be26a91e46d982b01c970a",
  "steps": [
    {
      "step": "fmt-check",
      "argv": [
        "task",
        "fmt-check"
      ],
      "started": "2026-09-06T08:09:51.753183+00:00",
      "finished": "2026-09-06T08:10:06.916557+00:00",
      "seconds": 15.163389062974602,
      "exit": 0
    },
    {
      "step": "status-check",
      "argv": [
        "task",
        "status-check"
      ],
      "started": "2026-09-06T08:10:06.916747+00:00",
      "finished": "2026-09-06T08:10:07.089600+00:00",
      "seconds": 0.17286594293545932,
      "exit": 0
    },
    {
      "step": "plan-check",
      "argv": [
        "task",
        "plan-check"
      ],
      "started": "2026-09-06T08:10:07.089799+00:00",
      "finished": "2026-09-06T08:10:45.503540+00:00",
      "seconds": 38.41375283198431,
      "exit": 0
    },
    {
      "step": "audit-check",
      "argv": [
        "task",
        "audit-check"
      ],
      "started": "2026-09-06T08:10:45.519090+00:00",
      "finished": "2026-09-06T08:10:48.253862+00:00",
      "seconds": 2.734784708940424,
      "exit": 0
    },
    {
      "step": "version-check",
      "argv": [
        "task",
        "version-check"
      ],
      "started": "2026-09-06T08:10:48.254054+00:00",
      "finished": "2026-09-06T08:10:48.387307+00:00",
      "seconds": 0.13326543499715626,
      "exit": 0
    },
    {
      "step": "dep-check",
      "argv": [
        "task",
        "dep-check"
      ],
      "started": "2026-09-06T08:10:48.387514+00:00",
      "finished": "2026-09-06T08:10:48.504360+00:00",
      "seconds": 0.11685851798392832,
      "exit": 0
    },
    {
      "step": "guard-check",
      "argv": [
        "task",
        "guard-check"
      ],
      "started": "2026-09-06T08:10:48.504563+00:00",
      "finished": "2026-09-06T08:10:48.693094+00:00",
      "seconds": 0.18854634696617723,
      "exit": 0
    },
    {
      "step": "claim-check",
      "argv": [
        "task",
        "claim-check"
      ],
      "started": "2026-09-06T08:10:48.693403+00:00",
      "finished": "2026-09-06T08:10:50.179865+00:00",
      "seconds": 1.486474673030898,
      "exit": 0
    },
    {
      "step": "clippy",
      "argv": [
        "task",
        "clippy"
      ],
      "started": "2026-09-06T08:10:50.180083+00:00",
      "finished": "2026-09-06T08:11:14.477520+00:00",
      "seconds": 24.297448510071263,
      "exit": 0
    },
    {
      "step": "test",
      "argv": [
        "task",
        "test"
      ],
      "started": "2026-09-06T08:16:25.126755+00:00",
      "finished": "2026-09-06T08:18:34.301970+00:00",
      "seconds": 129.175218904973,
      "exit": 0,
      "counts": {
        "passed": 2209,
        "failed": 0,
        "ignored": 0,
        "summaries": 147
      }
    },
    {
      "step": "docs-check",
      "argv": [
        "task",
        "docs-check"
      ],
      "started": "2026-09-06T08:18:34.302842+00:00",
      "finished": "2026-09-06T08:18:38.927921+00:00",
      "seconds": 4.625083729042672,
      "exit": 0
    },
    {
      "step": "postgres-check",
      "argv": [
        "task",
        "postgres-check"
      ],
      "started": "2026-09-06T08:18:38.928126+00:00",
      "finished": "2026-09-06T08:18:38.953312+00:00",
      "seconds": 0.025191328953951597,
      "exit": 0,
      "printed": "task: [postgres-check] scripts/postgres-check.sh\npostgres-check: skipped, ENTITY_POSTGRES_URL unset"
    },
    {
      "step": "doc-check",
      "argv": [
        "task",
        "doc-check"
      ],
      "started": "2026-09-06T08:18:38.953540+00:00",
      "finished": "2026-09-06T08:18:50.450163+00:00",
      "seconds": 11.496632250025868,
      "exit": 0
    },
    {
      "step": "schema-check",
      "argv": [
        "task",
        "schema-check"
      ],
      "started": "2026-09-06T08:18:50.450433+00:00",
      "finished": "2026-09-06T08:18:50.581542+00:00",
      "seconds": 0.1311148969689384,
      "exit": 0
    },
    {
      "step": "msrv",
      "argv": [
        "task",
        "msrv"
      ],
      "started": "2026-09-06T08:18:50.581768+00:00",
      "finished": "2026-09-06T08:19:17.377724+00:00",
      "seconds": 26.795960815041326,
      "exit": 0
    },
    {
      "step": "website",
      "argv": [
        "task",
        "website"
      ],
      "started": "2026-09-06T08:19:17.377949+00:00",
      "finished": "2026-09-06T08:19:34.313288+00:00",
      "seconds": 16.935346888029017,
      "exit": 0
    }
  ]
}
```

## Retention and remaining migration work

Exact argv, times, stdout/stderr and exits are in `target/ess-conformance-v2-counts/final-gate`. The complete unit handoff and review archive is `~/.cache/ess-review/2026-09-06-resume/aep-count-reader-evidence.tar.gz`: 3,043 verified evidence files, SHA-256 `bb7667d85405e2eae4b0eb8ae21f5b34561b48f5d1697cb4ffcb47f00685c41f`. Its adjacent manifest binds file hashes and eight excluded disposable cache directories. No symlink fixture was omitted. Existing shared dependency caches are incidental infrastructure effects; no shared build target or replacement cache daemon was used. No owned test process remains.

Per-agent token and tool-use counters are unavailable from the resumed harness rather than estimated. Measured command durations remain in the reports. The coordinator owns all store, Git, publication and lifecycle writes.

Publish the exact green reader source before ESS enables its opt-in writer, then qualify actual Rust and generated Go producer bytes through both readers and typed replay under Atlas ADR 0039. Unknown external adopters and installed/generated runtime pins remain unresolved. Public documentation delivery and managed cleanup are separate observed outcomes. No default switch, release tag or version bump is inferred.
