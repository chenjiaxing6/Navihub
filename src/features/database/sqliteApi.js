import { invoke } from "@tauri-apps/api/core";

export function testSqliteConnection(config) {
  return invoke("sqlite_test_connection", { config });
}

export function loadSqliteSchema(config) {
  return invoke("sqlite_load_schema", { config });
}

export function executeSqliteQuery(config, database, sql) {
  return invoke("sqlite_execute_query", { config, database, sql });
}

export function describeSqliteTable(config, database, table) {
  return invoke("sqlite_describe_table", { config, database, table });
}
