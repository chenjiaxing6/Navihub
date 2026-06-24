import { quoteIdentifier as quoteMysqlIdentifier, quoteString } from "./databaseQueryUtils";

export function databaseEngine(connection) {
  return connection?.config?.engine ?? connection?.engine ?? "mysql";
}

export function isSqliteEngine(connection) {
  return databaseEngine(connection) === "sqlite";
}

export function quoteSqliteIdentifier(value) {
  return `"${String(value).replaceAll("\"", "\"\"")}"`;
}

export function quoteDatabaseIdentifier(connection, value) {
  return isSqliteEngine(connection) ? quoteSqliteIdentifier(value) : quoteMysqlIdentifier(value);
}

export function qualifiedTableName(connection, schema, table) {
  const quote = (value) => quoteDatabaseIdentifier(connection, value);
  return isSqliteEngine(connection) ? quote(table) : `${quote(schema)}.${quote(table)}`;
}

export function sqlValueLiteral(value) {
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

export function nullSafeEquals(connection, column, value) {
  const quotedColumn = quoteDatabaseIdentifier(connection, column);
  const literal = sqlValueLiteral(value);
  return isSqliteEngine(connection)
    ? `${quotedColumn} IS ${literal}`
    : `${quotedColumn} <=> ${literal}`;
}

export function defaultSchemaName(connection) {
  return isSqliteEngine(connection) ? "main" : connection?.config?.database ?? "";
}
