use mysql::prelude::*;
use mysql::{params, Row, Value};
use serde::Serialize;
use tauri::State;

use super::mysql::{pool, MysqlConnectionConfig, MysqlState};

fn quote_identifier(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("名称不能为空".to_string());
    }
    if trimmed.contains('\0') {
        return Err("名称不能包含空字符".to_string());
    }

    Ok(format!("`{}`", trimmed.replace('`', "``")))
}

fn qualified_name(database: &str, table: &str) -> Result<String, String> {
    Ok(format!(
        "{}.{}",
        quote_identifier(database)?,
        quote_identifier(table)?
    ))
}

fn mysql_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn value_to_sql(value: &Value) -> String {
    match value {
        Value::NULL => "NULL".to_string(),
        Value::Bytes(bytes) => String::from_utf8(bytes.clone())
            .map(|value| mysql_string_literal(&value))
            .unwrap_or_else(|_| {
                let hex = bytes
                    .iter()
                    .map(|byte| format!("{byte:02X}"))
                    .collect::<String>();
                format!("X'{hex}'")
            }),
        Value::Int(value) => value.to_string(),
        Value::UInt(value) => value.to_string(),
        Value::Float(value) => {
            if value.is_finite() {
                value.to_string()
            } else {
                "NULL".to_string()
            }
        }
        Value::Double(value) => {
            if value.is_finite() {
                value.to_string()
            } else {
                "NULL".to_string()
            }
        }
        Value::Date(year, month, day, hour, minute, second, micros) => {
            if *hour == 0 && *minute == 0 && *second == 0 && *micros == 0 {
                mysql_string_literal(&format!("{year:04}-{month:02}-{day:02}"))
            } else if *micros == 0 {
                mysql_string_literal(&format!(
                    "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}"
                ))
            } else {
                mysql_string_literal(&format!(
                    "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{micros:06}"
                ))
            }
        }
        Value::Time(is_negative, days, hours, minutes, seconds, micros) => {
            let sign = if *is_negative { "-" } else { "" };
            let total_hours = days.saturating_mul(24).saturating_add(u32::from(*hours));
            if *micros == 0 {
                mysql_string_literal(&format!("{sign}{total_hours:02}:{minutes:02}:{seconds:02}"))
            } else {
                mysql_string_literal(&format!(
                    "{sign}{total_hours:02}:{minutes:02}:{seconds:02}.{micros:06}"
                ))
            }
        }
    }
}

fn show_create_table(
    conn: &mut mysql::PooledConn,
    database: &str,
    table: &str,
) -> Result<String, String> {
    let rows: Vec<Row> = conn
        .query(format!(
            "SHOW CREATE TABLE {}",
            qualified_name(database, table)?
        ))
        .map_err(|error| error.to_string())?;

    rows.first()
        .and_then(|row| row.as_ref(1))
        .and_then(|value| match value {
            Value::Bytes(bytes) => String::from_utf8(bytes.clone()).ok(),
            _ => None,
        })
        .ok_or_else(|| format!("无法读取表“{table}”的建表语句"))
}

fn append_table_inserts(
    conn: &mut mysql::PooledConn,
    sql: &mut String,
    database: &str,
    table: &str,
) -> Result<(), String> {
    let columns: Vec<String> = conn
        .exec_map(
            "SELECT COLUMN_NAME
             FROM information_schema.COLUMNS
             WHERE TABLE_SCHEMA = :schema AND TABLE_NAME = :table
             ORDER BY ORDINAL_POSITION",
            params! {
                "schema" => database,
                "table" => table,
            },
            |name: String| name,
        )
        .map_err(|error| error.to_string())?;

    if columns.is_empty() {
        return Ok(());
    }

    let column_list = columns
        .iter()
        .map(|column| quote_identifier(column))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    let mut result = conn
        .query_iter(format!(
            "SELECT * FROM {}",
            qualified_name(database, table)?
        ))
        .map_err(|error| error.to_string())?;
    let mut value_rows = Vec::new();

    while let Some(mut result_set) = result.iter() {
        while let Some(row_result) = result_set.next() {
            let row: Row = row_result.map_err(|error| error.to_string())?;
            let values = (0..columns.len())
                .map(|index| value_to_sql(row.as_ref(index).unwrap_or(&Value::NULL)))
                .collect::<Vec<_>>()
                .join(", ");
            value_rows.push(format!("({values})"));

            if value_rows.len() >= 200 {
                sql.push_str(&format!(
                    "INSERT INTO {} ({column_list}) VALUES\n{};\n",
                    qualified_name(database, table)?,
                    value_rows.join(",\n")
                ));
                value_rows.clear();
            }
        }
    }
    drop(result);

    if !value_rows.is_empty() {
        sql.push_str(&format!(
            "INSERT INTO {} ({column_list}) VALUES\n{};\n",
            qualified_name(database, table)?,
            value_rows.join(",\n")
        ));
    }

    Ok(())
}

fn list_base_tables(conn: &mut mysql::PooledConn, database: &str) -> Result<Vec<String>, String> {
    conn.exec_map(
        "SELECT TABLE_NAME
         FROM information_schema.TABLES
         WHERE TABLE_SCHEMA = :schema AND TABLE_TYPE = 'BASE TABLE'
         ORDER BY TABLE_NAME",
        params! {
            "schema" => database,
        },
        |name: String| name,
    )
    .map_err(|error| error.to_string())
}

fn append_tables_export_sql(
    conn: &mut mysql::PooledConn,
    sql: &mut String,
    database: &str,
    tables: Vec<String>,
    include_data: bool,
) -> Result<(), String> {
    for table in tables {
        sql.push_str(&format!(
            "-- Table structure for {}\n",
            qualified_name(database, &table)?
        ));
        sql.push_str(&format!(
            "DROP TABLE IF EXISTS {};\n",
            qualified_name(database, &table)?
        ));
        sql.push_str(&show_create_table(conn, database, &table)?);
        sql.push_str(";\n\n");

        if include_data {
            sql.push_str(&format!(
                "-- Data for {}\n",
                qualified_name(database, &table)?
            ));
            append_table_inserts(conn, sql, database, &table)?;
            sql.push('\n');
        }
    }

    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MysqlCharsetOption {
    pub name: String,
    pub default_collation: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MysqlCollationOption {
    pub name: String,
    pub charset: String,
    pub is_default: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MysqlDatabaseOptions {
    pub charsets: Vec<MysqlCharsetOption>,
    pub collations: Vec<MysqlCollationOption>,
}

fn validate_option_identifier(value: &str, field_name: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    if trimmed
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_' || character == '-')
    {
        return Ok(trimmed.to_string());
    }

    Err(format!("{field_name}格式不合法"))
}

#[tauri::command]
pub fn mysql_create_database(
    state: State<'_, MysqlState>,
    config: MysqlConnectionConfig,
    database: String,
    charset: Option<String>,
    collation: Option<String>,
) -> Result<(), String> {
    let pool = pool(&state, &config, None)?;
    let mut conn = pool.get_conn().map_err(|error| error.to_string())?;
    let mut sql = format!("CREATE DATABASE {}", quote_identifier(&database)?);
    let charset = validate_option_identifier(charset.as_deref().unwrap_or(""), "字符集")?;
    let collation = validate_option_identifier(collation.as_deref().unwrap_or(""), "排序规则")?;

    if !charset.is_empty() {
        sql.push_str(&format!(" DEFAULT CHARACTER SET {charset}"));
    }
    if !collation.is_empty() {
        sql.push_str(&format!(" COLLATE {collation}"));
    }

    conn.query_drop(sql).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn mysql_alter_database_options(
    state: State<'_, MysqlState>,
    config: MysqlConnectionConfig,
    database: String,
    charset: Option<String>,
    collation: Option<String>,
) -> Result<(), String> {
    let pool = pool(&state, &config, None)?;
    let mut conn = pool.get_conn().map_err(|error| error.to_string())?;
    let charset = validate_option_identifier(charset.as_deref().unwrap_or(""), "字符集")?;
    let collation = validate_option_identifier(collation.as_deref().unwrap_or(""), "排序规则")?;

    if charset.is_empty() && collation.is_empty() {
        return Err("请选择字符集或排序规则".to_string());
    }

    let mut sql = format!("ALTER DATABASE {}", quote_identifier(&database)?);
    if !charset.is_empty() {
        sql.push_str(&format!(" DEFAULT CHARACTER SET {charset}"));
    }
    if !collation.is_empty() {
        sql.push_str(&format!(" COLLATE {collation}"));
    }

    conn.query_drop(sql).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn mysql_list_database_options(
    state: State<'_, MysqlState>,
    config: MysqlConnectionConfig,
) -> Result<MysqlDatabaseOptions, String> {
    let pool = pool(&state, &config, None)?;
    let mut conn = pool.get_conn().map_err(|error| error.to_string())?;

    let charsets = conn
        .query_map(
            "SELECT CHARACTER_SET_NAME, DESCRIPTION, DEFAULT_COLLATE_NAME
             FROM information_schema.CHARACTER_SETS
             ORDER BY CHARACTER_SET_NAME",
            |(name, description, default_collation): (String, String, String)| MysqlCharsetOption {
                name,
                description,
                default_collation,
            },
        )
        .map_err(|error| error.to_string())?;

    let collations = conn
        .query_map(
            "SELECT COLLATION_NAME, CHARACTER_SET_NAME, IS_DEFAULT
             FROM information_schema.COLLATIONS
             ORDER BY CHARACTER_SET_NAME, COLLATION_NAME",
            |(name, charset, default_value): (String, String, String)| MysqlCollationOption {
                name,
                charset,
                is_default: default_value == "Yes",
            },
        )
        .map_err(|error| error.to_string())?;

    Ok(MysqlDatabaseOptions {
        charsets,
        collations,
    })
}

#[tauri::command]
pub fn mysql_drop_database(
    state: State<'_, MysqlState>,
    config: MysqlConnectionConfig,
    database: String,
) -> Result<(), String> {
    let pool = pool(&state, &config, None)?;
    let mut conn = pool.get_conn().map_err(|error| error.to_string())?;
    conn.query_drop(format!("DROP DATABASE {}", quote_identifier(&database)?))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn mysql_create_table(
    state: State<'_, MysqlState>,
    config: MysqlConnectionConfig,
    database: String,
    table: String,
) -> Result<(), String> {
    let pool = pool(&state, &config, Some(&database))?;
    let mut conn = pool.get_conn().map_err(|error| error.to_string())?;
    conn.query_drop(format!(
        "CREATE TABLE {} (
            `id` BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
            PRIMARY KEY (`id`)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci",
        qualified_name(&database, &table)?
    ))
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn mysql_copy_table(
    state: State<'_, MysqlState>,
    config: MysqlConnectionConfig,
    database: String,
    table: String,
    new_table: String,
    copy_data: bool,
) -> Result<(), String> {
    let pool = pool(&state, &config, Some(&database))?;
    let mut conn = pool.get_conn().map_err(|error| error.to_string())?;
    let source_table = qualified_name(&database, &table)?;
    let target_table = qualified_name(&database, &new_table)?;
    conn.query_drop(format!("CREATE TABLE {target_table} LIKE {source_table}"))
        .map_err(|error| error.to_string())?;

    if copy_data {
        if let Err(error) = conn.query_drop(format!(
            "INSERT INTO {target_table} SELECT * FROM {source_table}"
        )) {
            let _ = conn.query_drop(format!("DROP TABLE {target_table}"));
            return Err(error.to_string());
        }
    }

    Ok(())
}

#[tauri::command]
pub fn mysql_rename_table(
    state: State<'_, MysqlState>,
    config: MysqlConnectionConfig,
    database: String,
    table: String,
    new_table: String,
) -> Result<(), String> {
    let pool = pool(&state, &config, Some(&database))?;
    let mut conn = pool.get_conn().map_err(|error| error.to_string())?;
    conn.query_drop(format!(
        "RENAME TABLE {} TO {}",
        qualified_name(&database, &table)?,
        qualified_name(&database, &new_table)?
    ))
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn mysql_drop_table(
    state: State<'_, MysqlState>,
    config: MysqlConnectionConfig,
    database: String,
    table: String,
) -> Result<(), String> {
    let pool = pool(&state, &config, Some(&database))?;
    let mut conn = pool.get_conn().map_err(|error| error.to_string())?;
    conn.query_drop(format!("DROP TABLE {}", qualified_name(&database, &table)?))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn mysql_empty_table(
    state: State<'_, MysqlState>,
    config: MysqlConnectionConfig,
    database: String,
    table: String,
) -> Result<(), String> {
    let pool = pool(&state, &config, Some(&database))?;
    let mut conn = pool.get_conn().map_err(|error| error.to_string())?;
    conn.query_drop(format!(
        "DELETE FROM {}",
        qualified_name(&database, &table)?
    ))
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn mysql_truncate_table(
    state: State<'_, MysqlState>,
    config: MysqlConnectionConfig,
    database: String,
    table: String,
) -> Result<(), String> {
    let pool = pool(&state, &config, Some(&database))?;
    let mut conn = pool.get_conn().map_err(|error| error.to_string())?;
    conn.query_drop(format!(
        "TRUNCATE TABLE {}",
        qualified_name(&database, &table)?
    ))
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn mysql_export_tables_sql(
    state: State<'_, MysqlState>,
    config: MysqlConnectionConfig,
    database: String,
    tables: Vec<String>,
    include_data: bool,
) -> Result<String, String> {
    let target_tables: Vec<String> = tables
        .into_iter()
        .map(|table| table.trim().to_string())
        .filter(|table| !table.is_empty())
        .collect();
    if target_tables.is_empty() {
        return Err("请选择要导出的表".to_string());
    }

    let pool = pool(&state, &config, Some(&database))?;
    let mut conn = pool.get_conn().map_err(|error| error.to_string())?;
    let mut sql = String::new();

    sql.push_str("-- MyHub MySQL table export\n");
    sql.push_str(&format!("-- Database: {}\n\n", database));
    sql.push_str("SET FOREIGN_KEY_CHECKS=0;\n\n");

    append_tables_export_sql(&mut conn, &mut sql, &database, target_tables, include_data)?;

    sql.push_str("SET FOREIGN_KEY_CHECKS=1;\n");
    Ok(sql)
}

#[tauri::command]
pub fn mysql_export_database_sql(
    state: State<'_, MysqlState>,
    config: MysqlConnectionConfig,
    database: String,
    include_data: bool,
) -> Result<String, String> {
    let pool = pool(&state, &config, Some(&database))?;
    let mut conn = pool.get_conn().map_err(|error| error.to_string())?;
    let tables = list_base_tables(&mut conn, &database)?;
    let mut sql = String::new();

    sql.push_str("-- MyHub MySQL database export\n");
    sql.push_str(&format!("-- Database: {}\n\n", database));
    sql.push_str(&format!(
        "CREATE DATABASE IF NOT EXISTS {};\n",
        quote_identifier(&database)?
    ));
    sql.push_str(&format!("USE {};\n\n", quote_identifier(&database)?));
    sql.push_str("SET FOREIGN_KEY_CHECKS=0;\n\n");
    append_tables_export_sql(&mut conn, &mut sql, &database, tables, include_data)?;
    sql.push_str("SET FOREIGN_KEY_CHECKS=1;\n");

    Ok(sql)
}

#[tauri::command]
pub fn mysql_import_sql(
    state: State<'_, MysqlState>,
    config: MysqlConnectionConfig,
    database: String,
    sql: String,
) -> Result<(), String> {
    if sql.trim().is_empty() {
        return Err("SQL 文件内容为空".to_string());
    }

    let pool = pool(&state, &config, Some(&database))?;
    let mut conn = pool.get_conn().map_err(|error| error.to_string())?;
    conn.query_iter(sql)
        .map_err(|error| error.to_string())?
        .for_each(drop);
    Ok(())
}
