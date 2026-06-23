<script setup>
import { computed, defineAsyncComponent, onBeforeUnmount, ref, watch } from "vue";
import { FolderAdd, Plus } from "@element-plus/icons-vue";
import ModuleRail from "./ModuleRail.vue";
import WorkspaceTabs from "./WorkspaceTabs.vue";
import ConnectionExplorer from "../features/connections/ConnectionExplorer.vue";
import { workspaces } from "../shared/workspaces";

const activeWorkspace = defineModel("activeWorkspace", { type: String, required: true });

const props = defineProps({
  activeConnectionId: { type: String, required: true },
  activeSchemaConnectionId: { type: String, default: null },
  expandedConnectionIds: { type: Array, default: () => [] },
  activeConnection: { type: Object, default: null },
  databaseConnection: { type: Object, default: null },
  sshConnection: { type: Object, default: null },
  activeTopTab: { type: Object, default: null },
  databaseActiveTopTab: { type: Object, default: null },
  activeTopTabId: { type: String, default: null },
  openSchemaKeys: { type: Array, default: () => [] },
  schemaOpenVersions: { type: Object, default: () => ({}) },
  topTabs: { type: Array, required: true },
  pendingTableQuery: { type: Object, default: null },
  pendingSchemaOpen: { type: Object, default: null },
  visibleConnections: { type: Array, required: true },
  visibleConnectionFolders: { type: Array, default: () => [] },
  terminalTheme: { type: Object, required: true },
});

const emit = defineEmits([
  "create-connection",
  "create-connection-folder",
  "delete-connection",
  "delete-connection-folder",
  "database-object-action",
  "duplicate-connection",
  "edit-connection",
  "activate-schema",
  "close-connection",
  "close-schema",
  "close-top-tab",
  "close-top-tabs",
  "select-top-tab",
  "set-workspace",
  "select-connection",
  "open-connection",
  "open-schema",
  "refresh-connection",
  "rename-connection-folder",
  "table-design-saved",
  "toggle-schema-pin",
  "toggle-connection-expanded",
  "move-connection-to-folder",
  "update-mysql-connection",
  "schema-loaded",
  "create-query",
  "open-table-query",
  "save-query",
  "update-query-schema",
  "update-ssh-state",
  "open-settings",
]);

const currentWorkspace = computed(() =>
  workspaces.find((workspace) => workspace.id === activeWorkspace.value) ?? workspaces[0],
);
const workspaceComponents = {
  database: defineAsyncComponent(() => import("../features/database/DatabaseWorkspace.vue")),
  ssh: defineAsyncComponent(() => import("../features/terminal/SshWorkspace.vue")),
};
const activeWorkspaceComponent = computed(() => workspaceComponents[activeWorkspace.value] ?? workspaceComponents.database);
const searchQuery = ref("");
const sidebarWidths = ref({
  database: 316,
  ssh: 316,
});
const resizingSidebar = ref(null);
const SIDEBAR_MIN_WIDTH = 240;
const SIDEBAR_MAX_WIDTH = 520;
const RAIL_WIDTH = 56;
const WORKSPACE_MIN_WIDTH = 720;

const activeSidebarWidth = computed(() => sidebarWidths.value[activeWorkspace.value] ?? sidebarWidths.value.database);
const shellGridColumns = computed(() => `${RAIL_WIDTH}px ${activeSidebarWidth.value}px minmax(0, 1fr)`);

watch(activeWorkspace, () => {
  searchQuery.value = "";
});

onBeforeUnmount(() => {
  stopSidebarResize();
});

function clampSidebarWidth(width) {
  const viewportMax = window.innerWidth - RAIL_WIDTH - WORKSPACE_MIN_WIDTH;
  const maxWidth = Math.max(SIDEBAR_MIN_WIDTH, Math.min(SIDEBAR_MAX_WIDTH, viewportMax));
  return Math.min(Math.max(width, SIDEBAR_MIN_WIDTH), maxWidth);
}

function startSidebarResize(event) {
  if (event.button != null && event.button !== 0) {
    return;
  }

  resizingSidebar.value = {
    workspace: activeWorkspace.value,
    startX: event.clientX,
    startWidth: activeSidebarWidth.value,
  };
  document.body.classList.add("is-resizing-sidebar");
  window.addEventListener("pointermove", resizeSidebar);
  window.addEventListener("pointerup", stopSidebarResize);
}

function resizeSidebar(event) {
  if (!resizingSidebar.value) {
    return;
  }

  const { workspace, startX, startWidth } = resizingSidebar.value;
  sidebarWidths.value = {
    ...sidebarWidths.value,
    [workspace]: clampSidebarWidth(startWidth + event.clientX - startX),
  };
}

function stopSidebarResize() {
  if (!resizingSidebar.value) {
    return;
  }

  resizingSidebar.value = null;
  document.body.classList.remove("is-resizing-sidebar");
  window.removeEventListener("pointermove", resizeSidebar);
  window.removeEventListener("pointerup", stopSidebarResize);
}
</script>

<template>
  <main class="app-shell" :style="{ gridTemplateColumns: shellGridColumns }">
    <ModuleRail
      :active-workspace="activeWorkspace"
      @open-settings="emit('open-settings')"
      @set-workspace="emit('set-workspace', $event)"
    />

    <aside class="sidebar">
      <header class="sidebar__header">
        <div>
          <p>{{ currentWorkspace.eyebrow }}</p>
          <h1>{{ currentWorkspace.title }}</h1>
        </div>
        <div class="sidebar__actions">
          <el-button :icon="FolderAdd" circle title="新建文件夹" @click="emit('create-connection-folder', activeWorkspace)" />
          <el-button :icon="Plus" circle title="新建连接" @click="emit('create-connection')" />
        </div>
      </header>

      <div class="sidebar__search">
        <el-input v-model="searchQuery" :placeholder="currentWorkspace.search" size="small" clearable />
      </div>

      <ConnectionExplorer
        :connections="props.visibleConnections"
        :folders="props.visibleConnectionFolders"
        :active-connection-id="props.activeConnectionId"
        :expanded-connection-ids="props.expandedConnectionIds"
        :active-connection="props.activeConnection"
        :open-schema-keys="props.openSchemaKeys"
        :search-query="searchQuery"
        :schema-open-versions="props.schemaOpenVersions"
        @activate-schema="emit('activate-schema', $event)"
        @close-connection="emit('close-connection', $event)"
        @close-schema="emit('close-schema', $event)"
        @create-connection="emit('create-connection', $event)"
        @delete-connection="emit('delete-connection', $event)"
        @delete-folder="emit('delete-connection-folder', $event)"
        @duplicate-connection="emit('duplicate-connection', $event)"
        @edit-connection="emit('edit-connection', $event)"
        @create-query="emit('create-query', $event)"
        @database-object-action="emit('database-object-action', $event)"
        @move-connection-to-folder="emit('move-connection-to-folder', $event)"
        @open-schema="emit('open-schema', $event)"
        @open-table-query="emit('open-table-query', $event)"
        @refresh-connection="emit('refresh-connection', $event)"
        @rename-folder="emit('rename-connection-folder', $event)"
        @select-connection="emit('select-connection', $event)"
        @toggle-connection-expanded="emit('toggle-connection-expanded', $event)"
        @toggle-schema-pin="emit('toggle-schema-pin', $event)"
        @open-connection="emit('open-connection', $event)"
      />
      <div
        class="sidebar-resizer"
        role="separator"
        aria-label="调整连接区宽度"
        aria-orientation="vertical"
        :aria-valuemin="SIDEBAR_MIN_WIDTH"
        :aria-valuemax="SIDEBAR_MAX_WIDTH"
        :aria-valuenow="activeSidebarWidth"
        @pointerdown.prevent="startSidebarResize"
      />
    </aside>

    <section class="workspace">
      <WorkspaceTabs
        :active-tab-id="props.activeTopTabId"
        :tabs="props.topTabs"
        @close-tab="emit('close-top-tab', $event)"
        @close-tabs="emit('close-top-tabs', $event)"
        @select-tab="emit('select-top-tab', $event)"
      />

      <KeepAlive>
        <component
          :is="activeWorkspaceComponent"
          :key="activeWorkspace"
          :connection="activeWorkspace === 'database' ? props.databaseConnection : props.sshConnection"
          :active-top-tab="props.databaseActiveTopTab"
          :pending-schema-open="props.pendingSchemaOpen"
          :pending-table-query="props.pendingTableQuery"
          :terminal-theme="props.terminalTheme"
          @connection-state="emit('update-ssh-state', $event)"
          @database-object-action="emit('database-object-action', $event)"
          @open-table-query="emit('open-table-query', $event)"
          @refresh-connection="emit('refresh-connection', $event)"
          @save-query="emit('save-query', $event)"
          @schema-loaded="emit('schema-loaded', $event)"
          @table-design-saved="emit('table-design-saved', $event)"
          @update-connection="emit('update-mysql-connection', $event)"
          @update-query-schema="emit('update-query-schema', $event)"
        />
      </KeepAlive>
    </section>
  </main>
</template>

<style scoped>
.app-shell {
  display: grid;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  padding: 0;
  background: var(--app-bg);
}

.sidebar {
  position: relative;
  display: flex;
  min-width: 0;
  flex-direction: column;
  overflow: hidden;
  border-right: 1px solid var(--line);
  border-radius: 0;
  background: var(--sidebar-bg);
  box-shadow: none;
  backdrop-filter: none;
}

.sidebar-resizer {
  position: absolute;
  top: 0;
  right: -4px;
  bottom: 0;
  z-index: 2;
  width: 8px;
  background: transparent;
  cursor: col-resize;
}

.sidebar-resizer::before {
  position: absolute;
  top: 0;
  bottom: 0;
  left: 3px;
  width: 1px;
  background: transparent;
  content: "";
  transition: background 0.14s ease;
}

.sidebar-resizer::after {
  position: absolute;
  top: 0;
  bottom: 0;
  left: 2px;
  width: 3px;
  background: var(--orange);
  opacity: 0;
  content: "";
  transition: opacity 0.14s ease;
}

.sidebar-resizer:hover::after,
:global(body.is-resizing-sidebar) .sidebar-resizer::after {
  opacity: 0.75;
}

.sidebar-resizer:hover::before,
:global(body.is-resizing-sidebar) .sidebar-resizer::before {
  background: var(--orange-soft);
}

.sidebar__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 56px;
  min-height: 56px;
  padding: 0 12px;
  border-bottom: 1px solid var(--line);
  background: transparent;
}

.sidebar__header p {
  margin: 0 0 3px;
  color: var(--faint);
  font-size: 10px;
  font-weight: 760;
  text-transform: uppercase;
}

.sidebar__header h1 {
  margin: 0;
  font-size: 16px;
  font-weight: 780;
}

.sidebar__actions {
  display: flex;
  align-items: center;
  gap: 6px;
}

.sidebar__header :deep(.el-button) {
  width: 28px;
  height: 28px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #fff;
  color: var(--muted);
  box-shadow: none;
}

.sidebar__header :deep(.el-button:hover) {
  border-color: var(--line-strong);
  color: var(--text);
}

.sidebar__header :deep(.el-button:active) {
  background: var(--panel-muted);
  transform: none;
}

.sidebar__search {
  padding: 10px;
  border-bottom: 1px solid var(--line);
}

.sidebar__search :deep(.el-input__wrapper) {
  min-height: 32px;
  border-radius: 8px;
  background: #fff;
  box-shadow: 0 0 0 1px var(--line) inset;
}

.sidebar__search :deep(.el-input__wrapper.is-focus) {
  box-shadow: 0 0 0 1px var(--orange) inset, 0 0 0 3px rgba(242, 107, 58, 0.13);
}

.workspace {
  display: flex;
  min-width: 0;
  overflow: hidden;
  flex-direction: column;
  border: 0;
  border-radius: 0;
  background: var(--panel);
  box-shadow: none;
  backdrop-filter: none;
}

:global(body.is-resizing-sidebar) {
  cursor: col-resize;
  user-select: none;
}

</style>
