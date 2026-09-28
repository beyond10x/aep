# Direct-return reader verification

These logs retain the original ER prototype report/suite refusal, new closed-reader red tests,
producer-driven optional/integral-number corrections, restored passing reader tests and the full
repository `task check` run. All selected checks exited zero after restoration. The optional
PostgreSQL checks retain their explicit local configuration result; CI owns configured provider
verification. Home path prefixes in logs are replaced by `$HOME`.

`suite-digest-mutation.patch` removes only the report/suite identity guard. The named integrity
test then fails by admitting the wrong digest. The restored test passes. Patch headers use
repository-relative paths; the mutation is absent from source.

The complete freshly generated ER inventory suite was admitted while preserving its exact input
bytes. That shape probe is not execution evidence. The actual ER implementation report belongs to
ER's `docs/ess/evidence/final/`, where its precise suite association is retained and imported through
AEP's evidence command. The small producer fixtures under the reader's tests retain their source
and original suite bytes; independently authored test reports make no implementation-run claim.

Released AEP 0.63.1 read and validated the temporary planning history written by the extended CLI.
That demonstrates storage compatibility only: typed admission and engine replay require the
extended reader, as the design document explains.
