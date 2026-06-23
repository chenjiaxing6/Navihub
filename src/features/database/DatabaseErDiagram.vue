<script setup>
import { computed, reactive, ref, watch } from "vue";

const props = defineProps({
  columnsByTable: { type: Object, default: () => ({}) },
  relationships: { type: Array, default: () => [] },
  tables: { type: Array, default: () => [] },
});

const emit = defineEmits(["open-table"]);
const scale = ref(1);
const pan = reactive({ x: 0, y: 0 });
const dragState = ref(null);
const nodeLayouts = reactive({});
const nodeDragState = ref(null);
const nodeResizeState = ref(null);

const nodeWidth = 280;
const minNodeWidth = 220;
const minNodeHeight = 120;
const headerHeight = 32;
const rowHeight = 22;
const maxVisibleColumns = 16;
const gapX = 84;
const gapY = 92;
const columns = computed(() => Math.max(1, Math.ceil(Math.sqrt(props.tables.length || 1))));
watch(
  () => props.tables.map((table) => table.name).join("\n"),
  () => {
    props.tables.forEach((table, index) => {
      if (nodeLayouts[table.name]) {
        return;
      }

      const column = index % columns.value;
      const row = Math.floor(index / columns.value);
      nodeLayouts[table.name] = {
        x: 24 + column * (nodeWidth + gapX),
        y: 24 + row * (260 + gapY),
        width: nodeWidth,
        height: 0,
      };
    });
  },
  { immediate: true },
);
const nodes = computed(() => props.tables.map((table, index) => {
  const column = index % columns.value;
  const row = Math.floor(index / columns.value);
  const layout = nodeLayouts[table.name] ?? {
    x: 24 + column * (nodeWidth + gapX),
    y: 24 + row * (260 + gapY),
    width: nodeWidth,
    height: 0,
  };
  const fields = props.columnsByTable[table.name] ?? [];
  const visibleFields = fields.slice(0, maxVisibleColumns);
  const hiddenFieldCount = Math.max(0, fields.length - visibleFields.length);
  const contentHeight = headerHeight + Math.max(3, visibleFields.length + (hiddenFieldCount ? 1 : 0)) * rowHeight + 12;
  const nodeHeight = Math.max(minNodeHeight, layout.height || contentHeight);
  return {
    ...table,
    fields,
    hiddenFieldCount,
    height: nodeHeight,
    visibleFields,
    width: layout.width,
    x: layout.x,
    y: layout.y,
  };
}));
const nodeMap = computed(() => new Map(nodes.value.map((node) => [node.name, node])));
const canvasSize = computed(() => {
  const rowCount = Math.ceil((props.tables.length || 1) / columns.value);
  const maxRight = nodes.value.reduce((right, node) => Math.max(right, node.x + node.width), 0);
  const maxBottom = nodes.value.reduce((bottom, node) => Math.max(bottom, node.y + node.height), 0);
  return {
    width: Math.max(960, maxRight + 80, 48 + columns.value * nodeWidth + Math.max(0, columns.value - 1) * gapX),
    height: Math.max(620, maxBottom + 80, 48 + rowCount * 340 + Math.max(0, rowCount - 1) * gapY),
  };
});
const edges = computed(() => props.relationships
  .map((relationship) => {
    const source = nodeMap.value.get(relationship.table);
    const target = nodeMap.value.get(relationship.referencedTable);
    if (!source || !target) {
      return null;
    }

    const sourceX = source.x + source.width;
    const sourceY = source.y + Math.min(source.height - 24, headerHeight + 28);
    const targetX = target.x;
    const targetY = target.y + Math.min(target.height - 24, headerHeight + 28);
    const midX = sourceX + (targetX - sourceX) / 2;
    return {
      ...relationship,
      path: `M ${sourceX} ${sourceY} C ${midX} ${sourceY}, ${midX} ${targetY}, ${targetX} ${targetY}`,
    };
  })
  .filter(Boolean));

function handleWheel(event) {
  event.preventDefault();
  const delta = event.deltaY > 0 ? -0.08 : 0.08;
  scale.value = Math.min(1.6, Math.max(0.45, Number((scale.value + delta).toFixed(2))));
}

function startPan(event) {
  if (event.button !== 0 || event.target.closest?.(".er-node")) {
    return;
  }

  dragState.value = {
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY,
    panX: pan.x,
    panY: pan.y,
  };
  event.currentTarget.setPointerCapture(event.pointerId);
}

function movePan(event) {
  const drag = dragState.value;
  if (!drag) {
    return;
  }

  pan.x = drag.panX + event.clientX - drag.startX;
  pan.y = drag.panY + event.clientY - drag.startY;
}

function stopPan(event) {
  if (dragState.value?.pointerId === event.pointerId) {
    dragState.value = null;
  }
}

function startNodeDrag(event, node) {
  if (event.button !== 0 || event.target.closest?.(".er-resize-handle")) {
    return;
  }

  event.stopPropagation();
  nodeDragState.value = {
    name: node.name,
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY,
    x: node.x,
    y: node.y,
  };
  event.currentTarget.setPointerCapture(event.pointerId);
}

function moveNodeDrag(event) {
  const drag = nodeDragState.value;
  if (!drag) {
    return;
  }

  const layout = nodeLayouts[drag.name];
  if (!layout) {
    return;
  }

  layout.x = Math.max(0, drag.x + (event.clientX - drag.startX) / scale.value);
  layout.y = Math.max(0, drag.y + (event.clientY - drag.startY) / scale.value);
}

function stopNodeDrag(event) {
  if (nodeDragState.value?.pointerId === event.pointerId) {
    nodeDragState.value = null;
  }
}

function startNodeResize(event, node) {
  if (event.button !== 0) {
    return;
  }

  event.stopPropagation();
  nodeResizeState.value = {
    name: node.name,
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY,
    width: node.width,
    height: node.height,
  };
  event.currentTarget.setPointerCapture(event.pointerId);
}

function moveNodeResize(event) {
  const resize = nodeResizeState.value;
  if (!resize) {
    return;
  }

  const layout = nodeLayouts[resize.name];
  if (!layout) {
    return;
  }

  layout.width = Math.max(minNodeWidth, resize.width + (event.clientX - resize.startX) / scale.value);
  layout.height = Math.max(minNodeHeight, resize.height + (event.clientY - resize.startY) / scale.value);
}

function stopNodeResize(event) {
  if (nodeResizeState.value?.pointerId === event.pointerId) {
    nodeResizeState.value = null;
  }
}
</script>

<template>
  <div
    class="er-diagram"
    :class="{ dragging: dragState }"
    @pointerdown="startPan"
    @pointermove="movePan"
    @pointerup="stopPan"
    @pointercancel="stopPan"
    @wheel="handleWheel"
  >
    <div v-if="tables.length === 0" class="er-empty">暂无表</div>
    <div v-else class="er-stage">
      <div
        class="er-canvas"
        :style="{
          width: `${canvasSize.width}px`,
          height: `${canvasSize.height}px`,
          transform: `translate(${pan.x}px, ${pan.y}px) scale(${scale})`,
        }"
      >
        <svg class="er-lines" :viewBox="`0 0 ${canvasSize.width} ${canvasSize.height}`" aria-hidden="true">
          <defs>
            <marker id="er-arrow" markerHeight="7" markerWidth="7" orient="auto" refX="6" refY="3.5">
              <path d="M 0 0 L 7 3.5 L 0 7 z" />
            </marker>
          </defs>
          <path
            v-for="edge in edges"
            :key="`${edge.name}:${edge.table}:${edge.referencedTable}:${edge.column}`"
            class="er-edge"
            :d="edge.path"
            marker-end="url(#er-arrow)"
          />
        </svg>
        <button
          v-for="node in nodes"
          :key="node.name"
          class="er-node"
          :class="{ moving: nodeDragState?.name === node.name }"
          :style="{ left: `${node.x}px`, top: `${node.y}px`, width: `${node.width}px`, height: `${node.height}px` }"
          type="button"
          @dblclick="emit('open-table', node)"
          @pointerdown="startNodeDrag($event, node)"
          @pointermove="moveNodeDrag"
          @pointerup="stopNodeDrag"
          @pointercancel="stopNodeDrag"
        >
          <strong class="er-node__title">{{ node.name }}</strong>
          <div class="er-node__body">
            <span
              v-for="field in node.visibleFields"
              :key="field.name"
              class="er-field"
              :class="{ primary: field.primary, indexed: field.indexed }"
            >
              <i aria-hidden="true" />
              <span>{{ field.name }}</span>
              <em>{{ field.type }}</em>
            </span>
            <small v-if="node.hiddenFieldCount" class="er-more">{{ node.hiddenFieldCount }} more columns...</small>
          </div>
          <span
            class="er-resize-handle"
            aria-hidden="true"
            @pointerdown="startNodeResize($event, node)"
            @pointermove="moveNodeResize"
            @pointerup="stopNodeResize"
            @pointercancel="stopNodeResize"
          />
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.er-diagram {
  min-width: 0;
  min-height: 0;
  flex: 1;
  overflow: hidden;
  cursor: grab;
  background:
    linear-gradient(#edf1f5 1px, transparent 1px),
    linear-gradient(90deg, #edf1f5 1px, transparent 1px),
    linear-gradient(#f6f8fa 1px, transparent 1px),
    linear-gradient(90deg, #f6f8fa 1px, transparent 1px),
    #fff;
  background-size: 24px 24px, 24px 24px, 6px 6px, 6px 6px;
  user-select: none;
}

.er-diagram.dragging {
  cursor: grabbing;
}

.er-empty {
  padding: 18px;
  color: var(--faint);
  font-size: 13px;
}

.er-stage {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.er-canvas {
  position: absolute;
  top: 0;
  left: 0;
  min-width: 100%;
  min-height: 100%;
  transform-origin: 0 0;
}

.er-lines {
  position: absolute;
  inset: 0;
  overflow: visible;
  fill: var(--muted);
  pointer-events: none;
}

.er-edge {
  fill: none;
  stroke: #c7ccd3;
  stroke-width: 1.2;
}

.er-node {
  position: absolute;
  display: flex;
  overflow: hidden;
  flex-direction: column;
  padding: 0 0 10px;
  border: 1px solid var(--line-strong);
  border-radius: 6px;
  background: #fff;
  color: var(--text);
  cursor: pointer;
  text-align: left;
  box-shadow: none;
  touch-action: none;
}

.er-node.moving {
  border-color: #d58a70;
  box-shadow: 0 8px 18px rgba(24, 27, 35, 0.08);
}

.er-node__title {
  flex: 0 0 32px;
  display: block;
  overflow: hidden;
  height: 32px;
  padding: 7px 10px;
  border-bottom: 1px solid var(--line);
  background: var(--surface-muted);
  color: var(--text);
  font-size: 13px;
  font-weight: 760;
  line-height: 18px;
  text-align: left;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.er-node__body {
  min-height: 0;
  flex: 1;
  overflow: auto;
  padding-bottom: 10px;
}

.er-field {
  flex: 0 0 auto;
  display: grid;
  grid-template-columns: 18px minmax(0, auto) minmax(0, 1fr);
  align-items: center;
  gap: 3px;
  min-height: 22px;
  padding: 0 10px;
  color: var(--text);
  font-size: 12px;
}

.er-field i {
  width: 10px;
  height: 10px;
  border: 2px solid #a8b0b8;
  transform: rotate(45deg) scale(0.72);
}

.er-field.primary i,
.er-field.indexed i {
  border-color: #d58a70;
  background: #d58a70;
  border-radius: 50%;
  transform: none;
}

.er-field span,
.er-field em,
.er-more {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.er-field em {
  color: var(--faint);
  font-style: normal;
}

.er-more {
  flex: 0 0 auto;
  padding: 4px 10px 0 30px;
  color: var(--faint);
  font-size: 12px;
}

.er-resize-handle {
  position: absolute;
  right: 0;
  bottom: 0;
  width: 14px;
  height: 14px;
  cursor: nwse-resize;
}

.er-resize-handle::before {
  position: absolute;
  right: 4px;
  bottom: 4px;
  width: 7px;
  height: 7px;
  border-right: 1px solid var(--faint);
  border-bottom: 1px solid var(--faint);
  content: "";
}
</style>
