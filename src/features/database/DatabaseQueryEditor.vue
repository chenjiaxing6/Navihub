<script setup>
import { DocumentChecked, VideoPlay } from "@element-plus/icons-vue";

defineProps({
  schemaName: { type: String, default: "" },
  schemaOptions: { type: Array, default: () => [] },
  runLabel: { type: String, required: true },
  ready: { type: Boolean, default: false },
  fill: { type: Boolean, default: false },
});

const emit = defineEmits(["update:schemaName", "run", "save"]);
</script>

<template>
  <div class="query-editor" :class="{ 'is-fill': fill }">
    <div class="query-editor__bar">
      <el-select
        :model-value="schemaName"
        class="query-schema-select"
        size="small"
        filterable
        clearable
        placeholder="选择库"
        no-match-text="没有匹配的库"
        no-data-text="暂无库"
        popper-class="query-schema-select-popper"
        :disabled="schemaOptions.length === 0"
        @update:model-value="emit('update:schemaName', $event)"
      >
        <el-option
          v-for="schema in schemaOptions"
          :key="schema"
          :label="schema"
          :value="schema"
        />
      </el-select>
      <div class="query-editor__actions">
        <el-button
          class="query-run-button"
          :icon="VideoPlay"
          size="small"
          @click="emit('run')"
        >
          {{ runLabel }}
        </el-button>
        <el-button
          class="query-run-button"
          :icon="DocumentChecked"
          size="small"
          @click="emit('save')"
        >
          保存
        </el-button>
      </div>
    </div>
    <div class="query-editor__host" :class="{ ready }">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.query-editor {
  display: flex;
  flex: 0 0 238px;
  min-height: 160px;
  flex-direction: column;
  border-bottom: 1px solid var(--line);
  background: var(--panel);
}

.query-editor.is-fill {
  flex: 1 1 0;
  height: 100%;
  min-height: 0;
  border-bottom: 0;
}

.query-editor__bar {
  display: flex;
  align-items: center;
  min-height: 38px;
  padding: 0 8px 0 10px;
  border-bottom: 1px solid var(--line);
  background: var(--surface-muted);
}

.query-editor__actions {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-left: auto;
}

.query-editor__actions :deep(.el-button + .el-button) {
  margin-left: 0;
}

.query-editor__bar :deep(.query-schema-select) {
  width: min(220px, 48%);
}

.query-editor__bar :deep(.query-schema-select .el-select__wrapper) {
  min-height: 28px;
  border-radius: 7px;
  background: #fff;
  box-shadow: 0 0 0 1px var(--line) inset;
}

.query-editor__bar :deep(.query-schema-select .el-select__wrapper:hover) {
  box-shadow: 0 0 0 1px var(--line-strong) inset;
}

.query-editor__bar :deep(.query-schema-select .el-select__wrapper.is-focused) {
  box-shadow: 0 0 0 1px var(--orange) inset, 0 0 0 3px rgba(242, 107, 58, 0.10);
}

.query-editor__bar :deep(.query-schema-select .el-select__selected-item),
.query-editor__bar :deep(.query-schema-select .el-input__inner) {
  color: var(--muted);
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
  font-size: 12px;
}

.query-editor__bar :deep(.query-run-button) {
  height: 28px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: #fff;
  color: var(--text);
  font-size: 12px;
  font-weight: 650;
  box-shadow: none;
}

.query-editor__bar :deep(.query-run-button:hover) {
  border-color: var(--line-strong);
  background: var(--surface-strong);
  color: var(--text);
}

.query-editor__bar :deep(.query-run-button:active) {
  border-color: #f5c5b3;
  background: var(--orange-soft);
  color: var(--orange);
}

.query-editor__bar :deep(.query-run-button .el-icon) {
  color: var(--orange);
}

.query-editor__host {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  width: 100%;
  overflow: hidden;
  background: #fff;
}

.query-editor__host :deep(.cm-editor) {
  height: 100%;
}

.query-editor__host :slotted(.query-editor-root) {
  min-height: 0;
  flex: 1;
  height: 100%;
}

.query-editor__host :deep(.query-selected-text) {
  border-radius: 3px;
  background: #ffc7b3;
  color: var(--text);
}

.query-editor__host :deep(.cm-tooltip),
.query-editor__host :deep(.cm-tooltip-autocomplete) {
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #fff;
  box-shadow: var(--shadow-card);
  color: var(--text);
  font-family: var(--app-font);
  font-size: 12px;
}

.query-editor__host :deep(.cm-tooltip-autocomplete ul li[aria-selected]) {
  background: var(--surface-strong);
  color: var(--text);
}
</style>
