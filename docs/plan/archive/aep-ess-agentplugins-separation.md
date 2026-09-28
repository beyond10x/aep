# Separate AEP, ESS and agent plugins

**Status:** accepted by the operator on 2026-09-01  
**Tracking:** `epic:separate-aep-ess-and-agentplugins`  
**Cross-repository decision:** Atlas ADR 0017

## Outcome

Three repositories own three products:

| repository | owns | canonical command or marketplace |
|---|---|---|
| `beyond10x/aep` | Agentic Engineering Protocol, ADP/AOP profiles, planning, governance, driver, trace evaluation and AEP evidence | `aep`, with exact `protocol` compatibility |
| `beyond10x/ess` | Executable System Specification models, compiler, generators, synthesis, conformance and adapters | `ess` |
| `beyond10x/agentplugins` | curated harness integrations | marketplace `beyond10x` |

`infra-scout` moves into ESS as its credential-edge Kubernetes importer and is archived only after
the moved scanner proves the same sanitization behavior. ESS has no AEP dependency. AEP consumes a
standalone ESS conformance report only through an optional AEP-owned evidence adapter.

## Vocabulary

- **AEP** is the Agentic Engineering Protocol.
- **ADP** is the AEP development profile.
- **AOP** is the AEP operations profile.
- **ESS** is Executable System Specification.
- **IR** is a typed, validated intermediate representation produced from a system description.
- **Engineering Protocols / EP** is historical repository terminology and is not a new product
  acronym after migration.

Other workflows remain named AEP profiles until they establish distinct semantics. This migration
does not create additional protocol acronyms.

## Boundaries that must survive

### AEP

AEP retains every `aep-*` crate, including the development and operations profile crates;
provider-backed planning; the reference driver; trace checking and evaluation; artifact
definitions; protocols, profiles, principles and workflows; and governance documents. It owns the
generic planning substrate. ADP and AOP supply profile-specific workflow and vocabulary.

`aep` becomes canonical. `protocol` is not a deprecation stub: for every retained operation it must
produce the same stdout, stderr and exit status as `aep`. The compatibility test executes both
binaries over accepted, refused and usage-error cases.

### ESS

ESS receives the `ess-*`, `infra-*` and `schema-contract` crates; ESS and infrastructure examples;
generated artifacts; conformance suites; generators; accepted ESS/infra designs and plan history;
and the Kubernetes scanner. Filtered history retains the commits that introduced and changed those
paths.

Borrowed AEP primitives become ESS-owned validation, predicate, value, digest, timestamp,
consistency and report types. Their current serialized behavior is the compatibility baseline.
The standalone conformance report is an ESS document; it is not an AEP `Evidence` variant.

### Agent plugins

The initial marketplace contains three focused plugins, named by product and verb since
`agentplugins@a2077d2` (`CHANGELOG.md` carries the full id map):

- `aep-plan`: planning skill, decomposer, plan reviewer and reverse engineer;
- `aep-drive`: wave coordination, story scoper, implementor and adversary;
- `ess-specify`: schema validation and deterministic projection guidance.

Plugin manifests, skills and agent charters contain no obsolete marketplace id, former
organization identity, `track@agentplugins`, or mixed `aep` install path.

## ESS IR policy

Extraction is not authority for a generic facet registry or `ess-ir/2`.

- Extend the Rust-owned model with concrete typed kinds, handles, ordered collections and relations
  when an importer or projector establishes their semantics.
- Add system, service, CLI, infrastructure, organization, team, role and ownership types at their
  first real adapter, not in anticipation of one.
- Keep compiler-minted handles and total lookups for resolved references.
- Keep ordered collections and deterministic serialization.
- Preserve source-specific details in explicit typed structures, not arbitrary JSON property bags.
- Keep `EssIr` and `InfraIr` separate until a concrete comparison or adapter demonstrates that
  unification removes duplication or enables required behavior.

A persisted format version changes only when a field changes meaning; a field or kind is renamed or
removed; identity/reference semantics change; digest/canonicalization changes; an old reader would
refuse or misinterpret new bytes; or a new envelope replaces the separate IRs. Internal Rust types
and compiler capabilities do not alone move a format. Because `infra-ir/1` rejects unknown fields,
even an additive field requires an old-reader test.

## Import and projection contract

Every adapter declares supported kinds and directions.

`ess import <adapter>` reads a concrete source and returns validated typed IR plus coverage,
diagnostics and unresolved references. It does not guess missing semantics.

`ess project <adapter>` turns supported IR into artifacts plus obligations and explicit refusals.
It never applies infrastructure or mutates an external system.

Initial adapters:

1. Kubernetes cluster or bundle to sanitized infrastructure IR;
2. infrastructure intent to Kubernetes manifests;
3. OpenAPI to service, operation, type and interface structures;
4. ESS service/interface structures to OpenAPI.

Round-trip guarantees are adapter-local:

- IR → target → IR is semantically equivalent for the declared supported subset;
- target → IR → target may normalize representation;
- unsupported input is a coverage gap, obligation or refusal.

No adapter claims universal reversibility.

## Migration sequence

### 1. Pin the current contract

- Add a machine-readable inventory of ESS-to-AEP dependencies.
- Hash every persisted v1 fixture and generated byte tree that moves.
- Capture accepted, refused and usage-error behavior for every current `protocol ess` verb.
- Add the future cross-binary equivalence harness before removing the current command.

### 2. Extract ESS

- Filter the source history to ESS, infrastructure, schema contract, examples, generators, suites,
  designs and their build/gate support.
- Introduce ESS-owned primitives and remove every `aep-*` and historical repository dependency.
- Keep fixture and generated-byte hashes exact.
- Establish `ess` as the standalone CLI for validation, compilation, inspection, graphing, diff,
  impact, generation, synthesis, conformance, import and projection.

### 3. Move the scanner

- Merge `infra-scout` history into ESS.
- Keep kubeconfig and live cluster access in a named adapter crate.
- Preserve unconditional Secret `data`/`stringData` digest replacement and removal of the
  last-applied annotation.
- Mutation-test both redaction guards.
- Keep live-cluster tests outside the offline gate.

### 4. Extract plugins

- Create the `beyond10x` marketplace and three focused plugin packages.
- Validate manifests, rosters, skill references and repository identities.
- Publish before deleting the source integrations.

### 5. Reduce and rename AEP

- Remove ESS, infrastructure and plugin sources only after their released replacements exist.
- Remove core compilation against ESS types.
- Add the optional standalone-report-to-`ess_conformance` adapter.
- Rename the repository and canonical documentation URLs to `aep`.
- Publish `aep` and the exact `protocol` alias together.

### 6. Move consumers

In order: `aep-service`, `entity-runtime`, `metaharness`, Atlas and public documentation. Update
released Cargo pins, fixture source identities, command invocations, evaluation directories and
marketplace instructions. Archive `infra-scout` last.

## Verification

- ESS dependency scan finds no AEP or historical Engineering Protocol dependency.
- Every persisted v1 fixture and generated artifact matches its pre-extraction SHA-256.
- `protocol`, `aep` and `ess` equivalence cases match the declared command split exactly.
- IR serialization is deterministic and every resolved handle lookup is total.
- A strict v1 format extension includes an old-reader test.
- Each adapter reports coverage and passes its declared semantic round trip.
- Kubernetes redaction tests fail under mutations that retain a Secret value or its annotation
  copy; the offline gate never invokes a cluster.
- Plugin manifests and instructions contain only the `beyond10x` marketplace identity.
- Each repository's complete gate and documentation build exit zero.

## Explicitly out of scope

- a generic facet registry;
- `ess-ir/2` without an independently required compatibility break;
- merging `EssIr` and `InfraIr` merely because they share a repository;
- applying generated infrastructure;
- inventing semantics for unsupported import input;
- new protocol acronyms beyond AEP, ADP and AOP;
- removing `protocol` compatibility.
