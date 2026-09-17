//! SQLite catalog capture for the admitted legacy provider declarations.
//!
//! PRAGMAs expose columns and backing indexes but not CHECK expressions or constraint names.
//! Read those from the catalog's own CREATE TABLE statement, preserving their original bytes.
//! The bounded declaration reader refuses syntax outside the existing capture vocabulary.

use super::retained_failure::{unattempted, CatalogFailure, CatalogProgress as Progress};
use super::AcquisitionError;
#[allow(clippy::wildcard_imports)]
use aep_contract::migration::*;

fn invalid<T>() -> Result<T, AcquisitionError> {
    Err(AcquisitionError::InvalidCapture)
}

fn bytes(text: &str) -> HexBytesV1 {
    HexBytesV1::new(text.as_bytes().to_vec())
}

#[cfg(test)]
fn read(connection: &rusqlite::Connection) -> Result<SqlSchemaV1, AcquisitionError> {
    read_retained(connection).map_err(|_| AcquisitionError::InvalidCapture)
}

// `map_err` hands this adapter its error by value; retain only its display text.
#[allow(clippy::needless_pass_by_value)]
fn database(error: rusqlite::Error) -> AcquisitionError {
    AcquisitionError::Sqlite(error.to_string())
}

pub(super) fn cells(row: &rusqlite::Row<'_>) -> rusqlite::Result<Vec<SqlCellImageV1>> {
    use rusqlite::types::ValueRef;
    row.as_ref()
        .column_names()
        .iter()
        .enumerate()
        .map(|(ordinal, column)| {
            let value = match row.get_ref(ordinal)? {
                ValueRef::Null => SqlCellValueV1::Null,
                ValueRef::Integer(value) => SqlCellValueV1::Integer(value),
                ValueRef::Real(value) => SqlCellValueV1::RealBits(value.to_bits()),
                ValueRef::Text(value) => SqlCellValueV1::Text(HexBytesV1::new(value.to_vec())),
                ValueRef::Blob(value) => SqlCellValueV1::Blob(HexBytesV1::new(value.to_vec())),
            };
            Ok(SqlCellImageV1 {
                ordinal: u16::try_from(ordinal).map_err(|_| rusqlite::Error::InvalidQuery)?,
                column: (*column).to_owned(),
                value,
            })
        })
        .collect()
}

// Keep a query's obtained, not-yet-admitted cells until the visitor records their typed meaning.
// One query per family gives every retained row an unambiguous catalog-family ordinal.
fn visit_rows<F>(
    connection: &rusqlite::Connection,
    sql: &str,
    params: &[&dyn rusqlite::ToSql],
    progress: &mut Progress,
    mut visit: F,
) -> Result<(), AcquisitionError>
where
    F: FnMut(&rusqlite::Row<'_>, &mut Progress) -> Result<(), AcquisitionError>,
{
    let mut statement = connection.prepare(sql).map_err(database)?;
    let mut rows = statement.query(params).map_err(database)?;
    let mut ordinal = 0_u64;
    loop {
        progress.at.row = PresenceV1::Missing;
        let Some(row) = rows.next().map_err(database)? else {
            return Ok(());
        };
        progress.at.row = PresenceV1::Present(ordinal);
        progress.pending.push(RejectedCatalogRowV1 {
            at: PhysicalCoordinateV1::SqlCatalog(progress.at.clone()),
            cells: cells(row).map_err(database)?,
        });
        visit(row, progress)?;
        ordinal += 1;
    }
}

pub(super) fn read_retained(
    connection: &rusqlite::Connection,
) -> Result<SqlSchemaV1, Box<CatalogFailure>> {
    let mut progress = Progress {
        catalog: SqlCatalogEvidenceV1 {
            namespace: PresenceV1::Present("main".into()),
            objects: unattempted(),
            tables: Vec::new(),
            indexes: unattempted(),
            foreign_objects: unattempted(),
            rejected_rows: Vec::new(),
        },
        at: SqlCatalogCoordinateV1 {
            namespace: PresenceV1::Present("main".into()),
            family: CatalogFamilyV1::Objects,
            table: PresenceV1::Missing,
            row: PresenceV1::Missing,
        },
        pending: Vec::new(),
    };
    read_inner(connection, &mut progress).map_err(|error| progress.refused(&error))
}

// The catalog families must be observed in this fixed order for retained-prefix evidence.
#[allow(clippy::too_many_lines)]
fn read_inner(
    connection: &rusqlite::Connection,
    progress: &mut Progress,
) -> Result<SqlSchemaV1, AcquisitionError> {
    visit_rows(
        connection,
        "SELECT type,name,tbl_name FROM main.sqlite_schema \
         WHERE (name NOT GLOB 'sqlite_*' OR type='index') \
         AND NOT (type='index' AND sql IS NULL) ORDER BY type,name COLLATE BINARY",
        &[],
        progress,
        |row, progress| {
            let kind: String = row.get(0).map_err(database)?;
            let name: String = row.get(1).map_err(database)?;
            let owner: String = row.get(2).map_err(database)?;
            if kind == "table" && name != owner {
                return invalid();
            }
            let object_kind = match kind.as_str() {
                "table" => ForeignObjectKindV1::Table,
                "index" => ForeignObjectKindV1::Index,
                "view" => ForeignObjectKindV1::View,
                "trigger" => ForeignObjectKindV1::Trigger,
                _ => ForeignObjectKindV1::Other(kind.clone()),
            };
            progress.catalog.objects.items.push(CatalogObjectHeaderV1 {
                kind: object_kind,
                name: name.clone(),
                parent: if matches!(kind.as_str(), "index" | "trigger") {
                    PresenceV1::Present(owner)
                } else {
                    PresenceV1::Missing
                },
            });
            if kind == "table" {
                progress.catalog.tables.push(TableCatalogEvidenceV1 {
                    name,
                    catalog_definition: ObservedValueV1::NotAttempted,
                    columns: unattempted(),
                    primary_key: ObservedValueV1::NotAttempted,
                    unique_keys: unattempted(),
                    checks: unattempted(),
                });
            }
            progress.pending.clear();
            Ok(())
        },
    )?;
    progress.catalog.objects.terminal = EnumerationTerminalV1::Complete;
    let mut tables = Vec::new();
    for index in 0..progress.catalog.tables.len() {
        let name = progress.catalog.tables[index].name.clone();
        progress.next(
            CatalogFamilyV1::TableDefinition,
            PresenceV1::Present(name.clone()),
        );
        let mut definition = None;
        visit_rows(
            connection,
            "SELECT sql FROM main.sqlite_schema WHERE type='table' AND name=?1",
            &[&name],
            progress,
            |row, progress| {
                let text: Option<String> = row.get(0).map_err(database)?;
                progress.catalog.tables[index].catalog_definition = ObservedValueV1::Complete(
                    text.as_ref()
                        .map_or(PresenceV1::Missing, |text| PresenceV1::Present(bytes(text))),
                );
                definition = text;
                Ok(())
            },
        )?;
        if !progress.pending.is_empty() {
            progress.at.row = PresenceV1::Present(0);
        }
        let mut table = declaration(&name, &definition.ok_or(AcquisitionError::InvalidCapture)?)?;
        progress.pending.clear();
        progress.next(CatalogFamilyV1::Columns, PresenceV1::Present(name.clone()));
        observe_columns(connection, &mut table, index, progress)?;
        progress.next(
            CatalogFamilyV1::PrimaryKey,
            PresenceV1::Present(name.clone()),
        );
        observe_keys(connection, &mut table, index, "pk", progress)?;
        progress.next(
            CatalogFamilyV1::UniqueKeys,
            PresenceV1::Present(name.clone()),
        );
        observe_keys(connection, &mut table, index, "u", progress)?;
        progress.next(CatalogFamilyV1::Checks, PresenceV1::Present(name));
        table.checks.sort_by_key(sort_key_v1);
        // CHECK declarations were obtained as part of this table's retained DDL, not guessed.
        progress.catalog.tables[index].checks = super::complete_list(table.checks.clone());
        tables.push(table);
    }
    progress.next(CatalogFamilyV1::Indexes, PresenceV1::Missing);
    visit_rows(connection,
        "SELECT name,tbl_name,sql FROM main.sqlite_schema WHERE type='index' AND sql IS NOT NULL ORDER BY name COLLATE BINARY",
        &[], progress, |_, _| invalid())?;
    progress.catalog.indexes = super::complete_list(Vec::new());
    progress.next(CatalogFamilyV1::ForeignObjects, PresenceV1::Missing);
    visit_rows(connection,
        "SELECT type,name,tbl_name,sql FROM main.sqlite_schema WHERE name NOT GLOB 'sqlite_*' AND type NOT IN ('table','index') ORDER BY type,name COLLATE BINARY",
        &[], progress, |row, progress| {
            let kind: String = row.get(0).map_err(database)?;
            let name: String = row.get(1).map_err(database)?;
            let owner: String = row.get(2).map_err(database)?;
            let definition: Option<String> = row.get(3).map_err(database)?;
            progress.catalog.foreign_objects.items.push(ForeignObjectV1 {
                kind: match kind.as_str() {
                    "view" => ForeignObjectKindV1::View,
                    "trigger" => ForeignObjectKindV1::Trigger,
                    _ => ForeignObjectKindV1::Other(kind.clone()),
                },
                name,
                owner: if kind == "trigger" { PresenceV1::Present(owner) } else { PresenceV1::Missing },
                catalog_definition: definition.map_or(PresenceV1::Missing, |value| PresenceV1::Present(bytes(&value))),
            });
            progress.catalog.foreign_objects.items.sort_by_key(sort_key_v1);
            progress.pending.clear();
            Ok(())
        })?;
    progress.catalog.foreign_objects.terminal = EnumerationTerminalV1::Complete;
    if !progress.catalog.foreign_objects.items.is_empty() {
        return invalid();
    }
    let schema = SqlSchemaV1 {
        dialect: SqlDialectV1::Sqlite,
        namespace: "main".into(),
        tables,
        indexes: Vec::new(),
        foreign_objects: Vec::new(),
    };
    if !schema.is_known_provider_schema() {
        progress.next(CatalogFamilyV1::Objects, PresenceV1::Missing);
        return invalid();
    }
    Ok(schema)
}

fn observe_columns(
    connection: &rusqlite::Connection,
    table: &mut TableSchemaV1,
    index: usize,
    progress: &mut Progress,
) -> Result<(), AcquisitionError> {
    let mut primary = Vec::new();
    visit_rows(connection,
        "SELECT cid,name,type,\"notnull\",dflt_value,pk,hidden FROM pragma_table_xinfo(?1,'main') ORDER BY cid",
        &[&table.name.clone()], progress, |row, progress| {
            let ordinal: i64 = row.get(0).map_err(database)?;
            let name: String = row.get(1).map_err(database)?;
            let declared: String = row.get(2).map_err(database)?;
            let not_null: bool = row.get(3).map_err(database)?;
            let default: Option<String> = row.get(4).map_err(database)?;
            let primary_position: i64 = row.get(5).map_err(database)?;
            let hidden: i64 = row.get(6).map_err(database)?;
            let position = progress.catalog.tables[index].columns.items.len();
            let column = table.columns.get_mut(position).ok_or(AcquisitionError::InvalidCapture)?;
            if ordinal != i64::from(column.ordinal) || name != column.name || not_null != column.not_null
                || !declared.eq_ignore_ascii_case(std::str::from_utf8(column.declared_type.as_bytes()).map_err(|_| AcquisitionError::InvalidCapture)?)
                || default.is_some() || hidden != 0 {
                return invalid();
            }
            column.declared_type = bytes(&declared);
            if primary_position > 0 { primary.push((primary_position, name)); }
            progress.catalog.tables[index].columns.items.push(column.clone());
            progress.pending.clear();
            Ok(())
        })?;
    if progress.catalog.tables[index].columns.items.len() != table.columns.len() {
        return invalid();
    }
    primary.sort();
    let primary = primary
        .into_iter()
        .map(|(_, name)| name)
        .collect::<Vec<_>>();
    match &table.primary_key {
        PresenceV1::Present(key) if key.columns == primary => {}
        PresenceV1::Missing if primary.is_empty() => {}
        _ => return invalid(),
    }
    progress.catalog.tables[index].columns.terminal = EnumerationTerminalV1::Complete;
    Ok(())
}

struct KeyRows {
    name: String,
    columns: Vec<String>,
    next_term: i64,
}

fn admit_key(
    table: &mut TableSchemaV1,
    index: usize,
    origin: &str,
    key_rows: KeyRows,
    progress: &mut Progress,
) -> Result<(), AcquisitionError> {
    let target = if origin == "pk" {
        match &mut table.primary_key {
            PresenceV1::Present(key) => Some(key),
            PresenceV1::Missing => None,
        }
    } else {
        table.unique_keys.iter_mut().find(|key| {
            key.columns == key_rows.columns && matches!(key.backing_index, PresenceV1::Missing)
        })
    }
    .ok_or(AcquisitionError::InvalidCapture)?;
    if target.columns != key_rows.columns || !matches!(target.backing_index, PresenceV1::Missing) {
        return invalid();
    }
    target.backing_index = PresenceV1::Present(key_rows.name);
    if origin == "pk" {
        progress.catalog.tables[index].primary_key =
            ObservedValueV1::Complete(PresenceV1::Present(target.clone()));
    } else {
        progress.catalog.tables[index]
            .unique_keys
            .items
            .push(target.clone());
        progress.catalog.tables[index]
            .unique_keys
            .items
            .sort_by_key(sort_key_v1);
    }
    Ok(())
}

fn observe_keys(
    connection: &rusqlite::Connection,
    table: &mut TableSchemaV1,
    index: usize,
    origin: &str,
    progress: &mut Progress,
) -> Result<(), AcquisitionError> {
    let mut current: Option<KeyRows> = None;
    visit_rows(connection,
        "SELECT i.name AS index_name,i.\"unique\",i.origin,i.partial,x.seqno,x.cid,x.name AS column_name,x.desc,x.coll,x.key \
         FROM pragma_index_list(?1,'main') i JOIN pragma_index_xinfo(i.name,'main') x \
         WHERE i.origin=?2 ORDER BY i.name COLLATE BINARY,x.seqno",
        &[&table.name.clone(), &origin], progress, |row, progress| {
            let name: String = row.get(0).map_err(database)?;
            if current.as_ref().is_some_and(|key| key.name != name) {
                // The new group's first row has already arrived; retain it if admitting the
                // preceding group fails, and clear only the preceding group's admitted cells.
                let new_row = progress.pending.pop().ok_or(AcquisitionError::InvalidCapture)?;
                let previous_at = progress.at.clone();
                if let Some(last) = progress.pending.last() {
                    if let PhysicalCoordinateV1::SqlCatalog(at) = &last.at { progress.at = at.clone(); }
                }
                let result = admit_key(table, index, origin, current.take().ok_or(AcquisitionError::InvalidCapture)?, progress);
                if result.is_ok() { progress.pending.clear(); progress.at = previous_at; }
                progress.pending.push(new_row);
                result?;
            }
            if !row.get::<_, bool>(1).map_err(database)? || row.get::<_, String>(2).map_err(database)? != origin || row.get::<_, bool>(3).map_err(database)? { return invalid(); }
            let seqno: i64 = row.get(4).map_err(database)?;
            let key = current.get_or_insert_with(|| KeyRows { name, columns: Vec::new(), next_term: 0 });
            if seqno != key.next_term { return invalid(); }
            key.next_term += 1;
            if row.get::<_, bool>(9).map_err(database)? {
                if row.get::<_, i64>(5).map_err(database)? < 0 || row.get::<_, bool>(7).map_err(database)?
                    || row.get::<_, Option<String>>(8).map_err(database)?.as_deref() != Some("BINARY") { return invalid(); }
                key.columns.push(row.get::<_, Option<String>>(6).map_err(database)?.ok_or(AcquisitionError::InvalidCapture)?);
            }
            Ok(())
        })?;
    if let Some(key) = current {
        if let Some(last) = progress.pending.last() {
            if let PhysicalCoordinateV1::SqlCatalog(at) = &last.at {
                progress.at = at.clone();
            }
        }
        admit_key(table, index, origin, key, progress)?;
        progress.pending.clear();
    }
    if origin == "pk" {
        match &table.primary_key {
            PresenceV1::Present(key) if matches!(key.backing_index, PresenceV1::Missing) => {
                return invalid()
            }
            _ => {}
        }
        progress.catalog.tables[index].primary_key =
            ObservedValueV1::Complete(table.primary_key.clone());
    } else {
        if table
            .unique_keys
            .iter()
            .any(|key| matches!(key.backing_index, PresenceV1::Missing))
        {
            return invalid();
        }
        table.unique_keys.sort_by_key(sort_key_v1);
        progress.catalog.tables[index].unique_keys.terminal = EnumerationTerminalV1::Complete;
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Word,
    Identifier,
    Literal,
    Symbol,
}

struct Token {
    kind: Kind,
    text: String,
    start: usize,
    end: usize,
}

impl Token {
    fn word(&self, word: &str) -> bool {
        self.kind == Kind::Word && self.text.eq_ignore_ascii_case(word)
    }
    fn symbol(&self, symbol: &str) -> bool {
        self.kind == Kind::Symbol && self.text == symbol
    }
    fn identifier(&self) -> Result<String, AcquisitionError> {
        if matches!(self.kind, Kind::Word | Kind::Identifier) {
            Ok(self.text.clone())
        } else {
            invalid()
        }
    }
}

fn tokens(sql: &str) -> Result<Vec<Token>, AcquisitionError> {
    let raw = sql.as_bytes();
    let mut index = 0;
    let mut result = Vec::new();
    while index < raw.len() {
        if raw[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if raw[index..].starts_with(b"--") {
            while index < raw.len() && raw[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if raw[index..].starts_with(b"/*") {
            let Some(end) = sql[index + 2..].find("*/") else {
                return invalid();
            };
            index += end + 4;
            continue;
        }
        let start = index;
        let (kind, text) = match raw[index] {
            quote @ (b'\'' | b'"' | b'`' | b'[') => {
                let end_quote = if quote == b'[' { b']' } else { quote };
                let kind = if quote == b'\'' {
                    Kind::Literal
                } else {
                    Kind::Identifier
                };
                index += 1;
                let mut value = Vec::new();
                let mut closed = false;
                while index < raw.len() {
                    if raw[index] == end_quote {
                        index += 1;
                        if quote != b'[' && raw.get(index) == Some(&end_quote) {
                            value.push(end_quote);
                            index += 1;
                            continue;
                        }
                        closed = true;
                        break;
                    }
                    value.push(raw[index]);
                    index += 1;
                }
                if !closed {
                    return invalid();
                }
                (
                    kind,
                    String::from_utf8(value).map_err(|_| AcquisitionError::InvalidCapture)?,
                )
            }
            b'(' | b')' | b',' | b';' | b'.' => {
                index += 1;
                (Kind::Symbol, sql[start..index].into())
            }
            c if c.is_ascii_alphanumeric() || c == b'_' => {
                while index < raw.len()
                    && (raw[index].is_ascii_alphanumeric() || raw[index] == b'_')
                {
                    index += 1;
                }
                (Kind::Word, sql[start..index].into())
            }
            _ => return invalid(),
        };
        result.push(Token {
            kind,
            text,
            start,
            end: index,
        });
    }
    Ok(result)
}

fn declaration(name: &str, sql: &str) -> Result<TableSchemaV1, AcquisitionError> {
    let tokens = tokens(sql)?;
    let mut cursor = Cursor {
        tokens: &tokens,
        position: 0,
        sql,
    };
    cursor.word("CREATE")?;
    cursor.word("TABLE")?;
    if cursor.take_word("IF") {
        cursor.word("NOT")?;
        cursor.word("EXISTS")?;
    }
    let mut declared = cursor.next()?.identifier()?;
    if cursor.take_symbol(".") {
        if !declared.eq_ignore_ascii_case("main") {
            return invalid();
        }
        declared = cursor.next()?.identifier()?;
    }
    if declared != name {
        return invalid();
    }
    cursor.symbol("(")?;
    let mut table = TableSchemaV1 {
        name: name.into(),
        catalog_definition: PresenceV1::Present(bytes(sql)),
        columns: Vec::new(),
        primary_key: PresenceV1::Missing,
        unique_keys: Vec::new(),
        checks: Vec::new(),
    };
    loop {
        let start = cursor.position;
        let mut depth = 0;
        while let Some(token) = cursor.tokens.get(cursor.position) {
            if depth == 0 && (token.symbol(",") || token.symbol(")")) {
                break;
            }
            if token.symbol("(") {
                depth += 1;
            }
            if token.symbol(")") {
                depth -= 1;
            }
            cursor.position += 1;
        }
        if start == cursor.position || depth != 0 {
            return invalid();
        }
        item(&mut table, &tokens[start..cursor.position], sql)?;
        if cursor.take_symbol(",") {
            continue;
        }
        cursor.symbol(")")?;
        break;
    }
    cursor.take_symbol(";");
    if cursor.position != tokens.len() {
        return invalid();
    }
    Ok(table)
}

struct Cursor<'a> {
    tokens: &'a [Token],
    position: usize,
    sql: &'a str,
}

impl<'a> Cursor<'a> {
    fn next(&mut self) -> Result<&'a Token, AcquisitionError> {
        let token = self
            .tokens
            .get(self.position)
            .ok_or(AcquisitionError::InvalidCapture)?;
        self.position += 1;
        Ok(token)
    }
    fn take_word(&mut self, word: &str) -> bool {
        if self
            .tokens
            .get(self.position)
            .is_some_and(|token| token.word(word))
        {
            self.position += 1;
            true
        } else {
            false
        }
    }
    fn take_symbol(&mut self, symbol: &str) -> bool {
        if self
            .tokens
            .get(self.position)
            .is_some_and(|token| token.symbol(symbol))
        {
            self.position += 1;
            true
        } else {
            false
        }
    }
    fn word(&mut self, word: &str) -> Result<(), AcquisitionError> {
        if self.take_word(word) {
            Ok(())
        } else {
            invalid()
        }
    }
    fn symbol(&mut self, symbol: &str) -> Result<(), AcquisitionError> {
        if self.take_symbol(symbol) {
            Ok(())
        } else {
            invalid()
        }
    }
    fn columns(&mut self) -> Result<Vec<String>, AcquisitionError> {
        self.symbol("(")?;
        let mut columns = Vec::new();
        loop {
            columns.push(self.next()?.identifier()?);
            if !self.take_symbol(",") {
                break;
            }
        }
        self.symbol(")")?;
        Ok(columns)
    }
    fn definition(&self, start: usize) -> PresenceV1<HexBytesV1> {
        PresenceV1::Present(bytes(
            &self.sql[self.tokens[start].start..self.tokens[self.position - 1].end],
        ))
    }
}

#[allow(clippy::too_many_lines)] // One closed declaration keeps every admitted constraint visible.
fn item(table: &mut TableSchemaV1, tokens: &[Token], sql: &str) -> Result<(), AcquisitionError> {
    let mut cursor = Cursor {
        tokens,
        position: 0,
        sql,
    };
    let table_constraint = ["CONSTRAINT", "PRIMARY", "UNIQUE", "CHECK"]
        .iter()
        .any(|word| tokens[0].word(word));
    let mut column = if table_constraint {
        None
    } else {
        let name = cursor.next()?.identifier()?;
        let declared = cursor.next()?;
        let declared_name = declared.identifier()?;
        Some(ColumnSchemaV1 {
            ordinal: u16::try_from(table.columns.len())
                .map_err(|_| AcquisitionError::InvalidCapture)?,
            name,
            sql_type: if declared_name.eq_ignore_ascii_case("TEXT") {
                SqlTypeV1::Text
            } else if declared_name.eq_ignore_ascii_case("INTEGER") {
                SqlTypeV1::Integer
            } else {
                SqlTypeV1::Foreign(declared_name.clone())
            },
            declared_type: bytes(&declared_name),
            catalog_type_id: PresenceV1::Missing,
            not_null: false,
            default: PresenceV1::Missing,
            explicit_collation: PresenceV1::Missing,
        })
    };
    while cursor.position < tokens.len() {
        let start = cursor.position;
        let name = if cursor.take_word("CONSTRAINT") {
            PresenceV1::Present(cursor.next()?.identifier()?)
        } else {
            PresenceV1::Missing
        };
        if cursor.take_word("NOT") {
            cursor.word("NULL")?;
            // This wire format has no slot for a separately named NOT NULL declaration.
            if !matches!(name, PresenceV1::Missing) {
                return invalid();
            }
            column
                .as_mut()
                .ok_or(AcquisitionError::InvalidCapture)?
                .not_null = true;
        } else if cursor.take_word("PRIMARY") {
            cursor.word("KEY")?;
            if !matches!(table.primary_key, PresenceV1::Missing) {
                return invalid();
            }
            let columns = if let Some(column) = &column {
                vec![column.name.clone()]
            } else {
                cursor.columns()?
            };
            table.primary_key = PresenceV1::Present(KeySchemaV1 {
                name,
                backing_index: PresenceV1::Missing,
                columns,
                catalog_definition: cursor.definition(start),
            });
        } else if cursor.take_word("UNIQUE") {
            let columns = if let Some(column) = &column {
                vec![column.name.clone()]
            } else {
                cursor.columns()?
            };
            table.unique_keys.push(KeySchemaV1 {
                name,
                backing_index: PresenceV1::Missing,
                columns,
                catalog_definition: cursor.definition(start),
            });
        } else if cursor.take_word("CHECK") {
            cursor.symbol("(")?;
            let begin = cursor.position;
            let mut depth = 1;
            while depth > 0 {
                let token = cursor.next()?;
                if token.symbol("(") {
                    depth += 1;
                }
                if token.symbol(")") {
                    depth -= 1;
                }
            }
            let expression = &tokens[begin..cursor.position - 1];
            if expression.is_empty() {
                return invalid();
            }
            let raw = bytes(&sql[expression[0].start..expression[expression.len() - 1].end]);
            let definition = cursor.definition(start);
            let known = table.name == "history" && history_check(expression);
            table.checks.push(if known {
                CheckSchemaV1::HistoryKindDecisionObservation(HistoryCheckSchemaV1 {
                    name,
                    catalog_expression: raw,
                    catalog_definition: definition,
                })
            } else {
                CheckSchemaV1::Foreign(ForeignCheckSchemaV1 {
                    name,
                    catalog_expression: raw,
                    catalog_definition: definition,
                })
            });
        } else {
            return invalid();
        }
        if table_constraint && cursor.position != tokens.len() {
            return invalid();
        }
    }
    if let Some(column) = column {
        table.columns.push(column);
    }
    Ok(())
}

fn history_check(tokens: &[Token]) -> bool {
    tokens.len() == 7
        && tokens[0].identifier().is_ok_and(|name| name == "kind")
        && tokens[1].word("IN")
        && tokens[2].symbol("(")
        && tokens[3].kind == Kind::Literal
        && tokens[3].text == "decision"
        && tokens[4].symbol(",")
        && tokens[5].kind == Kind::Literal
        && tokens[5].text == "observation"
        && tokens[6].symbol(")")
}

#[cfg(test)]
mod tests {
    use super::*;

    const DDL: &str = "
        CREATE TABLE instances(entity TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,
            document TEXT NOT NULL,PRIMARY KEY(entity,id));
        CREATE TABLE events(entity TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,
            position INTEGER NOT NULL,document TEXT NOT NULL,PRIMARY KEY(entity,id,revision,position));
        CREATE TABLE history(entity TEXT NOT NULL,id TEXT NOT NULL,position INTEGER NOT NULL,
            kind TEXT NOT NULL CHECK(kind IN ('decision','observation')),record_id TEXT NOT NULL UNIQUE,
            document TEXT NOT NULL,PRIMARY KEY(entity,id,position));
        CREATE TABLE legacy_origins(entity TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,
            PRIMARY KEY(entity,id));
    ";

    #[test]
    fn inconsistent_column_catalog_retains_prefix_and_rejected_pragma_cells() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection.execute_batch(DDL).unwrap();
        // The disposable connection retains its loaded schema while its catalog DDL changes.
        connection.execute_batch("PRAGMA writable_schema=ON; UPDATE sqlite_schema SET sql=replace(sql,'revision INTEGER','revision TEXT') WHERE name='events'").unwrap();
        let failure =
            read_retained(&connection).expect_err("contradictory column metadata refuses");
        let table = &failure.catalog.tables[0];
        assert_eq!(table.name, "events");
        assert_eq!(table.columns.items.len(), 2, "earlier columns retained");
        assert!(matches!(
            table.columns.terminal,
            EnumerationTerminalV1::Refused(_)
        ));
        assert!(matches!(table.primary_key, ObservedValueV1::NotAttempted));
        assert_eq!(failure.catalog.rejected_rows.len(), 1);
        let row = &failure.catalog.rejected_rows[0];
        assert_eq!(
            row.at,
            PhysicalCoordinateV1::SqlCatalog(SqlCatalogCoordinateV1 {
                namespace: PresenceV1::Present("main".into()),
                family: CatalogFamilyV1::Columns,
                table: PresenceV1::Present("events".into()),
                row: PresenceV1::Present(2),
            })
        );
        assert!(row
            .cells
            .iter()
            .any(|cell| cell.column == "type"
                && cell.value == SqlCellValueV1::Text(bytes("INTEGER"))));
        assert!(failure.refusals.iter().any(|failure| failure.at == row.at));
    }

    #[test]
    fn unique_key_failure_keeps_primary_key_and_all_unadmitted_index_terms() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection.execute_batch(DDL).unwrap();
        connection.execute_batch("PRAGMA writable_schema=ON; UPDATE sqlite_schema SET sql=replace(sql,'record_id TEXT NOT NULL UNIQUE','record_id TEXT NOT NULL') WHERE name='history'").unwrap();
        let failure = read_retained(&connection).expect_err("unaccounted unique index refuses");
        let table = failure
            .catalog
            .tables
            .iter()
            .find(|table| table.name == "history")
            .unwrap();
        assert!(matches!(
            table.primary_key,
            ObservedValueV1::Complete(PresenceV1::Present(_))
        ));
        assert!(matches!(
            table.unique_keys.terminal,
            EnumerationTerminalV1::Refused(_)
        ));
        assert!(matches!(
            table.checks.terminal,
            EnumerationTerminalV1::NotAttempted
        ));
        assert!(!failure.catalog.rejected_rows.is_empty());
        assert!(failure.catalog.rejected_rows.iter().any(|row| row
            .cells
            .iter()
            .any(|cell| cell.column == "column_name"
                && cell.value == SqlCellValueV1::Text(bytes("record_id")))));
        assert!(failure
            .catalog
            .rejected_rows
            .iter()
            .all(|row| failure.refusals.iter().any(|failure| failure.at == row.at)));
    }

    #[test]
    fn sqlite_catalog_retains_observed_declarations_and_backing_indexes() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection.execute_batch(DDL).unwrap();
        let observed = read(&connection).expect("actual provider catalog must be admitted");
        assert!(observed.is_known_provider_schema());
        let history = observed
            .tables
            .iter()
            .find(|table| table.name == "history")
            .unwrap();
        assert!(
            matches!(&history.catalog_definition, PresenceV1::Present(value) if std::str::from_utf8(value.as_bytes()).unwrap().starts_with("CREATE TABLE history"))
        );
        assert_eq!(history.columns[2].declared_type, bytes("INTEGER"));
        assert!(
            matches!(&history.unique_keys[0].backing_index, PresenceV1::Present(name) if name.starts_with("sqlite_autoindex_history_"))
        );
        assert!(
            matches!(&history.checks[0], CheckSchemaV1::HistoryKindDecisionObservation(value) if value.catalog_expression == bytes("kind IN ('decision','observation')"))
        );
    }

    #[test]
    fn sqlite_capture_refuses_matching_names_with_changed_schema_predicates() {
        for (old, new) in [
            ("revision INTEGER NOT NULL", "revision TEXT NOT NULL"),
            ("document TEXT NOT NULL", "document TEXT"),
            (
                "revision INTEGER NOT NULL",
                "revision INTEGER NOT NULL DEFAULT 0",
            ),
            ("PRIMARY KEY(entity,id));", "UNIQUE(entity,id));"),
            ("record_id TEXT NOT NULL UNIQUE", "record_id TEXT NOT NULL"),
            (" CHECK(kind IN ('decision','observation'))", ""),
            ("'decision','observation'", "'de cision','observation'"),
            (
                "'decision','observation'",
                "'decision','observation','other'",
            ),
            (
                "entity TEXT NOT NULL",
                "entity TEXT COLLATE NOCASE NOT NULL",
            ),
        ] {
            let connection = rusqlite::Connection::open_in_memory().unwrap();
            connection
                .execute_batch(&DDL.replace(old, new))
                .expect("malformed schema is readable SQLite");
            assert!(
                matches!(read(&connection), Err(AcquisitionError::InvalidCapture)),
                "accepted mutation {old} -> {new}"
            );
        }
        for extra in [
            "CREATE INDEX extra ON instances(document)",
            "CREATE TABLE sqliteXforeign(value TEXT)",
            "CREATE VIEW extra AS SELECT * FROM instances",
            "CREATE TRIGGER extra AFTER INSERT ON instances BEGIN SELECT 1; END",
        ] {
            let connection = rusqlite::Connection::open_in_memory().unwrap();
            connection.execute_batch(DDL).unwrap();
            connection.execute_batch(extra).unwrap();
            assert!(
                matches!(read(&connection), Err(AcquisitionError::InvalidCapture)),
                "accepted extra object {extra}"
            );
        }
    }

    #[test]
    fn catalog_names_quotes_and_comments_do_not_change_constraint_meaning() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        let ddl = DDL
            .replace("CHECK(kind IN ('decision','observation'))", "CONSTRAINT \"history_kind\" CHECK(\"kind\" /* ignored */ IN ('decision','observation'))")
            .replace("record_id TEXT NOT NULL UNIQUE", "record_id TEXT NOT NULL CONSTRAINT record_once UNIQUE")
            .replace("PRIMARY KEY(entity,id,position)", "CONSTRAINT history_identity PRIMARY KEY(entity,id,position)");
        connection.execute_batch(&ddl).unwrap();
        let observed = read(&connection).expect("equivalent named declarations are admitted");
        let history = observed
            .tables
            .iter()
            .find(|table| table.name == "history")
            .unwrap();
        assert_eq!(
            history.unique_keys[0].name,
            PresenceV1::Present("record_once".into())
        );
        assert!(
            matches!(&history.primary_key, PresenceV1::Present(key) if key.name == PresenceV1::Present("history_identity".into()))
        );
        assert!(
            matches!(&history.checks[0], CheckSchemaV1::HistoryKindDecisionObservation(value) if value.name == PresenceV1::Present("history_kind".into()) && std::str::from_utf8(value.catalog_expression.as_bytes()).unwrap().contains("/* ignored */"))
        );
    }
}
