<script setup>
import { computed } from "vue";
import { Delete, Plus, RefreshLeft } from "@element-plus/icons-vue";
import {
  defaultDesignColumn,
  defaultDesignCheck,
  defaultDesignForeignKey,
  defaultDesignIndex,
  defaultDesignTrigger,
  MYSQL_COLUMN_TYPE_OPTIONS,
  SQLITE_COLUMN_TYPE_OPTIONS,
  TABLE_DESIGN_SECTIONS,
} from "./databaseTableDesigner";

const props = defineProps({
  schema: { type: String, required: true },
  engine: { type: String, default: "mysql" },
  sqlError: { type: String, default: "" },
  sqlPreview: { type: String, default: "" },
  state: { type: Object, default: null },
});

const emit = defineEmits(["save"]);
const activeSection = defineModel("section", { type: String, default: "fields" });
const visibleColumns = computed(() => props.state?.columns ?? []);
const visibleIndexes = computed(() => props.state?.indexes ?? []);
const visibleForeignKeys = computed(() => props.state?.foreignKeys ?? []);
const visibleTriggers = computed(() => props.state?.triggers ?? []);
const visibleChecks = computed(() => props.state?.checks ?? []);
const columnTypeOptions = computed(() => props.engine === "sqlite" ? SQLITE_COLUMN_TYPE_OPTIONS : MYSQL_COLUMN_TYPE_OPTIONS);
const sqlPreviewText = computed(() => {
  if (props.sqlError) {
    return props.sqlError;
  }
  return props.sqlPreview || "没有结构变更";
});

function addColumn() {
  props.state?.columns.push(defaultDesignColumn());
}

function addIndex() {
  props.state?.indexes.push(defaultDesignIndex());
}

function addForeignKey() {
  props.state?.foreignKeys.push(defaultDesignForeignKey());
}

function addTrigger() {
  props.state?.triggers.push(defaultDesignTrigger());
}

function addCheck() {
  props.state?.checks.push(defaultDesignCheck());
}

function removeColumn(index) {
  const column = props.state?.columns?.[index];
  if (!props.state || !column) {
    return;
  }

  if (column.original) {
    column.dropped = !column.dropped;
  } else {
    props.state.columns.splice(index, 1);
  }
}

function removeIndex(index) {
  const item = props.state?.indexes?.[index];
  if (!props.state || !item) {
    return;
  }

  if (item.original) {
    item.dropped = !item.dropped;
  } else {
    props.state.indexes.splice(index, 1);
  }
}

function toggleDropped(listName, index) {
  const item = props.state?.[listName]?.[index];
  if (!props.state || !item) {
    return;
  }

  if (item.original) {
    item.dropped = !item.dropped;
  } else {
    props.state[listName].splice(index, 1);
  }
}

function toggleCheck(target, key, disabled = false) {
  if (disabled || !target) {
    return;
  }
  target[key] = !target[key];
}

function handlePrimaryChange(column) {
  if (column.primary) {
    column.nullable = false;
    column.key = "PRIMARY";
  } else if (column.key === "PRIMARY") {
    column.key = "";
  }
}

function handleColumnTypeChange(column) {
  const match = String(column.typeName ?? "").match(/^(.+?)\(([^)]*)\)$/);
  if (!match) {
    return;
  }

  const [length = "", scale = ""] = match[2].split(",").map((item) => item.trim());
  column.typeName = match[1].trim().toUpperCase();
  column.length = length;
  column.scale = scale;
}

function handleAutoIncrementChange(column) {
  if (column.autoIncrement) {
    column.nullable = false;
    column.primary = true;
    column.key = "PRIMARY";
    if (!/int/i.test(column.typeName)) {
      column.typeName = props.engine === "sqlite" ? "INTEGER" : "BIGINT UNSIGNED";
      column.length = "";
      column.scale = "";
    }
  }
}
</script>

<template>
  <section class="tab-content table-designer">
    <header class="table-designer__bar">
      <div class="table-designer__identity">
        <span>{{ schema }}</span>
        <el-input
          v-if="state"
          v-model="state.tableName"
          class="table-name-input"
          size="small"
          placeholder="表名"
        />
      </div>
      <div class="table-designer__actions">
        <el-button v-if="activeSection === 'fields'" :icon="Plus" size="small" @click="addColumn">添加字段</el-button>
        <el-button v-if="activeSection === 'indexes'" :icon="Plus" size="small" @click="addIndex">添加索引</el-button>
        <el-button v-if="activeSection === 'foreignKeys'" :icon="Plus" size="small" @click="addForeignKey">添加外键</el-button>
        <el-button v-if="activeSection === 'triggers'" :icon="Plus" size="small" @click="addTrigger">添加触发器</el-button>
        <el-button v-if="activeSection === 'checks'" :icon="Plus" size="small" @click="addCheck">添加检查</el-button>
        <el-button
          type="primary"
          size="small"
          :disabled="Boolean(sqlError)"
          :loading="state?.saving"
          @click="emit('save')"
        >
          保存
        </el-button>
      </div>
    </header>
    <nav class="designer-tabs" aria-label="表设计区域">
      <button
        v-for="section in TABLE_DESIGN_SECTIONS"
        :key="section.key"
        type="button"
        :class="{ active: activeSection === section.key }"
        @click="activeSection = section.key"
      >
        {{ section.label }}
      </button>
    </nav>
    <div v-if="activeSection === 'fields'" class="design-grid-wrap">
      <div class="design-grid fields-grid">
        <div class="design-grid__head">字段名</div>
        <div class="design-grid__head">类型</div>
        <div class="design-grid__head">长度</div>
        <div class="design-grid__head">小数点</div>
        <div class="design-grid__head center">不是 Null</div>
        <div class="design-grid__head center">虚拟</div>
        <div class="design-grid__head">键</div>
        <div class="design-grid__head center">自增</div>
        <div class="design-grid__head">默认值</div>
        <div class="design-grid__head">注释</div>
        <div class="design-grid__head center">操作</div>
        <template v-for="(column, index) in visibleColumns" :key="`${column.originalName || 'new'}-${index}`">
          <div class="design-grid__cell" :class="{ dropped: column.dropped }">
            <el-input
              v-model="column.name"
              size="small"
              placeholder="name"
              :disabled="column.dropped"
            />
          </div>
          <div class="design-grid__cell" :class="{ dropped: column.dropped }">
            <el-select
              v-model="column.typeName"
              size="small"
              filterable
            allow-create
            default-first-option
            placeholder="选择或输入类型"
            popper-class="table-designer-type-popper"
            :disabled="column.dropped"
            @change="handleColumnTypeChange(column)"
          >
              <el-option
                v-for="type in columnTypeOptions"
                :key="type"
                :label="type"
                :value="type"
              />
            </el-select>
          </div>
          <div class="design-grid__cell" :class="{ dropped: column.dropped }">
            <el-input
              v-model="column.length"
              size="small"
              placeholder="长度"
              :disabled="column.dropped"
            />
          </div>
          <div class="design-grid__cell" :class="{ dropped: column.dropped }">
            <el-input
              v-model="column.scale"
              size="small"
              placeholder="小数"
              :disabled="column.dropped"
            />
          </div>
          <div class="design-grid__check" :class="{ dropped: column.dropped }">
            <button
              class="designer-check"
              :class="{ checked: !column.nullable }"
              type="button"
              :disabled="column.dropped || column.primary || column.autoIncrement"
              aria-label="不是 Null"
              @click="column.nullable = !column.nullable"
            />
          </div>
          <div class="design-grid__check" :class="{ dropped: column.dropped }">
            <button
              class="designer-check"
              :class="{ checked: column.virtual }"
              type="button"
              :disabled="column.dropped"
              aria-label="虚拟"
              @click="toggleCheck(column, 'virtual', column.dropped)"
            />
          </div>
          <div class="design-grid__cell" :class="{ dropped: column.dropped }">
            <el-select
              v-model="column.key"
              size="small"
              placeholder=""
              popper-class="table-designer-type-popper"
              :disabled="column.dropped"
              @change="column.primary = column.key === 'PRIMARY'; handlePrimaryChange(column)"
            >
              <el-option label="" value="" />
              <el-option label="PRIMARY" value="PRIMARY" />
            </el-select>
          </div>
          <div class="design-grid__check" :class="{ dropped: column.dropped }">
            <button
              class="designer-check"
              :class="{ checked: column.autoIncrement }"
              type="button"
              :disabled="column.dropped"
              aria-label="自增"
              @click="toggleCheck(column, 'autoIncrement', column.dropped); handleAutoIncrementChange(column)"
            />
          </div>
          <div class="design-grid__cell" :class="{ dropped: column.dropped }">
            <el-input
              v-model="column.defaultValue"
              size="small"
              placeholder="NULL / 0 / 文本"
              :disabled="column.dropped"
            />
          </div>
          <div class="design-grid__cell" :class="{ dropped: column.dropped }">
            <el-input
              v-model="column.comment"
              size="small"
              placeholder="注释"
              :disabled="column.dropped"
            />
          </div>
          <div class="design-grid__actions-cell" :class="{ dropped: column.dropped }">
            <button
              class="design-grid__icon-button"
              :class="{ 'is-delete': !column.dropped, 'is-restore': column.original && column.dropped }"
              type="button"
              :title="column.original && column.dropped ? '撤销删除字段' : '删除字段'"
              :aria-label="column.original && column.dropped ? '撤销删除字段' : '删除字段'"
              @click="removeColumn(index)"
            >
              <el-icon>
                <RefreshLeft v-if="column.original && column.dropped" />
                <Delete v-else />
              </el-icon>
            </button>
          </div>
        </template>
      </div>
    </div>
    <div v-else-if="activeSection === 'indexes'" class="design-grid-wrap">
      <div class="design-grid indexes-grid">
        <div class="design-grid__head">名称</div>
        <div class="design-grid__head center">唯一</div>
        <div class="design-grid__head">字段</div>
        <div class="design-grid__head">类型</div>
        <div class="design-grid__head center">操作</div>
        <template v-for="(indexItem, index) in visibleIndexes" :key="`${indexItem.originalName || 'new-index'}-${index}`">
          <div class="design-grid__cell" :class="{ dropped: indexItem.dropped }">
            <el-input v-model="indexItem.name" size="small" placeholder="idx_name" :disabled="indexItem.dropped" />
          </div>
          <div class="design-grid__check" :class="{ dropped: indexItem.dropped }">
            <button
              class="designer-check"
              :class="{ checked: indexItem.unique }"
              type="button"
              :disabled="indexItem.dropped"
              aria-label="唯一"
              @click="toggleCheck(indexItem, 'unique', indexItem.dropped)"
            />
          </div>
          <div class="design-grid__cell" :class="{ dropped: indexItem.dropped }">
            <el-input v-model="indexItem.columns" size="small" placeholder="字段名，多个用逗号分隔" :disabled="indexItem.dropped" />
          </div>
          <div class="design-grid__cell" :class="{ dropped: indexItem.dropped }">
            <el-select v-model="indexItem.type" size="small" popper-class="table-designer-type-popper" :disabled="indexItem.dropped">
              <el-option label="BTREE" value="BTREE" />
              <el-option label="HASH" value="HASH" />
            </el-select>
          </div>
          <div class="design-grid__actions-cell" :class="{ dropped: indexItem.dropped }">
            <button
              class="design-grid__icon-button"
              :class="{ 'is-delete': !indexItem.dropped, 'is-restore': indexItem.original && indexItem.dropped }"
              type="button"
              :title="indexItem.original && indexItem.dropped ? '撤销删除索引' : '删除索引'"
              :aria-label="indexItem.original && indexItem.dropped ? '撤销删除索引' : '删除索引'"
              @click="removeIndex(index)"
            >
              <el-icon>
                <RefreshLeft v-if="indexItem.original && indexItem.dropped" />
                <Delete v-else />
              </el-icon>
            </button>
          </div>
        </template>
      </div>
    </div>
    <div v-else-if="activeSection === 'options'" class="designer-form">
      <label>
        <span>引擎</span>
        <el-select v-model="state.engine" size="small" popper-class="table-designer-type-popper">
          <el-option label="InnoDB" value="InnoDB" />
          <el-option label="MyISAM" value="MyISAM" />
          <el-option label="MEMORY" value="MEMORY" />
        </el-select>
      </label>
      <label>
        <span>字符集</span>
        <el-select v-model="state.charset" size="small" filterable allow-create popper-class="table-designer-type-popper">
          <el-option label="utf8mb4" value="utf8mb4" />
          <el-option label="utf8" value="utf8" />
          <el-option label="latin1" value="latin1" />
        </el-select>
      </label>
      <label>
        <span>排序规则</span>
        <el-select v-model="state.collation" size="small" filterable allow-create popper-class="table-designer-type-popper">
          <el-option label="utf8mb4_unicode_ci" value="utf8mb4_unicode_ci" />
          <el-option label="utf8mb4_general_ci" value="utf8mb4_general_ci" />
          <el-option label="utf8mb4_0900_ai_ci" value="utf8mb4_0900_ai_ci" />
        </el-select>
      </label>
    </div>
    <div v-else-if="activeSection === 'foreignKeys'" class="design-grid-wrap">
      <div class="design-grid foreign-keys-grid">
        <div class="design-grid__head">名称</div>
        <div class="design-grid__head">字段</div>
        <div class="design-grid__head">引用表</div>
        <div class="design-grid__head">引用字段</div>
        <div class="design-grid__head">删除时</div>
        <div class="design-grid__head">更新时</div>
        <div class="design-grid__head center">操作</div>
        <template v-for="(foreignKey, index) in visibleForeignKeys" :key="`${foreignKey.originalName || 'new-fk'}-${index}`">
          <div class="design-grid__cell" :class="{ dropped: foreignKey.dropped }"><el-input v-model="foreignKey.name" size="small" :disabled="foreignKey.dropped" /></div>
          <div class="design-grid__cell" :class="{ dropped: foreignKey.dropped }"><el-input v-model="foreignKey.columns" size="small" placeholder="字段，逗号分隔" :disabled="foreignKey.dropped" /></div>
          <div class="design-grid__cell" :class="{ dropped: foreignKey.dropped }"><el-input v-model="foreignKey.referencedTable" size="small" :disabled="foreignKey.dropped" /></div>
          <div class="design-grid__cell" :class="{ dropped: foreignKey.dropped }"><el-input v-model="foreignKey.referencedColumns" size="small" placeholder="字段，逗号分隔" :disabled="foreignKey.dropped" /></div>
          <div class="design-grid__cell" :class="{ dropped: foreignKey.dropped }">
            <el-select v-model="foreignKey.onDelete" size="small" popper-class="table-designer-type-popper" :disabled="foreignKey.dropped">
              <el-option label="RESTRICT" value="RESTRICT" />
              <el-option label="CASCADE" value="CASCADE" />
              <el-option label="SET NULL" value="SET NULL" />
              <el-option label="NO ACTION" value="NO ACTION" />
            </el-select>
          </div>
          <div class="design-grid__cell" :class="{ dropped: foreignKey.dropped }">
            <el-select v-model="foreignKey.onUpdate" size="small" popper-class="table-designer-type-popper" :disabled="foreignKey.dropped">
              <el-option label="RESTRICT" value="RESTRICT" />
              <el-option label="CASCADE" value="CASCADE" />
              <el-option label="SET NULL" value="SET NULL" />
              <el-option label="NO ACTION" value="NO ACTION" />
            </el-select>
          </div>
          <div class="design-grid__actions-cell" :class="{ dropped: foreignKey.dropped }">
            <button
              class="design-grid__icon-button"
              :class="{ 'is-delete': !foreignKey.dropped, 'is-restore': foreignKey.original && foreignKey.dropped }"
              type="button"
              :title="foreignKey.original && foreignKey.dropped ? '撤销删除外键' : '删除外键'"
              :aria-label="foreignKey.original && foreignKey.dropped ? '撤销删除外键' : '删除外键'"
              @click="toggleDropped('foreignKeys', index)"
            >
              <el-icon><RefreshLeft v-if="foreignKey.original && foreignKey.dropped" /><Delete v-else /></el-icon>
            </button>
          </div>
        </template>
      </div>
    </div>
    <div v-else-if="activeSection === 'triggers'" class="design-grid-wrap">
      <div class="design-grid triggers-grid">
        <div class="design-grid__head">名称</div>
        <div class="design-grid__head">时机</div>
        <div class="design-grid__head">事件</div>
        <div class="design-grid__head">语句</div>
        <div class="design-grid__head center">操作</div>
        <template v-for="(trigger, index) in visibleTriggers" :key="`${trigger.originalName || 'new-trigger'}-${index}`">
          <div class="design-grid__cell" :class="{ dropped: trigger.dropped }"><el-input v-model="trigger.name" size="small" :disabled="trigger.dropped" /></div>
          <div class="design-grid__cell" :class="{ dropped: trigger.dropped }">
            <el-select v-model="trigger.timing" size="small" popper-class="table-designer-type-popper" :disabled="trigger.dropped">
              <el-option label="BEFORE" value="BEFORE" />
              <el-option label="AFTER" value="AFTER" />
            </el-select>
          </div>
          <div class="design-grid__cell" :class="{ dropped: trigger.dropped }">
            <el-select v-model="trigger.event" size="small" popper-class="table-designer-type-popper" :disabled="trigger.dropped">
              <el-option label="INSERT" value="INSERT" />
              <el-option label="UPDATE" value="UPDATE" />
              <el-option label="DELETE" value="DELETE" />
            </el-select>
          </div>
          <div class="design-grid__cell" :class="{ dropped: trigger.dropped }"><el-input v-model="trigger.statement" size="small" placeholder="BEGIN ... END 或 SET NEW.xxx = ..." :disabled="trigger.dropped" /></div>
          <div class="design-grid__actions-cell" :class="{ dropped: trigger.dropped }">
            <button
              class="design-grid__icon-button"
              :class="{ 'is-delete': !trigger.dropped, 'is-restore': trigger.original && trigger.dropped }"
              type="button"
              :title="trigger.original && trigger.dropped ? '撤销删除触发器' : '删除触发器'"
              :aria-label="trigger.original && trigger.dropped ? '撤销删除触发器' : '删除触发器'"
              @click="toggleDropped('triggers', index)"
            >
              <el-icon><RefreshLeft v-if="trigger.original && trigger.dropped" /><Delete v-else /></el-icon>
            </button>
          </div>
        </template>
      </div>
    </div>
    <div v-else-if="activeSection === 'checks'" class="design-grid-wrap">
      <div class="design-grid checks-grid">
        <div class="design-grid__head">名称</div>
        <div class="design-grid__head">表达式</div>
        <div class="design-grid__head center">启用</div>
        <div class="design-grid__head center">操作</div>
        <template v-for="(check, index) in visibleChecks" :key="`${check.originalName || 'new-check'}-${index}`">
          <div class="design-grid__cell" :class="{ dropped: check.dropped }"><el-input v-model="check.name" size="small" :disabled="check.dropped" /></div>
          <div class="design-grid__cell" :class="{ dropped: check.dropped }"><el-input v-model="check.expression" size="small" placeholder="price >= 0" :disabled="check.dropped" /></div>
          <div class="design-grid__check" :class="{ dropped: check.dropped }">
            <button
              class="designer-check"
              :class="{ checked: check.enforced }"
              type="button"
              :disabled="check.dropped"
              aria-label="启用检查"
              @click="toggleCheck(check, 'enforced', check.dropped)"
            />
          </div>
          <div class="design-grid__actions-cell" :class="{ dropped: check.dropped }">
            <button
              class="design-grid__icon-button"
              :class="{ 'is-delete': !check.dropped, 'is-restore': check.original && check.dropped }"
              type="button"
              :title="check.original && check.dropped ? '撤销删除检查' : '删除检查'"
              :aria-label="check.original && check.dropped ? '撤销删除检查' : '删除检查'"
              @click="toggleDropped('checks', index)"
            >
              <el-icon><RefreshLeft v-if="check.original && check.dropped" /><Delete v-else /></el-icon>
            </button>
          </div>
        </template>
      </div>
    </div>
    <div v-else-if="activeSection === 'comment'" class="designer-form single">
      <label>
        <span>表注释</span>
        <el-input v-model="state.tableComment" type="textarea" :rows="6" />
      </label>
    </div>
    <section v-else-if="activeSection === 'sql'" class="sql-preview full" :class="{ invalid: sqlError }">
      <header>
        <span>{{ state?.mode === 'create' ? '创建 SQL' : '变更 SQL' }}</span>
      </header>
      <pre>{{ sqlPreviewText }}</pre>
    </section>
  </section>
</template>

<style scoped>
.table-designer {
  display: grid;
  grid-template-rows: auto 42px minmax(0, 1fr);
  min-height: 0;
  height: 100%;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: #fff;
  box-shadow: none;
}

.designer-tabs {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  min-height: 38px;
  padding: 5px 8px;
  overflow-x: auto;
  border-bottom: 1px solid var(--line);
  background: var(--surface-muted);
}

.designer-tabs button {
  height: 28px;
  padding: 0 9px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  font: inherit;
  font-size: 12px;
  font-weight: 650;
  white-space: nowrap;
}

.designer-tabs button:hover {
  background: var(--surface-strong);
  color: var(--text);
}

.designer-tabs button.active {
  border-color: #f5c5b3;
  background: var(--orange-soft);
  color: var(--orange);
}

.table-designer__bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 44px;
  gap: 12px;
  padding: 7px 10px;
  border-bottom: 1px solid var(--line);
  background: var(--surface-muted);
}

.table-designer__identity {
  display: flex;
  align-items: center;
  min-width: 0;
  gap: 8px;
}

.table-designer__identity span {
  color: var(--muted);
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
  font-size: 12px;
}

.table-designer__identity :deep(.el-input) {
  width: 240px;
}

.table-designer__identity :deep(.el-input__wrapper),
.design-grid__cell :deep(.el-input__wrapper),
.design-grid__cell :deep(.el-select__wrapper) {
  height: 26px;
  min-height: 26px;
  padding: 0 8px;
  border-radius: 6px;
  background: transparent;
  box-shadow: 0 0 0 1px transparent inset;
}

.table-designer__identity :deep(.el-input__wrapper:hover),
.design-grid__cell :deep(.el-input__wrapper:hover),
.design-grid__cell :deep(.el-input__wrapper.is-focus),
.design-grid__cell :deep(.el-select__wrapper:hover),
.design-grid__cell :deep(.el-select__wrapper.is-focused) {
  box-shadow: 0 0 0 1px var(--line-strong) inset;
}

.design-grid__cell :deep(.el-input__inner),
.design-grid__cell :deep(.el-select__selected-item),
.design-grid__cell :deep(.el-select__placeholder) {
  color: var(--text);
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
  font-size: 12px;
  font-weight: 400;
}

.design-grid__cell :deep(.el-select__placeholder) {
  color: var(--faint);
}

.design-grid__cell :deep(.el-select__caret) {
  width: 14px;
  color: var(--faint);
  font-size: 12px;
}

.design-grid__cell :deep(.el-select__wrapper.is-focused) {
  box-shadow: 0 0 0 1px var(--orange) inset, 0 0 0 3px rgba(242, 107, 58, 0.10);
}

.table-designer__actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.table-designer__actions :deep(.el-button) {
  height: 28px;
  margin: 0;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: #fff;
  color: var(--text);
  font-size: 12px;
  font-weight: 600;
  box-shadow: none;
}

.table-designer__actions :deep(.el-button:hover) {
  border-color: var(--line-strong);
  background: var(--surface-strong);
  color: var(--text);
}

.table-designer__actions :deep(.el-button--primary) {
  border-color: var(--orange);
  background: var(--orange);
  color: #fff;
}

.table-designer__actions :deep(.el-button--primary:hover) {
  border-color: #e65d2e;
  background: #e65d2e;
  color: #fff;
}

.design-grid-wrap {
  min-width: 0;
  min-height: 0;
  height: 100%;
  overflow: auto;
  background: #fff;
}

.design-grid {
  display: grid;
  min-height: 100%;
  height: 100%;
  align-content: start;
  background: #fff;
}

.fields-grid {
  grid-template-columns: 180px 170px 90px 90px 92px 70px 110px 70px 160px minmax(180px, 1fr) 66px;
  min-width: 1288px;
  width: max(1288px, 100%);
}

.indexes-grid {
  grid-template-columns: 220px 82px minmax(280px, 1fr) 130px 66px;
  min-width: 780px;
  width: max(780px, 100%);
}

.foreign-keys-grid {
  grid-template-columns: 190px 210px 190px 210px 120px 120px 66px;
  min-width: 1106px;
  width: max(1106px, 100%);
}

.triggers-grid {
  grid-template-columns: 200px 120px 120px minmax(360px, 1fr) 66px;
  min-width: 866px;
  width: max(866px, 100%);
}

.checks-grid {
  grid-template-columns: 220px minmax(360px, 1fr) 82px 66px;
  min-width: 728px;
  width: max(728px, 100%);
}

.design-grid__head {
  position: sticky;
  top: 0;
  z-index: 1;
  min-height: 34px;
  padding: 0 10px;
  border-right: 1px solid var(--line);
  border-bottom: 1px solid var(--line);
  background: var(--surface-muted);
  color: var(--muted);
  font-size: 12px;
  font-weight: 400;
  line-height: 34px;
}

.design-grid__head.center {
  text-align: center;
}

.design-grid__cell,
.design-grid__check,
.design-grid__actions-cell {
  min-height: 34px;
  border-right: 1px solid var(--line);
  border-bottom: 1px solid var(--line);
  background: #fff;
}

.design-grid__cell {
  display: flex;
  align-items: center;
  padding: 3px 8px;
}

.design-grid__cell :deep(.el-input),
.design-grid__cell :deep(.el-select) {
  width: 100%;
}

.design-grid__check {
  display: flex;
  align-items: center;
  justify-content: center;
}

.designer-check {
  position: relative;
  width: 15px;
  height: 15px;
  padding: 0;
  border: 1px solid var(--line-strong);
  border-radius: 4px;
  background: #fff;
  cursor: pointer;
  appearance: none;
}

.designer-check::after {
  position: absolute;
  top: 1px;
  left: 4.5px;
  width: 3px;
  height: 7px;
  border: 1.5px solid #fff;
  border-top: 0;
  border-left: 0;
  content: "";
  opacity: 0;
  transform: rotate(45deg) scale(0.8);
}

.designer-check.checked {
  border-color: var(--orange);
  background: var(--orange);
}

.designer-check.checked::after {
  opacity: 1;
}

.designer-check:hover:not(:disabled) {
  border-color: var(--orange);
  box-shadow: 0 0 0 3px rgba(242, 107, 58, 0.10);
}

.designer-check:disabled {
  border-color: var(--line);
  background: var(--surface-muted);
  cursor: default;
}

.designer-check.checked:disabled {
  border-color: #f5c5b3;
  background: #f7b199;
}

.design-grid__actions-cell {
  display: flex;
  align-items: center;
  justify-content: center;
}

.design-grid__cell.dropped,
.design-grid__check.dropped,
.design-grid__actions-cell.dropped {
  background: #fff;
}

.design-grid__cell.dropped {
  box-shadow: none;
}

.fields-grid .design-grid__cell.dropped:nth-child(11n + 12),
.indexes-grid .design-grid__cell.dropped:nth-child(5n + 6),
.foreign-keys-grid .design-grid__cell.dropped:nth-child(7n + 8),
.triggers-grid .design-grid__cell.dropped:nth-child(5n + 6),
.checks-grid .design-grid__cell.dropped:nth-child(4n + 5) {
  box-shadow: inset 2px 0 0 #f5b5ad;
}

.design-grid__icon-button {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--faint);
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease, opacity 0.12s ease;
  appearance: none;
}

.design-grid__icon-button .el-icon {
  font-size: 13px;
}

.design-grid__actions-cell:not(.dropped) .design-grid__icon-button.is-delete {
  opacity: 0.35;
}

.design-grid__actions-cell:not(.dropped):hover .design-grid__icon-button.is-delete,
.design-grid__icon-button:focus-visible {
  opacity: 1;
}

.design-grid__icon-button:focus-visible {
  outline: none;
  box-shadow: inset 0 0 0 1px var(--orange);
}

.design-grid__icon-button:hover {
  background: var(--surface-strong);
  color: var(--text);
}

.design-grid__icon-button.is-delete:hover {
  background: #fff6f5;
  color: var(--red);
}

.design-grid__icon-button.is-restore {
  border-color: transparent;
  background: transparent;
  color: var(--orange);
  opacity: 0.8;
}

.design-grid__icon-button.is-restore:hover {
  background: var(--orange-soft);
  color: #c84d22;
}

.sql-preview {
  min-height: 0;
  overflow: hidden;
  border-top: 1px solid var(--line);
  background: var(--panel);
}

.sql-preview.full {
  border-top: 0;
}

.sql-preview header {
  display: flex;
  align-items: center;
  min-height: 30px;
  padding: 0 10px;
  border-bottom: 1px solid var(--line);
  background: var(--surface-muted);
}

.sql-preview header span {
  color: var(--muted);
  font-size: 12px;
  font-weight: 400;
}

.sql-preview pre {
  height: calc(100% - 30px);
  margin: 0;
  padding: 7px 10px;
  overflow: auto;
  color: #303647;
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
  font-size: 12px;
  line-height: 1.6;
  white-space: pre-wrap;
}

.designer-form {
  display: grid;
  align-content: start;
  grid-template-columns: repeat(3, minmax(180px, 260px));
  gap: 12px;
  min-height: 0;
  padding: 10px;
  overflow: auto;
  background: #fff;
}

.designer-form.single {
  grid-template-columns: minmax(260px, 620px);
}

.designer-form label {
  display: grid;
  gap: 6px;
}

.designer-form label span {
  color: var(--muted);
  font-size: 12px;
  font-weight: 400;
}

.designer-form :deep(.el-input__wrapper),
.designer-form :deep(.el-select__wrapper),
.designer-form :deep(.el-textarea__inner) {
  border-radius: 7px;
  box-shadow: 0 0 0 1px var(--line) inset;
}

.designer-form :deep(.el-input__wrapper.is-focus),
.designer-form :deep(.el-select__wrapper.is-focused),
.designer-form :deep(.el-textarea__inner:focus) {
  box-shadow: 0 0 0 1px var(--orange) inset, 0 0 0 3px rgba(242, 107, 58, 0.10);
}

.designer-empty {
  display: grid;
  place-content: center;
  gap: 8px;
  min-height: 0;
  padding: 20px;
  background: #fff;
  color: var(--faint);
  text-align: center;
}

.designer-empty span {
  color: var(--muted);
  font-size: 13px;
  font-weight: 700;
}

.designer-empty p {
  margin: 0;
  font-size: 12px;
}

.sql-preview.invalid pre {
  color: var(--red);
}
</style>
