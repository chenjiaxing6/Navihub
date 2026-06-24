import { invoke } from "@tauri-apps/api/core";

export function createSqliteTable(config, database, table) {
  return invoke("sqlite_create_table", { config, database, table });
}

export function copySqliteTable(config, database, table, newTable, options = {}) {
  return invoke("sqlite_copy_table", {
    config,
    database,
    table,
    newTable,
    copyData: Boolean(options.copyData),
  });
}

export function renameSqliteTable(config, database, table, newTable) {
  return invoke("sqlite_rename_table", { config, database, table, newTable });
}

export function dropSqliteTable(config, database, table) {
  return invoke("sqlite_drop_table", { config, database, table });
}

export function emptySqliteTable(config, database, table) {
  return invoke("sqlite_empty_table", { config, database, table });
}

export function truncateSqliteTable(config, database, table) {
  return emptySqliteTable(config, database, table);
}

export function exportSqliteTablesSql(config, database, tables, options = {}) {
  return invoke("sqlite_export_tables_sql", {
    config,
    database,
    tables,
    includeData: options.includeData !== false,
  });
}

export function exportSqliteDatabaseSql(config, database, options = {}) {
  return invoke("sqlite_export_database_sql", {
    config,
    database,
    includeData: options.includeData !== false,
  });
}

export function importSqliteSql(config, database, sql) {
  return invoke("sqlite_import_sql", { config, database, sql });
}

export function vacuumSqliteDatabase(config) {
  return invoke("sqlite_vacuum", { config });
}

export function checkSqliteIntegrity(config, options = {}) {
  return invoke("sqlite_integrity_check", { config, quick: Boolean(options.quick) });
}

export function analyzeSqliteDatabase(config) {
  return invoke("sqlite_analyze", { config });
}

export function reindexSqliteDatabase(config) {
  return invoke("sqlite_reindex", { config });
}

export function getSqliteDatabaseInfo(config) {
  return invoke("sqlite_database_info", { config });
}
