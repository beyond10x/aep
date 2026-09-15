//! Closed provider-neutral values for physical planning-store capture.
//!
//! The types in this module carry observations only. They perform no filesystem, database,
//! environment, clock, or network operation. [`RawCaptureObservationV1::validate`] checks the
//! relationships between those observations and accumulates stable code/path diagnostics.

#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use schemars::schema::{InstanceType, ObjectValidation, Schema, SchemaObject, SubschemaValidation};
use schemars::JsonSchema;
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest as _, Sha256};

use super::digest::{
    digest_parts_v1, frame_v1, sort_key_v1, CanonicalPartsV1, DigestV1, FrameError, HexBytesV1,
    RawCaptureFormatV1,
};

#[derive(Debug)]
struct StrictAdjacent {
    kind: String,
    value: Option<CheckedJsonValue>,
}

#[derive(Debug)]
struct CheckedJsonValue(serde_json::Value);

impl<'de> Deserialize<'de> for CheckedJsonValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct CheckedVisitor;

        impl<'de> Visitor<'de> for CheckedVisitor {
            type Value = CheckedJsonValue;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON-compatible value with unique object keys")
            }

            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(CheckedJsonValue(serde_json::Value::Bool(value)))
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(CheckedJsonValue(value.into()))
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(CheckedJsonValue(value.into()))
            }

            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(serde_json::Value::Number)
                    .map(CheckedJsonValue)
                    .ok_or_else(|| E::custom("non-finite JSON number"))
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                self.visit_string(value.to_owned())
            }

            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(CheckedJsonValue(serde_json::Value::String(value)))
            }

            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(CheckedJsonValue(serde_json::Value::Null))
            }

            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(CheckedJsonValue(serde_json::Value::Null))
            }

            fn visit_some<D: Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                CheckedJsonValue::deserialize(deserializer)
            }

            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = sequence.next_element::<CheckedJsonValue>()? {
                    values.push(value.0);
                }
                Ok(CheckedJsonValue(serde_json::Value::Array(values)))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom(format!("duplicate field {key:?}")));
                    }
                    values.insert(key, map.next_value::<CheckedJsonValue>()?.0);
                }
                Ok(CheckedJsonValue(serde_json::Value::Object(values)))
            }
        }

        deserializer.deserialize_any(CheckedVisitor)
    }
}

impl<'de> Deserialize<'de> for StrictAdjacent {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct AdjacentVisitor;

        impl<'de> Visitor<'de> for AdjacentVisitor {
            type Value = StrictAdjacent;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed adjacent enum envelope")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut kind = None;
                let mut value = None;
                let mut value_seen = false;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "kind" => {
                            if kind.is_some() {
                                return Err(serde::de::Error::duplicate_field("kind"));
                            }
                            kind = Some(map.next_value()?);
                        }
                        "value" => {
                            if value_seen {
                                return Err(serde::de::Error::duplicate_field("value"));
                            }
                            value_seen = true;
                            value = Some(map.next_value()?);
                        }
                        other => {
                            return Err(serde::de::Error::unknown_field(other, &["kind", "value"]));
                        }
                    }
                }
                Ok(StrictAdjacent {
                    kind: kind.ok_or_else(|| serde::de::Error::missing_field("kind"))?,
                    value,
                })
            }
        }

        deserializer.deserialize_map(AdjacentVisitor)
    }
}

macro_rules! decode_variant {
    ($value:expr, $constructor:path) => {
        match $value {
            None => Ok($constructor),
            Some(_) => Err(serde::de::Error::custom(
                "unit variant must not contain value",
            )),
        }
    };
    ($value:expr, $constructor:path, $payload:ty) => {
        match $value {
            Some(value) => serde_json::from_value::<$payload>(value.0)
                .map($constructor)
                .map_err(serde::de::Error::custom),
            None => Err(serde::de::Error::missing_field("value")),
        }
    };
}

macro_rules! variant_schema {
    ($generator:expr, $tag:expr) => {
        adjacent_variant_schema($tag, None)
    };
    ($generator:expr, $tag:expr, $payload:ty) => {
        adjacent_variant_schema($tag, Some($generator.subschema_for::<$payload>()))
    };
}

macro_rules! append_variant_payload {
    ($this:expr, $parts:expr, $name:ident, $variant:ident) => {};
    ($this:expr, $parts:expr, $name:ident, $variant:ident, $payload:ty) => {
        if let $name::$variant(value) = $this {
            value.append_parts($parts);
        }
    };
}

macro_rules! set_variant_tag {
    ($this:expr, $tag_slot:expr, $name:ident, $variant:ident, $tag:literal) => {
        if matches!($this, $name::$variant) {
            $tag_slot = Some($tag);
        }
    };
    ($this:expr, $tag_slot:expr, $name:ident, $variant:ident, $tag:literal, $payload:ty) => {
        if matches!($this, $name::$variant(_)) {
            $tag_slot = Some($tag);
        }
    };
}

macro_rules! closed_enum {
    (
        $name:ident {
            $( $variant:ident $(($payload:ty))? => $tag:literal ),+ $(,)?
        }
    ) => {
        // The direct payload keeps construction and matching aligned with the published wire DTO.
        #[allow(clippy::large_enum_variant)]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
        #[serde(tag = "kind", content = "value")]
        pub enum $name {
            $(
                #[serde(rename = $tag)]
                $variant $(($payload))?,
            )+
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let envelope = StrictAdjacent::deserialize(deserializer)?;
                match envelope.kind.as_str() {
                    $(
                        $tag => decode_variant!(
                            envelope.value,
                            Self::$variant
                            $(, $payload)?
                        ),
                    )+
                    other => Err(serde::de::Error::unknown_variant(other, &[$($tag),+])),
                }
            }
        }

        impl JsonSchema for $name {
            fn schema_name() -> String {
                stringify!($name).to_owned()
            }

            fn json_schema(generator: &mut schemars::gen::SchemaGenerator) -> Schema {
                let _ = &generator;
                let variants = vec![
                    $(variant_schema!(generator, $tag $(, $payload)?)),+
                ];
                SchemaObject {
                    subschemas: Some(Box::new(SubschemaValidation {
                        one_of: Some(variants),
                        ..SubschemaValidation::default()
                    })),
                    ..SchemaObject::default()
                }
                .into()
            }
        }

        impl CanonicalPartsV1 for $name {
            fn append_parts(&self, parts: &mut Vec<Vec<u8>>) {
                let mut tag = None;
                $(set_variant_tag!(self, tag, $name, $variant, $tag $(, $payload)?);)+
                let tag = tag.expect("every declared enum variant has a canonical tag");
                parts.push(tag.as_bytes().to_vec());
                $(append_variant_payload!(self, parts, $name, $variant $(, $payload)?);)+
            }
        }
    };
}

macro_rules! wire_struct {
    ($name:ident { $( $field:ident : $type:ty ),* $(,)? }) => {
        #[derive(
            Debug,
            Clone,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Serialize,
            Deserialize,
            JsonSchema,
        )]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            $(pub $field: $type,)*
        }

        impl CanonicalPartsV1 for $name {
            fn append_parts(&self, parts: &mut Vec<Vec<u8>>) {
                $(self.$field.append_parts(parts);)*
            }
        }
    };
}

fn adjacent_variant_schema(tag: &str, payload: Option<Schema>) -> Schema {
    let mut tag_schema = SchemaObject {
        instance_type: Some(InstanceType::String.into()),
        enum_values: Some(vec![serde_json::Value::String(tag.to_owned())]),
        ..SchemaObject::default()
    };
    tag_schema.metadata().description = Some("The explicitly declared variant tag.".to_owned());

    let mut object = ObjectValidation::default();
    object
        .properties
        .insert("kind".to_owned(), tag_schema.into());
    object.required.insert("kind".to_owned());
    if let Some(payload) = payload {
        object.properties.insert("value".to_owned(), payload);
        object.required.insert("value".to_owned());
    }
    object.additional_properties = Some(Box::new(Schema::Bool(false)));
    SchemaObject {
        instance_type: Some(InstanceType::Object.into()),
        object: Some(Box::new(object)),
        ..SchemaObject::default()
    }
    .into()
}

/// Explicit missing-or-present transport value. JSON null is never an absence spelling.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum PresenceV1<T> {
    #[serde(rename = "missing")]
    Missing,
    #[serde(rename = "present")]
    Present(T),
}

impl<'de, T> Deserialize<'de> for PresenceV1<T>
where
    T: serde::de::DeserializeOwned,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let envelope = StrictAdjacent::deserialize(deserializer)?;
        match envelope.kind.as_str() {
            "missing" => decode_variant!(envelope.value, Self::Missing),
            "present" => decode_variant!(envelope.value, Self::Present, T),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &["missing", "present"],
            )),
        }
    }
}

impl<T: JsonSchema> JsonSchema for PresenceV1<T> {
    fn schema_name() -> String {
        format!("PresenceV1_of_{}", T::schema_name())
    }

    fn json_schema(generator: &mut schemars::gen::SchemaGenerator) -> Schema {
        SchemaObject {
            subschemas: Some(Box::new(SubschemaValidation {
                one_of: Some(vec![
                    adjacent_variant_schema("missing", None),
                    adjacent_variant_schema("present", Some(generator.subschema_for::<T>())),
                ]),
                ..SubschemaValidation::default()
            })),
            ..SchemaObject::default()
        }
        .into()
    }
}

impl<T: CanonicalPartsV1> CanonicalPartsV1 for PresenceV1<T> {
    fn append_parts(&self, parts: &mut Vec<Vec<u8>>) {
        match self {
            Self::Missing => parts.push(b"missing".to_vec()),
            Self::Present(value) => {
                parts.push(b"present".to_vec());
                value.append_parts(parts);
            }
        }
    }
}

wire_struct!(RawCaptureObservationV1 {
    format: RawCaptureFormatV1,
    source: PresenceV1<SourceCoordinateV1>,
    selector: PresenceV1<SelectorCoordinateV1>,
    config_digest: PresenceV1<DigestV1>,
    observation: ObservationOutcomeV1,
});

wire_struct!(SelectorCoordinateV1 {
    project_root: HostPathV1,
    project_file: HostPathV1,
    presence: SelectorPresenceV1,
    store_field: StoreFieldV1,
});

wire_struct!(SelectorPresentV1 {
    selector_digest: DigestV1,
});

closed_enum!(SelectorPresenceV1 {
    MissingV1Default => "missing_v1_default",
    Present(SelectorPresentV1) => "present",
});

closed_enum!(StoreFieldV1 {
    MissingDefault => "missing_default",
    Present => "present",
});

wire_struct!(MarkdownSourceCoordinateV1 { root: HostPathV1 });
wire_struct!(SqliteSourceCoordinateV1 {
    database: HostPathV1
});
wire_struct!(PostgresSourceCoordinateV1 {
    endpoint: PostgresEndpointV1,
    endpoint_id: DigestV1,
});
wire_struct!(HybridSourceCoordinateV1 {
    local_root: HostPathV1,
    replica: SqlReplicaCoordinateV1,
    divergence_file: HostPathV1,
    policy: PresenceV1<HybridPolicyWordsV1>,
});

closed_enum!(SourceCoordinateV1 {
    Markdown(MarkdownSourceCoordinateV1) => "markdown",
    Sqlite(SqliteSourceCoordinateV1) => "sqlite",
    Postgres(PostgresSourceCoordinateV1) => "postgres",
    Hybrid(HybridSourceCoordinateV1) => "hybrid",
});

wire_struct!(SqliteReplicaCoordinateV1 {
    database: HostPathV1
});
wire_struct!(PostgresReplicaCoordinateV1 {
    endpoint: PostgresEndpointV1,
    endpoint_id: DigestV1,
});

closed_enum!(SqlReplicaCoordinateV1 {
    Sqlite(SqliteReplicaCoordinateV1) => "sqlite",
    Postgres(PostgresReplicaCoordinateV1) => "postgres",
});

wire_struct!(PostgresEndpointV1 {
    hosts: Vec<PostgresHostV1>,
    database: String,
    schema: String,
});
wire_struct!(TcpPostgresHostV1 {
    host: String,
    port: u16
});
wire_struct!(UnixPostgresHostV1 {
    directory: HostPathV1,
    port: u16,
});

closed_enum!(PostgresHostV1 {
    Tcp(TcpPostgresHostV1) => "tcp",
    Unix(UnixPostgresHostV1) => "unix",
});

closed_enum!(HostPathV1 {
    Unix(HexBytesV1) => "unix",
    Windows(Vec<u16>) => "windows",
});

closed_enum!(ObservationMethodV1 {
    MarkdownDoubleScan => "markdown_double_scan",
    SqliteReadTransaction => "sqlite_read_transaction",
    PostgresRepeatableReadOnly => "postgres_repeatable_read_only",
    HybridBracketedSqlSnapshot => "hybrid_bracketed_sql_snapshot",
});

wire_struct!(CompleteObservationV1 {
    method: ObservationMethodV1,
    phases: Vec<PhaseObservationV1>,
    capture: LegacyRawCaptureV1,
    transcript_digest: DigestV1,
    raw_snapshot_id: DigestV1,
});
wire_struct!(UnstableObservationV1 {
    method: ObservationMethodV1,
    phases: Vec<PhaseObservationV1>,
    changed: Vec<PhysicalCoordinateV1>,
});
wire_struct!(RefusedObservationV1 {
    method: PresenceV1<ObservationMethodV1>,
    phases: Vec<PhaseObservationV1>,
    preflight_refusals: Vec<CaptureRefusalV1>,
});

closed_enum!(ObservationOutcomeV1 {
    Complete(CompleteObservationV1) => "complete",
    Unstable(UnstableObservationV1) => "unstable",
    Refused(RefusedObservationV1) => "refused",
});

wire_struct!(PhaseObservationV1 {
    phase: CapturePhaseV1,
    result: PhaseResultV1,
});

closed_enum!(CapturePhaseV1 {
    MarkdownFirst => "markdown_first",
    MarkdownSecond => "markdown_second",
    SqlSnapshot => "sql_snapshot",
    LocalBefore => "local_before",
    DivergencesBefore => "divergences_before",
    ReplicaSnapshot => "replica_snapshot",
    LocalAfter => "local_after",
    DivergencesAfter => "divergences_after",
});

wire_struct!(CompletePhaseResultV1 {
    evidence: PhaseEvidenceV1,
    evidence_digest: DigestV1,
});
wire_struct!(RefusedPhaseResultV1 {
    evidence: PhaseEvidenceV1,
    evidence_digest: DigestV1,
    refusals: Vec<CaptureRefusalV1>,
});

closed_enum!(PhaseResultV1 {
    NotAttempted => "not_attempted",
    Complete(CompletePhaseResultV1) => "complete",
    Refused(RefusedPhaseResultV1) => "refused",
});

closed_enum!(PhaseEvidenceV1 {
    Markdown(MarkdownEvidenceV1) => "markdown",
    Sqlite(SqlEvidenceV1) => "sqlite",
    Postgres(SqlEvidenceV1) => "postgres",
    Divergences(ObservedValueV1<FileImageV1>) => "divergences",
});

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservedListV1<T> {
    pub items: Vec<T>,
    pub terminal: EnumerationTerminalV1,
}

impl<T: CanonicalPartsV1> CanonicalPartsV1 for ObservedListV1<T> {
    fn append_parts(&self, parts: &mut Vec<Vec<u8>>) {
        self.items.append_parts(parts);
        self.terminal.append_parts(parts);
    }
}

wire_struct!(EnumerationRefusalV1 {
    at: PhysicalCoordinateV1,
    code: CaptureRefusalCodeV1,
});

closed_enum!(EnumerationTerminalV1 {
    NotAttempted => "not_attempted",
    Complete => "complete",
    Refused(EnumerationRefusalV1) => "refused",
});

wire_struct!(ObservedValueRefusalV1 {
    at: PhysicalCoordinateV1,
    code: CaptureRefusalCodeV1,
});

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum ObservedValueV1<T> {
    #[serde(rename = "not_attempted")]
    NotAttempted,
    #[serde(rename = "complete")]
    Complete(T),
    #[serde(rename = "refused")]
    Refused(ObservedValueRefusalV1),
}

impl<'de, T> Deserialize<'de> for ObservedValueV1<T>
where
    T: serde::de::DeserializeOwned,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let envelope = StrictAdjacent::deserialize(deserializer)?;
        match envelope.kind.as_str() {
            "not_attempted" => decode_variant!(envelope.value, Self::NotAttempted),
            "complete" => decode_variant!(envelope.value, Self::Complete, T),
            "refused" => decode_variant!(envelope.value, Self::Refused, ObservedValueRefusalV1),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &["not_attempted", "complete", "refused"],
            )),
        }
    }
}

impl<T: JsonSchema> JsonSchema for ObservedValueV1<T> {
    fn schema_name() -> String {
        format!("ObservedValueV1_of_{}", T::schema_name())
    }

    fn json_schema(generator: &mut schemars::gen::SchemaGenerator) -> Schema {
        SchemaObject {
            subschemas: Some(Box::new(SubschemaValidation {
                one_of: Some(vec![
                    adjacent_variant_schema("not_attempted", None),
                    adjacent_variant_schema("complete", Some(generator.subschema_for::<T>())),
                    adjacent_variant_schema(
                        "refused",
                        Some(generator.subschema_for::<ObservedValueRefusalV1>()),
                    ),
                ]),
                ..SubschemaValidation::default()
            })),
            ..SchemaObject::default()
        }
        .into()
    }
}

impl<T: CanonicalPartsV1> CanonicalPartsV1 for ObservedValueV1<T> {
    fn append_parts(&self, parts: &mut Vec<Vec<u8>>) {
        match self {
            Self::NotAttempted => parts.push(b"not_attempted".to_vec()),
            Self::Complete(value) => {
                parts.push(b"complete".to_vec());
                value.append_parts(parts);
            }
            Self::Refused(value) => {
                parts.push(b"refused".to_vec());
                value.append_parts(parts);
            }
        }
    }
}

wire_struct!(MarkdownEvidenceV1 {
    root: PresenceV1<HostPathV1>,
    nodes: ObservedListV1<MarkdownNodeEvidenceV1>,
});

wire_struct!(UnreadableMarkdownNodeV1 {
    relative: HostPathV1,
    metadata: PresenceV1<NodeMetadataV1>,
});

closed_enum!(MarkdownNodeEvidenceV1 {
    Captured(MarkdownNodeV1) => "captured",
    Unreadable(UnreadableMarkdownNodeV1) => "unreadable",
});

wire_struct!(NodeMetadataV1 {
    kind: MarkdownNodeKindTagV1,
    length: PresenceV1<u64>,
});

wire_struct!(SqlEvidenceV1 {
    source: PresenceV1<SqlReplicaCoordinateV1>,
    catalog: SqlCatalogEvidenceV1,
    instances: ObservedListV1<InstanceRowV1>,
    events: ObservedListV1<EventRowV1>,
    history: ObservedListV1<HistoryRowV1>,
    legacy_origins: ObservedListV1<LegacyOriginRowV1>,
    provider_sequences: ObservedSequenceRowsV1,
    rejected_rows: Vec<RejectedSqlRowV1>,
});

closed_enum!(ObservedSequenceRowsV1 {
    NotApplicable => "not_applicable",
    Observed(ObservedListV1<ProviderSequenceRowV1>) => "observed",
});

wire_struct!(RejectedSqlRowV1 {
    at: PhysicalCoordinateV1,
    cells: Vec<SqlCellImageV1>,
});
wire_struct!(SqlCellImageV1 {
    ordinal: u16,
    column: String,
    value: SqlCellValueV1,
});
wire_struct!(PostgresBinaryCellV1 {
    type_id: String,
    bytes: HexBytesV1,
});

closed_enum!(SqlCellValueV1 {
    Null => "null",
    Integer(i64) => "integer",
    RealBits(u64) => "real_bits",
    Text(HexBytesV1) => "text",
    Blob(HexBytesV1) => "blob",
    PostgresBinary(PostgresBinaryCellV1) => "postgres_binary",
});

wire_struct!(SqlCatalogEvidenceV1 {
    namespace: PresenceV1<String>,
    objects: ObservedListV1<CatalogObjectHeaderV1>,
    tables: Vec<TableCatalogEvidenceV1>,
    indexes: ObservedListV1<IndexSchemaV1>,
    foreign_objects: ObservedListV1<ForeignObjectV1>,
    rejected_rows: Vec<RejectedCatalogRowV1>,
});
wire_struct!(CatalogObjectHeaderV1 {
    kind: ForeignObjectKindV1,
    name: String,
    parent: PresenceV1<String>,
});
wire_struct!(TableCatalogEvidenceV1 {
    name: String,
    catalog_definition: ObservedValueV1<PresenceV1<HexBytesV1>>,
    columns: ObservedListV1<ColumnSchemaV1>,
    primary_key: ObservedValueV1<PresenceV1<KeySchemaV1>>,
    unique_keys: ObservedListV1<KeySchemaV1>,
    checks: ObservedListV1<CheckSchemaV1>,
});
wire_struct!(RejectedCatalogRowV1 {
    at: PhysicalCoordinateV1,
    cells: Vec<SqlCellImageV1>,
});

closed_enum!(CatalogFamilyV1 {
    Objects => "objects",
    TableDefinition => "table_definition",
    Columns => "columns",
    PrimaryKey => "primary_key",
    UniqueKeys => "unique_keys",
    Checks => "checks",
    Indexes => "indexes",
    ForeignObjects => "foreign_objects",
});

wire_struct!(MarkdownRawV1 { nodes: Vec<MarkdownNodeV1> });
wire_struct!(MarkdownNodeV1 {
    relative: HostPathV1,
    node: MarkdownNodeKindV1,
});
wire_struct!(RegularMarkdownNodeV1 { bytes: HexBytesV1 });
wire_struct!(SymlinkMarkdownNodeV1 { target: HostPathV1 });
wire_struct!(OtherMarkdownNodeV1 {
    kind: OtherNodeKindV1
});

closed_enum!(MarkdownNodeKindV1 {
    Directory => "directory",
    Regular(RegularMarkdownNodeV1) => "regular",
    Symlink(SymlinkMarkdownNodeV1) => "symlink",
    Other(OtherMarkdownNodeV1) => "other",
});

closed_enum!(MarkdownNodeKindTagV1 {
    Directory => "directory",
    Regular => "regular",
    Symlink => "symlink",
    Other => "other",
});

closed_enum!(OtherNodeKindV1 {
    BlockDevice => "block_device",
    CharacterDevice => "character_device",
    Fifo => "fifo",
    Socket => "socket",
    Unknown => "unknown",
});

wire_struct!(PresentFileImageV1 { bytes: HexBytesV1 });
closed_enum!(FileImageV1 {
    Absent => "absent",
    Present(PresentFileImageV1) => "present",
});

wire_struct!(HybridRawV1 {
    policy: HybridPolicyWordsV1,
    local: MarkdownRawV1,
    replica: SqlRawV1,
    divergences: FileImageV1,
});
wire_struct!(HybridPolicyWordsV1 {
    authority: String,
    read: String,
    on_unreachable: String,
    on_divergence: String,
});

closed_enum!(LegacyRawCaptureV1 {
    Markdown(MarkdownRawV1) => "markdown",
    Sqlite(SqlRawV1) => "sqlite",
    Postgres(SqlRawV1) => "postgres",
    Hybrid(HybridRawV1) => "hybrid",
});

wire_struct!(SqlRawV1 {
    dialect: SqlDialectV1,
    schema: SqlSchemaV1,
    instances: Vec<InstanceRowV1>,
    events: Vec<EventRowV1>,
    history: Vec<HistoryRowV1>,
    legacy_origins: Vec<LegacyOriginRowV1>,
    provider_sequences: SequenceRowsV1,
});
wire_struct!(InstanceRowV1 {
    entity: String,
    id: String,
    revision: i64,
    document: HexBytesV1,
});
wire_struct!(EventRowV1 {
    entity: String,
    id: String,
    revision: i64,
    position: i64,
    document: HexBytesV1,
});
wire_struct!(HistoryRowV1 {
    entity: String,
    id: String,
    position: i64,
    kind: HistoryKindV1,
    record_id: String,
    document: HexBytesV1,
});
wire_struct!(LegacyOriginRowV1 {
    entity: String,
    id: String,
    revision: i64,
});
wire_struct!(ProviderSequenceRowV1 {
    namespace: String,
    next_value: i64,
});

closed_enum!(SequenceRowsV1 {
    NotApplicable => "not_applicable",
    Rows(Vec<ProviderSequenceRowV1>) => "rows",
});

closed_enum!(SqlDialectV1 {
    Sqlite => "sqlite",
    Postgres => "postgres",
});

closed_enum!(HistoryKindV1 {
    Decision => "decision",
    Observation => "observation",
});

wire_struct!(SqlSchemaV1 {
    dialect: SqlDialectV1,
    namespace: String,
    tables: Vec<TableSchemaV1>,
    indexes: Vec<IndexSchemaV1>,
    foreign_objects: Vec<ForeignObjectV1>,
});
wire_struct!(TableSchemaV1 {
    name: String,
    catalog_definition: PresenceV1<HexBytesV1>,
    columns: Vec<ColumnSchemaV1>,
    primary_key: PresenceV1<KeySchemaV1>,
    unique_keys: Vec<KeySchemaV1>,
    checks: Vec<CheckSchemaV1>,
});
wire_struct!(ColumnSchemaV1 {
    ordinal: u16,
    name: String,
    sql_type: SqlTypeV1,
    declared_type: HexBytesV1,
    catalog_type_id: PresenceV1<String>,
    not_null: bool,
    default: PresenceV1<HexBytesV1>,
    explicit_collation: PresenceV1<String>,
});

closed_enum!(SqlTypeV1 {
    Text => "text",
    Integer => "integer",
    BigInt => "big_int",
    Foreign(String) => "foreign",
});

wire_struct!(KeySchemaV1 {
    name: PresenceV1<String>,
    backing_index: PresenceV1<String>,
    columns: Vec<String>,
    catalog_definition: PresenceV1<HexBytesV1>,
});
wire_struct!(HistoryCheckSchemaV1 {
    name: PresenceV1<String>,
    catalog_expression: HexBytesV1,
    catalog_definition: PresenceV1<HexBytesV1>,
});
wire_struct!(NextValueCheckSchemaV1 {
    name: PresenceV1<String>,
    catalog_expression: HexBytesV1,
    catalog_definition: PresenceV1<HexBytesV1>,
});
wire_struct!(ForeignCheckSchemaV1 {
    name: PresenceV1<String>,
    catalog_expression: HexBytesV1,
    catalog_definition: PresenceV1<HexBytesV1>,
});

closed_enum!(CheckSchemaV1 {
    HistoryKindDecisionObservation(HistoryCheckSchemaV1) => "history_kind_decision_observation",
    NextValueNonnegative(NextValueCheckSchemaV1) => "next_value_nonnegative",
    Foreign(ForeignCheckSchemaV1) => "foreign",
});

wire_struct!(IndexSchemaV1 {
    name: String,
    table: String,
    unique: bool,
    method: IndexMethodV1,
    terms: Vec<IndexTermV1>,
    predicate: PresenceV1<HexBytesV1>,
    catalog_definition: HexBytesV1,
});

closed_enum!(IndexMethodV1 {
    Btree => "btree",
    Gin => "gin",
    Foreign(String) => "foreign",
});

wire_struct!(DocumentJsonbPathOpsV1 {
    catalog_expression: HexBytesV1,
    opclass: String,
});
closed_enum!(IndexTermV1 {
    Column(String) => "column",
    DocumentJsonbPathOps(DocumentJsonbPathOpsV1) => "document_jsonb_path_ops",
    Foreign(HexBytesV1) => "foreign",
});

wire_struct!(ForeignObjectV1 {
    kind: ForeignObjectKindV1,
    name: String,
    owner: PresenceV1<String>,
    catalog_definition: PresenceV1<HexBytesV1>,
});

closed_enum!(ForeignObjectKindV1 {
    Table => "table",
    View => "view",
    Trigger => "trigger",
    Index => "index",
    Constraint => "constraint",
    Other(String) => "other",
});

wire_struct!(CaptureRefusalV1 {
    code: CaptureRefusalCodeV1,
    at: PhysicalCoordinateV1,
});

closed_enum!(CaptureRefusalCodeV1 {
    SourceUnreachable => "source_unreachable",
    UnresolvedSourceCoordinate => "unresolved_source_coordinate",
    ReadFailure => "read_failure",
    ForeignMarkdownNode => "foreign_markdown_node",
    PendingBatchPresent => "pending_batch_present",
    InvalidRelativePath => "invalid_relative_path",
    UnsupportedSchema => "unsupported_schema",
    UnknownPhysicalObject => "unknown_physical_object",
    SqlTypeMismatch => "sql_type_mismatch",
    SqlNull => "sql_null",
    DuplicateCoordinate => "duplicate_coordinate",
    NonUtf8Text => "non_utf8_text",
    UnknownHistoryKind => "unknown_history_kind",
    ForeignInstanceKind => "foreign_instance_kind",
    NumericOutOfRange => "numeric_out_of_range",
    HybridContradiction => "hybrid_contradiction",
    NonCanonicalOrder => "non_canonical_order",
    IncompatibleVariant => "incompatible_variant",
    DigestMismatch => "digest_mismatch",
});

wire_struct!(MarkdownPathCoordinateV1 {
    relative: HostPathV1
});
wire_struct!(MarkdownSpanCoordinateV1 {
    relative: HostPathV1,
    start: u64,
    length: u64,
});
wire_struct!(SqlTableCoordinateV1 { table: String });
wire_struct!(SqlRowCoordinateV1 {
    table: String,
    key: Vec<SqlKeyPartV1>,
});
wire_struct!(SqlColumnCoordinateV1 {
    table: String,
    key: Vec<SqlKeyPartV1>,
    column: String,
});
wire_struct!(SqlPhysicalRowCoordinateV1 {
    table: String,
    locator: SqlPhysicalLocatorV1,
});
wire_struct!(SqlCatalogCoordinateV1 {
    namespace: PresenceV1<String>,
    family: CatalogFamilyV1,
    table: PresenceV1<String>,
    row: PresenceV1<u64>,
});

closed_enum!(PhysicalCoordinateV1 {
    Root(RootCoordinateV1) => "root",
    MarkdownPath(MarkdownPathCoordinateV1) => "markdown_path",
    MarkdownSpan(MarkdownSpanCoordinateV1) => "markdown_span",
    SqlTable(SqlTableCoordinateV1) => "sql_table",
    SqlRow(SqlRowCoordinateV1) => "sql_row",
    SqlColumn(SqlColumnCoordinateV1) => "sql_column",
    SqlPhysicalRow(SqlPhysicalRowCoordinateV1) => "sql_physical_row",
    SqlCatalog(SqlCatalogCoordinateV1) => "sql_catalog",
});

wire_struct!(PostgresTupleLocatorV1 {
    table_oid: u32,
    block: u32,
    offset: u16,
});
closed_enum!(SqlPhysicalLocatorV1 {
    SqliteRowId(i64) => "sqlite_row_id",
    PostgresTuple(PostgresTupleLocatorV1) => "postgres_tuple",
});

closed_enum!(SqlKeyPartV1 {
    Text(String) => "text",
    Integer(i64) => "integer",
});

wire_struct!(SelectorRootCoordinateV1 {
    project_root: HostPathV1,
    project_file: HostPathV1,
});
wire_struct!(MarkdownRootCoordinateV1 { root: HostPathV1 });
wire_struct!(SqliteDatabaseRootCoordinateV1 {
    database: HostPathV1
});
wire_struct!(PostgresEndpointRootCoordinateV1 {
    endpoint: PresenceV1<PostgresEndpointV1>,
    endpoint_id: PresenceV1<DigestV1>,
});
wire_struct!(HybridSideRootCoordinateV1 { side: HybridSideV1 });

closed_enum!(RootCoordinateV1 {
    Observation => "observation",
    Selector(SelectorRootCoordinateV1) => "selector",
    EffectiveConfig => "effective_config",
    MarkdownRoot(MarkdownRootCoordinateV1) => "markdown_root",
    SqliteDatabase(SqliteDatabaseRootCoordinateV1) => "sqlite_database",
    PostgresEndpoint(PostgresEndpointRootCoordinateV1) => "postgres_endpoint",
    HybridSide(HybridSideRootCoordinateV1) => "hybrid_side",
});

closed_enum!(HybridSideV1 {
    Local => "local",
    Replica => "replica",
    Divergences => "divergences",
});

/// Stable classification for pure capture validation failures.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum CaptureValidationCodeV1 {
    WrongFormat,
    MissingCoordinate,
    InvalidRelativePath,
    InvalidEndpoint,
    DigestMismatch,
    PhaseRoster,
    PhasePrefix,
    EmptyRefusals,
    IncompatibleVariant,
    NonCanonicalOrder,
    DuplicateCoordinate,
    PartialEvidence,
    UnsupportedSchema,
    CaptureMismatch,
    UnstableDifference,
    HybridContradiction,
    NumericOutOfRange,
    ForeignInstanceKind,
}

/// One stable, path-attributed pure validation failure.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CaptureValidationIssueV1 {
    pub code: CaptureValidationCodeV1,
    pub path: String,
}

/// Accumulated pure capture validation failures.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("raw capture validation failed with {} issue(s)", .0.len())]
pub struct CaptureValidationErrorsV1(Vec<CaptureValidationIssueV1>);

impl CaptureValidationErrorsV1 {
    pub fn as_slice(&self) -> &[CaptureValidationIssueV1] {
        &self.0
    }

    pub fn contains(&self, code: CaptureValidationCodeV1) -> bool {
        self.0.iter().any(|issue| issue.code == code)
    }
}

/// Lexical reason that a preserved host path cannot serve as a relative coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RelativePathErrorV1 {
    #[error("relative path is empty")]
    Empty,
    #[error("relative path is not valid in its host encoding")]
    InvalidEncoding,
    #[error("relative path contains a forbidden code unit")]
    ForbiddenUnit,
    #[error("relative path has an empty component")]
    EmptyComponent,
    #[error("relative path contains a dot component")]
    DotComponent,
    #[error("relative path contains a reserved Windows component")]
    ReservedComponent,
    #[error("relative path has a Windows component ending in dot or space")]
    TrailingDotOrSpace,
}

impl HostPathV1 {
    /// Checks the host-independent relative-coordinate grammar without changing preserved units.
    pub fn validate_relative(&self) -> Result<(), RelativePathErrorV1> {
        match self {
            Self::Unix(bytes) => validate_unix_relative(bytes.as_bytes()),
            Self::Windows(units) => validate_windows_relative(units),
        }
    }
}

impl HybridPolicyWordsV1 {
    /// Checks all four closed policy words.
    pub fn validate(&self) -> Result<(), CaptureValidationCodeV1> {
        let admitted = matches!(self.authority.as_str(), "local" | "replica")
            && matches!(
                self.read.as_str(),
                "local-first" | "replica-first" | "replica-only"
            )
            && matches!(self.on_unreachable.as_str(), "refuse" | "serve-stale")
            && matches!(self.on_divergence.as_str(), "refuse" | "record");
        admitted
            .then_some(())
            .ok_or(CaptureValidationCodeV1::HybridContradiction)
    }
}

impl PostgresEndpointV1 {
    /// Computes the credential-free endpoint identity.
    pub fn endpoint_id(&self) -> DigestV1 {
        digest_parts_v1("aep.migration.postgres-endpoint/1", &self.canonical_parts())
            .expect("an in-memory endpoint fits canonical framing")
    }
}

impl PhaseEvidenceV1 {
    /// Computes the digest binding exact partial or complete phase evidence.
    pub fn evidence_digest(&self) -> DigestV1 {
        digest_parts_v1(
            "aep.migration.raw-observation-evidence/1",
            &self.canonical_parts(),
        )
        .expect("in-memory phase evidence fits canonical framing")
    }
}

impl RawCaptureObservationV1 {
    /// Parses JSON using the closed Serde representation, then checks semantic invariants.
    pub fn from_json(bytes: &[u8]) -> Result<Self, RawCaptureReadErrorV1> {
        let value: Self = serde_json::from_slice(bytes).map_err(|error| RawCaptureReadErrorV1 {
            code: RawCaptureReadCodeV1::InvalidStructure,
            path: serde_json_error_path(&error),
        })?;
        value.validate().map_err(|errors| RawCaptureReadErrorV1 {
            code: RawCaptureReadCodeV1::InvalidSemantics,
            path: errors
                .as_slice()
                .first()
                .map_or_else(|| "$".to_owned(), |issue| issue.path.clone()),
        })?;
        Ok(value)
    }

    /// Accumulates deterministic semantic errors without mutating or normalizing observations.
    pub fn validate(&self) -> Result<(), CaptureValidationErrorsV1> {
        let mut errors = ValidationCollector::default();
        validate_source(&self.source, &mut errors);
        match &self.observation {
            ObservationOutcomeV1::Complete(complete) => {
                self.validate_complete(complete, &mut errors);
            }
            ObservationOutcomeV1::Unstable(unstable) => {
                self.validate_unstable(unstable, &mut errors);
            }
            ObservationOutcomeV1::Refused(refused) => self.validate_refused(refused, &mut errors),
        }
        errors.finish()
    }

    /// Returns the canonical complete transcript bytes, when all required coordinates are present.
    pub fn complete_transcript(
        &self,
        capture: &LegacyRawCaptureV1,
    ) -> Result<Vec<u8>, CompleteDigestErrorV1> {
        let selector = match &self.selector {
            PresenceV1::Present(value) => value,
            PresenceV1::Missing => return Err(CompleteDigestErrorV1::MissingSelector),
        };
        let config = match &self.config_digest {
            PresenceV1::Present(value) => value,
            PresenceV1::Missing => return Err(CompleteDigestErrorV1::MissingConfigDigest),
        };
        if matches!(self.source, PresenceV1::Missing) {
            return Err(CompleteDigestErrorV1::MissingSource);
        }
        let mut parts = self.format.canonical_parts();
        self.source.append_parts(&mut parts);
        selector.append_parts(&mut parts);
        parts.push(config.as_bytes().to_vec());
        capture.append_parts(&mut parts);
        Ok(frame_v1("aep.migration.raw-transcript/1", &parts)?)
    }

    /// Computes the transcript digest for complete input.
    pub fn complete_transcript_digest(
        &self,
        capture: &LegacyRawCaptureV1,
    ) -> Result<DigestV1, CompleteDigestErrorV1> {
        let transcript = self.complete_transcript(capture)?;
        Ok(DigestV1::from_bytes(sha256_bytes(&transcript)))
    }

    /// Computes the raw snapshot identity for complete input.
    pub fn complete_snapshot_id(
        &self,
        capture: &LegacyRawCaptureV1,
    ) -> Result<DigestV1, CompleteDigestErrorV1> {
        let selector = match &self.selector {
            PresenceV1::Present(value) => value,
            PresenceV1::Missing => return Err(CompleteDigestErrorV1::MissingSelector),
        };
        let config = match &self.config_digest {
            PresenceV1::Present(value) => value,
            PresenceV1::Missing => return Err(CompleteDigestErrorV1::MissingConfigDigest),
        };
        if matches!(self.source, PresenceV1::Missing) {
            return Err(CompleteDigestErrorV1::MissingSource);
        }
        let selector_binding = match &selector.presence {
            SelectorPresenceV1::MissingV1Default => {
                digest_parts_v1("aep.migration.missing-selector/1", &[])?.bytes()
            }
            SelectorPresenceV1::Present(value) => value.selector_digest.bytes(),
        };
        let transcript = self.complete_transcript(capture)?;
        Ok(digest_parts_v1(
            "aep.migration.raw-snapshot/1",
            &[
                config.as_bytes().to_vec(),
                selector_binding.to_vec(),
                transcript,
            ],
        )?)
    }

    fn validate_complete(
        &self,
        complete: &CompleteObservationV1,
        errors: &mut ValidationCollector,
    ) {
        self.require_complete_coordinates(errors);
        validate_method_source(
            &complete.method,
            &self.source,
            "$.observation.method",
            errors,
        );
        validate_phases(
            &complete.method,
            &self.source,
            &complete.phases,
            PhaseExpectation::Complete,
            errors,
        );
        let reconstructed =
            reconstruct_capture(&complete.method, &self.source, &complete.phases, errors);
        validate_capture(&complete.capture, errors);
        if let Some(reconstructed) = reconstructed {
            if complete.capture != reconstructed {
                errors.push(
                    CaptureValidationCodeV1::CaptureMismatch,
                    "$.observation.capture",
                );
            }
            if let Ok(expected) = self.complete_transcript_digest(&reconstructed) {
                if complete.transcript_digest != expected {
                    errors.push(
                        CaptureValidationCodeV1::DigestMismatch,
                        "$.observation.transcript_digest",
                    );
                }
            }
            if let Ok(expected) = self.complete_snapshot_id(&reconstructed) {
                if complete.raw_snapshot_id != expected {
                    errors.push(
                        CaptureValidationCodeV1::DigestMismatch,
                        "$.observation.raw_snapshot_id",
                    );
                }
            }
        }
    }

    fn validate_unstable(
        &self,
        unstable: &UnstableObservationV1,
        errors: &mut ValidationCollector,
    ) {
        self.require_complete_coordinates(errors);
        validate_method_source(
            &unstable.method,
            &self.source,
            "$.observation.method",
            errors,
        );
        validate_phases(
            &unstable.method,
            &self.source,
            &unstable.phases,
            PhaseExpectation::Complete,
            errors,
        );
        if !matches!(
            unstable.method,
            ObservationMethodV1::MarkdownDoubleScan
                | ObservationMethodV1::HybridBracketedSqlSnapshot
        ) {
            errors.push(
                CaptureValidationCodeV1::IncompatibleVariant,
                "$.observation.kind",
            );
            return;
        }
        let expected = changed_coordinates(&unstable.method, &unstable.phases, errors);
        if expected.is_empty() || expected != unstable.changed {
            errors.push(
                CaptureValidationCodeV1::UnstableDifference,
                "$.observation.changed",
            );
        }
        ensure_strict_sort(
            &unstable.changed,
            "$.observation.changed",
            CaptureValidationCodeV1::NonCanonicalOrder,
            errors,
        );
    }

    fn validate_refused(&self, refused: &RefusedObservationV1, errors: &mut ValidationCollector) {
        validate_refusals(
            &refused.preflight_refusals,
            "$.observation.preflight_refusals",
            errors,
        );
        match &refused.method {
            PresenceV1::Missing => {
                if !refused.phases.is_empty() {
                    errors.push(CaptureValidationCodeV1::PhaseRoster, "$.observation.phases");
                }
                if refused.preflight_refusals.is_empty() {
                    errors.push(
                        CaptureValidationCodeV1::EmptyRefusals,
                        "$.observation.preflight_refusals",
                    );
                }
            }
            PresenceV1::Present(method) => {
                let expectation = if refused.preflight_refusals.is_empty() {
                    PhaseExpectation::FailedPrefix
                } else {
                    PhaseExpectation::NotAttempted
                };
                if expectation == PhaseExpectation::FailedPrefix
                    || matches!(self.source, PresenceV1::Present(_))
                {
                    validate_method_source(method, &self.source, "$.observation.method", errors);
                }
                validate_phases(method, &self.source, &refused.phases, expectation, errors);
                if let PresenceV1::Present(SourceCoordinateV1::Hybrid(hybrid)) = &self.source {
                    if matches!(hybrid.policy, PresenceV1::Missing)
                        && expectation != PhaseExpectation::NotAttempted
                    {
                        errors.push(
                            CaptureValidationCodeV1::MissingCoordinate,
                            "$.source.value.policy",
                        );
                    }
                }
            }
        }
    }

    fn require_complete_coordinates(&self, errors: &mut ValidationCollector) {
        for (missing, path) in [
            (matches!(self.source, PresenceV1::Missing), "$.source"),
            (matches!(self.selector, PresenceV1::Missing), "$.selector"),
            (
                matches!(self.config_digest, PresenceV1::Missing),
                "$.config_digest",
            ),
        ] {
            if missing {
                errors.push(CaptureValidationCodeV1::MissingCoordinate, path);
            }
        }
        if let PresenceV1::Present(SourceCoordinateV1::Hybrid(hybrid)) = &self.source {
            match &hybrid.policy {
                PresenceV1::Present(policy) => {
                    if policy.validate().is_err() {
                        errors.push(
                            CaptureValidationCodeV1::HybridContradiction,
                            "$.source.value.policy",
                        );
                    }
                }
                PresenceV1::Missing => errors.push(
                    CaptureValidationCodeV1::MissingCoordinate,
                    "$.source.value.policy",
                ),
            }
        }
    }
}

/// Stable parse stage for a raw-capture read failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawCaptureReadCodeV1 {
    InvalidStructure,
    InvalidSemantics,
}

/// Why a complete transcript or snapshot identity cannot be computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CompleteDigestErrorV1 {
    #[error("complete transcript requires a resolved source")]
    MissingSource,
    #[error("complete transcript requires a resolved selector")]
    MissingSelector,
    #[error("complete transcript requires a configuration digest")]
    MissingConfigDigest,
    #[error(transparent)]
    Frame(#[from] FrameError),
}

/// Content-free raw-capture reader failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code:?} at {path}")]
pub struct RawCaptureReadErrorV1 {
    pub code: RawCaptureReadCodeV1,
    pub path: String,
}

#[derive(Debug, Default)]
struct ValidationCollector {
    issues: Vec<CaptureValidationIssueV1>,
}

impl ValidationCollector {
    fn push(&mut self, code: CaptureValidationCodeV1, path: impl Into<String>) {
        self.issues.push(CaptureValidationIssueV1 {
            code,
            path: path.into(),
        });
    }

    fn finish(self) -> Result<(), CaptureValidationErrorsV1> {
        if self.issues.is_empty() {
            Ok(())
        } else {
            Err(CaptureValidationErrorsV1(self.issues))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhaseExpectation {
    Complete,
    NotAttempted,
    FailedPrefix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedEvidence {
    Markdown,
    Sqlite,
    Postgres,
    Divergences,
}

fn validate_unix_relative(bytes: &[u8]) -> Result<(), RelativePathErrorV1> {
    if bytes.is_empty() {
        return Err(RelativePathErrorV1::Empty);
    }
    let path = std::str::from_utf8(bytes).map_err(|_| RelativePathErrorV1::InvalidEncoding)?;
    if bytes.contains(&0) {
        return Err(RelativePathErrorV1::ForbiddenUnit);
    }
    for component in path.split('/') {
        if component.is_empty() {
            return Err(RelativePathErrorV1::EmptyComponent);
        }
        if matches!(component, "." | "..") {
            return Err(RelativePathErrorV1::DotComponent);
        }
    }
    Ok(())
}

fn validate_windows_relative(units: &[u16]) -> Result<(), RelativePathErrorV1> {
    if units.is_empty() {
        return Err(RelativePathErrorV1::Empty);
    }
    if char::decode_utf16(units.iter().copied()).any(|decoded| decoded.is_err()) {
        return Err(RelativePathErrorV1::InvalidEncoding);
    }
    if units.iter().any(|unit| {
        *unit == 0
            || (1..=31).contains(unit)
            || matches!(*unit, 0x2f | 0x3a | 0x3c | 0x3e | 0x22 | 0x7c | 0x3f | 0x2a)
    }) {
        return Err(RelativePathErrorV1::ForbiddenUnit);
    }
    for component in units.split(|unit| *unit == 0x5c) {
        if component.is_empty() {
            return Err(RelativePathErrorV1::EmptyComponent);
        }
        if component == [0x2e] || component == [0x2e, 0x2e] {
            return Err(RelativePathErrorV1::DotComponent);
        }
        if matches!(component.last(), Some(0x2e | 0x20)) {
            return Err(RelativePathErrorV1::TrailingDotOrSpace);
        }
        let stem = component
            .split(|unit| *unit == 0x2e)
            .next()
            .expect("every component has a first stem");
        if windows_reserved_stem(stem) {
            return Err(RelativePathErrorV1::ReservedComponent);
        }
    }
    Ok(())
}

fn windows_reserved_stem(stem: &[u16]) -> bool {
    let Some(ascii) = stem
        .iter()
        .map(|unit| {
            u8::try_from(*unit)
                .ok()
                .map(|byte| byte.to_ascii_uppercase())
        })
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    matches!(ascii.as_slice(), b"CON" | b"PRN" | b"AUX" | b"NUL")
        || (ascii.len() == 4
            && matches!(&ascii[..3], b"COM" | b"LPT")
            && matches!(ascii[3], b'1'..=b'9'))
}

fn validate_source(source: &PresenceV1<SourceCoordinateV1>, errors: &mut ValidationCollector) {
    let PresenceV1::Present(source) = source else {
        return;
    };
    match source {
        SourceCoordinateV1::Postgres(postgres) => validate_endpoint(
            &postgres.endpoint,
            &postgres.endpoint_id,
            "$.source.value",
            errors,
        ),
        SourceCoordinateV1::Hybrid(hybrid) => {
            validate_replica(&hybrid.replica, "$.source.value.replica", errors);
            if let PresenceV1::Present(policy) = &hybrid.policy {
                if policy.validate().is_err() {
                    errors.push(
                        CaptureValidationCodeV1::HybridContradiction,
                        "$.source.value.policy",
                    );
                }
            }
        }
        SourceCoordinateV1::Markdown(_) | SourceCoordinateV1::Sqlite(_) => {}
    }
}

fn validate_replica(
    replica: &SqlReplicaCoordinateV1,
    path: &str,
    errors: &mut ValidationCollector,
) {
    if let SqlReplicaCoordinateV1::Postgres(postgres) = replica {
        validate_endpoint(&postgres.endpoint, &postgres.endpoint_id, path, errors);
    }
}

fn validate_endpoint(
    endpoint: &PostgresEndpointV1,
    endpoint_id: &DigestV1,
    path: &str,
    errors: &mut ValidationCollector,
) {
    let invalid_host = endpoint.hosts.iter().any(|host| match host {
        PostgresHostV1::Tcp(value) => value.host.is_empty(),
        PostgresHostV1::Unix(value) => match &value.directory {
            HostPathV1::Unix(value) => value.as_bytes().is_empty(),
            HostPathV1::Windows(value) => value.is_empty(),
        },
    });
    if endpoint.hosts.is_empty()
        || invalid_host
        || endpoint.database.is_empty()
        || endpoint.schema.is_empty()
    {
        errors.push(CaptureValidationCodeV1::InvalidEndpoint, path);
    }
    if endpoint.endpoint_id() != *endpoint_id {
        errors.push(
            CaptureValidationCodeV1::DigestMismatch,
            format!("{path}.endpoint_id"),
        );
    }
}

fn validate_method_source(
    method: &ObservationMethodV1,
    source: &PresenceV1<SourceCoordinateV1>,
    path: &str,
    errors: &mut ValidationCollector,
) {
    let compatible = matches!(
        (method, source),
        (
            ObservationMethodV1::MarkdownDoubleScan,
            PresenceV1::Present(SourceCoordinateV1::Markdown(_))
        ) | (
            ObservationMethodV1::SqliteReadTransaction,
            PresenceV1::Present(SourceCoordinateV1::Sqlite(_))
        ) | (
            ObservationMethodV1::PostgresRepeatableReadOnly,
            PresenceV1::Present(SourceCoordinateV1::Postgres(_))
        ) | (
            ObservationMethodV1::HybridBracketedSqlSnapshot,
            PresenceV1::Present(SourceCoordinateV1::Hybrid(_))
        )
    );
    if !compatible {
        errors.push(CaptureValidationCodeV1::IncompatibleVariant, path);
    }
}

fn expected_roster(
    method: &ObservationMethodV1,
    source: &PresenceV1<SourceCoordinateV1>,
) -> Vec<(CapturePhaseV1, ExpectedEvidence)> {
    match method {
        ObservationMethodV1::MarkdownDoubleScan => vec![
            (CapturePhaseV1::MarkdownFirst, ExpectedEvidence::Markdown),
            (CapturePhaseV1::MarkdownSecond, ExpectedEvidence::Markdown),
        ],
        ObservationMethodV1::SqliteReadTransaction => {
            vec![(CapturePhaseV1::SqlSnapshot, ExpectedEvidence::Sqlite)]
        }
        ObservationMethodV1::PostgresRepeatableReadOnly => {
            vec![(CapturePhaseV1::SqlSnapshot, ExpectedEvidence::Postgres)]
        }
        ObservationMethodV1::HybridBracketedSqlSnapshot => {
            let sql = match source {
                PresenceV1::Present(SourceCoordinateV1::Hybrid(hybrid)) => match hybrid.replica {
                    SqlReplicaCoordinateV1::Sqlite(_) => ExpectedEvidence::Sqlite,
                    SqlReplicaCoordinateV1::Postgres(_) => ExpectedEvidence::Postgres,
                },
                _ => ExpectedEvidence::Sqlite,
            };
            vec![
                (CapturePhaseV1::LocalBefore, ExpectedEvidence::Markdown),
                (
                    CapturePhaseV1::DivergencesBefore,
                    ExpectedEvidence::Divergences,
                ),
                (CapturePhaseV1::ReplicaSnapshot, sql),
                (CapturePhaseV1::LocalAfter, ExpectedEvidence::Markdown),
                (
                    CapturePhaseV1::DivergencesAfter,
                    ExpectedEvidence::Divergences,
                ),
            ]
        }
    }
}

fn validate_phases(
    method: &ObservationMethodV1,
    source: &PresenceV1<SourceCoordinateV1>,
    phases: &[PhaseObservationV1],
    expectation: PhaseExpectation,
    errors: &mut ValidationCollector,
) {
    let roster = expected_roster(method, source);
    if phases.len() != roster.len()
        || phases
            .iter()
            .zip(&roster)
            .any(|(actual, (phase, _))| actual.phase != *phase)
    {
        errors.push(CaptureValidationCodeV1::PhaseRoster, "$.observation.phases");
    }

    let mut refused_count = 0_usize;
    let mut saw_refused = false;
    for (index, phase) in phases.iter().enumerate() {
        let path = format!("$.observation.phases[{index}]");
        let expected_evidence = roster.get(index).map(|(_, evidence)| *evidence);
        match &phase.result {
            PhaseResultV1::NotAttempted => {
                if expectation == PhaseExpectation::Complete
                    || (expectation == PhaseExpectation::FailedPrefix && !saw_refused)
                {
                    errors.push(
                        CaptureValidationCodeV1::PhasePrefix,
                        format!("{path}.result"),
                    );
                }
            }
            PhaseResultV1::Complete(result) => {
                if expectation == PhaseExpectation::NotAttempted || saw_refused {
                    errors.push(
                        CaptureValidationCodeV1::PhasePrefix,
                        format!("{path}.result"),
                    );
                }
                validate_phase_result(
                    &result.evidence,
                    &result.evidence_digest,
                    expected_evidence,
                    source,
                    false,
                    &path,
                    errors,
                );
            }
            PhaseResultV1::Refused(result) => {
                refused_count += 1;
                saw_refused = true;
                if expectation != PhaseExpectation::FailedPrefix {
                    errors.push(
                        CaptureValidationCodeV1::PhasePrefix,
                        format!("{path}.result"),
                    );
                }
                if result.refusals.is_empty() {
                    errors.push(
                        CaptureValidationCodeV1::EmptyRefusals,
                        format!("{path}.result.value.refusals"),
                    );
                }
                validate_refusals(
                    &result.refusals,
                    &format!("{path}.result.value.refusals"),
                    errors,
                );
                validate_phase_result(
                    &result.evidence,
                    &result.evidence_digest,
                    expected_evidence,
                    source,
                    true,
                    &path,
                    errors,
                );
                validate_retained_refusal_coverage(
                    &result.evidence,
                    &result.refusals,
                    &path,
                    errors,
                );
            }
        }
    }
    if expectation == PhaseExpectation::FailedPrefix && refused_count != 1 {
        errors.push(CaptureValidationCodeV1::PhasePrefix, "$.observation.phases");
    }
}

fn validate_retained_refusal_coverage(
    evidence: &PhaseEvidenceV1,
    refusals: &[CaptureRefusalV1],
    path: &str,
    errors: &mut ValidationCollector,
) {
    fn push_terminal(value: &EnumerationTerminalV1, exact: &mut Vec<CaptureRefusalV1>) {
        if let EnumerationTerminalV1::Refused(value) = value {
            exact.push(CaptureRefusalV1 {
                code: value.code.clone(),
                at: value.at.clone(),
            });
        }
    }

    fn push_observed<T>(value: &ObservedValueV1<T>, exact: &mut Vec<CaptureRefusalV1>) {
        if let ObservedValueV1::Refused(value) = value {
            exact.push(CaptureRefusalV1 {
                code: value.code.clone(),
                at: value.at.clone(),
            });
        }
    }

    let mut exact = Vec::new();
    let mut coordinates = Vec::new();
    match evidence {
        PhaseEvidenceV1::Markdown(markdown) => {
            push_terminal(&markdown.nodes.terminal, &mut exact);
            for node in &markdown.nodes.items {
                match node {
                    MarkdownNodeEvidenceV1::Captured(node) => {
                        if let Some(code) = required_markdown_node_refusal(node) {
                            exact.push(CaptureRefusalV1 {
                                code,
                                at: PhysicalCoordinateV1::MarkdownPath(MarkdownPathCoordinateV1 {
                                    relative: node.relative.clone(),
                                }),
                            });
                        }
                    }
                    MarkdownNodeEvidenceV1::Unreadable(node) => coordinates.push(
                        PhysicalCoordinateV1::MarkdownPath(MarkdownPathCoordinateV1 {
                            relative: node.relative.clone(),
                        }),
                    ),
                }
            }
        }
        PhaseEvidenceV1::Sqlite(sql) | PhaseEvidenceV1::Postgres(sql) => {
            push_terminal(&sql.catalog.objects.terminal, &mut exact);
            for table in &sql.catalog.tables {
                push_observed(&table.catalog_definition, &mut exact);
                push_terminal(&table.columns.terminal, &mut exact);
                push_observed(&table.primary_key, &mut exact);
                push_terminal(&table.unique_keys.terminal, &mut exact);
                push_terminal(&table.checks.terminal, &mut exact);
            }
            push_terminal(&sql.catalog.indexes.terminal, &mut exact);
            push_terminal(&sql.catalog.foreign_objects.terminal, &mut exact);
            push_terminal(&sql.instances.terminal, &mut exact);
            push_terminal(&sql.events.terminal, &mut exact);
            push_terminal(&sql.history.terminal, &mut exact);
            push_terminal(&sql.legacy_origins.terminal, &mut exact);
            if let ObservedSequenceRowsV1::Observed(rows) = &sql.provider_sequences {
                push_terminal(&rows.terminal, &mut exact);
            }
            coordinates.extend(sql.catalog.rejected_rows.iter().map(|row| row.at.clone()));
            coordinates.extend(sql.rejected_rows.iter().map(|row| row.at.clone()));
        }
        PhaseEvidenceV1::Divergences(value) => {
            if let ObservedValueV1::Refused(value) = value {
                exact.push(CaptureRefusalV1 {
                    code: value.code.clone(),
                    at: value.at.clone(),
                });
            }
        }
    }
    if exact.iter().any(|failure| !refusals.contains(failure))
        || coordinates
            .iter()
            .any(|coordinate| !refusals.iter().any(|failure| &failure.at == coordinate))
    {
        errors.push(
            CaptureValidationCodeV1::IncompatibleVariant,
            format!("{path}.result.value.refusals"),
        );
    }
}

fn validate_phase_result(
    evidence: &PhaseEvidenceV1,
    digest: &DigestV1,
    expected: Option<ExpectedEvidence>,
    source: &PresenceV1<SourceCoordinateV1>,
    partial: bool,
    path: &str,
    errors: &mut ValidationCollector,
) {
    if evidence.evidence_digest() != *digest {
        errors.push(
            CaptureValidationCodeV1::DigestMismatch,
            format!("{path}.result.value.evidence_digest"),
        );
    }
    let compatible = matches!(
        (expected, evidence),
        (
            Some(ExpectedEvidence::Markdown),
            PhaseEvidenceV1::Markdown(_)
        ) | (Some(ExpectedEvidence::Sqlite), PhaseEvidenceV1::Sqlite(_))
            | (
                Some(ExpectedEvidence::Postgres),
                PhaseEvidenceV1::Postgres(_)
            )
            | (
                Some(ExpectedEvidence::Divergences),
                PhaseEvidenceV1::Divergences(_)
            )
    );
    if !compatible {
        errors.push(
            CaptureValidationCodeV1::IncompatibleVariant,
            format!("{path}.result.value.evidence"),
        );
    }
    match evidence {
        PhaseEvidenceV1::Markdown(markdown) => {
            validate_markdown_evidence(markdown, source, partial, path, errors);
        }
        PhaseEvidenceV1::Sqlite(sql) => {
            validate_sql_evidence(sql, SqlDialectV1::Sqlite, source, partial, path, errors);
        }
        PhaseEvidenceV1::Postgres(sql) => {
            validate_sql_evidence(sql, SqlDialectV1::Postgres, source, partial, path, errors);
        }
        PhaseEvidenceV1::Divergences(value) => {
            if !partial && !matches!(value, ObservedValueV1::Complete(_)) {
                errors.push(
                    CaptureValidationCodeV1::PartialEvidence,
                    format!("{path}.result.value.evidence"),
                );
            }
        }
    }
}

fn validate_markdown_evidence(
    evidence: &MarkdownEvidenceV1,
    source: &PresenceV1<SourceCoordinateV1>,
    partial: bool,
    path: &str,
    errors: &mut ValidationCollector,
) {
    let expected_root = match source {
        PresenceV1::Present(SourceCoordinateV1::Markdown(value)) => Some(&value.root),
        PresenceV1::Present(SourceCoordinateV1::Hybrid(value)) => Some(&value.local_root),
        _ => None,
    };
    if let (PresenceV1::Present(actual), Some(expected)) = (&evidence.root, expected_root) {
        if actual != expected {
            errors.push(
                CaptureValidationCodeV1::IncompatibleVariant,
                format!("{path}.result.value.evidence.value.root"),
            );
        }
    }
    if !partial && !matches!(evidence.root, PresenceV1::Present(_)) {
        errors.push(
            CaptureValidationCodeV1::PartialEvidence,
            format!("{path}.result.value.evidence.value.root"),
        );
    }
    validate_observed_list(
        &evidence.nodes,
        !partial,
        &format!("{path}.result.value.evidence.value.nodes"),
        errors,
    );
    ensure_strict_by(
        &evidence.nodes.items,
        |node| markdown_evidence_relative(node).clone(),
        &format!("{path}.result.value.evidence.value.nodes.items"),
        errors,
    );
    for (index, node) in evidence.nodes.items.iter().enumerate() {
        let relative = markdown_evidence_relative(node);
        if relative.validate_relative().is_err() {
            errors.push(
                CaptureValidationCodeV1::InvalidRelativePath,
                format!("{path}.result.value.evidence.value.nodes.items[{index}].relative"),
            );
        }
        if !partial && matches!(node, MarkdownNodeEvidenceV1::Unreadable(_)) {
            errors.push(
                CaptureValidationCodeV1::PartialEvidence,
                format!("{path}.result.value.evidence.value.nodes.items[{index}]"),
            );
        }
        if !partial {
            if let MarkdownNodeEvidenceV1::Captured(node) = node {
                validate_markdown_node(
                    node,
                    &format!("{path}.result.value.evidence.value.nodes.items[{index}]"),
                    errors,
                );
            }
        }
    }
}

fn markdown_evidence_relative(node: &MarkdownNodeEvidenceV1) -> &HostPathV1 {
    match node {
        MarkdownNodeEvidenceV1::Captured(node) => &node.relative,
        MarkdownNodeEvidenceV1::Unreadable(node) => &node.relative,
    }
}

#[allow(clippy::too_many_lines)] // Mirrors the fixed catalog-then-row read order in one audit path.
fn validate_sql_evidence(
    evidence: &SqlEvidenceV1,
    dialect: SqlDialectV1,
    source: &PresenceV1<SourceCoordinateV1>,
    partial: bool,
    path: &str,
    errors: &mut ValidationCollector,
) {
    let expected = expected_replica(source);
    if let (PresenceV1::Present(actual), Some(expected)) = (&evidence.source, expected.as_ref()) {
        if actual != expected {
            errors.push(
                CaptureValidationCodeV1::IncompatibleVariant,
                format!("{path}.result.value.evidence.value.source"),
            );
        }
    }
    if !partial && !matches!(evidence.source, PresenceV1::Present(_)) {
        errors.push(
            CaptureValidationCodeV1::PartialEvidence,
            format!("{path}.result.value.evidence.value.source"),
        );
    }
    if let (PresenceV1::Present(actual), Some(expected)) =
        (&evidence.catalog.namespace, expected_namespace(source))
    {
        if actual != expected {
            errors.push(
                CaptureValidationCodeV1::IncompatibleVariant,
                format!("{path}.result.value.evidence.value.catalog.namespace"),
            );
        }
    }

    validate_catalog(&evidence.catalog, !partial, path, errors);
    validate_observed_list(
        &evidence.instances,
        !partial,
        &format!("{path}.result.value.evidence.value.instances"),
        errors,
    );
    validate_observed_list(
        &evidence.events,
        !partial,
        &format!("{path}.result.value.evidence.value.events"),
        errors,
    );
    validate_observed_list(
        &evidence.history,
        !partial,
        &format!("{path}.result.value.evidence.value.history"),
        errors,
    );
    validate_observed_list(
        &evidence.legacy_origins,
        !partial,
        &format!("{path}.result.value.evidence.value.legacy_origins"),
        errors,
    );
    match (&dialect, &evidence.provider_sequences) {
        (SqlDialectV1::Sqlite, ObservedSequenceRowsV1::NotApplicable) => {}
        (SqlDialectV1::Postgres, ObservedSequenceRowsV1::Observed(rows)) => {
            validate_observed_list(
                rows,
                !partial,
                &format!("{path}.result.value.evidence.value.provider_sequences"),
                errors,
            );
            ensure_strict_by(
                &rows.items,
                |row| row.namespace.as_bytes().to_vec(),
                &format!("{path}.result.value.evidence.value.provider_sequences.value.items"),
                errors,
            );
        }
        _ => errors.push(
            CaptureValidationCodeV1::IncompatibleVariant,
            format!("{path}.result.value.evidence.value.provider_sequences"),
        ),
    }

    if !partial && !evidence.rejected_rows.is_empty() {
        errors.push(
            CaptureValidationCodeV1::PartialEvidence,
            format!("{path}.result.value.evidence.value.rejected_rows"),
        );
    }
    ensure_strict_by(
        &evidence.rejected_rows,
        |row| sort_key_v1(&row.at),
        &format!("{path}.result.value.evidence.value.rejected_rows"),
        errors,
    );
    for (index, row) in evidence.rejected_rows.iter().enumerate() {
        validate_cells(
            &row.cells,
            &format!("{path}.result.value.evidence.value.rejected_rows[{index}].cells"),
            errors,
        );
    }
    validate_rows(evidence, path, errors);
    validate_sql_read_prefix(evidence, &dialect, path, errors);
    if !partial {
        if let Some(raw) = sql_raw_from_evidence(evidence, dialect) {
            validate_sql_raw(
                &raw,
                &raw.dialect,
                &format!("{path}.result.value.evidence.value"),
                errors,
            );
        }
    }
}

#[allow(clippy::too_many_lines)] // Keeps the ordered catalog-family invariant visibly sequential.
fn validate_catalog(
    catalog: &SqlCatalogEvidenceV1,
    complete: bool,
    path: &str,
    errors: &mut ValidationCollector,
) {
    let base = format!("{path}.result.value.evidence.value.catalog");
    if complete && !matches!(catalog.namespace, PresenceV1::Present(_)) {
        errors.push(
            CaptureValidationCodeV1::PartialEvidence,
            format!("{base}.namespace"),
        );
    }
    validate_observed_list(
        &catalog.objects,
        complete,
        &format!("{base}.objects"),
        errors,
    );
    ensure_strict_by(
        &catalog.objects.items,
        catalog_header_order_key,
        &format!("{base}.objects.items"),
        errors,
    );
    ensure_strict_by(
        &catalog.tables,
        |table| table.name.as_bytes().to_vec(),
        &format!("{base}.tables"),
        errors,
    );
    let discovered_tables = catalog
        .objects
        .items
        .iter()
        .filter(|header| header.kind == ForeignObjectKindV1::Table)
        .map(|header| (header.name.as_str(), &header.parent))
        .collect::<Vec<_>>();
    if discovered_tables.len() != catalog.tables.len()
        || discovered_tables
            .iter()
            .zip(&catalog.tables)
            .any(|((name, parent), table)| {
                *name != table.name.as_str() || !matches!(parent, PresenceV1::Missing)
            })
    {
        errors.push(
            CaptureValidationCodeV1::UnsupportedSchema,
            format!("{base}.tables"),
        );
    }
    for (index, table) in catalog.tables.iter().enumerate() {
        let table_path = format!("{base}.tables[{index}]");
        validate_observed_value(
            &table.catalog_definition,
            complete,
            &format!("{table_path}.catalog_definition"),
            errors,
        );
        validate_observed_list(
            &table.columns,
            complete,
            &format!("{table_path}.columns"),
            errors,
        );
        ensure_strict_by(
            &table.columns.items,
            |column| column.ordinal.to_be_bytes().to_vec(),
            &format!("{table_path}.columns.items"),
            errors,
        );
        let mut column_names = BTreeSet::new();
        for column in &table.columns.items {
            if !column_names.insert(column.name.as_str()) {
                errors.push(
                    CaptureValidationCodeV1::DuplicateCoordinate,
                    format!("{table_path}.columns.items"),
                );
                break;
            }
        }
        validate_observed_value(
            &table.primary_key,
            complete,
            &format!("{table_path}.primary_key"),
            errors,
        );
        validate_observed_list(
            &table.unique_keys,
            complete,
            &format!("{table_path}.unique_keys"),
            errors,
        );
        ensure_strict_sort(
            &table.unique_keys.items,
            &format!("{table_path}.unique_keys.items"),
            CaptureValidationCodeV1::NonCanonicalOrder,
            errors,
        );
        validate_observed_list(
            &table.checks,
            complete,
            &format!("{table_path}.checks"),
            errors,
        );
        ensure_strict_sort(
            &table.checks.items,
            &format!("{table_path}.checks.items"),
            CaptureValidationCodeV1::NonCanonicalOrder,
            errors,
        );
        let mut key_sequences = BTreeSet::new();
        if let ObservedValueV1::Complete(PresenceV1::Present(primary)) = &table.primary_key {
            key_sequences.insert(primary.columns.clone());
        }
        if table
            .unique_keys
            .items
            .iter()
            .any(|key| !key_sequences.insert(key.columns.clone()))
        {
            errors.push(
                CaptureValidationCodeV1::DuplicateCoordinate,
                format!("{table_path}.unique_keys.items"),
            );
        }
        let mut constraint_names = BTreeSet::new();
        let primary_name = match &table.primary_key {
            ObservedValueV1::Complete(PresenceV1::Present(key)) => Some(&key.name),
            _ => None,
        };
        let duplicate_name = primary_name
            .into_iter()
            .chain(table.unique_keys.items.iter().map(|key| &key.name))
            .chain(table.checks.items.iter().map(check_name))
            .filter_map(|name| match name {
                PresenceV1::Present(name) => Some(name.as_str()),
                PresenceV1::Missing => None,
            })
            .any(|name| !constraint_names.insert(name));
        if duplicate_name {
            errors.push(
                CaptureValidationCodeV1::DuplicateCoordinate,
                format!("{table_path}.unique_keys.items"),
            );
        }
    }
    validate_observed_list(
        &catalog.indexes,
        complete,
        &format!("{base}.indexes"),
        errors,
    );
    ensure_strict_by(
        &catalog.indexes.items,
        |index| index.name.as_bytes().to_vec(),
        &format!("{base}.indexes.items"),
        errors,
    );
    validate_observed_list(
        &catalog.foreign_objects,
        complete,
        &format!("{base}.foreign_objects"),
        errors,
    );
    ensure_strict_sort(
        &catalog.foreign_objects.items,
        &format!("{base}.foreign_objects.items"),
        CaptureValidationCodeV1::NonCanonicalOrder,
        errors,
    );
    let mut index_names = BTreeSet::new();
    let duplicate_index = catalog
        .tables
        .iter()
        .flat_map(|table| {
            let primary = match &table.primary_key {
                ObservedValueV1::Complete(PresenceV1::Present(key)) => Some(key),
                _ => None,
            };
            primary.into_iter().chain(table.unique_keys.items.iter())
        })
        .filter_map(|key| match &key.backing_index {
            PresenceV1::Present(name) => Some(name.as_str()),
            PresenceV1::Missing => None,
        })
        .chain(
            catalog
                .indexes
                .items
                .iter()
                .map(|index| index.name.as_str()),
        )
        .any(|name| !index_names.insert(name));
    if duplicate_index {
        errors.push(
            CaptureValidationCodeV1::DuplicateCoordinate,
            format!("{base}.indexes.items"),
        );
    }
    if complete && !catalog.rejected_rows.is_empty() {
        errors.push(
            CaptureValidationCodeV1::PartialEvidence,
            format!("{base}.rejected_rows"),
        );
    }
    ensure_strict_by(
        &catalog.rejected_rows,
        |row| sort_key_v1(&row.at),
        &format!("{base}.rejected_rows"),
        errors,
    );
    for (index, row) in catalog.rejected_rows.iter().enumerate() {
        validate_cells(
            &row.cells,
            &format!("{base}.rejected_rows[{index}].cells"),
            errors,
        );
    }
    if complete {
        validate_catalog_correspondence(catalog, &base, errors);
    }
}

fn check_name(check: &CheckSchemaV1) -> &PresenceV1<String> {
    match check {
        CheckSchemaV1::HistoryKindDecisionObservation(value) => &value.name,
        CheckSchemaV1::NextValueNonnegative(value) => &value.name,
        CheckSchemaV1::Foreign(value) => &value.name,
    }
}

fn validate_catalog_correspondence(
    catalog: &SqlCatalogEvidenceV1,
    path: &str,
    errors: &mut ValidationCollector,
) {
    let headers = catalog
        .objects
        .items
        .iter()
        .map(|header| {
            (
                header.kind.clone(),
                header.name.clone(),
                header.parent.clone(),
            )
        })
        .collect::<BTreeSet<_>>();
    let expected = catalog
        .tables
        .iter()
        .map(|table| {
            (
                ForeignObjectKindV1::Table,
                table.name.clone(),
                PresenceV1::Missing,
            )
        })
        .chain(catalog.indexes.items.iter().map(|index| {
            (
                ForeignObjectKindV1::Index,
                index.name.clone(),
                PresenceV1::Present(index.table.clone()),
            )
        }))
        .chain(catalog.foreign_objects.items.iter().map(|object| {
            (
                object.kind.clone(),
                object.name.clone(),
                PresenceV1::Missing,
            )
        }))
        .collect::<BTreeSet<_>>();
    if headers.len() != catalog.objects.items.len() || headers != expected {
        errors.push(CaptureValidationCodeV1::UnsupportedSchema, path);
    }
}

#[allow(clippy::too_many_lines)] // Each closed row family is checked in its declared order here.
fn validate_rows(evidence: &SqlEvidenceV1, path: &str, errors: &mut ValidationCollector) {
    ensure_strict_by(
        &evidence.instances.items,
        |row| (row.entity.clone(), row.id.clone()),
        &format!("{path}.result.value.evidence.value.instances.items"),
        errors,
    );
    ensure_strict_by(
        &evidence.events.items,
        |row| {
            (
                row.entity.clone(),
                row.id.clone(),
                row.revision,
                row.position,
            )
        },
        &format!("{path}.result.value.evidence.value.events.items"),
        errors,
    );
    ensure_strict_by(
        &evidence.history.items,
        |row| (row.entity.clone(), row.id.clone(), row.position),
        &format!("{path}.result.value.evidence.value.history.items"),
        errors,
    );
    ensure_strict_by(
        &evidence.legacy_origins.items,
        |row| (row.entity.clone(), row.id.clone()),
        &format!("{path}.result.value.evidence.value.legacy_origins.items"),
        errors,
    );
    for (family, values) in [
        (
            "instances",
            evidence
                .instances
                .items
                .iter()
                .map(|row| (row.entity.as_str(), row.revision, 0_i64))
                .collect::<Vec<_>>(),
        ),
        (
            "events",
            evidence
                .events
                .items
                .iter()
                .map(|row| (row.entity.as_str(), row.revision, row.position))
                .collect(),
        ),
        (
            "history",
            evidence
                .history
                .items
                .iter()
                .map(|row| (row.entity.as_str(), 0_i64, row.position))
                .collect(),
        ),
        (
            "legacy_origins",
            evidence
                .legacy_origins
                .items
                .iter()
                .map(|row| (row.entity.as_str(), row.revision, 0_i64))
                .collect(),
        ),
    ] {
        for (index, (entity, first, second)) in values.iter().enumerate() {
            if !matches!(
                *entity,
                "aep.entity" | "aep.relation" | "aep.audit" | "aep.applied"
            ) {
                errors.push(
                    CaptureValidationCodeV1::ForeignInstanceKind,
                    format!("{path}.result.value.evidence.value.{family}.items[{index}].entity"),
                );
            }
            if *first < 0 || *second < 0 {
                errors.push(
                    CaptureValidationCodeV1::NumericOutOfRange,
                    format!("{path}.result.value.evidence.value.{family}.items[{index}]"),
                );
            }
        }
    }
    for (family, documents) in [
        (
            "instances",
            evidence
                .instances
                .items
                .iter()
                .map(|row| &row.document)
                .collect::<Vec<_>>(),
        ),
        (
            "events",
            evidence
                .events
                .items
                .iter()
                .map(|row| &row.document)
                .collect(),
        ),
        (
            "history",
            evidence
                .history
                .items
                .iter()
                .map(|row| &row.document)
                .collect(),
        ),
    ] {
        for (index, document) in documents.iter().enumerate() {
            if std::str::from_utf8(document.as_bytes()).is_err() {
                errors.push(
                    CaptureValidationCodeV1::UnsupportedSchema,
                    format!("{path}.result.value.evidence.value.{family}.items[{index}].document"),
                );
            }
        }
    }
    if let ObservedSequenceRowsV1::Observed(rows) = &evidence.provider_sequences {
        for (index, row) in rows.items.iter().enumerate() {
            if row.next_value < 0 {
                errors.push(
                    CaptureValidationCodeV1::NumericOutOfRange,
                    format!(
                        "{path}.result.value.evidence.value.provider_sequences.value.items[{index}].next_value"
                    ),
                );
            }
        }
    }
}

fn validate_sql_read_prefix(
    evidence: &SqlEvidenceV1,
    dialect: &SqlDialectV1,
    path: &str,
    errors: &mut ValidationCollector,
) {
    let mut states = Vec::new();
    states.push(terminal_state(&evidence.catalog.objects.terminal));
    for table in &evidence.catalog.tables {
        states.extend([
            observed_value_state(&table.catalog_definition),
            terminal_state(&table.columns.terminal),
            observed_value_state(&table.primary_key),
            terminal_state(&table.unique_keys.terminal),
            terminal_state(&table.checks.terminal),
        ]);
    }
    states.push(terminal_state(&evidence.catalog.indexes.terminal));
    states.push(terminal_state(&evidence.catalog.foreign_objects.terminal));
    states.extend([
        terminal_state(&evidence.instances.terminal),
        terminal_state(&evidence.events.terminal),
        terminal_state(&evidence.history.terminal),
        terminal_state(&evidence.legacy_origins.terminal),
    ]);
    if let (SqlDialectV1::Postgres, ObservedSequenceRowsV1::Observed(rows)) =
        (dialect, &evidence.provider_sequences)
    {
        states.push(terminal_state(&rows.terminal));
    }
    let mut failed = false;
    for state in states {
        match state {
            ReadState::Complete | ReadState::Refused if failed => {
                errors.push(
                    CaptureValidationCodeV1::PhasePrefix,
                    format!("{path}.result"),
                );
                break;
            }
            ReadState::Refused | ReadState::NotAttempted => failed = true,
            ReadState::Complete => {}
        }
    }
}

#[derive(Clone, Copy)]
enum ReadState {
    NotAttempted,
    Complete,
    Refused,
}

fn terminal_state(terminal: &EnumerationTerminalV1) -> ReadState {
    match terminal {
        EnumerationTerminalV1::NotAttempted => ReadState::NotAttempted,
        EnumerationTerminalV1::Complete => ReadState::Complete,
        EnumerationTerminalV1::Refused(_) => ReadState::Refused,
    }
}

fn observed_value_state<T>(value: &ObservedValueV1<T>) -> ReadState {
    match value {
        ObservedValueV1::NotAttempted => ReadState::NotAttempted,
        ObservedValueV1::Complete(_) => ReadState::Complete,
        ObservedValueV1::Refused(_) => ReadState::Refused,
    }
}

fn validate_observed_list<T>(
    list: &ObservedListV1<T>,
    complete: bool,
    path: &str,
    errors: &mut ValidationCollector,
) {
    if matches!(list.terminal, EnumerationTerminalV1::NotAttempted) && !list.items.is_empty() {
        errors.push(CaptureValidationCodeV1::PartialEvidence, path);
    }
    if complete && !matches!(list.terminal, EnumerationTerminalV1::Complete) {
        errors.push(CaptureValidationCodeV1::PartialEvidence, path);
    }
}

fn validate_observed_value<T>(
    value: &ObservedValueV1<T>,
    complete: bool,
    path: &str,
    errors: &mut ValidationCollector,
) {
    if complete && !matches!(value, ObservedValueV1::Complete(_)) {
        errors.push(CaptureValidationCodeV1::PartialEvidence, path);
    }
}

fn validate_cells(cells: &[SqlCellImageV1], path: &str, errors: &mut ValidationCollector) {
    if cells
        .windows(2)
        .any(|pair| pair[0].ordinal >= pair[1].ordinal)
    {
        errors.push(CaptureValidationCodeV1::NonCanonicalOrder, path);
    }
    let mut names = BTreeSet::new();
    if cells.iter().any(|cell| !names.insert(cell.column.as_str())) {
        errors.push(CaptureValidationCodeV1::DuplicateCoordinate, path);
    }
}

fn validate_refusals(refusals: &[CaptureRefusalV1], path: &str, errors: &mut ValidationCollector) {
    ensure_strict_by(
        refusals,
        |refusal| (sort_key_v1(&refusal.at), variant_tag_owned(&refusal.code)),
        path,
        errors,
    );
    for (index, refusal) in refusals.iter().enumerate() {
        if matches!(
            refusal.code,
            CaptureRefusalCodeV1::SourceUnreachable
                | CaptureRefusalCodeV1::UnresolvedSourceCoordinate
        ) && !matches!(refusal.at, PhysicalCoordinateV1::Root(_))
        {
            errors.push(
                CaptureValidationCodeV1::IncompatibleVariant,
                format!("{path}[{index}].at"),
            );
        }
    }
}

fn variant_tag_owned(value: &impl CanonicalPartsV1) -> Vec<u8> {
    value
        .canonical_parts()
        .into_iter()
        .next()
        .expect("every adjacent enum emits its fixed tag first")
}

fn catalog_header_order_key(header: &CatalogObjectHeaderV1) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    (
        variant_tag_owned(&header.kind),
        header.name.as_bytes().to_vec(),
        sort_key_v1(&header.parent),
    )
}

fn ensure_strict_sort<T: CanonicalPartsV1>(
    values: &[T],
    path: &str,
    code: CaptureValidationCodeV1,
    errors: &mut ValidationCollector,
) {
    for pair in values.windows(2) {
        match sort_key_v1(&pair[0]).cmp(&sort_key_v1(&pair[1])) {
            std::cmp::Ordering::Equal => {
                errors.push(CaptureValidationCodeV1::DuplicateCoordinate, path);
                break;
            }
            std::cmp::Ordering::Greater => {
                errors.push(code, path);
                break;
            }
            std::cmp::Ordering::Less => {}
        }
    }
}

fn ensure_strict_by<T, K: Ord>(
    values: &[T],
    key: impl Fn(&T) -> K,
    path: &str,
    errors: &mut ValidationCollector,
) {
    for pair in values.windows(2) {
        match key(&pair[0]).cmp(&key(&pair[1])) {
            std::cmp::Ordering::Equal => {
                errors.push(CaptureValidationCodeV1::DuplicateCoordinate, path);
                break;
            }
            std::cmp::Ordering::Greater => {
                errors.push(CaptureValidationCodeV1::NonCanonicalOrder, path);
                break;
            }
            std::cmp::Ordering::Less => {}
        }
    }
}

fn expected_replica(source: &PresenceV1<SourceCoordinateV1>) -> Option<SqlReplicaCoordinateV1> {
    match source {
        PresenceV1::Present(SourceCoordinateV1::Sqlite(value)) => {
            Some(SqlReplicaCoordinateV1::Sqlite(SqliteReplicaCoordinateV1 {
                database: value.database.clone(),
            }))
        }
        PresenceV1::Present(SourceCoordinateV1::Postgres(value)) => Some(
            SqlReplicaCoordinateV1::Postgres(PostgresReplicaCoordinateV1 {
                endpoint: value.endpoint.clone(),
                endpoint_id: value.endpoint_id,
            }),
        ),
        PresenceV1::Present(SourceCoordinateV1::Hybrid(value)) => Some(value.replica.clone()),
        _ => None,
    }
}

fn reconstruct_capture(
    method: &ObservationMethodV1,
    source: &PresenceV1<SourceCoordinateV1>,
    phases: &[PhaseObservationV1],
    errors: &mut ValidationCollector,
) -> Option<LegacyRawCaptureV1> {
    match method {
        ObservationMethodV1::MarkdownDoubleScan => {
            let first = markdown_raw_from_phase(phases.first()?)?;
            let second = markdown_raw_from_phase(phases.get(1)?)?;
            if first != second {
                errors.push(
                    CaptureValidationCodeV1::CaptureMismatch,
                    "$.observation.phases",
                );
            }
            Some(LegacyRawCaptureV1::Markdown(first))
        }
        ObservationMethodV1::SqliteReadTransaction => {
            let evidence = sql_from_phase(phases.first()?, &SqlDialectV1::Sqlite)?;
            sql_raw_from_evidence(evidence, SqlDialectV1::Sqlite).map(LegacyRawCaptureV1::Sqlite)
        }
        ObservationMethodV1::PostgresRepeatableReadOnly => {
            let evidence = sql_from_phase(phases.first()?, &SqlDialectV1::Postgres)?;
            sql_raw_from_evidence(evidence, SqlDialectV1::Postgres)
                .map(LegacyRawCaptureV1::Postgres)
        }
        ObservationMethodV1::HybridBracketedSqlSnapshot => {
            let PresenceV1::Present(SourceCoordinateV1::Hybrid(hybrid)) = source else {
                return None;
            };
            let PresenceV1::Present(policy) = &hybrid.policy else {
                return None;
            };
            let before = markdown_raw_from_phase(phases.first()?)?;
            let divergence_before = divergence_from_phase(phases.get(1)?)?;
            let dialect = match hybrid.replica {
                SqlReplicaCoordinateV1::Sqlite(_) => SqlDialectV1::Sqlite,
                SqlReplicaCoordinateV1::Postgres(_) => SqlDialectV1::Postgres,
            };
            let sql = sql_raw_from_evidence(sql_from_phase(phases.get(2)?, &dialect)?, dialect)?;
            let after = markdown_raw_from_phase(phases.get(3)?)?;
            let divergence_after = divergence_from_phase(phases.get(4)?)?;
            if before != after || divergence_before != divergence_after {
                errors.push(
                    CaptureValidationCodeV1::CaptureMismatch,
                    "$.observation.phases",
                );
            }
            Some(LegacyRawCaptureV1::Hybrid(HybridRawV1 {
                policy: policy.clone(),
                local: before,
                replica: sql,
                divergences: divergence_before,
            }))
        }
    }
}

fn markdown_raw_from_phase(phase: &PhaseObservationV1) -> Option<MarkdownRawV1> {
    let PhaseResultV1::Complete(result) = &phase.result else {
        return None;
    };
    let PhaseEvidenceV1::Markdown(evidence) = &result.evidence else {
        return None;
    };
    if !matches!(evidence.root, PresenceV1::Present(_))
        || !matches!(evidence.nodes.terminal, EnumerationTerminalV1::Complete)
    {
        return None;
    }
    let nodes = evidence
        .nodes
        .items
        .iter()
        .map(|node| match node {
            MarkdownNodeEvidenceV1::Captured(node) => Some(node.clone()),
            MarkdownNodeEvidenceV1::Unreadable(_) => None,
        })
        .collect::<Option<Vec<_>>>()?;
    Some(MarkdownRawV1 { nodes })
}

fn sql_from_phase<'a>(
    phase: &'a PhaseObservationV1,
    dialect: &SqlDialectV1,
) -> Option<&'a SqlEvidenceV1> {
    let PhaseResultV1::Complete(result) = &phase.result else {
        return None;
    };
    match (dialect, &result.evidence) {
        (SqlDialectV1::Sqlite, PhaseEvidenceV1::Sqlite(evidence))
        | (SqlDialectV1::Postgres, PhaseEvidenceV1::Postgres(evidence)) => Some(evidence),
        _ => None,
    }
}

fn divergence_from_phase(phase: &PhaseObservationV1) -> Option<FileImageV1> {
    let PhaseResultV1::Complete(result) = &phase.result else {
        return None;
    };
    let PhaseEvidenceV1::Divergences(ObservedValueV1::Complete(image)) = &result.evidence else {
        return None;
    };
    Some(image.clone())
}

fn sql_raw_from_evidence(evidence: &SqlEvidenceV1, dialect: SqlDialectV1) -> Option<SqlRawV1> {
    let PresenceV1::Present(namespace) = &evidence.catalog.namespace else {
        return None;
    };
    if !matches!(
        evidence.catalog.objects.terminal,
        EnumerationTerminalV1::Complete
    ) || !matches!(
        evidence.catalog.indexes.terminal,
        EnumerationTerminalV1::Complete
    ) || !matches!(
        evidence.catalog.foreign_objects.terminal,
        EnumerationTerminalV1::Complete
    ) || !evidence.catalog.rejected_rows.is_empty()
        || !evidence.rejected_rows.is_empty()
    {
        return None;
    }
    let tables = evidence
        .catalog
        .tables
        .iter()
        .map(table_schema_from_evidence)
        .collect::<Option<Vec<_>>>()?;
    let instances = completed_items(&evidence.instances)?.to_vec();
    let events = completed_items(&evidence.events)?.to_vec();
    let history = completed_items(&evidence.history)?.to_vec();
    let legacy_origins = completed_items(&evidence.legacy_origins)?.to_vec();
    let provider_sequences = match (&dialect, &evidence.provider_sequences) {
        (SqlDialectV1::Sqlite, ObservedSequenceRowsV1::NotApplicable) => {
            SequenceRowsV1::NotApplicable
        }
        (SqlDialectV1::Postgres, ObservedSequenceRowsV1::Observed(rows)) => {
            SequenceRowsV1::Rows(completed_items(rows)?.to_vec())
        }
        _ => return None,
    };
    Some(SqlRawV1 {
        dialect: dialect.clone(),
        schema: SqlSchemaV1 {
            dialect,
            namespace: namespace.clone(),
            tables,
            indexes: evidence.catalog.indexes.items.clone(),
            foreign_objects: evidence.catalog.foreign_objects.items.clone(),
        },
        instances,
        events,
        history,
        legacy_origins,
        provider_sequences,
    })
}

fn table_schema_from_evidence(table: &TableCatalogEvidenceV1) -> Option<TableSchemaV1> {
    let ObservedValueV1::Complete(catalog_definition) = &table.catalog_definition else {
        return None;
    };
    let ObservedValueV1::Complete(primary_key) = &table.primary_key else {
        return None;
    };
    Some(TableSchemaV1 {
        name: table.name.clone(),
        catalog_definition: catalog_definition.clone(),
        columns: completed_items(&table.columns)?.to_vec(),
        primary_key: primary_key.clone(),
        unique_keys: completed_items(&table.unique_keys)?.to_vec(),
        checks: completed_items(&table.checks)?.to_vec(),
    })
}

fn completed_items<T>(list: &ObservedListV1<T>) -> Option<&[T]> {
    matches!(list.terminal, EnumerationTerminalV1::Complete).then_some(list.items.as_slice())
}

fn changed_coordinates(
    method: &ObservationMethodV1,
    phases: &[PhaseObservationV1],
    errors: &mut ValidationCollector,
) -> Vec<PhysicalCoordinateV1> {
    let mut changed = match method {
        ObservationMethodV1::MarkdownDoubleScan => {
            match (
                phases.first().and_then(markdown_raw_from_phase),
                phases.get(1).and_then(markdown_raw_from_phase),
            ) {
                (Some(before), Some(after)) => markdown_changes(&before, &after),
                _ => Vec::new(),
            }
        }
        ObservationMethodV1::HybridBracketedSqlSnapshot => {
            let mut changed = match (
                phases.first().and_then(markdown_raw_from_phase),
                phases.get(3).and_then(markdown_raw_from_phase),
            ) {
                (Some(before), Some(after)) => markdown_changes(&before, &after),
                _ => Vec::new(),
            };
            match (
                phases.get(1).and_then(divergence_from_phase),
                phases.get(4).and_then(divergence_from_phase),
            ) {
                (Some(before), Some(after)) if before != after => {
                    changed.push(PhysicalCoordinateV1::Root(RootCoordinateV1::HybridSide(
                        HybridSideRootCoordinateV1 {
                            side: HybridSideV1::Divergences,
                        },
                    )));
                }
                (Some(_), Some(_)) => {}
                _ => errors.push(
                    CaptureValidationCodeV1::PartialEvidence,
                    "$.observation.phases",
                ),
            }
            changed
        }
        ObservationMethodV1::SqliteReadTransaction
        | ObservationMethodV1::PostgresRepeatableReadOnly => Vec::new(),
    };
    changed.sort_by_key(sort_key_v1);
    changed.dedup();
    changed
}

fn markdown_changes(before: &MarkdownRawV1, after: &MarkdownRawV1) -> Vec<PhysicalCoordinateV1> {
    let before: BTreeMap<_, _> = before
        .nodes
        .iter()
        .map(|node| (node.relative.clone(), node.node.clone()))
        .collect();
    let after: BTreeMap<_, _> = after
        .nodes
        .iter()
        .map(|node| (node.relative.clone(), node.node.clone()))
        .collect();
    before
        .keys()
        .chain(after.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|path| before.get(*path) != after.get(*path))
        .cloned()
        .map(|relative| PhysicalCoordinateV1::MarkdownPath(MarkdownPathCoordinateV1 { relative }))
        .collect()
}

fn validate_capture(capture: &LegacyRawCaptureV1, errors: &mut ValidationCollector) {
    match capture {
        LegacyRawCaptureV1::Markdown(markdown) => {
            validate_markdown_raw(markdown, "$.observation.capture.value", errors);
        }
        LegacyRawCaptureV1::Sqlite(sql) => {
            validate_sql_raw(
                sql,
                &SqlDialectV1::Sqlite,
                "$.observation.capture.value",
                errors,
            );
        }
        LegacyRawCaptureV1::Postgres(sql) => validate_sql_raw(
            sql,
            &SqlDialectV1::Postgres,
            "$.observation.capture.value",
            errors,
        ),
        LegacyRawCaptureV1::Hybrid(hybrid) => {
            if hybrid.policy.validate().is_err() {
                errors.push(
                    CaptureValidationCodeV1::HybridContradiction,
                    "$.observation.capture.value.policy",
                );
            }
            validate_markdown_raw(&hybrid.local, "$.observation.capture.value.local", errors);
            let dialect = hybrid.replica.dialect.clone();
            validate_sql_raw(
                &hybrid.replica,
                &dialect,
                "$.observation.capture.value.replica",
                errors,
            );
        }
    }
}

fn validate_markdown_raw(markdown: &MarkdownRawV1, path: &str, errors: &mut ValidationCollector) {
    ensure_strict_by(
        &markdown.nodes,
        |node| node.relative.clone(),
        &format!("{path}.nodes"),
        errors,
    );
    for (index, node) in markdown.nodes.iter().enumerate() {
        validate_markdown_node(node, &format!("{path}.nodes[{index}]"), errors);
    }
}

fn validate_markdown_node(node: &MarkdownNodeV1, path: &str, errors: &mut ValidationCollector) {
    if node.relative.validate_relative().is_err() {
        errors.push(
            CaptureValidationCodeV1::InvalidRelativePath,
            format!("{path}.relative"),
        );
    }
    if !matches!(
        node.node,
        MarkdownNodeKindV1::Directory | MarkdownNodeKindV1::Regular(_)
    ) {
        errors.push(
            CaptureValidationCodeV1::IncompatibleVariant,
            format!("{path}.node"),
        );
    }
    if matches!(node.node, MarkdownNodeKindV1::Regular(_))
        && !admitted_markdown_file(&node.relative)
    {
        errors.push(
            CaptureValidationCodeV1::UnsupportedSchema,
            format!("{path}.relative"),
        );
    }
}

fn required_markdown_node_refusal(node: &MarkdownNodeV1) -> Option<CaptureRefusalCodeV1> {
    match node.node {
        MarkdownNodeKindV1::Regular(_) if pending_batch_marker(&node.relative) => {
            Some(CaptureRefusalCodeV1::PendingBatchPresent)
        }
        MarkdownNodeKindV1::Regular(_) if !admitted_markdown_file(&node.relative) => {
            Some(CaptureRefusalCodeV1::ForeignMarkdownNode)
        }
        MarkdownNodeKindV1::Directory | MarkdownNodeKindV1::Regular(_) => None,
        MarkdownNodeKindV1::Symlink(_) | MarkdownNodeKindV1::Other(_) => {
            Some(CaptureRefusalCodeV1::ForeignMarkdownNode)
        }
    }
}

fn pending_batch_marker(path: &HostPathV1) -> bool {
    const PENDING_BATCH: &str = ".aep-batch.pending.json";
    match path {
        HostPathV1::Unix(bytes) => bytes.as_bytes() == PENDING_BATCH.as_bytes(),
        HostPathV1::Windows(units) => units.iter().copied().eq(PENDING_BATCH.encode_utf16()),
    }
}

#[allow(clippy::case_sensitive_file_extension_comparisons)] // The wire grammar requires lowercase .md.
fn admitted_markdown_file(path: &HostPathV1) -> bool {
    match path {
        HostPathV1::Unix(bytes) => {
            let Ok(path) = std::str::from_utf8(bytes.as_bytes()) else {
                return false;
            };
            path == "journal.jsonl"
                || (path.split('/').count() == 2
                    && path
                        .rsplit_once('/')
                        .is_some_and(|(_, name)| name.ends_with(".md") && name.len() > 3))
        }
        HostPathV1::Windows(units) => {
            const JOURNAL: &[u16] = &[
                0x006a, 0x006f, 0x0075, 0x0072, 0x006e, 0x0061, 0x006c, 0x002e, 0x006a, 0x0073,
                0x006f, 0x006e, 0x006c,
            ];
            if units == JOURNAL {
                return true;
            }
            let mut components = units.split(|unit| *unit == 0x005c);
            let Some(_kind) = components.next() else {
                return false;
            };
            let Some(name) = components.next() else {
                return false;
            };
            components.next().is_none()
                && name.len() > 3
                && name.ends_with(&[0x002e, 0x006d, 0x0064])
        }
    }
}

fn validate_sql_raw(
    sql: &SqlRawV1,
    expected: &SqlDialectV1,
    path: &str,
    errors: &mut ValidationCollector,
) {
    if sql.dialect != *expected || sql.schema.dialect != *expected {
        errors.push(
            CaptureValidationCodeV1::IncompatibleVariant,
            format!("{path}.dialect"),
        );
    }
    if !known_schema_shape(&sql.schema, expected) {
        errors.push(
            CaptureValidationCodeV1::UnsupportedSchema,
            format!("{path}.schema"),
        );
    }
    match (expected, &sql.provider_sequences) {
        (SqlDialectV1::Sqlite, SequenceRowsV1::NotApplicable)
        | (SqlDialectV1::Postgres, SequenceRowsV1::Rows(_)) => {}
        _ => errors.push(
            CaptureValidationCodeV1::IncompatibleVariant,
            format!("{path}.provider_sequences"),
        ),
    }
    ensure_strict_by(
        &sql.schema.tables,
        |table| table.name.as_bytes().to_vec(),
        &format!("{path}.schema.tables"),
        errors,
    );
    ensure_strict_by(
        &sql.schema.indexes,
        |index| index.name.as_bytes().to_vec(),
        &format!("{path}.schema.indexes"),
        errors,
    );
    ensure_strict_sort(
        &sql.schema.foreign_objects,
        &format!("{path}.schema.foreign_objects"),
        CaptureValidationCodeV1::NonCanonicalOrder,
        errors,
    );
    ensure_strict_by(
        &sql.instances,
        |row| (row.entity.clone(), row.id.clone()),
        &format!("{path}.instances"),
        errors,
    );
    ensure_strict_by(
        &sql.events,
        |row| {
            (
                row.entity.clone(),
                row.id.clone(),
                row.revision,
                row.position,
            )
        },
        &format!("{path}.events"),
        errors,
    );
    ensure_strict_by(
        &sql.history,
        |row| (row.entity.clone(), row.id.clone(), row.position),
        &format!("{path}.history"),
        errors,
    );
    ensure_strict_by(
        &sql.legacy_origins,
        |row| (row.entity.clone(), row.id.clone()),
        &format!("{path}.legacy_origins"),
        errors,
    );
    if let SequenceRowsV1::Rows(rows) = &sql.provider_sequences {
        ensure_strict_by(
            rows,
            |row| row.namespace.as_bytes().to_vec(),
            &format!("{path}.provider_sequences.value"),
            errors,
        );
    }
}

fn expected_namespace(source: &PresenceV1<SourceCoordinateV1>) -> Option<&str> {
    match source {
        PresenceV1::Present(SourceCoordinateV1::Sqlite(_)) => Some("main"),
        PresenceV1::Present(SourceCoordinateV1::Postgres(value)) => {
            Some(value.endpoint.schema.as_str())
        }
        PresenceV1::Present(SourceCoordinateV1::Hybrid(value)) => match &value.replica {
            SqlReplicaCoordinateV1::Sqlite(_) => Some("main"),
            SqlReplicaCoordinateV1::Postgres(value) => Some(value.endpoint.schema.as_str()),
        },
        _ => None,
    }
}

#[allow(clippy::too_many_lines)] // The explicit known-DDL roster is clearest as one comparison table.
fn known_schema_shape(schema: &SqlSchemaV1, dialect: &SqlDialectV1) -> bool {
    let expected_names: &[&str] = match dialect {
        SqlDialectV1::Sqlite => &["events", "history", "instances", "legacy_origins"],
        SqlDialectV1::Postgres => &[
            "events",
            "history",
            "instances",
            "legacy_origins",
            "provider_sequences",
        ],
    };
    if schema.tables.len() != expected_names.len()
        || schema
            .tables
            .iter()
            .map(|table| table.name.as_str())
            .ne(expected_names.iter().copied())
    {
        return false;
    }
    for table in &schema.tables {
        let (columns, primary_key, unique, check) = match table.name.as_str() {
            "events" => (
                &["entity", "id", "revision", "position", "document"][..],
                &["entity", "id", "revision", "position"][..],
                None,
                None,
            ),
            "history" => (
                &["entity", "id", "position", "kind", "record_id", "document"][..],
                &["entity", "id", "position"][..],
                Some(&["record_id"][..]),
                Some("history"),
            ),
            "instances" => (
                &["entity", "id", "revision", "document"][..],
                &["entity", "id"][..],
                None,
                None,
            ),
            "legacy_origins" => (
                &["entity", "id", "revision"][..],
                &["entity", "id"][..],
                None,
                None,
            ),
            "provider_sequences" if *dialect == SqlDialectV1::Postgres => (
                &["namespace", "next_value"][..],
                &["namespace"][..],
                None,
                Some("sequence"),
            ),
            _ => return false,
        };
        if table
            .columns
            .iter()
            .enumerate()
            .any(|(ordinal, column)| usize::from(column.ordinal) != ordinal)
            || table
                .columns
                .iter()
                .map(|column| column.name.as_str())
                .ne(columns.iter().copied())
            || table.columns.iter().any(|column| {
                !column.not_null
                    || !known_column_type(&column.name, &column.sql_type, dialect)
                    || !matches!(column.default, PresenceV1::Missing)
                    || !matches!(column.explicit_collation, PresenceV1::Missing)
            })
        {
            return false;
        }
        let PresenceV1::Present(primary) = &table.primary_key else {
            return false;
        };
        if primary
            .columns
            .iter()
            .map(String::as_str)
            .ne(primary_key.iter().copied())
        {
            return false;
        }
        match unique {
            Some(columns) => {
                if table.unique_keys.len() != 1
                    || table.unique_keys[0]
                        .columns
                        .iter()
                        .map(String::as_str)
                        .ne(columns.iter().copied())
                {
                    return false;
                }
            }
            None if !table.unique_keys.is_empty() => return false,
            None => {}
        }
        match check {
            Some("history")
                if matches!(
                    table.checks.as_slice(),
                    [CheckSchemaV1::HistoryKindDecisionObservation(_)]
                ) => {}
            Some("sequence")
                if matches!(
                    table.checks.as_slice(),
                    [CheckSchemaV1::NextValueNonnegative(_)]
                ) => {}
            None if table.checks.is_empty() => {}
            _ => return false,
        }
    }
    match dialect {
        SqlDialectV1::Sqlite => schema.indexes.is_empty() && schema.foreign_objects.is_empty(),
        SqlDialectV1::Postgres => {
            matches!(
                schema.indexes.as_slice(),
                [IndexSchemaV1 {
                    name,
                    table,
                    unique: false,
                    method: IndexMethodV1::Gin,
                    terms,
                    predicate: PresenceV1::Missing,
                    ..
                }] if name == "instances_document_query"
                    && table == "instances"
                    && matches!(terms.as_slice(), [IndexTermV1::DocumentJsonbPathOps(value)]
                        if value.opclass == "jsonb_path_ops")
            ) && schema.foreign_objects.is_empty()
        }
    }
}

fn known_column_type(name: &str, sql_type: &SqlTypeV1, dialect: &SqlDialectV1) -> bool {
    if matches!(
        name,
        "entity" | "id" | "document" | "kind" | "record_id" | "namespace"
    ) {
        return *sql_type == SqlTypeV1::Text;
    }
    match dialect {
        SqlDialectV1::Sqlite => *sql_type == SqlTypeV1::Integer,
        SqlDialectV1::Postgres => *sql_type == SqlTypeV1::BigInt,
    }
}

fn serde_json_error_path(error: &serde_json::Error) -> String {
    format!("$@{}:{}", error.line(), error.column())
}

fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[cfg(test)]
#[allow(
    clippy::comparison_chain,
    clippy::manual_let_else,
    clippy::match_wildcard_for_single_variants,
    clippy::needless_pass_by_value,
    clippy::too_many_lines
)]
mod tests {
    use super::*;
    use crate::migration::{config_digest_v1, selector_digest_v1};

    fn unix(value: &str) -> HostPathV1 {
        HostPathV1::Unix(HexBytesV1::new(value.as_bytes().to_vec()))
    }

    fn complete_list<T>(items: Vec<T>) -> ObservedListV1<T> {
        ObservedListV1 {
            items,
            terminal: EnumerationTerminalV1::Complete,
        }
    }

    fn empty_markdown(root: &str) -> PhaseEvidenceV1 {
        PhaseEvidenceV1::Markdown(MarkdownEvidenceV1 {
            root: PresenceV1::Present(unix(root)),
            nodes: complete_list(Vec::new()),
        })
    }

    fn complete_phase(phase: CapturePhaseV1, evidence: PhaseEvidenceV1) -> PhaseObservationV1 {
        let evidence_digest = evidence.evidence_digest();
        PhaseObservationV1 {
            phase,
            result: PhaseResultV1::Complete(CompletePhaseResultV1 {
                evidence,
                evidence_digest,
            }),
        }
    }

    fn refusal() -> CaptureRefusalV1 {
        CaptureRefusalV1 {
            code: CaptureRefusalCodeV1::ReadFailure,
            at: PhysicalCoordinateV1::Root(RootCoordinateV1::Observation),
        }
    }

    fn refused_phase(phase: CapturePhaseV1, evidence: PhaseEvidenceV1) -> PhaseObservationV1 {
        let evidence_digest = evidence.evidence_digest();
        PhaseObservationV1 {
            phase,
            result: PhaseResultV1::Refused(RefusedPhaseResultV1 {
                evidence,
                evidence_digest,
                refusals: vec![refusal()],
            }),
        }
    }

    fn selector() -> SelectorCoordinateV1 {
        SelectorCoordinateV1 {
            project_root: unix("repo"),
            project_file: unix(".engineering/project.yaml"),
            presence: SelectorPresenceV1::MissingV1Default,
            store_field: StoreFieldV1::MissingDefault,
        }
    }

    fn config_digest() -> DigestV1 {
        digest_parts_v1("aep.migration.source-config/1", &[Vec::new()]).expect("digest")
    }

    fn complete_markdown_fixture() -> RawCaptureObservationV1 {
        let phases = vec![
            complete_phase(CapturePhaseV1::MarkdownFirst, empty_markdown("planning")),
            complete_phase(CapturePhaseV1::MarkdownSecond, empty_markdown("planning")),
        ];
        let capture = LegacyRawCaptureV1::Markdown(MarkdownRawV1 { nodes: Vec::new() });
        let mut observation = RawCaptureObservationV1 {
            format: RawCaptureFormatV1,
            source: PresenceV1::Present(SourceCoordinateV1::Markdown(MarkdownSourceCoordinateV1 {
                root: unix("planning"),
            })),
            selector: PresenceV1::Present(selector()),
            config_digest: PresenceV1::Present(config_digest()),
            observation: ObservationOutcomeV1::Complete(CompleteObservationV1 {
                method: ObservationMethodV1::MarkdownDoubleScan,
                phases,
                capture: capture.clone(),
                transcript_digest: DigestV1::from_bytes([0; 32]),
                raw_snapshot_id: DigestV1::from_bytes([0; 32]),
            }),
        };
        let transcript_digest = observation
            .complete_transcript_digest(&capture)
            .expect("transcript digest");
        let raw_snapshot_id = observation
            .complete_snapshot_id(&capture)
            .expect("snapshot digest");
        let ObservationOutcomeV1::Complete(complete) = &mut observation.observation else {
            unreachable!();
        };
        complete.transcript_digest = transcript_digest;
        complete.raw_snapshot_id = raw_snapshot_id;
        observation
    }

    fn known_catalog(dialect: SqlDialectV1) -> SqlCatalogEvidenceV1 {
        let numeric = match &dialect {
            SqlDialectV1::Sqlite => SqlTypeV1::Integer,
            SqlDialectV1::Postgres => SqlTypeV1::BigInt,
        };
        let column = |ordinal: u16, name: &str, sql_type: SqlTypeV1| ColumnSchemaV1 {
            ordinal,
            name: name.to_owned(),
            sql_type,
            declared_type: HexBytesV1::new(Vec::new()),
            catalog_type_id: PresenceV1::Missing,
            not_null: true,
            default: PresenceV1::Missing,
            explicit_collation: PresenceV1::Missing,
        };
        let key = |columns: &[&str]| KeySchemaV1 {
            name: PresenceV1::Missing,
            backing_index: PresenceV1::Missing,
            columns: columns.iter().map(|value| (*value).to_owned()).collect(),
            catalog_definition: PresenceV1::Missing,
        };
        let mut tables = vec![
            TableSchemaV1 {
                name: "events".into(),
                catalog_definition: PresenceV1::Missing,
                columns: vec![
                    column(0, "entity", SqlTypeV1::Text),
                    column(1, "id", SqlTypeV1::Text),
                    column(2, "revision", numeric.clone()),
                    column(3, "position", numeric.clone()),
                    column(4, "document", SqlTypeV1::Text),
                ],
                primary_key: PresenceV1::Present(key(&["entity", "id", "revision", "position"])),
                unique_keys: Vec::new(),
                checks: Vec::new(),
            },
            TableSchemaV1 {
                name: "history".into(),
                catalog_definition: PresenceV1::Missing,
                columns: vec![
                    column(0, "entity", SqlTypeV1::Text),
                    column(1, "id", SqlTypeV1::Text),
                    column(2, "position", numeric.clone()),
                    column(3, "kind", SqlTypeV1::Text),
                    column(4, "record_id", SqlTypeV1::Text),
                    column(5, "document", SqlTypeV1::Text),
                ],
                primary_key: PresenceV1::Present(key(&["entity", "id", "position"])),
                unique_keys: vec![key(&["record_id"])],
                checks: vec![CheckSchemaV1::HistoryKindDecisionObservation(
                    HistoryCheckSchemaV1 {
                        name: PresenceV1::Missing,
                        catalog_expression: HexBytesV1::new(Vec::new()),
                        catalog_definition: PresenceV1::Missing,
                    },
                )],
            },
            TableSchemaV1 {
                name: "instances".into(),
                catalog_definition: PresenceV1::Missing,
                columns: vec![
                    column(0, "entity", SqlTypeV1::Text),
                    column(1, "id", SqlTypeV1::Text),
                    column(2, "revision", numeric.clone()),
                    column(3, "document", SqlTypeV1::Text),
                ],
                primary_key: PresenceV1::Present(key(&["entity", "id"])),
                unique_keys: Vec::new(),
                checks: Vec::new(),
            },
            TableSchemaV1 {
                name: "legacy_origins".into(),
                catalog_definition: PresenceV1::Missing,
                columns: vec![
                    column(0, "entity", SqlTypeV1::Text),
                    column(1, "id", SqlTypeV1::Text),
                    column(2, "revision", numeric.clone()),
                ],
                primary_key: PresenceV1::Present(key(&["entity", "id"])),
                unique_keys: Vec::new(),
                checks: Vec::new(),
            },
        ];
        if dialect == SqlDialectV1::Postgres {
            tables.push(TableSchemaV1 {
                name: "provider_sequences".into(),
                catalog_definition: PresenceV1::Missing,
                columns: vec![
                    column(0, "namespace", SqlTypeV1::Text),
                    column(1, "next_value", numeric),
                ],
                primary_key: PresenceV1::Present(key(&["namespace"])),
                unique_keys: Vec::new(),
                checks: vec![CheckSchemaV1::NextValueNonnegative(
                    NextValueCheckSchemaV1 {
                        name: PresenceV1::Missing,
                        catalog_expression: HexBytesV1::new(Vec::new()),
                        catalog_definition: PresenceV1::Missing,
                    },
                )],
            });
        }
        let indexes = if dialect == SqlDialectV1::Postgres {
            vec![IndexSchemaV1 {
                name: "instances_document_query".into(),
                table: "instances".into(),
                unique: false,
                method: IndexMethodV1::Gin,
                terms: vec![IndexTermV1::DocumentJsonbPathOps(DocumentJsonbPathOpsV1 {
                    catalog_expression: HexBytesV1::new(Vec::new()),
                    opclass: "jsonb_path_ops".into(),
                })],
                predicate: PresenceV1::Missing,
                catalog_definition: HexBytesV1::new(Vec::new()),
            }]
        } else {
            Vec::new()
        };
        let mut objects = tables
            .iter()
            .map(|table| CatalogObjectHeaderV1 {
                kind: ForeignObjectKindV1::Table,
                name: table.name.clone(),
                parent: PresenceV1::Missing,
            })
            .chain(indexes.iter().map(|index| CatalogObjectHeaderV1 {
                kind: ForeignObjectKindV1::Index,
                name: index.name.clone(),
                parent: PresenceV1::Present(index.table.clone()),
            }))
            .collect::<Vec<_>>();
        objects.sort_by_key(catalog_header_order_key);
        SqlCatalogEvidenceV1 {
            namespace: PresenceV1::Present(if dialect == SqlDialectV1::Sqlite {
                "main".to_owned()
            } else {
                "public".to_owned()
            }),
            objects: complete_list(objects),
            tables: tables
                .into_iter()
                .map(|table| TableCatalogEvidenceV1 {
                    name: table.name,
                    catalog_definition: ObservedValueV1::Complete(table.catalog_definition),
                    columns: complete_list(table.columns),
                    primary_key: ObservedValueV1::Complete(table.primary_key),
                    unique_keys: complete_list(table.unique_keys),
                    checks: complete_list(table.checks),
                })
                .collect(),
            indexes: complete_list(indexes),
            foreign_objects: complete_list(Vec::new()),
            rejected_rows: Vec::new(),
        }
    }

    fn empty_sql_evidence(replica: SqlReplicaCoordinateV1) -> PhaseEvidenceV1 {
        let dialect = match &replica {
            SqlReplicaCoordinateV1::Sqlite(_) => SqlDialectV1::Sqlite,
            SqlReplicaCoordinateV1::Postgres(_) => SqlDialectV1::Postgres,
        };
        let sequences = match &replica {
            SqlReplicaCoordinateV1::Sqlite(_) => ObservedSequenceRowsV1::NotApplicable,
            SqlReplicaCoordinateV1::Postgres(_) => {
                ObservedSequenceRowsV1::Observed(complete_list(Vec::new()))
            }
        };
        let sql = SqlEvidenceV1 {
            source: PresenceV1::Present(replica.clone()),
            catalog: known_catalog(dialect),
            instances: complete_list(Vec::new()),
            events: complete_list(Vec::new()),
            history: complete_list(Vec::new()),
            legacy_origins: complete_list(Vec::new()),
            provider_sequences: sequences,
            rejected_rows: Vec::new(),
        };
        match replica {
            SqlReplicaCoordinateV1::Sqlite(_) => PhaseEvidenceV1::Sqlite(sql),
            SqlReplicaCoordinateV1::Postgres(_) => PhaseEvidenceV1::Postgres(sql),
        }
    }

    fn sqlite_replica() -> SqlReplicaCoordinateV1 {
        SqlReplicaCoordinateV1::Sqlite(SqliteReplicaCoordinateV1 {
            database: unix("store.sqlite"),
        })
    }

    fn postgres_replica() -> SqlReplicaCoordinateV1 {
        let endpoint = PostgresEndpointV1 {
            hosts: vec![PostgresHostV1::Tcp(TcpPostgresHostV1 {
                host: "database.internal".to_owned(),
                port: 5432,
            })],
            database: "planning".to_owned(),
            schema: "public".to_owned(),
        };
        SqlReplicaCoordinateV1::Postgres(PostgresReplicaCoordinateV1 {
            endpoint_id: endpoint.endpoint_id(),
            endpoint,
        })
    }

    fn hybrid_fixture(
        policy: HybridPolicyWordsV1,
        replica: SqlReplicaCoordinateV1,
    ) -> RawCaptureObservationV1 {
        let local = MarkdownRawV1 { nodes: Vec::new() };
        let divergences = FileImageV1::Absent;
        let sql_evidence = empty_sql_evidence(replica.clone());
        let sql = match &sql_evidence {
            PhaseEvidenceV1::Sqlite(evidence) => {
                sql_raw_from_evidence(evidence, SqlDialectV1::Sqlite).expect("complete SQL")
            }
            PhaseEvidenceV1::Postgres(evidence) => {
                sql_raw_from_evidence(evidence, SqlDialectV1::Postgres).expect("complete SQL")
            }
            _ => unreachable!(),
        };
        let phases = vec![
            complete_phase(CapturePhaseV1::LocalBefore, empty_markdown("planning")),
            complete_phase(
                CapturePhaseV1::DivergencesBefore,
                PhaseEvidenceV1::Divergences(ObservedValueV1::Complete(divergences.clone())),
            ),
            complete_phase(CapturePhaseV1::ReplicaSnapshot, sql_evidence),
            complete_phase(CapturePhaseV1::LocalAfter, empty_markdown("planning")),
            complete_phase(
                CapturePhaseV1::DivergencesAfter,
                PhaseEvidenceV1::Divergences(ObservedValueV1::Complete(divergences.clone())),
            ),
        ];
        let capture = LegacyRawCaptureV1::Hybrid(HybridRawV1 {
            policy: policy.clone(),
            local,
            replica: sql,
            divergences,
        });
        let mut observation = RawCaptureObservationV1 {
            format: RawCaptureFormatV1,
            source: PresenceV1::Present(SourceCoordinateV1::Hybrid(HybridSourceCoordinateV1 {
                local_root: unix("planning"),
                replica,
                divergence_file: unix("divergences.json"),
                policy: PresenceV1::Present(policy),
            })),
            selector: PresenceV1::Present(selector()),
            config_digest: PresenceV1::Present(config_digest()),
            observation: ObservationOutcomeV1::Complete(CompleteObservationV1 {
                method: ObservationMethodV1::HybridBracketedSqlSnapshot,
                phases,
                capture: capture.clone(),
                transcript_digest: DigestV1::from_bytes([0; 32]),
                raw_snapshot_id: DigestV1::from_bytes([0; 32]),
            }),
        };
        let transcript = observation
            .complete_transcript_digest(&capture)
            .expect("transcript");
        let snapshot = observation
            .complete_snapshot_id(&capture)
            .expect("snapshot");
        let ObservationOutcomeV1::Complete(complete) = &mut observation.observation else {
            unreachable!();
        };
        complete.transcript_digest = transcript;
        complete.raw_snapshot_id = snapshot;
        observation
    }

    fn refresh_complete_digests(observation: &mut RawCaptureObservationV1) {
        let ObservationOutcomeV1::Complete(complete) = &observation.observation else {
            panic!("fixture must be complete");
        };
        let capture = complete.capture.clone();
        let transcript = observation
            .complete_transcript_digest(&capture)
            .expect("complete transcript");
        let snapshot = observation
            .complete_snapshot_id(&capture)
            .expect("complete snapshot");
        let ObservationOutcomeV1::Complete(complete) = &mut observation.observation else {
            unreachable!();
        };
        complete.transcript_digest = transcript;
        complete.raw_snapshot_id = snapshot;
    }

    fn refused_sqlite_at_step(fail_at: usize) -> RawCaptureObservationV1 {
        let replica = sqlite_replica();
        let PhaseEvidenceV1::Sqlite(mut evidence) = empty_sql_evidence(replica.clone()) else {
            unreachable!();
        };
        let mut cursor = 0_usize;
        let mut failure = None;
        macro_rules! list_step {
            ($list:expr, $at:expr) => {{
                let at = $at;
                if cursor == fail_at {
                    $list.terminal = EnumerationTerminalV1::Refused(EnumerationRefusalV1 {
                        at: at.clone(),
                        code: CaptureRefusalCodeV1::ReadFailure,
                    });
                    failure = Some(CaptureRefusalV1 {
                        at,
                        code: CaptureRefusalCodeV1::ReadFailure,
                    });
                } else if cursor > fail_at {
                    $list.items.clear();
                    $list.terminal = EnumerationTerminalV1::NotAttempted;
                }
                cursor += 1;
            }};
        }
        macro_rules! value_step {
            ($value:expr, $at:expr) => {{
                let at = $at;
                if cursor == fail_at {
                    $value = ObservedValueV1::Refused(ObservedValueRefusalV1 {
                        at: at.clone(),
                        code: CaptureRefusalCodeV1::ReadFailure,
                    });
                    failure = Some(CaptureRefusalV1 {
                        at,
                        code: CaptureRefusalCodeV1::ReadFailure,
                    });
                } else if cursor > fail_at {
                    $value = ObservedValueV1::NotAttempted;
                }
                cursor += 1;
            }};
        }
        let catalog_coordinate = |family, table: PresenceV1<String>| {
            PhysicalCoordinateV1::SqlCatalog(SqlCatalogCoordinateV1 {
                namespace: PresenceV1::Present("main".into()),
                family,
                table,
                row: PresenceV1::Missing,
            })
        };
        list_step!(
            evidence.catalog.objects,
            catalog_coordinate(CatalogFamilyV1::Objects, PresenceV1::Missing)
        );
        for table in &mut evidence.catalog.tables {
            let table_name = PresenceV1::Present(table.name.clone());
            value_step!(
                table.catalog_definition,
                catalog_coordinate(CatalogFamilyV1::TableDefinition, table_name.clone())
            );
            list_step!(
                table.columns,
                catalog_coordinate(CatalogFamilyV1::Columns, table_name.clone())
            );
            value_step!(
                table.primary_key,
                catalog_coordinate(CatalogFamilyV1::PrimaryKey, table_name.clone())
            );
            list_step!(
                table.unique_keys,
                catalog_coordinate(CatalogFamilyV1::UniqueKeys, table_name.clone())
            );
            list_step!(
                table.checks,
                catalog_coordinate(CatalogFamilyV1::Checks, table_name)
            );
        }
        list_step!(
            evidence.catalog.indexes,
            catalog_coordinate(CatalogFamilyV1::Indexes, PresenceV1::Missing)
        );
        list_step!(
            evidence.catalog.foreign_objects,
            catalog_coordinate(CatalogFamilyV1::ForeignObjects, PresenceV1::Missing)
        );
        for (table, list) in [
            (
                "instances",
                &mut evidence.instances as &mut dyn TestObservedList,
            ),
            ("events", &mut evidence.events as &mut dyn TestObservedList),
            (
                "history",
                &mut evidence.history as &mut dyn TestObservedList,
            ),
            (
                "legacy_origins",
                &mut evidence.legacy_origins as &mut dyn TestObservedList,
            ),
        ] {
            let at = PhysicalCoordinateV1::SqlTable(SqlTableCoordinateV1 {
                table: table.into(),
            });
            if cursor == fail_at {
                list.refuse(at.clone());
                failure = Some(CaptureRefusalV1 {
                    at,
                    code: CaptureRefusalCodeV1::ReadFailure,
                });
            } else if cursor > fail_at {
                list.mark_not_attempted();
            }
            cursor += 1;
        }
        assert!(fail_at < cursor, "test step must exist");
        let failure = failure.expect("one read step fails");
        let evidence = PhaseEvidenceV1::Sqlite(evidence);
        let evidence_digest = evidence.evidence_digest();
        RawCaptureObservationV1 {
            format: RawCaptureFormatV1,
            source: PresenceV1::Present(SourceCoordinateV1::Sqlite(SqliteSourceCoordinateV1 {
                database: match replica {
                    SqlReplicaCoordinateV1::Sqlite(value) => value.database,
                    _ => unreachable!(),
                },
            })),
            selector: PresenceV1::Present(selector()),
            config_digest: PresenceV1::Present(config_digest()),
            observation: ObservationOutcomeV1::Refused(RefusedObservationV1 {
                method: PresenceV1::Present(ObservationMethodV1::SqliteReadTransaction),
                phases: vec![PhaseObservationV1 {
                    phase: CapturePhaseV1::SqlSnapshot,
                    result: PhaseResultV1::Refused(RefusedPhaseResultV1 {
                        evidence,
                        evidence_digest,
                        refusals: vec![failure],
                    }),
                }],
                preflight_refusals: Vec::new(),
            }),
        }
    }

    trait TestObservedList {
        fn refuse(&mut self, at: PhysicalCoordinateV1);
        fn mark_not_attempted(&mut self);
    }

    impl<T> TestObservedList for ObservedListV1<T> {
        fn refuse(&mut self, at: PhysicalCoordinateV1) {
            self.terminal = EnumerationTerminalV1::Refused(EnumerationRefusalV1 {
                at,
                code: CaptureRefusalCodeV1::ReadFailure,
            });
        }

        fn mark_not_attempted(&mut self) {
            self.items.clear();
            self.terminal = EnumerationTerminalV1::NotAttempted;
        }
    }

    #[test]
    fn exact_adjacent_envelopes_reject_unit_payloads_and_payload_omissions() {
        assert_eq!(
            serde_json::to_value(StoreFieldV1::Present).expect("serialises"),
            serde_json::json!({"kind": "present"})
        );
        for invalid in [
            serde_json::json!({"kind": "present", "value": null}),
            serde_json::json!({"kind": "present", "extra": true}),
            serde_json::json!({"value": null}),
        ] {
            assert!(serde_json::from_value::<StoreFieldV1>(invalid).is_err());
        }
        assert!(serde_json::from_str::<StoreFieldV1>(
            r#"{"kind":"present","kind":"missing_default"}"#
        )
        .is_err());
        assert!(serde_json::from_value::<SqlTypeV1>(serde_json::json!({
            "kind": "foreign"
        }))
        .is_err());
        assert!(serde_json::from_value::<SqlTypeV1>(serde_json::json!({
            "kind": "foreign",
            "value": null
        }))
        .is_err());
        assert!(serde_json::from_str::<SourceCoordinateV1>(
            r#"{"kind":"markdown","value":{"root":{"kind":"unix","value":"hex:61"},"root":{"kind":"unix","value":"hex:62"}}}"#
        )
        .is_err());
    }

    #[test]
    fn every_concrete_enum_variant_round_trips_through_its_exact_envelope() {
        fn round_trip<T>(value: T)
        where
            T: Serialize + serde::de::DeserializeOwned + PartialEq + fmt::Debug,
        {
            let json = serde_json::to_vec(&value).expect("serialises");
            assert_eq!(
                serde_json::from_slice::<T>(&json).expect("deserialises"),
                value
            );
        }

        round_trip(PresenceV1::<u16>::Missing);
        round_trip(PresenceV1::Present(7_u16));
        for value in [
            SelectorPresenceV1::MissingV1Default,
            SelectorPresenceV1::Present(SelectorPresentV1 {
                selector_digest: config_digest(),
            }),
        ] {
            round_trip(value);
        }
        for value in [StoreFieldV1::MissingDefault, StoreFieldV1::Present] {
            round_trip(value);
        }
        for value in [
            HostPathV1::Unix(HexBytesV1::new(vec![0xff])),
            HostPathV1::Windows(vec![0xd800]),
        ] {
            round_trip(value);
        }
        let postgres = postgres_replica();
        let SqlReplicaCoordinateV1::Postgres(postgres_value) = postgres.clone() else {
            unreachable!();
        };
        for value in [
            SqlReplicaCoordinateV1::Sqlite(SqliteReplicaCoordinateV1 {
                database: unix("db"),
            }),
            postgres.clone(),
        ] {
            round_trip(value);
        }
        for value in [
            SourceCoordinateV1::Markdown(MarkdownSourceCoordinateV1 { root: unix("root") }),
            SourceCoordinateV1::Sqlite(SqliteSourceCoordinateV1 {
                database: unix("db"),
            }),
            SourceCoordinateV1::Postgres(PostgresSourceCoordinateV1 {
                endpoint: postgres_value.endpoint.clone(),
                endpoint_id: postgres_value.endpoint_id,
            }),
            SourceCoordinateV1::Hybrid(HybridSourceCoordinateV1 {
                local_root: unix("root"),
                replica: postgres.clone(),
                divergence_file: unix("divergences"),
                policy: PresenceV1::Missing,
            }),
        ] {
            round_trip(value);
        }
        for value in [
            PostgresHostV1::Tcp(TcpPostgresHostV1 {
                host: "db".into(),
                port: 5432,
            }),
            PostgresHostV1::Unix(UnixPostgresHostV1 {
                directory: unix("socket"),
                port: 5432,
            }),
        ] {
            round_trip(value);
        }
        for value in [
            ObservationMethodV1::MarkdownDoubleScan,
            ObservationMethodV1::SqliteReadTransaction,
            ObservationMethodV1::PostgresRepeatableReadOnly,
            ObservationMethodV1::HybridBracketedSqlSnapshot,
        ] {
            round_trip(value);
        }
        round_trip(complete_markdown_fixture().observation);
        round_trip(ObservationOutcomeV1::Unstable(UnstableObservationV1 {
            method: ObservationMethodV1::MarkdownDoubleScan,
            phases: Vec::new(),
            changed: Vec::new(),
        }));
        round_trip(ObservationOutcomeV1::Refused(RefusedObservationV1 {
            method: PresenceV1::Missing,
            phases: Vec::new(),
            preflight_refusals: vec![refusal()],
        }));
        for value in [
            CapturePhaseV1::MarkdownFirst,
            CapturePhaseV1::MarkdownSecond,
            CapturePhaseV1::SqlSnapshot,
            CapturePhaseV1::LocalBefore,
            CapturePhaseV1::DivergencesBefore,
            CapturePhaseV1::ReplicaSnapshot,
            CapturePhaseV1::LocalAfter,
            CapturePhaseV1::DivergencesAfter,
        ] {
            round_trip(value);
        }
        let markdown = empty_markdown("root");
        let sqlite = empty_sql_evidence(sqlite_replica());
        let postgres_evidence = empty_sql_evidence(postgres.clone());
        let divergence =
            PhaseEvidenceV1::Divergences(ObservedValueV1::Complete(FileImageV1::Absent));
        for value in [markdown.clone(), sqlite, postgres_evidence, divergence] {
            round_trip(value);
        }
        round_trip(PhaseResultV1::NotAttempted);
        round_trip(complete_phase(CapturePhaseV1::MarkdownFirst, markdown.clone()).result);
        round_trip(refused_phase(CapturePhaseV1::MarkdownFirst, markdown).result);
        for value in [
            EnumerationTerminalV1::NotAttempted,
            EnumerationTerminalV1::Complete,
            EnumerationTerminalV1::Refused(EnumerationRefusalV1 {
                at: PhysicalCoordinateV1::Root(RootCoordinateV1::Observation),
                code: CaptureRefusalCodeV1::ReadFailure,
            }),
        ] {
            round_trip(value);
        }
        round_trip(ObservedValueV1::<u16>::NotAttempted);
        round_trip(ObservedValueV1::Complete(1_u16));
        round_trip(ObservedValueV1::<u16>::Refused(ObservedValueRefusalV1 {
            at: PhysicalCoordinateV1::Root(RootCoordinateV1::Observation),
            code: CaptureRefusalCodeV1::ReadFailure,
        }));
        round_trip(MarkdownNodeEvidenceV1::Captured(MarkdownNodeV1 {
            relative: unix("kind/item.md"),
            node: MarkdownNodeKindV1::Directory,
        }));
        round_trip(MarkdownNodeEvidenceV1::Unreadable(
            UnreadableMarkdownNodeV1 {
                relative: unix("kind/item.md"),
                metadata: PresenceV1::Missing,
            },
        ));
        round_trip(ObservedSequenceRowsV1::NotApplicable);
        round_trip(ObservedSequenceRowsV1::Observed(complete_list(vec![])));
        for value in [
            SqlCellValueV1::Null,
            SqlCellValueV1::Integer(-1),
            SqlCellValueV1::RealBits(f64::NAN.to_bits()),
            SqlCellValueV1::Text(HexBytesV1::new(b"text".to_vec())),
            SqlCellValueV1::Blob(HexBytesV1::new(vec![0, 255])),
            SqlCellValueV1::PostgresBinary(PostgresBinaryCellV1 {
                type_id: "25".into(),
                bytes: HexBytesV1::new(vec![0]),
            }),
        ] {
            round_trip(value);
        }
        for value in [
            CatalogFamilyV1::Objects,
            CatalogFamilyV1::TableDefinition,
            CatalogFamilyV1::Columns,
            CatalogFamilyV1::PrimaryKey,
            CatalogFamilyV1::UniqueKeys,
            CatalogFamilyV1::Checks,
            CatalogFamilyV1::Indexes,
            CatalogFamilyV1::ForeignObjects,
        ] {
            round_trip(value);
        }
        for value in [
            MarkdownNodeKindV1::Directory,
            MarkdownNodeKindV1::Regular(RegularMarkdownNodeV1 {
                bytes: HexBytesV1::new(Vec::new()),
            }),
            MarkdownNodeKindV1::Symlink(SymlinkMarkdownNodeV1 {
                target: unix("target"),
            }),
            MarkdownNodeKindV1::Other(OtherMarkdownNodeV1 {
                kind: OtherNodeKindV1::Unknown,
            }),
        ] {
            round_trip(value);
        }
        for value in [
            MarkdownNodeKindTagV1::Directory,
            MarkdownNodeKindTagV1::Regular,
            MarkdownNodeKindTagV1::Symlink,
            MarkdownNodeKindTagV1::Other,
        ] {
            round_trip(value);
        }
        for value in [
            OtherNodeKindV1::BlockDevice,
            OtherNodeKindV1::CharacterDevice,
            OtherNodeKindV1::Fifo,
            OtherNodeKindV1::Socket,
            OtherNodeKindV1::Unknown,
        ] {
            round_trip(value);
        }
        round_trip(FileImageV1::Absent);
        round_trip(FileImageV1::Present(PresentFileImageV1 {
            bytes: HexBytesV1::new(Vec::new()),
        }));
        round_trip(SequenceRowsV1::NotApplicable);
        round_trip(SequenceRowsV1::Rows(Vec::new()));
        for value in [SqlDialectV1::Sqlite, SqlDialectV1::Postgres] {
            round_trip(value);
        }
        for value in [HistoryKindV1::Decision, HistoryKindV1::Observation] {
            round_trip(value);
        }
        for value in [
            SqlTypeV1::Text,
            SqlTypeV1::Integer,
            SqlTypeV1::BigInt,
            SqlTypeV1::Foreign("uuid".into()),
        ] {
            round_trip(value);
        }
        let check = || HistoryCheckSchemaV1 {
            name: PresenceV1::Missing,
            catalog_expression: HexBytesV1::new(Vec::new()),
            catalog_definition: PresenceV1::Missing,
        };
        round_trip(CheckSchemaV1::HistoryKindDecisionObservation(check()));
        round_trip(CheckSchemaV1::NextValueNonnegative(
            NextValueCheckSchemaV1 {
                name: PresenceV1::Missing,
                catalog_expression: HexBytesV1::new(Vec::new()),
                catalog_definition: PresenceV1::Missing,
            },
        ));
        round_trip(CheckSchemaV1::Foreign(ForeignCheckSchemaV1 {
            name: PresenceV1::Missing,
            catalog_expression: HexBytesV1::new(Vec::new()),
            catalog_definition: PresenceV1::Missing,
        }));
        for value in [
            IndexMethodV1::Btree,
            IndexMethodV1::Gin,
            IndexMethodV1::Foreign("hash".into()),
        ] {
            round_trip(value);
        }
        for value in [
            IndexTermV1::Column("id".into()),
            IndexTermV1::DocumentJsonbPathOps(DocumentJsonbPathOpsV1 {
                catalog_expression: HexBytesV1::new(Vec::new()),
                opclass: "jsonb_path_ops".into(),
            }),
            IndexTermV1::Foreign(HexBytesV1::new(Vec::new())),
        ] {
            round_trip(value);
        }
        for value in [
            ForeignObjectKindV1::Table,
            ForeignObjectKindV1::View,
            ForeignObjectKindV1::Trigger,
            ForeignObjectKindV1::Index,
            ForeignObjectKindV1::Constraint,
            ForeignObjectKindV1::Other("sequence".into()),
        ] {
            round_trip(value);
        }
        for value in [
            CaptureRefusalCodeV1::SourceUnreachable,
            CaptureRefusalCodeV1::UnresolvedSourceCoordinate,
            CaptureRefusalCodeV1::ReadFailure,
            CaptureRefusalCodeV1::ForeignMarkdownNode,
            CaptureRefusalCodeV1::PendingBatchPresent,
            CaptureRefusalCodeV1::InvalidRelativePath,
            CaptureRefusalCodeV1::UnsupportedSchema,
            CaptureRefusalCodeV1::UnknownPhysicalObject,
            CaptureRefusalCodeV1::SqlTypeMismatch,
            CaptureRefusalCodeV1::SqlNull,
            CaptureRefusalCodeV1::DuplicateCoordinate,
            CaptureRefusalCodeV1::NonUtf8Text,
            CaptureRefusalCodeV1::UnknownHistoryKind,
            CaptureRefusalCodeV1::ForeignInstanceKind,
            CaptureRefusalCodeV1::NumericOutOfRange,
            CaptureRefusalCodeV1::HybridContradiction,
            CaptureRefusalCodeV1::NonCanonicalOrder,
            CaptureRefusalCodeV1::IncompatibleVariant,
            CaptureRefusalCodeV1::DigestMismatch,
        ] {
            round_trip(value);
        }
        for value in [
            RootCoordinateV1::Observation,
            RootCoordinateV1::Selector(SelectorRootCoordinateV1 {
                project_root: unix("repo"),
                project_file: unix("project.yaml"),
            }),
            RootCoordinateV1::EffectiveConfig,
            RootCoordinateV1::MarkdownRoot(MarkdownRootCoordinateV1 { root: unix("root") }),
            RootCoordinateV1::SqliteDatabase(SqliteDatabaseRootCoordinateV1 {
                database: unix("db"),
            }),
            RootCoordinateV1::PostgresEndpoint(PostgresEndpointRootCoordinateV1 {
                endpoint: PresenceV1::Present(postgres_value.endpoint),
                endpoint_id: PresenceV1::Present(postgres_value.endpoint_id),
            }),
            RootCoordinateV1::HybridSide(HybridSideRootCoordinateV1 {
                side: HybridSideV1::Replica,
            }),
        ] {
            round_trip(value);
        }
        for value in [
            HybridSideV1::Local,
            HybridSideV1::Replica,
            HybridSideV1::Divergences,
        ] {
            round_trip(value);
        }
        round_trip(SqlPhysicalLocatorV1::SqliteRowId(-1));
        round_trip(SqlPhysicalLocatorV1::PostgresTuple(
            PostgresTupleLocatorV1 {
                table_oid: 1,
                block: 2,
                offset: 3,
            },
        ));
        round_trip(SqlKeyPartV1::Text("id".into()));
        round_trip(SqlKeyPartV1::Integer(-1));
        for value in [
            PhysicalCoordinateV1::Root(RootCoordinateV1::Observation),
            PhysicalCoordinateV1::MarkdownPath(MarkdownPathCoordinateV1 {
                relative: unix("kind/item.md"),
            }),
            PhysicalCoordinateV1::MarkdownSpan(MarkdownSpanCoordinateV1 {
                relative: unix("kind/item.md"),
                start: 1,
                length: 2,
            }),
            PhysicalCoordinateV1::SqlTable(SqlTableCoordinateV1 {
                table: "events".into(),
            }),
            PhysicalCoordinateV1::SqlRow(SqlRowCoordinateV1 {
                table: "events".into(),
                key: vec![SqlKeyPartV1::Text("aep.entity".into())],
            }),
            PhysicalCoordinateV1::SqlColumn(SqlColumnCoordinateV1 {
                table: "events".into(),
                key: vec![SqlKeyPartV1::Integer(1)],
                column: "document".into(),
            }),
            PhysicalCoordinateV1::SqlPhysicalRow(SqlPhysicalRowCoordinateV1 {
                table: "events".into(),
                locator: SqlPhysicalLocatorV1::SqliteRowId(1),
            }),
            PhysicalCoordinateV1::SqlCatalog(SqlCatalogCoordinateV1 {
                namespace: PresenceV1::Present("main".into()),
                family: CatalogFamilyV1::Columns,
                table: PresenceV1::Present("events".into()),
                row: PresenceV1::Present(1),
            }),
        ] {
            round_trip(value);
        }

        let sqlite_raw = match empty_sql_evidence(sqlite_replica()) {
            PhaseEvidenceV1::Sqlite(value) => {
                sql_raw_from_evidence(&value, SqlDialectV1::Sqlite).expect("complete sqlite")
            }
            _ => unreachable!(),
        };
        round_trip(LegacyRawCaptureV1::Markdown(MarkdownRawV1 {
            nodes: Vec::new(),
        }));
        round_trip(LegacyRawCaptureV1::Sqlite(sqlite_raw));
        let postgres_raw = match empty_sql_evidence(postgres_replica()) {
            PhaseEvidenceV1::Postgres(value) => {
                sql_raw_from_evidence(&value, SqlDialectV1::Postgres).expect("complete postgres")
            }
            _ => unreachable!(),
        };
        round_trip(LegacyRawCaptureV1::Postgres(postgres_raw));
        let ObservationOutcomeV1::Complete(complete) = hybrid_fixture(
            HybridPolicyWordsV1 {
                authority: "local".into(),
                read: "local-first".into(),
                on_unreachable: "refuse".into(),
                on_divergence: "record".into(),
            },
            sqlite_replica(),
        )
        .observation
        else {
            unreachable!();
        };
        round_trip(complete.capture);
    }

    #[test]
    fn host_paths_preserve_units_and_apply_each_relative_grammar() {
        for valid in [
            HostPathV1::Unix(HexBytesV1::new("a\\b:c".as_bytes().to_vec())),
            HostPathV1::Unix(HexBytesV1::new("é/文.md".as_bytes().to_vec())),
            HostPathV1::Windows("dir\\😀.md".encode_utf16().collect()),
        ] {
            assert!(valid.validate_relative().is_ok(), "{valid:?}");
        }
        for invalid in [
            HostPathV1::Unix(HexBytesV1::new(Vec::new())),
            HostPathV1::Unix(HexBytesV1::new(vec![0xff])),
            HostPathV1::Unix(HexBytesV1::new(b"a\0b".to_vec())),
            HostPathV1::Unix(HexBytesV1::new(b"/root".to_vec())),
            HostPathV1::Unix(HexBytesV1::new(b"a//b".to_vec())),
            HostPathV1::Unix(HexBytesV1::new(b"a/./b".to_vec())),
            HostPathV1::Unix(HexBytesV1::new(b"a/../b".to_vec())),
            HostPathV1::Windows(vec![0xd800]),
            HostPathV1::Windows(vec![0]),
            HostPathV1::Windows(vec![0x1f]),
            HostPathV1::Windows("C:\\file".encode_utf16().collect()),
            HostPathV1::Windows("\\\\server\\share".encode_utf16().collect()),
            HostPathV1::Windows("\\?\\device".encode_utf16().collect()),
            HostPathV1::Windows("a\\.\\b".encode_utf16().collect()),
            HostPathV1::Windows("a\\..\\b".encode_utf16().collect()),
            HostPathV1::Windows("dir/file".encode_utf16().collect()),
            HostPathV1::Windows("CON.txt".encode_utf16().collect()),
            HostPathV1::Windows("Lpt9".encode_utf16().collect()),
            HostPathV1::Windows("name.".encode_utf16().collect()),
        ] {
            assert!(invalid.validate_relative().is_err(), "{invalid:?}");
        }
        let raw = serde_json::from_value::<HostPathV1>(serde_json::json!({
            "kind": "windows",
            "value": [55296]
        }))
        .expect("raw paths preserve unpaired UTF-16 units");
        assert_eq!(raw, HostPathV1::Windows(vec![0xd800]));
    }

    #[test]
    fn complete_markdown_fixture_matches_literal_transcript_and_hashes() {
        let observation = complete_markdown_fixture();
        let ObservationOutcomeV1::Complete(complete) = &observation.observation else {
            unreachable!();
        };
        let transcript = observation
            .complete_transcript(&complete.capture)
            .expect("transcript");
        let expected = decode_fixture_hex(
            "6165702e6d6967726174696f6e2e7261772d7472616e7363726970742f3100000000000000000e00000000000000116165702e7261772d636170747572652f31000000000000000770726573656e7400000000000000086d61726b646f776e0000000000000004756e69780000000000000008706c616e6e696e670000000000000004756e697800000000000000047265706f0000000000000004756e697800000000000000192e656e67696e656572696e672f70726f6a6563742e79616d6c00000000000000126d697373696e675f76315f64656661756c74000000000000000f6d697373696e675f64656661756c74000000000000002088bc6280eaaf4014465569ec070c24ce635e811325635367a1d2edb8088f334500000000000000086d61726b646f776e00000000000000080000000000000000",
        );
        assert_eq!(transcript, expected);
        assert_eq!(
            config_digest().as_wire(),
            "sha256:88bc6280eaaf4014465569ec070c24ce635e811325635367a1d2edb8088f3345"
        );
        assert_eq!(
            complete.transcript_digest.as_wire(),
            "sha256:6a0d756282c097062cf05f6bd5b22b4adecff953d4b6d3167b27485622dd8fba"
        );
        assert_eq!(
            complete.raw_snapshot_id.as_wire(),
            "sha256:c6d643e43c970d7ec01da3f4e1de38a9ebf7480c683ac907d24497f6ccb5a502"
        );
        assert!(observation.validate().is_ok());
    }

    #[test]
    fn selector_presence_and_store_field_are_independent_and_digest_bound() {
        let mut observation = complete_markdown_fixture();
        let PresenceV1::Present(selector) = &mut observation.selector else {
            unreachable!();
        };
        selector.presence = SelectorPresenceV1::Present(SelectorPresentV1 {
            selector_digest: selector_digest_v1(b"selector one"),
        });
        assert_eq!(selector.store_field, StoreFieldV1::MissingDefault);
        refresh_complete_digests(&mut observation);
        assert!(observation.validate().is_ok());

        let PresenceV1::Present(selector) = &mut observation.selector else {
            unreachable!();
        };
        selector.presence = SelectorPresenceV1::Present(SelectorPresentV1 {
            selector_digest: selector_digest_v1(b"selector two"),
        });
        assert!(observation
            .validate()
            .expect_err("selector bytes change both complete digests")
            .contains(CaptureValidationCodeV1::DigestMismatch));
    }

    #[test]
    fn complete_coordinates_config_and_capture_bytes_are_digest_bound() {
        let mut missing = complete_markdown_fixture();
        missing.source = PresenceV1::Missing;
        assert_eq!(
            missing
                .complete_transcript_digest(&LegacyRawCaptureV1::Markdown(MarkdownRawV1 {
                    nodes: Vec::new(),
                }))
                .expect_err("source is mandatory"),
            CompleteDigestErrorV1::MissingSource
        );
        assert!(missing
            .validate()
            .expect_err("complete requires coordinates")
            .contains(CaptureValidationCodeV1::MissingCoordinate));

        let mut config_changed = complete_markdown_fixture();
        config_changed.config_digest = PresenceV1::Present(config_digest_v1(b"changed"));
        assert!(config_changed
            .validate()
            .expect_err("config digest is transcript input")
            .contains(CaptureValidationCodeV1::DigestMismatch));

        let mut bytes_changed = complete_markdown_fixture();
        let ObservationOutcomeV1::Complete(complete) = &mut bytes_changed.observation else {
            unreachable!();
        };
        let LegacyRawCaptureV1::Markdown(capture) = &mut complete.capture else {
            unreachable!();
        };
        capture.nodes.push(MarkdownNodeV1 {
            relative: unix("story/item.md"),
            node: MarkdownNodeKindV1::Regular(RegularMarkdownNodeV1 {
                bytes: HexBytesV1::new(b"body".to_vec()),
            }),
        });
        assert!(bytes_changed
            .validate()
            .expect_err("supplied capture cannot differ from phases")
            .contains(CaptureValidationCodeV1::CaptureMismatch));
    }

    #[test]
    fn source_method_dialect_and_namespace_contradictions_are_named() {
        let mut observation = complete_markdown_fixture();
        let ObservationOutcomeV1::Complete(complete) = &mut observation.observation else {
            unreachable!();
        };
        complete.method = ObservationMethodV1::SqliteReadTransaction;
        assert!(observation
            .validate()
            .expect_err("method must match source")
            .contains(CaptureValidationCodeV1::IncompatibleVariant));

        let replica = sqlite_replica();
        let mut evidence = match empty_sql_evidence(replica.clone()) {
            PhaseEvidenceV1::Sqlite(value) => value,
            _ => unreachable!(),
        };
        evidence.catalog.namespace = PresenceV1::Present("other".into());
        let phase = complete_phase(
            CapturePhaseV1::SqlSnapshot,
            PhaseEvidenceV1::Sqlite(evidence),
        );
        let source = PresenceV1::Present(SourceCoordinateV1::Sqlite(SqliteSourceCoordinateV1 {
            database: match replica {
                SqlReplicaCoordinateV1::Sqlite(value) => value.database,
                _ => unreachable!(),
            },
        }));
        let mut errors = ValidationCollector::default();
        validate_phases(
            &ObservationMethodV1::SqliteReadTransaction,
            &source,
            &[phase],
            PhaseExpectation::Complete,
            &mut errors,
        );
        assert!(errors
            .finish()
            .expect_err("namespace must match source")
            .contains(CaptureValidationCodeV1::IncompatibleVariant));
    }

    #[test]
    fn roster_mutations_and_partial_complete_claims_refuse() {
        let base = complete_markdown_fixture();
        let ObservationOutcomeV1::Complete(complete) = &base.observation else {
            unreachable!();
        };
        let mut dropped = base.clone();
        let ObservationOutcomeV1::Complete(value) = &mut dropped.observation else {
            unreachable!();
        };
        value.phases.pop();
        assert!(dropped
            .validate()
            .expect_err("phase cannot be dropped")
            .contains(CaptureValidationCodeV1::PhaseRoster));

        let mut added = base.clone();
        let ObservationOutcomeV1::Complete(value) = &mut added.observation else {
            unreachable!();
        };
        value.phases.push(complete.phases[1].clone());
        assert!(added
            .validate()
            .expect_err("phase cannot be added")
            .contains(CaptureValidationCodeV1::PhaseRoster));

        let mut partial = base;
        let ObservationOutcomeV1::Complete(value) = &mut partial.observation else {
            unreachable!();
        };
        let PhaseResultV1::Complete(second) = &value.phases[1].result else {
            unreachable!();
        };
        value.phases[1].result = PhaseResultV1::Refused(RefusedPhaseResultV1 {
            evidence: second.evidence.clone(),
            evidence_digest: second.evidence_digest,
            refusals: vec![refusal()],
        });
        assert!(partial
            .validate()
            .expect_err("partial evidence cannot mint Complete")
            .contains(CaptureValidationCodeV1::PhasePrefix));
    }

    #[test]
    fn every_sqlite_catalog_subquery_and_row_family_retains_a_valid_failed_prefix() {
        for fail_at in 0..27 {
            let observation = refused_sqlite_at_step(fail_at);
            assert!(observation.validate().is_ok(), "failed read step {fail_at}");
            let encoded = serde_json::to_vec(&observation).expect("serialises retained prefix");
            assert_eq!(
                RawCaptureObservationV1::from_json(&encoded).expect("reads retained prefix"),
                observation
            );
        }

        let mut missing_placeholder = refused_sqlite_at_step(0);
        let ObservationOutcomeV1::Refused(refused) = &mut missing_placeholder.observation else {
            unreachable!();
        };
        let PhaseResultV1::Refused(phase) = &mut refused.phases[0].result else {
            unreachable!();
        };
        let PhaseEvidenceV1::Sqlite(evidence) = &mut phase.evidence else {
            unreachable!();
        };
        evidence.catalog.tables.pop();
        phase.evidence_digest = phase.evidence.evidence_digest();
        assert!(missing_placeholder
            .validate()
            .expect_err("every discovered table needs a retained placeholder")
            .contains(CaptureValidationCodeV1::UnsupportedSchema));
    }

    #[test]
    fn refused_sql_rows_retain_uncoerced_cells_and_exact_physical_locator() {
        let mut observation = refused_sqlite_at_step(23);
        let ObservationOutcomeV1::Refused(refused) = &mut observation.observation else {
            unreachable!();
        };
        let PhaseResultV1::Refused(phase) = &mut refused.phases[0].result else {
            unreachable!();
        };
        let PhaseEvidenceV1::Sqlite(evidence) = &mut phase.evidence else {
            unreachable!();
        };
        let at = PhysicalCoordinateV1::SqlPhysicalRow(SqlPhysicalRowCoordinateV1 {
            table: "instances".into(),
            locator: SqlPhysicalLocatorV1::SqliteRowId(41),
        });
        evidence.instances.terminal = EnumerationTerminalV1::Refused(EnumerationRefusalV1 {
            at: at.clone(),
            code: CaptureRefusalCodeV1::SqlNull,
        });
        evidence.rejected_rows = vec![RejectedSqlRowV1 {
            at: at.clone(),
            cells: vec![
                SqlCellImageV1 {
                    ordinal: 0,
                    column: "entity".into(),
                    value: SqlCellValueV1::Null,
                },
                SqlCellImageV1 {
                    ordinal: 1,
                    column: "id".into(),
                    value: SqlCellValueV1::Blob(HexBytesV1::new(vec![0, 255])),
                },
            ],
        }];
        phase.refusals = vec![CaptureRefusalV1 {
            at,
            code: CaptureRefusalCodeV1::SqlNull,
        }];
        phase.evidence_digest = phase.evidence.evidence_digest();
        assert!(observation.validate().is_ok());
        let encoded = serde_json::to_vec(&observation).expect("serialises exact cells");
        assert_eq!(
            RawCaptureObservationV1::from_json(&encoded).expect("reads exact cells"),
            observation
        );
    }

    #[test]
    fn postgres_sequence_failure_retains_every_earlier_family() {
        let replica = postgres_replica();
        let SqlReplicaCoordinateV1::Postgres(coordinate) = replica.clone() else {
            unreachable!();
        };
        let PhaseEvidenceV1::Postgres(mut evidence) = empty_sql_evidence(replica) else {
            unreachable!();
        };
        let at = PhysicalCoordinateV1::SqlTable(SqlTableCoordinateV1 {
            table: "provider_sequences".into(),
        });
        let ObservedSequenceRowsV1::Observed(sequences) = &mut evidence.provider_sequences else {
            unreachable!();
        };
        sequences.terminal = EnumerationTerminalV1::Refused(EnumerationRefusalV1 {
            at: at.clone(),
            code: CaptureRefusalCodeV1::ReadFailure,
        });
        let evidence = PhaseEvidenceV1::Postgres(evidence);
        let digest = evidence.evidence_digest();
        let observation = RawCaptureObservationV1 {
            format: RawCaptureFormatV1,
            source: PresenceV1::Present(SourceCoordinateV1::Postgres(PostgresSourceCoordinateV1 {
                endpoint: coordinate.endpoint,
                endpoint_id: coordinate.endpoint_id,
            })),
            selector: PresenceV1::Present(selector()),
            config_digest: PresenceV1::Present(config_digest()),
            observation: ObservationOutcomeV1::Refused(RefusedObservationV1 {
                method: PresenceV1::Present(ObservationMethodV1::PostgresRepeatableReadOnly),
                phases: vec![PhaseObservationV1 {
                    phase: CapturePhaseV1::SqlSnapshot,
                    result: PhaseResultV1::Refused(RefusedPhaseResultV1 {
                        evidence,
                        evidence_digest: digest,
                        refusals: vec![CaptureRefusalV1 {
                            at,
                            code: CaptureRefusalCodeV1::ReadFailure,
                        }],
                    }),
                }],
                preflight_refusals: Vec::new(),
            }),
        };
        assert!(observation.validate().is_ok());
    }

    #[test]
    fn foreign_catalog_facts_and_unknown_complete_schema_refuse_without_erasure() {
        let replica = sqlite_replica();
        let PhaseEvidenceV1::Sqlite(mut evidence) = empty_sql_evidence(replica.clone()) else {
            unreachable!();
        };
        let foreign = ForeignObjectV1 {
            kind: ForeignObjectKindV1::View,
            name: "unexpected_view".into(),
            owner: PresenceV1::Missing,
            catalog_definition: PresenceV1::Present(HexBytesV1::new(b"CREATE VIEW".to_vec())),
        };
        evidence.catalog.objects.items.push(CatalogObjectHeaderV1 {
            kind: foreign.kind.clone(),
            name: foreign.name.clone(),
            parent: PresenceV1::Missing,
        });
        evidence
            .catalog
            .objects
            .items
            .sort_by_key(catalog_header_order_key);
        evidence.catalog.foreign_objects.items.push(foreign.clone());
        let mut errors = ValidationCollector::default();
        validate_sql_evidence(
            &evidence,
            SqlDialectV1::Sqlite,
            &PresenceV1::Present(SourceCoordinateV1::Sqlite(SqliteSourceCoordinateV1 {
                database: match replica {
                    SqlReplicaCoordinateV1::Sqlite(value) => value.database,
                    _ => unreachable!(),
                },
            })),
            false,
            "$.observation.phases[0]",
            &mut errors,
        );
        assert!(errors
            .finish()
            .expect_err("unknown complete catalog is retained but not admitted")
            .contains(CaptureValidationCodeV1::UnsupportedSchema));
        assert_eq!(evidence.catalog.foreign_objects.items, vec![foreign]);
    }

    #[test]
    fn canonical_set_order_distinguishes_duplicates_from_disorder() {
        let node = MarkdownNodeEvidenceV1::Captured(MarkdownNodeV1 {
            relative: unix("story/item.md"),
            node: MarkdownNodeKindV1::Regular(RegularMarkdownNodeV1 {
                bytes: HexBytesV1::new(Vec::new()),
            }),
        });
        let source =
            PresenceV1::Present(SourceCoordinateV1::Markdown(MarkdownSourceCoordinateV1 {
                root: unix("planning"),
            }));
        let mut duplicate_errors = ValidationCollector::default();
        validate_markdown_evidence(
            &MarkdownEvidenceV1 {
                root: PresenceV1::Present(unix("planning")),
                nodes: complete_list(vec![node.clone(), node.clone()]),
            },
            &source,
            false,
            "$.phase",
            &mut duplicate_errors,
        );
        assert!(duplicate_errors
            .finish()
            .expect_err("duplicate coordinate")
            .contains(CaptureValidationCodeV1::DuplicateCoordinate));

        let earlier = MarkdownNodeEvidenceV1::Captured(MarkdownNodeV1 {
            relative: unix("story/a.md"),
            node: MarkdownNodeKindV1::Directory,
        });
        let mut disorder_errors = ValidationCollector::default();
        validate_markdown_evidence(
            &MarkdownEvidenceV1 {
                root: PresenceV1::Present(unix("planning")),
                nodes: complete_list(vec![node, earlier]),
            },
            &source,
            false,
            "$.phase",
            &mut disorder_errors,
        );
        assert!(disorder_errors
            .finish()
            .expect_err("caller order is preserved and refused")
            .contains(CaptureValidationCodeV1::NonCanonicalOrder));
    }

    #[test]
    fn canonical_order_uses_native_path_units_and_literal_refusal_tags() {
        let source =
            PresenceV1::Present(SourceCoordinateV1::Markdown(MarkdownSourceCoordinateV1 {
                root: unix("planning"),
            }));
        let node = |relative: &str| {
            MarkdownNodeEvidenceV1::Captured(MarkdownNodeV1 {
                relative: unix(relative),
                node: MarkdownNodeKindV1::Directory,
            })
        };
        let mut errors = ValidationCollector::default();
        validate_markdown_evidence(
            &MarkdownEvidenceV1 {
                root: PresenceV1::Present(unix("planning")),
                nodes: complete_list(vec![node("aa"), node("z")]),
            },
            &source,
            false,
            "$.phase",
            &mut errors,
        );
        assert!(errors.finish().is_ok(), "native bytes order aa before z");

        let at = PhysicalCoordinateV1::Root(RootCoordinateV1::Observation);
        let mut errors = ValidationCollector::default();
        validate_refusals(
            &[
                CaptureRefusalV1 {
                    code: CaptureRefusalCodeV1::ReadFailure,
                    at: at.clone(),
                },
                CaptureRefusalV1 {
                    code: CaptureRefusalCodeV1::SqlNull,
                    at,
                },
            ],
            "$.refusals",
            &mut errors,
        );
        assert!(
            errors.finish().is_ok(),
            "read_failure sorts before sql_null"
        );
    }

    #[test]
    fn failed_resolution_uses_only_root_coordinates_and_no_fabricated_phases() {
        let observation = RawCaptureObservationV1 {
            format: RawCaptureFormatV1,
            source: PresenceV1::Missing,
            selector: PresenceV1::Missing,
            config_digest: PresenceV1::Missing,
            observation: ObservationOutcomeV1::Refused(RefusedObservationV1 {
                method: PresenceV1::Missing,
                phases: Vec::new(),
                preflight_refusals: vec![CaptureRefusalV1 {
                    code: CaptureRefusalCodeV1::UnresolvedSourceCoordinate,
                    at: PhysicalCoordinateV1::Root(RootCoordinateV1::Observation),
                }],
            }),
        };
        assert!(observation.validate().is_ok());
        let mut json = serde_json::to_value(&observation).expect("serialises");
        json["format"] = serde_json::json!("aep.raw-capture/2");
        assert_eq!(
            RawCaptureObservationV1::from_json(
                &serde_json::to_vec(&json).expect("serialises invalid fixture")
            )
            .expect_err("wrong format is structural")
            .code,
            RawCaptureReadCodeV1::InvalidStructure
        );
    }

    #[test]
    fn absent_and_empty_divergence_images_are_distinct() {
        let policy = HybridPolicyWordsV1 {
            authority: "local".into(),
            read: "local-first".into(),
            on_unreachable: "refuse".into(),
            on_divergence: "record".into(),
        };
        let complete = hybrid_fixture(policy, sqlite_replica());
        let ObservationOutcomeV1::Complete(complete) = complete.observation else {
            unreachable!();
        };
        let mut phases = complete.phases;
        let PhaseResultV1::Complete(after) = &mut phases[4].result else {
            unreachable!();
        };
        after.evidence = PhaseEvidenceV1::Divergences(ObservedValueV1::Complete(
            FileImageV1::Present(PresentFileImageV1 {
                bytes: HexBytesV1::new(Vec::new()),
            }),
        ));
        after.evidence_digest = after.evidence.evidence_digest();
        let observation = RawCaptureObservationV1 {
            format: RawCaptureFormatV1,
            source: PresenceV1::Present(SourceCoordinateV1::Hybrid(HybridSourceCoordinateV1 {
                local_root: unix("planning"),
                replica: sqlite_replica(),
                divergence_file: unix("divergences.json"),
                policy: PresenceV1::Present(HybridPolicyWordsV1 {
                    authority: "local".into(),
                    read: "local-first".into(),
                    on_unreachable: "refuse".into(),
                    on_divergence: "record".into(),
                }),
            })),
            selector: PresenceV1::Present(selector()),
            config_digest: PresenceV1::Present(config_digest()),
            observation: ObservationOutcomeV1::Unstable(UnstableObservationV1 {
                method: ObservationMethodV1::HybridBracketedSqlSnapshot,
                phases,
                changed: vec![PhysicalCoordinateV1::Root(RootCoordinateV1::HybridSide(
                    HybridSideRootCoordinateV1 {
                        side: HybridSideV1::Divergences,
                    },
                ))],
            }),
        };
        assert!(observation.validate().is_ok());
    }

    #[test]
    fn all_forty_eight_hybrid_policies_validate_for_both_replicas() {
        let mut count = 0;
        for authority in ["local", "replica"] {
            for read in ["local-first", "replica-first", "replica-only"] {
                for on_unreachable in ["refuse", "serve-stale"] {
                    for on_divergence in ["refuse", "record"] {
                        let policy = HybridPolicyWordsV1 {
                            authority: authority.to_owned(),
                            read: read.to_owned(),
                            on_unreachable: on_unreachable.to_owned(),
                            on_divergence: on_divergence.to_owned(),
                        };
                        for replica in [sqlite_replica(), postgres_replica()] {
                            let observation = hybrid_fixture(policy.clone(), replica);
                            assert!(observation.validate().is_ok(), "{policy:?}");
                            count += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(count, 48);
    }

    #[test]
    fn missing_and_contradictory_hybrid_policy_are_distinct() {
        let policy = HybridPolicyWordsV1 {
            authority: "local".to_owned(),
            read: "local-first".to_owned(),
            on_unreachable: "refuse".to_owned(),
            on_divergence: "record".to_owned(),
        };
        let mut complete = hybrid_fixture(policy.clone(), sqlite_replica());
        let ObservationOutcomeV1::Complete(result) = &mut complete.observation else {
            unreachable!();
        };
        let LegacyRawCaptureV1::Hybrid(capture) = &mut result.capture else {
            unreachable!();
        };
        capture.policy.authority = "replica".to_owned();
        let errors = complete
            .validate()
            .expect_err("contradictory policy refuses");
        assert!(errors.contains(CaptureValidationCodeV1::CaptureMismatch));

        let replica = sqlite_replica();
        let refused = RawCaptureObservationV1 {
            format: RawCaptureFormatV1,
            source: PresenceV1::Present(SourceCoordinateV1::Hybrid(HybridSourceCoordinateV1 {
                local_root: unix("planning"),
                replica,
                divergence_file: unix("divergences.json"),
                policy: PresenceV1::Missing,
            })),
            selector: PresenceV1::Missing,
            config_digest: PresenceV1::Missing,
            observation: ObservationOutcomeV1::Refused(RefusedObservationV1 {
                method: PresenceV1::Present(ObservationMethodV1::HybridBracketedSqlSnapshot),
                phases: vec![
                    CapturePhaseV1::LocalBefore,
                    CapturePhaseV1::DivergencesBefore,
                    CapturePhaseV1::ReplicaSnapshot,
                    CapturePhaseV1::LocalAfter,
                    CapturePhaseV1::DivergencesAfter,
                ]
                .into_iter()
                .map(|phase| PhaseObservationV1 {
                    phase,
                    result: PhaseResultV1::NotAttempted,
                })
                .collect(),
                preflight_refusals: vec![refusal()],
            }),
        };
        assert!(refused.validate().is_ok());
    }

    #[test]
    fn hybrid_policy_and_successful_prefix_survive_failure_at_every_phase() {
        let policy = HybridPolicyWordsV1 {
            authority: "replica".to_owned(),
            read: "replica-only".to_owned(),
            on_unreachable: "serve-stale".to_owned(),
            on_divergence: "record".to_owned(),
        };
        let complete = hybrid_fixture(policy, sqlite_replica());
        let PresenceV1::Present(source) = complete.source.clone() else {
            unreachable!();
        };
        let ObservationOutcomeV1::Complete(result) = complete.observation else {
            unreachable!();
        };
        for failed_at in 0..result.phases.len() {
            let phases = result
                .phases
                .iter()
                .enumerate()
                .map(|(index, phase)| {
                    if index < failed_at {
                        phase.clone()
                    } else if index == failed_at {
                        let PhaseResultV1::Complete(complete) = &phase.result else {
                            unreachable!();
                        };
                        refused_phase(phase.phase.clone(), complete.evidence.clone())
                    } else {
                        PhaseObservationV1 {
                            phase: phase.phase.clone(),
                            result: PhaseResultV1::NotAttempted,
                        }
                    }
                })
                .collect();
            let refused = RawCaptureObservationV1 {
                format: RawCaptureFormatV1,
                source: PresenceV1::Present(source.clone()),
                selector: PresenceV1::Present(selector()),
                config_digest: PresenceV1::Present(config_digest()),
                observation: ObservationOutcomeV1::Refused(RefusedObservationV1 {
                    method: PresenceV1::Present(ObservationMethodV1::HybridBracketedSqlSnapshot),
                    phases,
                    preflight_refusals: Vec::new(),
                }),
            };
            assert!(refused.validate().is_ok(), "failed phase {failed_at}");
        }
    }

    #[test]
    fn hybrid_failure_after_sql_retains_exact_snapshot_rows() {
        let policy = HybridPolicyWordsV1 {
            authority: "replica".to_owned(),
            read: "replica-first".to_owned(),
            on_unreachable: "serve-stale".to_owned(),
            on_divergence: "record".to_owned(),
        };
        let row = InstanceRowV1 {
            entity: "aep.entity".into(),
            id: "story:item".into(),
            revision: 7,
            document: HexBytesV1::new(b"{\"exact\":1}".to_vec()),
        };
        let mut complete = hybrid_fixture(policy, sqlite_replica());
        let ObservationOutcomeV1::Complete(result) = &mut complete.observation else {
            unreachable!();
        };
        let PhaseResultV1::Complete(sql_phase) = &mut result.phases[2].result else {
            unreachable!();
        };
        let PhaseEvidenceV1::Sqlite(sql_evidence) = &mut sql_phase.evidence else {
            unreachable!();
        };
        sql_evidence.instances.items.push(row.clone());
        sql_phase.evidence_digest = sql_phase.evidence.evidence_digest();
        let LegacyRawCaptureV1::Hybrid(capture) = &mut result.capture else {
            unreachable!();
        };
        capture.replica.instances.push(row.clone());
        refresh_complete_digests(&mut complete);
        assert!(complete.validate().is_ok());

        let ObservationOutcomeV1::Complete(result) = complete.observation else {
            unreachable!();
        };
        let PhaseResultV1::Complete(failed_evidence) = &result.phases[3].result else {
            unreachable!();
        };
        let mut phases = result.phases[..3].to_vec();
        phases.push(refused_phase(
            CapturePhaseV1::LocalAfter,
            failed_evidence.evidence.clone(),
        ));
        phases.push(PhaseObservationV1 {
            phase: CapturePhaseV1::DivergencesAfter,
            result: PhaseResultV1::NotAttempted,
        });
        let refused = RawCaptureObservationV1 {
            observation: ObservationOutcomeV1::Refused(RefusedObservationV1 {
                method: PresenceV1::Present(ObservationMethodV1::HybridBracketedSqlSnapshot),
                phases,
                preflight_refusals: Vec::new(),
            }),
            ..complete
        };
        assert!(refused.validate().is_ok());
        let ObservationOutcomeV1::Refused(result) = &refused.observation else {
            unreachable!();
        };
        let PhaseResultV1::Complete(sql_phase) = &result.phases[2].result else {
            unreachable!();
        };
        let PhaseEvidenceV1::Sqlite(sql_evidence) = &sql_phase.evidence else {
            unreachable!();
        };
        assert_eq!(sql_evidence.instances.items, vec![row]);
    }

    #[test]
    fn unstable_changed_set_is_recomputed_from_raw_bytes() {
        let mut observation = complete_markdown_fixture();
        let changed_node = MarkdownNodeV1 {
            relative: unix("story/item.md"),
            node: MarkdownNodeKindV1::Regular(RegularMarkdownNodeV1 {
                bytes: HexBytesV1::new(b"body".to_vec()),
            }),
        };
        let ObservationOutcomeV1::Complete(complete) = observation.observation else {
            unreachable!();
        };
        let mut phases = complete.phases;
        let PhaseResultV1::Complete(second) = &mut phases[1].result else {
            unreachable!();
        };
        let PhaseEvidenceV1::Markdown(second_evidence) = &mut second.evidence else {
            unreachable!();
        };
        second_evidence.nodes.items = vec![MarkdownNodeEvidenceV1::Captured(changed_node.clone())];
        second.evidence_digest = second.evidence.evidence_digest();
        observation.observation = ObservationOutcomeV1::Unstable(UnstableObservationV1 {
            method: ObservationMethodV1::MarkdownDoubleScan,
            phases,
            changed: vec![PhysicalCoordinateV1::MarkdownPath(
                MarkdownPathCoordinateV1 {
                    relative: changed_node.relative,
                },
            )],
        });
        assert!(observation.validate().is_ok());
        let ObservationOutcomeV1::Unstable(unstable) = &mut observation.observation else {
            unreachable!();
        };
        unstable.changed.clear();
        assert!(observation
            .validate()
            .expect_err("caller cannot choose changed set")
            .contains(CaptureValidationCodeV1::UnstableDifference));
    }

    #[test]
    fn second_scan_failure_retains_the_complete_first_scan() {
        let first = complete_phase(CapturePhaseV1::MarkdownFirst, empty_markdown("planning"));
        let partial = PhaseEvidenceV1::Markdown(MarkdownEvidenceV1 {
            root: PresenceV1::Present(unix("planning")),
            nodes: ObservedListV1 {
                items: vec![MarkdownNodeEvidenceV1::Unreadable(
                    UnreadableMarkdownNodeV1 {
                        relative: unix("story/item.md"),
                        metadata: PresenceV1::Missing,
                    },
                )],
                terminal: EnumerationTerminalV1::Refused(EnumerationRefusalV1 {
                    at: PhysicalCoordinateV1::MarkdownPath(MarkdownPathCoordinateV1 {
                        relative: unix("story/item.md"),
                    }),
                    code: CaptureRefusalCodeV1::ReadFailure,
                }),
            },
        });
        let mut second = refused_phase(CapturePhaseV1::MarkdownSecond, partial);
        let PhaseResultV1::Refused(result) = &mut second.result else {
            unreachable!();
        };
        result.refusals = vec![CaptureRefusalV1 {
            at: PhysicalCoordinateV1::MarkdownPath(MarkdownPathCoordinateV1 {
                relative: unix("story/item.md"),
            }),
            code: CaptureRefusalCodeV1::ReadFailure,
        }];
        let observation = RawCaptureObservationV1 {
            format: RawCaptureFormatV1,
            source: PresenceV1::Present(SourceCoordinateV1::Markdown(MarkdownSourceCoordinateV1 {
                root: unix("planning"),
            })),
            selector: PresenceV1::Present(selector()),
            config_digest: PresenceV1::Present(config_digest()),
            observation: ObservationOutcomeV1::Refused(RefusedObservationV1 {
                method: PresenceV1::Present(ObservationMethodV1::MarkdownDoubleScan),
                phases: vec![first.clone(), second],
                preflight_refusals: Vec::new(),
            }),
        };
        assert!(observation.validate().is_ok());
        let ObservationOutcomeV1::Refused(refused) = &observation.observation else {
            unreachable!();
        };
        assert_eq!(refused.phases[0], first);
    }

    fn decode_fixture_hex(value: &str) -> Vec<u8> {
        assert_eq!(value.len() % 2, 0);
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let digit = |byte| match byte {
                    b'0'..=b'9' => byte - b'0',
                    b'a'..=b'f' => byte - b'a' + 10,
                    _ => panic!("fixture contains non-hexadecimal byte"),
                };
                (digit(pair[0]) << 4) | digit(pair[1])
            })
            .collect()
    }
}
