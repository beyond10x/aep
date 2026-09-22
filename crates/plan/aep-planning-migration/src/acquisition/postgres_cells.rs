//! Preserve the PostgreSQL wire value before attempting the capture's typed interpretation.

use super::AcquisitionError;
#[allow(clippy::wildcard_imports)]
use aep_contract::migration::*;
use postgres::types::{FromSql, Type};

struct RawCell(Vec<u8>);

impl<'a> FromSql<'a> for RawCell {
    fn from_sql(
        _: &Type,
        bytes: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        Ok(Self(bytes.to_vec()))
    }

    fn accepts(_: &Type) -> bool {
        true
    }
}

pub(super) fn cells(row: &postgres::Row) -> Result<Vec<SqlCellImageV1>, AcquisitionError> {
    row.columns()
        .iter()
        .enumerate()
        .map(|(ordinal, column)| {
            Ok(SqlCellImageV1 {
                ordinal: u16::try_from(ordinal).map_err(|_| AcquisitionError::InvalidCapture)?,
                column: column.name().to_owned(),
                value: cell(row, ordinal)?,
            })
        })
        .collect()
}

pub(super) fn cell(
    row: &postgres::Row,
    ordinal: usize,
) -> Result<SqlCellValueV1, AcquisitionError> {
    let column = row
        .columns()
        .get(ordinal)
        .ok_or(AcquisitionError::InvalidCapture)?;
    let raw = row
        .try_get::<_, Option<RawCell>>(ordinal)
        .map_err(|error| AcquisitionError::Postgres(error.to_string()))?;
    Ok(raw.map_or(SqlCellValueV1::Null, |raw| value(column.type_(), raw.0)))
}

fn value(kind: &Type, bytes: Vec<u8>) -> SqlCellValueV1 {
    // PostgreSQL's binary integer representation is signed, network byte order. Anything not
    // represented by a v1 scalar retains its actual type OID and original binary bytes.
    let integer = match kind.oid() {
        21 => <[u8; 2]>::try_from(bytes.as_slice())
            .ok()
            .map(|value| i64::from(i16::from_be_bytes(value))),
        23 => <[u8; 4]>::try_from(bytes.as_slice())
            .ok()
            .map(|value| i64::from(i32::from_be_bytes(value))),
        20 => <[u8; 8]>::try_from(bytes.as_slice())
            .ok()
            .map(i64::from_be_bytes),
        _ => None,
    };
    if let Some(integer) = integer {
        SqlCellValueV1::Integer(integer)
    } else if matches!(kind.oid(), 19 | 25 | 1042 | 1043) {
        SqlCellValueV1::Text(HexBytesV1::new(bytes))
    } else {
        SqlCellValueV1::PostgresBinary(PostgresBinaryCellV1 {
            type_id: kind.oid().to_string(),
            bytes: HexBytesV1::new(bytes),
        })
    }
}
