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
  "create-query",
  "database-object-action",
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
const mysqlIconPath = "M16.405 5.501c-.115 0-.193.014-.274.033v.013h.014c.054.104.146.18.214.273.054.107.1.214.154.32l.014-.015c.094-.066.14-.172.14-.333-.04-.047-.046-.094-.08-.14-.04-.067-.126-.1-.18-.153zM5.77 18.695h-.927a50.854 50.854 0 00-.27-4.41h-.008l-1.41 4.41H2.45l-1.4-4.41h-.01a72.892 72.892 0 00-.195 4.41H0c.055-1.966.192-3.81.41-5.53h1.15l1.335 4.064h.008l1.347-4.064h1.095c.242 2.015.384 3.86.428 5.53zm4.017-4.08c-.378 2.045-.876 3.533-1.492 4.46-.482.716-1.01 1.073-1.583 1.073-.153 0-.34-.046-.566-.138v-.494c.11.017.24.026.386.026.268 0 .483-.075.647-.222.197-.18.295-.382.295-.605 0-.155-.077-.47-.23-.944L6.23 14.615h.91l.727 2.36c.164.536.233.91.205 1.123.4-1.064.678-2.227.835-3.483zm12.325 4.08h-2.63v-5.53h.885v4.85h1.745zm-3.32.135l-1.016-.5c.09-.076.177-.158.255-.25.433-.506.648-1.258.648-2.253 0-1.83-.718-2.746-2.155-2.746-.704 0-1.254.232-1.65.697-.43.508-.646 1.256-.646 2.245 0 .972.19 1.686.574 2.14.35.41.877.615 1.583.615.264 0 .506-.033.725-.098l1.325.772.36-.622zM15.5 17.588c-.225-.36-.337-.94-.337-1.736 0-1.393.424-2.09 1.27-2.09.443 0 .77.167.977.5.224.362.336.936.336 1.723 0 1.404-.424 2.108-1.27 2.108-.445 0-.77-.167-.978-.5zm-1.658-.425c0 .47-.172.856-.516 1.156-.344.3-.803.45-1.384.45-.543 0-1.064-.172-1.573-.515l.237-.476c.438.22.833.328 1.19.328.332 0 .593-.073.783-.22a.754.754 0 00.3-.615c0-.33-.23-.61-.648-.845-.388-.213-1.163-.657-1.163-.657-.422-.307-.632-.636-.632-1.177 0-.45.157-.81.47-1.085.315-.278.72-.415 1.22-.415.512 0 .98.136 1.4.41l-.213.476a2.726 2.726 0 00-1.064-.23c-.283 0-.502.068-.654.206a.685.685 0 00-.248.524c0 .328.234.61.666.85.393.215 1.187.67 1.187.67.433.305.648.63.648 1.168zm9.382-5.852c-.535-.014-.95.04-1.297.188-.1.04-.26.04-.274.167.055.053.063.14.11.214.08.134.218.313.346.407.14.11.28.216.427.31.26.16.555.255.81.416.145.094.293.213.44.313.073.05.12.14.214.172v-.02c-.046-.06-.06-.147-.105-.214-.067-.067-.134-.127-.2-.193a3.223 3.223 0 00-.695-.675c-.214-.146-.682-.35-.77-.595l-.013-.014c.146-.013.32-.066.46-.106.227-.06.435-.047.67-.106.106-.027.213-.06.32-.094v-.06c-.12-.12-.21-.283-.334-.395a8.867 8.867 0 00-1.104-.823c-.21-.134-.476-.22-.697-.334-.08-.04-.214-.06-.26-.127-.12-.146-.19-.34-.275-.514a17.69 17.69 0 01-.547-1.163c-.12-.262-.193-.523-.34-.763-.69-1.137-1.437-1.826-2.586-2.5-.247-.14-.543-.2-.856-.274-.167-.008-.334-.02-.5-.027-.11-.047-.216-.174-.31-.235-.38-.24-1.364-.76-1.644-.072-.18.434.267.862.422 1.082.115.153.26.328.34.5.047.116.06.235.107.356.106.294.207.622.347.897.073.14.153.287.247.413.054.073.146.107.167.227-.094.136-.1.334-.154.5-.24.757-.146 1.693.194 2.25.107.166.362.534.703.393.3-.12.234-.5.32-.835.02-.08.007-.133.048-.187v.015c.094.188.188.367.274.555.206.328.566.668.867.895.16.12.287.328.487.402v-.02h-.015c-.043-.058-.1-.086-.154-.133a3.445 3.445 0 01-.35-.4 8.76 8.76 0 01-.747-1.218c-.11-.21-.202-.436-.29-.643-.04-.08-.04-.2-.107-.24-.1.146-.247.273-.32.453-.127.288-.14.642-.188 1.01-.027.007-.014 0-.027.014-.214-.052-.287-.274-.367-.46-.2-.475-.233-1.238-.06-1.785.047-.14.247-.582.167-.716-.042-.127-.174-.2-.247-.303a2.478 2.478 0 01-.24-.427c-.16-.374-.24-.788-.414-1.162-.08-.173-.22-.354-.334-.513-.127-.18-.267-.307-.368-.52-.033-.073-.08-.194-.027-.274.014-.054.042-.075.094-.09.088-.072.335.022.422.062.247.1.455.194.662.334.094.066.195.193.315.226h.14c.214.047.455.014.655.073.355.114.675.28.962.46a5.953 5.953 0 012.085 2.286c.08.154.115.295.188.455.14.33.313.663.455.982.14.315.275.636.476.897.1.14.502.213.682.286.133.06.34.115.46.188.23.14.454.3.67.454.11.076.443.243.463.378z";

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
    { key: "create-database", label: "新建库", disabled: connection?.workspace !== "database" || !connected },
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
  } else if (item.key === "create-database") {
    emit("database-object-action", { connection: contextConnection.value, action: "create-database" });
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
                  <svg
                    v-else-if="connection.iconClass === 'mysql'"
                    class="mysql-node-icon"
                    viewBox="0 0 32 32"
                    aria-hidden="true"
                  >
                    <path :d="mysqlIconPath" />
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
              @create-query="(payload) => emit('create-query', { connection, ...payload })"
              @database-object-action="(payload) => emit('database-object-action', { connection, ...payload })"
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
            <svg
              v-else-if="connection.iconClass === 'mysql'"
              class="mysql-node-icon"
              viewBox="0 0 32 32"
              aria-hidden="true"
            >
              <path :d="mysqlIconPath" />
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
        @create-query="(payload) => emit('create-query', { connection, ...payload })"
        @database-object-action="(payload) => emit('database-object-action', { connection, ...payload })"
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

.connection-item .node-icon.mysql,
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
