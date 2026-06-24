export const DEFAULT_PAGE_SIZE = 1000;
export const PAGE_SIZE_OPTIONS = [100, 500, 1000, 2000, 5000];
export const TABLE_ROW_HEIGHT = 34;
export const TABLE_OVERSCAN = 8;
export const TABLE_COLUMN_MIN_WIDTH = 58;
export const TABLE_COLUMN_MAX_WIDTH = 640;

export function schemaTableSelectKey(row) {
  return `${row.schema}.${row.name}`;
}

export function emptyRowRange() {
  return { start: null, end: null };
}

export function emptyCellRange() {
  return { startRow: null, endRow: null, startColumn: null, endColumn: null };
}

export function clampRangeIndexes(start, end, maxLength) {
  const from = Math.max(0, Math.min(start, end));
  const to = Math.min(maxLength - 1, Math.max(start, end));
  return { from, to };
}

export function normalizePositiveInteger(value, fallback) {
  const number = Number(value);
  return Number.isFinite(number) && number > 0 ? Math.floor(number) : fallback;
}

export function columnMinWidth(column) {
  const length = String(column).length;
  return Math.min(320, Math.max(140, length * 10 + 44));
}

export function clampColumnWidth(width) {
  return Math.min(TABLE_COLUMN_MAX_WIDTH, Math.max(TABLE_COLUMN_MIN_WIDTH, Math.round(width)));
}

export function formatCellValue(value) {
  if (value === null || value === undefined) {
    return "NULL";
  }
  if (typeof value === "object") {
    return JSON.stringify(value);
  }
  return String(value);
}

export function formatRowCount(value) {
  const number = Number(value ?? 0);
  return Number.isFinite(number) ? number.toLocaleString("zh-CN") : "0";
}

export function formatDataLength(value) {
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

export function formatTableDate(value) {
  return value ? String(value).replace(".000000", "") : "";
}

export function formatBooleanText(value) {
  return value ? "是" : "否";
}

export function schemaTableCellValue(row, column) {
  const value = row[column.key];
  return column.formatter ? column.formatter(value) : formatCellValue(value);
}

export function rowMatchesSearch(row, columns, query, valueGetter) {
  return columns.some((column) => valueGetter(row, column).toLowerCase().includes(query));
}

export function copyTextForRows(rows, columns) {
  return rows
    .map((row) =>
      columns
        .map((column) => (column.formatter ? column.formatter(row[column.key]) : formatCellValue(row[column.key])))
        .join("\t"),
    )
    .join("\n");
}

export function copyTextForCells(rows, columns, range) {
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
