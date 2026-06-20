export function createSshConnection(overrides = {}) {
  const now = Date.now();
  const config = {
    host: "127.0.0.1",
    port: 22,
    username: "root",
    password: "",
    privateKey: "",
    remotePath: "",
    ...(overrides.config ?? {}),
  };

  return {
    id: `ssh-${now}`,
    workspace: "ssh",
    name: "new-host",
    meta: formatSshMeta(config),
    iconClass: "ssh",
    iconText: "S",
    config,
    status: "disconnected",
    connectVersion: 0,
    ...overrides,
    config,
    meta: overrides.meta ?? formatSshMeta(config),
    status: overrides.status ?? "disconnected",
    connectVersion: overrides.connectVersion ?? 0,
  };
}

export function formatSshMeta(config) {
  return `SSH/SFTP · ${config?.username ?? "root"}@${config?.host ?? "127.0.0.1"}:${config?.port ?? 22}`;
}

export function normalizeSshConnection(connection, index = 0) {
  if (connection?.workspace !== "ssh") {
    return connection;
  }

  const normalized = createSshConnection({
    ...connection,
    id: connection.id ?? `ssh-${Date.now()}-${index}`,
    name: connection.name ?? "host",
  });

  return {
    ...normalized,
    iconClass: connection.iconClass ?? "ssh",
    iconText: connection.iconText ?? "S",
  };
}
