import { ElMessage } from "element-plus/es/components/message/index";
import { ElMessageBox } from "element-plus/es/components/message-box/index";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import {
  copyMysqlTable,
  dropMysqlDatabase,
  dropMysqlTable,
  emptyMysqlTable,
  exportMysqlDatabaseSql,
  exportMysqlTablesSql,
  importMysqlSql,
  renameMysqlTable,
  truncateMysqlTable,
} from "./mysqlAdminApi";
import {
  analyzeSqliteDatabase,
  checkSqliteIntegrity,
  copySqliteTable,
  dropSqliteTable,
  emptySqliteTable,
  exportSqliteDatabaseSql,
  exportSqliteTablesSql,
  getSqliteDatabaseInfo,
  importSqliteSql,
  reindexSqliteDatabase,
  renameSqliteTable,
  truncateSqliteTable,
  vacuumSqliteDatabase,
} from "./sqliteAdminApi";

const promptOptions = {
  confirmButtonText: "确认",
  cancelButtonText: "取消",
  inputPattern: /\S+/,
  inputErrorMessage: "名称不能为空",
  customClass: "bruno-message-box folder-prompt-box",
};

const confirmOptions = {
  confirmButtonText: "删除",
  cancelButtonText: "取消",
  type: "warning",
  customClass: "bruno-message-box",
  dangerouslyUseHTMLString: false,
};

const tableDataActionMeta = {
  "empty-table": {
    verb: "清空",
    message: "表数据已清空",
    mysql: emptyMysqlTable,
    sqlite: emptySqliteTable,
  },
  "truncate-table": {
    verb: "截断",
    message: "表已截断",
    mysql: truncateMysqlTable,
    sqlite: truncateSqliteTable,
  },
};

const adminByEngine = {
  mysql: {
    copyTable: copyMysqlTable,
    dropDatabase: dropMysqlDatabase,
    dropTable: dropMysqlTable,
    exportDatabaseSql: exportMysqlDatabaseSql,
    exportTablesSql: exportMysqlTablesSql,
    importSql: importMysqlSql,
    renameTable: renameMysqlTable,
  },
  sqlite: {
    copyTable: copySqliteTable,
    dropDatabase: null,
    dropTable: dropSqliteTable,
    exportDatabaseSql: exportSqliteDatabaseSql,
    exportTablesSql: exportSqliteTablesSql,
    importSql: importSqliteSql,
    renameTable: renameSqliteTable,
  },
};

function trimName(value) {
  return String(value ?? "").trim();
}

async function promptName(message, title, inputValue = "") {
  const { value } = await ElMessageBox.prompt(message, title, {
    ...promptOptions,
    inputValue,
  });
  return trimName(value);
}

function safeFileName(value) {
  return String(value ?? "export")
    .trim()
    .replace(/[\\/:*?"<>|]+/g, "_")
    .replace(/\s+/g, "_") || "export";
}

async function chooseSqlExportPath(database, tables) {
  const tablePart = tables.length === 1 ? tables[0] : `${tables.length}_tables`;
  const path = await save({
    title: "导出表为 SQL",
    defaultPath: `${safeFileName(database)}_${safeFileName(tablePart)}.sql`,
    canCreateDirectories: true,
    filters: [{ name: "SQL", extensions: ["sql"] }],
  });
  return typeof path === "string" ? path : "";
}

async function chooseDatabaseSqlExportPath(database) {
  const path = await save({
    title: "导出库为 SQL",
    defaultPath: `${safeFileName(database)}.sql`,
    canCreateDirectories: true,
    filters: [{ name: "SQL", extensions: ["sql"] }],
  });
  return typeof path === "string" ? path : "";
}

async function chooseSqlImportPath() {
  const path = await open({
    title: "导入 SQL 文件",
    multiple: false,
    filters: [{ name: "SQL", extensions: ["sql"] }],
  });
  return typeof path === "string" ? path : "";
}

export async function runDatabaseObjectAction(payload) {
  if (!payload?.connection?.config) {
    return null;
  }

  const config = payload.connection.config;
  const engine = config.engine === "sqlite" ? "sqlite" : "mysql";
  const admin = adminByEngine[engine];
  const schemaName = payload.schema?.name ?? payload.schema;
  const tableName = payload.table?.name ?? payload.table;
  const tableNames = Array.isArray(payload.tables) ? [...new Set(payload.tables.filter(Boolean))] : [];

  if (payload.action === "import-sql" && schemaName) {
    const path = await chooseSqlImportPath();
    if (!path) return null;
    const sql = await readTextFile(path);
    await ElMessageBox.confirm(`确认将 SQL 文件导入到库“${schemaName}”？`, "导入 SQL", {
      confirmButtonText: "导入",
      cancelButtonText: "取消",
      type: "warning",
      customClass: "bruno-message-box",
      dangerouslyUseHTMLString: false,
    });
    await admin.importSql(config, schemaName, sql);
    ElMessage.success("SQL 已导入");
    return { changed: true, type: "import-sql", database: schemaName };
  }

  if (payload.action === "export-database-sql" && schemaName) {
    const path = await chooseDatabaseSqlExportPath(schemaName);
    if (!path) return null;
    const sql = await admin.exportDatabaseSql(config, schemaName, { includeData: true });
    await writeTextFile(path, sql);
    ElMessage.success("库已导出");
    return { changed: false, type: "export-database-sql", database: schemaName };
  }

  if (payload.action === "drop-database" && schemaName) {
    await ElMessageBox.confirm(`确认删除库“${schemaName}”？此操作会删除库内所有对象。`, "删除库", confirmOptions);
    if (!admin.dropDatabase) {
      ElMessage.warning("SQLite 不支持删除数据库，请删除连接或数据库文件");
      return null;
    }
    await admin.dropDatabase(config, schemaName);
    ElMessage.success("库已删除");
    return { changed: true, type: "drop-database", database: schemaName };
  }

  if (payload.action === "create-table" && schemaName) {
    return { openDesigner: true, type: "create-table", database: schemaName };
  }

  if (payload.action === "rename-table" && schemaName && tableName) {
    const newTable = await promptName("输入新的表名称", "重命名表", tableName);
    if (!newTable || newTable === tableName) return null;
    await admin.renameTable(config, schemaName, tableName, newTable);
    ElMessage.success("表已重命名");
    return { changed: true, type: "rename-table", database: schemaName, table: tableName, newTable };
  }

  if ((payload.action === "copy-table-structure" || payload.action === "copy-table-data") && schemaName && tableName) {
    const suffix = payload.action === "copy-table-data" ? "_copy" : "_struct";
    const newTable = await promptName("输入复制后的表名称", "复制表", `${tableName}${suffix}`);
    if (!newTable) return null;
    await admin.copyTable(config, schemaName, tableName, newTable, {
      copyData: payload.action === "copy-table-data",
    });
    ElMessage.success(payload.action === "copy-table-data" ? "表结构和数据已复制" : "表结构已复制");
    return { changed: true, type: "copy-table", database: schemaName, table: tableName, newTable };
  }

  if (payload.action === "export-table-sql" && schemaName && (tableName || tableNames.length > 0)) {
    const targetTables = tableNames.length > 0 ? tableNames : [tableName];
    const path = await chooseSqlExportPath(schemaName, targetTables);
    if (!path) return null;
    const sql = await admin.exportTablesSql(config, schemaName, targetTables, { includeData: true });
    await writeTextFile(path, sql);
    ElMessage.success(targetTables.length > 1 ? `已导出 ${targetTables.length} 张表` : "表已导出");
    return {
      changed: false,
      type: "export-table-sql",
      database: schemaName,
      table: targetTables.length === 1 ? targetTables[0] : undefined,
      tables: targetTables.length > 1 ? targetTables : undefined,
    };
  }

  if (tableDataActionMeta[payload.action] && schemaName && (tableName || tableNames.length > 0)) {
    const meta = tableDataActionMeta[payload.action];
    const targetTables = tableNames.length > 0 ? tableNames : [tableName];
    const confirmMessage = targetTables.length > 1
      ? `确认${meta.verb}选中的 ${targetTables.length} 张表？此操作会删除表内所有数据。`
      : `确认${meta.verb}表“${schemaName}.${targetTables[0]}”？此操作会删除表内所有数据。`;
    await ElMessageBox.confirm(confirmMessage, `${meta.verb}表`, confirmOptions);
    for (const table of targetTables) {
      await meta[engine](config, schemaName, table);
    }
    ElMessage.success(meta.message);
    return {
      changed: true,
      type: payload.action,
      database: schemaName,
      table: targetTables.length === 1 ? targetTables[0] : undefined,
      tables: targetTables.length > 1 ? targetTables : undefined,
    };
  }

  if (payload.action === "drop-table" && schemaName && tableNames.length > 1) {
    await ElMessageBox.confirm(`确认删除选中的 ${tableNames.length} 张表？`, "删除表", confirmOptions);
    for (const table of tableNames) {
      await admin.dropTable(config, schemaName, table);
    }
    ElMessage.success("表已删除");
    return { changed: true, type: "drop-table", database: schemaName, tables: tableNames };
  }

  if (payload.action === "drop-table" && schemaName && tableName) {
    await ElMessageBox.confirm(`确认删除表“${schemaName}.${tableName}”？`, "删除表", confirmOptions);
    await admin.dropTable(config, schemaName, tableName);
    ElMessage.success("表已删除");
    return { changed: true, type: "drop-table", database: schemaName, table: tableName };
  }

  if (engine === "sqlite" && payload.action === "sqlite-vacuum") {
    const result = await vacuumSqliteDatabase(config);
    ElMessage.success(result.message || "VACUUM 已完成");
    return { changed: true, type: "sqlite-vacuum", database: schemaName };
  }

  if (engine === "sqlite" && payload.action === "sqlite-integrity-check") {
    const result = await checkSqliteIntegrity(config, { quick: false });
    ElMessage[result.ok ? "success" : "warning"](result.message || "检查完成");
    return { changed: false, type: "sqlite-integrity-check", database: schemaName };
  }

  if (engine === "sqlite" && payload.action === "sqlite-quick-check") {
    const result = await checkSqliteIntegrity(config, { quick: true });
    ElMessage[result.ok ? "success" : "warning"](result.message || "检查完成");
    return { changed: false, type: "sqlite-quick-check", database: schemaName };
  }

  if (engine === "sqlite" && payload.action === "sqlite-analyze") {
    const result = await analyzeSqliteDatabase(config);
    ElMessage.success(result.message || "ANALYZE 已完成");
    return { changed: false, type: "sqlite-analyze", database: schemaName };
  }

  if (engine === "sqlite" && payload.action === "sqlite-reindex") {
    const result = await reindexSqliteDatabase(config);
    ElMessage.success(result.message || "REINDEX 已完成");
    return { changed: false, type: "sqlite-reindex", database: schemaName };
  }

  if (engine === "sqlite" && payload.action === "sqlite-database-info") {
    const info = await getSqliteDatabaseInfo(config);
    ElMessage.info(`SQLite ${Math.round((info.size ?? 0) / 1024)} KB · page ${info.pageSize} × ${info.pageCount}`);
    return { changed: false, type: "sqlite-database-info", database: schemaName };
  }

  return null;
}
