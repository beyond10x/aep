# Planning migration raw capture and digest contract v0.1

Status: accepted by the initiative coordinator for bounded pure implementation under
story:eventlog-planning-authority-migration and Atlas ADR0050. Two independent technical design
rounds completed: the final verdict remains needs-revision; its one new hybrid-policy omission was
corrected by the coordinator after the review. No independent approval of that correction is claimed.
Implementation must verify that correction and every contract rule below; source acceptance is pending.
This specifies one pure transport unit within the complete migration. It does not admit acquisition,
writer exclusion, semantic import, runtime readiness or any of the six cutovers. Existing migration
coordinates live in specs/planning-migration/; this page defines their raw observation input, not
a replacement logical entity model. Rust types own generated schemas; no ESS dependency enters AEP.

Source basis: AEP 4eb999e0ae3cc77d1c387152e23a85ad4eae86dc and its pinned Entity Runtime
faadc04f2f273517e21815d32ba3866f3aea7642. Clauses below are proposed decisions unless marked observed.
The full owner acceptance in eventlog-planning-authority-v0.1.md remains required.

## Ownership and wire language

`aep-contract::migration::{capture,digest}` owns provider-neutral raw values. Observed dependency:
the Markdown backend depends on aep-contract; its PlanningDocument and journal::Entry therefore
stay backend-local. No backend, ER provider, filesystem, environment, clock or network enters this unit.

Every struct is closed and requires every declared field. Every enum uses a closed adjacent envelope:
a unit is exactly `{"kind":"tag"}`; a payload variant exactly `{"kind":"tag","value":payload}`.
Unit `value` (including null), missing payload, null, duplicate/unknown keys and other kinds refuse.
Object member order does not affect reading. Declaration order controls writing. Use explicit tags
listed below and custom envelope reading where derives accept a wider language. Schemas must describe
that same language, including unit envelopes, rather than relying on Schemars to infer closure.

`PresenceV1<T> = Missing | Present(T)` makes absence explicit without JSON null. `RawCaptureFormatV1`
is the scalar string literal `aep.raw-capture/1`. `HexBytesV1` is `hex:` followed by an even number of
lowercase hex digits; `hex:` is empty. `DigestV1` is `sha256:` plus 64 lowercase hex digits and exposes
decoded [u8;32]. Custom scalar readers and schemas reject noncanonical values. No omitted/defaulted
fields, arbitrary JSON property bags, raw parser errors or private configuration values are permitted.

`HostPathV1 = Unix(HexBytesV1) | Windows(Vec<u16>)` preserves exact host units. Never normalize,
resolve symlinks, case-fold or lossily display paths. Relative-coordinate admission is lexical and
independent of the machine running the validator:

- Unix: nonempty valid UTF-8 bytes, no NUL; split only at byte 2f (`/`); refuse leading/trailing or
  adjacent separators and components `.` or `..`. Backslash and colon are ordinary Unix bytes.
- Windows: nonempty well-formed UTF-16 (paired surrogates only), no NUL; split only at unit 005c
  (backslash). Refuse leading/trailing/adjacent separators, `.`/`..` components, forward slash, colon,
  units 0001..001f and ASCII `< > " | ? *`, or a component ending in dot/space. Thus rooted, drive,
  UNC and device-prefixed paths refuse without host filesystem calls. Also refuse ASCII-insensitive
  reserved component stems CON, PRN, AUX, NUL, COM1..COM9 and LPT1..LPT9 (stem is before first dot).
- Raw path deserialization still preserves every host unit, including malformed/unpaired units.
  These admission failures are InvalidRelativePath with the original coordinate retained. They never
  cause replacement characters, separator rewriting or omitted observed nodes. Absolute source locator
  acquisition is a later edge obligation; this grammar applies only to relative captured coordinates.

## Coordinates, completeness and retained evidence

```text
RawCaptureObservationV1 { format: RawCaptureFormatV1,
  source: PresenceV1<SourceCoordinateV1>, selector: PresenceV1<SelectorCoordinateV1>,
  config_digest: PresenceV1<DigestV1>, observation: ObservationOutcomeV1 }
SelectorCoordinateV1 { project_root: HostPathV1, project_file: HostPathV1,
  presence: SelectorPresenceV1, store_field: StoreFieldV1 }
SelectorPresenceV1 = MissingV1Default | Present { selector_digest: DigestV1 }
StoreFieldV1 = MissingDefault | Present
SourceCoordinateV1 = Markdown { root: HostPathV1 } | Sqlite { database: HostPathV1 }
 | Postgres { endpoint: PostgresEndpointV1, endpoint_id: DigestV1 }
 | Hybrid { local_root: HostPathV1, replica: SqlReplicaCoordinateV1, divergence_file: HostPathV1,
            policy: PresenceV1<HybridPolicyWordsV1> }
SqlReplicaCoordinateV1 = Sqlite { database: HostPathV1 }
 | Postgres { endpoint: PostgresEndpointV1, endpoint_id: DigestV1 }
PostgresEndpointV1 { hosts: Vec<PostgresHostV1>, database: String, schema: String }
PostgresHostV1 = Tcp { host: String, port: u16 } | Unix { directory: HostPathV1, port: u16 }
ObservationMethodV1 = MarkdownDoubleScan | SqliteReadTransaction
 | PostgresRepeatableReadOnly | HybridBracketedSqlSnapshot
ObservationOutcomeV1 = Complete { method: ObservationMethodV1, phases: Vec<PhaseObservationV1>,
  capture: LegacyRawCaptureV1,
  transcript_digest: DigestV1, raw_snapshot_id: DigestV1 }
 | Unstable { method: ObservationMethodV1, phases: Vec<PhaseObservationV1>,
              changed: Vec<PhysicalCoordinateV1> }
 | Refused { method: PresenceV1<ObservationMethodV1>, phases: Vec<PhaseObservationV1>,
             preflight_refusals: Vec<CaptureRefusalV1> }
PhaseObservationV1 { phase: CapturePhaseV1, result: PhaseResultV1 }
CapturePhaseV1 = MarkdownFirst | MarkdownSecond | SqlSnapshot
 | LocalBefore | DivergencesBefore | ReplicaSnapshot | LocalAfter | DivergencesAfter
PhaseResultV1 = NotAttempted
 | Complete { evidence: PhaseEvidenceV1, evidence_digest: DigestV1 }
 | Refused { evidence: PhaseEvidenceV1, evidence_digest: DigestV1, refusals: Vec<CaptureRefusalV1> }
PhaseEvidenceV1 = Markdown(MarkdownEvidenceV1) | Sqlite(SqlEvidenceV1)
 | Postgres(SqlEvidenceV1) | Divergences(ObservedValueV1<FileImageV1>)
ObservedListV1<T> { items: Vec<T>, terminal: EnumerationTerminalV1 }
EnumerationTerminalV1 = NotAttempted | Complete
 | Refused { at: PhysicalCoordinateV1, code: CaptureRefusalCodeV1 }
ObservedValueV1<T> = NotAttempted | Complete(T)
 | Refused { at: PhysicalCoordinateV1, code: CaptureRefusalCodeV1 }
MarkdownEvidenceV1 { root: PresenceV1<HostPathV1>, nodes: ObservedListV1<MarkdownNodeEvidenceV1> }
MarkdownNodeEvidenceV1 = Captured(MarkdownNodeV1)
 | Unreadable { relative: HostPathV1, metadata: PresenceV1<NodeMetadataV1> }
NodeMetadataV1 { kind: MarkdownNodeKindTagV1, length: PresenceV1<u64> }
SqlEvidenceV1 { source: PresenceV1<SqlReplicaCoordinateV1>, catalog: SqlCatalogEvidenceV1,
  instances: ObservedListV1<InstanceRowV1>, events: ObservedListV1<EventRowV1>,
  history: ObservedListV1<HistoryRowV1>, legacy_origins: ObservedListV1<LegacyOriginRowV1>,
  provider_sequences: ObservedSequenceRowsV1, rejected_rows: Vec<RejectedSqlRowV1> }
ObservedSequenceRowsV1 = NotApplicable | Observed(ObservedListV1<ProviderSequenceRowV1>)
RejectedSqlRowV1 { at: PhysicalCoordinateV1, cells: Vec<SqlCellImageV1> }
SqlCellImageV1 { ordinal: u16, column: String, value: SqlCellValueV1 }
SqlCellValueV1 = Null | Integer(i64) | RealBits(u64) | Text(HexBytesV1) | Blob(HexBytesV1)
 | PostgresBinary { type_id: String, bytes: HexBytesV1 }
SqlCatalogEvidenceV1 { namespace: PresenceV1<String>, objects: ObservedListV1<CatalogObjectHeaderV1>,
  tables: Vec<TableCatalogEvidenceV1>, indexes: ObservedListV1<IndexSchemaV1>,
  foreign_objects: ObservedListV1<ForeignObjectV1>, rejected_rows: Vec<RejectedCatalogRowV1> }
CatalogObjectHeaderV1 { kind: ForeignObjectKindV1, name: String, parent: PresenceV1<String> }
TableCatalogEvidenceV1 { name: String,
  catalog_definition: ObservedValueV1<PresenceV1<HexBytesV1>>,
  columns: ObservedListV1<ColumnSchemaV1>, primary_key: ObservedValueV1<PresenceV1<KeySchemaV1>>,
  unique_keys: ObservedListV1<KeySchemaV1>, checks: ObservedListV1<CheckSchemaV1> }
RejectedCatalogRowV1 { at: PhysicalCoordinateV1, cells: Vec<SqlCellImageV1> }
CatalogFamilyV1 = Objects | TableDefinition | Columns | PrimaryKey | UniqueKeys | Checks
 | Indexes | ForeignObjects
```

The root permits absent selector/config/source because resolution itself can fail; Complete and
Unstable require all three present. A failed list preserves every obtained item and its exact terminal
refusal; it cannot assert unseen rows absent. NotAttempted requires an empty items list, whereas
Complete with empty items asserts an actual empty enumeration. ObservedValue Complete(Missing) asserts
that a query found no value; NotAttempted says it was not queried. Never fabricate a refusal for an
unattempted read. Retain malformed SQL/catalog cells without coercing their storage class.

Each known method has exactly this phase roster, in this order, with no extra/repeated phase:

| Method | Required ordered phases | Evidence variants |
| --- | --- | --- |
| MarkdownDoubleScan | MarkdownFirst, MarkdownSecond | Markdown, Markdown |
| SqliteReadTransaction | SqlSnapshot | Sqlite |
| PostgresRepeatableReadOnly | SqlSnapshot | Postgres |
| HybridBracketedSqlSnapshot | LocalBefore, DivergencesBefore, ReplicaSnapshot, LocalAfter, DivergencesAfter | Markdown, Divergences, selected Sqlite/Postgres, Markdown, Divergences |

Unknown method is allowed only for a preflight Refused result: phases is empty and preflight_refusals
nonempty. For a known method, preflight refusal has a complete roster of NotAttempted phases.
Otherwise preflight_refusals is empty, execution is a prefix of Complete phases followed by exactly
one Refused phase and a NotAttempted suffix. Each Refused phase has nonempty actual refusals and retains
its partial evidence; an open failure may leave every inner read NotAttempted. A failed second scan
thus retains the complete first scan. A failure after a hybrid SQL snapshot retains that snapshot
and both earlier side observations. Internal terminal failures must be named by the phase's refusals;
phase failures such as snapshot-close failure may occur after every inner enumeration completed.
Do not continue acquisition after a failed phase or retry within one observation. A new attempt is
a separate observation. Complete and Unstable contain the entire roster of Complete phases.
Every observed phase uses the evidence variant in its method roster; a complete Markdown phase root
equals the outer Markdown root or Hybrid local_root, and a complete SQL phase source equals the outer
SQL coordinate or Hybrid replica. Complete inner sources/namespaces cannot be Missing. Partial evidence
may omit unobtained coordinates, but any present coordinate must equal its corresponding outer value.

Catalog read order inside each SQL phase is Objects; then each discovered table by UTF-8 name, reading
TableDefinition, Columns, PrimaryKey, UniqueKeys, Checks in order; then Indexes and ForeignObjects.
The tables vector contains one record per discovered table header, including NotAttempted placeholders
when discovery or an earlier table read failed. A partial object enumeration makes no claim about
undiscovered tables. Table fields that have not been queried remain NotAttempted. After successful
catalog admission, row-family order is instances, events, history, legacy_origins, provider_sequences.
SQLite's final sequence family is NotApplicable; PostgreSQL has an observed sequence list. Across
catalog and rows there is at most one terminal failure; every later applicable read is NotAttempted.
A phase-level failure before a read can leave that read and its suffix NotAttempted, with the failure
at the phase level only. Successful empty data must never be used to fill these placeholders.

Catalog headers retain discovered object coordinates; dependent constraints/indexes name their table
as parent, independent objects use Missing. Partial columns, keys and checks retain every complete
record already read; malformed catalog rows retain exact cells beside the terminal coordinate.
Assemble SqlSchemaV1 only after every required catalog read is Complete, all declarations are admitted
and no rejected catalog row remains. Its tables/keys/checks/indexes must equal the observed records,
and header-to-object correspondence must be complete, unique and free of contradictory duplicates.
The header roster's exact backend queries remain an IO design obligation, not proof from the DTO.

Unstable exists only for MarkdownDoubleScan or HybridBracketedSqlSnapshot, with every phase Complete.
Compute its changed vector, never trust a supplied list: compare MarkdownFirst/Second for Markdown;
compare LocalBefore/After plus DivergencesBefore/After for Hybrid. Compare maps keyed by exact relative
host path, preserving directory presence, kind, bytes and symlink target. Include MarkdownPath once
for each added, removed or changed node; parent directories are included only if their own node differs.
For a differing divergence image (including absent versus empty), include Root(HybridSide(Divergences))
once. The exact canonical union must be nonempty and equal supplied changed. No SQL cross-snapshot
comparison or global chronology is invented. Equal compared images require Complete, not Unstable.
Complete's capture is deterministically reconstructed from the first complete local image, first
divergence image and SQL phase as appropriate; its supplied capture must equal that reconstruction.
Neither Refused nor Unstable gets a snapshot identity or becomes admitted merely because it hashes.

Complete declares complete physical acquisition, not complete semantic admission. Markdown acquisition
needs two equal complete tree scans; Hybrid brackets a nonmutating SQL snapshot with equal complete
local-tree/divergence scans. SQL acquisition needs its noninitializing read-only transaction. The pure
reader checks internal consistency only: a deserialized method or Complete tag is not proof that an
IO operation happened. Later acquisition must produce real evidence; apply also needs writer control.
No matching digest establishes point-in-time consistency, authenticity, a receipt or exclusion.

Raw observations are capture blobs. CLI results are separate content-free reports containing only
coordinates, counts, digests and named refusal codes. Inspect never mutates source, selector, destination,
projection or lock authority; any explicitly selected evidence-output sink must have a later reviewed
edge contract. This unit introduces no implicit inspect-time file writes or command surface.

## Complete physical values

```text
LegacyRawCaptureV1 = Markdown(MarkdownRawV1) | Sqlite(SqlRawV1)
 | Postgres(SqlRawV1) | Hybrid(HybridRawV1)
MarkdownRawV1 { nodes: Vec<MarkdownNodeV1> }
MarkdownNodeV1 { relative: HostPathV1, node: MarkdownNodeKindV1 }
MarkdownNodeKindV1 = Directory | Regular { bytes: HexBytesV1 }
 | Symlink { target: HostPathV1 } | Other { kind: OtherNodeKindV1 }
MarkdownNodeKindTagV1 = Directory | Regular | Symlink | Other
OtherNodeKindV1 = BlockDevice | CharacterDevice | Fifo | Socket | Unknown
FileImageV1 = Absent | Present { bytes: HexBytesV1 }
HybridRawV1 { policy: HybridPolicyWordsV1, local: MarkdownRawV1,
  replica: SqlRawV1, divergences: FileImageV1 }
HybridPolicyWordsV1 { authority: String, read: String, on_unreachable: String, on_divergence: String }
SqlRawV1 { dialect: SqlDialectV1, schema: SqlSchemaV1, instances: Vec<InstanceRowV1>,
  events: Vec<EventRowV1>, history: Vec<HistoryRowV1>, legacy_origins: Vec<LegacyOriginRowV1>,
  provider_sequences: SequenceRowsV1 }
InstanceRowV1 { entity: String, id: String, revision: i64, document: HexBytesV1 }
EventRowV1 { entity: String, id: String, revision: i64, position: i64, document: HexBytesV1 }
HistoryRowV1 { entity: String, id: String, position: i64, kind: HistoryKindV1,
  record_id: String, document: HexBytesV1 }
LegacyOriginRowV1 { entity: String, id: String, revision: i64 }
ProviderSequenceRowV1 { namespace: String, next_value: i64 }
SequenceRowsV1 = NotApplicable | Rows(Vec<ProviderSequenceRowV1>)
SqlDialectV1 = Sqlite | Postgres
HistoryKindV1 = Decision | Observation
SqlSchemaV1 { dialect: SqlDialectV1, namespace: String, tables: Vec<TableSchemaV1>,
  indexes: Vec<IndexSchemaV1>, foreign_objects: Vec<ForeignObjectV1> }
TableSchemaV1 { name: String, catalog_definition: PresenceV1<HexBytesV1>,
  columns: Vec<ColumnSchemaV1>, primary_key: PresenceV1<KeySchemaV1>,
  unique_keys: Vec<KeySchemaV1>, checks: Vec<CheckSchemaV1> }
ColumnSchemaV1 { ordinal: u16, name: String, sql_type: SqlTypeV1,
  declared_type: HexBytesV1, catalog_type_id: PresenceV1<String>, not_null: bool,
  default: PresenceV1<HexBytesV1>, explicit_collation: PresenceV1<String> }
SqlTypeV1 = Text | Integer | BigInt | Foreign(String)
KeySchemaV1 { name: PresenceV1<String>, backing_index: PresenceV1<String>,
  columns: Vec<String>, catalog_definition: PresenceV1<HexBytesV1> }
CheckSchemaV1 = HistoryKindDecisionObservation { name: PresenceV1<String>,
  catalog_expression: HexBytesV1, catalog_definition: PresenceV1<HexBytesV1> }
 | NextValueNonnegative { name: PresenceV1<String>, catalog_expression: HexBytesV1,
                         catalog_definition: PresenceV1<HexBytesV1> }
 | Foreign { name: PresenceV1<String>, catalog_expression: HexBytesV1,
             catalog_definition: PresenceV1<HexBytesV1> }
IndexSchemaV1 { name: String, table: String, unique: bool, method: IndexMethodV1,
  terms: Vec<IndexTermV1>, predicate: PresenceV1<HexBytesV1>, catalog_definition: HexBytesV1 }
IndexMethodV1 = Btree | Gin | Foreign(String)
IndexTermV1 = Column(String)
 | DocumentJsonbPathOps { catalog_expression: HexBytesV1, opclass: String } | Foreign(HexBytesV1)
ForeignObjectV1 { kind: ForeignObjectKindV1, name: String, owner: PresenceV1<String>,
  catalog_definition: PresenceV1<HexBytesV1> }
ForeignObjectKindV1 = Table | View | Trigger | Index | Constraint | Other(String)
```

Markdown inventories every descendant, retaining exact document/journal bytes and absent versus empty
files. Admission permits directories and kind/id `.md` documents plus `journal.jsonl`; a pending
`.aep-batch.pending.json`, foreign node or symlink refuses. Hybrid captures its divergence sidecar
separately and exactly once. Normal readers recover pending work or skip journal rows (observed
aep-backend-markdown/src/provider.rs:215-229,251-263,589-632); they cannot implement this capture.
All policy words are closed by validation: authority local|replica, read local-first|replica-first|
replica-only, on_unreachable refuse|serve-stale, on_divergence refuse|record. Both replica kinds and
all 48 policy combinations require both complete sides; no policy selects a migration winner.

Observed SQLite DDL (ER entity-sqlite/src/lib.rs:108-137): four tables, all columns NOT NULL:
instances(entity TEXT,id TEXT,revision INTEGER,document TEXT), PK(entity,id);
events(entity TEXT,id TEXT,revision INTEGER,position INTEGER,document TEXT), PK(entity,id,revision,position);
history(entity TEXT,id TEXT,position INTEGER,kind TEXT,record_id TEXT,document TEXT), PK(entity,id,position),
UNIQUE(record_id), CHECK(kind IN ('decision','observation')); legacy_origins(entity TEXT,id TEXT,
revision INTEGER), PK(entity,id). Namespace main, no defaults/explicit collations/independent indexes;
SequenceRows is NotApplicable. PostgreSQL (ER entity-postgres/src/lib.rs:182-217) uses BIGINT, adds
provider_sequences(namespace TEXT PRIMARY KEY,next_value BIGINT NOT NULL CHECK(next_value>=0)), and
instances_document_query GIN ((document::jsonb) jsonb_path_ops); SequenceRows is Rows.

Catalog acquisition must enumerate all selected-namespace tables, columns, keys, checks, indexes and
foreign objects. Known meaning never erases original catalog names, expressions or definitions.
Missing catalog metadata means the catalog supplies none, not that the capturer skipped the query.
Backing indexes appear through keys; independent indexes appear once in indexes. Unknown schema,
extra objects, unsupported constraints/features or unrepresentable facts refuse before Complete;
retain obtained catalog facts in refused evidence. Exact catalog queries and all admission comparisons
remain a separate required IO contract. A typed known variant is not itself proof of its catalog meaning.

Document columns retain exact retrieved UTF-8 TEXT bytes before parsing. Revision/position/next_value
must be nonnegative before checked u64 conversion. Known instance entities are exactly aep.entity,
aep.relation, aep.audit and aep.applied; unknown kinds refuse. Removed relations, refused audits and
applied records remain captured (observed aep-backend-entity/src/lib.rs:898-937). SQL position proves
only its source subject-local order, never a new cross-subject chronology.

## Exact refusals and coordinates

```text
CaptureRefusalV1 { code: CaptureRefusalCodeV1, at: PhysicalCoordinateV1 }
PhysicalCoordinateV1 = Root(RootCoordinateV1) | MarkdownPath { relative: HostPathV1 }
 | MarkdownSpan { relative: HostPathV1, start: u64, length: u64 }
 | SqlTable { table: String } | SqlRow { table: String, key: Vec<SqlKeyPartV1> }
 | SqlColumn { table: String, key: Vec<SqlKeyPartV1>, column: String }
 | SqlPhysicalRow { table: String, locator: SqlPhysicalLocatorV1 }
 | SqlCatalog { namespace: PresenceV1<String>, family: CatalogFamilyV1,
                table: PresenceV1<String>, row: PresenceV1<u64> }
SqlPhysicalLocatorV1 = SqliteRowId(i64) | PostgresTuple { table_oid: u32, block: u32, offset: u16 }
SqlKeyPartV1 = Text(String) | Integer(i64)
RootCoordinateV1 = Observation | Selector { project_root: HostPathV1, project_file: HostPathV1 }
 | EffectiveConfig | MarkdownRoot { root: HostPathV1 } | SqliteDatabase { database: HostPathV1 }
 | PostgresEndpoint { endpoint: PresenceV1<PostgresEndpointV1>, endpoint_id: PresenceV1<DigestV1> }
 | HybridSide { side: HybridSideV1 }
HybridSideV1 = Local | Replica | Divergences
```

The exact unit refusal tags are source_unreachable, unresolved_source_coordinate, read_failure,
foreign_markdown_node, pending_batch_present, invalid_relative_path, unsupported_schema,
unknown_physical_object, sql_type_mismatch, sql_null, duplicate_coordinate, non_utf8_text,
unknown_history_kind, foreign_instance_kind, numeric_out_of_range, hybrid_contradiction,
non_canonical_order, incompatible_variant and digest_mismatch. No free-form diagnostic string carries
raw source content. Schema/type errors remain structured, path-attributed reader errors.
Use root coordinates for resolution/unreachable failures; never invent a SQL table or Markdown path.
Malformed SQL primary-key cells can use an actually read physical row locator; these locators belong
only to refused evidence, are not portable identity or chronology, and are never fabricated. When a
row locator cannot be read, a table-level terminal refusal asserts no successful row acquisition.
SqlCatalog names the actual catalog family/table and, when obtained, the row ordinal of that query;
this ordinal belongs only to partial/refused catalog evidence and is never logical identity or order.
Absent row means failure before a particular row was obtained, not a fabricated zero ordinal.

## Canonical transcript

F(d,parts) = UTF8(d) || 00 || U64_BE(part_count) || each(U64_BE(part_length) || part).
D(d,parts) = SHA-256(F(d,parts)), exactly 32 bytes. H(d,parts) = `sha256:` + lowercase_hex(D).
Nested digest inputs are decoded bytes, never hex text. Length conversion is checked.

P is an explicitly ordered flat part list: structs emit fields in declaration order; enums emit
their fixed tag then payload fields in declaration order; newtype variants emit their value. Strings
emit UTF-8, bytes decoded bytes, digests decoded 32 bytes, bool 00/01, u16/u32/u64 fixed-width big-endian,
i64 two's-complement big-endian. A list emits U64_BE(count), then each P(item). Unit tags have no payload.
No display text, JSON serialization, host integer width or unordered collection feeds this grammar.

Each variant's fixed tag is its explicitly declared snake_case spelling, with these exact acronym
spellings: Sqlite=sqlite, Postgres=postgres, BigInt=big_int, SqliteRowId=sqlite_row_id,
MissingV1Default=missing_v1_default. This page's variant declarations fix field order. Generated
rename rules are not the authority: Rust must list literal serde renames and matching transcript tags.
The raw format scalar is special: it emits `aep.raw-capture/1`, not an enum envelope/tag.

Set vectors must arrive in canonical order; validation refuses disorder rather than sorting input.
Construction may sort acquired records before validating. Ordered vectors retain input order:
PostgreSQL hosts, key columns, index terms, Windows units and the fixed phase roster. Set order is Markdown path
by host variant (Unix before Windows) then native units lexicographically; tables/indexes by UTF-8 name;
columns/cells by ordinal; unique keys/checks/foreign objects by K(record); changed coordinates by K;
refusals by (K(at),UTF8(code-tag)); rejected rows by K(at); catalog headers by
(UTF8(kind-tag),UTF8(name),K(parent)). K(x)=F(`aep.migration.raw-sort-key/1`,P(x)).
Evidence Markdown nodes use their relative path; evidence row lists use the same keys as complete rows.
Catalog evidence tables use UTF-8 name order and their inner lists use the corresponding complete schema
orders. Duplicate catalog (kind,name,parent) coordinates refuse; headers for table objects have Missing
parent. Each divergence phase contains one observed value, never a made-up empty array. SQL row keys are instances(entity,id),
events(entity,id,revision,position), history(entity,id,position), origins(entity,id), sequences(namespace),
using UTF-8 string and signed numeric comparisons. Duplicate primary coordinates, column names/ordinals,
identical set entries or duplicate key column sequences refuse. Column/constraint name uniqueness is
per table; index/backing-index name uniqueness is per namespace. Do not reject equal constraint names
on different tables merely because the spelling matches.

Complete requires the exact phase roster and reconstruction/comparison rules above, compatible
source/method/capture/dialect/namespace/sequence variants, nonempty
Postgres hosts/database/schema, exact endpoint digest, valid source-relative coordinates, closed policy
words, canonical ordered rows/schema and recomputed digests. No complete capture contains refused
evidence or foreign/malformed/pending objects. Unstable/Refused validate the same exact phase and inner
read-prefix rules, compatibility, order, digests and actual difference/failure facts; unavailable outer
coordinates are never made up. A scalar metadata Missing value is distinct from a NotAttempted query.
Hybrid source coordinates retain the resolved policy explicitly as Present, or Missing when it was
not obtained. Complete/Unstable hybrid observations require Present with the four admitted policy
words; Complete reconstructs HybridRawV1.policy from that value and refuses a different supplied policy.
Refused observations retain any obtained resolved policy even after a later scan/SQL failure. Missing
policy permits only a preflight Refused result with every known-method phase NotAttempted; no hybrid
acquisition phase starts without the resolved policy. P(SourceCoordinateV1::Hybrid) emits local_root,
replica, divergence_file, then policy-presence in that exact order, so policy is bound by the complete
transcript as well as reconstructed capture. Test all48 combinations, Missing policy, contradictory
capture policy, and policy retention after every hybrid failed phase. Markdown fixture parts are unchanged.

selector_digest = H(`aep.migration.project-selector/1`,[exact selector file bytes]).
config_digest = H(`aep.migration.source-config/1`,[exact effective private config bytes]); these private
bytes are ephemeral. Endpoint ID = H(`aep.migration.postgres-endpoint/1`,P(endpoint)), without user,
password or DSN. Exact effective-input acquisition and credential-free resolution remain IO obligations.
Each phase evidence_digest = H(`aep.migration.raw-observation-evidence/1`,P(PhaseEvidenceV1)).
Complete transcript = F(`aep.migration.raw-transcript/1`,[UTF8(format)] || P(source-presence) ||
P(unwrapped-present-selector) || [decoded-present-config-digest] || P(capture)). The selector/config
presence wrappers are deliberately excluded here; their presence is required before this operation.
transcript_digest = `sha256:` + lowercase_hex(SHA-256(transcript)).
raw_snapshot_id = H(`aep.migration.raw-snapshot/1`,[decoded(config_digest), selector-binding, transcript]);
selector-binding is decoded selector_digest, or D(`aep.migration.missing-selector/1`,[]) for missing v1.
Outcome tags, observation method, phase records and output digest fields do not enter the complete
transcript: this identifies the complete raw snapshot contents, while phase evidence retains all reads.

## Independent fixture constants and admission tests

These constants are proposed fixtures, not executed Rust evidence. Test exact F bytes as well as hashes:
F(`aep.migration.test/1`,[]) hex = 6165702e6d6967726174696f6e2e746573742f31000000000000000000;
H = sha256:355279afe5bea97f0fec16a7f42e45c268a0dd0c7e8b6db35d3295cbc9af27ed.
F(`aep.migration.test/1`,[empty,00ff]) hex =
6165702e6d6967726174696f6e2e746573742f310000000000000000020000000000000000000000000000000200ff;
H = sha256:381a16f9663dd619850ee3f3e2c78f3bf95f309477e6164ac0a3d112632371cd.

A complete synthetic Markdown fixture has present source Markdown root Unix bytes `planning`, present
selector {project_root Unix `repo`, project_file Unix `.engineering/project.yaml`, presence MissingV1Default,
store_field MissingDefault}, present config digest of the empty byte string, and empty Markdown nodes.
Its method is MarkdownDoubleScan, and its two phases are MarkdownFirst and MarkdownSecond, both
Complete with equal Markdown evidence {root Present(Unix `planning`), nodes {items [], terminal Complete}}.
Each phase evidence digest uses the literal six parts `markdown`, `present`, `unix`, `planning`,
U64_BE(0), `complete` under the phase evidence domain. Phases are required in the observation but do
not change the complete snapshot transcript. Config digest hex is
88bc6280eaaf4014465569ec070c24ce635e811325635367a1d2edb8088f3345; missing-selector digest hex is
2fb9732ed613a9fec59f7d99cf98116a6517d32e72c2c951e655fdfa159e20ba. Its 14 transcript parts are:
`aep.raw-capture/1`, `present`, `markdown`, `unix`, `planning`, `unix`, `repo`, `unix`,
`.engineering/project.yaml`, `missing_v1_default`, `missing_default`, decoded config digest,
`markdown`, U64_BE(0). Transcript hash hex is
6a0d756282c097062cf05f6bd5b22b4adecff953d4b6d3167b27485622dd8fba; snapshot hash hex is
c6d643e43c970d7ec01da3f4e1de38a9ebf7480c683ac907d24497f6ccb5a502.
Retain literal independent expected bytes for this full fixture; do not derive the expected transcript
through the implementation under test. Exercise both path variants and all concrete enum variants.

Tests must cover required/missing/null/unknown/duplicate members and tag payloads, canonical scalars,
wrong format, malformed hex/digests, source/method/dialect contradictions, absent versus empty files,
partial enumeration, failed source resolution, retained malformed SQL cells, foreign catalog facts,
all canonical collection rules, changed raw bytes/coordinates/config/selector and nested digest bytes.
For each method exercise every failure phase with its successful prefix and NotAttempted suffix;
specifically retain first-scan bytes after second-scan failure and SQL bytes after a later hybrid failure.
Exercise failure inside each catalog family and each table subquery, preserving earlier records and
keeping absent metadata distinct from skipped queries. Mutating a NotAttempted field to empty Complete,
dropping a successful phase, adding a phase, or replacing the exact changed set must fail named cases.
Path fixtures cover Unix backslash/colon, UTF-8 boundaries, Windows surrogate pairs/unpaired surrogates,
both separator rules, NUL, dot components, drive/UNC/device prefixes and reserved Windows stems.
Schema and reader must agree on wire structure; validation separately tests semantic invariants.
Complete cannot be minted from a partial/refused observation or from digest equality alone.

Backend semantic admission stays separate: preserve exact JSON numbers and duplicate-key information
before strict typed parsing. Observed DomainEvent permits missing or null from_state
(entity-core/src/runtime.rs:47-94). Legacy Entry and DomainEvent need closed mirrors, not normal readers
that discard lines; a serde_json::Value roundtrip cannot prove original numeric fidelity. Original
SQL relation IDs, migration-owned Markdown relation mappings, full history envelopes versus bare
events and incomplete boundaries still require the owner’s later semantic/import contract.

## Implementation and remaining dependencies

After independent technical review, the pure unit may add migration modules/tests and lib export to
aep-contract; SHA dependency (existing workspace crates use sha2 0.11); aep-contract wiring and the
RawCaptureObservation schema entry in aep-schema; generated schema/index outputs and rejection fixtures.
Preserve existing schema bytes and pure Rust1.85 minima. Run exact contract/schema tests, schema-check,
formatting, strict Clippy and the pure MSRV lane; full AEP task check remains required before integration.

This unit does not implement catalog queries, nonmutating SQLite/PostgreSQL acquisition, credential-free
endpoint discovery, operational writer control, phase records, semantic mappings/import, CLI reports,
qualified ER/Eventlog runtime or six cutovers. These remain required under the single owner. No force,
attestation, assumed quiescence or fake runtime can clear WriterExclusionUnavailable. The existing
runtime dependency blocker remains open; pure-value work is independent of that missing runtime.
