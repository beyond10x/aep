//! Physical read-only acquisition for legacy planning stores.

#![allow(missing_docs)]

use std::fs;
use std::path::Path;
use std::str::FromStr as _;

#[allow(clippy::wildcard_imports)]
// This adapter implements the complete closed migration vocabulary.
use aep_contract::migration::*;

#[cfg(test)]
mod filesystem_tests;
mod local_capture;
mod postgres_catalog;
mod postgres_cells;
mod postgres_snapshot;
#[cfg(test)]
mod postgres_tests;
mod retained_failure;
mod sqlite_catalog;
mod sqlite_snapshot;

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
    #[error("SQL snapshot acquisition was refused")]
    SqlPhase(Box<RefusedPhaseResultV1>),
    #[error("physical source capture was refused; retained observation is available")]
    RefusedObservation(Box<RawCaptureObservationV1>),
}

pub struct SelectorBinding {
    pub project_root: HostPathV1,
    pub project_file: HostPathV1,
    pub selector_digest: DigestV1,
    pub store_field: StoreFieldV1,
    pub config_digest: DigestV1,
}

/// Reads a Markdown tree twice without following symbolic links and retains every node byte.
// Preserve the public capture API's owned coordinate while both scan phases borrow it.
#[allow(clippy::needless_pass_by_value)]
pub fn capture_markdown(
    root: &Path,
    source_root: HostPathV1,
    selector: SelectorBinding,
) -> Result<RawCaptureObservationV1, AcquisitionError> {
    let coordinate = source_root.clone();
    markdown_scans(&source_root, selector, || {
        local_capture::scan(root, &coordinate)
    })
}

fn markdown_scans<F>(
    source_root: &HostPathV1,
    selector: SelectorBinding,
    mut scan: F,
) -> Result<RawCaptureObservationV1, AcquisitionError>
where
    F: FnMut() -> Result<MarkdownRawV1, Box<RefusedPhaseResultV1>>,
{
    let source = SourceCoordinateV1::Markdown(MarkdownSourceCoordinateV1 {
        root: source_root.clone(),
    });
    let first = match scan() {
        Ok(value) => value,
        Err(failure) => {
            return Err(retained_failure::observation(
                source,
                selector,
                ObservationMethodV1::MarkdownDoubleScan,
                vec![
                    PhaseObservationV1 {
                        phase: CapturePhaseV1::MarkdownFirst,
                        result: PhaseResultV1::Refused(*failure),
                    },
                    PhaseObservationV1 {
                        phase: CapturePhaseV1::MarkdownSecond,
                        result: PhaseResultV1::NotAttempted,
                    },
                ],
            ))
        }
    };
    let mut phases = vec![complete_phase(
        CapturePhaseV1::MarkdownFirst,
        markdown_evidence(source_root, &first),
    )];
    let second = match scan() {
        Ok(value) => value,
        Err(failure) => {
            phases.push(PhaseObservationV1 {
                phase: CapturePhaseV1::MarkdownSecond,
                result: PhaseResultV1::Refused(*failure),
            });
            return Err(retained_failure::observation(
                source,
                selector,
                ObservationMethodV1::MarkdownDoubleScan,
                phases,
            ));
        }
    };
    phases.push(complete_phase(
        CapturePhaseV1::MarkdownSecond,
        markdown_evidence(source_root, &second),
    ));
    if first != second {
        return unstable_observation(
            source,
            selector,
            ObservationMethodV1::MarkdownDoubleScan,
            phases,
            markdown_changed_coordinates(&first, &second),
        );
    }
    finish_complete(
        source,
        selector,
        ObservationMethodV1::MarkdownDoubleScan,
        phases,
        LegacyRawCaptureV1::Markdown(first),
    )
}

/// Captures all legacy SQLite rows inside one read-only transaction.
pub fn capture_sqlite(
    database: &Path,
    selector: SelectorBinding,
) -> Result<RawCaptureObservationV1, AcquisitionError> {
    let coordinate = SqliteSourceCoordinateV1 {
        database: host_path(database),
    };
    let raw = match sqlite_raw(database) {
        Ok(raw) => raw,
        Err(AcquisitionError::SqlPhase(failure)) => {
            return Err(retained_failure::observation(
                SourceCoordinateV1::Sqlite(coordinate),
                selector,
                ObservationMethodV1::SqliteReadTransaction,
                vec![PhaseObservationV1 {
                    phase: CapturePhaseV1::SqlSnapshot,
                    result: PhaseResultV1::Refused(*failure),
                }],
            ));
        }
        Err(error) => return Err(error),
    };
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
    let (raw, endpoint) = match postgres_raw(url) {
        Ok(value) => value,
        Err(AcquisitionError::SqlPhase(failure)) => {
            let source = match &failure.evidence {
                PhaseEvidenceV1::Postgres(sql) => match &sql.source {
                    PresenceV1::Present(SqlReplicaCoordinateV1::Postgres(source)) => {
                        PresenceV1::Present(SourceCoordinateV1::Postgres(
                            PostgresSourceCoordinateV1 {
                                endpoint: source.endpoint.clone(),
                                endpoint_id: source.endpoint_id,
                            },
                        ))
                    }
                    _ => PresenceV1::Missing,
                },
                _ => return Err(AcquisitionError::InvalidCapture),
            };
            return Err(retained_failure::partial_observation(
                source,
                selector,
                ObservationMethodV1::PostgresRepeatableReadOnly,
                vec![PhaseObservationV1 {
                    phase: CapturePhaseV1::SqlSnapshot,
                    result: PhaseResultV1::Refused(*failure),
                }],
            ));
        }
        Err(error) => return Err(error),
    };
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
    let replica = SqlReplicaCoordinateV1::Sqlite(SqliteReplicaCoordinateV1 {
        database: host_path(database),
    });
    capture_hybrid(
        local,
        divergence_file,
        policy,
        selector,
        &PresenceV1::Present(replica),
        || {
            Ok((
                sqlite_raw(database)?,
                SqlReplicaCoordinateV1::Sqlite(SqliteReplicaCoordinateV1 {
                    database: host_path(database),
                }),
            ))
        },
    )
}

/// Captures the local tree and divergence file both before and after one PostgreSQL snapshot.
pub fn capture_hybrid_postgres(
    local: &Path,
    url: &str,
    divergence_file: &Path,
    policy: HybridPolicyWordsV1,
    selector: SelectorBinding,
) -> Result<RawCaptureObservationV1, AcquisitionError> {
    capture_hybrid(
        local,
        divergence_file,
        policy,
        selector,
        &PresenceV1::Missing,
        || {
            let (raw, endpoint) = postgres_raw(url)?;
            let endpoint_id = endpoint.endpoint_id();
            Ok((
                raw,
                SqlReplicaCoordinateV1::Postgres(PostgresReplicaCoordinateV1 {
                    endpoint,
                    endpoint_id,
                }),
            ))
        },
    )
}

fn hybrid_source(
    local: &Path,
    divergence: &Path,
    policy: &HybridPolicyWordsV1,
    replica: &PresenceV1<SqlReplicaCoordinateV1>,
) -> PresenceV1<SourceCoordinateV1> {
    match replica {
        PresenceV1::Missing => PresenceV1::Missing,
        PresenceV1::Present(replica) => {
            PresenceV1::Present(SourceCoordinateV1::Hybrid(HybridSourceCoordinateV1 {
                local_root: host_path(local),
                replica: replica.clone(),
                divergence_file: host_path(divergence),
                policy: PresenceV1::Present(policy.clone()),
            }))
        }
    }
}

fn hybrid_failure(
    source: PresenceV1<SourceCoordinateV1>,
    selector: SelectorBinding,
    mut phases: Vec<PhaseObservationV1>,
    failed: CapturePhaseV1,
    failure: RefusedPhaseResultV1,
) -> AcquisitionError {
    phases.push(PhaseObservationV1 {
        phase: failed,
        result: PhaseResultV1::Refused(failure),
    });
    let roster = [
        CapturePhaseV1::LocalBefore,
        CapturePhaseV1::DivergencesBefore,
        CapturePhaseV1::ReplicaSnapshot,
        CapturePhaseV1::LocalAfter,
        CapturePhaseV1::DivergencesAfter,
    ];
    for phase in roster.into_iter().skip(phases.len()) {
        phases.push(PhaseObservationV1 {
            phase,
            result: PhaseResultV1::NotAttempted,
        });
    }
    retained_failure::partial_observation(
        source,
        selector,
        ObservationMethodV1::HybridBracketedSqlSnapshot,
        phases,
    )
}

// The five bracketed phases must stay together to preserve the first failed coordinate.
#[allow(clippy::too_many_lines)]
fn capture_hybrid<F>(
    local: &Path,
    divergence_file: &Path,
    policy: HybridPolicyWordsV1,
    selector: SelectorBinding,
    initial_replica: &PresenceV1<SqlReplicaCoordinateV1>,
    acquire_replica: F,
) -> Result<RawCaptureObservationV1, AcquisitionError>
where
    F: FnOnce() -> Result<(SqlRawV1, SqlReplicaCoordinateV1), AcquisitionError>,
{
    let mut source = hybrid_source(local, divergence_file, &policy, initial_replica);
    let mut phases = Vec::new();
    macro_rules! local_phase {
        ($phase:ident, $read:expr, $evidence:expr) => {
            match $read {
                Ok(value) => {
                    phases.push(complete_phase(CapturePhaseV1::$phase, ($evidence)(&value)));
                    value
                }
                Err(failure) => {
                    return Err(hybrid_failure(
                        source,
                        selector,
                        phases,
                        CapturePhaseV1::$phase,
                        *failure,
                    ))
                }
            }
        };
    }
    let local_before = local_phase!(
        LocalBefore,
        local_capture::scan(local, &host_path(local)),
        |raw| markdown_evidence(&host_path(local), raw)
    );
    let divergences_before = local_phase!(
        DivergencesBefore,
        local_capture::divergence(divergence_file),
        |image: &FileImageV1| PhaseEvidenceV1::Divergences(ObservedValueV1::Complete(
            image.clone()
        ))
    );
    // Do not discover a PostgreSQL endpoint before the local bracket or after a failed phase.
    let (raw, replica) = match acquire_replica() {
        Ok(value) => value,
        Err(AcquisitionError::SqlPhase(failure)) => {
            let observed_replica = match &failure.evidence {
                PhaseEvidenceV1::Sqlite(sql) | PhaseEvidenceV1::Postgres(sql) => sql.source.clone(),
                _ => return Err(AcquisitionError::InvalidCapture),
            };
            if matches!(observed_replica, PresenceV1::Present(_)) {
                source = hybrid_source(local, divergence_file, &policy, &observed_replica);
            }
            return Err(hybrid_failure(
                source,
                selector,
                phases,
                CapturePhaseV1::ReplicaSnapshot,
                *failure,
            ));
        }
        Err(error) => return Err(error),
    };
    source = hybrid_source(
        local,
        divergence_file,
        &policy,
        &PresenceV1::Present(replica.clone()),
    );
    phases.push(complete_phase(
        CapturePhaseV1::ReplicaSnapshot,
        sql_phase_evidence(&replica, &raw),
    ));
    let local_after = local_phase!(
        LocalAfter,
        local_capture::scan(local, &host_path(local)),
        |raw| markdown_evidence(&host_path(local), raw)
    );
    let divergences_after = local_phase!(
        DivergencesAfter,
        local_capture::divergence(divergence_file),
        |image: &FileImageV1| PhaseEvidenceV1::Divergences(ObservedValueV1::Complete(
            image.clone()
        ))
    );
    let PresenceV1::Present(source) = source else {
        return Err(AcquisitionError::InvalidCapture);
    };
    if local_before != local_after || divergences_before != divergences_after {
        let mut changed = markdown_changed_coordinates(&local_before, &local_after);
        if divergences_before != divergences_after {
            changed.push(PhysicalCoordinateV1::Root(RootCoordinateV1::HybridSide(
                HybridSideRootCoordinateV1 {
                    side: HybridSideV1::Divergences,
                },
            )));
        }
        return unstable_observation(
            source,
            selector,
            ObservationMethodV1::HybridBracketedSqlSnapshot,
            phases,
            changed,
        );
    }
    finish_complete(
        source,
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
    method: ObservationMethodV1,
    phases: Vec<PhaseObservationV1>,
    mut changed: Vec<PhysicalCoordinateV1>,
) -> Result<RawCaptureObservationV1, AcquisitionError> {
    changed.sort_by_key(sort_key_v1);
    changed.dedup();
    let config_digest = selector.config_digest;
    let observation = RawCaptureObservationV1 {
        format: RawCaptureFormatV1,
        source: PresenceV1::Present(source),
        selector: PresenceV1::Present(selector_coordinate(selector)),
        config_digest: PresenceV1::Present(config_digest),
        observation: ObservationOutcomeV1::Unstable(UnstableObservationV1 {
            method,
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
    sqlite_snapshot::read(path)
}

fn postgres_raw(url: &str) -> Result<(SqlRawV1, PostgresEndpointV1), AcquisitionError> {
    postgres_snapshot::read(url)
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
