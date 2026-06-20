import { invoke } from "@tauri-apps/api/core";

export function testMysqlConnection(config) {
  return invoke("mysql_test_connection", { config });
}

export function loadMysqlSchema(config) {
  return invoke("mysql_load_schema", { config });
}

export function executeMysqlQuery(config, database, sql) {
  return invoke("mysql_execute_query", { config, database, sql });
}

export function describeMysqlTable(config, database, table) {
  return invoke("mysql_describe_table", { config, database, table });
}
