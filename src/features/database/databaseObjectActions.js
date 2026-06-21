import { ElMessage } from "element-plus/es/components/message/index";
import { ElMessageBox } from "element-plus/es/components/message-box/index";
import {
  copyMysqlTable,
  dropMysqlDatabase,
  dropMysqlTable,
  renameMysqlTable,
} from "./mysqlAdminApi";

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

export async function runDatabaseObjectAction(payload) {
  if (!payload?.connection?.config) {
    return null;
  }

  const config = payload.connection.config;
  const schemaName = payload.schema?.name ?? payload.schema;
  const tableName = payload.table?.name ?? payload.table;
  const tableNames = Array.isArray(payload.tables) ? [...new Set(payload.tables.filter(Boolean))] : [];

  if (payload.action === "drop-database" && schemaName) {
    await ElMessageBox.confirm(`确认删除库“${schemaName}”？此操作会删除库内所有对象。`, "删除库", confirmOptions);
    await dropMysqlDatabase(config, schemaName);
    ElMessage.success("库已删除");
    return { changed: true, type: "drop-database", database: schemaName };
  }

  if (payload.action === "create-table" && schemaName) {
    return { openDesigner: true, type: "create-table", database: schemaName };
  }

  if (payload.action === "rename-table" && schemaName && tableName) {
    const newTable = await promptName("输入新的表名称", "重命名表", tableName);
    if (!newTable || newTable === tableName) return null;
    await renameMysqlTable(config, schemaName, tableName, newTable);
    ElMessage.success("表已重命名");
    return { changed: true, type: "rename-table", database: schemaName, table: tableName, newTable };
  }

  if ((payload.action === "copy-table-structure" || payload.action === "copy-table-data") && schemaName && tableName) {
    const suffix = payload.action === "copy-table-data" ? "_copy" : "_struct";
    const newTable = await promptName("输入复制后的表名称", "复制表", `${tableName}${suffix}`);
    if (!newTable) return null;
    await copyMysqlTable(config, schemaName, tableName, newTable, {
      copyData: payload.action === "copy-table-data",
    });
    ElMessage.success(payload.action === "copy-table-data" ? "表结构和数据已复制" : "表结构已复制");
    return { changed: true, type: "copy-table", database: schemaName, table: tableName, newTable };
  }

  if (payload.action === "drop-table" && schemaName && tableNames.length > 1) {
    await ElMessageBox.confirm(`确认删除选中的 ${tableNames.length} 张表？`, "删除表", confirmOptions);
    for (const table of tableNames) {
      await dropMysqlTable(config, schemaName, table);
    }
    ElMessage.success("表已删除");
    return { changed: true, type: "drop-table", database: schemaName, tables: tableNames };
  }

  if (payload.action === "drop-table" && schemaName && tableName) {
    await ElMessageBox.confirm(`确认删除表“${schemaName}.${tableName}”？`, "删除表", confirmOptions);
    await dropMysqlTable(config, schemaName, tableName);
    ElMessage.success("表已删除");
    return { changed: true, type: "drop-table", database: schemaName, table: tableName };
  }

  return null;
}
