<script setup>
import { computed, ref } from "vue";
import { ElMessage } from "element-plus/es/components/message/index";
import { ElMessageBox } from "element-plus/es/components/message-box/index";
import zhCn from "element-plus/es/locale/lang/zh-cn";
import AppShell from "./layouts/AppShell.vue";
import ConnectionDialog from "./features/connections/ConnectionDialog.vue";
import DatabaseCreateDialog from "./features/database/DatabaseCreateDialog.vue";
import { createMysqlConnection, formatMysqlMeta } from "./features/database/databaseDefaults";
import { runDatabaseObjectAction } from "./features/database/databaseObjectActions";
import { createMysqlDatabase } from "./features/database/mysqlAdminApi";
import { loadMysqlSchema } from "./features/database/mysqlApi";
import SettingsDialog from "./features/settings/SettingsDialog.vue";
import { useAppConnections } from "./shared/useAppConnections";
import { createSshConnection, formatSshMeta } from "./features/terminal/sshDefaults";

const {
  activeConnection,
  activeConnectionId,
  activeConnectionIds,
  activeWorkspace,
  appSettings,
  connectionFolders,
  connectionList,
  databaseConnection,
  setWorkspaceActiveConnectionId,
  sshConnection,
  terminalTheme,
  visibleConnectionFolders,
  visibleConnections,
} = useAppConnections();
const activeSchemaConnectionId = ref(null);
const pendingTableQuery = ref(null);
const pendingSchemaOpen = ref(null);
const connectionDialogVisible = ref(false);
const databaseCreateDialogVisible = ref(false);
const databaseCreateLoading = ref(false);
const settingsDialogVisible = ref(false);
const editingConnection = ref(null);
const pendingDatabaseCreatePayload = ref(null);
const pendingConnectionFolderId = ref(null);
const dynamicTabs = ref([]);
const expandedConnectionIds = ref([]);
const activeTopTabIds = ref({
  database: null,
});
const openSchemaKeys = ref([]);
const schemaOpenVersions = ref({});

const topTabs = computed(() => dynamicTabs.value.filter((tab) => tab.workspace === activeWorkspace.value));

const activeTopTab = computed(() => topTabs.value.find((tab) => tab.id === activeTopTabId.value) ?? null);
const activeTopTabId = computed({
  get() {
    return activeTopTabIds.value[activeWorkspace.value] ?? null;
  },
  set(tabId) {
    setWorkspaceActiveTopTabId(activeWorkspace.value, tabId);
  },
});
const databaseActiveTopTab = computed(() =>
  dynamicTabs.value.find((tab) => tab.workspace === "database" && tab.id === activeTopTabIds.value.database) ?? null,
);

function mergeSavedQueriesIntoSchemas(schemas = [], savedQueries = []) {
  if (!Array.isArray(schemas) || !Array.isArray(savedQueries) || savedQueries.length === 0) {
    return schemas;
  }

  return schemas.map((schema) => {
    const schemaQueries = savedQueries.filter((query) => query.schema === schema.name);
    if (schemaQueries.length === 0) {
      return schema;
    }

    const groups = Array.isArray(schema.groups) ? [...schema.groups] : [];
    const queryGroupIndex = groups.findIndex((group) => (group.groupType ?? group.type) === "query");
    const queryItems = schemaQueries
      .slice()
      .sort((first, second) => first.name.localeCompare(second.name, "zh-CN"))
      .map((query) => ({
        id: query.id,
        name: query.name,
        sql: query.sql,
        schema: query.schema,
        type: "query",
      }));
    const queryGroup = {
      groupType: "query",
      type: "query",
      title: "查询",
      count: queryItems.length,
      items: queryItems,
    };

    if (queryGroupIndex >= 0) {
      groups[queryGroupIndex] = queryGroup;
    } else {
      groups.splice(Math.min(2, groups.length), 0, queryGroup);
    }

    return { ...schema, groups };
  });
}

function updateConnectionSchemas(connectionId, schemas) {
  connectionList.value = connectionList.value.map((item) =>
    item.id === connectionId
      ? { ...item, status: "connected", schemas: mergeSavedQueriesIntoSchemas(schemas, item.savedQueries) }
      : item,
  );
}

function toggleSchemaPin(payload) {
  const schemaName = String(payload?.schema?.name ?? "").trim();
  const connectionId = payload?.connection?.id;
  if (!connectionId || !schemaName) {
    return;
  }

  let pinned = false;
  connectionList.value = connectionList.value.map((connection) => {
    if (connection.id !== connectionId) {
      return connection;
    }

    const pinnedSchemas = Array.isArray(connection.pinnedSchemas) ? connection.pinnedSchemas : [];
    pinned = !pinnedSchemas.includes(schemaName);
    const nextPinnedSchemas = pinned
      ? [...pinnedSchemas, schemaName]
      : pinnedSchemas.filter((item) => item !== schemaName);
    return {
      ...connection,
      pinnedSchemas: nextPinnedSchemas,
    };
  });
  ElMessage.success(pinned ? "数据库已置顶" : "数据库已取消置顶");
}

function setWorkspaceActiveTopTabId(workspace, tabId) {
  activeTopTabIds.value = {
    ...activeTopTabIds.value,
    [workspace]: tabId,
  };
}

function getLastWorkspaceTab(workspace) {
  const workspaceTabs = dynamicTabs.value.filter((tab) => tab.workspace === workspace);
  return workspaceTabs[workspaceTabs.length - 1] ?? null;
}

function setWorkspace(workspace) {
  activeWorkspace.value = workspace;

  if (!activeConnectionIds.value[workspace]) {
    const firstConnection = connectionList.value.find((connection) => connection.workspace === workspace);
    if (firstConnection) {
      setWorkspaceActiveConnectionId(workspace, firstConnection.id);
    }
  }
}

function selectConnection(connection) {
  activeWorkspace.value = connection.workspace;
  setWorkspaceActiveConnectionId(connection.workspace, connection.id);
}

function expandConnection(connectionId) {
  if (!connectionId || expandedConnectionIds.value.includes(connectionId)) {
    return;
  }

  expandedConnectionIds.value = [...expandedConnectionIds.value, connectionId];
}

function collapseConnection(connectionId) {
  expandedConnectionIds.value = expandedConnectionIds.value.filter((id) => id !== connectionId);
}

function toggleConnectionExpanded(connection) {
  if (!connection || connection.workspace !== "database" || !["connected", "connecting"].includes(connection.status)) {
    return;
  }

  if (expandedConnectionIds.value.includes(connection.id)) {
    collapseConnection(connection.id);
  } else {
    expandConnection(connection.id);
  }
}

async function openConnection(connection) {
  selectConnection(connection);

  if (connection.workspace !== "database") {
    connectionList.value = connectionList.value.map((item) =>
      item.id === connection.id
        ? { ...item, status: "connecting", connectVersion: (item.connectVersion ?? 0) + 1 }
        : item,
    );
    activeSchemaConnectionId.value = null;
    return;
  }

  connectionList.value = connectionList.value.map((item) =>
    item.id === connection.id ? { ...item, status: "connecting" } : item,
  );
  activeSchemaConnectionId.value = connection.id;
  expandConnection(connection.id);
  openSchemaKeys.value = openSchemaKeys.value.filter((key) => !key.startsWith(`schema:${connection.id}:`));

  try {
    const schemas = await loadMysqlSchema(connection.config);
    updateConnectionSchemas(connection.id, schemas);
  } catch (error) {
    connectionList.value = connectionList.value.map((item) =>
      item.id === connection.id ? { ...item, status: "disconnected" } : item,
    );
    activeSchemaConnectionId.value = activeSchemaConnectionId.value === connection.id ? null : activeSchemaConnectionId.value;
    collapseConnection(connection.id);
    ElMessage.error(`连接失败：${error}`);
  }
}

function closeConnection(connection) {
  if (!connection) {
    return;
  }

  if (connection.workspace === "ssh") {
    connectionList.value = connectionList.value.map((item) =>
      item.id === connection.id ? { ...item, status: "disconnected" } : item,
    );
    return;
  }

  if (connection.workspace !== "database") {
    return;
  }

  connectionList.value = connectionList.value.map((item) =>
    item.id === connection.id ? { ...item, status: "disconnected" } : item,
  );
  activeSchemaConnectionId.value = activeSchemaConnectionId.value === connection.id ? null : activeSchemaConnectionId.value;
  collapseConnection(connection.id);
  openSchemaKeys.value = openSchemaKeys.value.filter((key) => !key.startsWith(`schema:${connection.id}:`));
  closeConnectionTabs(connection.id);
}

async function refreshConnection(connection) {
  if (!connection || connection.workspace !== "database") {
    return;
  }

  try {
    const schemas = await loadMysqlSchema(connection.config);
    updateConnectionSchemas(connection.id, schemas);
    const latestConnection = connectionList.value.find((item) => item.id === connection.id) ?? connection;
    const savedQueries = latestConnection.savedQueries ?? connection.savedQueries;
    const mergedSchemas = mergeSavedQueriesIntoSchemas(schemas, savedQueries);
    dynamicTabs.value = dynamicTabs.value.map((tab) => {
      if (tab.connectionId !== connection.id || tab.kind !== "schema") {
        return tab;
      }

      const nextSchema = mergedSchemas.find((schema) => schema.name === tab.schema.name);
      return nextSchema ? { ...tab, schema: nextSchema } : tab;
    });
    activeWorkspace.value = "database";
    setWorkspaceActiveConnectionId("database", connection.id);
    activeSchemaConnectionId.value = connection.id;
    expandConnection(connection.id);
    const nextSchemaOpenVersions = { ...schemaOpenVersions.value };
    for (const key of openSchemaKeys.value.filter((item) => item.startsWith(`schema:${connection.id}:`))) {
      nextSchemaOpenVersions[key] = (nextSchemaOpenVersions[key] ?? 0) + 1;
    }
    schemaOpenVersions.value = nextSchemaOpenVersions;
    ElMessage.success("连接结构已刷新");
  } catch (error) {
    ElMessage.error(`刷新连接失败：${error}`);
  }
}

function updateMysqlConnection(payload) {
  connectionList.value = connectionList.value.map((connection) => {
    if (connection.id !== activeConnectionIds.value.database) {
      return connection;
    }

    const nextConfig = payload.config ?? connection.config;
    return {
      ...connection,
      ...payload,
      meta: payload.meta ?? formatMysqlMeta(nextConfig),
      config: nextConfig,
    };
  });

}

function updateSshState(payload) {
  if (!payload.connection) {
    return;
  }

  connectionList.value = connectionList.value.map((connection) =>
    connection.id === payload.connection.id ? { ...connection, status: payload.status } : connection,
  );
}

function handleSchemaLoaded(payload) {
  activeWorkspace.value = "database";
  setWorkspaceActiveConnectionId("database", payload.connectionId);
  activeSchemaConnectionId.value = payload.connectionId;
  expandConnection(payload.connectionId);
  connectionList.value = connectionList.value.map((connection) =>
    connection.id === payload.connectionId ? { ...connection, status: "connected" } : connection,
  );
}

function openCreateConnection(folder = null) {
  editingConnection.value = null;
  pendingConnectionFolderId.value = folder?.id ?? null;
  if (folder?.workspace) {
    activeWorkspace.value = folder.workspace;
  }
  connectionDialogVisible.value = true;
}

function createConnection(payload) {
  if (editingConnection.value) {
    updateConnection(payload);
    return;
  }

  const connection = payload.workspace === "ssh"
    ? createSshConnection({ ...payload, folderId: pendingConnectionFolderId.value })
    : createMysqlConnection({ ...payload, folderId: pendingConnectionFolderId.value });

  connectionList.value = [...connectionList.value, connection];
  activeWorkspace.value = connection.workspace;
  setWorkspaceActiveConnectionId(connection.workspace, connection.id);
  activeSchemaConnectionId.value = null;
  pendingConnectionFolderId.value = null;
  connectionDialogVisible.value = false;
}

async function createConnectionFolder(workspace = activeWorkspace.value) {
  try {
    const { value } = await ElMessageBox.prompt("输入文件夹名称", "新建文件夹", {
      confirmButtonText: "创建",
      cancelButtonText: "取消",
      inputPlaceholder: "例如：生产环境",
      inputPattern: /\S+/,
      inputErrorMessage: "名称不能为空",
      customClass: "bruno-message-box folder-prompt-box",
    });
    const name = String(value ?? "").trim();
    if (!name) {
      return;
    }

    const folder = {
      id: `folder-${Date.now()}-${Math.random().toString(16).slice(2)}`,
      workspace,
      name,
    };
    connectionFolders.value = [...connectionFolders.value, folder];
    activeWorkspace.value = workspace;
    ElMessage.success("文件夹已创建");
  } catch {
    // 用户取消
  }
}

async function renameConnectionFolder(folder) {
  if (!folder) {
    return;
  }

  try {
    const { value } = await ElMessageBox.prompt("输入新的文件夹名称", "重命名文件夹", {
      confirmButtonText: "保存",
      cancelButtonText: "取消",
      inputValue: folder.name,
      inputPattern: /\S+/,
      inputErrorMessage: "名称不能为空",
      customClass: "bruno-message-box folder-prompt-box",
    });
    const name = String(value ?? "").trim();
    if (!name) {
      return;
    }

    connectionFolders.value = connectionFolders.value.map((item) =>
      item.id === folder.id ? { ...item, name } : item,
    );
    ElMessage.success("文件夹已重命名");
  } catch {
    // 用户取消
  }
}

async function deleteConnectionFolder(folder) {
  if (!folder) {
    return;
  }

  await ElMessageBox.confirm(
    `删除“${folder.name}”后，里面的连接会移到顶层。`,
    "删除文件夹",
    {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning",
      customClass: "bruno-message-box",
      dangerouslyUseHTMLString: false,
    },
  );

  connectionFolders.value = connectionFolders.value.filter((item) => item.id !== folder.id);
  connectionList.value = connectionList.value.map((connection) =>
    connection.folderId === folder.id ? { ...connection, folderId: null } : connection,
  );
  ElMessage.success("文件夹已删除");
}

function moveConnectionToFolder(payload) {
  if (!payload?.connection) {
    return;
  }

  const folder = payload.folderId
    ? connectionFolders.value.find((item) => item.id === payload.folderId && item.workspace === payload.connection.workspace)
    : null;
  connectionList.value = connectionList.value.map((connection) =>
    connection.id === payload.connection.id ? { ...connection, folderId: folder?.id ?? null } : connection,
  );
}

function openEditConnection(connection) {
  editingConnection.value = {
    ...connection,
    config: { ...(connection.config ?? {}) },
  };
  activeWorkspace.value = connection.workspace;
  setWorkspaceActiveConnectionId(connection.workspace, connection.id);
  connectionDialogVisible.value = true;
}

function updateConnection(payload) {
  const editing = editingConnection.value;
  if (!editing) {
    return;
  }

  const nextConfig = payload.config ?? editing.config;
  connectionList.value = connectionList.value.map((connection) => {
    if (connection.id !== editing.id) {
      return connection;
    }

    return {
      ...connection,
      name: payload.name,
      config: nextConfig,
      meta: connection.workspace === "ssh" ? formatSshMeta(nextConfig) : formatMysqlMeta(nextConfig),
      schemas: connection.workspace === "database" ? connection.schemas : undefined,
    };
  });
  activeWorkspace.value = editing.workspace;
  setWorkspaceActiveConnectionId(editing.workspace, editing.id);
  editingConnection.value = null;
  pendingConnectionFolderId.value = null;
  connectionDialogVisible.value = false;
  ElMessage.success("连接已更新");
}

async function deleteConnection(connection) {
  if (!connection) {
    return;
  }

  await ElMessageBox.confirm(
    `此操作会从连接列表中移除“${connection.name}”。`,
    "删除连接",
    {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning",
      customClass: "bruno-message-box",
      dangerouslyUseHTMLString: false,
    },
  );

  connectionList.value = connectionList.value.filter((item) => item.id !== connection.id);
  const next = connectionList.value.find((item) => item.workspace === connection.workspace);
  setWorkspaceActiveConnectionId(connection.workspace, next?.id ?? "");
  activeSchemaConnectionId.value = activeSchemaConnectionId.value === connection.id ? null : activeSchemaConnectionId.value;
  collapseConnection(connection.id);
  closeConnectionTabs(connection.id);
}

function duplicateConnection(connection) {
  const createConnectionByWorkspace = connection.workspace === "ssh" ? createSshConnection : createMysqlConnection;
  const duplicate = createConnectionByWorkspace({
    ...connection,
    id: `${connection.workspace}-${Date.now()}`,
    name: `${connection.name} copy`,
    schemas: connection.workspace === "database" ? [] : undefined,
    status: "disconnected",
    folderId: connection.folderId ?? null,
  });

  connectionList.value = [...connectionList.value, duplicate];
  setWorkspaceActiveConnectionId(duplicate.workspace, duplicate.id);
  activeSchemaConnectionId.value = null;
  collapseConnection(duplicate.id);
}

function openTableQuery(payload) {
  selectConnection(payload.connection);
  activeSchemaConnectionId.value = payload.connection.id;
  expandConnection(payload.connection.id);
  if (payload.groupType === "query") {
    openSavedQuery(payload);
    return;
  }

  const tabKey = `table:${payload.connection.id}:${payload.schema}:${payload.item}`;
  const existing = dynamicTabs.value.find((tab) => tab.key === tabKey);
  if (existing) {
    setWorkspaceActiveTopTabId("database", existing.id);
    return;
  }

  const tab = {
    id: `table-${Date.now()}-${Math.random().toString(16).slice(2)}`,
    key: tabKey,
    label: payload.item,
    workspace: "database",
    closable: true,
    kind: "table",
    connectionId: payload.connection.id,
    schema: payload.schema,
    table: payload.item,
  };
  dynamicTabs.value = [...dynamicTabs.value, tab];
  setWorkspaceActiveTopTabId("database", tab.id);
  pendingTableQuery.value = {
    id: Date.now(),
    schema: payload.schema,
    groupType: payload.groupType,
    table: payload.item,
  };
}

function createQuery(payload) {
  selectConnection(payload.connection);
  activeSchemaConnectionId.value = payload.connection.id;
  expandConnection(payload.connection.id);
  const schemaName = payload.schema?.name ?? payload.schema;
  const createdAt = Date.now();
  const tab = {
    id: `query-${createdAt}-${Math.random().toString(16).slice(2)}`,
    key: `query:${payload.connection.id}:${schemaName}:${createdAt}`,
    label: "新建查询",
    workspace: "database",
    closable: true,
    kind: "query",
    connectionId: payload.connection.id,
    schema: schemaName,
    sql: "",
    savedQueryId: null,
  };
  dynamicTabs.value = [...dynamicTabs.value, tab];
  setWorkspaceActiveTopTabId("database", tab.id);
}

function openSavedQuery(payload) {
  selectConnection(payload.connection);
  activeSchemaConnectionId.value = payload.connection.id;
  expandConnection(payload.connection.id);
  const schemaName = payload.schema?.name ?? payload.schema;
  const query = typeof payload.item === "object"
    ? payload.item
    : payload.connection.savedQueries?.find((item) => item.schema === schemaName && item.name === payload.item);
  if (!query) {
    ElMessage.warning("未找到查询");
    return;
  }

  const tabKey = `query:${payload.connection.id}:${schemaName}:${query.id}`;
  const existing = dynamicTabs.value.find((tab) => tab.key === tabKey);
  if (existing) {
    setWorkspaceActiveTopTabId("database", existing.id);
    return;
  }

  const tab = {
    id: `query-${Date.now()}-${Math.random().toString(16).slice(2)}`,
    key: tabKey,
    label: query.name,
    workspace: "database",
    closable: true,
    kind: "query",
    connectionId: payload.connection.id,
    schema: schemaName,
    sql: query.sql ?? "",
    savedQueryId: query.id,
  };
  dynamicTabs.value = [...dynamicTabs.value, tab];
  setWorkspaceActiveTopTabId("database", tab.id);
}

function openTableDesigner(payload) {
  if (!payload?.connection) {
    return;
  }

  selectConnection(payload.connection);
  activeSchemaConnectionId.value = payload.connection.id;
  expandConnection(payload.connection.id);
  const schemaName = payload.schema?.name ?? payload.schema;
  const tableName = payload.table?.name ?? payload.table ?? "";
  const mode = payload.mode ?? (tableName ? "edit" : "create");
  const tabKey = mode === "edit"
    ? `table-design:${payload.connection.id}:${schemaName}:${tableName}`
    : `table-design:${payload.connection.id}:${schemaName}:new`;
  const existing = dynamicTabs.value.find((tab) => tab.key === tabKey);
  if (existing) {
    setWorkspaceActiveTopTabId("database", existing.id);
    return;
  }

  const tab = {
    id: `table-design-${Date.now()}-${Math.random().toString(16).slice(2)}`,
    key: tabKey,
    label: mode === "edit" ? `设计 ${tableName}` : "新建表",
    workspace: "database",
    closable: true,
    kind: "table-design",
    connectionId: payload.connection.id,
    schema: schemaName,
    table: tableName,
    mode,
  };
  dynamicTabs.value = [...dynamicTabs.value, tab];
  setWorkspaceActiveTopTabId("database", tab.id);
}

function updateQuerySchema(payload) {
  dynamicTabs.value = dynamicTabs.value.map((tab) =>
    tab.id === payload.tabId && tab.kind === "query"
      ? { ...tab, schema: payload.schema }
      : tab,
  );
}

function saveQuery(payload) {
  const savedAt = Date.now();
  const queryId = payload.queryId ?? `saved-query-${savedAt}-${Math.random().toString(16).slice(2)}`;
  let savedQuery = null;

  connectionList.value = connectionList.value.map((connection) => {
    if (connection.id !== payload.connectionId) {
      return connection;
    }

    const existingQueries = Array.isArray(connection.savedQueries) ? connection.savedQueries : [];
    const existing = existingQueries.find((query) => query.id === queryId);
    savedQuery = {
      id: queryId,
      schema: payload.schema,
      name: payload.name,
      sql: payload.sql,
      createdAt: existing?.createdAt ?? savedAt,
      updatedAt: savedAt,
    };
    const savedQueries = existing
      ? existingQueries.map((query) => query.id === queryId ? savedQuery : query)
      : [...existingQueries, savedQuery];

    return {
      ...connection,
      savedQueries,
      schemas: mergeSavedQueriesIntoSchemas(connection.schemas, savedQueries),
    };
  });

  dynamicTabs.value = dynamicTabs.value.map((tab) =>
    tab.id === payload.tabId && tab.kind === "query"
      ? {
        ...tab,
        key: `query:${payload.connectionId}:${payload.schema}:${queryId}`,
        label: payload.name,
        schema: payload.schema,
        sql: payload.sql,
        savedQueryId: queryId,
      }
      : tab,
  );
  ElMessage.success("查询已保存");
}

function openSchema(payload) {
  selectConnection(payload.connection);
  activeSchemaConnectionId.value = payload.connection.id;
  expandConnection(payload.connection.id);
  const tabKey = `schema:${payload.connection.id}:${payload.schema.name}`;
  const isOpen = openSchemaKeys.value.includes(tabKey);
  const existing = dynamicTabs.value.find((tab) => tab.key === tabKey);

  if (isOpen) {
    openSchemaKeys.value = openSchemaKeys.value.filter((key) => key !== tabKey);
    if (existing) {
      dynamicTabs.value = dynamicTabs.value.filter((tab) => tab.id !== existing.id);
      if (activeTopTabIds.value.database === existing.id) {
        setWorkspaceActiveTopTabId("database", null);
      }
    }
    return;
  }

  if (existing) {
    setWorkspaceActiveTopTabId("database", existing.id);
    openSchemaKeys.value = [...openSchemaKeys.value, tabKey];
    schemaOpenVersions.value = {
      ...schemaOpenVersions.value,
      [tabKey]: (schemaOpenVersions.value[tabKey] ?? 0) + 1,
    };
    return;
  }

  const tab = {
    id: `schema-${Date.now()}-${Math.random().toString(16).slice(2)}`,
    key: tabKey,
    label: `${payload.schema.name} · 表`,
    workspace: "database",
    closable: true,
    kind: "schema",
    connectionId: payload.connection.id,
    schema: payload.schema,
  };
  dynamicTabs.value = [...dynamicTabs.value, tab];
  openSchemaKeys.value = [...openSchemaKeys.value, tabKey];
  schemaOpenVersions.value = {
    ...schemaOpenVersions.value,
    [tabKey]: (schemaOpenVersions.value[tabKey] ?? 0) + 1,
  };
  setWorkspaceActiveTopTabId("database", tab.id);
  pendingSchemaOpen.value = {
    id: Date.now(),
    schema: payload.schema,
  };
}

function activateSchema(payload) {
  const tabKey = `schema:${payload.connection.id}:${payload.schema.name}`;
  const existing = dynamicTabs.value.find((tab) => tab.key === tabKey);
  if (existing) {
    selectConnection(payload.connection);
    activeSchemaConnectionId.value = payload.connection.id;
    expandConnection(payload.connection.id);
    setWorkspaceActiveTopTabId("database", existing.id);
  }
}

function selectTopTab(tabId) {
  const tab = topTabs.value.find((item) => item.id === tabId);
  if (!tab) {
    return;
  }

  activeWorkspace.value = tab.workspace;
  setWorkspaceActiveTopTabId(tab.workspace, tab.id);
  if (tab.connectionId) {
    setWorkspaceActiveConnectionId(tab.workspace, tab.connectionId);
  }
}

function closeConnectionTabs(connectionId) {
  const closingIds = new Set(
    dynamicTabs.value
      .filter((tab) => tab.connectionId === connectionId)
      .map((tab) => tab.id),
  );

  if (closingIds.size === 0) {
    return;
  }

  dynamicTabs.value = dynamicTabs.value.filter((tab) => !closingIds.has(tab.id));
  openSchemaKeys.value = openSchemaKeys.value.filter((key) => !key.startsWith(`schema:${connectionId}:`));
  collapseConnection(connectionId);

  for (const workspace of Object.keys(activeTopTabIds.value)) {
    if (closingIds.has(activeTopTabIds.value[workspace])) {
      const nextTab = getLastWorkspaceTab(workspace);
      setWorkspaceActiveTopTabId(workspace, nextTab?.id ?? null);
      if (nextTab?.connectionId) {
        setWorkspaceActiveConnectionId(workspace, nextTab.connectionId);
      }
    }
  }
}

function closeTabsByIds(tabIds) {
  const closingIds = new Set(tabIds);
  if (closingIds.size === 0) {
    return;
  }

  const closingTabs = dynamicTabs.value.filter((tab) => closingIds.has(tab.id));
  dynamicTabs.value = dynamicTabs.value.filter((tab) => !closingIds.has(tab.id));
  openSchemaKeys.value = openSchemaKeys.value.filter((key) =>
    !closingTabs.some((tab) => tab.kind === "schema" && tab.key === key),
  );

  for (const workspace of Object.keys(activeTopTabIds.value)) {
    if (closingIds.has(activeTopTabIds.value[workspace])) {
      const nextTab = getLastWorkspaceTab(workspace);
      setWorkspaceActiveTopTabId(workspace, nextTab?.id ?? null);
      if (nextTab?.connectionId) {
        setWorkspaceActiveConnectionId(workspace, nextTab.connectionId);
      }
    }
  }
}

function closeTopTabs(payload) {
  const scopedTabs = dynamicTabs.value.filter((tab) => tab.workspace === activeWorkspace.value);
  const index = scopedTabs.findIndex((tab) => tab.id === payload.tabId);
  if (index < 0) {
    return;
  }

  const tabsByScope = {
    all: scopedTabs,
    left: scopedTabs.slice(0, index),
    right: scopedTabs.slice(index + 1),
  };

  closeTabsByIds((tabsByScope[payload.scope] ?? []).map((tab) => tab.id));
}

function closeTopTab(tabId) {
  closeTabsByIds([tabId]);
}

function renameTableTabs(connectionId, database, table, newTable) {
  dynamicTabs.value = dynamicTabs.value.map((tab) => {
    if (tab.connectionId !== connectionId || !["table", "table-design"].includes(tab.kind) || tab.schema !== database || tab.table !== table) {
      return tab;
    }

    return {
      ...tab,
      key: tab.kind === "table"
        ? `table:${connectionId}:${database}:${newTable}`
        : `table-design:${connectionId}:${database}:${newTable}`,
      label: tab.kind === "table" ? newTable : `设计 ${newTable}`,
      table: newTable,
    };
  });
}

function handleTableDesignSaved(payload) {
  dynamicTabs.value = dynamicTabs.value.map((tab) => {
    if (tab.id === payload.tabId && tab.kind === "table-design") {
      return {
        ...tab,
        key: `table-design:${payload.connectionId}:${payload.database}:${payload.newTable}`,
        label: `设计 ${payload.newTable}`,
        table: payload.newTable,
        mode: "edit",
      };
    }

    if (
      !payload.wasCreate &&
      tab.connectionId === payload.connectionId &&
      tab.kind === "table" &&
      tab.schema === payload.database &&
      tab.table === payload.table &&
      payload.table !== payload.newTable
    ) {
      return {
        ...tab,
        key: `table:${payload.connectionId}:${payload.database}:${payload.newTable}`,
        label: payload.newTable,
        table: payload.newTable,
      };
    }

    return tab;
  });
}

function closeDroppedObjectTabs(connectionId, result) {
  if (result.type === "drop-database") {
    closeTabsByIds(dynamicTabs.value
      .filter((tab) => tab.connectionId === connectionId && tab.schema === result.database)
      .map((tab) => tab.id));
    return;
  }

  if (result.type !== "drop-table") {
    return;
  }

  const droppedTables = new Set(result.tables ?? [result.table].filter(Boolean));
  closeTabsByIds(dynamicTabs.value
    .filter((tab) => tab.connectionId === connectionId && ["table", "table-design"].includes(tab.kind) && tab.schema === result.database && droppedTables.has(tab.table))
    .map((tab) => tab.id));
}

async function handleDatabaseObjectAction(payload) {
  if (payload?.action === "create-database") {
    pendingDatabaseCreatePayload.value = payload;
    databaseCreateDialogVisible.value = true;
    return;
  }

  if (payload?.action === "design-table") {
    openTableDesigner({ ...payload, mode: "edit" });
    return;
  }

  try {
    const result = await runDatabaseObjectAction(payload);
    if (!result?.changed) {
      if (result?.openDesigner) {
        openTableDesigner({ ...payload, mode: "create" });
      }
      return;
    }

    if (result.type === "rename-table") {
      renameTableTabs(payload.connection.id, result.database, result.table, result.newTable);
    } else {
      closeDroppedObjectTabs(payload.connection.id, result);
    }

    await refreshConnection(payload.connection);
  } catch (error) {
    if (error === "cancel" || error === "close") {
      return;
    }
    ElMessage.error(`操作失败：${error}`);
  }
}

async function handleCreateDatabaseSubmit(form) {
  const payload = pendingDatabaseCreatePayload.value;
  const database = String(form?.database ?? "").trim();
  if (!payload?.connection?.config || !database) {
    ElMessage.warning("请输入数据库名称");
    return;
  }

  databaseCreateLoading.value = true;
  try {
    await createMysqlDatabase(payload.connection.config, database, {
      charset: form.charset,
      collation: form.collation,
    });
    ElMessage.success("库已创建");
    databaseCreateDialogVisible.value = false;
    pendingDatabaseCreatePayload.value = null;
    await refreshConnection(payload.connection);
  } catch (error) {
    ElMessage.error(`创建数据库失败：${error}`);
  } finally {
    databaseCreateLoading.value = false;
  }
}
</script>

<template>
  <el-config-provider :locale="zhCn">
    <AppShell
      v-model:active-workspace="activeWorkspace"
      :active-connection-id="activeConnectionId"
      :active-schema-connection-id="activeSchemaConnectionId"
      :expanded-connection-ids="expandedConnectionIds"
      :active-connection="activeConnection"
      :database-connection="databaseConnection"
      :ssh-connection="sshConnection"
      :active-top-tab="activeTopTab"
      :database-active-top-tab="databaseActiveTopTab"
      :active-top-tab-id="activeTopTabId"
      :open-schema-keys="openSchemaKeys"
      :schema-open-versions="schemaOpenVersions"
      :pending-schema-open="pendingSchemaOpen"
      :pending-table-query="pendingTableQuery"
      :top-tabs="topTabs"
      :terminal-theme="terminalTheme"
      :visible-connections="visibleConnections"
      :visible-connection-folders="visibleConnectionFolders"
      @activate-schema="activateSchema"
      @close-connection="closeConnection"
      @close-top-tab="closeTopTab"
      @close-top-tabs="closeTopTabs"
      @create-connection="openCreateConnection"
      @create-connection-folder="createConnectionFolder"
      @create-query="createQuery"
      @database-object-action="handleDatabaseObjectAction"
      @delete-connection="deleteConnection"
      @delete-connection-folder="deleteConnectionFolder"
      @duplicate-connection="duplicateConnection"
      @edit-connection="openEditConnection"
      @set-workspace="setWorkspace"
      @select-connection="selectConnection"
      @open-connection="openConnection"
      @open-schema="openSchema"
      @open-settings="settingsDialogVisible = true"
      @open-table-query="openTableQuery"
      @move-connection-to-folder="moveConnectionToFolder"
      @rename-connection-folder="renameConnectionFolder"
      @refresh-connection="refreshConnection"
      @save-query="saveQuery"
      @select-top-tab="selectTopTab"
      @schema-loaded="handleSchemaLoaded"
      @table-design-saved="handleTableDesignSaved"
      @toggle-connection-expanded="toggleConnectionExpanded"
      @toggle-schema-pin="toggleSchemaPin"
      @update-mysql-connection="updateMysqlConnection"
      @update-query-schema="updateQuerySchema"
      @update-ssh-state="updateSshState"
    />

    <ConnectionDialog
      v-model="connectionDialogVisible"
      :connection="editingConnection"
      :workspace="activeWorkspace"
      @submit="createConnection"
    />

    <DatabaseCreateDialog
      v-model="databaseCreateDialogVisible"
      :config="pendingDatabaseCreatePayload?.connection?.config"
      :loading="databaseCreateLoading"
      @submit="handleCreateDatabaseSubmit"
    />

    <SettingsDialog
      v-model="settingsDialogVisible"
      v-model:terminal-theme-id="appSettings.terminalThemeId"
    />
  </el-config-provider>
</template>
