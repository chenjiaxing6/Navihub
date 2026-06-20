<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import ContextMenu from "../shared/ContextMenu.vue";

const props = defineProps({
  activeTabId: { type: String, default: null },
  tabs: { type: Array, required: true },
});

const emit = defineEmits(["close-tab", "close-tabs", "select-tab"]);
const tabsWrap = ref(null);
const tabsViewportWidth = ref(0);
const contextMenuOpen = ref(false);
const contextMenuPosition = ref({ x: 0, y: 0 });
const contextTabId = ref(null);
let tabsResizeObserver = null;

const contextTabIndex = computed(() => props.tabs.findIndex((tab) => tab.id === contextTabId.value));
const contextMenuItems = computed(() => [
  { key: "all", label: "关闭所有", disabled: props.tabs.length === 0 },
  { key: "left", label: "关闭左侧", disabled: contextTabIndex.value <= 0 },
  {
    key: "right",
    label: "关闭右侧",
    disabled: contextTabIndex.value < 0 || contextTabIndex.value >= props.tabs.length - 1,
  },
]);
const visibleTabLimit = computed(() => {
  if (!tabsViewportWidth.value) {
    return props.tabs.length;
  }

  const tabsPerRow = Math.max(1, Math.floor(tabsViewportWidth.value / 132));
  const totalSlots = tabsPerRow * 2;
  return props.tabs.length > totalSlots ? Math.max(totalSlots - 1, 1) : totalSlots;
});
const visibleTabs = computed(() => {
  const limit = visibleTabLimit.value;
  if (props.tabs.length <= limit) {
    return props.tabs;
  }

  const activeIndex = props.tabs.findIndex((tab) => tab.id === props.activeTabId);
  if (activeIndex < limit) {
    return props.tabs.slice(0, limit);
  }

  return [...props.tabs.slice(0, Math.max(limit - 1, 0)), props.tabs[activeIndex]];
});
const overflowTabs = computed(() => {
  const visibleIds = new Set(visibleTabs.value.map((tab) => tab.id));
  return props.tabs.filter((tab) => !visibleIds.has(tab.id));
});

async function setupTabsResizeObserver() {
  await nextTick();
  if (!tabsWrap.value || tabsResizeObserver) {
    return;
  }

  const updateWidth = () => {
    tabsViewportWidth.value = tabsWrap.value?.clientWidth ?? 0;
  };
  updateWidth();
  tabsResizeObserver = new ResizeObserver(updateWidth);
  tabsResizeObserver.observe(tabsWrap.value);
}

function openContextMenu(event, tab) {
  contextTabId.value = tab.id;
  contextMenuPosition.value = { x: event.clientX, y: event.clientY };
  contextMenuOpen.value = true;
}

function handleContextSelect(item) {
  emit("close-tabs", { tabId: contextTabId.value, scope: item.key });
}

watch(
  () => props.tabs.length,
  () => {
    setupTabsResizeObserver();
  },
);

onMounted(() => {
  setupTabsResizeObserver();
});

onBeforeUnmount(() => {
  tabsResizeObserver?.disconnect();
});
</script>

<template>
  <div v-if="tabs.length > 0" ref="tabsWrap" class="workspace-tabs" role="tablist">
    <button
      v-for="tab in visibleTabs"
      :key="tab.id"
      class="workspace-tab"
      :class="{ active: activeTabId === tab.id }"
      :aria-selected="activeTabId === tab.id"
      role="tab"
      @click="emit('select-tab', tab.id)"
      @contextmenu.prevent="openContextMenu($event, tab)"
    >
      <span class="tab-label" :title="tab.label">{{ tab.label }}</span>
      <span v-if="tab.closable" class="tab-close" @click.stop="emit('close-tab', tab.id)">×</span>
    </button>

    <el-dropdown v-if="overflowTabs.length" trigger="click" @command="emit('select-tab', $event)">
      <button class="workspace-tab-more" type="button">
        <span>更多</span>
        <strong>{{ overflowTabs.length }}</strong>
      </button>
      <template #dropdown>
        <el-dropdown-menu>
          <el-dropdown-item
            v-for="tab in overflowTabs"
            :key="tab.id"
            :command="tab.id"
            :class="{ active: activeTabId === tab.id }"
          >
            {{ tab.label }}
          </el-dropdown-item>
        </el-dropdown-menu>
      </template>
    </el-dropdown>

    <ContextMenu
      v-model="contextMenuOpen"
      :items="contextMenuItems"
      :x="contextMenuPosition.x"
      :y="contextMenuPosition.y"
      @select="handleContextSelect"
    />
  </div>
</template>

<style scoped>
.workspace-tabs {
  display: flex;
  flex-wrap: wrap;
  min-width: 0;
  flex: 0 0 auto;
  max-height: 72px;
  align-content: end;
  align-items: end;
  gap: 2px;
  overflow: hidden;
  padding: 6px 10px 0;
  border-bottom: 1px solid var(--line);
  background: var(--surface-muted);
}

.workspace-tab {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  max-width: 220px;
  min-width: 0;
  height: 32px;
  padding: 0 12px;
  overflow: hidden;
  border: 1px solid var(--line);
  border-bottom: 0;
  border-radius: 9px 9px 0 0;
  background: var(--surface-strong);
  color: var(--muted);
  cursor: pointer;
  font: inherit;
  font-size: 12px;
  font-weight: 400;
  appearance: none;
  white-space: nowrap;
}

.tab-label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.workspace-tab:hover {
  border-color: var(--line-strong);
  background: #fff;
  color: var(--text);
}

.workspace-tab.active {
  border-color: var(--line-strong);
  background: #fff;
  color: var(--text);
  box-shadow: var(--shadow-card);
  font-weight: 400;
}

.workspace-tab:active {
  background: var(--surface-strong);
  transform: none;
}

.tab-close {
  display: inline-grid;
  place-items: center;
  width: 16px;
  height: 16px;
  flex: 0 0 16px;
  color: var(--faint);
  font-size: 15px;
  line-height: 1;
}

.tab-close:hover {
  color: #d92d20;
}

.workspace-tab-more {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 10px;
  border: 1px solid var(--line);
  border-bottom: 0;
  border-radius: 9px 9px 0 0;
  background: #fff;
  color: var(--muted);
  cursor: pointer;
  font: inherit;
  font-size: 12px;
  appearance: none;
}

.workspace-tab-more strong {
  display: grid;
  place-items: center;
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  border-radius: 999px;
  background: var(--orange-soft);
  color: var(--orange);
  font-size: 11px;
  font-weight: 760;
}

.workspace-tab-more:hover {
  border-color: var(--line-strong);
  color: var(--text);
}
</style>
