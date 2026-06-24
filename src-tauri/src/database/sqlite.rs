use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::fs;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteConnectionConfig {
    pub path: String,
    pub read_only: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteSchemaGroup {
    pub group_type: String,
    pub title: String,
    pub count: usize,
    pub items: Vec<JsonValue>,
}

#[derive(Debug, Serialize)]
pub struct SqliteSchema {
    pub name: String,
    pub collation: String,
    pub groups: Vec<SqliteSchemaGroup>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteQueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<JsonValue>,
    pub affected_rows: u64,
    pub elapsed_ms: u128,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteColumnInfo {
    pub name: String,
    pub column_type: String,
    pub nullable: String,
    pub key: String,
    pub default_value: Option<String>,
    pub extra: String,
    pub comment: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteIndexInfo {
    pub name: String,
    pub column_name: String,
    pub non_unique: u8,
    pub seq_in_index: u64,
    pub index_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteForeignKeyInfo {
    pub name: String,
    pub column_name: String,
    pub referenced_table: String,
    pub referenced_column: String,
    pub update_rule: String,
    pub delete_rule: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteTriggerInfo {
    pub name: String,
    pub timing: String,
    pub event: String,
    pub statement: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteCheckInfo {
    pub name: String,
    pub expression: String,
    pub enforced: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteTableOptionsInfo {
    pub engine: String,
    pub collation: String,
    pub comment: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteTableDetail {
    pub columns: Vec<SqliteColumnInfo>,
    pub indexes: Vec<SqliteIndexInfo>,
    pub foreign_keys: Vec<SqliteForeignKeyInfo>,
    pub triggers: Vec<SqliteTriggerInfo>,
    pub checks: Vec<SqliteCheckInfo>,
    pub options: SqliteTableOptionsInfo,
    pub ddl: String,
}

pub(crate) fn quote_identifier(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("名称不能为空".to_string());
    }
    if trimmed.contains('\0') {
        return Err("名称不能包含空字符".to_string());
    }
    Ok(format!("\"{}\"", trimmed.replace('"', "\"\"")))
}

pub(crate) fn sqlite_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub(crate) fn connection(config: &SqliteConnectionConfig) -> Result<Connection, String> {
    let path = config.path.trim();
    if path.is_empty() {
        return Err("请选择 SQLite 数据库文件".to_string());
    }

    let flags = if config.read_only.unwrap_or(false) {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    } else {
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
    };
    let conn = Connection::open_with_flags(path, flags).map_err(|error| error.to_string())?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|error| error.to_string())?;
    Ok(conn)
}

pub(crate) fn value_ref_to_json(value: ValueRef<'_>) -> JsonValue {
    match value {
        ValueRef::Null => JsonValue::Null,
        ValueRef::Integer(value) => json!(value),
        ValueRef::Real(value) => json!(value),
        ValueRef::Text(value) => String::from_utf8_lossy(value).to_string().into(),
        ValueRef::Blob(value) => {
            let hex = value
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<String>();
            JsonValue::String(format!("0x{hex}"))
        }
    }
}

fn query_rows(conn: &Connection, sql: &str) -> Result<(Vec<String>, Vec<JsonValue>), String> {
    let mut statement = conn.prepare(sql).map_err(|error| error.to_string())?;
    let columns = statement
        .column_names()
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    let mut query = statement.query([]).map_err(|error| error.to_string())?;

    while let Some(row) = query.next().map_err(|error| error.to_string())? {
        let mut object = serde_json::Map::new();
        for (index, column) in columns.iter().enumerate() {
            let value = row.get_ref(index).map_err(|error| error.to_string())?;
            object.insert(column.clone(), value_ref_to_json(value));
        }
        rows.push(JsonValue::Object(object));
    }

    Ok((columns, rows))
}

fn fetch_objects(conn: &Connection, object_type: &str) -> Result<Vec<JsonValue>, String> {
    let sql = if object_type == "table" {
        "SELECT m.name,
                m.sql,
                m.rootpage,
                EXISTS(
                    SELECT 1
                    FROM sqlite_master i
                    WHERE i.type = 'index'
                      AND i.tbl_name = m.name
                ) AS has_index,
                EXISTS(
                    SELECT 1
                    FROM sqlite_master t
                    WHERE t.type = 'trigger'
                      AND t.tbl_name = m.name
                      AND t.name NOT LIKE 'sqlite_%'
                ) AS has_trigger
         FROM sqlite_master m
         WHERE m.type = ?1
           AND m.name NOT LIKE 'sqlite_%'
         ORDER BY m.name"
    } else {
        "SELECT name, sql, rootpage, 0 AS has_index, 0 AS has_trigger
         FROM sqlite_master
         WHERE type = ?1
           AND name NOT LIKE 'sqlite_%'
         ORDER BY name"
    };
    let mut statement = conn.prepare(sql).map_err(|error| error.to_string())?;
    let objects = statement
        .query_map([object_type], |row| {
            let name: String = row.get(0)?;
            let sql: Option<String> = row.get(1)?;
            let root_page: i64 = row.get(2)?;
            let has_index: i64 = row.get(3)?;
            let has_trigger: i64 = row.get(4)?;
            Ok(json!({
                "name": name,
                "rowCount": 0,
                "rootPage": root_page,
                "hasIndex": has_index != 0,
                "hasTrigger": has_trigger != 0,
                "dataLength": 0,
                "engine": "SQLite",
                "createTime": "",
                "updateTime": "",
                "collation": "",
                "comment": sql.unwrap_or_default(),
            }))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(objects)
}

fn fetch_named_objects(conn: &Connection, object_type: &str) -> Result<Vec<JsonValue>, String> {
    let mut statement = conn
        .prepare(
            "SELECT name
             FROM sqlite_master
             WHERE type = ?1
               AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )
        .map_err(|error| error.to_string())?;
    let objects = statement
        .query_map([object_type], |row| {
            let name: String = row.get(0)?;
            Ok(json!({ "name": name }))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(objects)
}

#[tauri::command]
pub fn sqlite_test_connection(config: SqliteConnectionConfig) -> Result<String, String> {
    let conn = connection(&config)?;
    conn.query_row("SELECT 1", [], |_| Ok(()))
        .map_err(|error| error.to_string())?;
    Ok("连接成功".to_string())
}

#[tauri::command]
pub fn sqlite_load_schema(config: SqliteConnectionConfig) -> Result<Vec<SqliteSchema>, String> {
    let conn = connection(&config)?;
    let tables = fetch_objects(&conn, "table")?;
    let views = fetch_objects(&conn, "view")?;
    let indexes = fetch_named_objects(&conn, "index")?;
    let triggers = fetch_named_objects(&conn, "trigger")?;

    Ok(vec![SqliteSchema {
        name: "main".to_string(),
        collation: "BINARY".to_string(),
        groups: vec![
            SqliteSchemaGroup {
                group_type: "table".to_string(),
                title: "表".to_string(),
                count: tables.len(),
                items: tables,
            },
            SqliteSchemaGroup {
                group_type: "view".to_string(),
                title: "视图".to_string(),
                count: views.len(),
                items: views,
            },
            SqliteSchemaGroup {
                group_type: "index".to_string(),
                title: "索引".to_string(),
                count: indexes.len(),
                items: indexes,
            },
            SqliteSchemaGroup {
                group_type: "trigger".to_string(),
                title: "触发器".to_string(),
                count: triggers.len(),
                items: triggers,
            },
            SqliteSchemaGroup {
                group_type: "query".to_string(),
                title: "查询".to_string(),
                count: 0,
                items: vec![],
            },
        ],
    }])
}

#[tauri::command]
pub fn sqlite_execute_query(
    config: SqliteConnectionConfig,
    _database: Option<String>,
    sql: String,
) -> Result<SqliteQueryResult, String> {
    let start = Instant::now();
    let conn = connection(&config)?;
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return Err("SQL 为空".to_string());
    }

    let is_query = trimmed
        .split_whitespace()
        .next()
        .map(|keyword| {
            matches!(
                keyword.to_ascii_lowercase().as_str(),
                "select" | "with" | "pragma" | "explain"
            )
        })
        .unwrap_or(false);

    if is_query {
        let (columns, rows) = query_rows(&conn, trimmed)?;
        Ok(SqliteQueryResult {
            columns,
            rows,
            affected_rows: 0,
            elapsed_ms: start.elapsed().as_millis(),
        })
    } else {
        conn.execute_batch(trimmed)
            .map_err(|error| error.to_string())?;
        Ok(SqliteQueryResult {
            columns: vec![],
            rows: vec![],
            affected_rows: conn.changes(),
            elapsed_ms: start.elapsed().as_millis(),
        })
    }
}

#[tauri::command]
pub fn sqlite_describe_table(
    config: SqliteConnectionConfig,
    _database: String,
    table: String,
) -> Result<SqliteTableDetail, String> {
    let conn = connection(&config)?;
    let quoted_table = quote_identifier(&table)?;

    let columns = {
        let mut statement = conn
            .prepare(&format!("PRAGMA table_info({quoted_table})"))
            .map_err(|error| error.to_string())?;
        let columns = statement
            .query_map([], |row| {
                let name: String = row.get(1)?;
                let column_type: String = row.get(2)?;
                let not_null: i64 = row.get(3)?;
                let default_value: Option<String> = row.get(4)?;
                let pk: i64 = row.get(5)?;
                Ok(SqliteColumnInfo {
                    name,
                    column_type,
                    nullable: if not_null == 0 { "YES" } else { "NO" }.to_string(),
                    key: if pk > 0 { "PRI" } else { "" }.to_string(),
                    default_value,
                    extra: if pk > 0 {
                        format!("pk:{pk}")
                    } else {
                        String::new()
                    },
                    comment: String::new(),
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        columns
    };

    let indexes = {
        let mut indexes = Vec::new();
        let mut index_statement = conn
            .prepare(&format!("PRAGMA index_list({quoted_table})"))
            .map_err(|error| error.to_string())?;
        let index_rows = index_statement
            .query_map([], |row| {
                let seq: i64 = row.get(0)?;
                let name: String = row.get(1)?;
                let unique: i64 = row.get(2)?;
                let origin: String = row.get(3)?;
                Ok((seq, name, unique, origin))
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;

        for (_seq, index_name, unique, origin) in index_rows {
            let quoted_index = quote_identifier(&index_name)?;
            let mut column_statement = conn
                .prepare(&format!("PRAGMA index_info({quoted_index})"))
                .map_err(|error| error.to_string())?;
            let columns = column_statement
                .query_map([], |row| {
                    let seq_in_index: i64 = row.get(0)?;
                    let column_name: String = row.get(2)?;
                    Ok((seq_in_index, column_name))
                })
                .map_err(|error| error.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?;

            for (seq_in_index, column_name) in columns {
                indexes.push(SqliteIndexInfo {
                    name: if origin == "pk" {
                        "PRIMARY".to_string()
                    } else {
                        index_name.clone()
                    },
                    column_name,
                    non_unique: if unique == 0 { 1 } else { 0 },
                    seq_in_index: (seq_in_index + 1) as u64,
                    index_type: "BTREE".to_string(),
                });
            }
        }

        indexes
    };

    let foreign_keys = {
        let mut statement = conn
            .prepare(&format!("PRAGMA foreign_key_list({quoted_table})"))
            .map_err(|error| error.to_string())?;
        let foreign_keys = statement
            .query_map([], |row| {
                let id: i64 = row.get(0)?;
                let seq: i64 = row.get(1)?;
                let referenced_table: String = row.get(2)?;
                let column_name: String = row.get(3)?;
                let referenced_column: String = row.get(4)?;
                let update_rule: String = row.get(5)?;
                let delete_rule: String = row.get(6)?;
                Ok(SqliteForeignKeyInfo {
                    name: format!("fk_{table}_{id}_{seq}"),
                    column_name,
                    referenced_table,
                    referenced_column,
                    update_rule,
                    delete_rule,
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        foreign_keys
    };

    let triggers = {
        let mut statement = conn
            .prepare(
                "SELECT name, sql
                 FROM sqlite_master
                 WHERE type = 'trigger' AND tbl_name = ?1
                 ORDER BY name",
            )
            .map_err(|error| error.to_string())?;
        let triggers = statement
            .query_map([table.as_str()], |row| {
                let name: String = row.get(0)?;
                let sql: Option<String> = row.get(1)?;
                let statement = sql.unwrap_or_default();
                let upper = statement.to_ascii_uppercase();
                let timing = if upper.contains(" INSTEAD OF ") {
                    "INSTEAD OF"
                } else if upper.contains(" AFTER ") {
                    "AFTER"
                } else {
                    "BEFORE"
                };
                let event = ["INSERT", "UPDATE", "DELETE"]
                    .into_iter()
                    .find(|event| upper.contains(event))
                    .unwrap_or("INSERT");
                Ok(SqliteTriggerInfo {
                    name,
                    timing: timing.to_string(),
                    event: event.to_string(),
                    statement,
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        triggers
    };

    let ddl: String = conn
        .query_row(
            "SELECT COALESCE(sql, '') FROM sqlite_master WHERE type IN ('table', 'view') AND name = ?1",
            [table.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    Ok(SqliteTableDetail {
        columns,
        indexes,
        foreign_keys,
        triggers,
        checks: vec![],
        options: SqliteTableOptionsInfo {
            engine: "SQLite".to_string(),
            collation: "BINARY".to_string(),
            comment: String::new(),
        },
        ddl,
    })
}

pub(crate) fn list_base_tables(conn: &Connection) -> Result<Vec<String>, String> {
    let mut statement = conn
        .prepare(
            "SELECT name
             FROM sqlite_master
             WHERE type = 'table'
               AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )
        .map_err(|error| error.to_string())?;
    let tables = statement
        .query_map([], |row| row.get(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<String>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(tables)
}

pub(crate) fn file_size(path: &str) -> u64 {
    fs::metadata(Path::new(path))
        .map(|metadata| metadata.len())
        .unwrap_or(0)
}
