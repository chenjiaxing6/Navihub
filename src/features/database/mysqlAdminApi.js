import { invoke } from "@tauri-apps/api/core";

export function createMysqlDatabase(config, database, options = {}) {
  return invoke("mysql_create_database", {
    config,
    database,
    charset: options.charset || null,
    collation: options.collation || null,
  });
}

export function alterMysqlDatabaseOptions(config, database, options = {}) {
  return invoke("mysql_alter_database_options", {
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

export function copyMysqlTable(config, database, table, newTable, options = {}) {
  return invoke("mysql_copy_table", {
    config,
    database,
    table,
    newTable,
    copyData: Boolean(options.copyData),
  });
}

export function renameMysqlTable(config, database, table, newTable) {
  return invoke("mysql_rename_table", { config, database, table, newTable });
}

export function dropMysqlTable(config, database, table) {
  return invoke("mysql_drop_table", { config, database, table });
}

export function emptyMysqlTable(config, database, table) {
  return invoke("mysql_empty_table", { config, database, table });
}

export function truncateMysqlTable(config, database, table) {
  return invoke("mysql_truncate_table", { config, database, table });
}

export function exportMysqlTablesSql(config, database, tables, options = {}) {
  return invoke("mysql_export_tables_sql", {
    config,
    database,
    tables,
    includeData: options.includeData !== false,
  });
}

export function exportMysqlDatabaseSql(config, database, options = {}) {
  return invoke("mysql_export_database_sql", {
    config,
    database,
    includeData: options.includeData !== false,
  });
}

export function importMysqlSql(config, database, sql) {
  return invoke("mysql_import_sql", { config, database, sql });
}
