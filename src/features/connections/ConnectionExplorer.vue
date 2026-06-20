<script setup>
import { computed, ref } from "vue";
import { Folder, FolderOpened, Monitor as TerminalIcon } from "@element-plus/icons-vue";
import ContextMenu from "../../shared/ContextMenu.vue";
import SchemaTree from "./SchemaTree.vue";

const props = defineProps({
  connections: { type: Array, required: true },
  folders: { type: Array, default: () => [] },
  activeConnectionId: { type: String, required: true },
  activeSchemaConnectionId: { type: String, default: null },
  activeConnection: { type: Object, default: null },
  openSchemaKeys: { type: Array, default: () => [] },
  searchQuery: { type: String, default: "" },
  schemaOpenVersions: { type: Object, default: () => ({}) },
});

const emit = defineEmits([
  "close-connection",
  "create-connection",
  "delete-connection",
  "duplicate-connection",
  "edit-connection",
  "activate-schema",
  "refresh-connection",
  "select-connection",
  "open-connection",
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
  { id: null, name: "未归档" },
  ...props.folders,
]);
const ungroupedFolder = {
  id: "__ungrouped",
  name: "未归档",
};

const hasVisibleConnections = computed(() => filteredConnectionCount.value > 0);
const hasVisibleContent = computed(() =>
  groupedConnections.value.folders.length > 0 || groupedConnections.value.uncategorized.length > 0,
);

const connectionContextItems = computed(() => {
  const connection = contextConnection.value;
  const connected = connection?.status === "connected";
  const connecting = connection?.status === "connecting";
  return [
    { key: "open", label: "连接", disabled: connected || connecting },
    { key: "close", label: "断开", disabled: !connected && !connecting },
    { key: "refresh", label: "刷新", disabled: connection?.workspace !== "database" },
    { key: "edit", label: "编辑", divided: true },
    { key: "duplicate", label: "复制连接" },
    { key: "move:none", label: "移到未归档", disabled: !connection?.folderId, divided: true },
    ...props.folders.map((folder) => ({
      key: `move:${folder.id}`,
      label: `移到 ${folder.name}`,
      disabled: connection?.folderId === folder.id,
    })),
    { key: "delete", label: "删除", danger: true, divided: props.folders.length === 0 },
  ];
});

function handleConnectionClick(connection) {
  emit("select-connection", connection);
}

function handleConnectionDoubleClick(connection) {
  emit("open-connection", connection);
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
  } else if (item.key === "edit") {
    emit("edit-connection", contextConnection.value);
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
                <span class="node-icon" :class="connection.iconClass">
                  <el-icon v-if="connection.workspace === 'ssh'"><TerminalIcon /></el-icon>
                  <template v-else>{{ connection.iconText }}</template>
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
                  <el-dropdown trigger="click" @command="(command) => handleDropdownCommand(command, connection)">
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
              v-if="connection.workspace === 'database' && ['connected', 'connecting'].includes(connection.status) && activeSchemaConnectionId === connection.id"
              :connection-id="connection.id"
              :loading="connection.status === 'connecting'"
              :open-schema-keys="openSchemaKeys"
              :search-query="searchQuery"
              :schema-open-versions="schemaOpenVersions"
              :schemas="connection.id === activeConnection?.id ? activeConnection.schemas : connection.schemas"
              @activate-schema="(payload) => emit('activate-schema', { connection, ...payload })"
              @open-schema="(payload) => emit('open-schema', { connection, ...payload })"
              @open-table-query="(payload) => emit('open-table-query', { connection, ...payload })"
            />
          </template>
          <div v-if="folder.connections.length === 0" class="folder-empty">空文件夹</div>
        </div>
      </section>
    </template>

    <section v-if="groupedConnections.uncategorized.length > 0 || (!normalizedSearch && props.folders.length > 0)" class="connection-folder">
      <button class="folder-row open static" type="button">
        <el-icon><FolderOpened /></el-icon>
        <span>{{ ungroupedFolder.name }}</span>
        <em>{{ groupedConnections.uncategorized.length }}</em>
      </button>
      <div class="folder-children">
        <template v-for="connection in groupedConnections.uncategorized" :key="connection.id">
      <div class="connection-row">
        <button
          class="connection-item"
          :class="{ active: activeConnectionId === connection.id }"
          @click="handleConnectionClick(connection)"
          @contextmenu.prevent="openConnectionContextMenu($event, connection)"
          @dblclick.stop="handleConnectionDoubleClick(connection)"
        >
          <span class="node-icon" :class="connection.iconClass">
            <el-icon v-if="connection.workspace === 'ssh'"><TerminalIcon /></el-icon>
            <template v-else>{{ connection.iconText }}</template>
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
          <el-dropdown trigger="click" @command="(command) => handleDropdownCommand(command, connection)">
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
        v-if="connection.workspace === 'database' && ['connected', 'connecting'].includes(connection.status) && activeSchemaConnectionId === connection.id"
        :connection-id="connection.id"
        :loading="connection.status === 'connecting'"
        :open-schema-keys="openSchemaKeys"
        :search-query="searchQuery"
        :schema-open-versions="schemaOpenVersions"
        :schemas="connection.id === activeConnection?.id ? activeConnection.schemas : connection.schemas"
        @activate-schema="(payload) => emit('activate-schema', { connection, ...payload })"
        @open-schema="(payload) => emit('open-schema', { connection, ...payload })"
          @open-table-query="(payload) => emit('open-table-query', { connection, ...payload })"
        />
        </template>
      </div>
    </section>

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

.folder-row.static {
  cursor: default;
}

.folder-row.static:hover {
  background: transparent;
  color: var(--muted);
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
  grid-template-columns: 22px 1fr auto;
  gap: 8px;
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
