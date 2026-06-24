use rusqlite::types::ValueRef;
use serde::Serialize;

use super::sqlite::{
    connection, file_size, list_base_tables, quote_identifier, sqlite_string_literal,
    value_ref_to_json, SqliteConnectionConfig,
};

fn value_to_sql(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => "NULL".to_string(),
        ValueRef::Integer(value) => value.to_string(),
        ValueRef::Real(value) => {
            if value.is_finite() {
                value.to_string()
            } else {
                "NULL".to_string()
            }
        }
        ValueRef::Text(value) => sqlite_string_literal(&String::from_utf8_lossy(value)),
        ValueRef::Blob(value) => {
            let hex = value
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<String>();
            format!("X'{hex}'")
        }
    }
}

fn show_create_object(
    conn: &rusqlite::Connection,
    object_type: &str,
    name: &str,
) -> Result<String, String> {
    conn.query_row(
        "SELECT COALESCE(sql, '') FROM sqlite_master WHERE type = ?1 AND name = ?2",
        [object_type, name],
        |row| row.get::<_, String>(0),
    )
    .map_err(|error| error.to_string())
}

fn table_columns(conn: &rusqlite::Connection, table: &str) -> Result<Vec<String>, String> {
    let quoted_table = quote_identifier(table)?;
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({quoted_table})"))
        .map_err(|error| error.to_string())?;
    let columns = statement
        .query_map([], |row| row.get(1))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<String>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(columns)
}

fn append_table_inserts(
    conn: &rusqlite::Connection,
    sql: &mut String,
    table: &str,
) -> Result<(), String> {
    let columns = table_columns(conn, table)?;
    if columns.is_empty() {
        return Ok(());
    }

    let quoted_table = quote_identifier(table)?;
    let column_list = columns
        .iter()
        .map(|column| quote_identifier(column))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    let mut statement = conn
        .prepare(&format!("SELECT * FROM {quoted_table}"))
        .map_err(|error| error.to_string())?;
    let mut rows = statement.query([]).map_err(|error| error.to_string())?;
    let mut value_rows = Vec::new();

    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        let values = (0..columns.len())
            .map(|index| {
                row.get_ref(index)
                    .map(value_to_sql)
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");
        value_rows.push(format!("({values})"));

        if value_rows.len() >= 200 {
            sql.push_str(&format!(
                "INSERT INTO {quoted_table} ({column_list}) VALUES\n{};\n",
                value_rows.join(",\n")
            ));
            value_rows.clear();
        }
    }

    if !value_rows.is_empty() {
        sql.push_str(&format!(
            "INSERT INTO {quoted_table} ({column_list}) VALUES\n{};\n",
            value_rows.join(",\n")
        ));
    }

    Ok(())
}

fn append_tables_export_sql(
    conn: &rusqlite::Connection,
    sql: &mut String,
    tables: Vec<String>,
    include_data: bool,
) -> Result<(), String> {
    for table in tables {
        let quoted_table = quote_identifier(&table)?;
        sql.push_str(&format!("-- Table structure for {quoted_table}\n"));
        sql.push_str(&format!("DROP TABLE IF EXISTS {quoted_table};\n"));
        sql.push_str(&show_create_object(conn, "table", &table)?);
        sql.push_str(";\n\n");

        if include_data {
            sql.push_str(&format!("-- Data for {quoted_table}\n"));
            append_table_inserts(conn, sql, &table)?;
            sql.push('\n');
        }
    }

    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteMaintenanceResult {
    pub ok: bool,
    pub message: String,
    pub rows: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteDatabaseInfo {
    pub path: String,
    pub size: u64,
    pub page_size: i64,
    pub page_count: i64,
    pub freelist_count: i64,
    pub journal_mode: String,
    pub foreign_keys: i64,
}

#[tauri::command]
pub fn sqlite_create_table(
    config: SqliteConnectionConfig,
    _database: String,
    table: String,
) -> Result<(), String> {
    let conn = connection(&config)?;
    conn.execute_batch(&format!(
        "CREATE TABLE {} (
            \"id\" INTEGER PRIMARY KEY AUTOINCREMENT
        );",
        quote_identifier(&table)?
    ))
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn sqlite_copy_table(
    config: SqliteConnectionConfig,
    _database: String,
    table: String,
    new_table: String,
    copy_data: bool,
) -> Result<(), String> {
    let conn = connection(&config)?;
    let ddl = show_create_object(&conn, "table", &table)?;
    let quoted_target = quote_identifier(&new_table)?;
    let rewritten = ddl
        .replacen(
            &format!("CREATE TABLE {}", quote_identifier(&table)?),
            &format!("CREATE TABLE {quoted_target}"),
            1,
        )
        .replacen(
            &format!("CREATE TABLE {table}"),
            &format!("CREATE TABLE {quoted_target}"),
            1,
        );
    conn.execute_batch(&rewritten)
        .map_err(|error| error.to_string())?;
    if copy_data {
        let quoted_source = quote_identifier(&table)?;
        conn.execute_batch(&format!(
            "INSERT INTO {quoted_target} SELECT * FROM {quoted_source};"
        ))
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn sqlite_rename_table(
    config: SqliteConnectionConfig,
    _database: String,
    table: String,
    new_table: String,
) -> Result<(), String> {
    let conn = connection(&config)?;
    conn.execute_batch(&format!(
        "ALTER TABLE {} RENAME TO {};",
        quote_identifier(&table)?,
        quote_identifier(&new_table)?
    ))
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn sqlite_drop_table(
    config: SqliteConnectionConfig,
    _database: String,
    table: String,
) -> Result<(), String> {
    let conn = connection(&config)?;
    conn.execute_batch(&format!("DROP TABLE {}", quote_identifier(&table)?))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn sqlite_empty_table(
    config: SqliteConnectionConfig,
    _database: String,
    table: String,
) -> Result<(), String> {
    let conn = connection(&config)?;
    conn.execute_batch(&format!("DELETE FROM {}", quote_identifier(&table)?))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn sqlite_export_tables_sql(
    config: SqliteConnectionConfig,
    _database: String,
    tables: Vec<String>,
    include_data: bool,
) -> Result<String, String> {
    let target_tables = tables
        .into_iter()
        .map(|table| table.trim().to_string())
        .filter(|table| !table.is_empty())
        .collect::<Vec<_>>();
    if target_tables.is_empty() {
        return Err("请选择要导出的表".to_string());
    }

    let conn = connection(&config)?;
    let mut sql = String::new();
    sql.push_str("-- MyHub SQLite table export\n");
    sql.push_str("PRAGMA foreign_keys=OFF;\nBEGIN TRANSACTION;\n\n");
    append_tables_export_sql(&conn, &mut sql, target_tables, include_data)?;
    sql.push_str("COMMIT;\nPRAGMA foreign_keys=ON;\n");
    Ok(sql)
}

#[tauri::command]
pub fn sqlite_export_database_sql(
    config: SqliteConnectionConfig,
    _database: String,
    include_data: bool,
) -> Result<String, String> {
    let conn = connection(&config)?;
    let tables = list_base_tables(&conn)?;
    let mut sql = String::new();
    sql.push_str("-- MyHub SQLite database export\n");
    sql.push_str("PRAGMA foreign_keys=OFF;\nBEGIN TRANSACTION;\n\n");
    append_tables_export_sql(&conn, &mut sql, tables, include_data)?;

    let mut statement = conn
        .prepare("SELECT type, name, sql FROM sqlite_master WHERE type IN ('view', 'index', 'trigger') AND sql IS NOT NULL ORDER BY type, name")
        .map_err(|error| error.to_string())?;
    let objects = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    for (object_type, name, ddl) in objects {
        sql.push_str(&format!("-- {object_type} {name}\n{ddl};\n\n"));
    }

    sql.push_str("COMMIT;\nPRAGMA foreign_keys=ON;\n");
    Ok(sql)
}

#[tauri::command]
pub fn sqlite_import_sql(
    config: SqliteConnectionConfig,
    _database: String,
    sql: String,
) -> Result<(), String> {
    if sql.trim().is_empty() {
        return Err("SQL 文件内容为空".to_string());
    }
    let conn = connection(&config)?;
    conn.execute_batch(&sql).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn sqlite_vacuum(config: SqliteConnectionConfig) -> Result<SqliteMaintenanceResult, String> {
    let conn = connection(&config)?;
    conn.execute_batch("VACUUM")
        .map_err(|error| error.to_string())?;
    Ok(SqliteMaintenanceResult {
        ok: true,
        message: "VACUUM 已完成".to_string(),
        rows: vec![],
    })
}

#[tauri::command]
pub fn sqlite_integrity_check(
    config: SqliteConnectionConfig,
    quick: bool,
) -> Result<SqliteMaintenanceResult, String> {
    let conn = connection(&config)?;
    let pragma = if quick {
        "PRAGMA quick_check"
    } else {
        "PRAGMA integrity_check"
    };
    let mut statement = conn.prepare(pragma).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            let value = row.get_ref(0)?;
            Ok(serde_json::json!({ "result": value_ref_to_json(value) }))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let ok = rows
        .iter()
        .all(|row| row.get("result").and_then(|value| value.as_str()) == Some("ok"));
    Ok(SqliteMaintenanceResult {
        ok,
        message: if ok {
            "检查通过"
        } else {
            "检查发现问题"
        }
        .to_string(),
        rows,
    })
}

#[tauri::command]
pub fn sqlite_analyze(config: SqliteConnectionConfig) -> Result<SqliteMaintenanceResult, String> {
    let conn = connection(&config)?;
    conn.execute_batch("ANALYZE")
        .map_err(|error| error.to_string())?;
    Ok(SqliteMaintenanceResult {
        ok: true,
        message: "ANALYZE 已完成".to_string(),
        rows: vec![],
    })
}

#[tauri::command]
pub fn sqlite_reindex(config: SqliteConnectionConfig) -> Result<SqliteMaintenanceResult, String> {
    let conn = connection(&config)?;
    conn.execute_batch("REINDEX")
        .map_err(|error| error.to_string())?;
    Ok(SqliteMaintenanceResult {
        ok: true,
        message: "REINDEX 已完成".to_string(),
        rows: vec![],
    })
}

#[tauri::command]
pub fn sqlite_database_info(config: SqliteConnectionConfig) -> Result<SqliteDatabaseInfo, String> {
    let conn = connection(&config)?;
    let page_size: i64 = conn
        .query_row("PRAGMA page_size", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let page_count: i64 = conn
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let freelist_count: i64 = conn
        .query_row("PRAGMA freelist_count", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let foreign_keys: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    Ok(SqliteDatabaseInfo {
        path: config.path.clone(),
        size: file_size(&config.path),
        page_size,
        page_count,
        freelist_count,
        journal_mode,
        foreign_keys,
    })
}
