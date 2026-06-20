export function createMysqlConnection(overrides = {}) {
  const now = Date.now();
  const config = {
    host: "127.0.0.1",
    port: 3306,
    username: "root",
    password: "",
    database: "",
    ...(overrides.config ?? {}),
  };

  return {
    id: `mysql-${now}`,
    workspace: "database",
    name: "new-mysql",
    meta: formatMysqlMeta(config),
    iconClass: "mysql",
    iconText: "M",
    config,
    status: "disconnected",
    schemas: [],
    ...overrides,
    config,
    meta: overrides.meta ?? formatMysqlMeta(config),
    status: overrides.status ?? "disconnected",
  };
}

export function formatMysqlMeta(config) {
  return `MySQL · ${config?.host ?? "127.0.0.1"}:${config?.port ?? 3306}`;
}

export function normalizeDatabaseConnection(connection, index = 0) {
  if (connection?.workspace !== "database") {
    return connection;
  }

  return createMysqlConnection({
    ...connection,
    id: connection.id ?? `mysql-${Date.now()}-${index}`,
    name: connection.name ?? "mysql",
    schemas: Array.isArray(connection.schemas) ? connection.schemas : [],
  });
}

export function ensureMysqlConnection(connection) {
  return normalizeDatabaseConnection(connection ?? createMysqlConnection());
}
