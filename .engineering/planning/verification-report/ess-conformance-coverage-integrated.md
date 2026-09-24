---
format: aep.planning-md/1
id: verification-report:ess-conformance-coverage-integrated
kind: verification-report
status: draft
title: ESS coverage reader integrated verification
relations:
- verifies: story:admit-ess-conformance-coverage
revision: 1
---
## Integrated source and result

Clean integration 58fe2f3e7bc1286d5de8f9bc6188da90ef9e8f8c contains the complete reader implementation, correction143ad4afc2d4ecb6f280bcad2932239bed5fbfd5 and both independent source-review test additions through00b6108d7656b3947fa8940015147b317338ad7c. Every one of the16 unchanged Taskfile gate steps returned0. Workspace tests passed2269cases,0failed/ignored,154summaries in204.210432894seconds; docs-check separately passed2cases. The actual Rust1.85 toolchain, schema drift, strict Clippy and Website build passed. PostgreSQL explicitly skipped because ENTITY_POSTGRES_URL was unset; no local database execution is claimed.

The runner compared the complete tracked-source manifest after every step and at completion: no tracked bytes changed. The source gate ran from2026-09-06T15:27:34.342124Z through2026-09-06T15:33:27.361723Z. Source/toolchains, original argv, own statuses, timestamps, durations, raw logs and hashes are retained under target/ess-conformance-coverage/gate-58fe2f3e7bc1. The assigned external TMPDIR is home-path:sha256:bf7fda14784fbf6a0550cd33a889e586381626b9fad9d4ad6ac24aac19f93aab and remains owned for archival/cleanup.

## Independent review

The first source attack executed1366cases,1363passed and3failed, with two confirmed comparison findings. Correction added9class cases and killed/restored4guard mutants; its five-package run passed1375cases. The second/final source attack added5cases and passed1380cases, returning no semantic findings. Its helper-only lint correction and affected reruns are retained separately; the final integrated workspace gate above includes the corrected final bytes. Both immutable reports and the actual findings comparison remain in the wave record. The first review's fixed outcome is linked to the correction; the empty final review receives no fictitious outcome.

The implementation preserves old/count carriers, exact original source identities, exact u64 metadata, inherited Node/default semantics at typed owners and the optional ESS dependency boundary. Coverage qualification requires the same admitted complete nonempty selection and independent task expectation across evidence evaluation, submission and replay. Runtime defaults and installed binaries remain unchanged.

## Separate delivery and producer obligations

This is source verification. Exact bot source publication, CI and immutable Website/Atlas delivery are recorded separately when observed. Reader-first publication must precede the future ESS suite5 writer. Actual Rust and generated Go report/input correspondence through both readers and typed replay remains mandatory before that writer is published; preparation harness compilation supplies none of that evidence. No release, tag or version bump is selected.

## Individual gate results

```json
[
  {
    "name": "fmt-check",
    "argv": [
      "task",
      "fmt-check"
    ],
    "started_at": "2026-09-06T15:27:34.342298+00:00",
    "finished_at": "2026-09-06T15:27:46.926545+00:00",
    "elapsed_seconds": 12.584252735017799,
    "exit": 0,
    "log": "fmt-check.log",
    "log_sha256": "026067bee084e4803e39a807e9be8cc1f748931a5da9f2c6e5f68913ebc1ccfa",
    "free_bytes_before": 51734425600,
    "free_bytes_after": 51367116800,
    "source_unchanged": true
  },
  {
    "name": "status-check",
    "argv": [
      "task",
      "status-check"
    ],
    "started_at": "2026-09-06T15:27:46.980204+00:00",
    "finished_at": "2026-09-06T15:27:47.152828+00:00",
    "elapsed_seconds": 0.17262258904520422,
    "exit": 0,
    "log": "status-check.log",
    "log_sha256": "8969d1a461fa36532042db81cac500f40fa3743306ab8fc92d06796855765afd",
    "free_bytes_before": 51366637568,
    "free_bytes_after": 51356442624,
    "source_unchanged": true
  },
  {
    "name": "plan-check",
    "argv": [
      "task",
      "plan-check"
    ],
    "started_at": "2026-09-06T15:27:47.203712+00:00",
    "finished_at": "2026-09-06T15:28:26.304697+00:00",
    "elapsed_seconds": 39.10098475194536,
    "exit": 0,
    "log": "plan-check.log",
    "log_sha256": "95288e00e26b53a4f02a2973f73508def6d20d132dab881845efea0c536e8e61",
    "free_bytes_before": 51356446720,
    "free_bytes_after": 49014394880,
    "source_unchanged": true
  },
  {
    "name": "audit-check",
    "argv": [
      "task",
      "audit-check"
    ],
    "started_at": "2026-09-06T15:28:26.359914+00:00",
    "finished_at": "2026-09-06T15:28:30.312445+00:00",
    "elapsed_seconds": 3.95253271702677,
    "exit": 0,
    "log": "audit-check.log",
    "log_sha256": "e181294286450a246d0d220bbb3d4ffd7536ce825bd39645e616d5a2cd3b12bf",
    "free_bytes_before": 49011486720,
    "free_bytes_after": 48978771968,
    "source_unchanged": true
  },
  {
    "name": "version-check",
    "argv": [
      "task",
      "version-check"
    ],
    "started_at": "2026-09-06T15:28:30.700059+00:00",
    "finished_at": "2026-09-06T15:28:31.359630+00:00",
    "elapsed_seconds": 0.6595696019940078,
    "exit": 0,
    "log": "version-check.log",
    "log_sha256": "986ac780df3c762b7c6b7205e9d2bc1ce51742c533c4097b763c9bd68710a90a",
    "free_bytes_before": 48974884864,
    "free_bytes_after": 48971526144,
    "source_unchanged": true
  },
  {
    "name": "dep-check",
    "argv": [
      "task",
      "dep-check"
    ],
    "started_at": "2026-09-06T15:28:31.421295+00:00",
    "finished_at": "2026-09-06T15:28:31.549768+00:00",
    "elapsed_seconds": 0.1284741839626804,
    "exit": 0,
    "log": "dep-check.log",
    "log_sha256": "e6502386c86d7333174ddf3824629ddd40c5672568872ca1e46a6f4f91d25ab2",
    "free_bytes_before": 48971517952,
    "free_bytes_after": 48970825728,
    "source_unchanged": true
  },
  {
    "name": "guard-check",
    "argv": [
      "task",
      "guard-check"
    ],
    "started_at": "2026-09-06T15:28:31.608314+00:00",
    "finished_at": "2026-09-06T15:28:31.837113+00:00",
    "elapsed_seconds": 0.22879671503324062,
    "exit": 0,
    "log": "guard-check.log",
    "log_sha256": "114906c4ad7618cad4ac31529b3a80f59cd991f14fd3f5694b2f88a90e9d1e2a",
    "free_bytes_before": 48970801152,
    "free_bytes_after": 48969326592,
    "source_unchanged": true
  },
  {
    "name": "claim-check",
    "argv": [
      "task",
      "claim-check"
    ],
    "started_at": "2026-09-06T15:28:31.892634+00:00",
    "finished_at": "2026-09-06T15:28:33.505841+00:00",
    "elapsed_seconds": 1.6132063739933074,
    "exit": 0,
    "log": "claim-check.log",
    "log_sha256": "4e355be4802309d45d0238a8fff85ffa333c29a3e2c89a17d6e6284e79f9f229",
    "free_bytes_before": 48969322496,
    "free_bytes_after": 48968880128,
    "source_unchanged": true
  },
  {
    "name": "clippy",
    "argv": [
      "task",
      "clippy"
    ],
    "started_at": "2026-09-06T15:28:33.573281+00:00",
    "finished_at": "2026-09-06T15:28:59.519185+00:00",
    "elapsed_seconds": 25.945905768079683,
    "exit": 0,
    "log": "clippy.log",
    "log_sha256": "0df4f39ce3ffaf52a0e4f5e60b69e057093efaa6185d82991cc8ca84224510cc",
    "free_bytes_before": 48968876032,
    "free_bytes_after": 48772702208,
    "source_unchanged": true
  },
  {
    "name": "test",
    "argv": [
      "task",
      "test"
    ],
    "started_at": "2026-09-06T15:28:59.577470+00:00",
    "finished_at": "2026-09-06T15:32:23.787906+00:00",
    "elapsed_seconds": 204.21043289394584,
    "exit": 0,
    "log": "test.log",
    "log_sha256": "9aba33a1fe633b9b69d2c0ec5afe6d452b989b09afc59efff13dc7d8948c5bf9",
    "free_bytes_before": 48772632576,
    "free_bytes_after": 47414915072,
    "counts": {
      "passed": 2269,
      "failed": 0,
      "ignored": 0,
      "summaries": 154
    },
    "source_unchanged": true
  },
  {
    "name": "docs-check",
    "argv": [
      "task",
      "docs-check"
    ],
    "started_at": "2026-09-06T15:32:23.838467+00:00",
    "finished_at": "2026-09-06T15:32:28.421402+00:00",
    "elapsed_seconds": 4.582936963066459,
    "exit": 0,
    "log": "docs-check.log",
    "log_sha256": "7750894c8167c61a6af214c933d9d69ec20c27aef9b1608408e57af8df7b02fd",
    "free_bytes_before": 47414915072,
    "free_bytes_after": 47411347456,
    "counts": {
      "passed": 2,
      "failed": 0,
      "ignored": 0,
      "summaries": 1
    },
    "source_unchanged": true
  },
  {
    "name": "postgres-check",
    "argv": [
      "task",
      "postgres-check"
    ],
    "started_at": "2026-09-06T15:32:28.471289+00:00",
    "finished_at": "2026-09-06T15:32:28.495211+00:00",
    "elapsed_seconds": 0.023922297987155616,
    "exit": 0,
    "log": "postgres-check.log",
    "log_sha256": "36f5069cf1be33b40db1d6a26e225a96532b756632ad31f2ca11cbdfe01773ef",
    "free_bytes_before": 47411347456,
    "free_bytes_after": 47411343360,
    "raw_observation": "task: [postgres-check] scripts/postgres-check.sh\npostgres-check: skipped, ENTITY_POSTGRES_URL unset\n",
    "skipped": true,
    "source_unchanged": true
  },
  {
    "name": "doc-check",
    "argv": [
      "task",
      "doc-check"
    ],
    "started_at": "2026-09-06T15:32:28.544803+00:00",
    "finished_at": "2026-09-06T15:32:44.301520+00:00",
    "elapsed_seconds": 15.756718315067701,
    "exit": 0,
    "log": "doc-check.log",
    "log_sha256": "b588028246fade3ee4736a13b22bcbd4908a183ccf992c1d47396eb71d44ad6e",
    "free_bytes_before": 47411343360,
    "free_bytes_after": 47403057152,
    "source_unchanged": true
  },
  {
    "name": "schema-check",
    "argv": [
      "task",
      "schema-check"
    ],
    "started_at": "2026-09-06T15:32:44.351661+00:00",
    "finished_at": "2026-09-06T15:32:44.504310+00:00",
    "elapsed_seconds": 0.15264775208197534,
    "exit": 0,
    "log": "schema-check.log",
    "log_sha256": "979f4c7554f1bbf3f32a455dbd3b5d9d7a644fc2badb17be965a385f9c189bbd",
    "free_bytes_before": 47403057152,
    "free_bytes_after": 47403053056,
    "source_unchanged": true
  },
  {
    "name": "msrv",
    "argv": [
      "task",
      "msrv"
    ],
    "started_at": "2026-09-06T15:32:44.554071+00:00",
    "finished_at": "2026-09-06T15:33:03.808860+00:00",
    "elapsed_seconds": 19.254789412021637,
    "exit": 0,
    "log": "msrv.log",
    "log_sha256": "088273a3e8a0bbbb45be4b3bb52b997934bba8f4a1b6d638c91e032708e5104d",
    "free_bytes_before": 47403053056,
    "free_bytes_after": 47384711168,
    "source_unchanged": true
  },
  {
    "name": "website",
    "argv": [
      "task",
      "website"
    ],
    "started_at": "2026-09-06T15:33:03.858514+00:00",
    "finished_at": "2026-09-06T15:33:27.312273+00:00",
    "elapsed_seconds": 23.45376301300712,
    "exit": 0,
    "log": "website.log",
    "log_sha256": "c25da7ffd2de96d733363cf1d9bc1b24268386439cf6637be12a2d83994b2e04",
    "free_bytes_before": 47384616960,
    "free_bytes_after": 47158468608,
    "source_unchanged": true
  }
]
```
