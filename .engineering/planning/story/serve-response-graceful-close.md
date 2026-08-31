---
format: aep.planning-md/1
id: story:serve-response-graceful-close
kind: story
status: active
title: Read-only server test retains startup output
summary: Keep the child stdout pipe alive so the final startup notice cannot terminate the server before its first request.
relations:
- serves: vision:O2
revision: 6
---
## Context

The 0.36.3 remote release gate twice received `ECONNRESET` before the read-only server test could read its first response. A local backtrace stress run reproduced it on the first `GET /api/board` and, with server stderr visible, showed `Broken pipe` first. The harness stopped reading and dropped the child stdout pipe as soon as it saw the URL, but `protocol serve --read-only` prints its read-only notice after that URL. Depending on scheduling, the final startup write therefore terminated the process while the client was connecting. Response close timing was not the cause.

## Acceptance

The socket harness retains the server's stdout pipe until the child is dropped, exposes server stderr in the test output, and completes repeated read-only socket exchanges without a broken startup pipe. The complete remote release gate passes the exchange on new release source.

## Implementation

The socket harness now keeps the child stdout reader in `Served` until `Drop` kills and waits for the child. That lets `protocol serve --read-only` finish printing the startup notice after the URL without racing a closed pipe. Server stderr is inherited so a future child-side failure is visible at its source. The complete socket suite passed, the formerly intermittent read-only case passed 500 consecutive runs, and the complete local gate passed against live PostgreSQL. The story remains active until the new release source passes its remote release gate.
