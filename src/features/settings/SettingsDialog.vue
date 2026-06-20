<script setup>
import { computed } from "vue";
import { Close } from "@element-plus/icons-vue";
import { terminalThemes } from "./terminalThemes";

const visible = defineModel({ type: Boolean, default: false });
const terminalThemeId = defineModel("terminalThemeId", { type: String, required: true });

const activeTheme = computed(() =>
  terminalThemes.find((theme) => theme.id === terminalThemeId.value) ?? terminalThemes[0],
);
</script>

<template>
  <el-dialog
    v-model="visible"
    class="settings-dialog"
    custom-class="settings-dialog"
    width="620px"
    :show-close="false"
    destroy-on-close
  >
    <template #header>
      <div class="settings-dialog__header">
        <strong>设置</strong>
        <button class="settings-dialog__close" type="button" aria-label="关闭" @click="visible = false">
          <el-icon><Close /></el-icon>
        </button>
      </div>
    </template>

    <div class="settings-panel">
      <aside class="settings-panel__nav">
        <button class="active" type="button">终端</button>
      </aside>

      <section class="settings-panel__body">
        <div class="settings-section">
          <div class="settings-section__title">
            <p>外观</p>
            <h2>终端主题</h2>
          </div>

          <div class="theme-grid">
            <button
              v-for="theme in terminalThemes"
              :key="theme.id"
              class="theme-card"
              :class="{ active: terminalThemeId === theme.id }"
              type="button"
              @click="terminalThemeId = theme.id"
            >
              <span class="theme-card__preview" :style="{ background: theme.background, color: theme.foreground }">
                <span class="theme-card__line"><b :style="{ color: theme.theme.green }">$</b> git status</span>
                <span class="theme-card__line" :style="{ color: theme.theme.blue }">main</span>
                <span class="theme-card__line" :style="{ color: theme.theme.yellow }">2 files changed</span>
                <i :style="{ background: theme.cursor }" />
              </span>
              <span class="theme-card__name">{{ theme.name }}</span>
            </button>
          </div>

          <div class="terminal-preview" :style="{ background: activeTheme.background, color: activeTheme.foreground }">
            <span><b :style="{ color: activeTheme.theme.green }">user@host</b>:<b :style="{ color: activeTheme.theme.blue }">~/app</b>$ npm run build</span>
            <span :style="{ color: activeTheme.theme.cyan }">vite building client environment...</span>
            <span><b :style="{ color: activeTheme.theme.green }">✓</b> built in 2.4s</span>
          </div>
        </div>
      </section>
    </div>
  </el-dialog>
</template>

<style scoped>
.settings-dialog__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 46px;
  padding: 0 12px 0 16px;
}

.settings-dialog__header strong {
  font-size: 14px;
  font-weight: 760;
}

.settings-dialog__close {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
}

.settings-dialog__close:hover {
  border-color: var(--line);
  background: #fff;
  color: var(--text);
}

.settings-panel {
  display: grid;
  grid-template-columns: 132px minmax(0, 1fr);
  min-height: 390px;
}

.settings-panel__nav {
  padding: 10px;
  border-right: 1px solid var(--line);
  background: var(--surface-muted);
}

.settings-panel__nav button {
  width: 100%;
  height: 32px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  font-size: 12px;
  font-weight: 700;
  text-align: left;
}

.settings-panel__nav button.active {
  background: #fff;
  color: var(--orange);
  box-shadow: var(--shadow-card);
}

.settings-panel__body {
  min-width: 0;
  padding: 16px;
}

.settings-section__title p {
  margin: 0 0 3px;
  color: var(--faint);
  font-size: 10px;
  font-weight: 760;
  text-transform: uppercase;
}

.settings-section__title h2 {
  margin: 0 0 14px;
  font-size: 16px;
  font-weight: 760;
}

.theme-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.theme-card {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 8px;
  padding: 8px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #fff;
  color: var(--text);
  cursor: pointer;
  text-align: left;
}

.theme-card:hover {
  border-color: var(--line-strong);
}

.theme-card.active {
  border-color: var(--orange);
  box-shadow: 0 0 0 2px var(--orange-soft);
}

.theme-card__preview {
  position: relative;
  display: grid;
  grid-template-rows: repeat(3, 16px);
  min-height: 68px;
  overflow: hidden;
  padding: 9px;
  border-radius: 6px;
  font-family: SFMono-Regular, Consolas, Liberation Mono, monospace;
  font-size: 11px;
}

.theme-card__preview i {
  position: absolute;
  right: 10px;
  bottom: 10px;
  width: 7px;
  height: 13px;
}

.theme-card__line {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.theme-card__name {
  font-size: 12px;
  font-weight: 720;
}

.terminal-preview {
  display: grid;
  gap: 6px;
  margin-top: 14px;
  padding: 12px;
  border-radius: 8px;
  font-family: SFMono-Regular, Consolas, Liberation Mono, monospace;
  font-size: 12px;
  line-height: 1.35;
}
</style>
