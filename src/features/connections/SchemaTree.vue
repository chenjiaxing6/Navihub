<script setup>
import { computed, ref, watch } from "vue";

const props = defineProps({
  connectionId: { type: String, required: true },
  openSchemaKeys: { type: Array, default: () => [] },
  searchQuery: { type: String, default: "" },
  schemaOpenVersions: { type: Object, default: () => ({}) },
  schemas: { type: Array, required: true },
  loading: { type: Boolean, default: false },
});

const emit = defineEmits(["activate-schema", "open-schema", "open-table-query"]);
const selectedKey = ref("");
const openGroupKeys = ref(new Set());
const normalizedSearch = computed(() => props.searchQuery.trim().toLowerCase());
const hasSearch = computed(() => Boolean(normalizedSearch.value));
const filteredSchemas = computed(() => {
  if (!hasSearch.value) {
    return props.schemas;
  }

  const query = normalizedSearch.value;
  return props.schemas
    .map((schema) => {
      const schemaMatched = includesQuery(schema.name, query);
      const groups = (schema.groups ?? [])
        .map((group) => {
          const groupMatched = includesQuery(group.title, query) || groupType(group).includes(query);
          const items = (group.items ?? []).filter((item) => schemaMatched || groupMatched || includesQuery(itemName(item), query));
          return groupMatched || items.length > 0 ? { ...group, items, count: items.length } : null;
        })
        .filter(Boolean);

      return schemaMatched || groups.length > 0 ? { ...schema, groups } : null;
    })
    .filter(Boolean);
});

watch(
  () => props.schemaOpenVersions,
  () => {
    openGroupKeys.value = new Set();
  },
  { deep: true },
);

const folderClass = {
  table: "group-icon table-folder",
  view: "group-icon view-folder",
  query: "group-icon query-folder",
  function: "group-icon function-folder",
};

const itemIconClass = {
  table: "table-icon",
  view: "view-icon",
  query: "query-icon",
  function: "function-icon",
};

function groupType(group) {
  return group.groupType ?? group.type;
}

function schemaKey(schema) {
  return `schema:${props.connectionId}:${schema.name}`;
}

function isSchemaOpen(schema) {
  if (hasSearch.value) {
    return true;
  }

  return props.openSchemaKeys?.includes(schemaKey(schema));
}

function schemaRenderKey(schema) {
  const key = schemaKey(schema);
  return `${key}:${props.schemaOpenVersions?.[key] ?? 0}`;
}

function groupKey(schema, group) {
  return `${schemaKey(schema)}:${groupType(group)}`;
}

function isGroupOpen(schema, group) {
  if (hasSearch.value) {
    return true;
  }

  return openGroupKeys.value.has(groupKey(schema, group));
}

function toggleGroup(schema, group) {
  const key = groupKey(schema, group);
  const nextKeys = new Set(openGroupKeys.value);
  if (nextKeys.has(key)) {
    nextKeys.delete(key);
  } else {
    nextKeys.add(key);
  }
  openGroupKeys.value = nextKeys;
}

function selectOnly(key) {
  selectedKey.value = key;
}

function itemName(item) {
  return typeof item === "string" ? item : item.name;
}

function includesQuery(value, query) {
  return String(value ?? "").toLowerCase().includes(query);
}
</script>

<template>
  <div class="schema-tree">
    <div v-if="filteredSchemas.length === 0" class="schema-empty">
      {{ loading ? "正在加载库表..." : hasSearch ? "没有匹配的库表" : "未加载到库表。请确认连接账号有库表权限，或右键连接刷新重试。" }}
    </div>

    <details
      v-for="schema in filteredSchemas"
      v-else
      :key="schemaRenderKey(schema)"
      :open="isSchemaOpen(schema)"
    >
      <summary
        :class="{ opened: isSchemaOpen(schema), selected: selectedKey === schemaKey(schema) }"
        @click.prevent="() => {
          selectOnly(schemaKey(schema));
          if (isSchemaOpen(schema)) emit('activate-schema', { schema });
        }"
        @dblclick.prevent="emit('open-schema', { schema })"
      >
        <span class="schema-icon" />
        <span>{{ schema.name }}</span>
      </summary>

      <details v-for="group in schema.groups" :key="groupType(group)" :open="isGroupOpen(schema, group)">
        <summary
          :class="{ selected: selectedKey === groupKey(schema, group) }"
          @click.prevent="selectOnly(groupKey(schema, group))"
          @dblclick.prevent="toggleGroup(schema, group)"
        >
          <button
            class="tree-toggle"
            :class="{ open: isGroupOpen(schema, group) }"
            :aria-label="isGroupOpen(schema, group) ? '收起' : '展开'"
            @click.stop.prevent="toggleGroup(schema, group)"
          />
          <span :class="folderClass[groupType(group)] ?? 'group-icon'" />
          <span>{{ group.title }}</span>
          <em>{{ group.count }}</em>
        </summary>
        <button
          v-for="item in group.items"
          :key="itemName(item)"
          class="tree-item"
          :class="{ selected: selectedKey === `${schemaKey(schema)}:${groupType(group)}:${itemName(item)}` }"
          @click.prevent="selectOnly(`${schemaKey(schema)}:${groupType(group)}:${itemName(item)}`)"
          @dblclick="emit('open-table-query', { schema: schema.name, groupType: groupType(group), item: itemName(item) })"
        >
          <span class="object-icon" :class="itemIconClass[groupType(group)] ?? 'object-icon-default'" />
          {{ itemName(item) }}
        </button>
      </details>
    </details>
  </div>
</template>

<style scoped>
.schema-tree {
  margin: 2px 0 6px 30px;
  padding: 0 0 4px 8px;
  border-left: 1px solid var(--line);
}

.schema-empty {
  padding: 7px;
  color: var(--faint);
  font-size: 12px;
  line-height: 1.5;
}

details {
  margin: 0;
}

details details {
  margin-left: 17px;
}

summary::-webkit-details-marker {
  display: none;
}

summary {
  display: flex;
  align-items: center;
  gap: 6px;
  min-height: 28px;
  padding: 0 7px 0 3px;
  border-radius: 8px;
  color: var(--muted);
  cursor: pointer;
  font-size: 13px;
  list-style: none;
}

summary:hover {
  background: var(--surface-strong);
  color: var(--text);
}

summary:active {
  background: #e8e9ec;
  transform: none;
}

summary.opened {
  background: var(--orange-soft);
  color: #a8421f;
  font-weight: 760;
}

summary.selected:not(.opened) {
  background: var(--surface-strong);
}

summary em {
  margin-left: auto;
  color: var(--faint);
  font-size: 11px;
  font-style: normal;
}

.tree-toggle {
  position: relative;
  display: grid;
  place-items: center;
  width: 18px;
  height: 18px;
  flex: 0 0 18px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--faint);
  cursor: pointer;
  appearance: none;
}

.tree-toggle::before {
  width: 0;
  height: 0;
  border-top: 4px solid transparent;
  border-bottom: 4px solid transparent;
  border-left: 5px solid currentColor;
  content: "";
  transition: transform 0.12s ease;
}

.tree-toggle.open::before {
  transform: rotate(90deg);
}

.tree-item {
  display: flex;
  align-items: center;
  gap: 7px;
  width: 100%;
  min-height: 28px;
  padding: 0 7px 0 32px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  font-size: 13px;
  text-align: left;
}

.tree-item:hover {
  background: var(--surface-strong);
  color: var(--text);
}

.tree-item:active {
  background: #e8e9ec;
  transform: none;
}

.tree-item.selected {
  background: var(--orange-soft);
  color: #a8421f;
}

.schema-icon,
.group-icon,
.object-icon {
  position: relative;
  display: inline-grid;
  place-items: center;
  width: 18px;
  height: 18px;
  flex: 0 0 18px;
  color: var(--muted);
}

.schema-icon {
  border: 1px solid #d7ab9a;
  border-radius: 8px / 5px;
  background:
    linear-gradient(#fff7f4 0 45%, #ffe5dc 45% 100%);
}

summary.opened .schema-icon {
  border-color: #e9a38a;
  background:
    linear-gradient(#fff8f5 0 45%, #ffd8ca 45% 100%);
}

summary.opened .schema-icon::before,
summary.opened .schema-icon::after {
  border-color: #e9a38a;
}

.schema-icon::before,
.schema-icon::after {
  position: absolute;
  right: 2px;
  left: 2px;
  height: 5px;
  border: 1px solid #d7ab9a;
  border-radius: 50%;
  background: #fff;
  content: "";
}

.schema-icon::before {
  top: -1px;
}

.schema-icon::after {
  bottom: -1px;
  background: transparent;
}

.group-icon {
  border: 1px solid var(--line-strong);
  border-radius: 5px;
  background: #fff;
}

.group-icon::before {
  position: absolute;
  top: 3px;
  left: 2px;
  width: 7px;
  height: 3px;
  border-radius: 2px 2px 0 0;
  background: currentColor;
  opacity: 0.65;
  content: "";
}

.group-icon::after {
  position: absolute;
  right: 2px;
  bottom: 3px;
  left: 2px;
  height: 9px;
  border-radius: 3px;
  background: currentColor;
  opacity: 0.28;
  content: "";
}

.table-folder {
  color: #2563eb;
}

.view-folder {
  color: #7c3aed;
}

.query-folder {
  color: #0891b2;
}

.function-folder {
  color: #d97706;
}

.object-icon {
  border: 1px solid var(--line-strong);
  border-radius: 5px;
  background: #fff;
}

.table-icon {
  color: var(--blue);
  background: #fff;
}

.table-icon::before {
  position: absolute;
  width: 12px;
  height: 10px;
  border: 1.5px solid currentColor;
  border-radius: 2px;
  content: "";
}

.table-icon::after {
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

.view-icon {
  color: #7c3aed;
}

.view-icon::before {
  width: 13px;
  height: 8px;
  border: 1.5px solid currentColor;
  border-radius: 50% / 65%;
  content: "";
}

.view-icon::after {
  position: absolute;
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: currentColor;
  content: "";
}

.query-icon {
  color: #0891b2;
  font-size: 8px;
  font-weight: 850;
}

.query-icon::before {
  content: "SQL";
}

.function-icon {
  color: #d97706;
  font-family: Georgia, "Times New Roman", serif;
  font-size: 15px;
  font-style: italic;
  font-weight: 700;
}

.function-icon::before {
  content: "ƒ";
}

.object-icon-default::before {
  width: 8px;
  height: 8px;
  border-radius: 3px;
  background: currentColor;
  opacity: 0.45;
  content: "";
}
</style>
