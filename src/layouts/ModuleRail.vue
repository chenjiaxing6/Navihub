<script setup>
import { Setting } from "@element-plus/icons-vue";
import { workspaces } from "../shared/workspaces";

defineProps({
  activeWorkspace: { type: String, required: true },
});

const emit = defineEmits(["open-settings", "set-workspace"]);
</script>

<template>
  <nav class="module-rail" aria-label="主导航">
    <div class="brand" title="NaviHub">
      <img src="../../src-tauri/icons/navihub-icon-grand.svg" alt="NaviHub" />
    </div>
    <button
      v-for="workspace in workspaces"
      :key="workspace.id"
      class="rail-button"
      :class="{ active: activeWorkspace === workspace.id }"
      :title="workspace.title"
      @click="emit('set-workspace', workspace.id)"
    >
      <el-icon :size="21"><component :is="workspace.icon" /></el-icon>
      <span>{{ workspace.label }}</span>
    </button>
    <div class="rail-spacer" />
    <button class="rail-icon" title="设置" @click="emit('open-settings')">
      <el-icon><Setting /></el-icon>
    </button>
  </nav>
</template>

<style scoped>
.module-rail {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 10px 7px;
  border-right: 1px solid var(--line);
  background: var(--rail-bg);
}

.brand {
  display: grid;
  place-items: center;
  width: 38px;
  height: 38px;
  margin: 0 0 8px;
  border-radius: 12px;
  background: transparent;
  box-shadow: none;
}

.brand img {
  display: block;
  width: 34px;
  height: 34px;
  object-fit: contain;
}

.rail-button,
.rail-icon {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 0;
  border-radius: 10px;
  background: transparent;
  cursor: pointer;
}

.rail-button {
  width: 46px;
  height: 44px;
  flex-direction: column;
  gap: 1px;
  color: var(--muted);
  font-size: 11px;
  font-weight: 680;
}

.rail-button.active,
.rail-icon.active {
  background: #fff;
  color: var(--orange);
  box-shadow: var(--shadow-card);
}

.rail-button:hover,
.rail-icon:hover {
  background: #fff;
  color: var(--text);
}

.rail-button:active,
.rail-icon:active {
  background: var(--surface-strong);
  transform: none;
}

.rail-spacer {
  flex: 1;
}

.rail-icon {
  width: 40px;
  height: 38px;
  color: var(--muted);
}
</style>
