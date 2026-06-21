<script setup>
import { computed } from "vue";
import { CopyDocument, Document, Refresh, Right } from "@element-plus/icons-vue";

const props = defineProps({
  open: { type: Boolean, default: false },
  title: { type: String, default: "DDL" },
  ddl: { type: String, default: "" },
  loading: { type: Boolean, default: false },
  error: { type: String, default: "" },
});

const emit = defineEmits(["toggle", "refresh", "copy"]);

const SQL_KEYWORDS = new Set([
  "add",
  "alter",
  "auto_increment",
  "charset",
  "check",
  "collate",
  "comment",
  "constraint",
  "create",
  "current_timestamp",
  "default",
  "delete",
  "engine",
  "exists",
  "foreign",
  "if",
  "index",
  "insert",
  "key",
  "not",
  "null",
  "on",
  "primary",
  "references",
  "table",
  "unique",
  "update",
]);

const highlightedDdl = computed(() => tokenizeSql(props.ddl || "暂无 DDL"));

function tokenizeSql(sql) {
  const tokens = [];
  const pattern = /(--[^\n]*|\/\*[\s\S]*?\*\/|`(?:``|[^`])*`|'(?:''|\\'|[^'])*'|"(?:\\"|[^"])*"|\b\d+(?:\.\d+)?\b|\b[A-Za-z_][\w$]*\b|[(),.;=<>+\-*/]|\s+|.)/g;

  for (const match of sql.matchAll(pattern)) {
    const text = match[0];
    const lowerText = text.toLowerCase();
    let type = "plain";

    if (/^\s+$/.test(text)) {
      type = "space";
    } else if (text.startsWith("--") || text.startsWith("/*")) {
      type = "comment";
    } else if (text.startsWith("`")) {
      type = "identifier";
    } else if (text.startsWith("'") || text.startsWith('"')) {
      type = "string";
    } else if (/^\d/.test(text)) {
      type = "number";
    } else if (SQL_KEYWORDS.has(lowerText)) {
      type = "keyword";
    } else if (/^[(),.;=<>+\-*/]$/.test(text)) {
      type = "operator";
    }

    tokens.push({ text, type });
  }

  return tokens;
}
</script>

<template>
  <aside class="ddl-panel" :class="{ open }">
    <button type="button" class="ddl-panel__toggle" :title="open ? '收起 DDL' : '展开 DDL'" @click="emit('toggle')">
      <el-icon>
        <Right v-if="open" />
        <Document v-else />
      </el-icon>
    </button>
    <template v-if="open">
      <header class="ddl-panel__header">
        <strong>{{ title }}</strong>
        <div class="ddl-panel__actions">
          <button type="button" title="刷新 DDL" :disabled="loading" @click="emit('refresh')">
            <el-icon><Refresh /></el-icon>
          </button>
          <button type="button" title="复制 DDL" :disabled="!ddl" @click="emit('copy')">
            <el-icon><CopyDocument /></el-icon>
          </button>
        </div>
      </header>
      <div class="ddl-panel__body">
        <div v-if="loading" class="ddl-panel__state">加载中</div>
        <div v-else-if="error" class="ddl-panel__state error">{{ error }}</div>
        <pre v-else class="ddl-panel__code"><span
          v-for="(token, index) in highlightedDdl"
          :key="index"
          :class="`ddl-token ddl-token--${token.type}`"
        >{{ token.text }}</span></pre>
      </div>
    </template>
  </aside>
</template>

<style scoped>
.ddl-panel {
  position: relative;
  display: flex;
  width: 28px;
  min-width: 28px;
  flex: 0 0 28px;
  flex-direction: column;
  border-left: 1px solid var(--line);
  background: var(--surface-muted);
}

.ddl-panel.open {
  width: min(42%, 520px);
  min-width: 320px;
  flex-basis: min(42%, 520px);
  background: #fff;
}

.ddl-panel__toggle {
  display: grid;
  place-items: center;
  width: 28px;
  height: 34px;
  padding: 0;
  border: 0;
  border-bottom: 1px solid var(--line);
  background: transparent;
  color: var(--muted);
  cursor: pointer;
}

.ddl-panel.open .ddl-panel__toggle {
  position: absolute;
  top: 0;
  left: -28px;
  z-index: 2;
  border-left: 1px solid var(--line);
  background: var(--surface-muted);
}

.ddl-panel__toggle:hover {
  background: var(--surface-strong);
  color: var(--text);
}

.ddl-panel__header {
  display: flex;
  align-items: center;
  min-height: 34px;
  padding: 0 8px 0 12px;
  border-bottom: 1px solid var(--line);
  background: var(--surface-muted);
}

.ddl-panel__header strong {
  min-width: 0;
  overflow: hidden;
  color: var(--text);
  font-size: 12px;
  font-weight: 680;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ddl-panel__actions {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-left: auto;
}

.ddl-panel__actions button {
  display: grid;
  place-items: center;
  width: 22px;
  height: 22px;
  padding: 0;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
}

.ddl-panel__actions button:hover:not(:disabled) {
  background: var(--surface-strong);
  color: var(--text);
}

.ddl-panel__actions button:disabled {
  color: var(--faint);
  cursor: default;
}

.ddl-panel__body {
  min-height: 0;
  flex: 1;
  overflow: auto;
  background: #fff;
}

.ddl-panel__code {
  min-width: max-content;
  margin: 0;
  padding: 12px;
  color: #303647;
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
  font-size: 12px;
  line-height: 1.6;
  white-space: pre;
}

.ddl-token--keyword {
  color: #b44b1e;
  font-weight: 700;
}

.ddl-token--identifier {
  color: #1f6f8b;
}

.ddl-token--string {
  color: #1b7f3a;
}

.ddl-token--number {
  color: #7b4fb3;
}

.ddl-token--comment {
  color: #8a9099;
  font-style: italic;
}

.ddl-token--operator {
  color: #6f7480;
}

.ddl-panel__state {
  display: grid;
  place-items: center;
  min-height: 120px;
  padding: 16px;
  color: var(--muted);
  font-size: 12px;
}

.ddl-panel__state.error {
  color: #c53d2d;
}
</style>
