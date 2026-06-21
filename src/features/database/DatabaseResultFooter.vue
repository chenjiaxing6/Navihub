<script setup>
import { Check, Close, Minus, Plus, Refresh } from "@element-plus/icons-vue";
import { DEFAULT_PAGE_SIZE, PAGE_SIZE_OPTIONS } from "./databaseTableUtils";

defineProps({
  activeKind: { type: String, required: true },
  result: { type: Object, default: null },
  searchedRowCount: { type: Number, default: 0 },
  canEdit: { type: Boolean, default: false },
  selectedRowCount: { type: Number, default: 0 },
  hasPendingChanges: { type: Boolean, default: false },
  loading: { type: Boolean, default: false },
  canStop: { type: Boolean, default: false },
});

const emit = defineEmits([
  "add-row",
  "delete-records",
  "commit-changes",
  "cancel-changes",
  "refresh",
  "stop",
  "page-size-change",
  "page-change",
]);
</script>

<template>
  <footer class="table-footer">
    <div class="table-footer__tools" aria-label="数据工具">
      <button type="button" class="table-footer__tool" title="新增记录" :disabled="!canEdit" @click="emit('add-row')">
        <el-icon><Plus /></el-icon>
      </button>
      <button
        type="button"
        class="table-footer__tool"
        title="删除记录"
        :disabled="activeKind !== 'table' || selectedRowCount === 0"
        @click="emit('delete-records')"
      >
        <el-icon><Minus /></el-icon>
      </button>
      <span class="table-footer__tool-separator" />
      <button type="button" class="table-footer__tool" title="提交更改" :disabled="!hasPendingChanges || loading" @click="emit('commit-changes')">
        <el-icon><Check /></el-icon>
      </button>
      <button type="button" class="table-footer__tool" title="取消更改" :disabled="!hasPendingChanges" @click="emit('cancel-changes')">
        <el-icon><Close /></el-icon>
      </button>
      <span class="table-footer__tool-separator" />
      <button type="button" class="table-footer__tool" title="刷新" :disabled="loading" @click="emit('refresh')">
        <el-icon><Refresh /></el-icon>
      </button>
      <button
        type="button"
        class="table-footer__tool table-footer__tool--block"
        title="停止"
        :disabled="!canStop"
        @click="emit('stop')"
      />
    </div>
    <span v-if="result" class="table-footer__summary">
      <template v-if="activeKind === 'table'">
        第 {{ result.page }} 页 · 显示 {{ searchedRowCount }} / {{ result.totalRows }} 行 · {{ result.elapsedMs }}ms
      </template>
      <template v-else>
        显示 {{ searchedRowCount }} / {{ result.totalRows }} 行 · {{ result.elapsedMs }}ms
      </template>
    </span>
    <el-pagination
      v-if="activeKind === 'table'"
      background
      layout="sizes, prev, pager, next, jumper"
      popper-class="table-page-size-popper"
      :current-page="result?.page ?? 1"
      :page-size="result?.pageSize ?? DEFAULT_PAGE_SIZE"
      :page-sizes="PAGE_SIZE_OPTIONS"
      :total="result?.totalRows ?? 0"
      @size-change="emit('page-size-change', $event)"
      @current-change="emit('page-change', $event)"
    />
  </footer>
</template>

<style scoped>
.table-footer {
  display: flex;
  flex: 0 0 38px;
  align-items: center;
  justify-content: flex-start;
  gap: 10px;
  min-height: 38px;
  padding: 0 10px;
  border-top: 1px solid var(--line);
  background: var(--surface-muted);
  font-size: 12px;
  font-weight: 400;
}

.table-footer__tools {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 4px;
  min-width: 0;
}

.table-footer__tool {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  padding: 0;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: #686d76;
  cursor: pointer;
  appearance: none;
}

.table-footer__tool .el-icon {
  font-size: 16px;
  font-weight: 700;
}

.table-footer__tool:hover:not(:disabled) {
  background: var(--surface-strong);
  color: var(--text);
}

.table-footer__tool:active:not(:disabled) {
  background: #e8e9ec;
}

.table-footer__tool:disabled {
  color: #c6c9ce;
  cursor: default;
}

.table-footer__tool--block {
  width: 22px;
}

.table-footer__tool--block::before {
  width: 14px;
  height: 14px;
  border-radius: 2px;
  background: currentColor;
  content: "";
}

.table-footer__tool-separator {
  width: 8px;
  height: 1px;
}

.table-footer__summary {
  flex: 0 0 auto;
  margin-left: 4px;
  color: var(--muted);
  font-size: 12px;
  font-weight: 400;
  line-height: 24px;
}

.table-footer :deep(.el-pagination) {
  --el-pagination-bg-color: #fff;
  --el-pagination-button-color: var(--muted);
  --el-pagination-button-disabled-bg-color: var(--surface-muted);
  --el-pagination-button-disabled-color: var(--faint);
  --el-pagination-hover-color: var(--orange);
  --el-pagination-font-size: 12px;
  --el-pagination-button-width: 24px;
  --el-pagination-button-height: 24px;
  margin-left: auto;
  color: var(--muted);
  font-size: 12px;
  font-weight: 400;
  line-height: 24px;
}

.table-footer :deep(.el-pagination span:not([class*="suffix"])),
.table-footer :deep(.el-pagination button),
.table-footer :deep(.el-pager li),
.table-footer :deep(.el-pagination__jump),
.table-footer :deep(.el-pagination__goto),
.table-footer :deep(.el-pagination__classifier) {
  color: var(--muted);
  font-size: 12px;
  font-weight: 400;
  line-height: 24px;
}

.table-footer :deep(.el-pagination.is-background .btn-next),
.table-footer :deep(.el-pagination.is-background .btn-prev),
.table-footer :deep(.el-pagination.is-background .el-pager li) {
  min-width: 24px;
  height: 24px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: #fff;
  color: var(--muted);
  font-size: 12px;
  font-weight: 400;
  line-height: 22px;
}

.table-footer :deep(.el-pagination.is-background .btn-next:hover),
.table-footer :deep(.el-pagination.is-background .btn-prev:hover),
.table-footer :deep(.el-pagination.is-background .el-pager li:hover) {
  border-color: var(--line-strong);
  background: var(--surface-strong);
  color: var(--text);
}

.table-footer :deep(.el-pagination.is-background .el-pager li.is-active) {
  border-color: var(--orange);
  background: var(--orange);
  color: #fff;
  font-weight: 400;
}

.table-footer :deep(.el-pagination.is-background .btn-next.is-disabled),
.table-footer :deep(.el-pagination.is-background .btn-prev.is-disabled) {
  border-color: var(--line);
  background: var(--surface-muted);
  color: var(--faint);
}

.table-footer :deep(.el-select__wrapper),
.table-footer :deep(.el-input__wrapper) {
  height: 24px;
  min-height: 24px;
  border-radius: 6px;
  background: #fff;
  box-shadow: 0 0 0 1px var(--line) inset;
}

.table-footer :deep(.el-select__wrapper:hover),
.table-footer :deep(.el-input__wrapper:hover) {
  box-shadow: 0 0 0 1px var(--line-strong) inset;
}

.table-footer :deep(.el-select__wrapper.is-focused),
.table-footer :deep(.el-input__wrapper.is-focus) {
  box-shadow: 0 0 0 1px var(--orange) inset, 0 0 0 3px rgba(242, 107, 58, 0.10);
}

.table-footer :deep(.el-select__selected-item),
.table-footer :deep(.el-input__inner) {
  color: var(--text);
  font-size: 12px;
  font-weight: 400;
}

.table-footer :deep(.el-pagination__jump) {
  margin-left: 10px;
  color: var(--muted);
}

.table-footer :deep(.el-pagination__sizes) {
  margin-right: 10px;
}
</style>
