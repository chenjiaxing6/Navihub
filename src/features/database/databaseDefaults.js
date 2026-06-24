export function createMysqlConnection(overrides = {}) {
  const now = Date.now();
  const id = overrides.id ?? `mysql-${now}`;
  const name = overrides.name ?? "new-mysql";
  const config = {
    engine: "mysql",
    host: "127.0.0.1",
    port: 3306,
    username: "root",
    password: "",
    database: "",
    ...(overrides.config ?? {}),
  };

  return {
    id,
    workspace: "database",
    name,
    meta: formatMysqlMeta(config),
    iconClass: "mysql",
    iconText: "M",
    config,
    status: "disconnected",
    schemas: [],
    pinnedSchemas: [],
    savedQueries: [],
    ...overrides,
    id,
    workspace: "database",
    name,
    config,
    meta: overrides.meta ?? formatMysqlMeta(config),
    status: overrides.status ?? "disconnected",
    schemas: Array.isArray(overrides.schemas) ? overrides.schemas : [],
    pinnedSchemas: normalizePinnedSchemas(overrides.pinnedSchemas),
    savedQueries: normalizeSavedQueries(overrides.savedQueries),
  };
}

export function createSqliteConnection(overrides = {}) {
  const now = Date.now();
  const id = overrides.id ?? `sqlite-${now}`;
  const name = overrides.name ?? "new-sqlite";
  const config = {
    engine: "sqlite",
    path: "",
    readOnly: false,
    ...(overrides.config ?? {}),
  };

  return {
    id,
    workspace: "database",
    name,
    meta: overrides.meta ?? formatSqliteMeta(config),
    iconClass: "sqlite",
    iconText: "S",
    config,
    status: overrides.status ?? "disconnected",
    schemas: Array.isArray(overrides.schemas) ? overrides.schemas : [],
    pinnedSchemas: normalizePinnedSchemas(overrides.pinnedSchemas),
    savedQueries: normalizeSavedQueries(overrides.savedQueries),
    ...overrides,
    id,
    workspace: "database",
    name,
    config,
    meta: overrides.meta ?? formatSqliteMeta(config),
    status: overrides.status ?? "disconnected",
    schemas: Array.isArray(overrides.schemas) ? overrides.schemas : [],
    pinnedSchemas: normalizePinnedSchemas(overrides.pinnedSchemas),
    savedQueries: normalizeSavedQueries(overrides.savedQueries),
  };
}

export function formatMysqlMeta(config) {
  return `MySQL · ${config?.host ?? "127.0.0.1"}:${config?.port ?? 3306}`;
}

export function formatSqliteMeta(config) {
  const path = String(config?.path ?? "").trim();
  return `SQLite · ${path || "未选择文件"}`;
}

export function formatDatabaseMeta(config) {
  return config?.engine === "sqlite" ? formatSqliteMeta(config) : formatMysqlMeta(config);
}

export function normalizeDatabaseConnection(connection, index = 0) {
  if (connection?.workspace !== "database") {
    return connection;
  }

  if (connection.config?.engine === "sqlite" || connection.iconClass === "sqlite") {
    return createSqliteConnection({
      ...connection,
      id: connection.id ?? `sqlite-${Date.now()}-${index}`,
      name: connection.name ?? "sqlite",
      schemas: Array.isArray(connection.schemas) ? connection.schemas : [],
      pinnedSchemas: normalizePinnedSchemas(connection.pinnedSchemas),
      savedQueries: normalizeSavedQueries(connection.savedQueries),
    });
  }

  return createMysqlConnection({
    ...connection,
    id: connection.id ?? `mysql-${Date.now()}-${index}`,
    name: connection.name ?? "mysql",
    config: { engine: "mysql", ...(connection.config ?? {}) },
    schemas: Array.isArray(connection.schemas) ? connection.schemas : [],
    pinnedSchemas: normalizePinnedSchemas(connection.pinnedSchemas),
    savedQueries: normalizeSavedQueries(connection.savedQueries),
  });
}

export function ensureMysqlConnection(connection) {
  return normalizeDatabaseConnection(connection ?? createMysqlConnection());
}

export function normalizeSavedQueries(queries) {
  if (!Array.isArray(queries)) {
    return [];
  }

  return queries
    .filter((query) => query && typeof query === "object")
    .map((query, index) => {
      const now = Date.now();
      return {
        id: String(query.id ?? `query-${now}-${index}`),
        schema: String(query.schema ?? "").trim(),
        name: String(query.name ?? "未命名查询").trim() || "未命名查询",
        sql: String(query.sql ?? ""),
        createdAt: Number(query.createdAt ?? now),
        updatedAt: Number(query.updatedAt ?? query.createdAt ?? now),
      };
    })
    .filter((query) => query.schema);
}

function normalizePinnedSchemas(schemas) {
  if (!Array.isArray(schemas)) {
    return [];
  }

  return [...new Set(
    schemas
      .map((schema) => String(schema ?? "").trim())
      .filter(Boolean),
  )];
}
