//! Physical read-only acquisition for legacy planning stores.

#![allow(missing_docs)]

use std::fs;
use std::path::Path;
use std::str::FromStr as _;

#[allow(clippy::wildcard_imports)]
// This adapter implements the complete closed migration vocabulary.
use aep_contract::migration::*;

#[derive(Debug, thiserror::Error)]
pub enum AcquisitionError {
    #[error("physical source read failed at {path}")]
    Io { path: String },
    #[error("constructed raw capture violated its closed contract")]
    InvalidCapture,
    #[error("SQLite acquisition failed: {0}")]
    Sqlite(String),
    #[error("PostgreSQL acquisition failed: {0}")]
    Postgres(String),
}

pub struct SelectorBinding {
    pub project_root: HostPathV1,
    pub project_file: HostPathV1,
    pub selector_digest: DigestV1,
    pub store_field: StoreFieldV1,
    pub config_digest: DigestV1,
}

/// Reads a Markdown tree twice without following symbolic links and retains every node byte.
pub fn capture_markdown(
    root: &Path,
    source_root: HostPathV1,
    selector: SelectorBinding,
) -> Result<RawCaptureObservationV1, AcquisitionError> {
    let config_digest = selector.config_digest;
    let first = scan_markdown(root)?;
    let second = scan_markdown(root)?;
    let first_phase = markdown_phase(CapturePhaseV1::MarkdownFirst, &source_root, &first);
    let second_phase = markdown_phase(CapturePhaseV1::MarkdownSecond, &source_root, &second);
    let source = PresenceV1::Present(SourceCoordinateV1::Markdown(MarkdownSourceCoordinateV1 {
        root: source_root.clone(),
    }));
    let selector = PresenceV1::Present(SelectorCoordinateV1 {
        project_root: selector.project_root,
        project_file: selector.project_file,
        presence: SelectorPresenceV1::Present(SelectorPresentV1 {
            selector_digest: selector.selector_digest,
        }),
        store_field: selector.store_field,
    });
    if first != second {
        let observation = RawCaptureObservationV1 {
            format: RawCaptureFormatV1,
            source,
            selector,
            config_digest: PresenceV1::Present(config_digest),
            observation: ObservationOutcomeV1::Unstable(UnstableObservationV1 {
                method: ObservationMethodV1::MarkdownDoubleScan,
                phases: vec![first_phase, second_phase],
                changed: vec![PhysicalCoordinateV1::Root(RootCoordinateV1::MarkdownRoot(
                    MarkdownRootCoordinateV1 { root: source_root },
                ))],
            }),
        };
        return Ok(observation);
    }
    let capture = LegacyRawCaptureV1::Markdown(first);
    let mut observation = RawCaptureObservationV1 {
        format: RawCaptureFormatV1,
        source,
        selector,
        config_digest: PresenceV1::Present(config_digest),
        observation: ObservationOutcomeV1::Complete(CompleteObservationV1 {
            method: ObservationMethodV1::MarkdownDoubleScan,
            phases: vec![first_phase, second_phase],
            capture: capture.clone(),
            transcript_digest: DigestV1::from_bytes([0; 32]),
            raw_snapshot_id: DigestV1::from_bytes([0; 32]),
        }),
    };
    let transcript = observation
        .complete_transcript_digest(&capture)
        .map_err(|_| AcquisitionError::InvalidCapture)?;
    let snapshot = observation
        .complete_snapshot_id(&capture)
        .map_err(|_| AcquisitionError::InvalidCapture)?;
    if let ObservationOutcomeV1::Complete(complete) = &mut observation.observation {
        complete.transcript_digest = transcript;
        complete.raw_snapshot_id = snapshot;
    }
    observation
        .validate()
        .map_err(|_| AcquisitionError::InvalidCapture)?;
    Ok(observation)
}

/// Captures all legacy SQLite rows inside one read-only transaction.
pub fn capture_sqlite(
    database: &Path,
    selector: SelectorBinding,
) -> Result<RawCaptureObservationV1, AcquisitionError> {
    let coordinate = SqliteSourceCoordinateV1 {
        database: host_path(database),
    };
    let raw = sqlite_raw(database)?;
    complete_sql_observation(
        SourceCoordinateV1::Sqlite(coordinate.clone()),
        SqlReplicaCoordinateV1::Sqlite(SqliteReplicaCoordinateV1 {
            database: coordinate.database,
        }),
        selector,
        ObservationMethodV1::SqliteReadTransaction,
        CapturePhaseV1::SqlSnapshot,
        raw,
    )
}

/// Captures all legacy PostgreSQL rows in one repeatable-read read-only transaction. The returned
/// endpoint contains no user, password, or options.
pub fn capture_postgres(
    url: &str,
    selector: SelectorBinding,
) -> Result<RawCaptureObservationV1, AcquisitionError> {
    let (raw, endpoint) = postgres_raw(url)?;
    let endpoint_id = endpoint.endpoint_id();
    complete_sql_observation(
        SourceCoordinateV1::Postgres(PostgresSourceCoordinateV1 {
            endpoint: endpoint.clone(),
            endpoint_id,
        }),
        SqlReplicaCoordinateV1::Postgres(PostgresReplicaCoordinateV1 {
            endpoint,
            endpoint_id,
        }),
        selector,
        ObservationMethodV1::PostgresRepeatableReadOnly,
        CapturePhaseV1::SqlSnapshot,
        raw,
    )
}

/// Resolves only the credential-free endpoint identity used in public results.
pub fn postgres_source_coordinate(
    url: &str,
) -> Result<PostgresSourceCoordinateV1, AcquisitionError> {
    use postgres::NoTls;
    let config = postgres::Config::from_str(url)
        .map_err(|error| AcquisitionError::Postgres(error.to_string()))?;
    let mut client = config
        .connect(NoTls)
        .map_err(|error| AcquisitionError::Postgres(error.to_string()))?;
    let schema: String = client
        .query_one("SELECT current_schema()", &[])
        .map_err(|error| AcquisitionError::Postgres(error.to_string()))?
        .get(0);
    let endpoint = postgres_endpoint(&config, schema)?;
    Ok(PostgresSourceCoordinateV1 {
        endpoint_id: endpoint.endpoint_id(),
        endpoint,
    })
}

/// Captures the local tree and divergence file both before and after one SQLite snapshot.
pub fn capture_hybrid_sqlite(
    local: &Path,
    database: &Path,
    divergence_file: &Path,
    policy: HybridPolicyWordsV1,
    selector: SelectorBinding,
) -> Result<RawCaptureObservationV1, AcquisitionError> {
    capture_hybrid(local, divergence_file, policy, selector, || {
        Ok((
            sqlite_raw(database)?,
            SqlReplicaCoordinateV1::Sqlite(SqliteReplicaCoordinateV1 {
                database: host_path(database),
            }),
        ))
    })
}

/// Captures the local tree and divergence file both before and after one PostgreSQL snapshot.
pub fn capture_hybrid_postgres(
    local: &Path,
    url: &str,
    divergence_file: &Path,
    policy: HybridPolicyWordsV1,
    selector: SelectorBinding,
) -> Result<RawCaptureObservationV1, AcquisitionError> {
    capture_hybrid(local, divergence_file, policy, selector, || {
        let (raw, endpoint) = postgres_raw(url)?;
        let endpoint_id = endpoint.endpoint_id();
        Ok((
            raw,
            SqlReplicaCoordinateV1::Postgres(PostgresReplicaCoordinateV1 {
                endpoint,
                endpoint_id,
            }),
        ))
    })
}

fn capture_hybrid<F>(
    local: &Path,
    divergence_file: &Path,
    policy: HybridPolicyWordsV1,
    selector: SelectorBinding,
    acquire_replica: F,
) -> Result<RawCaptureObservationV1, AcquisitionError>
where
    F: FnOnce() -> Result<(SqlRawV1, SqlReplicaCoordinateV1), AcquisitionError>,
{
    let local_before = scan_markdown(local)?;
    let divergences_before = file_image(divergence_file)?;
    // Endpoint discovery belongs inside this bracket too. No PostgreSQL connection or SQL read is
    // allowed to precede the first local observation.
    let (raw, replica) = acquire_replica()?;
    let local_after = scan_markdown(local)?;
    let divergences_after = file_image(divergence_file)?;
    let source_coordinate = SourceCoordinateV1::Hybrid(HybridSourceCoordinateV1 {
        local_root: host_path(local),
        replica: replica.clone(),
        divergence_file: host_path(divergence_file),
        policy: PresenceV1::Present(policy.clone()),
    });
    let phases = vec![
        complete_phase(
            CapturePhaseV1::LocalBefore,
            markdown_evidence(&host_path(local), &local_before),
        ),
        complete_phase(
            CapturePhaseV1::DivergencesBefore,
            PhaseEvidenceV1::Divergences(ObservedValueV1::Complete(divergences_before.clone())),
        ),
        complete_phase(
            CapturePhaseV1::ReplicaSnapshot,
            sql_phase_evidence(&replica, &raw),
        ),
        complete_phase(
            CapturePhaseV1::LocalAfter,
            markdown_evidence(&host_path(local), &local_after),
        ),
        complete_phase(
            CapturePhaseV1::DivergencesAfter,
            PhaseEvidenceV1::Divergences(ObservedValueV1::Complete(divergences_after.clone())),
        ),
    ];
    if local_before != local_after || divergences_before != divergences_after {
        let mut changed = markdown_changed_coordinates(&local_before, &local_after);
        if divergences_before != divergences_after {
            changed.push(PhysicalCoordinateV1::Root(RootCoordinateV1::HybridSide(
                HybridSideRootCoordinateV1 {
                    side: HybridSideV1::Divergences,
                },
            )));
        }
        return unstable_observation(source_coordinate, selector, phases, changed);
    }
    finish_complete(
        source_coordinate,
        selector,
        ObservationMethodV1::HybridBracketedSqlSnapshot,
        phases,
        LegacyRawCaptureV1::Hybrid(HybridRawV1 {
            policy,
            local: local_before,
            replica: raw,
            divergences: divergences_before,
        }),
    )
}

fn markdown_changed_coordinates(
    before: &MarkdownRawV1,
    after: &MarkdownRawV1,
) -> Vec<PhysicalCoordinateV1> {
    use std::collections::{BTreeMap, BTreeSet};

    let before = before
        .nodes
        .iter()
        .map(|node| (node.relative.clone(), &node.node))
        .collect::<BTreeMap<_, _>>();
    let after = after
        .nodes
        .iter()
        .map(|node| (node.relative.clone(), &node.node))
        .collect::<BTreeMap<_, _>>();
    before
        .keys()
        .chain(after.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|relative| before.get(*relative) != after.get(*relative))
        .cloned()
        .map(|relative| PhysicalCoordinateV1::MarkdownPath(MarkdownPathCoordinateV1 { relative }))
        .collect()
}

#[allow(clippy::needless_pass_by_value)] // Owned capture parts are assembled into one observation.
fn complete_sql_observation(
    source: SourceCoordinateV1,
    replica: SqlReplicaCoordinateV1,
    selector: SelectorBinding,
    method: ObservationMethodV1,
    phase: CapturePhaseV1,
    raw: SqlRawV1,
) -> Result<RawCaptureObservationV1, AcquisitionError> {
    let capture = match raw.dialect {
        SqlDialectV1::Sqlite => LegacyRawCaptureV1::Sqlite(raw.clone()),
        SqlDialectV1::Postgres => LegacyRawCaptureV1::Postgres(raw.clone()),
    };
    finish_complete(
        source,
        selector,
        method,
        vec![complete_phase(phase, sql_phase_evidence(&replica, &raw))],
        capture,
    )
}

#[allow(clippy::needless_pass_by_value)] // The capture is retained and hashed in the same envelope.
fn finish_complete(
    source: SourceCoordinateV1,
    selector: SelectorBinding,
    method: ObservationMethodV1,
    phases: Vec<PhaseObservationV1>,
    capture: LegacyRawCaptureV1,
) -> Result<RawCaptureObservationV1, AcquisitionError> {
    let config_digest = selector.config_digest;
    let mut observation = RawCaptureObservationV1 {
        format: RawCaptureFormatV1,
        source: PresenceV1::Present(source),
        selector: PresenceV1::Present(selector_coordinate(selector)),
        config_digest: PresenceV1::Present(config_digest),
        observation: ObservationOutcomeV1::Complete(CompleteObservationV1 {
            method,
            phases,
            capture: capture.clone(),
            transcript_digest: DigestV1::from_bytes([0; 32]),
            raw_snapshot_id: DigestV1::from_bytes([0; 32]),
        }),
    };
    let transcript = observation
        .complete_transcript_digest(&capture)
        .map_err(|_| AcquisitionError::InvalidCapture)?;
    let snapshot = observation
        .complete_snapshot_id(&capture)
        .map_err(|_| AcquisitionError::InvalidCapture)?;
    let ObservationOutcomeV1::Complete(complete) = &mut observation.observation else {
        unreachable!()
    };
    complete.transcript_digest = transcript;
    complete.raw_snapshot_id = snapshot;
    observation
        .validate()
        .map_err(|_| AcquisitionError::InvalidCapture)?;
    Ok(observation)
}

fn unstable_observation(
    source: SourceCoordinateV1,
    selector: SelectorBinding,
    phases: Vec<PhaseObservationV1>,
    changed: Vec<PhysicalCoordinateV1>,
) -> Result<RawCaptureObservationV1, AcquisitionError> {
    let config_digest = selector.config_digest;
    let observation = RawCaptureObservationV1 {
        format: RawCaptureFormatV1,
        source: PresenceV1::Present(source),
        selector: PresenceV1::Present(selector_coordinate(selector)),
        config_digest: PresenceV1::Present(config_digest),
        observation: ObservationOutcomeV1::Unstable(UnstableObservationV1 {
            method: ObservationMethodV1::HybridBracketedSqlSnapshot,
            phases,
            changed,
        }),
    };
    observation
        .validate()
        .map_err(|_| AcquisitionError::InvalidCapture)?;
    Ok(observation)
}

fn selector_coordinate(selector: SelectorBinding) -> SelectorCoordinateV1 {
    SelectorCoordinateV1 {
        project_root: selector.project_root,
        project_file: selector.project_file,
        presence: SelectorPresenceV1::Present(SelectorPresentV1 {
            selector_digest: selector.selector_digest,
        }),
        store_field: selector.store_field,
    }
}

fn file_image(path: &Path) -> Result<FileImageV1, AcquisitionError> {
    match fs::read(path) {
        Ok(bytes) => Ok(FileImageV1::Present(PresentFileImageV1 {
            bytes: HexBytesV1::new(bytes),
        })),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(FileImageV1::Absent),
        Err(_) => Err(AcquisitionError::Io {
            path: path.display().to_string(),
        }),
    }
}

fn markdown_evidence(root: &HostPathV1, capture: &MarkdownRawV1) -> PhaseEvidenceV1 {
    PhaseEvidenceV1::Markdown(MarkdownEvidenceV1 {
        root: PresenceV1::Present(root.clone()),
        nodes: complete_list(
            capture
                .nodes
                .iter()
                .cloned()
                .map(MarkdownNodeEvidenceV1::Captured)
                .collect(),
        ),
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

fn complete_list<T>(items: Vec<T>) -> ObservedListV1<T> {
    ObservedListV1 {
        items,
        terminal: EnumerationTerminalV1::Complete,
    }
}

fn sql_phase_evidence(replica: &SqlReplicaCoordinateV1, raw: &SqlRawV1) -> PhaseEvidenceV1 {
    let mut objects = raw
        .schema
        .tables
        .iter()
        .map(|table| CatalogObjectHeaderV1 {
            kind: ForeignObjectKindV1::Table,
            name: table.name.clone(),
            parent: PresenceV1::Missing,
        })
        .chain(
            raw.schema
                .indexes
                .iter()
                .map(|index| CatalogObjectHeaderV1 {
                    kind: ForeignObjectKindV1::Index,
                    name: index.name.clone(),
                    parent: PresenceV1::Present(index.table.clone()),
                }),
        )
        .collect::<Vec<_>>();
    objects.sort_by(|left, right| {
        catalog_kind(&left.kind)
            .cmp(catalog_kind(&right.kind))
            .then_with(|| left.name.cmp(&right.name))
    });
    let catalog = SqlCatalogEvidenceV1 {
        namespace: PresenceV1::Present(raw.schema.namespace.clone()),
        objects: complete_list(objects),
        tables: raw
            .schema
            .tables
            .iter()
            .map(|table| TableCatalogEvidenceV1 {
                name: table.name.clone(),
                catalog_definition: ObservedValueV1::Complete(table.catalog_definition.clone()),
                columns: complete_list(table.columns.clone()),
                primary_key: ObservedValueV1::Complete(table.primary_key.clone()),
                unique_keys: complete_list(table.unique_keys.clone()),
                checks: complete_list(table.checks.clone()),
            })
            .collect(),
        indexes: complete_list(raw.schema.indexes.clone()),
        foreign_objects: complete_list(raw.schema.foreign_objects.clone()),
        rejected_rows: Vec::new(),
    };
    let value = SqlEvidenceV1 {
        source: PresenceV1::Present(replica.clone()),
        catalog,
        instances: complete_list(raw.instances.clone()),
        events: complete_list(raw.events.clone()),
        history: complete_list(raw.history.clone()),
        legacy_origins: complete_list(raw.legacy_origins.clone()),
        provider_sequences: match &raw.provider_sequences {
            SequenceRowsV1::NotApplicable => ObservedSequenceRowsV1::NotApplicable,
            SequenceRowsV1::Rows(rows) => {
                ObservedSequenceRowsV1::Observed(complete_list(rows.clone()))
            }
        },
        rejected_rows: Vec::new(),
    };
    match raw.dialect {
        SqlDialectV1::Sqlite => PhaseEvidenceV1::Sqlite(value),
        SqlDialectV1::Postgres => PhaseEvidenceV1::Postgres(value),
    }
}

fn catalog_kind(kind: &ForeignObjectKindV1) -> &str {
    match kind {
        ForeignObjectKindV1::Table => "table",
        ForeignObjectKindV1::View => "view",
        ForeignObjectKindV1::Trigger => "trigger",
        ForeignObjectKindV1::Index => "index",
        ForeignObjectKindV1::Constraint => "constraint",
        ForeignObjectKindV1::Other(value) => value,
    }
}

fn sqlite_raw(path: &Path) -> Result<SqlRawV1, AcquisitionError> {
    use rusqlite::{Connection, OpenFlags};
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| AcquisitionError::Sqlite(error.to_string()))?;
    connection
        .execute_batch("BEGIN DEFERRED TRANSACTION")
        .map_err(|error| AcquisitionError::Sqlite(error.to_string()))?;
    let instances = sqlite_rows(
        &connection,
        "SELECT entity,id,revision,document FROM instances ORDER BY entity,id",
        |row| {
            Ok(InstanceRowV1 {
                entity: row.get(0)?,
                id: row.get(1)?,
                revision: row.get(2)?,
                document: HexBytesV1::new(row.get::<_, String>(3)?.into_bytes()),
            })
        },
    )?;
    let events = sqlite_rows(&connection, "SELECT entity,id,revision,position,document FROM events ORDER BY entity,id,revision,position", |row| Ok(EventRowV1 {
        entity: row.get(0)?, id: row.get(1)?, revision: row.get(2)?, position: row.get(3)?, document: HexBytesV1::new(row.get::<_, String>(4)?.into_bytes()),
    }))?;
    let history = sqlite_rows(&connection, "SELECT entity,id,position,kind,record_id,document FROM history ORDER BY entity,id,position", |row| {
        let kind: String = row.get(3)?;
        let kind = match kind.as_str() {
            "decision" => HistoryKindV1::Decision,
            "observation" => HistoryKindV1::Observation,
            _ => return Err(rusqlite::Error::InvalidQuery),
        };
        Ok(HistoryRowV1 { entity: row.get(0)?, id: row.get(1)?, position: row.get(2)?, kind, record_id: row.get(4)?, document: HexBytesV1::new(row.get::<_, String>(5)?.into_bytes()) })
    })?;
    let legacy_origins = sqlite_rows(
        &connection,
        "SELECT entity,id,revision FROM legacy_origins ORDER BY entity,id",
        |row| {
            Ok(LegacyOriginRowV1 {
                entity: row.get(0)?,
                id: row.get(1)?,
                revision: row.get(2)?,
            })
        },
    )?;
    let schema = fixed_schema(SqlDialectV1::Sqlite, "main");
    verify_sqlite_catalog(&connection, &schema)?;
    connection
        .execute_batch("COMMIT")
        .map_err(|error| AcquisitionError::Sqlite(error.to_string()))?;
    Ok(SqlRawV1 {
        dialect: SqlDialectV1::Sqlite,
        schema,
        instances,
        events,
        history,
        legacy_origins,
        provider_sequences: SequenceRowsV1::NotApplicable,
    })
}

fn sqlite_rows<T, F>(
    connection: &rusqlite::Connection,
    sql: &str,
    mut map: F,
) -> Result<Vec<T>, AcquisitionError>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| AcquisitionError::Sqlite(error.to_string()))?;
    let rows = statement
        .query_map([], |row| map(row))
        .map_err(|error| AcquisitionError::Sqlite(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AcquisitionError::Sqlite(error.to_string()))
}

fn verify_sqlite_catalog(
    connection: &rusqlite::Connection,
    schema: &SqlSchemaV1,
) -> Result<(), AcquisitionError> {
    let names = sqlite_rows(connection,
        "SELECT name FROM sqlite_master WHERE type IN ('table','view','trigger') AND name NOT LIKE 'sqlite_%' ORDER BY name",
        |row| row.get::<_, String>(0),
    )?;
    let expected = schema
        .tables
        .iter()
        .map(|table| table.name.clone())
        .collect::<Vec<_>>();
    if names != expected {
        return Err(AcquisitionError::InvalidCapture);
    }
    for table in &schema.tables {
        let quoted = table.name.replace('"', "\"\"");
        let actual = sqlite_rows(
            connection,
            &format!("PRAGMA table_info(\"{quoted}\")"),
            |row| row.get::<_, String>(1),
        )?;
        let expected = table
            .columns
            .iter()
            .map(|column| column.name.clone())
            .collect::<Vec<_>>();
        if actual != expected {
            return Err(AcquisitionError::InvalidCapture);
        }
    }
    Ok(())
}

fn postgres_raw(url: &str) -> Result<(SqlRawV1, PostgresEndpointV1), AcquisitionError> {
    use postgres::{IsolationLevel, NoTls};
    let config = postgres::Config::from_str(url)
        .map_err(|error| AcquisitionError::Postgres(error.to_string()))?;
    let mut client = config
        .connect(NoTls)
        .map_err(|error| AcquisitionError::Postgres(error.to_string()))?;
    let mut transaction = client
        .build_transaction()
        .isolation_level(IsolationLevel::RepeatableRead)
        .read_only(true)
        .start()
        .map_err(|error| AcquisitionError::Postgres(error.to_string()))?;
    let schema_name: String = transaction
        .query_one("SELECT current_schema()", &[])
        .map_err(|error| AcquisitionError::Postgres(error.to_string()))?
        .get(0);
    let endpoint = postgres_endpoint(&config, schema_name.clone())?;
    let instances = transaction
        .query(
            "SELECT entity,id,revision,document FROM instances ORDER BY entity,id",
            &[],
        )
        .map_err(|error| AcquisitionError::Postgres(error.to_string()))?
        .into_iter()
        .map(|row| InstanceRowV1 {
            entity: row.get(0),
            id: row.get(1),
            revision: row.get(2),
            document: HexBytesV1::new(row.get::<_, String>(3).into_bytes()),
        })
        .collect();
    let events = transaction.query("SELECT entity,id,revision,position,document FROM events ORDER BY entity,id,revision,position", &[])
        .map_err(|error| AcquisitionError::Postgres(error.to_string()))?.into_iter().map(|row| EventRowV1 {
            entity: row.get(0), id: row.get(1), revision: row.get(2), position: row.get(3), document: HexBytesV1::new(row.get::<_, String>(4).into_bytes()),
        }).collect();
    let mut history = Vec::new();
    for row in transaction.query("SELECT entity,id,position,kind,record_id,document FROM history ORDER BY entity,id,position", &[])
        .map_err(|error| AcquisitionError::Postgres(error.to_string()))? {
        let kind: String = row.get(3);
        let kind = match kind.as_str() { "decision" => HistoryKindV1::Decision, "observation" => HistoryKindV1::Observation, _ => return Err(AcquisitionError::InvalidCapture) };
        history.push(HistoryRowV1 { entity: row.get(0), id: row.get(1), position: row.get(2), kind, record_id: row.get(4), document: HexBytesV1::new(row.get::<_, String>(5).into_bytes()) });
    }
    let legacy_origins = transaction
        .query(
            "SELECT entity,id,revision FROM legacy_origins ORDER BY entity,id",
            &[],
        )
        .map_err(|error| AcquisitionError::Postgres(error.to_string()))?
        .into_iter()
        .map(|row| LegacyOriginRowV1 {
            entity: row.get(0),
            id: row.get(1),
            revision: row.get(2),
        })
        .collect();
    let provider_sequences = transaction
        .query(
            "SELECT namespace,next_value FROM provider_sequences ORDER BY namespace",
            &[],
        )
        .map_err(|error| AcquisitionError::Postgres(error.to_string()))?
        .into_iter()
        .map(|row| ProviderSequenceRowV1 {
            namespace: row.get(0),
            next_value: row.get(1),
        })
        .collect();
    let schema = fixed_schema(SqlDialectV1::Postgres, &schema_name);
    verify_postgres_catalog(&mut transaction, &schema)?;
    transaction
        .commit()
        .map_err(|error| AcquisitionError::Postgres(error.to_string()))?;
    Ok((
        SqlRawV1 {
            dialect: SqlDialectV1::Postgres,
            schema,
            instances,
            events,
            history,
            legacy_origins,
            provider_sequences: SequenceRowsV1::Rows(provider_sequences),
        },
        endpoint,
    ))
}

fn postgres_endpoint(
    config: &postgres::Config,
    schema: String,
) -> Result<PostgresEndpointV1, AcquisitionError> {
    let ports = config.get_ports();
    let hosts = config
        .get_hosts()
        .iter()
        .enumerate()
        .map(|(index, host)| {
            let port = ports
                .get(index)
                .copied()
                .or_else(|| ports.first().copied())
                .unwrap_or(5432);
            match host {
                postgres::config::Host::Tcp(host) => PostgresHostV1::Tcp(TcpPostgresHostV1 {
                    host: host.clone(),
                    port,
                }),
                postgres::config::Host::Unix(directory) => {
                    PostgresHostV1::Unix(UnixPostgresHostV1 {
                        directory: host_path(directory),
                        port,
                    })
                }
            }
        })
        .collect::<Vec<_>>();
    let database = config
        .get_dbname()
        .ok_or(AcquisitionError::InvalidCapture)?
        .to_owned();
    if hosts.is_empty() {
        return Err(AcquisitionError::InvalidCapture);
    }
    Ok(PostgresEndpointV1 {
        hosts,
        database,
        schema,
    })
}

fn verify_postgres_catalog(
    transaction: &mut postgres::Transaction<'_>,
    schema: &SqlSchemaV1,
) -> Result<(), AcquisitionError> {
    let names = transaction.query(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = current_schema() AND table_type = 'BASE TABLE' ORDER BY table_name",
        &[],
    ).map_err(|error| AcquisitionError::Postgres(error.to_string()))?.into_iter().map(|row| row.get::<_, String>(0)).collect::<Vec<_>>();
    let expected = schema
        .tables
        .iter()
        .map(|table| table.name.clone())
        .collect::<Vec<_>>();
    if names != expected {
        return Err(AcquisitionError::InvalidCapture);
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // Mirrors the fixed provider catalog in one auditable table.
fn fixed_schema(dialect: SqlDialectV1, namespace: &str) -> SqlSchemaV1 {
    let numeric = if dialect == SqlDialectV1::Sqlite {
        SqlTypeV1::Integer
    } else {
        SqlTypeV1::BigInt
    };
    let column = |ordinal, name: &str, sql_type| ColumnSchemaV1 {
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
            unique_keys: vec![],
            checks: vec![],
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
            unique_keys: vec![],
            checks: vec![],
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
            unique_keys: vec![],
            checks: vec![],
        },
    ];
    let mut indexes = Vec::new();
    if dialect == SqlDialectV1::Postgres {
        tables.push(TableSchemaV1 {
            name: "provider_sequences".into(),
            catalog_definition: PresenceV1::Missing,
            columns: vec![
                column(0, "namespace", SqlTypeV1::Text),
                column(1, "next_value", numeric),
            ],
            primary_key: PresenceV1::Present(key(&["namespace"])),
            unique_keys: vec![],
            checks: vec![CheckSchemaV1::NextValueNonnegative(
                NextValueCheckSchemaV1 {
                    name: PresenceV1::Missing,
                    catalog_expression: HexBytesV1::new(Vec::new()),
                    catalog_definition: PresenceV1::Missing,
                },
            )],
        });
        indexes.push(IndexSchemaV1 {
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
        });
    }
    tables.sort_by(|left, right| left.name.cmp(&right.name));
    SqlSchemaV1 {
        dialect,
        namespace: namespace.to_owned(),
        tables,
        indexes,
        foreign_objects: Vec::new(),
    }
}

fn scan_markdown(root: &Path) -> Result<MarkdownRawV1, AcquisitionError> {
    fn visit(
        root: &Path,
        directory: &Path,
        nodes: &mut Vec<MarkdownNodeV1>,
    ) -> Result<(), AcquisitionError> {
        let mut entries = fs::read_dir(directory)
            .map_err(|_| AcquisitionError::Io {
                path: directory.display().to_string(),
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AcquisitionError::Io {
                path: directory.display().to_string(),
            })?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .map_err(|_| AcquisitionError::InvalidCapture)?;
            let metadata = fs::symlink_metadata(&path).map_err(|_| AcquisitionError::Io {
                path: path.display().to_string(),
            })?;
            let node = if metadata.file_type().is_symlink() {
                let target = fs::read_link(&path).map_err(|_| AcquisitionError::Io {
                    path: path.display().to_string(),
                })?;
                MarkdownNodeKindV1::Symlink(SymlinkMarkdownNodeV1 {
                    target: host_path(&target),
                })
            } else if metadata.is_dir() {
                MarkdownNodeKindV1::Directory
            } else if metadata.is_file() {
                let bytes = fs::read(&path).map_err(|_| AcquisitionError::Io {
                    path: path.display().to_string(),
                })?;
                MarkdownNodeKindV1::Regular(RegularMarkdownNodeV1 {
                    bytes: HexBytesV1::new(bytes),
                })
            } else {
                MarkdownNodeKindV1::Other(OtherMarkdownNodeV1 {
                    kind: other_kind(&metadata),
                })
            };
            nodes.push(MarkdownNodeV1 {
                relative: host_path(relative),
                node,
            });
            if metadata.is_dir() {
                visit(root, &path, nodes)?;
            }
        }
        Ok(())
    }
    let mut nodes = Vec::new();
    visit(root, root, &mut nodes)?;
    nodes.sort_by(|left, right| left.relative.cmp(&right.relative));
    Ok(MarkdownRawV1 { nodes })
}

fn markdown_phase(
    phase: CapturePhaseV1,
    root: &HostPathV1,
    capture: &MarkdownRawV1,
) -> PhaseObservationV1 {
    let evidence = PhaseEvidenceV1::Markdown(MarkdownEvidenceV1 {
        root: PresenceV1::Present(root.clone()),
        nodes: ObservedListV1 {
            items: capture
                .nodes
                .iter()
                .cloned()
                .map(MarkdownNodeEvidenceV1::Captured)
                .collect(),
            terminal: EnumerationTerminalV1::Complete,
        },
    });
    let evidence_digest = evidence.evidence_digest();
    PhaseObservationV1 {
        phase,
        result: PhaseResultV1::Complete(CompletePhaseResultV1 {
            evidence,
            evidence_digest,
        }),
    }
}

#[cfg(unix)]
fn host_path(path: &Path) -> HostPathV1 {
    use std::os::unix::ffi::OsStrExt as _;
    HostPathV1::Unix(HexBytesV1::new(path.as_os_str().as_bytes().to_vec()))
}

#[cfg(windows)]
fn host_path(path: &Path) -> HostPathV1 {
    use std::os::windows::ffi::OsStrExt as _;
    HostPathV1::Windows(path.as_os_str().encode_wide().collect())
}

#[cfg(unix)]
fn other_kind(metadata: &fs::Metadata) -> OtherNodeKindV1 {
    use std::os::unix::fs::FileTypeExt as _;
    let kind = metadata.file_type();
    if kind.is_block_device() {
        OtherNodeKindV1::BlockDevice
    } else if kind.is_char_device() {
        OtherNodeKindV1::CharacterDevice
    } else if kind.is_fifo() {
        OtherNodeKindV1::Fifo
    } else if kind.is_socket() {
        OtherNodeKindV1::Socket
    } else {
        OtherNodeKindV1::Unknown
    }
}

#[cfg(windows)]
fn other_kind(_: &fs::Metadata) -> OtherNodeKindV1 {
    OtherNodeKindV1::Unknown
}
