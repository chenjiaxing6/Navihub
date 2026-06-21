<script setup>
import { ref } from "vue";

defineProps({
  columns: { type: Array, required: true },
  rows: { type: Array, required: true },
  gridTemplate: { type: String, required: true },
  contentWidth: { type: Number, required: true },
  scrollLeft: { type: Number, default: 0 },
  visibleRows: { type: Object, default: null },
  rowKeyPrefix: { type: String, default: "row" },
  isSchemaTable: { type: Boolean, default: false },
  hasSearch: { type: Boolean, default: false },
  normalizedSearch: { type: String, default: "" },
  editingCell: { type: Object, default: null },
  cellValue: { type: Function, required: true },
  isRowSelected: { type: Function, required: true },
  isCellSelected: { type: Function, required: true },
  isChangedCell: { type: Function, default: () => false },
  isNewRow: { type: Function, default: () => false },
  isEditingCell: { type: Function, default: () => false },
  absoluteRowIndex: { type: Function, default: (index) => index },
});

const emit = defineEmits([
  "scroll",
  "resize-column",
  "row-selection-start",
  "row-selection-extend",
  "cell-selection-start",
  "cell-selection-extend",
  "context-menu",
  "open-row",
  "edit-cell",
  "commit-edit",
  "cancel-edit",
  "update-edit-value",
]);

const viewportRef = ref(null);

defineExpose({
  viewportRef,
});

function renderedRows(propsRows, visibleRows) {
  return visibleRows?.rows ?? propsRows;
}

function spacerHeight(visibleRows, key) {
  return visibleRows ? `${visibleRows[key]}px` : "0px";
}
</script>

<template>
  <div class="virtual-table" @contextmenu.prevent="emit('context-menu', $event)">
    <div class="virtual-table__header-wrap">
      <div class="virtual-table__gutter-head" />
      <div
        class="virtual-table__header"
        :style="{
          gridTemplateColumns: gridTemplate,
          width: `${contentWidth}px`,
          transform: `translateX(-${scrollLeft}px)`,
        }"
      >
        <div v-for="column in columns" :key="column.key" class="virtual-table__th">
          <span class="virtual-table__th-label">{{ column.label }}</span>
          <span
            class="virtual-table__resize-handle"
            @mousedown.stop.prevent="emit('resize-column', $event, column)"
          />
        </div>
      </div>
    </div>
    <div ref="viewportRef" class="virtual-table__viewport" @scroll="emit('scroll', $event)">
      <div class="virtual-table__body" :style="{ width: `${contentWidth + 32}px` }">
        <div class="virtual-table__gutter">
          <div v-if="visibleRows" class="virtual-table__spacer" :style="{ height: spacerHeight(visibleRows, 'top') }" />
          <div
            v-for="(row, visibleIndex) in renderedRows(rows, visibleRows)"
            :key="`${rowKeyPrefix}-gutter-${absoluteRowIndex(visibleIndex)}-${row.name ?? ''}`"
            class="virtual-table__gutter-cell"
            :class="{ selected: isRowSelected(visibleIndex) }"
            @mousedown.prevent="emit('row-selection-start', $event, visibleIndex)"
            @mouseenter="emit('row-selection-extend', visibleIndex)"
            @dblclick="emit('open-row', row)"
            @contextmenu.prevent.stop="emit('context-menu', $event, absoluteRowIndex(visibleIndex))"
          />
          <div v-if="visibleRows" class="virtual-table__spacer" :style="{ height: spacerHeight(visibleRows, 'bottom') }" />
        </div>
        <div class="virtual-table__rows">
          <div v-if="visibleRows" class="virtual-table__spacer" :style="{ height: spacerHeight(visibleRows, 'top') }" />
          <div
            v-for="(row, visibleIndex) in renderedRows(rows, visibleRows)"
            :key="`${rowKeyPrefix}-${absoluteRowIndex(visibleIndex)}-${row.name ?? ''}`"
            class="virtual-table__row"
            :class="{ selected: isRowSelected(visibleIndex) }"
            :style="{ gridTemplateColumns: gridTemplate }"
            @dblclick="emit('open-row', row)"
            @contextmenu.prevent.stop="emit('context-menu', $event, absoluteRowIndex(visibleIndex))"
          >
            <div
              v-for="(column, columnIndex) in columns"
              :key="column.key"
              class="virtual-table__cell"
              :class="{
                selected: isCellSelected(visibleIndex, columnIndex),
                changed: isChangedCell(row, column),
                'is-new-row': isNewRow(row),
                matched: hasSearch && cellValue(row, column).toLowerCase().includes(normalizedSearch),
                'is-right': column.align === 'right',
              }"
              :title="cellValue(row, column)"
              @mousedown.prevent="emit('cell-selection-start', $event, visibleIndex, columnIndex)"
              @mouseenter="emit('cell-selection-extend', visibleIndex, columnIndex)"
              @contextmenu.prevent.stop="emit('context-menu', $event, absoluteRowIndex(visibleIndex), columnIndex)"
              @dblclick.stop="isSchemaTable ? emit('open-row', row) : emit('edit-cell', absoluteRowIndex(visibleIndex), columnIndex)"
            >
              <span v-if="isSchemaTable && column.key === 'name'" class="schema-table-name">
                <span class="schema-table-icon table-icon" />
                <span class="schema-table-name__text">{{ row.name }}</span>
              </span>
              <input
                v-else-if="isEditingCell(row, column)"
                :value="editingCell?.value ?? ''"
                class="virtual-table__cell-input"
                type="text"
                autofocus
                @input="emit('update-edit-value', $event.target.value)"
                @mousedown.stop
                @blur="emit('commit-edit')"
                @keydown.enter.prevent="emit('commit-edit')"
                @keydown.esc.prevent="emit('cancel-edit')"
              />
              <template v-else>{{ cellValue(row, column) }}</template>
            </div>
          </div>
          <div v-if="visibleRows" class="virtual-table__spacer" :style="{ height: spacerHeight(visibleRows, 'bottom') }" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.schema-table-name {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.schema-table-icon {
  position: relative;
  display: inline-grid;
  place-items: center;
  width: 18px;
  height: 18px;
  flex: 0 0 18px;
  color: var(--muted);
}

.schema-table-icon.table-icon {
  border: 1px solid var(--line-strong);
  border-radius: 5px;
  background: #fff;
  color: var(--blue);
}

.schema-table-icon.table-icon::before {
  position: absolute;
  width: 12px;
  height: 10px;
  border: 1.5px solid currentColor;
  border-radius: 2px;
  content: "";
}

.schema-table-icon.table-icon::after {
  position: absolute;
  width: 12px;
  height: 1.5px;
  background:
    linear-gradient(currentColor, currentColor) 0 0 / 100% 100% no-repeat,
    linear-gradient(currentColor, currentColor) 0 4px / 100% 100% no-repeat;
  box-shadow: 0 -3px 0 currentColor;
  opacity: 0.55;
  content: "";
}

.schema-table-name__text {
  min-width: 0;
  overflow: hidden;
  color: var(--text);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.virtual-table {
  display: flex;
  min-width: 0;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  overflow: hidden;
  color: #303647;
  font-size: 12px;
  font-weight: 400;
}

.virtual-table__header-wrap {
  display: flex;
  flex: 0 0 34px;
  overflow: hidden;
  border-bottom: 1px solid var(--line);
  background: var(--surface-muted);
}

.virtual-table__gutter-head {
  position: relative;
  z-index: 3;
  width: 32px;
  flex: 0 0 32px;
  border-right: 1px solid var(--line);
  background: var(--surface-muted);
}

.virtual-table__header {
  display: grid;
  min-width: max-content;
  will-change: transform;
}

.virtual-table__th {
  position: relative;
  display: flex;
  align-items: center;
  min-width: 0;
  height: 34px;
  padding: 0 10px;
  overflow: hidden;
  border-right: 1px solid var(--line);
  color: var(--muted);
  font-size: 12px;
  font-weight: 400;
  line-height: 34px;
  user-select: none;
}

.virtual-table__th-label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.virtual-table__resize-handle {
  position: absolute;
  top: 0;
  right: -4px;
  z-index: 2;
  width: 8px;
  height: 100%;
  cursor: col-resize;
}

.virtual-table__resize-handle::after {
  position: absolute;
  top: 7px;
  right: 3px;
  width: 1px;
  height: 20px;
  background: transparent;
  content: "";
}

.virtual-table__resize-handle:hover::after,
:global(body.is-resizing-table-column) .virtual-table__resize-handle::after {
  background: var(--orange);
}

:global(body.is-resizing-table-column) {
  cursor: col-resize;
  user-select: none;
}

.virtual-table__viewport {
  min-height: 0;
  flex: 1;
  overflow: auto;
}

.virtual-table__body {
  display: flex;
  min-width: max-content;
}

.virtual-table__gutter {
  position: sticky;
  left: 0;
  z-index: 2;
  width: 32px;
  flex: 0 0 32px;
  border-right: 1px solid var(--line);
  background: var(--panel);
}

.virtual-table__gutter-cell {
  height: 34px;
  border-bottom: 1px solid var(--line);
  background: var(--surface-muted);
  cursor: default;
  user-select: none;
}

.virtual-table__gutter-cell:hover {
  background: var(--surface-strong);
}

.virtual-table__gutter-cell.selected {
  background: var(--orange-soft);
  box-shadow: inset 2px 0 0 var(--orange);
}

.virtual-table__rows {
  min-width: max-content;
}

.virtual-table__row {
  display: grid;
  min-width: max-content;
  cursor: default;
  user-select: none;
}

.virtual-table__row:hover .virtual-table__cell {
  background: #fafafa;
}

.virtual-table__row.selected .virtual-table__cell {
  background: var(--orange-soft);
}

.virtual-table__row.selected:hover .virtual-table__cell {
  background: #ffe7de;
}

.virtual-table__cell {
  display: flex;
  align-items: center;
  min-width: 0;
  height: 34px;
  padding: 0 10px;
  overflow: hidden;
  border-right: 1px solid var(--line);
  border-bottom: 1px solid var(--line);
  color: #303647;
  line-height: 34px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.virtual-table__cell.is-right {
  justify-content: flex-end;
  text-align: right;
}

.virtual-table__cell.selected {
  background: #ffe0d4;
  color: var(--text);
  box-shadow: inset 0 0 0 1px rgba(242, 107, 58, 0.18);
}

.virtual-table__cell.changed {
  background: #fff4cf;
  box-shadow: inset 0 -2px 0 #d9a621;
}

.virtual-table__cell.is-new-row {
  background: #eaf7ef;
  box-shadow: inset 0 -2px 0 #5aa469;
}

.virtual-table__cell-input {
  width: calc(100% + 12px);
  height: 26px;
  margin: 0 -6px;
  padding: 0 6px;
  border: 1px solid var(--orange);
  border-radius: 4px;
  outline: none;
  background: #fff;
  color: var(--text);
  font: inherit;
  line-height: 24px;
}

.virtual-table__cell.matched {
  background: #fff7d6;
  color: var(--text);
}

.virtual-table__row:hover .virtual-table__cell.selected,
.virtual-table__row.selected .virtual-table__cell.selected {
  background: #ffd6c8;
  color: var(--text);
}

.virtual-table__row:hover .virtual-table__cell.matched {
  background: #fff0b8;
}

.virtual-table__spacer {
  min-width: 1px;
}
</style>
