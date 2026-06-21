import { ElMessage } from "element-plus/es/components/message/index";
import { ElMessageBox } from "element-plus/es/components/message-box/index";
import {
  createMysqlTable,
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

  if (payload.action === "drop-database" && schemaName) {
    await ElMessageBox.confirm(`确认删除库“${schemaName}”？此操作会删除库内所有对象。`, "删除库", confirmOptions);
    await dropMysqlDatabase(config, schemaName);
    ElMessage.success("库已删除");
    return { changed: true, type: "drop-database", database: schemaName };
  }

  if (payload.action === "create-table" && schemaName) {
    const table = await promptName("输入表名称，将创建一个带 id 主键的空表", "新建表");
    if (!table) return null;
    await createMysqlTable(config, schemaName, table);
    ElMessage.success("表已创建");
    return { changed: true, type: "create-table", database: schemaName, table };
  }

  if (payload.action === "rename-table" && schemaName && tableName) {
    const newTable = await promptName("输入新的表名称", "重命名表", tableName);
    if (!newTable || newTable === tableName) return null;
    await renameMysqlTable(config, schemaName, tableName, newTable);
    ElMessage.success("表已重命名");
    return { changed: true, type: "rename-table", database: schemaName, table: tableName, newTable };
  }

  if (payload.action === "drop-table" && schemaName && tableName) {
    await ElMessageBox.confirm(`确认删除表“${schemaName}.${tableName}”？`, "删除表", confirmOptions);
    await dropMysqlTable(config, schemaName, tableName);
    ElMessage.success("表已删除");
    return { changed: true, type: "drop-table", database: schemaName, table: tableName };
  }

  return null;
}
