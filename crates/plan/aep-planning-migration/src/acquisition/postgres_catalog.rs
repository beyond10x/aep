//! Observe the legacy PostgreSQL catalog in the capture contract's fixed read order.

use super::retained_failure::{unattempted, CatalogFailure, CatalogProgress as Progress};
use super::{postgres_cells, AcquisitionError};
#[allow(clippy::wildcard_imports)]
use aep_contract::migration::*;
use postgres::fallible_iterator::FallibleIterator as _;
use postgres::types::{FromSqlOwned, ToSql};

// `map_err` hands this adapter its error by value; retain only its display text.
#[allow(clippy::needless_pass_by_value)]
fn database(error: postgres::Error) -> AcquisitionError {
    AcquisitionError::Postgres(error.to_string())
}

fn bytes(value: String) -> HexBytesV1 {
    HexBytesV1::new(value.into_bytes())
}

fn optional_bytes(value: Option<String>) -> PresenceV1<HexBytesV1> {
    value.map_or(PresenceV1::Missing, |value| {
        PresenceV1::Present(bytes(value))
    })
}

fn get<T: FromSqlOwned>(row: &postgres::Row, column: usize) -> Result<T, AcquisitionError> {
    row.try_get(column).map_err(database)
}

fn visit_rows<F>(
    transaction: &mut postgres::Transaction<'_>,
    query: &str,
    params: &[&(dyn ToSql + Sync)],
    progress: &mut Progress,
    mut visit: F,
) -> Result<(), AcquisitionError>
where
    F: FnMut(&postgres::Row, &mut Progress) -> Result<(), AcquisitionError>,
{
    let mut rows = transaction
        .query_raw(query, params.iter().copied())
        .map_err(database)?;
    let mut ordinal = 0_u64;
    loop {
        progress.at.row = PresenceV1::Missing;
        let Some(row) = rows.next().map_err(database)? else {
            return Ok(());
        };
        progress.at.row = PresenceV1::Present(ordinal);
        progress.pending.push(RejectedCatalogRowV1 {
            at: PhysicalCoordinateV1::SqlCatalog(progress.at.clone()),
            cells: postgres_cells::cells(&row)?,
        });
        visit(&row, progress)?;
        ordinal += 1;
    }
}

#[cfg(test)]
fn read(
    transaction: &mut postgres::Transaction<'_>,
    namespace: &str,
) -> Result<SqlSchemaV1, AcquisitionError> {
    read_retained(transaction, namespace).map_err(|_| AcquisitionError::InvalidCapture)
}

pub(super) fn read_retained(
    transaction: &mut postgres::Transaction<'_>,
    namespace: &str,
) -> Result<SqlSchemaV1, Box<CatalogFailure>> {
    let mut progress = Progress {
        catalog: SqlCatalogEvidenceV1 {
            namespace: PresenceV1::Present(namespace.to_owned()),
            objects: unattempted(),
            tables: Vec::new(),
            indexes: unattempted(),
            foreign_objects: unattempted(),
            rejected_rows: Vec::new(),
        },
        at: SqlCatalogCoordinateV1 {
            namespace: PresenceV1::Present(namespace.to_owned()),
            family: CatalogFamilyV1::Objects,
            table: PresenceV1::Missing,
            row: PresenceV1::Missing,
        },
        pending: Vec::new(),
    };
    read_inner(transaction, namespace, &mut progress).map_err(|error| progress.refused(&error))
}

fn object_kind(kind: &str) -> ForeignObjectKindV1 {
    match kind {
        "table" => ForeignObjectKindV1::Table,
        "index" => ForeignObjectKindV1::Index,
        "view" => ForeignObjectKindV1::View,
        "trigger" => ForeignObjectKindV1::Trigger,
        "constraint" => ForeignObjectKindV1::Constraint,
        other => ForeignObjectKindV1::Other(other.to_owned()),
    }
}

// Backing indexes belong to their keys. Everything else in this namespace is discovered before
// any table family, including objects that the provider's closed schema will refuse.
const OBJECTS: &str = r#"
SELECT kind,name,parent FROM (
 SELECT CASE c.relkind WHEN 'r' THEN 'table' WHEN 'p' THEN 'table'
          WHEN 'i' THEN 'index' WHEN 'I' THEN 'index' WHEN 'v' THEN 'view'
          ELSE c.relkind::text END AS kind,
        c.relname::text AS name, t.relname::text AS parent
 FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace
 LEFT JOIN pg_catalog.pg_index x ON x.indexrelid=c.oid
 LEFT JOIN pg_catalog.pg_class t ON t.oid=x.indrelid
 WHERE n.nspname=$1 AND NOT EXISTS(
   SELECT 1 FROM pg_catalog.pg_constraint k WHERE k.conindid=c.oid AND k.contype IN ('p','u'))
 UNION ALL
 SELECT 'constraint',k.conname::text,t.relname::text FROM pg_catalog.pg_constraint k
 JOIN pg_catalog.pg_class t ON t.oid=k.conrelid
 JOIN pg_catalog.pg_namespace n ON n.oid=t.relnamespace
 WHERE n.nspname=$1 AND k.contype NOT IN ('p','u','c')
 UNION ALL
 SELECT 'trigger',g.tgname::text,t.relname::text FROM pg_catalog.pg_trigger g
 JOIN pg_catalog.pg_class t ON t.oid=g.tgrelid
 JOIN pg_catalog.pg_namespace n ON n.oid=t.relnamespace
 WHERE n.nspname=$1 AND NOT g.tgisinternal
 UNION ALL
 SELECT 'rule',r.rulename::text,t.relname::text FROM pg_catalog.pg_rewrite r
 JOIN pg_catalog.pg_class t ON t.oid=r.ev_class
 JOIN pg_catalog.pg_namespace n ON n.oid=t.relnamespace
 WHERE n.nspname=$1
) objects ORDER BY kind COLLATE "C",name COLLATE "C",parent COLLATE "C"
"#;

// The catalog families must be observed in this fixed order for retained-prefix evidence.
#[allow(clippy::too_many_lines)]
fn read_inner(
    transaction: &mut postgres::Transaction<'_>,
    namespace: &str,
    progress: &mut Progress,
) -> Result<SqlSchemaV1, AcquisitionError> {
    visit_rows(
        transaction,
        OBJECTS,
        &[&namespace],
        progress,
        |row, progress| {
            let kind: String = get(row, 0)?;
            let name: String = get(row, 1)?;
            let parent: Option<String> = get(row, 2)?;
            progress.catalog.objects.items.push(CatalogObjectHeaderV1 {
                kind: object_kind(&kind),
                name: name.clone(),
                parent: parent.map_or(PresenceV1::Missing, PresenceV1::Present),
            });
            progress.catalog.objects.items.sort_by_key(|header| {
                let tag = if matches!(header.kind, ForeignObjectKindV1::Other(_)) {
                    "other"
                } else {
                    super::catalog_kind(&header.kind)
                };
                (
                    tag.to_owned(),
                    header.name.clone(),
                    sort_key_v1(&header.parent),
                )
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
    progress
        .catalog
        .tables
        .sort_by(|left, right| left.name.cmp(&right.name));
    let mut tables = Vec::new();
    for index in 0..progress.catalog.tables.len() {
        let name = progress.catalog.tables[index].name.clone();
        progress.next(
            CatalogFamilyV1::TableDefinition,
            PresenceV1::Present(name.clone()),
        );
        let mut oid = None;
        visit_rows(
            transaction,
            r"
            SELECT c.oid,c.relkind::text AS relkind,c.relrowsecurity,c.relforcerowsecurity,c.relispartition,
                   c.relpersistence::text AS relpersistence,c.reloptions IS NULL AS options_absent,
                   EXISTS(SELECT 1 FROM pg_catalog.pg_inherits WHERE inhrelid=c.oid OR inhparent=c.oid) AS has_inheritance,
                   EXISTS(SELECT 1 FROM pg_catalog.pg_attribute WHERE attrelid=c.oid AND attisdropped) AS has_dropped_attribute
            FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace
            WHERE n.nspname=$1 AND c.relname=$2
        ",
            &[&namespace, &name],
            progress,
            |row, progress| {
                if oid.is_some()
                    || get::<String>(row, 1)? != "r"
                    || get::<bool>(row, 2)?
                    || get::<bool>(row, 3)?
                    || get::<bool>(row, 4)?
                    || get::<String>(row, 5)? != "p"
                    || !get::<bool>(row, 6)?
                    || get::<bool>(row, 7)?
                    || get::<bool>(row, 8)?
                {
                    return Err(AcquisitionError::InvalidCapture);
                }
                oid = Some(get::<u32>(row, 0)?);
                // PostgreSQL has no CREATE TABLE text. The actual relation was read; its column,
                // key and check definitions follow separately, rather than a manufactured DDL.
                progress.catalog.tables[index].catalog_definition =
                    ObservedValueV1::Complete(PresenceV1::Missing);
                progress.pending.clear();
                Ok(())
            },
        )?;
        let oid = oid.ok_or(AcquisitionError::InvalidCapture)?;
        progress.next(CatalogFamilyV1::Columns, PresenceV1::Present(name.clone()));
        columns(transaction, oid, index, progress)?;
        progress.next(
            CatalogFamilyV1::PrimaryKey,
            PresenceV1::Present(name.clone()),
        );
        keys(transaction, oid, index, "p", progress)?;
        progress.next(
            CatalogFamilyV1::UniqueKeys,
            PresenceV1::Present(name.clone()),
        );
        keys(transaction, oid, index, "u", progress)?;
        progress.next(CatalogFamilyV1::Checks, PresenceV1::Present(name.clone()));
        checks(transaction, oid, index, progress)?;
        let table = &progress.catalog.tables[index];
        let ObservedValueV1::Complete(primary_key) = &table.primary_key else {
            return Err(AcquisitionError::InvalidCapture);
        };
        tables.push(TableSchemaV1 {
            name,
            catalog_definition: PresenceV1::Missing,
            columns: table.columns.items.clone(),
            primary_key: primary_key.clone(),
            unique_keys: table.unique_keys.items.clone(),
            checks: table.checks.items.clone(),
        });
    }
    progress.next(CatalogFamilyV1::Indexes, PresenceV1::Missing);
    indexes(transaction, namespace, progress)?;
    progress.next(CatalogFamilyV1::ForeignObjects, PresenceV1::Missing);
    foreign_objects(transaction, namespace, progress)?;
    let schema = SqlSchemaV1 {
        dialect: SqlDialectV1::Postgres,
        namespace: namespace.to_owned(),
        tables,
        indexes: progress.catalog.indexes.items.clone(),
        foreign_objects: progress.catalog.foreign_objects.items.clone(),
    };
    if !schema.is_known_provider_schema() {
        progress.next(CatalogFamilyV1::Objects, PresenceV1::Missing);
        return Err(AcquisitionError::InvalidCapture);
    }
    Ok(schema)
}

fn columns(
    transaction: &mut postgres::Transaction<'_>,
    oid: u32,
    index: usize,
    progress: &mut Progress,
) -> Result<(), AcquisitionError> {
    visit_rows(
        transaction,
        r"
        SELECT a.attnum,a.attname,a.atttypid,pg_catalog.format_type(a.atttypid,a.atttypmod) AS declared_type,
               a.attnotnull,pg_catalog.pg_get_expr(d.adbin,d.adrelid) AS default_expression,
               CASE WHEN a.attcollation<>t.typcollation THEN nc.nspname || '.' || c.collname END AS explicit_collation,
               a.attisdropped,a.attidentity::text AS identity_kind,a.attgenerated::text AS generated_kind
        FROM pg_catalog.pg_attribute a JOIN pg_catalog.pg_type t ON t.oid=a.atttypid
        LEFT JOIN pg_catalog.pg_attrdef d ON d.adrelid=a.attrelid AND d.adnum=a.attnum
        LEFT JOIN pg_catalog.pg_collation c ON c.oid=a.attcollation
        LEFT JOIN pg_catalog.pg_namespace nc ON nc.oid=c.collnamespace
        WHERE a.attrelid=$1 AND a.attnum>0 ORDER BY a.attnum
    ",
        &[&oid],
        progress,
        |row, progress| {
            let ordinal: i16 = get(row, 0)?;
            let type_id: u32 = get(row, 2)?;
            let declared: String = get(row, 3)?;
            if get::<bool>(row, 7)?
                || !get::<String>(row, 8)?.is_empty()
                || !get::<String>(row, 9)?.is_empty()
            {
                return Err(AcquisitionError::InvalidCapture);
            }
            progress.catalog.tables[index]
                .columns
                .items
                .push(ColumnSchemaV1 {
                    ordinal: u16::try_from(
                        ordinal
                            .checked_sub(1)
                            .ok_or(AcquisitionError::InvalidCapture)?,
                    )
                    .map_err(|_| AcquisitionError::InvalidCapture)?,
                    name: get(row, 1)?,
                    sql_type: match type_id {
                        25 if declared == "text" => SqlTypeV1::Text,
                        20 if declared == "bigint" => SqlTypeV1::BigInt,
                        _ => SqlTypeV1::Foreign(declared.clone()),
                    },
                    declared_type: bytes(declared),
                    catalog_type_id: PresenceV1::Present(type_id.to_string()),
                    not_null: get(row, 4)?,
                    default: optional_bytes(get(row, 5)?),
                    explicit_collation: get::<Option<String>>(row, 6)?
                        .map_or(PresenceV1::Missing, PresenceV1::Present),
                });
            progress.pending.clear();
            Ok(())
        },
    )?;
    progress.catalog.tables[index].columns.terminal = EnumerationTerminalV1::Complete;
    Ok(())
}

fn keys(
    transaction: &mut postgres::Transaction<'_>,
    oid: u32,
    index: usize,
    kind: &str,
    progress: &mut Progress,
) -> Result<(), AcquisitionError> {
    visit_rows(
        transaction,
        r#"
        SELECT c.conname,c.condeferrable,c.condeferred,c.convalidated,
               COALESCE((to_jsonb(c)->>'conenforced')::boolean,true) AS conenforced,
               pg_catalog.pg_get_constraintdef(c.oid) AS constraint_definition,i.relname AS index_name,
               ARRAY(SELECT a.attname::text FROM unnest(c.conkey) WITH ORDINALITY k(num,ord)
                   JOIN pg_catalog.pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num ORDER BY k.ord) AS key_columns,
               x.indisvalid AND x.indisready AND x.indislive AND x.indisunique AND x.indimmediate AS index_ready,
               am.amname,x.indexprs IS NULL AND x.indpred IS NULL AS expression_free,x.indnkeyatts=x.indnatts AS all_attrs_key,
               ARRAY(SELECT no.nspname || '.' || o.opcname FROM unnest(x.indclass) WITH ORDINALITY k(oid,ord)
                   JOIN pg_catalog.pg_opclass o ON o.oid=k.oid
                   JOIN pg_catalog.pg_namespace no ON no.oid=o.opcnamespace ORDER BY k.ord) AS opclasses,
               COALESCE((to_jsonb(x)->>'indnullsnotdistinct')::boolean,false) AS nulls_not_distinct,
               NOT EXISTS(SELECT 1 FROM unnest(x.indkey::smallint[],x.indcollation::oid[]) k(num,coll)
                   LEFT JOIN pg_catalog.pg_attribute a ON a.attrelid=x.indrelid AND a.attnum=k.num
                   WHERE a.attnum IS NULL OR a.attcollation<>k.coll) AS collations_match,
               x.indisprimary, x.indisexclusion
        FROM pg_catalog.pg_constraint c
        LEFT JOIN pg_catalog.pg_class i ON i.oid=c.conindid
        LEFT JOIN pg_catalog.pg_index x ON x.indexrelid=c.conindid
        LEFT JOIN pg_catalog.pg_am am ON am.oid=i.relam
        WHERE c.conrelid=$1 AND c.contype::text=$2 ORDER BY c.conname COLLATE "C"
    "#,
        &[&oid, &kind],
        progress,
        |row, progress| {
            if get::<bool>(row, 1)?
                || get::<bool>(row, 2)?
                || !get::<bool>(row, 3)?
                || !get::<bool>(row, 4)?
                || !get::<bool>(row, 8)?
                || get::<String>(row, 9)? != "btree"
                || !get::<bool>(row, 10)?
                || !get::<bool>(row, 11)?
                || get::<bool>(row, 13)?
                || !get::<bool>(row, 14)?
                || get::<bool>(row, 15)? != (kind == "p")
                || get::<bool>(row, 16)?
                || get::<Vec<String>>(row, 12)?.iter().any(|value| {
                    !matches!(
                        value.as_str(),
                        "pg_catalog.text_ops" | "pg_catalog.int8_ops"
                    )
                })
            {
                return Err(AcquisitionError::InvalidCapture);
            }
            let key = KeySchemaV1 {
                name: PresenceV1::Present(get(row, 0)?),
                backing_index: PresenceV1::Present(get(row, 6)?),
                columns: get(row, 7)?,
                catalog_definition: PresenceV1::Present(bytes(get(row, 5)?)),
            };
            let table = &mut progress.catalog.tables[index];
            if kind == "p" {
                if !matches!(table.primary_key, ObservedValueV1::NotAttempted) {
                    return Err(AcquisitionError::InvalidCapture);
                }
                table.primary_key = ObservedValueV1::Complete(PresenceV1::Present(key));
            } else {
                table.unique_keys.items.push(key);
                table.unique_keys.items.sort_by_key(sort_key_v1);
            }
            progress.pending.clear();
            Ok(())
        },
    )?;
    let table = &mut progress.catalog.tables[index];
    if kind == "p" {
        if matches!(table.primary_key, ObservedValueV1::NotAttempted) {
            table.primary_key = ObservedValueV1::Complete(PresenceV1::Missing);
        }
    } else {
        table.unique_keys.terminal = EnumerationTerminalV1::Complete;
    }
    Ok(())
}

fn checks(
    transaction: &mut postgres::Transaction<'_>,
    oid: u32,
    index: usize,
    progress: &mut Progress,
) -> Result<(), AcquisitionError> {
    visit_rows(
        transaction,
        r#"
        SELECT c.conname,c.condeferrable,c.condeferred,c.convalidated,
               COALESCE((to_jsonb(c)->>'conenforced')::boolean,true) AS conenforced,c.connoinherit,
               pg_catalog.pg_get_constraintdef(c.oid) AS constraint_definition,
               pg_catalog.pg_get_expr(c.conbin,c.conrelid) AS check_expression
        FROM pg_catalog.pg_constraint c
        WHERE c.conrelid=$1 AND c.contype='c' ORDER BY c.conname COLLATE "C"
    "#,
        &[&oid],
        progress,
        |row, progress| {
            if get::<bool>(row, 1)?
                || get::<bool>(row, 2)?
                || !get::<bool>(row, 3)?
                || !get::<bool>(row, 4)?
                || get::<bool>(row, 5)?
            {
                return Err(AcquisitionError::InvalidCapture);
            }
            let table = &mut progress.catalog.tables[index];
            table.checks.items.push(check(
                &table.name,
                PresenceV1::Present(get(row, 0)?),
                get(row, 7)?,
                PresenceV1::Present(bytes(get(row, 6)?)),
            ));
            table.checks.items.sort_by_key(sort_key_v1);
            progress.pending.clear();
            Ok(())
        },
    )?;
    progress.catalog.tables[index].checks.terminal = EnumerationTerminalV1::Complete;
    Ok(())
}

fn indexes(
    transaction: &mut postgres::Transaction<'_>,
    namespace: &str,
    progress: &mut Progress,
) -> Result<(), AcquisitionError> {
    visit_rows(
        transaction,
        r#"
        SELECT i.relname AS index_name,t.relname AS table_name,x.indisunique,am.amname,
               pg_catalog.pg_get_expr(x.indexprs,x.indrelid) AS index_expression,
               pg_catalog.pg_get_expr(x.indpred,x.indrelid) AS index_predicate,
               pg_catalog.pg_get_indexdef(i.oid) AS index_definition,
               x.indisvalid AND x.indisready AND x.indislive AS index_ready,x.indnkeyatts,x.indnatts,
               ARRAY(SELECT no.nspname || '.' || o.opcname FROM unnest(x.indclass) WITH ORDINALITY k(oid,ord)
                   JOIN pg_catalog.pg_opclass o ON o.oid=k.oid
                   JOIN pg_catalog.pg_namespace no ON no.oid=o.opcnamespace ORDER BY k.ord) AS opclasses,
               COALESCE((to_jsonb(x)->>'indnullsnotdistinct')::boolean,false) AS nulls_not_distinct
        FROM pg_catalog.pg_index x JOIN pg_catalog.pg_class i ON i.oid=x.indexrelid
        JOIN pg_catalog.pg_class t ON t.oid=x.indrelid
        JOIN pg_catalog.pg_namespace n ON n.oid=i.relnamespace
        JOIN pg_catalog.pg_am am ON am.oid=i.relam
        WHERE n.nspname=$1 AND NOT EXISTS(
            SELECT 1 FROM pg_catalog.pg_constraint c WHERE c.conindid=i.oid AND c.contype IN ('p','u'))
        ORDER BY i.relname COLLATE "C"
    "#,
        &[&namespace],
        progress,
        |row, progress| {
            let method: String = get(row, 3)?;
            let expression: Option<String> = get(row, 4)?;
            let opclasses: Vec<String> = get(row, 10)?;
            // The only independent index in the admitted legacy PostgreSQL profile is this GIN
            // expression. Keep other rows intact as rejected cells rather than inventing terms.
            let Some(expression) = expression else {
                return Err(AcquisitionError::InvalidCapture);
            };
            if !get::<bool>(row, 7)?
                || get::<bool>(row, 11)?
                || method != "gin"
                || get::<i16>(row, 8)? != 1
                || get::<i16>(row, 9)? != 1
                || opclasses != ["pg_catalog.jsonb_path_ops"]
                || !matches!(
                    compact_expression(&expression).as_str(),
                    "(document)::jsonb" | "document::jsonb"
                )
            {
                return Err(AcquisitionError::InvalidCapture);
            }
            progress.catalog.indexes.items.push(IndexSchemaV1 {
                name: get(row, 0)?,
                table: get(row, 1)?,
                unique: get(row, 2)?,
                method: IndexMethodV1::Gin,
                terms: vec![IndexTermV1::DocumentJsonbPathOps(DocumentJsonbPathOpsV1 {
                    catalog_expression: bytes(expression),
                    opclass: "jsonb_path_ops".into(),
                })],
                predicate: optional_bytes(get(row, 5)?),
                catalog_definition: bytes(get(row, 6)?),
            });
            progress.pending.clear();
            Ok(())
        },
    )?;
    progress.catalog.indexes.terminal = EnumerationTerminalV1::Complete;
    Ok(())
}

fn foreign_objects(
    transaction: &mut postgres::Transaction<'_>,
    namespace: &str,
    progress: &mut Progress,
) -> Result<(), AcquisitionError> {
    visit_rows(
        transaction,
        r#"
        SELECT kind,name,parent,definition FROM (
          SELECT CASE c.relkind WHEN 'v' THEN 'view' ELSE c.relkind::text END AS kind,
                 c.relname::text AS name,NULL::text AS parent,
                 CASE WHEN c.relkind IN ('v','m') THEN pg_catalog.pg_get_viewdef(c.oid) END AS definition
          FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace
          WHERE n.nspname=$1 AND c.relkind NOT IN ('r','p','i','I')
          UNION ALL
          SELECT 'constraint',c.conname::text,t.relname::text,pg_catalog.pg_get_constraintdef(c.oid)
          FROM pg_catalog.pg_constraint c JOIN pg_catalog.pg_class t ON t.oid=c.conrelid
          JOIN pg_catalog.pg_namespace n ON n.oid=t.relnamespace
          WHERE n.nspname=$1 AND c.contype NOT IN ('p','u','c')
          UNION ALL
          SELECT 'trigger',g.tgname::text,t.relname::text,pg_catalog.pg_get_triggerdef(g.oid)
          FROM pg_catalog.pg_trigger g JOIN pg_catalog.pg_class t ON t.oid=g.tgrelid
          JOIN pg_catalog.pg_namespace n ON n.oid=t.relnamespace WHERE n.nspname=$1 AND NOT g.tgisinternal
          UNION ALL
          SELECT 'rule',r.rulename::text,t.relname::text,pg_catalog.pg_get_ruledef(r.oid)
          FROM pg_catalog.pg_rewrite r JOIN pg_catalog.pg_class t ON t.oid=r.ev_class
          JOIN pg_catalog.pg_namespace n ON n.oid=t.relnamespace WHERE n.nspname=$1
        ) objects ORDER BY kind COLLATE "C",name COLLATE "C",parent COLLATE "C"
    "#,
        &[&namespace],
        progress,
        |row, progress| {
            let kind: String = get(row, 0)?;
            progress
                .catalog
                .foreign_objects
                .items
                .push(ForeignObjectV1 {
                    kind: object_kind(&kind),
                    name: get(row, 1)?,
                    owner: get::<Option<String>>(row, 2)?
                        .map_or(PresenceV1::Missing, PresenceV1::Present),
                    catalog_definition: optional_bytes(get(row, 3)?),
                });
            progress
                .catalog
                .foreign_objects
                .items
                .sort_by_key(sort_key_v1);
            progress.pending.clear();
            Ok(())
        },
    )?;
    progress.catalog.foreign_objects.terminal = EnumerationTerminalV1::Complete;
    Ok(())
}

fn check(
    table: &str,
    name: PresenceV1<String>,
    expression: String,
    catalog_definition: PresenceV1<HexBytesV1>,
) -> CheckSchemaV1 {
    // Recognize only the provider predicates deparsed by PostgreSQL. Preserve the original text;
    // a different predicate remains Foreign and the closed capture admission refuses it.
    let compact = compact_expression(&expression);
    if table == "history" && compact == "(kind=ANY(ARRAY['decision'::text,'observation'::text]))" {
        CheckSchemaV1::HistoryKindDecisionObservation(HistoryCheckSchemaV1 {
            name,
            catalog_expression: bytes(expression),
            catalog_definition,
        })
    } else if table == "provider_sequences" && compact == "(next_value>=0)" {
        CheckSchemaV1::NextValueNonnegative(NextValueCheckSchemaV1 {
            name,
            catalog_expression: bytes(expression),
            catalog_definition,
        })
    } else {
        CheckSchemaV1::Foreign(ForeignCheckSchemaV1 {
            name,
            catalog_expression: bytes(expression),
            catalog_definition,
        })
    }
}

fn compact_expression(expression: &str) -> String {
    let mut quote = None;
    expression
        .chars()
        .filter(|character| {
            if matches!(character, '\'' | '"') {
                if quote == Some(*character) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(*character);
                }
            }
            quote.is_some() || !character.is_ascii_whitespace()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predicate_recognition_preserves_literal_contents_and_catalog_bytes() {
        let expression = "(kind = ANY (ARRAY['decision'::text, 'observation'::text]))";
        let name = PresenceV1::Present("history_kind_check".into());
        let definition = PresenceV1::Present(bytes(format!("CHECK {expression}")));
        let observed = check(
            "history",
            name.clone(),
            expression.into(),
            definition.clone(),
        );
        let CheckSchemaV1::HistoryKindDecisionObservation(observed) = observed else {
            panic!("the actual provider predicate must be recognized");
        };
        assert_eq!(observed.name, name);
        assert_eq!(observed.catalog_expression, bytes(expression.into()));
        assert_eq!(observed.catalog_definition, definition);
        for expression in [
            "(kind = ANY (ARRAY['de cision'::text, 'observation'::text]))",
            "(kind = ANY (ARRAY['decision'::text, 'ob servation'::text]))",
            "((kind = ANY (ARRAY['decision'::text, 'observation'::text])) OR true)",
            "(kind = ANY (ARRAY['decision'::text, 'observation'::text, 'other'::text]))",
        ] {
            assert!(
                matches!(
                    check(
                        "history",
                        PresenceV1::Missing,
                        expression.into(),
                        PresenceV1::Missing
                    ),
                    CheckSchemaV1::Foreign(_)
                ),
                "{expression}"
            );
        }
        assert!(matches!(
            check(
                "provider_sequences",
                PresenceV1::Missing,
                "(next_value >= 0)".into(),
                PresenceV1::Missing
            ),
            CheckSchemaV1::NextValueNonnegative(_)
        ));
        assert!(matches!(
            check(
                "provider_sequences",
                PresenceV1::Missing,
                "(next_value > 0)".into(),
                PresenceV1::Missing
            ),
            CheckSchemaV1::Foreign(_)
        ));
    }

    #[test]
    fn postgres_capture_observes_catalog_and_refuses_changed_predicates() {
        let Ok(url) = std::env::var("ENTITY_POSTGRES_URL") else {
            eprintln!("PostgreSQL catalog acceptance not executed: ENTITY_POSTGRES_URL unset");
            return;
        };
        let client = postgres::Client::connect(&url, postgres::NoTls)
            .expect("connect to the explicitly selected disposable PostgreSQL fixture");
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("fixture clock")
            .as_nanos();
        let mut fixture = Fixture {
            client,
            schema: format!("aep_catalog_{}_{nonce}", std::process::id()),
        };
        fixture.reset();
        let observed = fixture
            .capture()
            .expect("the original provider catalog is admitted");
        let history = observed
            .tables
            .iter()
            .find(|table| table.name == "history")
            .unwrap();
        assert_eq!(history.columns[0].declared_type, bytes("text".into()));
        assert_eq!(
            history.columns[0].catalog_type_id,
            PresenceV1::Present("25".into())
        );
        assert!(
            matches!(&history.primary_key, PresenceV1::Present(key) if matches!(&key.backing_index, PresenceV1::Present(name) if name == "history_pkey"))
        );
        assert!(
            matches!(&history.checks[0], CheckSchemaV1::HistoryKindDecisionObservation(value) if !value.catalog_expression.as_bytes().is_empty() && matches!(value.catalog_definition, PresenceV1::Present(_)))
        );
        assert!(!observed.indexes[0].catalog_definition.as_bytes().is_empty());

        for mutation in [
            "ALTER TABLE instances ALTER COLUMN revision TYPE INTEGER",
            "ALTER TABLE instances ALTER COLUMN document DROP NOT NULL",
            "ALTER TABLE instances ALTER COLUMN revision SET DEFAULT 0",
            "ALTER TABLE history DROP CONSTRAINT history_record_id_key",
            "ALTER TABLE history DROP CONSTRAINT history_kind_check",
            "ALTER TABLE history DROP CONSTRAINT history_kind_check; ALTER TABLE history ADD CHECK (kind IN ('de cision','observation'))",
            "DROP INDEX instances_document_query",
            "DROP INDEX instances_document_query; CREATE INDEX instances_document_query ON instances USING gin ((document::jsonb) jsonb_path_ops) WHERE document IS NOT NULL",
        ] {
            fixture.reset();
            fixture.client.batch_execute(mutation).expect("apply one malformed-catalog fixture");
            assert!(matches!(fixture.capture(), Err(AcquisitionError::InvalidCapture)), "accepted changed catalog: {mutation}");
        }
        fixture
            .client
            .batch_execute(&format!("DROP SCHEMA {} CASCADE", fixture.schema))
            .expect("retire the disposable catalog fixture");
    }

    struct Fixture {
        client: postgres::Client,
        schema: String,
    }

    impl Fixture {
        fn reset(&mut self) {
            self.client
                .batch_execute(&format!(
                    "DROP SCHEMA IF EXISTS {} CASCADE; CREATE SCHEMA {}; SET search_path TO {}; {}",
                    self.schema, self.schema, self.schema, PROVIDER_DDL
                ))
                .expect("recreate only this test's schema");
        }

        fn capture(&mut self) -> Result<SqlSchemaV1, AcquisitionError> {
            let mut transaction = self
                .client
                .build_transaction()
                .isolation_level(postgres::IsolationLevel::RepeatableRead)
                .read_only(true)
                .start()
                .map_err(database)?;
            let result = read(&mut transaction, &self.schema);
            transaction.rollback().map_err(database)?;
            result
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = self
                .client
                .batch_execute(&format!("DROP SCHEMA IF EXISTS {} CASCADE", self.schema));
        }
    }

    const PROVIDER_DDL: &str = "
        CREATE TABLE instances (entity TEXT NOT NULL, id TEXT NOT NULL, revision BIGINT NOT NULL,
            document TEXT NOT NULL, PRIMARY KEY(entity,id));
        CREATE TABLE events (entity TEXT NOT NULL, id TEXT NOT NULL, revision BIGINT NOT NULL,
            position BIGINT NOT NULL, document TEXT NOT NULL, PRIMARY KEY(entity,id,revision,position));
        CREATE TABLE history (entity TEXT NOT NULL, id TEXT NOT NULL, position BIGINT NOT NULL,
            kind TEXT NOT NULL CHECK(kind IN ('decision','observation')), record_id TEXT NOT NULL UNIQUE,
            document TEXT NOT NULL, PRIMARY KEY(entity,id,position));
        CREATE TABLE legacy_origins (entity TEXT NOT NULL, id TEXT NOT NULL, revision BIGINT NOT NULL,
            PRIMARY KEY(entity,id));
        CREATE TABLE provider_sequences (namespace TEXT PRIMARY KEY, next_value BIGINT NOT NULL CHECK(next_value>=0));
        CREATE INDEX instances_document_query ON instances USING gin ((document::jsonb) jsonb_path_ops);
    ";
}
