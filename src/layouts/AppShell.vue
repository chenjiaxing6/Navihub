<script setup>
import { computed, ref, watch } from "vue";
import { FolderAdd, Plus } from "@element-plus/icons-vue";
import ModuleRail from "./ModuleRail.vue";
import WorkspaceTabs from "./WorkspaceTabs.vue";
import ConnectionExplorer from "../features/connections/ConnectionExplorer.vue";
import DatabaseWorkspace from "../features/database/DatabaseWorkspace.vue";
import SshWorkspace from "../features/terminal/SshWorkspace.vue";
import { workspaces } from "../shared/workspaces";

const activeWorkspace = defineModel("activeWorkspace", { type: String, required: true });

const props = defineProps({
  activeConnectionId: { type: String, required: true },
  activeSchemaConnectionId: { type: String, default: null },
  activeConnection: { type: Object, default: null },
  activeTopTab: { type: Object, default: null },
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
  "duplicate-connection",
  "edit-connection",
  "activate-schema",
  "close-connection",
  "close-top-tab",
  "close-top-tabs",
  "select-top-tab",
  "set-workspace",
  "select-connection",
  "open-connection",
  "open-schema",
  "refresh-connection",
  "rename-connection-folder",
  "move-connection-to-folder",
  "update-mysql-connection",
  "schema-loaded",
  "open-table-query",
  "update-ssh-state",
  "open-settings",
]);

const currentWorkspace = computed(() =>
  workspaces.find((workspace) => workspace.id === activeWorkspace.value) ?? workspaces[0],
);
const searchQuery = ref("");

watch(activeWorkspace, () => {
  searchQuery.value = "";
});
</script>

<template>
  <main class="app-shell">
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
        :active-schema-connection-id="props.activeSchemaConnectionId"
        :active-connection="props.activeConnection"
        :open-schema-keys="props.openSchemaKeys"
        :search-query="searchQuery"
        :schema-open-versions="props.schemaOpenVersions"
        @activate-schema="emit('activate-schema', $event)"
        @close-connection="emit('close-connection', $event)"
        @create-connection="emit('create-connection', $event)"
        @delete-connection="emit('delete-connection', $event)"
        @delete-folder="emit('delete-connection-folder', $event)"
        @duplicate-connection="emit('duplicate-connection', $event)"
        @edit-connection="emit('edit-connection', $event)"
        @move-connection-to-folder="emit('move-connection-to-folder', $event)"
        @open-schema="emit('open-schema', $event)"
        @open-table-query="emit('open-table-query', $event)"
        @refresh-connection="emit('refresh-connection', $event)"
        @rename-folder="emit('rename-connection-folder', $event)"
        @select-connection="emit('select-connection', $event)"
        @open-connection="emit('open-connection', $event)"
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

      <DatabaseWorkspace
        v-if="activeWorkspace === 'database'"
        :connection="props.activeConnection"
        :active-top-tab="props.activeTopTab"
        :pending-schema-open="props.pendingSchemaOpen"
        :pending-table-query="props.pendingTableQuery"
        @open-table-query="emit('open-table-query', $event)"
        @schema-loaded="emit('schema-loaded', $event)"
        @update-connection="emit('update-mysql-connection', $event)"
      />
      <SshWorkspace
        v-else-if="activeWorkspace === 'ssh'"
        :connection="props.activeConnection"
        :terminal-theme="props.terminalTheme"
        @connection-state="emit('update-ssh-state', $event)"
      />
    </section>
  </main>
</template>

<style scoped>
.app-shell {
  display: grid;
  grid-template-columns: 56px 316px 1fr;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  padding: 0;
  background: var(--app-bg);
}

.sidebar {
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

</style>
