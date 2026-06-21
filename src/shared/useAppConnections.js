import { computed, ref } from "vue";
import { connections } from "../features/connections/connectionData";
import { normalizeDatabaseConnection } from "../features/database/databaseDefaults";
import { defaultTerminalThemeId, getTerminalTheme, terminalThemes } from "../features/settings/terminalThemes";
import { normalizeSshConnection } from "../features/terminal/sshDefaults";
import { usePersistentState } from "./usePersistentState";

const storageKey = "myhub.connections.v1";
const folderStorageKey = "myhub.connectionFolders.v1";
const settingsStorageKey = "myhub.settings.v1";

function normalizeConnection(connection, index = 0) {
  if (connection?.workspace === "ssh") {
    return normalizeSshConnection(connection, index);
  }

  return normalizeDatabaseConnection(connection, index);
}

function normalizeConnections(value) {
  const source = Array.isArray(value) ? value : connections;
  return source.map(normalizeConnection);
}

function normalizeConnectionFolders(folders) {
  if (!Array.isArray(folders)) {
    return [];
  }

  return folders
    .filter((folder) => folder && typeof folder === "object")
    .map((folder, index) => ({
      id: folder.id ?? `folder-${Date.now()}-${index}`,
      workspace: folder.workspace === "ssh" ? "ssh" : "database",
      name: String(folder.name ?? "新建文件夹").trim() || "新建文件夹",
    }));
}

function normalizeSettings(settings) {
  const terminalThemeId = terminalThemes.some((theme) => theme.id === settings?.terminalThemeId)
    ? settings.terminalThemeId
    : defaultTerminalThemeId;
  return { terminalThemeId };
}

export function useAppConnections() {
  const activeWorkspace = ref("database");
  const connectionList = usePersistentState(storageKey, connections, normalizeConnections);
  const connectionFolders = usePersistentState(folderStorageKey, [], normalizeConnectionFolders);
  const appSettings = usePersistentState(settingsStorageKey, {}, normalizeSettings);
  const activeConnectionIds = ref({
    database: connectionList.value.find((connection) => connection.workspace === "database")?.id ?? "",
    ssh: connectionList.value.find((connection) => connection.workspace === "ssh")?.id ?? "",
  });

  const visibleConnections = computed(() =>
    connectionList.value.filter((connection) => connection.workspace === activeWorkspace.value),
  );
  const visibleConnectionFolders = computed(() =>
    connectionFolders.value.filter((folder) => folder.workspace === activeWorkspace.value),
  );
  const activeConnectionId = computed({
    get() {
      return activeConnectionIds.value[activeWorkspace.value] ?? "";
    },
    set(connectionId) {
      setWorkspaceActiveConnectionId(activeWorkspace.value, connectionId);
    },
  });
  const activeConnection = computed(() =>
    visibleConnections.value.find((connection) => connection.id === activeConnectionId.value) ??
      visibleConnections.value[0] ??
      null,
  );
  const databaseConnection = computed(() => {
    const databaseConnections = connectionList.value.filter((connection) => connection.workspace === "database");
    return databaseConnections.find((connection) => connection.id === activeConnectionIds.value.database) ??
      databaseConnections[0] ??
      null;
  });
  const sshConnection = computed(() => {
    const sshConnections = connectionList.value.filter((connection) => connection.workspace === "ssh");
    return sshConnections.find((connection) => connection.id === activeConnectionIds.value.ssh) ??
      sshConnections[0] ??
      null;
  });
  const terminalTheme = computed(() => getTerminalTheme(appSettings.value.terminalThemeId));

  function setWorkspaceActiveConnectionId(workspace, connectionId) {
    activeConnectionIds.value = {
      ...activeConnectionIds.value,
      [workspace]: connectionId,
    };
  }

  return {
    activeConnection,
    activeConnectionId,
    activeConnectionIds,
    activeWorkspace,
    appSettings,
    connectionFolders,
    connectionList,
    databaseConnection,
    setWorkspaceActiveConnectionId,
    sshConnection,
    terminalTheme,
    visibleConnectionFolders,
    visibleConnections,
  };
}
