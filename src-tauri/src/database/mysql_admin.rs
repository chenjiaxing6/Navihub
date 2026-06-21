use mysql::prelude::*;
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

    conn.query_drop(sql)
    .map_err(|error| error.to_string())
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

    Ok(MysqlDatabaseOptions { charsets, collations })
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
        if let Err(error) = conn.query_drop(format!("INSERT INTO {target_table} SELECT * FROM {source_table}")) {
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
