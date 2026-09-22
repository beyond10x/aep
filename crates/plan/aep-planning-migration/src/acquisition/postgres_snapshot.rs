//! A repeatable-read, read-only PostgreSQL snapshot with retained acquisition failures.

use super::{
    complete_list, postgres_catalog, postgres_cells, postgres_endpoint, retained_failure,
    sql_phase_evidence, AcquisitionError,
};
#[allow(clippy::wildcard_imports)]
use aep_contract::migration::*;
use postgres::fallible_iterator::FallibleIterator as _;
use postgres::{IsolationLevel, NoTls, Row};
use std::str::FromStr as _;

struct RowFailure<T> {
    prefix: Vec<T>,
    refusal: CaptureRefusalV1,
    rejected: Option<RejectedSqlRowV1>,
}

fn text(row: &Row, column: usize) -> Result<String, CaptureRefusalCodeV1> {
    match postgres_cells::cell(row, column).map_err(|_| CaptureRefusalCodeV1::ReadFailure)? {
        SqlCellValueV1::Text(bytes) => String::from_utf8(bytes.as_bytes().to_vec())
            .map_err(|_| CaptureRefusalCodeV1::NonUtf8Text),
        SqlCellValueV1::Null => Err(CaptureRefusalCodeV1::SqlNull),
        _ => Err(CaptureRefusalCodeV1::SqlTypeMismatch),
    }
}

fn integer(row: &Row, column: usize) -> Result<i64, CaptureRefusalCodeV1> {
    match postgres_cells::cell(row, column).map_err(|_| CaptureRefusalCodeV1::ReadFailure)? {
        SqlCellValueV1::Integer(value) if value >= 0 => Ok(value),
        SqlCellValueV1::Integer(_) => Err(CaptureRefusalCodeV1::NumericOutOfRange),
        SqlCellValueV1::Null => Err(CaptureRefusalCodeV1::SqlNull),
        _ => Err(CaptureRefusalCodeV1::SqlTypeMismatch),
    }
}

fn entity(row: &Row) -> Result<String, CaptureRefusalCodeV1> {
    let value = text(row, 2)?;
    if matches!(
        value.as_str(),
        "aep.entity" | "aep.relation" | "aep.audit" | "aep.applied"
    ) {
        Ok(value)
    } else {
        Err(CaptureRefusalCodeV1::ForeignInstanceKind)
    }
}

fn document(row: &Row, column: usize) -> Result<HexBytesV1, CaptureRefusalCodeV1> {
    text(row, column).map(|value| HexBytesV1::new(value.into_bytes()))
}

fn locator(row: &Row) -> Result<PostgresTupleLocatorV1, CaptureRefusalCodeV1> {
    let table_oid = row
        .try_get(0)
        .map_err(|_| CaptureRefusalCodeV1::ReadFailure)?;
    let tid = text(row, 1)?;
    let (block, offset) = tid
        .strip_prefix('(')
        .and_then(|tid| tid.strip_suffix(')'))
        .and_then(|tid| tid.split_once(','))
        .ok_or(CaptureRefusalCodeV1::ReadFailure)?;
    Ok(PostgresTupleLocatorV1 {
        table_oid,
        block: block
            .parse()
            .map_err(|_| CaptureRefusalCodeV1::ReadFailure)?,
        offset: offset
            .parse()
            .map_err(|_| CaptureRefusalCodeV1::ReadFailure)?,
    })
}

fn row_cells(row: &Row) -> Result<Vec<SqlCellImageV1>, CaptureRefusalCodeV1> {
    let mut values = postgres_cells::cells(row).map_err(|_| CaptureRefusalCodeV1::ReadFailure)?;
    // tableoid and ctid describe the physical row; the remaining columns are its data image.
    values.drain(..2);
    for value in &mut values {
        value.ordinal -= 2;
    }
    Ok(values)
}

fn rows<T, F>(
    transaction: &mut postgres::Transaction<'_>,
    namespace: &str,
    table: &str,
    columns: &str,
    order: &str,
    mut map: F,
) -> Result<Vec<T>, Box<RowFailure<T>>>
where
    F: FnMut(&Row) -> Result<T, CaptureRefusalCodeV1>,
{
    // Bind rows to the admitted namespace. An earlier pg_temp/search_path entry must not redirect
    // reads after the catalog for another namespace was admitted. Identifiers are quoted, never
    // interpreted as an operator-supplied SQL fragment; columns/order are private constants.
    let query = format!(
        "SELECT tableoid,ctid::text,{columns} FROM \"{}\".\"{}\" ORDER BY {order}",
        namespace.replace('"', "\"\""),
        table.replace('"', "\"\""),
    );
    let mut prefix = Vec::new();
    let mut at = PhysicalCoordinateV1::SqlTable(SqlTableCoordinateV1 {
        table: table.into(),
    });
    let mut rejected = None;
    let result = (|| {
        let params: [&(dyn postgres::types::ToSql + Sync); 0] = [];
        let mut rows = transaction
            .query_raw(&query, params)
            .map_err(|_| CaptureRefusalCodeV1::ReadFailure)?;
        loop {
            at = PhysicalCoordinateV1::SqlTable(SqlTableCoordinateV1 {
                table: table.into(),
            });
            let Some(row) = rows.next().map_err(|_| CaptureRefusalCodeV1::ReadFailure)? else {
                return Ok(());
            };
            let physical = locator(&row);
            if let Ok(locator) = &physical {
                at = PhysicalCoordinateV1::SqlPhysicalRow(SqlPhysicalRowCoordinateV1 {
                    table: table.into(),
                    locator: SqlPhysicalLocatorV1::PostgresTuple(locator.clone()),
                });
            }
            rejected = Some(RejectedSqlRowV1 {
                at: at.clone(),
                cells: row_cells(&row)?,
            });
            physical?;
            prefix.push(map(&row)?);
            rejected = None;
        }
    })();
    match result {
        Ok(()) => Ok(prefix),
        Err(code) => Err(Box::new(RowFailure {
            prefix,
            refusal: CaptureRefusalV1 { code, at },
            rejected,
        })),
    }
}

fn snapshot_error(observed: &SqlEvidenceV1, code: CaptureRefusalCodeV1) -> AcquisitionError {
    let (endpoint, endpoint_id) = match &observed.source {
        PresenceV1::Present(SqlReplicaCoordinateV1::Postgres(source)) => (
            PresenceV1::Present(source.endpoint.clone()),
            PresenceV1::Present(source.endpoint_id),
        ),
        _ => (PresenceV1::Missing, PresenceV1::Missing),
    };
    retained_failure::sql_phase(
        observed.clone(),
        vec![CaptureRefusalV1 {
            code,
            at: PhysicalCoordinateV1::Root(RootCoordinateV1::PostgresEndpoint(
                PostgresEndpointRootCoordinateV1 {
                    endpoint,
                    endpoint_id,
                },
            )),
        }],
    )
}

fn retain_rows<T>(
    result: Result<Vec<T>, Box<RowFailure<T>>>,
    target: &mut ObservedListV1<T>,
    rejected: &mut Vec<RejectedSqlRowV1>,
) -> Result<(), CaptureRefusalV1> {
    match result {
        Ok(values) => {
            *target = complete_list(values);
            Ok(())
        }
        Err(failure) => {
            *target = ObservedListV1 {
                items: failure.prefix,
                terminal: EnumerationTerminalV1::Refused(EnumerationRefusalV1 {
                    at: failure.refusal.at.clone(),
                    code: failure.refusal.code.clone(),
                }),
            };
            if let Some(row) = failure.rejected {
                rejected.push(row);
            }
            Err(failure.refusal)
        }
    }
}

pub(super) fn read(url: &str) -> Result<(SqlRawV1, PostgresEndpointV1), AcquisitionError> {
    let observed = retained_failure::unresolved_postgres();
    let config = postgres::Config::from_str(url)
        .map_err(|_| snapshot_error(&observed, CaptureRefusalCodeV1::UnresolvedSourceCoordinate))?;
    read_config(&config)
}

// The snapshot reads the admitted row families in capture order and stops on the first refusal.
#[allow(clippy::too_many_lines)]
pub(super) fn read_config(
    config: &postgres::Config,
) -> Result<(SqlRawV1, PostgresEndpointV1), AcquisitionError> {
    let mut observed = retained_failure::unresolved_postgres();
    let mut client = config
        .connect(NoTls)
        .map_err(|_| snapshot_error(&observed, CaptureRefusalCodeV1::SourceUnreachable))?;
    let mut transaction = client
        .build_transaction()
        .isolation_level(IsolationLevel::RepeatableRead)
        .read_only(true)
        .start()
        .map_err(|_| snapshot_error(&observed, CaptureRefusalCodeV1::ReadFailure))?;
    let namespace: Option<String> = transaction
        .query_one("SELECT current_schema()", &[])
        .and_then(|row| row.try_get(0))
        .map_err(|_| snapshot_error(&observed, CaptureRefusalCodeV1::ReadFailure))?;
    let namespace = namespace.ok_or_else(|| {
        snapshot_error(&observed, CaptureRefusalCodeV1::UnresolvedSourceCoordinate)
    })?;
    let endpoint = postgres_endpoint(config, namespace.clone())
        .map_err(|_| snapshot_error(&observed, CaptureRefusalCodeV1::UnresolvedSourceCoordinate))?;
    let source = SqlReplicaCoordinateV1::Postgres(PostgresReplicaCoordinateV1 {
        endpoint: endpoint.clone(),
        endpoint_id: endpoint.endpoint_id(),
    });
    observed = retained_failure::empty_sql(source.clone());
    let schema = postgres_catalog::read_retained(&mut transaction, &namespace)
        .map_err(|failure| retained_failure::sql_catalog(source.clone(), *failure))?;
    let raw = SqlRawV1 {
        dialect: SqlDialectV1::Postgres,
        schema,
        instances: Vec::new(),
        events: Vec::new(),
        history: Vec::new(),
        legacy_origins: Vec::new(),
        provider_sequences: SequenceRowsV1::Rows(Vec::new()),
    };
    let PhaseEvidenceV1::Postgres(catalog) = sql_phase_evidence(&source, &raw) else {
        return Err(AcquisitionError::InvalidCapture);
    };
    observed.catalog = catalog.catalog;

    macro_rules! family {
        ($field:ident, $table:literal, $columns:literal, $order:literal, $map:expr) => {
            if let Err(refusal) = retain_rows(
                rows(&mut transaction, &namespace, $table, $columns, $order, $map),
                &mut observed.$field,
                &mut observed.rejected_rows,
            ) {
                return Err(retained_failure::sql_phase(observed, vec![refusal]));
            }
        };
    }
    family!(
        instances,
        "instances",
        "entity,id,revision,document",
        r#"entity COLLATE "C",id COLLATE "C""#,
        |row| Ok(InstanceRowV1 {
            entity: entity(row)?,
            id: text(row, 3)?,
            revision: integer(row, 4)?,
            document: document(row, 5)?,
        })
    );
    family!(
        events,
        "events",
        "entity,id,revision,position,document",
        r#"entity COLLATE "C",id COLLATE "C",revision,position"#,
        |row| Ok(EventRowV1 {
            entity: entity(row)?,
            id: text(row, 3)?,
            revision: integer(row, 4)?,
            position: integer(row, 5)?,
            document: document(row, 6)?,
        })
    );
    family!(
        history,
        "history",
        "entity,id,position,kind,record_id,document",
        r#"entity COLLATE "C",id COLLATE "C",position"#,
        |row| {
            let kind = match text(row, 5)?.as_str() {
                "decision" => HistoryKindV1::Decision,
                "observation" => HistoryKindV1::Observation,
                _ => return Err(CaptureRefusalCodeV1::UnknownHistoryKind),
            };
            Ok(HistoryRowV1 {
                entity: entity(row)?,
                id: text(row, 3)?,
                position: integer(row, 4)?,
                kind,
                record_id: text(row, 6)?,
                document: document(row, 7)?,
            })
        }
    );
    family!(
        legacy_origins,
        "legacy_origins",
        "entity,id,revision",
        r#"entity COLLATE "C",id COLLATE "C""#,
        |row| Ok(LegacyOriginRowV1 {
            entity: entity(row)?,
            id: text(row, 3)?,
            revision: integer(row, 4)?,
        })
    );
    let ObservedSequenceRowsV1::Observed(sequences) = &mut observed.provider_sequences else {
        return Err(AcquisitionError::InvalidCapture);
    };
    if let Err(refusal) = retain_rows(
        rows(
            &mut transaction,
            &namespace,
            "provider_sequences",
            "namespace,next_value",
            r#"namespace COLLATE "C""#,
            |row| {
                Ok(ProviderSequenceRowV1 {
                    namespace: text(row, 2)?,
                    next_value: integer(row, 3)?,
                })
            },
        ),
        sequences,
        &mut observed.rejected_rows,
    ) {
        return Err(retained_failure::sql_phase(observed, vec![refusal]));
    }
    transaction
        .commit()
        .map_err(|_| snapshot_error(&observed, CaptureRefusalCodeV1::ReadFailure))?;
    let ObservedSequenceRowsV1::Observed(sequences) = observed.provider_sequences else {
        return Err(AcquisitionError::InvalidCapture);
    };
    Ok((
        SqlRawV1 {
            instances: observed.instances.items,
            events: observed.events.items,
            history: observed.history.items,
            legacy_origins: observed.legacy_origins.items,
            provider_sequences: SequenceRowsV1::Rows(sequences.items),
            ..raw
        },
        endpoint,
    ))
}
