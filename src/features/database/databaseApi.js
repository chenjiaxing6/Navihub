import { describeMysqlTable, executeMysqlQuery, loadMysqlSchema, testMysqlConnection } from "./mysqlApi";
import { describeSqliteTable, executeSqliteQuery, loadSqliteSchema, testSqliteConnection } from "./sqliteApi";

export function databaseEngine(connectionOrConfig) {
  return connectionOrConfig?.engine ?? connectionOrConfig?.config?.engine ?? "mysql";
}

export function isSqliteConnection(connectionOrConfig) {
  return databaseEngine(connectionOrConfig) === "sqlite";
}

export function testDatabaseConnection(connectionOrConfig) {
  const config = connectionOrConfig?.config ?? connectionOrConfig;
  return isSqliteConnection(connectionOrConfig)
    ? testSqliteConnection(config)
    : testMysqlConnection(config);
}

export function loadDatabaseSchema(connectionOrConfig) {
  const config = connectionOrConfig?.config ?? connectionOrConfig;
  return isSqliteConnection(connectionOrConfig)
    ? loadSqliteSchema(config)
    : loadMysqlSchema(config);
}

export function executeDatabaseQuery(connectionOrConfig, database, sql) {
  const config = connectionOrConfig?.config ?? connectionOrConfig;
  return isSqliteConnection(connectionOrConfig)
    ? executeSqliteQuery(config, database, sql)
    : executeMysqlQuery(config, database, sql);
}

export function describeDatabaseTable(connectionOrConfig, database, table) {
  const config = connectionOrConfig?.config ?? connectionOrConfig;
  return isSqliteConnection(connectionOrConfig)
    ? describeSqliteTable(config, database, table)
    : describeMysqlTable(config, database, table);
}
