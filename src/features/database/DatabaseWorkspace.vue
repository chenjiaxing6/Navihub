<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { ElMessage } from "element-plus";
import { Close, DataAnalysis, Right, Search } from "@element-plus/icons-vue";
import ContextMenu from "../../shared/ContextMenu.vue";
import { executeMysqlQuery } from "./mysqlApi";
import { ensureMysqlConnection, formatMysqlMeta } from "./databaseDefaults";

const props = defineProps({
  connection: { type: Object, required: true },
  activeTopTab: { type: Object, default: null },
  pendingSchemaOpen: { type: Object, default: null },
  pendingTableQuery: { type: Object, default: null },
});

const emit = defineEmits(["schema-loaded", "update-connection", "open-table-query"]);
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
const tableColumnWidths = ref({});
const tabViewStates = ref({});
const resizingColumn = ref(null);
const copyContextOpen = ref(false);
const copyContextPosition = ref({ x: 0, y: 0 });
const schemaTableScrollLeft = ref(0);
const schemaTableScrollTop = ref(0);
const schemaTableViewport = ref(null);
const tableViewport = ref(null);
const searchInputRef = ref(null);
const tableScrollTop = ref(0);
const tableScrollLeft = ref(0);
const tableViewportHeight = ref(420);
const tableSearchOpen = ref(false);
const tableSearchQuery = ref("");
const DEFAULT_PAGE_SIZE = 1000;
const PAGE_SIZE_OPTIONS = [100, 500, 1000, 2000, 5000];
const TABLE_ROW_HEIGHT = 34;
const TABLE_OVERSCAN = 8;
const TABLE_COLUMN_MIN_WIDTH = 58;
const TABLE_COLUMN_MAX_WIDTH = 640;
let tableResizeObserver = null;
let activeViewStateTabId = null;
let restoringViewState = false;

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

    await nextTick();
    applyStoredScrollState();
  },
  { immediate: true },
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

const tableColumns = computed(() => {
  const tabId = props.activeTopTab?.id;
  const widths = tabId ? tableColumnWidths.value[tabId] ?? {} : {};
  const columns = activeResult.value?.columns ?? [];
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

function quoteIdentifier(value) {
  return `\`${String(value).replaceAll("`", "``")}\``;
}

function quoteString(value) {
  return `'${String(value).replaceAll("\\", "\\\\").replaceAll("'", "''")}'`;
}

function schemaTableCountKey(schema, table) {
  return `${normalizedConnection.value.id}:${schema}.${table}`;
}

function schemaTableSelectKey(row) {
  return `${row.schema}.${row.name}`;
}

function emptyRowRange() {
  return { start: null, end: null };
}

function emptyCellRange() {
  return { startRow: null, endRow: null, startColumn: null, endColumn: null };
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

function clampRangeIndexes(start, end, maxLength) {
  const from = Math.max(0, Math.min(start, end));
  const to = Math.min(maxLength - 1, Math.max(start, end));
  return { from, to };
}

function normalizePositiveInteger(value, fallback) {
  const number = Number(value);
  return Number.isFinite(number) && number > 0 ? Math.floor(number) : fallback;
}

function columnMinWidth(column) {
  const length = String(column).length;
  return Math.min(320, Math.max(140, length * 10 + 44));
}

function clampColumnWidth(width) {
  return Math.min(TABLE_COLUMN_MAX_WIDTH, Math.max(TABLE_COLUMN_MIN_WIDTH, Math.round(width)));
}

function formatCellValue(value) {
  if (value === null || value === undefined) {
    return "NULL";
  }
  if (typeof value === "object") {
    return JSON.stringify(value);
  }
  return String(value);
}

function formatRowCount(value) {
  const number = Number(value ?? 0);
  return Number.isFinite(number) ? number.toLocaleString("zh-CN") : "0";
}

function formatDataLength(value) {
  const bytes = Number(value ?? 0);
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }

  const units = ["B", "KB", "MB", "GB", "TB"];
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const size = bytes / 1024 ** exponent;
  const formatted = size >= 10 || exponent === 0 ? Math.round(size).toString() : size.toFixed(1);
  return `${formatted} ${units[exponent]}`;
}

function formatTableDate(value) {
  return value ? String(value).replace(".000000", "") : "";
}

function schemaTableCellValue(row, column) {
  const value = row[column.key];
  return column.formatter ? column.formatter(value) : formatCellValue(value);
}

function rowMatchesSearch(row, columns, query, valueGetter) {
  return columns.some((column) => valueGetter(row, column).toLowerCase().includes(query));
}

async function openTableSearch() {
  if (!props.activeTopTab || !["schema", "table"].includes(props.activeTopTab.kind)) {
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

function openCopyContextMenu(event) {
  if (!selectedCopyText()) {
    return;
  }

  copyContextPosition.value = { x: event.clientX, y: event.clientY };
  copyContextOpen.value = true;
}

function handleCopyContextSelect(item) {
  if (item.key === "copy") {
    copySelectedText();
  }
}

function handleKeydown(event) {
  const isFindShortcut = (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f";
  if (isFindShortcut && props.activeTopTab && ["schema", "table"].includes(props.activeTopTab.kind)) {
    event.preventDefault();
    openTableSearch();
    return;
  }

  const isCopyShortcut = (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "c";
  const isPasteShortcut = (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "v";
  if (!isCopyShortcut && !isPasteShortcut) {
    return;
  }

  const text = selectedCopyText();
  if (!text) {
    return;
  }

  event.preventDefault();
  copySelectedText();
}

function copyTextForRows(rows, columns) {
  return rows
    .map((row) => columns.map((column) => (column.formatter ? column.formatter(row[column.key]) : formatCellValue(row[column.key]))).join("\t"))
    .join("\n");
}

function copyTextForCells(rows, columns, range) {
  const rowRange = clampRangeIndexes(range.startRow, range.endRow, rows.length);
  const columnRange = clampRangeIndexes(range.startColumn, range.endColumn, columns.length);
  return rows
    .slice(rowRange.from, rowRange.to + 1)
    .map((row) =>
      columns
        .slice(columnRange.from, columnRange.to + 1)
        .map((column) => (column.formatter ? column.formatter(row[column.key]) : formatCellValue(row[column.key])))
        .join("\t"),
    )
    .join("\n");
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

  return "";
}

async function copySelectedText() {
  const text = selectedCopyText();
  if (!text) {
    return;
  }

  try {
    await navigator.clipboard.writeText(text);
    ElMessage.success("已复制");
  } catch (error) {
    ElMessage.error(`复制失败：${error}`);
  }
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

  loading.value = true;
  const tableName = `${quoteIdentifier(schema)}.${quoteIdentifier(table)}`;
  const sql = `SELECT *\nFROM ${tableName}\nLIMIT ${pageSize} OFFSET ${offset};`;
  const countSql = `SELECT COUNT(*) AS total\nFROM ${tableName};`;
  try {
    const [result, countResult] = await Promise.all([
      executeMysqlQuery(currentConfig(), schema, sql),
      executeMysqlQuery(currentConfig(), schema, countSql),
    ]);
    const totalRows = Number(countResult.rows?.[0]?.total ?? 0);
    tableResults.value = {
      ...tableResults.value,
      [tabId]: {
        ...result,
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
    resetTableScroll();
    saveTabViewState(tabId);
  } catch (error) {
    ElMessage.error(`加载表数据失败：${error}`);
  } finally {
    loading.value = false;
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

      <section v-else-if="activeTopTab.kind === 'table'" class="tab-content has-footer">
        <div v-if="tableSearchOpen" class="table-search">
          <el-icon><Search /></el-icon>
          <input
            ref="searchInputRef"
            v-model="tableSearchQuery"
            type="search"
            placeholder="搜索当前页数据"
            @keydown="handleSearchKeydown"
          />
          <span>{{ searchedResultRows.length }} / {{ activeResult?.rows.length ?? 0 }}</span>
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
                >
              <div
                v-for="(column, columnIndex) in tableColumns"
                :key="column.key"
                class="virtual-table__cell"
                :class="{
                  selected: isResultCellSelected(visibleIndex, columnIndex),
                  matched: hasTableSearch && tableCellValue(row, column).toLowerCase().includes(normalizedTableSearch),
                }"
                :title="tableCellValue(row, column)"
                @mousedown.prevent="startResultCellSelection($event, visibleIndex, columnIndex)"
                @mouseenter="extendResultCellSelection(visibleIndex, columnIndex)"
              >
                {{ tableCellValue(row, column) }}
              </div>
                </div>
                <div class="virtual-table__spacer" :style="{ height: `${visibleTableRows.bottom}px` }" />
              </div>
            </div>
          </div>
        </div>
        <footer class="table-footer">
          <span v-if="activeResult" class="table-footer__summary">
            第 {{ activeResult.page }} 页 · 显示 {{ searchedResultRows.length }} / {{ activeResult.totalRows }} 行 · {{ activeResult.elapsedMs }}ms
          </span>
          <el-pagination
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
      :items="[{ key: 'copy', label: '复制', disabled: !selectedCopyText() }]"
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
  flex: 0 0 44px;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-height: 44px;
  padding: 0 12px;
  border-top: 1px solid var(--line);
  background: var(--surface-muted);
  font-size: 12px;
  font-weight: 400;
}

.table-footer__summary {
  flex: 0 0 auto;
  color: var(--muted);
  font-size: 12px;
  font-weight: 400;
  line-height: 28px;
}

.table-footer :deep(.el-pagination) {
  --el-pagination-bg-color: #fff;
  --el-pagination-button-color: var(--muted);
  --el-pagination-button-disabled-bg-color: var(--surface-muted);
  --el-pagination-button-disabled-color: var(--faint);
  --el-pagination-hover-color: var(--orange);
  --el-pagination-font-size: 12px;
  --el-pagination-button-width: 28px;
  --el-pagination-button-height: 28px;
  margin-left: auto;
  color: var(--muted);
  font-size: 12px;
  font-weight: 400;
  line-height: 28px;
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
  line-height: 28px;
}

.table-footer :deep(.el-pagination.is-background .btn-next),
.table-footer :deep(.el-pagination.is-background .btn-prev),
.table-footer :deep(.el-pagination.is-background .el-pager li) {
  min-width: 28px;
  height: 28px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: #fff;
  color: var(--muted);
  font-size: 12px;
  font-weight: 400;
  line-height: 26px;
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
  height: 28px;
  min-height: 28px;
  border-radius: 7px;
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
