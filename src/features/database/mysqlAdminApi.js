import { invoke } from "@tauri-apps/api/core";

export function createMysqlDatabase(config, database, options = {}) {
  return invoke("mysql_create_database", {
    config,
    database,
    charset: options.charset || null,
    collation: options.collation || null,
  });
}

export function listMysqlDatabaseOptions(config) {
  return invoke("mysql_list_database_options", { config });
}

export function dropMysqlDatabase(config, database) {
  return invoke("mysql_drop_database", { config, database });
}

export function createMysqlTable(config, database, table) {
  return invoke("mysql_create_table", { config, database, table });
}

export function renameMysqlTable(config, database, table, newTable) {
  return invoke("mysql_rename_table", { config, database, table, newTable });
}

export function dropMysqlTable(config, database, table) {
  return invoke("mysql_drop_table", { config, database, table });
}
