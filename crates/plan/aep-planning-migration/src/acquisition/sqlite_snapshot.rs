//! One read-only SQLite snapshot, retaining every admitted prefix and failed row's cells.

use super::{
    complete_list, host_path, retained_failure, sql_phase_evidence, sqlite_catalog,
    AcquisitionError,
};
#[allow(clippy::wildcard_imports)]
use aep_contract::migration::*;
use rusqlite::{Connection, OpenFlags, Row};
use std::path::Path;

struct RowFailure<T> {
    prefix: Vec<T>,
    refusal: CaptureRefusalV1,
    rejected: Option<RejectedSqlRowV1>,
}

fn text(row: &Row<'_>, column: usize) -> Result<String, CaptureRefusalCodeV1> {
    match row
        .get_ref(column)
        .map_err(|_| CaptureRefusalCodeV1::ReadFailure)?
    {
        rusqlite::types::ValueRef::Text(value) => std::str::from_utf8(value)
            .map(str::to_owned)
            .map_err(|_| CaptureRefusalCodeV1::NonUtf8Text),
        rusqlite::types::ValueRef::Null => Err(CaptureRefusalCodeV1::SqlNull),
        _ => Err(CaptureRefusalCodeV1::SqlTypeMismatch),
    }
}

fn integer(row: &Row<'_>, column: usize) -> Result<i64, CaptureRefusalCodeV1> {
    match row
        .get_ref(column)
        .map_err(|_| CaptureRefusalCodeV1::ReadFailure)?
    {
        rusqlite::types::ValueRef::Integer(value) if value >= 0 => Ok(value),
        rusqlite::types::ValueRef::Integer(_) => Err(CaptureRefusalCodeV1::NumericOutOfRange),
        rusqlite::types::ValueRef::Null => Err(CaptureRefusalCodeV1::SqlNull),
        _ => Err(CaptureRefusalCodeV1::SqlTypeMismatch),
    }
}

fn entity(row: &Row<'_>) -> Result<String, CaptureRefusalCodeV1> {
    let value = text(row, 1)?;
    if matches!(
        value.as_str(),
        "aep.entity" | "aep.relation" | "aep.audit" | "aep.applied"
    ) {
        Ok(value)
    } else {
        Err(CaptureRefusalCodeV1::ForeignInstanceKind)
    }
}

fn document(row: &Row<'_>, column: usize) -> Result<HexBytesV1, CaptureRefusalCodeV1> {
    text(row, column).map(|value| HexBytesV1::new(value.into_bytes()))
}

fn row_cells(row: &Row<'_>) -> Result<Vec<SqlCellImageV1>, CaptureRefusalCodeV1> {
    let mut values = sqlite_catalog::cells(row).map_err(|_| CaptureRefusalCodeV1::ReadFailure)?;
    values.remove(0); // The actual rowid is retained as the physical coordinate, not a data column.
    for value in &mut values {
        value.ordinal -= 1;
    }
    Ok(values)
}

fn rows<T, F>(
    connection: &Connection,
    table: &str,
    sql: &str,
    mut map: F,
) -> Result<Vec<T>, Box<RowFailure<T>>>
where
    F: FnMut(&Row<'_>) -> Result<T, CaptureRefusalCodeV1>,
{
    let mut prefix = Vec::new();
    let mut at = PhysicalCoordinateV1::SqlTable(SqlTableCoordinateV1 {
        table: table.into(),
    });
    let mut rejected = None;
    let result = (|| {
        let mut statement = connection
            .prepare(sql)
            .map_err(|_| CaptureRefusalCodeV1::ReadFailure)?;
        let mut rows = statement
            .query([])
            .map_err(|_| CaptureRefusalCodeV1::ReadFailure)?;
        loop {
            at = PhysicalCoordinateV1::SqlTable(SqlTableCoordinateV1 {
                table: table.into(),
            });
            let Some(row) = rows.next().map_err(|_| CaptureRefusalCodeV1::ReadFailure)? else {
                return Ok(());
            };
            if let Ok(rowid) = row.get::<_, i64>(0) {
                at = PhysicalCoordinateV1::SqlPhysicalRow(SqlPhysicalRowCoordinateV1 {
                    table: table.into(),
                    locator: SqlPhysicalLocatorV1::SqliteRowId(rowid),
                });
            }
            rejected = Some(RejectedSqlRowV1 {
                at: at.clone(),
                cells: row_cells(row)?,
            });
            prefix.push(map(row)?);
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

fn snapshot_error(
    sql: SqlEvidenceV1,
    at: PhysicalCoordinateV1,
    code: CaptureRefusalCodeV1,
) -> AcquisitionError {
    retained_failure::sql_phase(sql, vec![CaptureRefusalV1 { at, code }])
}

pub(super) fn read(path: &Path) -> Result<SqlRawV1, AcquisitionError> {
    let source = SqlReplicaCoordinateV1::Sqlite(SqliteReplicaCoordinateV1 {
        database: host_path(path),
    });
    let root = PhysicalCoordinateV1::Root(RootCoordinateV1::SqliteDatabase(
        SqliteDatabaseRootCoordinateV1 {
            database: host_path(path),
        },
    ));
    let mut observed = retained_failure::empty_sql(source.clone());
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| {
        snapshot_error(
            observed.clone(),
            root.clone(),
            CaptureRefusalCodeV1::SourceUnreachable,
        )
    })?;
    connection
        .execute_batch("BEGIN DEFERRED TRANSACTION")
        .map_err(|_| {
            snapshot_error(
                observed.clone(),
                root.clone(),
                CaptureRefusalCodeV1::ReadFailure,
            )
        })?;
    let schema = sqlite_catalog::read_retained(&connection)
        .map_err(|failure| retained_failure::sql_catalog(source.clone(), *failure))?;
    let raw = SqlRawV1 {
        dialect: SqlDialectV1::Sqlite,
        schema,
        instances: Vec::new(),
        events: Vec::new(),
        history: Vec::new(),
        legacy_origins: Vec::new(),
        provider_sequences: SequenceRowsV1::NotApplicable,
    };
    let PhaseEvidenceV1::Sqlite(catalog) = sql_phase_evidence(&source, &raw) else {
        return Err(AcquisitionError::InvalidCapture);
    };
    observed.catalog = catalog.catalog;

    macro_rules! family {
        ($field:ident, $table:literal, $query:literal, $map:expr) => {
            match rows(&connection, $table, $query, $map) {
                Ok(values) => observed.$field = complete_list(values),
                Err(failure) => {
                    observed.$field = ObservedListV1 {
                        items: failure.prefix,
                        terminal: EnumerationTerminalV1::Refused(EnumerationRefusalV1 {
                            at: failure.refusal.at.clone(),
                            code: failure.refusal.code.clone(),
                        }),
                    };
                    if let Some(row) = failure.rejected {
                        observed.rejected_rows.push(row);
                    }
                    return Err(retained_failure::sql_phase(observed, vec![failure.refusal]));
                }
            }
        };
    }
    family!(instances, "instances", "SELECT rowid,entity,id,revision,document FROM instances ORDER BY entity COLLATE BINARY,id COLLATE BINARY", |row| Ok(InstanceRowV1 {
        entity: entity(row)?, id: text(row, 2)?, revision: integer(row, 3)?, document: document(row, 4)?,
    }));
    family!(events, "events", "SELECT rowid,entity,id,revision,position,document FROM events ORDER BY entity COLLATE BINARY,id COLLATE BINARY,revision,position", |row| Ok(EventRowV1 {
        entity: entity(row)?, id: text(row, 2)?, revision: integer(row, 3)?, position: integer(row, 4)?, document: document(row, 5)?,
    }));
    family!(history, "history", "SELECT rowid,entity,id,position,kind,record_id,document FROM history ORDER BY entity COLLATE BINARY,id COLLATE BINARY,position", |row| {
        let kind = match text(row, 4)?.as_str() {
            "decision" => HistoryKindV1::Decision, "observation" => HistoryKindV1::Observation,
            _ => return Err(CaptureRefusalCodeV1::UnknownHistoryKind),
        };
        Ok(HistoryRowV1 { entity: entity(row)?, id: text(row, 2)?, position: integer(row, 3)?, kind, record_id: text(row, 5)?, document: document(row, 6)? })
    });
    family!(legacy_origins, "legacy_origins", "SELECT rowid,entity,id,revision FROM legacy_origins ORDER BY entity COLLATE BINARY,id COLLATE BINARY", |row| Ok(LegacyOriginRowV1 {
        entity: entity(row)?, id: text(row, 2)?, revision: integer(row, 3)?,
    }));
    connection
        .execute_batch("COMMIT")
        .map_err(|_| snapshot_error(observed.clone(), root, CaptureRefusalCodeV1::ReadFailure))?;
    Ok(SqlRawV1 {
        instances: observed.instances.items,
        events: observed.events.items,
        history: observed.history.items,
        legacy_origins: observed.legacy_origins.items,
        ..raw
    })
}
