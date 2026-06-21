<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, shallowRef, watch } from "vue";
import { ElMessage } from "element-plus/es/components/message/index";
import { Check, Close, DataAnalysis, DocumentChecked, Minus, Plus, Refresh, Right, Search, VideoPlay } from "@element-plus/icons-vue";
import { ElMessageBox } from "element-plus/es/components/message-box/index";
import ContextMenu from "../../shared/ContextMenu.vue";
import TableDesigner from "./TableDesigner.vue";
import { describeMysqlTable, executeMysqlQuery } from "./mysqlApi";
import { ensureMysqlConnection, formatMysqlMeta } from "./databaseDefaults";
import { SQL_COMPLETION_KEYWORDS, defaultQuerySql, quoteIdentifier, quoteString } from "./databaseQueryUtils";
import {
  buildTableDesignSql,
  applyOptionsFromTableDetail,
  checksFromTableDetail,
  columnsFromTableDetail,
  createDesignState,
  foreignKeysFromTableDetail,
  indexesFromTableDetail,
  markDesignStateSaved,
  triggersFromTableDetail,
} from "./databaseTableDesigner";
import {
  DEFAULT_PAGE_SIZE,
  PAGE_SIZE_OPTIONS,
  TABLE_COLUMN_MIN_WIDTH,
  TABLE_OVERSCAN,
  TABLE_ROW_HEIGHT,
  clampColumnWidth,
  clampRangeIndexes,
  columnMinWidth,
  copyTextForCells,
  copyTextForRows,
  emptyCellRange,
  emptyRowRange,
  formatCellValue,
  formatDataLength,
  formatRowCount,
  formatTableDate,
  normalizePositiveInteger,
  rowMatchesSearch,
  schemaTableCellValue,
  schemaTableSelectKey,
} from "./databaseTableUtils";

const props = defineProps({
  connection: { type: Object, required: true },
  activeTopTab: { type: Object, default: null },
  pendingSchemaOpen: { type: Object, default: null },
  pendingTableQuery: { type: Object, default: null },
});

const emit = defineEmits(["schema-loaded", "table-design-saved", "update-connection", "open-table-query", "refresh-connection", "save-query", "update-query-schema"]);
const normalizedConnection = computed(() => ensureMysqlConnection(props.connection));
const databaseTarget = computed(() => {
  const config = normalizedConnection.value.config;
  const database = config.database ? `/${config.database}` : "";
  return `${config.username}@${config.host}:${config.port}${database}`;
});

const form = reactive({
  name: normalizedConnection.value.name,
  host: normalizedConnection.value.config.host,
  port: normalizedConnection.value.config.port,
  username: normalizedConnection.value.config.username,
  password: normalizedConnection.value.config.password,
  database: normalizedConnection.value.config.database,
});

const loading = ref(false);
const tableResults = ref({});
const querySqlByTab = ref({});
const queryColumnCompletionsBySchema = ref({});
const schemaTableCounts = ref({});
const schemaTableMetadata = ref({});
const selectedSchemaTableKey = ref("");
const selectedSchemaRowRange = ref({ start: null, end: null });
const selectedSchemaCellRange = ref({ startRow: null, endRow: null, startColumn: null, endColumn: null });
const isSelectingSchemaRows = ref(false);
const isSelectingSchemaCells = ref(false);
const selectedResultRowRange = ref({ start: null, end: null });
const selectedResultCellRange = ref({ startRow: null, endRow: null, startColumn: null, endColumn: null });
const isSelectingResultRows = ref(false);
const isSelectingResultCells = ref(false);
const tableEditStates = ref({});
const tableColumnWidths = ref({});
const tabViewStates = ref({});
const resizingColumn = ref(null);
const copyContextOpen = ref(false);
const copyContextPosition = ref({ x: 0, y: 0 });
const tableDetailCache = ref({});
const editingCell = ref(null);
const schemaTableScrollLeft = ref(0);
const schemaTableScrollTop = ref(0);
const schemaTableViewport = ref(null);
const tableViewport = ref(null);
const queryEditorRoot = ref(null);
const queryEditorView = shallowRef(null);
const queryEditorReady = ref(false);
const searchInputRef = ref(null);
const tableScrollTop = ref(0);
const tableScrollLeft = ref(0);
const tableViewportHeight = ref(420);
const tableSearchOpen = ref(false);
const tableSearchQuery = ref("");
const querySelection = ref({ start: 0, end: 0 });
const tableDesignStates = ref({});
let tableResizeObserver = null;
let queryEditorModules = null;
let queryEditorModulesPromise = null;
let activeViewStateTabId = null;
let restoringViewState = false;
let applyingQueryEditorContent = false;
let tableOperationToken = 0;
let newRowSequence = 0;

watch(
  () => normalizedConnection.value,
  (connection) => {
    form.name = connection.name;
    Object.assign(form, connection.config);
  },
  { immediate: true },
);

watch(
  () => props.pendingTableQuery,
  (payload) => {
    if (!payload || payload.groupType !== "table") {
      return;
    }

    form.database = payload.schema;
    loadTableData(payload.schema, payload.table, props.activeTopTab?.id);
    persistConfig();
  },
);

watch(
  () => props.activeTopTab?.id,
  async (nextTabId, previousTabId) => {
    saveTabViewState(previousTabId);
    stopSchemaRowSelection();
    stopSchemaCellSelection();
    stopResultRowSelection();
    stopResultCellSelection();
    stopColumnResize();
    restoreTabViewState(nextTabId);

    if (props.activeTopTab?.kind === "schema") {
      loadSchemaTableMetadata(props.activeTopTab.schema);
      loadSchemaTableCounts(props.activeTopTab.schema);
    }
    ensureTableDesignState(props.activeTopTab);

    await nextTick();
    if (props.activeTopTab?.kind === "query") {
      await createQueryEditor();
      syncQueryEditorContent(activeQueryText.value);
      loadQueryColumnCompletions(props.activeTopTab.schema);
    } else {
      destroyQueryEditor();
    }
    applyStoredScrollState();
  },
  { immediate: true },
);

watch(
  () => props.activeTopTab?.kind === "query" ? props.activeTopTab.schema : null,
  async (schema) => {
    if (!schema || props.activeTopTab?.kind !== "query") {
      return;
    }

    await nextTick();
    loadQueryColumnCompletions(schema);
    queryEditorView.value?.dispatch({ effects: [] });
  },
);

watch(
  () => tableSearchQuery.value,
  (nextQuery, previousQuery) => {
    if (restoringViewState || nextQuery === previousQuery) {
      return;
    }

    clearSchemaRowSelection();
    clearSchemaCellSelection();
    clearResultRowSelection();
    clearResultCellSelection();
    resetTableScroll();
  },
);

const activeSchemaTables = computed(() => {
  const schema = props.activeTopTab?.kind === "schema" ? props.activeTopTab.schema : null;
  const tableGroup = schema?.groups?.find((group) => (group.groupType ?? group.type) === "table");
  return tableGroup?.items?.map((item) => {
    const name = typeof item === "string" ? item : item.name;
    const metadata = schemaTableMetadata.value[schemaTableCountKey(schema.name, name)] ?? {};

    return {
      name,
      rowCount: schemaTableCounts.value[schemaTableCountKey(schema.name, name)]
        ?? metadata.rowCount
        ?? (typeof item === "string" ? 0 : item.rowCount),
      dataLength: metadata.dataLength ?? (typeof item === "string" ? 0 : item.dataLength),
      engine: metadata.engine ?? (typeof item === "string" ? "" : item.engine),
      createTime: metadata.createTime ?? (typeof item === "string" ? "" : item.createTime),
      updateTime: metadata.updateTime ?? (typeof item === "string" ? "" : item.updateTime),
      collation: metadata.collation ?? (typeof item === "string" ? "" : item.collation),
      comment: metadata.comment ?? (typeof item === "string" ? "" : item.comment),
      schema: schema.name,
    };
  }) ?? [];
});
const normalizedTableSearch = computed(() => tableSearchQuery.value.trim().toLowerCase());
const hasTableSearch = computed(() => Boolean(normalizedTableSearch.value));
const searchedSchemaTables = computed(() => {
  if (!hasTableSearch.value) {
    return activeSchemaTables.value;
  }

  return activeSchemaTables.value.filter((row) =>
    rowMatchesSearch(row, schemaTableColumns.value, normalizedTableSearch.value, schemaTableCellValue),
  );
});

const activeResult = computed(() => {
  const key = props.activeTopTab?.id;
  return key ? tableResults.value[key] : null;
});
const shouldShowResultPanel = computed(() => (
  props.activeTopTab?.kind === "table" || Boolean(activeResult.value)
));
const activeEditState = computed(() => {
  const tabId = props.activeTopTab?.id;
  return tabId ? tableEditStates.value[tabId] ?? null : null;
});
const hasPendingTableChanges = computed(() => {
  const state = activeEditState.value;
  return Boolean(state && (state.newRows.length > 0 || state.updatedRows.size > 0));
});
const canEditActiveResult = computed(() => props.activeTopTab?.kind === "table" && Boolean(activeResult.value?.table));
const canStopActiveOperation = computed(() => loading.value);

const tableColumns = computed(() => {
  const tabId = props.activeTopTab?.id;
  const widths = tabId ? tableColumnWidths.value[tabId] ?? {} : {};
  const columns = (activeResult.value?.columns ?? []).filter((column) => column !== "__myhubRowId");
  const baseColumns = columns.map((column) => ({
    key: column,
    label: column,
    width: columnMinWidth(column),
  }));

  return baseColumns.map((column) => ({
    ...column,
    width: widths[column.key] ?? column.width,
  }));
});

const schemaTableColumns = computed(() => {
  const tabId = props.activeTopTab?.id;
  const widths = tabId ? tableColumnWidths.value[tabId] ?? {} : {};
  const columns = [
    { key: "name", label: "名称", width: 240 },
    { key: "rowCount", label: "行", width: 110, align: "right" },
    { key: "dataLength", label: "数据长度", width: 140, align: "right", formatter: formatDataLength },
    { key: "engine", label: "引擎", width: 150 },
    { key: "createTime", label: "创建日期", width: 200, formatter: formatTableDate },
    { key: "updateTime", label: "修改日期", width: 200, formatter: formatTableDate },
    { key: "collation", label: "排序规则", width: 190 },
    { key: "comment", label: "注释", width: 220 },
  ];

  return columns.map((column) => ({
    ...column,
    width: widths[column.key] ?? column.width,
  }));
});

const schemaTableGridTemplate = computed(() => (
  schemaTableColumns.value.map((column) => `${column.width}px`).join(" ")
));

const schemaTableContentWidth = computed(() => (
  schemaTableColumns.value.reduce((total, column) => total + column.width, 0)
));

const tableGridTemplate = computed(() => (
  tableColumns.value.map((column) => `${column.width}px`).join(" ")
));

const tableContentWidth = computed(() => (
  tableColumns.value.reduce((total, column) => total + column.width, 0)
));

const visibleTableRows = computed(() => {
  const rows = searchedResultRows.value;
  const visibleCount = Math.ceil(tableViewportHeight.value / TABLE_ROW_HEIGHT) + TABLE_OVERSCAN * 2;
  const start = Math.max(0, Math.floor(tableScrollTop.value / TABLE_ROW_HEIGHT) - TABLE_OVERSCAN);
  const end = Math.min(rows.length, start + visibleCount);

  return {
    rows: rows.slice(start, end),
    start,
    end,
    top: start * TABLE_ROW_HEIGHT,
    bottom: Math.max(0, (rows.length - end) * TABLE_ROW_HEIGHT),
  };
});
const searchedResultRows = computed(() => {
  const rows = activeResult.value?.rows ?? [];
  if (!hasTableSearch.value) {
    return rows;
  }

  return rows.filter((row) =>
    rowMatchesSearch(row, tableColumns.value, normalizedTableSearch.value, tableCellValue),
  );
});
const activeQueryText = computed({
  get() {
    const tab = props.activeTopTab;
    if (!tab || tab.kind !== "query") {
      return "";
    }

    return querySqlByTab.value[tab.id] ?? tab.sql ?? defaultQuerySql(tab.schema);
  },
  set(value) {
    const tab = props.activeTopTab;
    if (!tab || tab.kind !== "query") {
      return;
    }

    querySqlByTab.value = {
      ...querySqlByTab.value,
      [tab.id]: value,
    };
  },
});
const hasSelectedQueryText = computed(() => {
  querySelection.value;
  return Boolean(selectedQueryText().trim());
});
const queryRunLabel = computed(() => (hasSelectedQueryText.value ? "执行选中" : "执行"));
const querySchemaOptions = computed(() => normalizedConnection.value.schemas?.map((schema) => schema.name) ?? []);
const activeQuerySchemaName = computed({
  get() {
    return props.activeTopTab?.kind === "query" ? props.activeTopTab.schema : "";
  },
  set(schema) {
    changeQuerySchema(schema);
  },
});
const activeDesignState = computed(() => {
  const tabId = props.activeTopTab?.id;
  return tabId ? tableDesignStates.value[tabId] : null;
});
const activeDesignSqlPreview = computed(() => {
  const tab = props.activeTopTab;
  const state = activeDesignState.value;
  if (!tab || tab.kind !== "table-design" || !state) {
    return { sql: "", error: "" };
  }

  try {
    const sql = buildTableDesignSql(tab, state);
    return { sql: Array.isArray(sql) ? sql.join("\n\n") : sql, error: "" };
  } catch (error) {
    return { sql: "", error: error.message ?? String(error) };
  }
});
const copyContextItems = computed(() => {
  const hasSelection = Boolean(selectedCopyText());
  const isResult = ["table", "query"].includes(props.activeTopTab?.kind);
  const isTable = props.activeTopTab?.kind === "table";

  if (!isResult) {
    return [{ key: "copy", label: "复制", disabled: !hasSelection }];
  }

  return [
    { key: "delete-records", label: "删除记录", danger: true, disabled: !isTable || selectedResultRows().length === 0 },
    { key: "copy", label: "复制", divided: true, disabled: !hasSelection },
    { key: "copy-fields", label: "复制字段名称", disabled: selectedResultColumns().length === 0 },
    {
      key: "copy-as",
      label: "复制为",
      disabled: !hasSelection,
      children: [
        { key: "copy-insert", label: "Insert 语句", disabled: !isTable || selectedResultRows().length === 0 },
        { key: "copy-update", label: "Update 语句", disabled: !isTable || selectedResultRows().length === 0 },
        { key: "copy-tsv-data", label: "制表符分隔值（数据）", divided: true, disabled: !hasSelection },
        { key: "copy-tsv-fields", label: "制表符分隔值（字段名称）", disabled: selectedResultColumns().length === 0 },
        { key: "copy-tsv-fields-data", label: "制表符分隔值（字段名和数据）", disabled: !hasSelection },
      ],
    },
    { key: "paste", label: "粘贴", disabled: !canPasteIntoResultSelection() },
  ];
});

function currentConfig() {
  return {
    host: form.host,
    port: Number(form.port),
    username: form.username,
    password: form.password,
    database: form.database,
  };
}

function persistConfig(extra = {}) {
  const config = currentConfig();
  emit("update-connection", {
    name: form.name || "未命名连接",
    meta: formatMysqlMeta(config),
    config,
    ...extra,
  });
}

function activeQuerySchema() {
  const schemaName = props.activeTopTab?.kind === "query" ? props.activeTopTab.schema : null;
  return normalizedConnection.value.schemas?.find((schema) => schema.name === schemaName) ?? null;
}

function changeQuerySchema(schema) {
  if (!schema || props.activeTopTab?.kind !== "query" || schema === props.activeTopTab.schema) {
    return;
  }

  const tabId = props.activeTopTab.id;
  emit("update-query-schema", { tabId, schema });
  form.database = schema;
  tableResults.value = {
    ...tableResults.value,
    [tabId]: {
      columns: [],
      rows: [],
      affectedRows: 0,
      elapsedMs: 0,
      schema,
      table: null,
      page: 1,
      pageSize: 0,
      totalRows: 0,
    },
  };
  loadQueryColumnCompletions(schema);
  persistConfig();
}

function currentSchemaObjectCompletions() {
  const schema = activeQuerySchema();
  if (!schema) {
    return [];
  }

  return (schema.groups ?? [])
    .filter((group) => ["table", "view"].includes(group.groupType ?? group.type))
    .flatMap((group) => {
      const type = (group.groupType ?? group.type) === "view" ? "视图" : "表";
      return (group.items ?? []).map((item) => {
        const label = typeof item === "string" ? item : item.name;
        return {
          label,
          type: "variable",
          detail: type,
          apply: label,
        };
      });
    });
}

function queryColumnCompletionKey(schemaName) {
  return `${normalizedConnection.value.id}:${schemaName}`;
}

function currentSchemaColumnCompletions() {
  const schemaName = props.activeTopTab?.kind === "query" ? props.activeTopTab.schema : null;
  return schemaName ? queryColumnCompletionsBySchema.value[queryColumnCompletionKey(schemaName)] ?? [] : [];
}

function queryCompletionSource(context) {
  const word = context.matchBefore(/[\w.`\u4e00-\u9fa5]+/);
  if (!word || (word.from === word.to && !context.explicit)) {
    return null;
  }

  const options = [
    ...SQL_COMPLETION_KEYWORDS.map((keyword) => ({
      label: keyword,
      type: "keyword",
      detail: "SQL",
      apply: keyword,
    })),
    ...currentSchemaObjectCompletions(),
    ...currentSchemaColumnCompletions(),
  ];

  return {
    from: word.from,
    options,
    validFor: /^[\w.`\u4e00-\u9fa5]*$/,
  };
}

async function loadQueryColumnCompletions(schemaName) {
  if (!schemaName) {
    return;
  }

  const cacheKey = queryColumnCompletionKey(schemaName);
  if (queryColumnCompletionsBySchema.value[cacheKey]) {
    return;
  }

  const schema = activeQuerySchema();
  const objects = (schema?.groups ?? [])
    .filter((group) => ["table", "view"].includes(group.groupType ?? group.type))
    .flatMap((group) => (group.items ?? []).map((item) => (typeof item === "string" ? item : item.name)))
    .filter(Boolean);

  if (objects.length === 0) {
    queryColumnCompletionsBySchema.value = {
      ...queryColumnCompletionsBySchema.value,
      [cacheKey]: [],
    };
    return;
  }

  const completions = [];
  const unqualifiedColumns = new Map();
  const workers = Array.from({ length: Math.min(4, objects.length) }, async (_, workerIndex) => {
    for (let index = workerIndex; index < objects.length; index += 4) {
      const table = objects[index];
      try {
        const detail = await describeMysqlTable(currentConfig(), schemaName, table);
        for (const column of detail.columns ?? []) {
          const existing = unqualifiedColumns.get(column.name);
          unqualifiedColumns.set(column.name, {
            label: column.name,
            type: "property",
            detail: existing ? "多表字段" : `${table} · ${column.columnType ?? "字段"}`,
            apply: column.name,
          });
          completions.push({
            label: `${table}.${column.name}`,
            type: "property",
            detail: column.columnType ?? "字段",
            apply: `${table}.${column.name}`,
          });
        }
      } catch {
        // 单表字段加载失败不影响其他提示。
      }
    }
  });

  await Promise.all(workers);
  queryColumnCompletionsBySchema.value = {
    ...queryColumnCompletionsBySchema.value,
    [cacheKey]: [...unqualifiedColumns.values(), ...completions],
  };
  if (props.activeTopTab?.kind === "query" && props.activeTopTab.schema === schemaName) {
    queryEditorView.value?.dispatch({ effects: [] });
  }
}

async function ensureQueryEditorModules() {
  if (queryEditorModules) {
    return queryEditorModules;
  }

  queryEditorModulesPromise ??= Promise.all([
    import("codemirror"),
    import("@codemirror/state"),
    import("@codemirror/view"),
    import("@codemirror/lang-sql"),
    import("@codemirror/autocomplete"),
  ]).then(([codemirror, stateModule, viewModule, sqlModule, autocompleteModule]) => ({
    basicSetup: codemirror.basicSetup,
    StateField: stateModule.StateField,
    EditorView: codemirror.EditorView,
    Decoration: viewModule.Decoration,
    sql: sqlModule.sql,
    autocompletion: autocompleteModule.autocompletion,
  }));
  queryEditorModules = await queryEditorModulesPromise;
  return queryEditorModules;
}

async function createQueryEditor() {
  if (!queryEditorRoot.value || queryEditorView.value) {
    return;
  }

  const { basicSetup, EditorView, StateField, Decoration, sql, autocompletion } = await ensureQueryEditorModules();
  if (!queryEditorRoot.value || queryEditorView.value) {
    return;
  }

  const selectedTextDecoration = Decoration.mark({ class: "query-selected-text" });
  const selectedTextField = StateField.define({
    create(state) {
      return selectedTextDecorations(state);
    },
    update(decorations, transaction) {
      if (!transaction.docChanged && !transaction.selection) {
        return decorations.map(transaction.changes);
      }
      return selectedTextDecorations(transaction.state);
    },
    provide(field) {
      return EditorView.decorations.from(field);
    },
  });

  function selectedTextDecorations(state) {
    const ranges = [];
    for (const range of state.selection.ranges) {
      if (!range.empty) {
        ranges.push(selectedTextDecoration.range(range.from, range.to));
      }
    }
    return Decoration.set(ranges, true);
  }

  queryEditorView.value = new EditorView({
    doc: activeQueryText.value,
    parent: queryEditorRoot.value,
    extensions: [
      basicSetup,
      sql({ upperCaseKeywords: true }),
      autocompletion({
        override: [queryCompletionSource],
      }),
      selectedTextField,
      EditorView.lineWrapping,
      EditorView.updateListener.of((update) => {
        if (update.docChanged && !applyingQueryEditorContent) {
          activeQueryText.value = update.state.doc.toString();
        }
        if (update.docChanged || update.selectionSet) {
          updateQuerySelection();
        }
      }),
      EditorView.theme({
        "&": {
          height: "100%",
          backgroundColor: "#fff",
          color: "#24262d",
          fontSize: "13px",
        },
        ".cm-scroller": {
          fontFamily: '"SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace',
          lineHeight: "1.6",
        },
        ".cm-content": {
          padding: "8px 0",
        },
        ".cm-line": {
          padding: "0 12px",
        },
        ".cm-gutters": {
          backgroundColor: "#f7f7f8",
          borderRight: "1px solid #dedfe3",
          color: "#9aa0aa",
        },
        ".cm-activeLine": {
          backgroundColor: "#fff7f4",
        },
        ".cm-activeLineGutter": {
          backgroundColor: "#fff0eb",
          color: "#686d78",
        },
        ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": {
          backgroundColor: "#ffc7b3",
        },
        ".cm-selectionMatch": {
          backgroundColor: "#fff0eb",
          outline: "1px solid #f5c5b3",
        },
        "&.cm-focused .cm-selectionLayer .cm-selectionBackground": {
          boxShadow: "inset 0 -1px 0 #f26b3a",
        },
        "&.cm-focused": {
          outline: "none",
        },
      }),
    ],
  });
  queryEditorReady.value = true;
  updateQuerySelection();
}

function syncQueryEditorContent(content) {
  if (!queryEditorView.value || queryEditorView.value.state.doc.toString() === content) {
    return;
  }

  applyingQueryEditorContent = true;
  queryEditorView.value.dispatch({
    changes: { from: 0, to: queryEditorView.value.state.doc.length, insert: content },
  });
  applyingQueryEditorContent = false;
}

function destroyQueryEditor() {
  queryEditorView.value?.destroy();
  queryEditorView.value = null;
  queryEditorReady.value = false;
  querySelection.value = { start: 0, end: 0 };
}

function selectedQueryText() {
  const view = queryEditorView.value;
  if (!view || props.activeTopTab?.kind !== "query") {
    return "";
  }

  return view.state.selection.ranges
    .filter((range) => !range.empty)
    .map((range) => view.state.doc.sliceString(range.from, range.to))
    .join("\n");
}

function updateQuerySelection() {
  const ranges = queryEditorView.value?.state.selection.ranges ?? [];
  const firstRange = ranges[0];
  querySelection.value = {
    start: firstRange?.from ?? 0,
    end: firstRange?.to ?? 0,
  };
}

function schemaTableCountKey(schema, table) {
  return `${normalizedConnection.value.id}:${schema}.${table}`;
}

function defaultTabViewState() {
  return {
    searchOpen: false,
    searchQuery: "",
    schemaTableKey: "",
    schemaRowRange: emptyRowRange(),
    schemaCellRange: emptyCellRange(),
    resultRowRange: emptyRowRange(),
    resultCellRange: emptyCellRange(),
    schemaScrollTop: 0,
    schemaScrollLeft: 0,
    tableScrollTop: 0,
    tableScrollLeft: 0,
  };
}

function cloneRange(range, fallback) {
  return { ...fallback(), ...(range ?? {}) };
}

function currentTabViewState() {
  return {
    searchOpen: tableSearchOpen.value,
    searchQuery: tableSearchQuery.value,
    schemaTableKey: selectedSchemaTableKey.value,
    schemaRowRange: cloneRange(selectedSchemaRowRange.value, emptyRowRange),
    schemaCellRange: cloneRange(selectedSchemaCellRange.value, emptyCellRange),
    resultRowRange: cloneRange(selectedResultRowRange.value, emptyRowRange),
    resultCellRange: cloneRange(selectedResultCellRange.value, emptyCellRange),
    schemaScrollTop: schemaTableScrollTop.value,
    schemaScrollLeft: schemaTableScrollLeft.value,
    tableScrollTop: tableScrollTop.value,
    tableScrollLeft: tableScrollLeft.value,
  };
}

function saveTabViewState(tabId = activeViewStateTabId) {
  if (!tabId) {
    return;
  }

  tabViewStates.value = {
    ...tabViewStates.value,
    [tabId]: currentTabViewState(),
  };
}

function restoreTabViewState(tabId) {
  activeViewStateTabId = tabId ?? null;
  const state = tabId ? tabViewStates.value[tabId] ?? defaultTabViewState() : defaultTabViewState();

  restoringViewState = true;
  tableSearchOpen.value = state.searchOpen;
  tableSearchQuery.value = state.searchQuery;
  selectedSchemaTableKey.value = state.schemaTableKey;
  selectedSchemaRowRange.value = cloneRange(state.schemaRowRange, emptyRowRange);
  selectedSchemaCellRange.value = cloneRange(state.schemaCellRange, emptyCellRange);
  selectedResultRowRange.value = cloneRange(state.resultRowRange, emptyRowRange);
  selectedResultCellRange.value = cloneRange(state.resultCellRange, emptyCellRange);
  schemaTableScrollTop.value = state.schemaScrollTop;
  schemaTableScrollLeft.value = state.schemaScrollLeft;
  tableScrollTop.value = state.tableScrollTop;
  tableScrollLeft.value = state.tableScrollLeft;
  nextTick(() => {
    restoringViewState = false;
  });
}

function applyStoredScrollState() {
  if (schemaTableViewport.value) {
    schemaTableViewport.value.scrollTop = schemaTableScrollTop.value;
    schemaTableViewport.value.scrollLeft = schemaTableScrollLeft.value;
  }

  if (tableViewport.value) {
    tableViewport.value.scrollTop = tableScrollTop.value;
    tableViewport.value.scrollLeft = tableScrollLeft.value;
  }
  updateTableViewportHeight();
}

async function openTableSearch() {
  if (
    !props.activeTopTab ||
    !["schema", "table", "query"].includes(props.activeTopTab.kind) ||
    (props.activeTopTab.kind === "query" && !shouldShowResultPanel.value)
  ) {
    return;
  }

  tableSearchOpen.value = true;
  await nextTick();
  searchInputRef.value?.focus();
  searchInputRef.value?.select();
}

function closeTableSearch() {
  tableSearchOpen.value = false;
  tableSearchQuery.value = "";
}

function handleSearchKeydown(event) {
  if (event.key === "Escape") {
    closeTableSearch();
  }
}

function clearSchemaRowSelection() {
  selectedSchemaRowRange.value = { start: null, end: null };
  isSelectingSchemaRows.value = false;
}

function clearSchemaCellSelection() {
  selectedSchemaCellRange.value = { startRow: null, endRow: null, startColumn: null, endColumn: null };
  isSelectingSchemaCells.value = false;
}

function selectSchemaRowRange(start, end = start) {
  selectedSchemaRowRange.value = { start, end };
}

function selectSchemaCellRange(startRow, startColumn, endRow = startRow, endColumn = startColumn) {
  selectedSchemaCellRange.value = { startRow, endRow, startColumn, endColumn };
}

function isSchemaRowSelected(rowIndex) {
  const { start, end } = selectedSchemaRowRange.value;
  if (start === null || end === null) {
    return false;
  }

  return rowIndex >= Math.min(start, end) && rowIndex <= Math.max(start, end);
}

function isSchemaCellSelected(rowIndex, columnIndex) {
  const { startRow, endRow, startColumn, endColumn } = selectedSchemaCellRange.value;
  if (startRow === null || endRow === null || startColumn === null || endColumn === null) {
    return false;
  }

  return rowIndex >= Math.min(startRow, endRow)
    && rowIndex <= Math.max(startRow, endRow)
    && columnIndex >= Math.min(startColumn, endColumn)
    && columnIndex <= Math.max(startColumn, endColumn);
}

function startSchemaRowSelection(event, rowIndex) {
  if (event.button !== 0) {
    return;
  }

  clearSchemaCellSelection();
  isSelectingSchemaRows.value = true;
  selectSchemaRowRange(rowIndex);
}

function extendSchemaRowSelection(rowIndex) {
  if (!isSelectingSchemaRows.value || selectedSchemaRowRange.value.start === null) {
    return;
  }

  selectSchemaRowRange(selectedSchemaRowRange.value.start, rowIndex);
}

function stopSchemaRowSelection() {
  isSelectingSchemaRows.value = false;
}

function startSchemaCellSelection(event, rowIndex, columnIndex) {
  if (event.button !== 0) {
    return;
  }

  clearSchemaRowSelection();
  isSelectingSchemaCells.value = true;
  selectSchemaCellRange(rowIndex, columnIndex);
}

function extendSchemaCellSelection(rowIndex, columnIndex) {
  if (!isSelectingSchemaCells.value) {
    return;
  }

  const { startRow, startColumn } = selectedSchemaCellRange.value;
  if (startRow === null || startColumn === null) {
    return;
  }

  selectSchemaCellRange(startRow, startColumn, rowIndex, columnIndex);
}

function stopSchemaCellSelection() {
  isSelectingSchemaCells.value = false;
}

function openCopyContextMenu(event, rowIndex = null, columnIndex = null) {
  if (props.activeTopTab?.kind !== "schema" && rowIndex !== null) {
    const hasCellSelection = selectedResultCellRange.value.startRow !== null;
    const hasRowSelection = selectedResultRowRange.value.start !== null;
    const isInsideSelection = columnIndex === null
      ? isAbsoluteResultRowSelected(rowIndex)
      : isAbsoluteResultCellSelected(rowIndex, columnIndex);

    if (!hasCellSelection && !hasRowSelection) {
      if (columnIndex === null) {
        clearResultCellSelection();
        selectResultRowRange(rowIndex);
      } else {
        clearResultRowSelection();
        selectResultCellRange(rowIndex, columnIndex);
      }
    } else if (!isInsideSelection) {
      if (columnIndex === null) {
        clearResultCellSelection();
        selectResultRowRange(rowIndex);
      } else {
        clearResultRowSelection();
        selectResultCellRange(rowIndex, columnIndex);
      }
    }
  }

  if (copyContextItems.value.every((item) => item.disabled || item.children?.every((child) => child.disabled))) {
    return;
  }

  copyContextPosition.value = { x: event.clientX, y: event.clientY };
  copyContextOpen.value = true;
}

function handleCopyContextSelect(item) {
  if (item.key === "copy") {
    copySelectedText();
  } else if (item.key === "copy-fields") {
    copyText(selectedResultColumnNames().join("\t"), "已复制字段名称");
  } else if (item.key === "copy-insert") {
    copyGeneratedSql("insert");
  } else if (item.key === "copy-update") {
    copyGeneratedSql("update");
  } else if (item.key === "copy-tsv-data") {
    copySelectedText();
  } else if (item.key === "copy-tsv-fields") {
    copyText(selectedResultColumnNames().join("\t"), "已复制字段名称");
  } else if (item.key === "copy-tsv-fields-data") {
    copyText([selectedResultColumnNames().join("\t"), selectedResultCopyText()].filter(Boolean).join("\n"), "已复制");
  } else if (item.key === "paste") {
    pasteIntoSelectedCells();
  } else if (item.key === "delete-records") {
    deleteSelectedRecords();
  }
}

function handleKeydown(event) {
  const isFindShortcut = (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f";
  if (isFindShortcut && props.activeTopTab && ["schema", "table", "query"].includes(props.activeTopTab.kind)) {
    event.preventDefault();
    openTableSearch();
    return;
  }

  const isRunShortcut = (event.ctrlKey || event.metaKey) && event.key === "Enter";
  if (isRunShortcut && props.activeTopTab?.kind === "query") {
    event.preventDefault();
    executeActiveQuery();
    return;
  }

  const target = event.target;
  const isEditingText = target instanceof HTMLElement && Boolean(target.closest("input, textarea, [contenteditable='true']"));
  const isCopyShortcut = (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "c";
  const isPasteShortcut = (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "v";
  if (!isCopyShortcut && !isPasteShortcut) {
    return;
  }

  if (isEditingText) {
    return;
  }

  const text = selectedCopyText();
  if (!text) {
    return;
  }

  event.preventDefault();
  copySelectedText();
}

function selectedSchemaCopyText() {
  const { startRow, endRow, startColumn, endColumn } = selectedSchemaCellRange.value;
  if (startRow !== null && endRow !== null && startColumn !== null && endColumn !== null) {
    return copyTextForCells(searchedSchemaTables.value, schemaTableColumns.value, selectedSchemaCellRange.value);
  }

  const { start, end } = selectedSchemaRowRange.value;
  if (start !== null && end !== null) {
    const rowRange = clampRangeIndexes(start, end, searchedSchemaTables.value.length);
    return copyTextForRows(
      searchedSchemaTables.value.slice(rowRange.from, rowRange.to + 1),
      schemaTableColumns.value,
    );
  }

  return "";
}

function selectedResultCopyText() {
  const rows = searchedResultRows.value;
  const { startRow, endRow, startColumn, endColumn } = selectedResultCellRange.value;
  if (startRow !== null && endRow !== null && startColumn !== null && endColumn !== null) {
    return copyTextForCells(rows, tableColumns.value, selectedResultCellRange.value);
  }

  const { start, end } = selectedResultRowRange.value;
  if (start !== null && end !== null) {
    const rowRange = clampRangeIndexes(start, end, rows.length);
    return copyTextForRows(rows.slice(rowRange.from, rowRange.to + 1), tableColumns.value);
  }

  return "";
}

function selectedCopyText() {
  if (props.activeTopTab?.kind === "schema") {
    return selectedSchemaCopyText();
  }

  if (props.activeTopTab?.kind === "table") {
    return selectedResultCopyText();
  }

  if (props.activeTopTab?.kind === "query") {
    return selectedResultCopyText();
  }

  return "";
}

async function copySelectedText() {
  const text = selectedCopyText();
  if (!text) {
    return;
  }

  copyText(text, "已复制");
}

async function copyText(text, successMessage = "已复制") {
  if (!text) {
    return;
  }

  try {
    await navigator.clipboard.writeText(text);
    ElMessage.success(successMessage);
  } catch (error) {
    ElMessage.error(`复制失败：${error}`);
  }
}

function isAbsoluteResultRowSelected(rowIndex) {
  const { start, end } = selectedResultRowRange.value;
  if (start === null || end === null) {
    return false;
  }

  return rowIndex >= Math.min(start, end) && rowIndex <= Math.max(start, end);
}

function isAbsoluteResultCellSelected(rowIndex, columnIndex) {
  const { startRow, endRow, startColumn, endColumn } = selectedResultCellRange.value;
  if (startRow === null || endRow === null || startColumn === null || endColumn === null) {
    return false;
  }

  return rowIndex >= Math.min(startRow, endRow)
    && rowIndex <= Math.max(startRow, endRow)
    && columnIndex >= Math.min(startColumn, endColumn)
    && columnIndex <= Math.max(startColumn, endColumn);
}

function selectedResultColumnIndexes() {
  const { startColumn, endColumn } = selectedResultCellRange.value;
  if (startColumn !== null && endColumn !== null) {
    const columnRange = clampRangeIndexes(startColumn, endColumn, tableColumns.value.length);
    return Array.from({ length: columnRange.to - columnRange.from + 1 }, (_, index) => columnRange.from + index);
  }

  return tableColumns.value.map((_, index) => index);
}

function selectedResultColumns() {
  return selectedResultColumnIndexes().map((index) => tableColumns.value[index]).filter(Boolean);
}

function selectedResultColumnNames() {
  return selectedResultColumns().map((column) => column.key);
}

function selectedResultRows() {
  const rows = searchedResultRows.value;
  const { startRow, endRow } = selectedResultCellRange.value;
  if (startRow !== null && endRow !== null) {
    const rowRange = clampRangeIndexes(startRow, endRow, rows.length);
    return rows.slice(rowRange.from, rowRange.to + 1);
  }

  const { start, end } = selectedResultRowRange.value;
  if (start !== null && end !== null) {
    const rowRange = clampRangeIndexes(start, end, rows.length);
    return rows.slice(rowRange.from, rowRange.to + 1);
  }

  return [];
}

function rowInternalId(row) {
  return row?.__myhubRowId ?? "";
}

function editableRows(rows) {
  return (rows ?? []).map((row) => ({
    ...row,
    __myhubRowId: rowInternalId(row) || `row-${newRowSequence += 1}`,
  }));
}

function publicRow(row) {
  const next = { ...(row ?? {}) };
  delete next.__myhubRowId;
  return next;
}

function publicRows(rows) {
  return (rows ?? []).map(publicRow);
}

function createEditState(rows = []) {
  return {
    originalRows: new Map(rows.map((row) => [rowInternalId(row), publicRow(row)])),
    newRows: [],
    updatedRows: new Map(),
  };
}

function setEditState(tabId, state) {
  tableEditStates.value = {
    ...tableEditStates.value,
    [tabId]: state,
  };
}

function clearEditState(tabId = props.activeTopTab?.id) {
  if (!tabId) {
    return;
  }

  const nextStates = { ...tableEditStates.value };
  delete nextStates[tabId];
  tableEditStates.value = nextStates;
}

function ensureActiveEditState() {
  const tabId = props.activeTopTab?.id;
  if (!tabId || !activeResult.value) {
    return null;
  }

  const existing = tableEditStates.value[tabId];
  if (existing) {
    return existing;
  }

  const state = createEditState(activeResult.value.rows ?? []);
  setEditState(tabId, state);
  return state;
}

function updateActiveRows(rows) {
  const tabId = props.activeTopTab?.id;
  const result = activeResult.value;
  if (!tabId || !result) {
    return;
  }

  tableResults.value = {
    ...tableResults.value,
    [tabId]: {
      ...result,
      rows,
      totalRows: props.activeTopTab?.kind === "table" ? Math.max(result.totalRows ?? 0, rows.length) : rows.length,
    },
  };
}

function markRowChanged(row) {
  if (!canEditActiveResult.value) {
    return;
  }

  const state = ensureActiveEditState();
  if (!state) {
    return;
  }

  const rowId = rowInternalId(row);
  if (state.newRows.includes(rowId)) {
    return;
  }

  const original = state.originalRows.get(rowId);
  const current = publicRow(row);
  if (JSON.stringify(original ?? {}) === JSON.stringify(current)) {
    state.updatedRows.delete(rowId);
  } else {
    state.updatedRows.set(rowId, current);
  }
}

function isNewResultRow(row) {
  return Boolean(activeEditState.value?.newRows.includes(rowInternalId(row)));
}

function isChangedResultRow(row) {
  return isNewResultRow(row) || Boolean(activeEditState.value?.updatedRows.has(rowInternalId(row)));
}

function isChangedResultCell(row, column) {
  const state = activeEditState.value;
  const rowId = rowInternalId(row);
  if (!state || state.newRows.includes(rowId)) {
    return false;
  }

  const original = state.originalRows.get(rowId);
  return state.updatedRows.has(rowId) && original?.[column.key] !== row[column.key];
}

function setResultCellValue(row, column, value) {
  if (!canEditActiveResult.value) {
    return;
  }

  const resultRows = activeResult.value?.rows ?? [];
  const rowId = rowInternalId(row);
  const rowIndex = resultRows.findIndex((item) => rowInternalId(item) === rowId);
  if (rowIndex < 0) {
    return;
  }

  const nextRow = { ...resultRows[rowIndex], [column.key]: value };
  const nextRows = [...resultRows];
  nextRows[rowIndex] = nextRow;
  updateActiveRows(nextRows);
  markRowChanged(nextRow);
}

function mysqlValueLiteral(value) {
  if (value === null || value === undefined) {
    return "NULL";
  }
  if (value instanceof Date) {
    return quoteString(value.toISOString());
  }
  if (typeof value === "number") {
    return Number.isFinite(value) ? String(value) : "NULL";
  }
  if (typeof value === "bigint") {
    return String(value);
  }
  if (typeof value === "boolean") {
    return value ? "1" : "0";
  }
  if (typeof value === "object") {
    return quoteString(JSON.stringify(value));
  }
  return quoteString(value);
}

function activeTableNameSql() {
  const result = activeResult.value;
  if (!result?.schema || !result?.table) {
    return "";
  }
  return `${quoteIdentifier(result.schema)}.${quoteIdentifier(result.table)}`;
}

function tableDetailCacheKey(schema, table) {
  return `${normalizedConnection.value.id}:${schema}.${table}`;
}

async function loadActiveTableDetail() {
  const result = activeResult.value;
  if (!result?.schema || !result?.table) {
    return null;
  }

  const cacheKey = tableDetailCacheKey(result.schema, result.table);
  if (tableDetailCache.value[cacheKey]) {
    return tableDetailCache.value[cacheKey];
  }

  const detail = await describeMysqlTable(currentConfig(), result.schema, result.table);
  tableDetailCache.value = {
    ...tableDetailCache.value,
    [cacheKey]: detail,
  };
  return detail;
}

function primaryKeyColumns(detail) {
  return (detail?.indexes ?? [])
    .filter((index) => index.name === "PRIMARY")
    .sort((left, right) => Number(left.seqInIndex ?? 0) - Number(right.seqInIndex ?? 0))
    .map((index) => index.columnName)
    .filter(Boolean);
}

function rowWhereClause(row, columns) {
  return columns
    .map((column) => `${quoteIdentifier(column)} <=> ${mysqlValueLiteral(row[column])}`)
    .join(" AND ");
}

async function copyGeneratedSql(kind) {
  try {
    const rows = selectedResultRows();
    const columns = selectedResultColumnNames();
    const tableName = activeTableNameSql();
    if (!tableName || rows.length === 0 || columns.length === 0) {
      return;
    }

    const detail = await loadActiveTableDetail();
    const whereColumns = primaryKeyColumns(detail);
    const sql = rows.map((row) => {
      if (kind === "insert") {
        const columnSql = columns.map(quoteIdentifier).join(", ");
        const valueSql = columns.map((column) => mysqlValueLiteral(row[column])).join(", ");
        return `INSERT INTO ${tableName} (${columnSql}) VALUES (${valueSql});`;
      }

      const setSql = columns.map((column) => `${quoteIdentifier(column)} = ${mysqlValueLiteral(row[column])}`).join(", ");
      const whereSql = whereColumns.length > 0 ? rowWhereClause(row, whereColumns) : rowWhereClause(row, tableColumns.value.map((column) => column.key));
      return `UPDATE ${tableName} SET ${setSql} WHERE ${whereSql};`;
    }).join("\n");

    copyText(sql, kind === "insert" ? "已复制 Insert 语句" : "已复制 Update 语句");
  } catch (error) {
    ElMessage.error(`生成 SQL 失败：${error}`);
  }
}

function canPasteIntoResultSelection() {
  return ["table", "query"].includes(props.activeTopTab?.kind)
    && selectedResultCellRange.value.startRow !== null
    && selectedResultCellRange.value.startColumn !== null;
}

async function pasteIntoSelectedCells() {
  if (!canPasteIntoResultSelection()) {
    return;
  }

  try {
    const text = await navigator.clipboard.readText();
    if (!text) {
      return;
    }

    const { startRow, startColumn } = selectedResultCellRange.value;
    const pastedRows = text.replace(/\r\n/g, "\n").replace(/\r/g, "\n").split("\n").map((row) => row.split("\t"));
    const result = activeResult.value;
    if (!result || startRow === null || startColumn === null) {
      return;
    }

    const sourceRows = searchedResultRows.value;
    const nextRows = [...(result.rows ?? [])];
    for (let rowOffset = 0; rowOffset < pastedRows.length; rowOffset += 1) {
      const rowIndex = startRow + rowOffset;
      const sourceRow = sourceRows[rowIndex];
      const resultRowIndex = nextRows.findIndex((row) => rowInternalId(row) === rowInternalId(sourceRow));
      if (!sourceRow || resultRowIndex < 0) {
        break;
      }

      const nextRow = { ...sourceRow };
      for (let columnOffset = 0; columnOffset < pastedRows[rowOffset].length; columnOffset += 1) {
        const column = tableColumns.value[startColumn + columnOffset];
        if (!column) {
          break;
        }
        nextRow[column.key] = pastedRows[rowOffset][columnOffset];
      }
      nextRows[resultRowIndex] = nextRow;
      if (canEditActiveResult.value) {
        markRowChanged(nextRow);
      }
    }

    updateActiveRows(nextRows);
    ElMessage.success(canEditActiveResult.value ? "已粘贴，待提交" : "已粘贴到当前结果");
  } catch (error) {
    ElMessage.error(`粘贴失败：${error}`);
  }
}

async function deleteSelectedRecords() {
  const rows = selectedResultRows();
  const tableName = activeTableNameSql();
  if (!tableName || rows.length === 0) {
    return;
  }

  try {
    const detail = await loadActiveTableDetail();
    const keyColumns = primaryKeyColumns(detail);
    if (keyColumns.length === 0) {
      ElMessage.warning("当前表没有主键，无法安全删除记录");
      return;
    }

    await ElMessageBox.confirm(`确定删除选中的 ${rows.length} 条记录吗？`, "删除记录", {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning",
      customClass: "bruno-message-box",
    });

    const operationToken = tableOperationToken += 1;
    loading.value = true;
    for (const row of rows) {
      if (operationToken !== tableOperationToken) {
        return;
      }
      const sql = `DELETE FROM ${tableName} WHERE ${rowWhereClause(row, keyColumns)} LIMIT 1;`;
      await executeMysqlQuery(currentConfig(), activeResult.value.schema, sql);
    }
    if (operationToken !== tableOperationToken) {
      return;
    }
    ElMessage.success("记录已删除");
    await loadTableData(activeResult.value.schema, activeResult.value.table, props.activeTopTab.id, {
      page: activeResult.value.page,
      pageSize: activeResult.value.pageSize,
    });
  } catch (error) {
    if (error !== "cancel" && error !== "close") {
      ElMessage.error(`删除失败：${error}`);
    }
  } finally {
    loading.value = false;
  }
}

function refreshActiveResult() {
  const tab = props.activeTopTab;
  if (tab?.kind === "table" && activeResult.value?.schema && activeResult.value?.table) {
    loadTableData(activeResult.value.schema, activeResult.value.table, tab.id, {
      page: activeResult.value.page,
      pageSize: activeResult.value.pageSize,
    });
  } else if (tab?.kind === "query") {
    executeActiveQuery();
  }
}

function addResultRow() {
  if (!canEditActiveResult.value) {
    return;
  }

  const state = ensureActiveEditState();
  if (!state) {
    return;
  }

  const row = Object.fromEntries(tableColumns.value.map((column) => [column.key, null]));
  row.__myhubRowId = `new-${newRowSequence += 1}`;
  state.newRows.push(row.__myhubRowId);
  updateActiveRows([row, ...(activeResult.value?.rows ?? [])]);
  clearResultRowSelection();
  selectResultCellRange(0, 0);
}

function cancelTableChanges() {
  const result = activeResult.value;
  const state = activeEditState.value;
  if (!result || !state) {
    return;
  }

  const restoredRows = (result.rows ?? [])
    .filter((row) => !state.newRows.includes(rowInternalId(row)))
    .map((row) => {
      const original = state.originalRows.get(rowInternalId(row));
      return original ? { ...original, __myhubRowId: rowInternalId(row) } : row;
    });
  clearEditState();
  editingCell.value = null;
  updateActiveRows(restoredRows);
  ElMessage.success("已取消未提交更改");
}

async function commitTableChanges() {
  const result = activeResult.value;
  const state = activeEditState.value;
  const tableName = activeTableNameSql();
  if (!canEditActiveResult.value || !result || !state || !tableName || !hasPendingTableChanges.value) {
    return;
  }

  try {
    const detail = await loadActiveTableDetail();
    const keyColumns = primaryKeyColumns(detail);
    if (state.updatedRows.size > 0 && keyColumns.length === 0) {
      ElMessage.warning("当前表没有主键，无法安全提交更新");
      return;
    }

    const operationToken = tableOperationToken += 1;
    loading.value = true;
    const newRowIds = new Set(state.newRows);
    const rowsById = new Map((result.rows ?? []).map((row) => [rowInternalId(row), row]));
    for (const rowId of state.newRows) {
      const row = rowsById.get(rowId);
      if (!row) {
        continue;
      }
      const values = publicRow(row);
      const columns = Object.keys(values);
      const sql = `INSERT INTO ${tableName} (${columns.map(quoteIdentifier).join(", ")}) VALUES (${columns.map((column) => mysqlValueLiteral(values[column])).join(", ")});`;
      await executeMysqlQuery(currentConfig(), result.schema, sql);
      if (operationToken !== tableOperationToken) {
        return;
      }
    }

    for (const [rowId] of state.updatedRows) {
      if (newRowIds.has(rowId)) {
        continue;
      }
      const row = rowsById.get(rowId);
      const original = state.originalRows.get(rowId);
      if (!row || !original) {
        continue;
      }
      const changedColumns = tableColumns.value
        .map((column) => column.key)
        .filter((column) => original[column] !== row[column]);
      if (changedColumns.length === 0) {
        continue;
      }
      const setSql = changedColumns.map((column) => `${quoteIdentifier(column)} = ${mysqlValueLiteral(row[column])}`).join(", ");
      const sql = `UPDATE ${tableName} SET ${setSql} WHERE ${rowWhereClause(original, keyColumns)} LIMIT 1;`;
      await executeMysqlQuery(currentConfig(), result.schema, sql);
      if (operationToken !== tableOperationToken) {
        return;
      }
    }

    ElMessage.success("更改已提交");
    clearEditState();
    await loadTableData(result.schema, result.table, props.activeTopTab.id, {
      page: result.page,
      pageSize: result.pageSize,
    });
  } catch (error) {
    ElMessage.error(`提交失败：${error}`);
  } finally {
    loading.value = false;
  }
}

function stopActiveOperation() {
  tableOperationToken += 1;
  loading.value = false;
  ElMessage.info("已停止等待当前操作");
}

function startCellEdit(rowIndex, columnIndex) {
  if (!canEditActiveResult.value) {
    return;
  }

  const row = searchedResultRows.value[rowIndex];
  const column = tableColumns.value[columnIndex];
  if (!row || !column) {
    return;
  }

  editingCell.value = {
    rowId: rowInternalId(row),
    columnKey: column.key,
    value: row[column.key] ?? "",
  };
  nextTick(() => {
    const input = tableViewport.value?.querySelector(".virtual-table__cell-input");
    input?.focus();
    input?.select();
  });
}

function commitCellEdit() {
  const edit = editingCell.value;
  if (!edit) {
    return;
  }

  const row = (activeResult.value?.rows ?? []).find((item) => rowInternalId(item) === edit.rowId);
  const column = tableColumns.value.find((item) => item.key === edit.columnKey);
  if (row && column) {
    setResultCellValue(row, column, edit.value);
  }
  editingCell.value = null;
}

function cancelCellEdit() {
  editingCell.value = null;
}

function isEditingResultCell(row, column) {
  return editingCell.value?.rowId === rowInternalId(row) && editingCell.value?.columnKey === column.key;
}

function selectSchemaTable(row) {
  selectedSchemaTableKey.value = schemaTableSelectKey(row);
}

function schemaTableRowClass({ row }) {
  return selectedSchemaTableKey.value === schemaTableSelectKey(row) ? "selected-schema-table-row" : "";
}

function absoluteResultRowIndex(visibleIndex) {
  return visibleTableRows.value.start + visibleIndex;
}

function clearResultRowSelection() {
  selectedResultRowRange.value = { start: null, end: null };
  isSelectingResultRows.value = false;
}

function clearResultCellSelection() {
  selectedResultCellRange.value = { startRow: null, endRow: null, startColumn: null, endColumn: null };
  isSelectingResultCells.value = false;
}

function selectResultRowRange(start, end = start) {
  selectedResultRowRange.value = { start, end };
}

function selectResultCellRange(startRow, startColumn, endRow = startRow, endColumn = startColumn) {
  selectedResultCellRange.value = { startRow, endRow, startColumn, endColumn };
}

function isResultRowSelected(visibleIndex) {
  const { start, end } = selectedResultRowRange.value;
  if (start === null || end === null) {
    return false;
  }

  const index = absoluteResultRowIndex(visibleIndex);
  return index >= Math.min(start, end) && index <= Math.max(start, end);
}

function isResultCellSelected(visibleIndex, columnIndex) {
  const { startRow, endRow, startColumn, endColumn } = selectedResultCellRange.value;
  if (startRow === null || endRow === null || startColumn === null || endColumn === null) {
    return false;
  }

  const rowIndex = absoluteResultRowIndex(visibleIndex);
  return rowIndex >= Math.min(startRow, endRow)
    && rowIndex <= Math.max(startRow, endRow)
    && columnIndex >= Math.min(startColumn, endColumn)
    && columnIndex <= Math.max(startColumn, endColumn);
}

function startResultRowSelection(event, visibleIndex) {
  if (event.button !== 0) {
    return;
  }

  const index = absoluteResultRowIndex(visibleIndex);
  clearResultCellSelection();
  isSelectingResultRows.value = true;
  selectResultRowRange(index);
}

function extendResultRowSelection(visibleIndex) {
  if (!isSelectingResultRows.value) {
    return;
  }

  const { start } = selectedResultRowRange.value;
  if (start === null) {
    return;
  }

  selectResultRowRange(start, absoluteResultRowIndex(visibleIndex));
}

function stopResultRowSelection() {
  isSelectingResultRows.value = false;
}

function startResultCellSelection(event, visibleIndex, columnIndex) {
  if (event.button !== 0) {
    return;
  }

  const rowIndex = absoluteResultRowIndex(visibleIndex);
  clearResultRowSelection();
  isSelectingResultCells.value = true;
  selectResultCellRange(rowIndex, columnIndex);
}

function extendResultCellSelection(visibleIndex, columnIndex) {
  if (!isSelectingResultCells.value) {
    return;
  }

  const { startRow, startColumn } = selectedResultCellRange.value;
  if (startRow === null || startColumn === null) {
    return;
  }

  selectResultCellRange(startRow, startColumn, absoluteResultRowIndex(visibleIndex), columnIndex);
}

function stopResultCellSelection() {
  isSelectingResultCells.value = false;
}

function startColumnResize(event, column) {
  if (event.button !== 0 || !props.activeTopTab?.id) {
    return;
  }

  stopResultRowSelection();
  stopResultCellSelection();
  resizingColumn.value = {
    tabId: props.activeTopTab.id,
    columnKey: column.key,
    startX: event.clientX,
    startWidth: column.width,
  };
  document.body.classList.add("is-resizing-table-column");
}

function resizeColumn(event) {
  if (!resizingColumn.value) {
    return;
  }

  const { tabId, columnKey, startX, startWidth } = resizingColumn.value;
  const width = clampColumnWidth(startWidth + event.clientX - startX);
  tableColumnWidths.value = {
    ...tableColumnWidths.value,
    [tabId]: {
      ...(tableColumnWidths.value[tabId] ?? {}),
      [columnKey]: width,
    },
  };
}

function stopColumnResize() {
  if (!resizingColumn.value) {
    return;
  }

  resizingColumn.value = null;
  document.body.classList.remove("is-resizing-table-column");
}

function openSchemaTable(row) {
  selectSchemaTable(row);
  emit("open-table-query", {
    connection: normalizedConnection.value,
    schema: row.schema,
    groupType: "table",
    item: row.name,
  });
}

function tableCellValue(row, column) {
  return formatCellValue(row[column.key]);
}

async function ensureTableDesignState(tab = props.activeTopTab) {
  if (!tab || tab.kind !== "table-design") {
    return;
  }

  if (tableDesignStates.value[tab.id]) {
    return;
  }

  const state = createDesignState(tab);
  tableDesignStates.value = {
    ...tableDesignStates.value,
    [tab.id]: state,
  };

  if (tab.mode !== "edit" || !tab.table) {
    return;
  }

  loading.value = true;
  try {
    const detail = await describeMysqlTable(currentConfig(), tab.schema, tab.table);
    applyOptionsFromTableDetail(state, detail);
    tableDesignStates.value = {
      ...tableDesignStates.value,
      [tab.id]: {
        ...state,
        loaded: true,
        columns: columnsFromTableDetail(detail),
        indexes: indexesFromTableDetail(detail),
        foreignKeys: foreignKeysFromTableDetail(detail),
        triggers: triggersFromTableDetail(detail),
        checks: checksFromTableDetail(detail),
      },
    };
  } catch (error) {
    ElMessage.error(`加载表结构失败：${error}`);
  } finally {
    loading.value = false;
  }
}

async function saveTableDesign() {
  const tab = props.activeTopTab;
  const state = activeDesignState.value;
  if (!tab || tab.kind !== "table-design" || !state) {
    return;
  }

  try {
    const wasCreate = state.mode === "create";
    const sql = buildTableDesignSql(tab, state);
    const statements = Array.isArray(sql) ? sql.filter(Boolean) : [sql].filter(Boolean);
    if (statements.length === 0) {
      ElMessage.info("没有结构变更");
      return;
    }

    state.saving = true;
    for (const statement of statements) {
      await executeMysqlQuery(currentConfig(), tab.schema, statement);
    }
    markDesignStateSaved(state);
    ElMessage.success(wasCreate ? "表已创建" : "表结构已保存");
    emit("table-design-saved", {
      tabId: tab.id,
      connectionId: normalizedConnection.value.id,
      database: tab.schema,
      table: wasCreate ? "" : tab.table,
      newTable: state.tableName,
      wasCreate,
    });
    emit("refresh-connection", normalizedConnection.value);
  } catch (error) {
    ElMessage.error(`保存表结构失败：${error.message ?? error}`);
  } finally {
    if (state) {
      state.saving = false;
    }
  }
}

function handleSchemaTableScroll(event) {
  schemaTableScrollTop.value = event.currentTarget.scrollTop;
  schemaTableScrollLeft.value = event.currentTarget.scrollLeft;
}

function handleTableScroll(event) {
  tableScrollTop.value = event.currentTarget.scrollTop;
  tableScrollLeft.value = event.currentTarget.scrollLeft;
}

function updateTableViewportHeight() {
  tableViewportHeight.value = tableViewport.value?.clientHeight || 420;
}

function resetTableScroll() {
  tableScrollTop.value = 0;
  tableScrollLeft.value = 0;
  schemaTableScrollTop.value = 0;
  schemaTableScrollLeft.value = 0;
  if (schemaTableViewport.value) {
    schemaTableViewport.value.scrollTop = 0;
    schemaTableViewport.value.scrollLeft = 0;
  }
  if (tableViewport.value) {
    tableViewport.value.scrollTop = 0;
    tableViewport.value.scrollLeft = 0;
  }
  updateTableViewportHeight();
}

onMounted(() => {
  updateTableViewportHeight();
  if (tableViewport.value) {
    tableResizeObserver = new ResizeObserver(updateTableViewportHeight);
    tableResizeObserver.observe(tableViewport.value);
  }
  window.addEventListener("mouseup", stopSchemaRowSelection);
  window.addEventListener("mouseup", stopSchemaCellSelection);
  window.addEventListener("mouseup", stopResultRowSelection);
  window.addEventListener("mouseup", stopResultCellSelection);
  window.addEventListener("mousemove", resizeColumn);
  window.addEventListener("mouseup", stopColumnResize);
  window.addEventListener("keydown", handleKeydown);
});

onBeforeUnmount(() => {
  saveTabViewState();
  destroyQueryEditor();
  tableResizeObserver?.disconnect();
  window.removeEventListener("mouseup", stopSchemaRowSelection);
  window.removeEventListener("mouseup", stopSchemaCellSelection);
  window.removeEventListener("mouseup", stopResultRowSelection);
  window.removeEventListener("mouseup", stopResultCellSelection);
  window.removeEventListener("mousemove", resizeColumn);
  window.removeEventListener("mouseup", stopColumnResize);
  window.removeEventListener("keydown", handleKeydown);
  document.body.classList.remove("is-resizing-table-column");
});

async function loadTableData(schema, table, tabId = props.activeTopTab?.id, options = {}) {
  if (!tabId) {
    return;
  }

  const previous = tableResults.value[tabId];
  const pageSize = normalizePositiveInteger(options.pageSize ?? previous?.pageSize, DEFAULT_PAGE_SIZE);
  const page = normalizePositiveInteger(options.page ?? previous?.page, 1);
  const offset = (page - 1) * pageSize;
  const operationToken = tableOperationToken += 1;

  loading.value = true;
  const tableName = `${quoteIdentifier(schema)}.${quoteIdentifier(table)}`;
  const sql = `SELECT *\nFROM ${tableName}\nLIMIT ${pageSize} OFFSET ${offset};`;
  const countSql = `SELECT COUNT(*) AS total\nFROM ${tableName};`;
  try {
    const [result, countResult] = await Promise.all([
      executeMysqlQuery(currentConfig(), schema, sql),
      executeMysqlQuery(currentConfig(), schema, countSql),
    ]);
    if (operationToken !== tableOperationToken) {
      return;
    }
    const totalRows = Number(countResult.rows?.[0]?.total ?? 0);
    const rows = editableRows(result.rows ?? []);
    tableResults.value = {
      ...tableResults.value,
      [tabId]: {
        ...result,
        rows,
        schema,
        table,
        page,
        pageSize,
        totalRows,
      },
    };
    await nextTick();
    clearResultRowSelection();
    clearResultCellSelection();
    clearEditState(tabId);
    resetTableScroll();
    saveTabViewState(tabId);
  } catch (error) {
    if (operationToken === tableOperationToken) {
      ElMessage.error(`加载表数据失败：${error}`);
    }
  } finally {
    if (operationToken === tableOperationToken) {
      loading.value = false;
    }
  }
}

async function executeActiveQuery() {
  const tab = props.activeTopTab;
  if (!tab || tab.kind !== "query") {
    return;
  }

  const selectedSql = selectedQueryText().trim();
  const sql = (selectedSql || activeQueryText.value).trim();
  if (!sql) {
    ElMessage.warning("请输入 SQL");
    return;
  }

  const operationToken = tableOperationToken += 1;
  loading.value = true;
  try {
    const result = await executeMysqlQuery(currentConfig(), tab.schema, sql);
    if (operationToken !== tableOperationToken) {
      return;
    }
    tableResults.value = {
      ...tableResults.value,
      [tab.id]: {
        ...result,
        rows: editableRows(result.rows ?? []),
        schema: tab.schema,
        table: null,
        page: 1,
        pageSize: result.rows?.length ?? 0,
        totalRows: result.rows?.length ?? 0,
      },
    };
    await nextTick();
    clearResultRowSelection();
    clearResultCellSelection();
    clearEditState(tab.id);
    resetTableScroll();
    saveTabViewState(tab.id);
  } catch (error) {
    if (operationToken === tableOperationToken) {
      ElMessage.error(`执行查询失败：${error}`);
    }
  } finally {
    if (operationToken === tableOperationToken) {
      loading.value = false;
    }
  }
}

async function saveActiveQuery() {
  const tab = props.activeTopTab;
  if (!tab || tab.kind !== "query") {
    return;
  }

  const sql = activeQueryText.value.trim();
  if (!sql) {
    ElMessage.warning("请输入 SQL");
    return;
  }

  try {
    const { value } = await ElMessageBox.prompt("输入查询名称", tab.savedQueryId ? "保存查询" : "新建查询", {
      inputValue: tab.savedQueryId ? tab.label : "",
      inputPlaceholder: "例如：用户增长明细",
      confirmButtonText: "保存",
      cancelButtonText: "取消",
      customClass: "bruno-message-box folder-prompt-box",
      inputValidator(value) {
        return String(value ?? "").trim() ? true : "请输入查询名称";
      },
    });
    const name = String(value ?? "").trim();
    emit("save-query", {
      tabId: tab.id,
      connectionId: normalizedConnection.value.id,
      queryId: tab.savedQueryId,
      schema: tab.schema,
      name,
      sql: activeQueryText.value,
    });
  } catch (error) {
    if (error !== "cancel" && error !== "close") {
      ElMessage.error(`保存查询失败：${error}`);
    }
  }
}

async function loadSchemaTableCounts(schema) {
  const tableGroup = schema?.groups?.find((group) => (group.groupType ?? group.type) === "table");
  const tables = tableGroup?.items?.map((item) => (typeof item === "string" ? item : item.name)) ?? [];
  const missingTables = tables.filter((table) => schemaTableCounts.value[schemaTableCountKey(schema.name, table)] === undefined);

  if (missingTables.length === 0) {
    return;
  }

  const nextCounts = { ...schemaTableCounts.value };
  const workers = Array.from({ length: Math.min(4, missingTables.length) }, async (_, workerIndex) => {
    for (let index = workerIndex; index < missingTables.length; index += 4) {
      const table = missingTables[index];
      const sql = `SELECT COUNT(*) AS total\nFROM ${quoteIdentifier(schema.name)}.${quoteIdentifier(table)};`;
      const result = await executeMysqlQuery(currentConfig(), schema.name, sql);
      nextCounts[schemaTableCountKey(schema.name, table)] = Number(result.rows?.[0]?.total ?? 0);
    }
  });

  try {
    await Promise.all(workers);
    schemaTableCounts.value = nextCounts;
  } catch (error) {
    ElMessage.error(`加载表数据量失败：${error}`);
  }
}

async function loadSchemaTableMetadata(schema) {
  const tableGroup = schema?.groups?.find((group) => (group.groupType ?? group.type) === "table");
  const tables = tableGroup?.items?.map((item) => (typeof item === "string" ? item : item.name)) ?? [];
  if (tables.length === 0) {
    return;
  }

  const sql = `SELECT
    TABLE_NAME AS name,
    TABLE_ROWS AS rowCount,
    DATA_LENGTH AS dataLength,
    ENGINE AS engine,
    DATE_FORMAT(CREATE_TIME, '%Y-%m-%d %H:%i:%s') AS createTime,
    DATE_FORMAT(UPDATE_TIME, '%Y-%m-%d %H:%i:%s') AS updateTime,
    TABLE_COLLATION AS collation,
    TABLE_COMMENT AS comment
  FROM information_schema.TABLES
  WHERE TABLE_SCHEMA = ${quoteString(schema.name)}
    AND TABLE_TYPE = 'BASE TABLE'
  ORDER BY TABLE_NAME;`;

  try {
    const result = await executeMysqlQuery(currentConfig(), "information_schema", sql);
    const nextMetadata = { ...schemaTableMetadata.value };
    for (const row of result.rows ?? []) {
      nextMetadata[schemaTableCountKey(schema.name, row.name)] = {
        rowCount: Number(row.rowCount ?? 0),
        dataLength: Number(row.dataLength ?? 0),
        engine: row.engine ?? "",
        createTime: row.createTime ?? "",
        updateTime: row.updateTime ?? "",
        collation: row.collation ?? "",
        comment: row.comment ?? "",
      };
    }
    schemaTableMetadata.value = nextMetadata;
  } catch (error) {
    ElMessage.error(`加载表结构信息失败：${error}`);
  }
}

function handlePageSizeChange(pageSize) {
  if (!props.activeTopTab || props.activeTopTab.kind !== "table") {
    return;
  }

  loadTableData(props.activeTopTab.schema, props.activeTopTab.table, props.activeTopTab.id, {
    page: 1,
    pageSize,
  });
}

function handlePageChange(page) {
  if (!props.activeTopTab || props.activeTopTab.kind !== "table") {
    return;
  }

  loadTableData(props.activeTopTab.schema, props.activeTopTab.table, props.activeTopTab.id, {
    page,
    pageSize: activeResult.value?.pageSize ?? DEFAULT_PAGE_SIZE,
  });
}

</script>

<template>
  <section class="database-workspace" v-loading="loading">
    <section class="content-panel" :class="{ 'is-empty': !activeTopTab || activeTopTab.id === 'database' }">
      <div v-if="!activeTopTab || activeTopTab.id === 'database'" class="empty-workspace">
        <div class="empty-workspace__panel">
          <div class="empty-workspace__visual">
            <el-icon><DataAnalysis /></el-icon>
          </div>
          <div class="empty-workspace__copy">
            <p>待打开</p>
            <strong>{{ normalizedConnection.name ?? "选择一个数据库连接" }}</strong>
            <span>{{ databaseTarget }}</span>
          </div>
          <div class="empty-workspace__steps">
            <div class="empty-workspace__step active">
              <span>1</span>
              <strong>选择连接</strong>
            </div>
            <el-icon><Right /></el-icon>
            <div class="empty-workspace__step">
              <span>2</span>
              <strong>打开库</strong>
            </div>
            <el-icon><Right /></el-icon>
            <div class="empty-workspace__step">
              <span>3</span>
              <strong>查看表</strong>
            </div>
          </div>
        </div>
      </div>

      <section v-else-if="activeTopTab.kind === 'schema'" class="tab-content">
        <div v-if="tableSearchOpen" class="table-search">
          <el-icon><Search /></el-icon>
          <input
            ref="searchInputRef"
            v-model="tableSearchQuery"
            type="search"
            placeholder="搜索表列表"
            @keydown="handleSearchKeydown"
          />
          <span>{{ searchedSchemaTables.length }} / {{ activeSchemaTables.length }}</span>
          <button type="button" aria-label="关闭搜索" @click="closeTableSearch">
            <el-icon><Close /></el-icon>
          </button>
        </div>
        <div class="virtual-table" @contextmenu.prevent="openCopyContextMenu">
          <div class="virtual-table__header-wrap">
            <div class="virtual-table__gutter-head" />
            <div
              class="virtual-table__header"
              :style="{
                gridTemplateColumns: schemaTableGridTemplate,
                width: `${schemaTableContentWidth}px`,
                transform: `translateX(-${schemaTableScrollLeft}px)`,
              }"
            >
              <div v-for="column in schemaTableColumns" :key="column.key" class="virtual-table__th">
                <span class="virtual-table__th-label">{{ column.label }}</span>
                <span
                  class="virtual-table__resize-handle"
                  @mousedown.stop.prevent="startColumnResize($event, column)"
                />
              </div>
            </div>
          </div>
          <div ref="schemaTableViewport" class="virtual-table__viewport" @scroll="handleSchemaTableScroll">
            <div class="virtual-table__body" :style="{ width: `${schemaTableContentWidth + 32}px` }">
              <div class="virtual-table__gutter">
                <div
                  v-for="(row, rowIndex) in searchedSchemaTables"
                  :key="`schema-gutter-${row.name}`"
                  class="virtual-table__gutter-cell"
                  :class="{ selected: isSchemaRowSelected(rowIndex) }"
                  @mousedown.prevent="startSchemaRowSelection($event, rowIndex)"
                  @mouseenter="extendSchemaRowSelection(rowIndex)"
                  @dblclick="openSchemaTable(row)"
                />
              </div>
              <div class="virtual-table__rows">
                <div
                  v-for="(row, rowIndex) in searchedSchemaTables"
                  :key="row.name"
                  class="virtual-table__row"
                  :class="{ selected: isSchemaRowSelected(rowIndex) }"
                  :style="{ gridTemplateColumns: schemaTableGridTemplate }"
                  @dblclick="openSchemaTable(row)"
                >
                  <div
                    v-for="(column, columnIndex) in schemaTableColumns"
                    :key="column.key"
                    class="virtual-table__cell"
                    :class="{
                      selected: isSchemaCellSelected(rowIndex, columnIndex),
                      matched: hasTableSearch && schemaTableCellValue(row, column).toLowerCase().includes(normalizedTableSearch),
                      'is-right': column.align === 'right',
                    }"
                    :title="schemaTableCellValue(row, column)"
                    @mousedown.prevent="startSchemaCellSelection($event, rowIndex, columnIndex)"
                    @mouseenter="extendSchemaCellSelection(rowIndex, columnIndex)"
                  >
                    <span v-if="column.key === 'name'" class="schema-table-name">
                      <span class="schema-table-icon table-icon" />
                      <span class="schema-table-name__text">{{ row.name }}</span>
                    </span>
                    <template v-else>{{ schemaTableCellValue(row, column) }}</template>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      <TableDesigner
        v-else-if="activeTopTab.kind === 'table-design'"
        :schema="activeTopTab.schema"
        :sql-error="activeDesignSqlPreview.error"
        :sql-preview="activeDesignSqlPreview.sql"
        :state="activeDesignState"
        @save="saveTableDesign"
      />

      <section
        v-else-if="['table', 'query'].includes(activeTopTab.kind)"
        class="tab-content has-footer"
        :class="{
          'query-tab': activeTopTab.kind === 'query',
          'query-tab-empty': activeTopTab.kind === 'query' && !shouldShowResultPanel,
        }"
      >
        <div v-if="activeTopTab.kind === 'query'" class="query-editor">
          <div class="query-editor__bar">
            <el-select
              v-model="activeQuerySchemaName"
              class="query-schema-select"
              size="small"
              filterable
              clearable
              placeholder="选择库"
              no-match-text="没有匹配的库"
              no-data-text="暂无库"
              popper-class="query-schema-select-popper"
              :disabled="querySchemaOptions.length === 0"
            >
              <el-option
                v-for="schema in querySchemaOptions"
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
                @click="executeActiveQuery"
              >
                {{ queryRunLabel }}
              </el-button>
              <el-button
                class="query-run-button"
                :icon="DocumentChecked"
                size="small"
                @click="saveActiveQuery"
              >
                保存
              </el-button>
            </div>
          </div>
          <div ref="queryEditorRoot" class="query-editor__host" :class="{ ready: queryEditorReady }" />
        </div>
        <div v-if="shouldShowResultPanel && tableSearchOpen" class="table-search">
          <el-icon><Search /></el-icon>
          <input
            ref="searchInputRef"
            v-model="tableSearchQuery"
            type="search"
            :placeholder="activeTopTab.kind === 'query' ? '搜索查询结果' : '搜索当前页数据'"
            @keydown="handleSearchKeydown"
          />
          <span>{{ searchedResultRows.length }} / {{ activeResult?.rows.length ?? 0 }}</span>
          <button type="button" aria-label="关闭搜索" @click="closeTableSearch">
            <el-icon><Close /></el-icon>
          </button>
        </div>
        <div v-if="shouldShowResultPanel" class="virtual-table" @contextmenu.prevent="openCopyContextMenu">
          <div class="virtual-table__header-wrap">
            <div class="virtual-table__gutter-head" />
            <div
              class="virtual-table__header"
              :style="{
                gridTemplateColumns: tableGridTemplate,
                width: `${tableContentWidth}px`,
                transform: `translateX(-${tableScrollLeft}px)`,
              }"
            >
              <div v-for="column in tableColumns" :key="column.key" class="virtual-table__th">
                <span class="virtual-table__th-label">{{ column.label }}</span>
                <span
                  class="virtual-table__resize-handle"
                  @mousedown.stop.prevent="startColumnResize($event, column)"
                />
              </div>
            </div>
          </div>
          <div ref="tableViewport" class="virtual-table__viewport" @scroll="handleTableScroll">
            <div class="virtual-table__body" :style="{ width: `${tableContentWidth + 32}px` }">
              <div class="virtual-table__gutter">
                <div class="virtual-table__spacer" :style="{ height: `${visibleTableRows.top}px` }" />
                <div
                  v-for="(_, visibleIndex) in visibleTableRows.rows"
                  :key="`gutter-${visibleTableRows.start + visibleIndex}`"
                  class="virtual-table__gutter-cell"
                  :class="{ selected: isResultRowSelected(visibleIndex) }"
                  @mousedown.prevent="startResultRowSelection($event, visibleIndex)"
                  @mouseenter="extendResultRowSelection(visibleIndex)"
                  @contextmenu.prevent.stop="openCopyContextMenu($event, absoluteResultRowIndex(visibleIndex))"
                />
                <div class="virtual-table__spacer" :style="{ height: `${visibleTableRows.bottom}px` }" />
              </div>
              <div class="virtual-table__rows">
                <div class="virtual-table__spacer" :style="{ height: `${visibleTableRows.top}px` }" />
                <div
                  v-for="(row, visibleIndex) in visibleTableRows.rows"
                  :key="visibleTableRows.start + visibleIndex"
                class="virtual-table__row"
                :class="{ selected: isResultRowSelected(visibleIndex) }"
                :style="{ gridTemplateColumns: tableGridTemplate }"
                @contextmenu.prevent.stop="openCopyContextMenu($event, absoluteResultRowIndex(visibleIndex))"
              >
              <div
                v-for="(column, columnIndex) in tableColumns"
                :key="column.key"
                class="virtual-table__cell"
                :class="{
                  selected: isResultCellSelected(visibleIndex, columnIndex),
                  changed: isChangedResultCell(row, column),
                  'is-new-row': isNewResultRow(row),
                  matched: hasTableSearch && tableCellValue(row, column).toLowerCase().includes(normalizedTableSearch),
                }"
                :title="tableCellValue(row, column)"
                @mousedown.prevent="startResultCellSelection($event, visibleIndex, columnIndex)"
                @mouseenter="extendResultCellSelection(visibleIndex, columnIndex)"
                @contextmenu.prevent.stop="openCopyContextMenu($event, absoluteResultRowIndex(visibleIndex), columnIndex)"
                @dblclick.stop="startCellEdit(absoluteResultRowIndex(visibleIndex), columnIndex)"
              >
                <input
                  v-if="isEditingResultCell(row, column)"
                  v-model="editingCell.value"
                  class="virtual-table__cell-input"
                  type="text"
                  autofocus
                  @mousedown.stop
                  @blur="commitCellEdit"
                  @keydown.enter.prevent="commitCellEdit"
                  @keydown.esc.prevent="cancelCellEdit"
                />
                <template v-else>{{ tableCellValue(row, column) }}</template>
              </div>
                </div>
                <div class="virtual-table__spacer" :style="{ height: `${visibleTableRows.bottom}px` }" />
              </div>
            </div>
          </div>
        </div>
        <footer v-if="shouldShowResultPanel" class="table-footer">
          <div class="table-footer__tools" aria-label="数据工具">
            <button type="button" class="table-footer__tool" title="新增记录" :disabled="!canEditActiveResult" @click="addResultRow">
              <el-icon><Plus /></el-icon>
            </button>
            <button
              type="button"
              class="table-footer__tool"
              title="删除记录"
              :disabled="activeTopTab.kind !== 'table' || selectedResultRows().length === 0"
              @click="deleteSelectedRecords"
            >
              <el-icon><Minus /></el-icon>
            </button>
            <span class="table-footer__tool-separator" />
            <button type="button" class="table-footer__tool" title="提交更改" :disabled="!hasPendingTableChanges || loading" @click="commitTableChanges">
              <el-icon><Check /></el-icon>
            </button>
            <button type="button" class="table-footer__tool" title="取消更改" :disabled="!hasPendingTableChanges" @click="cancelTableChanges">
              <el-icon><Close /></el-icon>
            </button>
            <span class="table-footer__tool-separator" />
            <button type="button" class="table-footer__tool" title="刷新" :disabled="loading" @click="refreshActiveResult">
              <el-icon><Refresh /></el-icon>
            </button>
            <button
              type="button"
              class="table-footer__tool table-footer__tool--block"
              title="停止"
              :disabled="!canStopActiveOperation"
              @click="stopActiveOperation"
            />
          </div>
          <span v-if="activeResult" class="table-footer__summary">
            <template v-if="activeTopTab.kind === 'table'">
              第 {{ activeResult.page }} 页 · 显示 {{ searchedResultRows.length }} / {{ activeResult.totalRows }} 行 · {{ activeResult.elapsedMs }}ms
            </template>
            <template v-else>
              显示 {{ searchedResultRows.length }} / {{ activeResult.totalRows }} 行 · {{ activeResult.elapsedMs }}ms
            </template>
          </span>
          <el-pagination
            v-if="activeTopTab.kind === 'table'"
            background
            layout="sizes, prev, pager, next, jumper"
            popper-class="table-page-size-popper"
            :current-page="activeResult?.page ?? 1"
            :page-size="activeResult?.pageSize ?? DEFAULT_PAGE_SIZE"
            :page-sizes="PAGE_SIZE_OPTIONS"
            :total="activeResult?.totalRows ?? 0"
            @size-change="handlePageSizeChange"
            @current-change="handlePageChange"
          />
        </footer>
      </section>
    </section>

    <ContextMenu
      v-model="copyContextOpen"
      :items="copyContextItems"
      :x="copyContextPosition.x"
      :y="copyContextPosition.y"
      @select="handleCopyContextSelect"
    />
  </section>
</template>

<style scoped>
.database-workspace {
  min-height: 0;
  flex: 1;
  background: transparent;
}

.content-panel {
  min-height: 0;
  height: 100%;
  padding: 10px;
}

.content-panel.is-empty {
  padding-top: 0;
  padding-right: 10px;
  padding-bottom: 10px;
  padding-left: 0;
}

.empty-workspace {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  min-width: 0;
  min-height: 0;
  padding: 26px;
  background:
    linear-gradient(90deg, rgba(242, 107, 58, 0.05), transparent 34%),
    linear-gradient(180deg, var(--surface-muted), #fff);
  color: var(--muted);
}

.empty-workspace__panel {
  display: grid;
  justify-items: center;
  width: min(520px, 100%);
  gap: 18px;
  text-align: center;
}

.empty-workspace__visual {
  display: grid;
  place-items: center;
  width: 58px;
  height: 58px;
  border: 1px solid #f5c5b3;
  border-radius: 8px;
  background: #fff7f4;
  color: var(--orange);
  box-shadow: 0 10px 26px rgba(242, 107, 58, 0.13);
}

.empty-workspace__visual .el-icon {
  font-size: 28px;
}

.empty-workspace__copy {
  display: grid;
  gap: 6px;
}

.empty-workspace__copy p {
  margin: 0;
  color: var(--orange);
  font-size: 11px;
  font-weight: 780;
}

.empty-workspace__copy strong {
  color: var(--text);
  font-size: 20px;
  font-weight: 780;
}

.empty-workspace__copy span {
  color: var(--muted);
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
  font-size: 12px;
}

.empty-workspace__steps {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 16px minmax(0, 1fr) 16px minmax(0, 1fr);
  align-items: center;
  width: 100%;
  max-width: 430px;
  gap: 8px;
}

.empty-workspace__steps > .el-icon {
  color: var(--line-strong);
  font-size: 13px;
}

.empty-workspace__step {
  display: grid;
  justify-items: center;
  min-width: 0;
  gap: 7px;
}

.empty-workspace__step span {
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #fff;
  color: var(--muted);
  font-size: 12px;
  font-weight: 760;
}

.empty-workspace__step strong {
  max-width: 100%;
  overflow: hidden;
  color: var(--muted);
  font-size: 12px;
  font-weight: 650;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.empty-workspace__step.active span {
  border-color: #f5c5b3;
  background: var(--orange-soft);
  color: var(--orange);
}

.empty-workspace__step.active strong {
  color: var(--text);
}

.empty-workspace__actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 8px;
}

.empty-workspace__actions :deep(.el-button) {
  height: 34px;
  margin: 0;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 650;
}

.empty-workspace__actions :deep(.el-button--primary) {
  border-color: var(--orange);
  background: var(--orange);
}

.empty-workspace__actions :deep(.el-button--primary:hover) {
  border-color: #e65d2e;
  background: #e65d2e;
}

.tab-content {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: #fff;
  box-shadow: none;
}

.query-editor {
  display: flex;
  flex: 0 0 238px;
  min-height: 160px;
  flex-direction: column;
  border-bottom: 1px solid var(--line);
  background: var(--panel);
}

.query-tab-empty .query-editor {
  flex: 1 1 auto;
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
  min-height: 0;
  flex: 1;
  width: 100%;
  overflow: hidden;
  background: #fff;
}

.query-editor__host :deep(.cm-editor) {
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

.table-search {
  display: grid;
  grid-template-columns: 18px minmax(0, 1fr) auto 28px;
  gap: 7px;
  align-items: center;
  min-height: 38px;
  padding: 5px 7px;
  border-bottom: 1px solid var(--line);
  background: #fff;
  color: var(--muted);
  font-size: 12px;
}

.table-search input {
  width: 100%;
  height: 28px;
  min-width: 0;
  padding: 0 9px;
  border: 1px solid var(--line);
  border-radius: 8px;
  outline: none;
  background: #fff;
  color: var(--text);
  font: inherit;
}

.table-search input:focus {
  border-color: var(--orange);
  box-shadow: 0 0 0 3px rgba(242, 107, 58, 0.13);
}

.table-search span {
  color: var(--faint);
  white-space: nowrap;
}

.table-search button {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #fff;
  color: var(--muted);
  cursor: pointer;
}

.table-search button:hover {
  border-color: var(--line-strong);
  background: var(--surface-strong);
  color: var(--text);
}

.hint {
  color: var(--muted);
  font-size: 12px;
  font-weight: 400;
}

.tab-content :deep(.el-table) {
  min-height: 0;
  flex: 1;
  --el-table-border-color: var(--line);
  --el-table-border: 1px solid var(--line);
  --el-table-header-bg-color: var(--surface-muted);
  --el-table-header-text-color: var(--muted);
  --el-table-row-hover-bg-color: #fafafa;
  color: #303647;
  font-size: 12px;
  font-weight: 450;
}

.tab-content.has-footer :deep(.el-table) {
  flex-basis: calc(100% - 82px);
}

.tab-content :deep(.el-table th.el-table__cell) {
  height: 34px;
  padding: 0;
  background: var(--surface-muted);
  border-bottom: 1px solid var(--line);
  color: var(--muted);
  font-size: 12px;
  font-weight: 400;
}

.tab-content :deep(.el-table th.el-table__cell .cell) {
  color: var(--muted);
  font-size: 12px;
  font-weight: 400;
  line-height: 34px;
}

.tab-content :deep(.el-table .sort-caret.ascending) {
  border-bottom-color: var(--faint);
}

.tab-content :deep(.el-table .sort-caret.descending) {
  border-top-color: var(--faint);
}

.tab-content :deep(.el-table .ascending .sort-caret.ascending) {
  border-bottom-color: var(--orange);
}

.tab-content :deep(.el-table .descending .sort-caret.descending) {
  border-top-color: var(--orange);
}

.tab-content :deep(.el-table .el-table__cell) {
  height: 34px;
  padding: 0;
  border-right-color: var(--line);
  border-bottom-color: var(--line);
}

.tab-content :deep(.el-table .cell) {
  display: flex;
  align-items: center;
  min-height: 34px;
  padding: 0 10px;
  line-height: 34px;
  white-space: nowrap;
}

.tab-content :deep(.el-table .selected-schema-table-row > .el-table__cell) {
  background: var(--orange-soft);
}

.tab-content :deep(.el-table .selected-schema-table-row:hover > .el-table__cell) {
  background: #ffe7de;
}

.tab-content :deep(.el-table__fixed),
.tab-content :deep(.el-table__fixed-right) {
  box-shadow: 1px 0 0 var(--line);
}

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
