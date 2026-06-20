<script setup>
import { computed, ref, watch } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import AppShell from "./layouts/AppShell.vue";
import ConnectionDialog from "./features/connections/ConnectionDialog.vue";
import { connections } from "./features/connections/connectionData";
import { createMysqlConnection, formatMysqlMeta, normalizeDatabaseConnection } from "./features/database/databaseDefaults";
import { loadMysqlSchema } from "./features/database/mysqlApi";
import SettingsDialog from "./features/settings/SettingsDialog.vue";
import { defaultTerminalThemeId, getTerminalTheme, terminalThemes } from "./features/settings/terminalThemes";
import { createSshConnection, formatSshMeta, normalizeSshConnection } from "./features/terminal/sshDefaults";

const storageKey = "myhub.connections.v1";
const folderStorageKey = "myhub.connectionFolders.v1";
const settingsStorageKey = "myhub.settings.v1";

function loadConnections() {
  try {
    const stored = JSON.parse(localStorage.getItem(storageKey) || "null");
    const source = Array.isArray(stored) ? stored : connections;
    return source.map(normalizeConnection);
  } catch {
    return connections.map(normalizeConnection);
  }
}

function loadConnectionFolders() {
  try {
    const stored = JSON.parse(localStorage.getItem(folderStorageKey) || "null");
    return normalizeConnectionFolders(stored);
  } catch {
    return normalizeConnectionFolders([]);
  }
}

function loadSettings() {
  try {
    const stored = JSON.parse(localStorage.getItem(settingsStorageKey) || "null");
    const terminalThemeId = terminalThemes.some((theme) => theme.id === stored?.terminalThemeId)
      ? stored.terminalThemeId
      : defaultTerminalThemeId;
    return { terminalThemeId };
  } catch {
    return { terminalThemeId: defaultTerminalThemeId };
  }
}

function normalizeConnectionFolders(folders) {
  if (!Array.isArray(folders)) {
    return [];
  }

  return folders
    .filter((folder) => folder && typeof folder === "object")
    .map((folder, index) => ({
      id: folder.id ?? `folder-${Date.now()}-${index}`,
      workspace: folder.workspace === "ssh" ? "ssh" : "database",
      name: String(folder.name ?? "新建文件夹").trim() || "新建文件夹",
    }));
}

function normalizeConnection(connection, index = 0) {
  if (connection?.workspace === "ssh") {
    return normalizeSshConnection(connection, index);
  }

  return normalizeDatabaseConnection(connection, index);
}

const activeWorkspace = ref("database");
const activeConnectionId = ref("prod");
const activeSchemaConnectionId = ref(null);
const connectionList = ref(loadConnections());
const connectionFolders = ref(loadConnectionFolders());
const pendingTableQuery = ref(null);
const pendingSchemaOpen = ref(null);
const connectionDialogVisible = ref(false);
const settingsDialogVisible = ref(false);
const editingConnection = ref(null);
const pendingConnectionFolderId = ref(null);
const dynamicTabs = ref([]);
const activeTopTabId = ref(null);
const openSchemaKeys = ref([]);
const schemaOpenVersions = ref({});
const appSettings = ref(loadSettings());

const visibleConnections = computed(() =>
  connectionList.value.filter((connection) => connection.workspace === activeWorkspace.value),
);

const visibleConnectionFolders = computed(() =>
  connectionFolders.value.filter((folder) => folder.workspace === activeWorkspace.value),
);

const activeConnection = computed(() =>
  visibleConnections.value.find((connection) => connection.id === activeConnectionId.value) ??
    visibleConnections.value[0] ??
    null,
);

const topTabs = computed(() => dynamicTabs.value);

const activeTopTab = computed(() => topTabs.value.find((tab) => tab.id === activeTopTabId.value) ?? null);
const terminalTheme = computed(() => getTerminalTheme(appSettings.value.terminalThemeId));

watch(
  connectionList,
  (value) => {
    localStorage.setItem(storageKey, JSON.stringify(value));
  },
  { deep: true },
);

watch(
  connectionFolders,
  (value) => {
    localStorage.setItem(folderStorageKey, JSON.stringify(value));
  },
  { deep: true },
);

watch(
  appSettings,
  (value) => {
    localStorage.setItem(settingsStorageKey, JSON.stringify(value));
  },
  { deep: true },
);

function setWorkspace(workspace) {
  activeWorkspace.value = workspace;
  activeTopTabId.value = null;
  activeSchemaConnectionId.value = null;

  const firstConnection = connectionList.value.find((connection) => connection.workspace === workspace);
  if (firstConnection) {
    activeConnectionId.value = firstConnection.id;
  }
}

function selectConnection(connection) {
  activeConnectionId.value = connection.id;
  activeWorkspace.value = connection.workspace;
  if (connection.workspace !== "database") {
    activeTopTabId.value = null;
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

  try {
    const schemas = await loadMysqlSchema(connection.config);
    connectionList.value = connectionList.value.map((item) =>
      item.id === connection.id ? { ...item, status: "connected", schemas } : item,
    );
  } catch (error) {
    connectionList.value = connectionList.value.map((item) =>
      item.id === connection.id ? { ...item, status: "disconnected" } : item,
    );
    activeSchemaConnectionId.value = activeSchemaConnectionId.value === connection.id ? null : activeSchemaConnectionId.value;
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
  openSchemaKeys.value = openSchemaKeys.value.filter((key) => !key.startsWith(`schema:${connection.id}:`));
  closeConnectionTabs(connection.id);
}

async function refreshConnection(connection) {
  if (!connection || connection.workspace !== "database") {
    return;
  }

  try {
    const schemas = await loadMysqlSchema(connection.config);
    connectionList.value = connectionList.value.map((item) =>
      item.id === connection.id ? { ...item, status: "connected", schemas } : item,
    );
    dynamicTabs.value = dynamicTabs.value.map((tab) => {
      if (tab.connectionId !== connection.id || tab.kind !== "schema") {
        return tab;
      }

      const nextSchema = schemas.find((schema) => schema.name === tab.schema.name);
      return nextSchema ? { ...tab, schema: nextSchema } : tab;
    });
    activeWorkspace.value = "database";
    activeConnectionId.value = connection.id;
    activeSchemaConnectionId.value = connection.id;
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
    if (connection.id !== activeConnectionId.value) {
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
  activeConnectionId.value = payload.connectionId;
  activeSchemaConnectionId.value = payload.connectionId;
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
  activeConnectionId.value = connection.id;
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
    `删除“${folder.name}”后，里面的连接会移到未归档。`,
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
  activeConnectionId.value = connection.id;
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
  activeConnectionId.value = editing.id;
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
  const next = connectionList.value.find((item) => item.workspace === activeWorkspace.value);
  if (next) {
    activeConnectionId.value = next.id;
  } else {
    activeConnectionId.value = "";
  }
  activeSchemaConnectionId.value = activeSchemaConnectionId.value === connection.id ? null : activeSchemaConnectionId.value;
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
  activeConnectionId.value = duplicate.id;
  activeSchemaConnectionId.value = null;
}

function openTableQuery(payload) {
  selectConnection(payload.connection);
  activeSchemaConnectionId.value = payload.connection.id;
  const tabKey = `table:${payload.connection.id}:${payload.schema}:${payload.item}`;
  const existing = dynamicTabs.value.find((tab) => tab.key === tabKey);
  if (existing) {
    activeTopTabId.value = existing.id;
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
  activeTopTabId.value = tab.id;
  pendingTableQuery.value = {
    id: Date.now(),
    schema: payload.schema,
    groupType: payload.groupType,
    table: payload.item,
  };
}

function openSchema(payload) {
  selectConnection(payload.connection);
  activeSchemaConnectionId.value = payload.connection.id;
  const tabKey = `schema:${payload.connection.id}:${payload.schema.name}`;
  const isOpen = openSchemaKeys.value.includes(tabKey);
  const existing = dynamicTabs.value.find((tab) => tab.key === tabKey);

  if (isOpen) {
    openSchemaKeys.value = openSchemaKeys.value.filter((key) => key !== tabKey);
    if (existing) {
      dynamicTabs.value = dynamicTabs.value.filter((tab) => tab.id !== existing.id);
      if (activeTopTabId.value === existing.id) {
        activeTopTabId.value = null;
      }
    }
    return;
  }

  if (existing) {
    activeTopTabId.value = existing.id;
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
  activeTopTabId.value = tab.id;
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
    activeTopTabId.value = existing.id;
  }
}

function selectTopTab(tabId) {
  const tab = topTabs.value.find((item) => item.id === tabId);
  if (!tab) {
    return;
  }

  activeTopTabId.value = tab.id;
  activeWorkspace.value = tab.workspace;
  if (tab.connectionId) {
    activeConnectionId.value = tab.connectionId;
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

  if (closingIds.has(activeTopTabId.value)) {
    const nextTab = dynamicTabs.value[dynamicTabs.value.length - 1] ?? null;
    activeTopTabId.value = nextTab?.id ?? null;
    activeWorkspace.value = nextTab?.workspace ?? activeWorkspace.value;
    if (nextTab?.connectionId) {
      activeConnectionId.value = nextTab.connectionId;
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

  if (closingIds.has(activeTopTabId.value)) {
    const nextTab = dynamicTabs.value[dynamicTabs.value.length - 1] ?? null;
    activeTopTabId.value = nextTab?.id ?? null;
    activeWorkspace.value = nextTab?.workspace ?? activeWorkspace.value;
    if (nextTab?.connectionId) {
      activeConnectionId.value = nextTab.connectionId;
    }
  }
}

function closeTopTabs(payload) {
  const index = dynamicTabs.value.findIndex((tab) => tab.id === payload.tabId);
  if (index < 0) {
    return;
  }

  const tabsByScope = {
    all: dynamicTabs.value,
    left: dynamicTabs.value.slice(0, index),
    right: dynamicTabs.value.slice(index + 1),
  };

  closeTabsByIds((tabsByScope[payload.scope] ?? []).map((tab) => tab.id));
}

function closeTopTab(tabId) {
  closeTabsByIds([tabId]);
}
</script>

<template>
  <AppShell
    v-model:active-workspace="activeWorkspace"
    :active-connection-id="activeConnectionId"
    :active-schema-connection-id="activeSchemaConnectionId"
    :active-connection="activeConnection"
    :active-top-tab="activeTopTab"
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
    @select-top-tab="selectTopTab"
    @schema-loaded="handleSchemaLoaded"
    @update-mysql-connection="updateMysqlConnection"
    @update-ssh-state="updateSshState"
  />

  <ConnectionDialog
    v-model="connectionDialogVisible"
    :connection="editingConnection"
    :workspace="activeWorkspace"
    @submit="createConnection"
  />

  <SettingsDialog
    v-model="settingsDialogVisible"
    v-model:terminal-theme-id="appSettings.terminalThemeId"
  />
</template>
