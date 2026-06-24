<script setup>
import { computed, ref } from "vue";
import { Folder, FolderOpened, Monitor as TerminalIcon } from "@element-plus/icons-vue";
import ContextMenu from "../../shared/ContextMenu.vue";
import { MYSQL_LOGO_PATH, SQLITE_LOGO_CUT_PATH, SQLITE_LOGO_FEATHER_PATH, SQLITE_LOGO_LINE_PATH } from "../database/databaseLogos";
import SchemaTree from "./SchemaTree.vue";

const props = defineProps({
  connections: { type: Array, required: true },
  folders: { type: Array, default: () => [] },
  activeConnectionId: { type: String, required: true },
  expandedConnectionIds: { type: Array, default: () => [] },
  activeConnection: { type: Object, default: null },
  openSchemaKeys: { type: Array, default: () => [] },
  searchQuery: { type: String, default: "" },
  schemaOpenVersions: { type: Object, default: () => ({}) },
});

const emit = defineEmits([
  "close-connection",
  "close-schema",
  "create-connection",
  "delete-connection",
  "duplicate-connection",
  "edit-connection",
  "activate-schema",
  "refresh-connection",
  "select-connection",
  "toggle-connection-expanded",
  "open-connection",
  "create-query",
  "database-object-action",
  "toggle-schema-pin",
  "open-schema",
  "open-table-query",
  "delete-folder",
  "move-connection-to-folder",
  "rename-folder",
]);

const statusText = {
  connecting: "连接中",
  connected: "已连接",
  disconnected: "未连接",
};

const connectionContextOpen = ref(false);
const connectionContextPosition = ref({ x: 0, y: 0 });
const contextConnection = ref(null);
const folderContextOpen = ref(false);
const folderContextPosition = ref({ x: 0, y: 0 });
const contextFolder = ref(null);
const openFolderIds = ref(new Set());
const normalizedSearch = computed(() => props.searchQuery.trim().toLowerCase());
const groupedConnections = computed(() => {
  const query = normalizedSearch.value;
  const folders = props.folders.map((folder) => {
    const folderMatched = includesQuery(folder.name, query);
    const connections = props.connections.filter((connection) => {
      if (connection.folderId !== folder.id) {
        return false;
      }

      return !query || folderMatched || connectionMatchesSearch(connection);
    });
    return {
      ...folder,
      connections,
      matched: !query || folderMatched || connections.length > 0,
    };
  }).filter((folder) => folder.matched);

  const knownFolderIds = new Set(props.folders.map((folder) => folder.id));
  const uncategorized = props.connections.filter((connection) => {
    const isUncategorized = !connection.folderId || !knownFolderIds.has(connection.folderId);
    return isUncategorized && (!query || connectionMatchesSearch(connection));
  });

  return { folders, uncategorized };
});
const filteredConnectionCount = computed(() =>
  groupedConnections.value.uncategorized.length +
    groupedConnections.value.folders.reduce((total, folder) => total + folder.connections.length, 0),
);
const folderContextItems = computed(() => [
  { key: "create-connection", label: "新建连接" },
  { key: "rename", label: "重命名" },
  { key: "delete", label: "删除文件夹", danger: true, divided: true },
]);
const moveFolderDropdownItems = computed(() => [
  { id: null, name: "顶层" },
  ...props.folders,
]);

const hasVisibleConnections = computed(() => filteredConnectionCount.value > 0);
const hasVisibleContent = computed(() =>
  groupedConnections.value.folders.length > 0 || groupedConnections.value.uncategorized.length > 0,
);

const connectionContextItems = computed(() => {
  const connection = contextConnection.value;
  const connected = connection?.status === "connected";
  const connecting = connection?.status === "connecting";
  const isDatabase = connection?.workspace === "database";
  return [
    {
      key: connected || connecting ? "close" : "open",
      label: connected || connecting ? "关闭连接" : "打开连接",
    },
    { key: "edit", label: "编辑连接...", divided: true },
    { key: "create-connection", label: "新建连接" },
    { key: "duplicate", label: "复制连接..." },
    { key: "delete", label: "删除连接", danger: true },
    { key: "create-database", label: "新建数据库...", divided: true, disabled: !isDatabase || !connected },
    { key: "create-query", label: "新建查询", disabled: !isDatabase || !connected },
    { key: "import-sql", label: "运行 SQL 文件...", divided: true, disabled: !isDatabase || !connected },
    { key: "refresh", label: "刷新", divided: true, disabled: !isDatabase },
  ];
});

function handleConnectionClick(connection) {
  emit("select-connection", connection);
}

function handleConnectionDoubleClick(connection) {
  if (["connected", "connecting"].includes(connection.status)) {
    return;
  }

  emit("open-connection", connection);
}

function canToggleConnection(connection) {
  return connection.workspace === "database" && ["connected", "connecting"].includes(connection.status);
}

function isConnectionExpanded(connection) {
  return canToggleConnection(connection) && props.expandedConnectionIds.includes(connection.id);
}

function openConnectionContextMenu(event, connection) {
  event.preventDefault();
  contextConnection.value = connection;
  connectionContextPosition.value = { x: event.clientX, y: event.clientY };
  connectionContextOpen.value = true;
  emit("select-connection", connection);
}

function handleConnectionContextSelect(item) {
  if (!contextConnection.value) {
    return;
  }

  if (item.key === "open") {
    emit("open-connection", contextConnection.value);
  } else if (item.key === "close") {
    emit("close-connection", contextConnection.value);
  } else if (item.key === "refresh") {
    emit("refresh-connection", contextConnection.value);
  } else if (item.key === "create-database") {
    emit("database-object-action", { connection: contextConnection.value, action: "create-database" });
  } else if (item.key === "create-query") {
    emit("create-query", { connection: contextConnection.value, schema: contextConnection.value.config?.database ?? "" });
  } else if (item.key === "import-sql") {
    emit("database-object-action", {
      connection: contextConnection.value,
      action: "import-sql",
      schema: contextConnection.value.config?.database ?? "",
    });
  } else if (item.key === "edit") {
    emit("edit-connection", contextConnection.value);
  } else if (item.key === "create-connection") {
    emit("create-connection");
  } else if (item.key === "duplicate") {
    emit("duplicate-connection", contextConnection.value);
  } else if (item.key === "delete") {
    emit("delete-connection", contextConnection.value);
  } else if (item.key?.startsWith("move:")) {
    emit("move-connection-to-folder", {
      connection: contextConnection.value,
      folderId: item.key.slice("move:".length) || null,
    });
  }
}

function isFolderOpen(folder) {
  return normalizedSearch.value || openFolderIds.value.has(folder.id);
}

function toggleFolder(folder) {
  const nextIds = new Set(openFolderIds.value);
  if (nextIds.has(folder.id)) {
    nextIds.delete(folder.id);
  } else {
    nextIds.add(folder.id);
  }
  openFolderIds.value = nextIds;
}

function openFolderContextMenu(event, folder) {
  event.preventDefault();
  contextFolder.value = folder;
  folderContextPosition.value = { x: event.clientX, y: event.clientY };
  folderContextOpen.value = true;
}

function handleFolderContextSelect(item) {
  if (!contextFolder.value) {
    return;
  }

  if (item.key === "rename") {
    emit("rename-folder", contextFolder.value);
  } else if (item.key === "delete") {
    emit("delete-folder", contextFolder.value);
  } else if (item.key === "create-connection") {
    emit("create-connection", contextFolder.value);
  }
}

function handleDropdownCommand(command, connection) {
  if (command === "open-connection" || command === "edit-connection" || command === "duplicate-connection" || command === "delete-connection") {
    emit(command, connection);
    return;
  }

  if (String(command).startsWith("move:")) {
    emit("move-connection-to-folder", {
      connection,
      folderId: String(command).slice("move:".length) || null,
    });
  }
}

function connectionMatchesSearch(connection) {
  const query = normalizedSearch.value;
  const searchable = [
    connection.name,
    connection.meta,
    connection.workspace,
    connection.config?.host,
    connection.config?.username,
    connection.config?.database,
    connection.config?.remotePath,
  ];

  if (searchable.some((value) => includesQuery(value, query))) {
    return true;
  }

  return (connection.schemas ?? []).some((schema) =>
    includesQuery(schema.name, query) ||
      (schema.groups ?? []).some((group) =>
        includesQuery(group.title, query) ||
          groupType(group).includes(query) ||
          (group.items ?? []).some((item) => includesQuery(itemName(item), query)),
      ),
  );
}

function includesQuery(value, query) {
  return String(value ?? "").toLowerCase().includes(query);
}

function groupType(group) {
  return group.groupType ?? group.type ?? "";
}

function itemName(item) {
  return typeof item === "string" ? item : item?.name;
}

function schemasForConnection(connection) {
  const schemas = connection.id === props.activeConnection?.id
    ? props.activeConnection.schemas
    : connection.schemas;
  const pinnedOrder = new Map((connection.pinnedSchemas ?? []).map((schemaName, index) => [schemaName, index]));
  return (schemas ?? []).slice().sort((first, second) => {
    const firstPinnedIndex = pinnedOrder.get(first.name);
    const secondPinnedIndex = pinnedOrder.get(second.name);
    const firstPinned = firstPinnedIndex !== undefined;
    const secondPinned = secondPinnedIndex !== undefined;
    if (firstPinned && secondPinned) {
      return firstPinnedIndex - secondPinnedIndex;
    }

    if (firstPinned !== secondPinned) {
      return firstPinned ? -1 : 1;
    }

    return 0;
  });
}
</script>

<template>
  <nav class="connection-explorer" aria-label="连接列表">
    <template v-for="folder in groupedConnections.folders" :key="folder.id">
      <section class="connection-folder">
        <button
          class="folder-row"
          :class="{ open: isFolderOpen(folder) }"
          @click="toggleFolder(folder)"
          @contextmenu.prevent="openFolderContextMenu($event, folder)"
        >
          <el-icon>
            <FolderOpened v-if="isFolderOpen(folder)" />
            <Folder v-else />
          </el-icon>
          <span>{{ folder.name }}</span>
          <em>{{ folder.connections.length }}</em>
        </button>

        <div v-if="isFolderOpen(folder)" class="folder-children">
          <template v-for="connection in folder.connections" :key="connection.id">
            <div class="connection-row">
              <button
                class="connection-item"
                :class="{ active: activeConnectionId === connection.id }"
                @click="handleConnectionClick(connection)"
                @contextmenu.prevent="openConnectionContextMenu($event, connection)"
                @dblclick.stop="handleConnectionDoubleClick(connection)"
              >
                <span class="connection-expand-slot">
                  <button
                    v-if="canToggleConnection(connection)"
                    class="connection-expand-toggle"
                    :class="{ open: isConnectionExpanded(connection) }"
                    :aria-label="isConnectionExpanded(connection) ? '折叠连接' : '展开连接'"
                    @click.stop="emit('toggle-connection-expanded', connection)"
                  />
                </span>
                <span class="node-icon" :class="connection.iconClass">
                  <el-icon v-if="connection.workspace === 'ssh'"><TerminalIcon /></el-icon>
                  <svg
                    v-else-if="connection.iconClass === 'mysql'"
                    class="mysql-node-icon"
                    viewBox="0 0 32 32"
                    aria-hidden="true"
                  >
                    <path :d="MYSQL_LOGO_PATH" />
                  </svg>
                  <svg
                    v-else-if="connection.iconClass === 'sqlite'"
                    class="sqlite-node-icon"
                    viewBox="0 0 96 96"
                    aria-hidden="true"
                  >
                    <path class="sqlite-logo-feather" :d="SQLITE_LOGO_FEATHER_PATH" />
                    <path class="sqlite-logo-cut" :d="SQLITE_LOGO_CUT_PATH" />
                    <path class="sqlite-logo-line" :d="SQLITE_LOGO_LINE_PATH" />
                  </svg>
                  <span v-else class="database-node-icon" aria-hidden="true" />
                </span>
                <span class="connection-copy">
                  <strong>
                    <span>{{ connection.name }}</span>
                  </strong>
                  <small>{{ connection.meta }}</small>
                </span>
                <span class="connection-trailing">
                  <em
                    class="connection-status"
                    :class="{ connected: connection.status === 'connected', connecting: connection.status === 'connecting' }"
                  >
                    <span class="state-dot" />
                    {{ statusText[connection.status] ?? "未连接" }}
                  </em>
                  <el-dropdown
                    trigger="click"
                    popper-class="connection-actions-popper"
                    @command="(command) => handleDropdownCommand(command, connection)"
                  >
                    <button class="connection-menu" @click.stop>⋯</button>
                    <template #dropdown>
                      <el-dropdown-menu>
                        <el-dropdown-item command="open-connection">连接</el-dropdown-item>
                        <el-dropdown-item command="edit-connection">编辑</el-dropdown-item>
                        <el-dropdown-item command="duplicate-connection">复制连接</el-dropdown-item>
                        <el-dropdown-item
                          v-for="targetFolder in moveFolderDropdownItems"
                          :key="targetFolder.id ?? 'none'"
                          :command="`move:${targetFolder.id ?? ''}`"
                          divided
                        >
                          移到 {{ targetFolder.name }}
                        </el-dropdown-item>
                        <el-dropdown-item command="delete-connection" divided>删除连接</el-dropdown-item>
                      </el-dropdown-menu>
                    </template>
                  </el-dropdown>
                </span>
              </button>
            </div>

            <SchemaTree
              v-if="isConnectionExpanded(connection)"
              :connection-id="connection.id"
              :connection-engine="connection.config?.engine ?? 'mysql'"
              :loading="connection.status === 'connecting'"
              :open-schema-keys="openSchemaKeys"
              :pinned-schemas="connection.pinnedSchemas"
              :search-query="searchQuery"
              :schema-open-versions="schemaOpenVersions"
              :schemas="schemasForConnection(connection)"
              @activate-schema="(payload) => emit('activate-schema', { connection, ...payload })"
              @close-schema="(payload) => emit('close-schema', { connection, ...payload })"
              @create-query="(payload) => emit('create-query', { connection, ...payload })"
              @database-object-action="(payload) => emit('database-object-action', { connection, ...payload })"
              @open-schema="(payload) => emit('open-schema', { connection, ...payload })"
              @open-table-query="(payload) => emit('open-table-query', { connection, ...payload })"
              @refresh-connection="emit('refresh-connection', connection)"
              @toggle-schema-pin="(payload) => emit('toggle-schema-pin', { connection, ...payload })"
            />
          </template>
          <div v-if="folder.connections.length === 0" class="folder-empty">空文件夹</div>
        </div>
      </section>
    </template>

    <template v-for="connection in groupedConnections.uncategorized" :key="connection.id">
      <div class="connection-row">
        <button
          class="connection-item"
          :class="{ active: activeConnectionId === connection.id }"
          @click="handleConnectionClick(connection)"
          @contextmenu.prevent="openConnectionContextMenu($event, connection)"
          @dblclick.stop="handleConnectionDoubleClick(connection)"
        >
          <span class="connection-expand-slot">
            <button
              v-if="canToggleConnection(connection)"
              class="connection-expand-toggle"
              :class="{ open: isConnectionExpanded(connection) }"
              :aria-label="isConnectionExpanded(connection) ? '折叠连接' : '展开连接'"
              @click.stop="emit('toggle-connection-expanded', connection)"
            />
          </span>
          <span class="node-icon" :class="connection.iconClass">
            <el-icon v-if="connection.workspace === 'ssh'"><TerminalIcon /></el-icon>
            <svg
              v-else-if="connection.iconClass === 'mysql'"
              class="mysql-node-icon"
              viewBox="0 0 32 32"
              aria-hidden="true"
            >
              <path :d="MYSQL_LOGO_PATH" />
            </svg>
            <svg
              v-else-if="connection.iconClass === 'sqlite'"
              class="sqlite-node-icon"
              viewBox="0 0 96 96"
              aria-hidden="true"
            >
              <path class="sqlite-logo-feather" :d="SQLITE_LOGO_FEATHER_PATH" />
              <path class="sqlite-logo-cut" :d="SQLITE_LOGO_CUT_PATH" />
              <path class="sqlite-logo-line" :d="SQLITE_LOGO_LINE_PATH" />
            </svg>
            <span v-else class="database-node-icon" aria-hidden="true" />
          </span>
          <span class="connection-copy">
            <strong>
              <span>{{ connection.name }}</span>
            </strong>
            <small>{{ connection.meta }}</small>
          </span>
          <span class="connection-trailing">
            <em
              class="connection-status"
              :class="{ connected: connection.status === 'connected', connecting: connection.status === 'connecting' }"
            >
              <span class="state-dot" />
              {{ statusText[connection.status] ?? "未连接" }}
            </em>
            <el-dropdown
              trigger="click"
              popper-class="connection-actions-popper"
              @command="(command) => handleDropdownCommand(command, connection)"
            >
              <button class="connection-menu" @click.stop>⋯</button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item command="open-connection">连接</el-dropdown-item>
                  <el-dropdown-item command="edit-connection">编辑</el-dropdown-item>
                  <el-dropdown-item command="duplicate-connection">复制连接</el-dropdown-item>
                  <el-dropdown-item
                    v-for="targetFolder in moveFolderDropdownItems"
                    :key="targetFolder.id ?? 'none'"
                    :command="`move:${targetFolder.id ?? ''}`"
                    divided
                  >
                    移到 {{ targetFolder.name }}
                  </el-dropdown-item>
                  <el-dropdown-item command="delete-connection" divided>删除连接</el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </span>
        </button>
      </div>

      <SchemaTree
        v-if="isConnectionExpanded(connection)"
        :connection-id="connection.id"
        :connection-engine="connection.config?.engine ?? 'mysql'"
        :loading="connection.status === 'connecting'"
        :open-schema-keys="openSchemaKeys"
        :pinned-schemas="connection.pinnedSchemas"
        :search-query="searchQuery"
        :schema-open-versions="schemaOpenVersions"
        :schemas="schemasForConnection(connection)"
        @activate-schema="(payload) => emit('activate-schema', { connection, ...payload })"
        @close-schema="(payload) => emit('close-schema', { connection, ...payload })"
        @create-query="(payload) => emit('create-query', { connection, ...payload })"
        @database-object-action="(payload) => emit('database-object-action', { connection, ...payload })"
        @open-schema="(payload) => emit('open-schema', { connection, ...payload })"
        @open-table-query="(payload) => emit('open-table-query', { connection, ...payload })"
        @refresh-connection="emit('refresh-connection', connection)"
        @toggle-schema-pin="(payload) => emit('toggle-schema-pin', { connection, ...payload })"
      />
    </template>

    <div v-if="!hasVisibleContent || !hasVisibleConnections" class="connection-empty">
      没有匹配的连接
    </div>

    <ContextMenu
      v-model="connectionContextOpen"
      :items="connectionContextItems"
      :x="connectionContextPosition.x"
      :y="connectionContextPosition.y"
      @select="handleConnectionContextSelect"
    />
    <ContextMenu
      v-model="folderContextOpen"
      :items="folderContextItems"
      :x="folderContextPosition.x"
      :y="folderContextPosition.y"
      @select="handleFolderContextSelect"
    />
  </nav>
</template>

<style scoped>
.connection-explorer {
  display: block;
  min-height: 0;
  flex: 1;
  overflow: auto;
  padding: 8px;
}

.connection-row {
  display: block;
  align-items: center;
  width: 100%;
}

.connection-empty {
  padding: 14px 8px;
  color: var(--faint);
  font-size: 12px;
}

.connection-folder {
  display: block;
}

.folder-row {
  display: grid;
  grid-template-columns: 18px 1fr auto;
  align-items: center;
  gap: 7px;
  width: 100%;
  min-height: 32px;
  padding: 5px 8px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  text-align: left;
}

.folder-row:hover {
  background: var(--surface-strong);
  color: var(--text);
}

.folder-row span {
  min-width: 0;
  overflow: hidden;
  font-size: 12px;
  font-weight: 760;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.folder-row em {
  color: var(--faint);
  font-size: 11px;
  font-style: normal;
  font-weight: 720;
}

.folder-children {
  padding-left: 12px;
}

.folder-empty {
  padding: 8px 8px 10px;
  color: var(--faint);
  font-size: 12px;
}

.connection-item {
  display: grid;
  grid-template-columns: 16px 22px 1fr auto;
  gap: 6px;
  align-items: center;
  width: 100%;
  min-height: 44px;
  padding: 6px 8px;
  border: 1px solid transparent;
  border-radius: 9px;
  background: transparent;
  color: var(--text);
  cursor: pointer;
  text-align: left;
}

.connection-expand-slot {
  display: grid;
  place-items: center;
  width: 16px;
  height: 100%;
}

.connection-expand-toggle {
  position: relative;
  display: grid;
  place-items: center;
  width: 16px;
  height: 16px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--faint);
  cursor: pointer;
  appearance: none;
}

.connection-expand-toggle:hover {
  background: var(--surface-strong);
  color: var(--muted);
}

.connection-expand-toggle::before {
  width: 0;
  height: 0;
  border-top: 4px solid transparent;
  border-bottom: 4px solid transparent;
  border-left: 5px solid currentColor;
  content: "";
  transition: transform 0.12s ease;
}

.connection-expand-toggle.open::before {
  transform: rotate(90deg);
}

.connection-item .node-icon.mysql,
.connection-item .node-icon.sqlite,
.connection-item .node-icon.postgres {
  overflow: visible;
  border: 0;
  border-radius: 0;
  background: transparent;
  color: #0b5d8e;
}

.connection-item .node-icon.postgres {
  border: 1px solid #b9c3d2;
  border-radius: 7px;
  background: #edf1f6;
  color: #475569;
}

.mysql-node-icon {
  width: 23px;
  height: 23px;
  margin-left: -2px;
  fill: #00758f;
}

.sqlite-node-icon {
  width: 24px;
  height: 24px;
  margin-left: -2px;
}

.sqlite-node-icon .sqlite-logo-feather {
  fill: #0b80bd;
}

.sqlite-node-icon .sqlite-logo-cut {
  fill: #78c7e8;
  opacity: 0.95;
}

.sqlite-node-icon .sqlite-logo-line {
  fill: none;
  stroke: #f5fbff;
  stroke-linecap: round;
  stroke-width: 4;
}

.database-node-icon {
  position: relative;
  display: block;
  width: 13px;
  height: 14px;
  border: 1.5px solid currentColor;
  border-top: 0;
  border-radius: 0 0 5px 5px;
  background:
    linear-gradient(to bottom, transparent 4px, currentColor 4px, currentColor 5.5px, transparent 5.5px),
    linear-gradient(to bottom, transparent 8px, currentColor 8px, currentColor 9.5px, transparent 9.5px);
}

.database-node-icon::before {
  position: absolute;
  top: -4px;
  left: -1.5px;
  width: 13px;
  height: 7px;
  border: 1.5px solid currentColor;
  border-radius: 50%;
  background: #f7fbff;
  content: "";
}

.node-icon.postgres .database-node-icon::before {
  background: #f8fafc;
}

.node-icon.sqlite {
  color: #2563eb;
}

.node-icon.sqlite .database-node-icon::before {
  background: #eff6ff;
}

.connection-menu {
  width: 26px;
  height: 26px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--faint);
  cursor: pointer;
}

.connection-menu:hover {
  background: var(--surface-strong);
  color: var(--muted);
}

.connection-menu:active {
  background: #e8e9ec;
  transform: none;
}

:global(.connection-actions-popper.el-popper) {
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.98);
  box-shadow: 0 12px 28px rgba(24, 27, 35, 0.12), 0 2px 6px rgba(24, 27, 35, 0.06);
  backdrop-filter: saturate(160%) blur(14px);
}

:global(.connection-actions-popper .el-dropdown-menu) {
  min-width: 154px;
  padding: 4px;
  border-radius: 8px;
}

:global(.connection-actions-popper .el-dropdown-menu__item) {
  min-height: 28px;
  padding: 0 10px;
  border-radius: 6px;
  color: var(--muted);
  font-size: 12px;
  line-height: 28px;
}

:global(.connection-actions-popper .el-dropdown-menu__item:not(.is-disabled):focus),
:global(.connection-actions-popper .el-dropdown-menu__item:not(.is-disabled):hover) {
  background: var(--surface-strong);
  color: var(--text);
}

:global(.connection-actions-popper .el-dropdown-menu__item--divided) {
  margin-top: 4px;
  border-top: 1px solid var(--line);
}

:global(.connection-actions-popper .el-dropdown-menu__item--divided::before) {
  display: none;
}

:global(.connection-actions-popper .el-dropdown-menu__item.is-disabled) {
  color: var(--faint);
}

:global(.connection-actions-popper .el-dropdown-menu__item:last-child:not(.is-disabled)) {
  color: var(--red);
}

:global(.connection-actions-popper .el-dropdown-menu__item:last-child:not(.is-disabled):hover),
:global(.connection-actions-popper .el-dropdown-menu__item:last-child:not(.is-disabled):focus) {
  background: #fef2f2;
  color: var(--red);
}

.connection-item:hover {
  background: var(--surface-strong);
}

.connection-item:active {
  background: #e8e9ec;
  transform: none;
}

.connection-item.active {
  border-color: #ffd6c8;
  background: var(--orange-soft);
  box-shadow: none;
}

.connection-row:has(+ .schema-tree) .connection-item {
  border-color: #ffd6c8;
  background: var(--orange-soft);
  box-shadow: none;
}

.state-dot {
  width: 8px;
  height: 8px;
  flex: 0 0 8px;
  border-radius: 50%;
  background: #a7b0bd;
}

.connection-status.connected .state-dot {
  background: var(--green);
}

.connection-status.connecting .state-dot {
  background: var(--orange);
}

.connection-item strong,
.connection-item small {
  display: block;
}

.connection-copy {
  min-width: 0;
}

.connection-item strong {
  display: flex;
  align-items: center;
  gap: 8px;
  overflow: hidden;
  font-size: 13px;
  font-weight: 760;
}

.connection-item strong > span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.connection-status {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--faint);
  font-size: 11px;
  font-style: normal;
  font-weight: 650;
  white-space: nowrap;
}

.connection-trailing {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 6px;
  min-width: 82px;
}

.connection-status.connected {
  color: var(--green);
}

.connection-status.connecting {
  color: var(--orange);
}

.connection-item small {
  margin-top: 3px;
  overflow: hidden;
  color: var(--muted);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
