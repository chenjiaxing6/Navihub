use mysql::prelude::*;
use mysql::{params, OptsBuilder, Pool, PooledConn, Row, Value};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};
use tauri::State;

#[derive(Clone, Default)]
pub struct MysqlState {
    pools: Arc<Mutex<HashMap<String, Pool>>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MysqlConnectionConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MysqlSchemaGroup {
    pub group_type: String,
    pub title: String,
    pub count: usize,
    pub items: Vec<JsonValue>,
}

#[derive(Debug, Serialize)]
pub struct MysqlSchema {
    pub name: String,
    pub groups: Vec<MysqlSchemaGroup>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MysqlQueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<JsonValue>,
    pub affected_rows: u64,
    pub elapsed_ms: u128,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MysqlColumnInfo {
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
pub struct MysqlIndexInfo {
    pub name: String,
    pub column_name: String,
    pub non_unique: u8,
    pub seq_in_index: u64,
    pub index_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MysqlForeignKeyInfo {
    pub name: String,
    pub column_name: String,
    pub referenced_table: String,
    pub referenced_column: String,
    pub update_rule: String,
    pub delete_rule: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MysqlTriggerInfo {
    pub name: String,
    pub timing: String,
    pub event: String,
    pub statement: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MysqlCheckInfo {
    pub name: String,
    pub expression: String,
    pub enforced: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MysqlTableOptionsInfo {
    pub engine: String,
    pub collation: String,
    pub comment: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MysqlTableDetail {
    pub columns: Vec<MysqlColumnInfo>,
    pub indexes: Vec<MysqlIndexInfo>,
    pub foreign_keys: Vec<MysqlForeignKeyInfo>,
    pub triggers: Vec<MysqlTriggerInfo>,
    pub checks: Vec<MysqlCheckInfo>,
    pub options: MysqlTableOptionsInfo,
    pub ddl: String,
}

fn effective_database<'a>(config: &'a MysqlConnectionConfig, database: Option<&'a str>) -> Option<&'a str> {
    database
        .or(config.database.as_deref())
        .filter(|value| !value.is_empty())
}

fn pool_key(config: &MysqlConnectionConfig, database: Option<&str>) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        config.host,
        config.port,
        config.username,
        config.password,
        effective_database(config, database).unwrap_or_default()
    )
}

fn create_pool(config: &MysqlConnectionConfig, database: Option<&str>) -> Result<Pool, String> {
    let mut builder = OptsBuilder::new()
        .ip_or_hostname(Some(config.host.clone()))
        .tcp_port(config.port)
        .user(Some(config.username.clone()))
        .pass(Some(config.password.clone()));

    if let Some(database) = effective_database(config, database) {
        builder = builder.db_name(Some(database.to_string()));
    }

    Pool::new(builder).map_err(|error| error.to_string())
}

pub(crate) fn pool(state: &MysqlState, config: &MysqlConnectionConfig, database: Option<&str>) -> Result<Pool, String> {
    let key = pool_key(config, database);
    if let Some(pool) = state
        .pools
        .lock()
        .map_err(|_| "MySQL connection pool state is unavailable".to_string())?
        .get(&key)
        .cloned()
    {
        return Ok(pool);
    }

    let pool = create_pool(config, database)?;
    state
        .pools
        .lock()
        .map_err(|_| "MySQL connection pool state is unavailable".to_string())?
        .insert(key, pool.clone());
    Ok(pool)
}

fn value_to_json(value: Value) -> JsonValue {
    match value {
        Value::NULL => JsonValue::Null,
        Value::Bytes(bytes) => String::from_utf8(bytes)
            .map(JsonValue::String)
            .unwrap_or_else(|bytes| JsonValue::String(format!("{:?}", bytes.into_bytes()))),
        Value::Int(value) => json!(value),
        Value::UInt(value) => json!(value),
        Value::Float(value) => json!(value),
        Value::Double(value) => json!(value),
        Value::Date(year, month, day, hour, minute, second, micros) => {
            json!(format!(
                "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{micros:06}"
            ))
        }
        Value::Time(is_negative, days, hours, minutes, seconds, micros) => {
            let sign = if is_negative { "-" } else { "" };
            json!(format!(
                "{sign}{days} {hours:02}:{minutes:02}:{seconds:02}.{micros:06}"
            ))
        }
    }
}

fn fetch_table_items(
    conn: &mut PooledConn,
    database: &str,
    table_type: &str,
) -> Result<Vec<JsonValue>, String> {
    conn.exec_map(
        "SELECT
             TABLE_NAME,
             TABLE_ROWS,
             DATA_LENGTH,
             ENGINE,
             DATE_FORMAT(CREATE_TIME, '%Y-%m-%d %H:%i:%s') AS CREATE_TIME,
             DATE_FORMAT(UPDATE_TIME, '%Y-%m-%d %H:%i:%s') AS UPDATE_TIME,
             TABLE_COLLATION,
             TABLE_COMMENT
         FROM TABLES
         WHERE TABLE_SCHEMA = :schema AND TABLE_TYPE = :table_type
         ORDER BY TABLE_NAME",
        params! {
            "schema" => database,
            "table_type" => table_type,
        },
        |(name, row_count, data_length, engine, create_time, update_time, collation, comment): (
            String,
            Option<u64>,
            Option<u64>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        )| {
            json!({
                "name": name,
                "rowCount": row_count.unwrap_or(0),
                "dataLength": data_length.unwrap_or(0),
                "engine": engine.unwrap_or_default(),
                "createTime": create_time.unwrap_or_default(),
                "updateTime": update_time.unwrap_or_default(),
                "collation": collation.unwrap_or_default(),
                "comment": comment.unwrap_or_default(),
            })
        },
    )
    .map_err(|error| error.to_string())
}

fn fetch_routines(
    conn: &mut PooledConn,
    database: &str,
) -> Result<Vec<JsonValue>, String> {
    conn.exec_map(
        "SELECT ROUTINE_NAME
         FROM ROUTINES
         WHERE ROUTINE_SCHEMA = :schema
         ORDER BY ROUTINE_NAME",
        params! {
            "schema" => database,
        },
        |name: String| json!({ "name": name }),
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn mysql_test_connection(
    state: State<'_, MysqlState>,
    config: MysqlConnectionConfig,
) -> Result<String, String> {
    let pool = pool(&state, &config, None)?;
    let mut conn = pool.get_conn().map_err(|error| error.to_string())?;
    conn.query_drop("SELECT 1")
        .map_err(|error| error.to_string())?;
    Ok("连接成功".to_string())
}

#[tauri::command]
pub fn mysql_load_schema(
    state: State<'_, MysqlState>,
    config: MysqlConnectionConfig,
) -> Result<Vec<MysqlSchema>, String> {
    let pool = pool(&state, &config, Some("information_schema"))?;
    let mut conn = pool.get_conn().map_err(|error| error.to_string())?;

    let databases: Vec<String> = conn
        .query_map(
            "SELECT SCHEMA_NAME
             FROM SCHEMATA
             ORDER BY SCHEMA_NAME",
            |name: String| name,
        )
        .map_err(|error| error.to_string())?;

    databases
        .into_iter()
        .map(|database| {
            let tables = fetch_table_items(&mut conn, &database, "BASE TABLE")?;
            let views = fetch_table_items(&mut conn, &database, "VIEW")?;
            let routines = fetch_routines(&mut conn, &database)?;

            Ok(MysqlSchema {
                name: database,
                groups: vec![
                    MysqlSchemaGroup {
                        group_type: "table".to_string(),
                        title: "表".to_string(),
                        count: tables.len(),
                        items: tables,
                    },
                    MysqlSchemaGroup {
                        group_type: "view".to_string(),
                        title: "视图".to_string(),
                        count: views.len(),
                        items: views,
                    },
                    MysqlSchemaGroup {
                        group_type: "query".to_string(),
                        title: "查询".to_string(),
                        count: 0,
                        items: vec![],
                    },
                    MysqlSchemaGroup {
                        group_type: "function".to_string(),
                        title: "存储过程/函数".to_string(),
                        count: routines.len(),
                        items: routines,
                    },
                ],
            })
        })
        .collect()
}

#[tauri::command]
pub fn mysql_execute_query(
    state: State<'_, MysqlState>,
    config: MysqlConnectionConfig,
    database: Option<String>,
    sql: String,
) -> Result<MysqlQueryResult, String> {
    let start = Instant::now();
    let pool = pool(&state, &config, database.as_deref())?;
    let mut conn = pool.get_conn().map_err(|error| error.to_string())?;

    let mut result = conn.query_iter(sql).map_err(|error| error.to_string())?;
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    let mut affected_rows = 0;

    while let Some(mut result_set) = result.iter() {
        affected_rows = result_set.affected_rows();
        let set_columns: Vec<String> = result_set
            .columns()
            .as_ref()
            .iter()
            .map(|column| column.name_str().to_string())
            .collect();

        if set_columns.is_empty() {
            continue;
        }

        let mut set_rows = Vec::new();
        while let Some(row_result) = result_set.next() {
            let row: Row = row_result.map_err(|error| error.to_string())?;
            let mut object = serde_json::Map::new();

            for (index, column) in set_columns.iter().enumerate() {
                let value = row.as_ref(index).cloned().unwrap_or(Value::NULL);
                object.insert(column.clone(), value_to_json(value));
            }

            set_rows.push(JsonValue::Object(object));
        }

        columns = set_columns;
        rows = set_rows;
    }

    Ok(MysqlQueryResult {
        columns,
        rows,
        affected_rows,
        elapsed_ms: start.elapsed().as_millis(),
    })
}

#[tauri::command]
pub fn mysql_describe_table(
    state: State<'_, MysqlState>,
    config: MysqlConnectionConfig,
    database: String,
    table: String,
) -> Result<MysqlTableDetail, String> {
    let pool = pool(&state, &config, Some(&database))?;
    let mut conn = pool.get_conn().map_err(|error| error.to_string())?;

    let columns = conn
        .exec_map(
            "SELECT COLUMN_NAME, COLUMN_TYPE, IS_NULLABLE, COLUMN_KEY, COLUMN_DEFAULT, EXTRA, COLUMN_COMMENT
             FROM information_schema.COLUMNS
             WHERE TABLE_SCHEMA = :schema AND TABLE_NAME = :table
             ORDER BY ORDINAL_POSITION",
            params! {
                "schema" => &database,
                "table" => &table,
            },
            |(name, column_type, nullable, key, default_value, extra, comment): (String, String, String, String, Option<String>, String, String)| {
                MysqlColumnInfo {
                    name,
                    column_type,
                    nullable,
                    key,
                    default_value,
                    extra,
                    comment,
                }
            },
        )
        .map_err(|error| error.to_string())?;

    let indexes = conn
        .exec_map(
            "SELECT INDEX_NAME, COLUMN_NAME, NON_UNIQUE, SEQ_IN_INDEX, INDEX_TYPE
             FROM information_schema.STATISTICS
             WHERE TABLE_SCHEMA = :schema AND TABLE_NAME = :table
             ORDER BY INDEX_NAME, SEQ_IN_INDEX",
            params! {
                "schema" => &database,
                "table" => &table,
            },
            |(name, column_name, non_unique, seq_in_index, index_type): (
                String,
                String,
                u8,
                u64,
                String,
            )| {
                MysqlIndexInfo {
                    name,
                    column_name,
                    non_unique,
                    seq_in_index,
                    index_type,
                }
            },
        )
        .map_err(|error| error.to_string())?;

    let create_rows: Vec<Row> = conn
        .query(format!(
            "SHOW CREATE TABLE `{}`.`{}`",
            database.replace('`', "``"),
            table.replace('`', "``")
        ))
        .map_err(|error| error.to_string())?;

    let ddl = create_rows
        .first()
        .and_then(|row| row.as_ref(1))
        .cloned()
        .map(value_to_json)
        .and_then(|value| value.as_str().map(ToString::to_string))
        .unwrap_or_default();

    let foreign_keys = conn
        .exec_map(
            "SELECT
                k.CONSTRAINT_NAME,
                k.COLUMN_NAME,
                k.REFERENCED_TABLE_NAME,
                k.REFERENCED_COLUMN_NAME,
                r.UPDATE_RULE,
                r.DELETE_RULE
             FROM information_schema.KEY_COLUMN_USAGE k
             JOIN information_schema.REFERENTIAL_CONSTRAINTS r
               ON r.CONSTRAINT_SCHEMA = k.CONSTRAINT_SCHEMA
              AND r.CONSTRAINT_NAME = k.CONSTRAINT_NAME
             WHERE k.TABLE_SCHEMA = :schema
               AND k.TABLE_NAME = :table
               AND k.REFERENCED_TABLE_NAME IS NOT NULL
             ORDER BY k.CONSTRAINT_NAME, k.ORDINAL_POSITION",
            params! {
                "schema" => &database,
                "table" => &table,
            },
            |(name, column_name, referenced_table, referenced_column, update_rule, delete_rule): (
                String,
                String,
                String,
                String,
                String,
                String,
            )| MysqlForeignKeyInfo {
                name,
                column_name,
                referenced_table,
                referenced_column,
                update_rule,
                delete_rule,
            },
        )
        .map_err(|error| error.to_string())?;

    let triggers = conn
        .exec_map(
            "SELECT TRIGGER_NAME, ACTION_TIMING, EVENT_MANIPULATION, ACTION_STATEMENT
             FROM information_schema.TRIGGERS
             WHERE TRIGGER_SCHEMA = :schema AND EVENT_OBJECT_TABLE = :table
             ORDER BY TRIGGER_NAME",
            params! {
                "schema" => &database,
                "table" => &table,
            },
            |(name, timing, event, statement): (String, String, String, String)| MysqlTriggerInfo {
                name,
                timing,
                event,
                statement,
            },
        )
        .map_err(|error| error.to_string())?;

    let checks = conn
        .exec_map(
            "SELECT cc.CONSTRAINT_NAME, cc.CHECK_CLAUSE, tc.ENFORCED
             FROM information_schema.CHECK_CONSTRAINTS cc
             JOIN information_schema.TABLE_CONSTRAINTS tc
               ON tc.CONSTRAINT_SCHEMA = cc.CONSTRAINT_SCHEMA
              AND tc.CONSTRAINT_NAME = cc.CONSTRAINT_NAME
             WHERE tc.TABLE_SCHEMA = :schema AND tc.TABLE_NAME = :table
             ORDER BY cc.CONSTRAINT_NAME",
            params! {
                "schema" => &database,
                "table" => &table,
            },
            |(name, expression, enforced): (String, String, String)| MysqlCheckInfo {
                name,
                expression,
                enforced,
            },
        )
        .unwrap_or_default();

    let options = conn
        .exec_first(
            "SELECT ENGINE, TABLE_COLLATION, TABLE_COMMENT
             FROM information_schema.TABLES
             WHERE TABLE_SCHEMA = :schema AND TABLE_NAME = :table",
            params! {
                "schema" => &database,
                "table" => &table,
            },
        )
        .map_err(|error| error.to_string())?
        .map(|(engine, collation, comment): (Option<String>, Option<String>, Option<String>)| MysqlTableOptionsInfo {
            engine: engine.unwrap_or_default(),
            collation: collation.unwrap_or_default(),
            comment: comment.unwrap_or_default(),
        })
        .unwrap_or(MysqlTableOptionsInfo {
            engine: String::new(),
            collation: String::new(),
            comment: String::new(),
        });

    Ok(MysqlTableDetail {
        columns,
        indexes,
        foreign_keys,
        triggers,
        checks,
        options,
        ddl,
    })
}
