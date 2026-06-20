<script setup>
import { computed, defineAsyncComponent, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { Close, Connection, FolderOpened, Monitor as MonitorIcon, Right } from "@element-plus/icons-vue";
import { save } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";
import TerminalSessionPane from "./TerminalSessionPane.vue";
import SftpFilePane from "./SftpFilePane.vue";
import MonitorWorkspace from "../monitor/MonitorWorkspace.vue";
import ContextMenu from "../../shared/ContextMenu.vue";
import {
  connectSshTerminal,
  connectSftpSession,
  cancelSftpDownload,
  createSftpDir,
  deleteSftpPath,
  disconnectSftpSession,
  disconnectSshTerminal,
  downloadSftpFile,
  finishSftpDownload,
  finishSftpUpload,
  getSftpRealpath,
  listSftpDir,
  onSshTerminalEvent,
  readSftpDownload,
  renameSftpPath,
  resizeSshTerminal,
  startSftpDownload,
  startSftpUpload,
  uploadSftpFile,
  writeSftpUpload,
  writeSshTerminal,
} from "./sshApi";

const props = defineProps({
  connection: { type: Object, default: null },
  terminalTheme: { type: Object, required: true },
});

const emit = defineEmits(["connection-state"]);
const SftpEditorDialog = defineAsyncComponent(() => import("./SftpEditorDialog.vue"));

const tabs = ref([]);
const activeTabId = ref(null);
const tabsScroll = ref(null);
const tabsViewportWidth = ref(0);
const tabContextOpen = ref(false);
const tabContextPosition = ref({ x: 0, y: 0 });
const contextTabId = ref(null);
const editorDialog = ref({
  visible: false,
  loading: false,
  saving: false,
  tabId: "",
  fileName: "",
  path: "",
  content: "",
});
let unlistenTerminal = null;
let tabsResizeObserver = null;
const pendingSftpConnectTimers = new Set();

const activeTab = computed(() => tabs.value.find((tab) => tab.id === activeTabId.value) ?? null);
const hasTerminalTab = computed(() => tabs.value.some((tab) => tab.kind === "terminal"));
const activeTerminalMonitorVisible = computed(() => activeTab.value?.kind === "terminal" && activeTab.value.monitorVisible);
const selectedSshTarget = computed(() => {
  if (!props.connection?.config) {
    return "未选择主机";
  }

  const { host = "127.0.0.1", port = 22, username = "root" } = props.connection.config;
  return `${username}@${host}:${port}`;
});
const visibleTabLimit = computed(() => {
  const tabsPerRow = Math.max(1, Math.floor(tabsViewportWidth.value / 132));
  const totalSlots = tabsPerRow * 2;
  return tabs.value.length > totalSlots ? Math.max(totalSlots - 1, 1) : totalSlots;
});
const visibleTabs = computed(() => {
  const limit = visibleTabLimit.value;
  if (tabs.value.length <= limit) {
    return tabs.value;
  }

  const activeIndex = tabs.value.findIndex((tab) => tab.id === activeTabId.value);
  if (activeIndex < limit) {
    return tabs.value.slice(0, limit);
  }

  return [...tabs.value.slice(0, Math.max(limit - 1, 0)), tabs.value[activeIndex]];
});
const overflowTabs = computed(() => {
  const visibleIds = new Set(visibleTabs.value.map((tab) => tab.id));
  return tabs.value.filter((tab) => !visibleIds.has(tab.id));
});
const contextTabIndex = computed(() => tabs.value.findIndex((tab) => tab.id === contextTabId.value));
const tabContextItems = computed(() => [
  { key: "all", label: "关闭所有", disabled: tabs.value.length === 0 },
  { key: "left", label: "关闭左侧", disabled: contextTabIndex.value <= 0 },
  {
    key: "right",
    label: "关闭右侧",
    disabled: contextTabIndex.value < 0 || contextTabIndex.value >= tabs.value.length - 1,
  },
]);

function currentTab(tab) {
  return tabs.value.find((item) => item.id === tab?.id) ?? tab;
}

function patchTab(tab, patch) {
  let nextTab = null;
  tabs.value = tabs.value.map((item) => {
    if (item.id !== tab?.id) {
      return item;
    }

    const nextPatch = typeof patch === "function" ? patch(item) : patch;
    nextTab = { ...item, ...nextPatch };
    return nextTab;
  });
  return nextTab ?? tab;
}

async function setupTabsResizeObserver() {
  await nextTick();
  if (!tabsScroll.value || tabsResizeObserver) {
    return;
  }

  const updateWidth = () => {
    tabsViewportWidth.value = tabsScroll.value?.clientWidth ?? 0;
  };
  updateWidth();
  tabsResizeObserver = new ResizeObserver(updateWidth);
  tabsResizeObserver.observe(tabsScroll.value);
}

watch(
  () => [props.connection?.id, props.connection?.connectVersion],
  (next, previous) => {
    const [nextId, nextVersion] = next;
    const [previousId, previousVersion] = previous ?? [];
    if (!nextId || nextVersion == null || nextId !== previousId || nextVersion === previousVersion || !props.connection) {
      return;
    }

    openTerminalTab(props.connection);
  },
);

onMounted(() => {
  setupTabsResizeObserver();
});

onBeforeUnmount(async () => {
  if (unlistenTerminal) {
    unlistenTerminal();
  }
  tabsResizeObserver?.disconnect();
  for (const timer of pendingSftpConnectTimers) {
    clearTimeout(timer);
  }
  pendingSftpConnectTimers.clear();

  await Promise.all(
    tabs.value
      .filter((tab) => tab.kind === "terminal")
      .map((tab) => disconnectSshTerminal(tab.sessionId).catch(() => {})),
  );
  await Promise.all(
    tabs.value
      .filter((tab) => tab.kind === "sftp")
      .map((tab) => disconnectSftpSession(tab.sessionId).catch(() => {})),
  );
});

async function ensureTerminalListener() {
  if (unlistenTerminal) {
    return;
  }

  unlistenTerminal = await onSshTerminalEvent((event) => {
    const tab = tabs.value.find((item) => item.kind === "terminal" && item.sessionId === event.sessionId);
    if (!tab) {
      return;
    }

    if (event.kind === "data") {
      tab.output += event.data;
    } else if (event.kind === "connected") {
      tab.state = "connected";
      emit("connection-state", { connection: tab.connection, status: "connected" });
      resizeTerminal(tab);
    } else if (event.kind === "disconnected") {
      tab.state = "disconnected";
      emit("connection-state", { connection: tab.connection, status: "disconnected" });
    } else if (event.kind === "error") {
      if (isSshTransportDisconnect(event.data)) {
        tab.state = "disconnected";
        tab.error = "";
        emit("connection-state", { connection: tab.connection, status: "disconnected" });
        return;
      }

      tab.state = "error";
      tab.error = event.data;
      emit("connection-state", { connection: tab.connection, status: "disconnected" });
      ElMessage.error(`SSH 连接失败：${event.data}`);
    }
  });
}

async function openTerminalTab(connection) {
  const tab = {
    id: `terminal-${Date.now()}-${Math.random().toString(16).slice(2)}`,
    kind: "terminal",
    label: connection.name,
    connection: { ...connection, config: { ...connection.config } },
    sessionId: `ssh-${connection.id}-${Date.now()}-${Math.random().toString(16).slice(2)}`,
    output: "",
    state: "connecting",
    error: "",
    size: { cols: 120, rows: 32 },
    pendingInput: "",
    flushingInput: false,
    monitorVisible: false,
  };

  tabs.value = [...tabs.value, tab];
  activeTabId.value = tab.id;

  try {
    await ensureTerminalListener();
    await connectSshTerminal(tab.sessionId, tab.connection.config);
  } catch (error) {
    tab.state = "error";
    tab.error = String(error);
    emit("connection-state", { connection: tab.connection, status: "disconnected" });
    ElMessage.error(`SSH 连接失败：${error}`);
  }
}

function openSftpTab() {
  if (!props.connection) {
    ElMessage.info("请先选择一个 SSH 主机");
    return;
  }

  const existing = tabs.value.find((tab) => tab.kind === "sftp" && tab.connection.id === props.connection.id);
  if (existing) {
    activeTabId.value = existing.id;
    return;
  }

  const tab = {
    id: `sftp-${props.connection.id}-${Date.now()}`,
    kind: "sftp",
    label: `${props.connection.name} · 文件`,
    connection: { ...props.connection, config: { ...props.connection.config } },
    sessionId: `sftp-${props.connection.id}-${Date.now()}-${Math.random().toString(16).slice(2)}`,
    path: initialSftpPath(props.connection),
    files: [],
    tasks: [],
    clipboardFile: null,
    busyCount: 0,
    loading: true,
    connecting: true,
    error: "",
  };

  tabs.value = [...tabs.value, tab];
  activeTabId.value = tab.id;
  scheduleSftpConnect(tab);
}

function scheduleSftpConnect(tab) {
  const timer = setTimeout(() => {
    pendingSftpConnectTimers.delete(timer);
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        if (tabs.value.some((item) => item.id === tab.id)) {
          connectSftpTab(tab);
        }
      });
    });
  }, 0);
  pendingSftpConnectTimers.add(timer);
}

async function closeTab(tabId) {
  const tab = tabs.value.find((item) => item.id === tabId);
  if (!tab) {
    return;
  }

  if (tab.kind === "terminal") {
    await disconnectSshTerminal(tab.sessionId).catch(() => {});
  } else if (tab.kind === "sftp") {
    await disconnectSftpSession(tab.sessionId).catch(() => {});
  }

  tabs.value = tabs.value.filter((item) => item.id !== tabId);
  if (activeTabId.value === tabId) {
    activeTabId.value = tabs.value[tabs.value.length - 1]?.id ?? null;
  }
  syncConnectionStateAfterTabClose(tab.connection);
}

async function closeTabsByScope(scope) {
  const index = tabs.value.findIndex((tab) => tab.id === contextTabId.value);
  if (index < 0) {
    return;
  }

  const tabsByScope = {
    all: tabs.value,
    left: tabs.value.slice(0, index),
    right: tabs.value.slice(index + 1),
  };

  for (const tab of tabsByScope[scope] ?? []) {
    await closeTab(tab.id);
  }
}

function openTabContextMenu(event, tab) {
  contextTabId.value = tab.id;
  tabContextPosition.value = { x: event.clientX, y: event.clientY };
  tabContextOpen.value = true;
}

function handleTabContextSelect(item) {
  closeTabsByScope(item.key);
}

function syncConnectionStateAfterTabClose(connection) {
  if (!connection) {
    return;
  }

  const hasOpenSession = tabs.value.some((item) => item.connection?.id === connection.id);
  if (!hasOpenSession) {
    emit("connection-state", { connection, status: "disconnected" });
  }
}

async function connectSftpTab(tab) {
  patchTab(tab, { connecting: true, loading: true, error: "" });
  await waitForLoadingPaint();

  try {
    await connectSftpSession(tab.sessionId, tab.connection.config);
    patchTab(tab, { connecting: false });
    const initialPath = await resolveInitialSftpPath(tab);
    await loadRemoteFiles(tab, initialPath);
  } catch (error) {
    patchTab(tab, { error: String(error) });
    ElMessage.error(`SFTP 连接失败：${error}`);
  } finally {
    patchTab(tab, { connecting: false, loading: false });
  }
}

async function handleTerminalResize(tab, size) {
  tab.size = {
    cols: Math.max(1, Math.floor(size.cols || 120)),
    rows: Math.max(1, Math.floor(size.rows || 32)),
  };

  if (tab.state === "connected") {
    await resizeTerminal(tab);
  }
}

async function resizeTerminal(tab) {
  try {
    await resizeSshTerminal(tab.sessionId, tab.size.cols, tab.size.rows);
  } catch {
    // The session may be closing while the layout is still resizing.
  }
}

function handleTerminalInput(tab, data) {
  if (tab.state !== "connected") {
    return;
  }

  tab.pendingInput += data;
  flushTerminalInput(tab);
}

async function flushTerminalInput(tab) {
  if (tab.flushingInput) {
    return;
  }

  tab.flushingInput = true;
  try {
    while (tab.pendingInput && tab.state === "connected") {
      const chunk = tab.pendingInput;
      tab.pendingInput = "";

      try {
        await writeSshTerminal(tab.sessionId, chunk);
      } catch (error) {
        tab.pendingInput = "";
        if (isSshTransportDisconnect(error)) {
          tab.state = "disconnected";
          tab.error = "";
          emit("connection-state", { connection: tab.connection, status: "disconnected" });
          return;
        }

        tab.state = "error";
        tab.error = String(error);
        ElMessage.error(`终端写入失败：${error}`);
        return;
      }
    }
  } finally {
    tab.flushingInput = false;
    if (tab.pendingInput) {
      flushTerminalInput(tab);
    }
  }
}

async function loadRemoteFiles(tab, path) {
  if (!tab?.connection?.config) {
    return;
  }

  patchTab(tab, { path, loading: true, error: "" });
  await waitForLoadingPaint();

  try {
    const entries = await listSftpDir(tab.connection.config, path, tab.sessionId);
    patchTab(tab, {
      path,
      files: entries.map((entry) => ({
        ...entry,
        displaySize: entry.folder ? "-" : formatFileSize(entry.size),
      })),
    });
  } catch (error) {
    patchTab(tab, { error: String(error) });
    ElMessage.error(`SFTP 加载失败：${error}`);
  } finally {
    patchTab(tab, { loading: false });
  }
}

async function handleSftpUpload(tab, file) {
  if (!file) {
    return;
  }

  const task = createTransferTask("upload", file.name, file.size);
  patchTab(tab, (current) => ({ tasks: [task, ...current.tasks] }));
  updateTransferTask(tab, task.id, { status: "preparing", progress: 8, cancelable: true });
  beginSftpBusy(tab);
  try {
    await uploadSftpFileInChunks(tab, file, file.name, task.id);
    updateTransferTask(tab, task.id, { status: "done", transferred: file.size, progress: 100, finishedAt: Date.now(), cancelable: false });
    ElMessage.success("上传完成");
    await loadRemoteFiles(tab, tab.path);
  } catch (error) {
    if (isTransferCanceled(error)) {
      updateTransferTask(tab, task.id, { status: "canceled", error: "已取消", finishedAt: Date.now(), cancelable: false });
      return;
    }
    updateTransferTask(tab, task.id, { status: "failed", error: String(error), finishedAt: Date.now(), cancelable: false });
    ElMessage.error(`上传失败：${error}`);
  } finally {
    endSftpBusy(tab);
  }
}

async function uploadSftpFileInChunks(tab, file, fileName, taskId) {
  const chunkSize = 128 * 1024;
  const uploadId = `upload-${tab.sessionId}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  let uploaded = 0;

  await startSftpUpload(tab.connection.config, tab.path, fileName, uploadId, tab.sessionId);
  try {
    updateTransferTask(tab, taskId, { status: "running", transferred: 0, progress: 0, cancelable: true, uploadId });
    while (uploaded < file.size) {
      if (isTaskCanceled(tab, taskId)) {
        throw new Error("TRANSFER_CANCELED");
      }
      const chunk = file.slice(uploaded, Math.min(uploaded + chunkSize, file.size));
      const buffer = await chunk.arrayBuffer();
      await writeSftpUpload(uploadId, Array.from(new Uint8Array(buffer)));
      if (isTaskCanceled(tab, taskId)) {
        throw new Error("TRANSFER_CANCELED");
      }
      uploaded += buffer.byteLength;
      updateTransferTask(tab, taskId, {
        transferred: uploaded,
        progress: file.size ? (uploaded / file.size) * 100 : 100,
      });
      await nextFrame();
    }
    await finishSftpUpload(uploadId);
  } catch (error) {
    await finishSftpUpload(uploadId).catch(() => {});
    throw error;
  }
}

async function handleSftpDownload(tab, payload) {
  const file = payload?.file ?? payload;
  const choosePath = Boolean(payload?.choosePath);
  if (!file || file.folder) {
    return;
  }

  let targetPath = "";
  if (choosePath) {
    targetPath = await chooseDownloadPath(file.name);
    if (!targetPath) {
      return;
    }
  }

  const task = createTransferTask("download", file.name, file.size);
  patchTab(tab, (current) => ({ tasks: [task, ...current.tasks] }));
  updateTransferTask(tab, task.id, { status: "preparing", progress: 0, cancelable: true });
  beginSftpBusy(tab);
  try {
    const data = await downloadSftpFileInChunks(tab, file, task.id);
    if (targetPath) {
      await saveDownloadedDataToPath(targetPath, data);
      ElMessage.success("下载完成");
    } else {
      saveDownloadedData(file.name, data);
    }
    updateTransferTask(tab, task.id, { status: "done", transferred: data.size, progress: 100, finishedAt: Date.now(), cancelable: false });
  } catch (error) {
    if (isTransferCanceled(error)) {
      updateTransferTask(tab, task.id, { status: "canceled", error: "已取消", finishedAt: Date.now(), cancelable: false });
      return;
    }
    updateTransferTask(tab, task.id, { status: "failed", error: String(error), finishedAt: Date.now(), cancelable: false });
    ElMessage.error(`下载失败：${error}`);
  } finally {
    endSftpBusy(tab);
  }
}

async function downloadSftpFileInChunks(tab, file, taskId) {
  const chunkSize = 32 * 1024;
  const downloadId = `download-${tab.sessionId}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  const chunks = [];
  let downloaded = 0;

  await startSftpDownload(tab.connection.config, joinPath(tab.path, file.name), downloadId, tab.sessionId);
  try {
    updateTransferTask(tab, taskId, { status: "running", transferred: 0, progress: 0, cancelable: true, downloadId });
    while (true) {
      if (isTaskCanceled(tab, taskId)) {
        throw new Error("TRANSFER_CANCELED");
      }
      const response = await readSftpDownload(downloadId, chunkSize);
      const chunk = Array.isArray(response) ? response : response.data;
      const done = Array.isArray(response) ? chunk.length === 0 : response.done;
      if (isTaskCanceled(tab, taskId)) {
        throw new Error("TRANSFER_CANCELED");
      }
      if (done) {
        break;
      }
      if (!chunk.length) {
        await nextFrame();
        continue;
      }
      chunks.push(new Uint8Array(chunk));
      downloaded += chunk.length;
      updateTransferTask(tab, taskId, {
        transferred: downloaded,
        progress: file.size ? (downloaded / file.size) * 100 : 0,
      });
      await nextFrame();
    }
    await finishSftpDownload(downloadId);
    return new Blob(chunks);
  } catch (error) {
    if (!isTransferCanceled(error)) {
      await finishSftpDownload(downloadId).catch(() => {});
    }
    throw error;
  }
}

function saveDownloadedData(fileName, blob) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName;
  link.click();
  URL.revokeObjectURL(url);
}

async function chooseDownloadPath(fileName) {
  const path = await save({
    title: `下载至 · ${fileName}`,
    defaultPath: fileName,
    canCreateDirectories: true,
  });
  return typeof path === "string" ? path : "";
}

async function saveDownloadedDataToPath(path, blob) {
  const buffer = await blob.arrayBuffer();
  await writeFile(path, new Uint8Array(buffer));
}

async function handleSftpCopyFile(tab, file) {
  if (!file || file.folder) {
    return;
  }

  patchTab(tab, {
    clipboardFile: {
      name: file.name,
      path: joinPath(tab.path, file.name),
      size: file.size,
    },
  });
  ElMessage.success("已复制文件");
}

async function handleSftpPaste(tab) {
  const source = currentTab(tab).clipboardFile;
  if (!source) {
    return;
  }

  const targetName = nextCopyName(currentTab(tab).files, source.name);
  const task = createTransferTask("upload", targetName, source.size);
  patchTab(tab, (current) => ({ tasks: [task, ...current.tasks] }));
  updateTransferTask(tab, task.id, { status: "running", progress: 14, transferred: Math.round((source.size ?? 0) * 0.14) });

  beginSftpBusy(tab);
  try {
    const data = await downloadSftpFile(tab.connection.config, source.path, tab.sessionId);
    updateTransferTask(tab, task.id, { progress: 55, transferred: Math.round(data.length * 0.55) });
    await uploadSftpFile(tab.connection.config, tab.path, targetName, Array.from(data), tab.sessionId);
    updateTransferTask(tab, task.id, { status: "done", transferred: data.length, progress: 100, finishedAt: Date.now() });
    ElMessage.success("粘贴完成");
    await loadRemoteFiles(tab, tab.path);
  } catch (error) {
    updateTransferTask(tab, task.id, { status: "failed", error: String(error), finishedAt: Date.now() });
    ElMessage.error(`粘贴失败：${error}`);
  } finally {
    endSftpBusy(tab);
  }
}

async function handleSftpCopyPath(tab, file) {
  if (!file) {
    return;
  }

  try {
    await navigator.clipboard.writeText(joinPath(tab.path, file.name));
    ElMessage.success("路径已复制");
  } catch (error) {
    ElMessage.error(`复制路径失败：${error}`);
  }
}

async function handleSftpEdit(tab, file) {
  if (!file || file.folder) {
    return;
  }

  const maxEditSize = 1024 * 1024;
  if ((file.size ?? 0) > maxEditSize) {
    ElMessage.warning("在线编辑仅支持 1MB 以内的文本文件");
    return;
  }

  const remotePath = joinPath(tab.path, file.name);
  editorDialog.value = {
    visible: true,
    loading: true,
    saving: false,
    tabId: tab.id,
    fileName: file.name,
    path: remotePath,
    content: "",
  };

  await waitForLoadingPaint();

  try {
    const data = await downloadSftpFile(tab.connection.config, remotePath, tab.sessionId);
    const text = new TextDecoder().decode(new Uint8Array(data));
    editorDialog.value = {
      ...editorDialog.value,
      loading: false,
      saving: false,
      content: text,
    };
  } catch (error) {
    editorDialog.value = { ...editorDialog.value, visible: false, loading: false };
    ElMessage.error(`在线编辑失败：${error}`);
  }
}

async function saveSftpEditorContent(content) {
  const editor = editorDialog.value;
  const tab = tabs.value.find((item) => item.id === editor.tabId);
  if (!tab || editor.saving) {
    return;
  }

  editorDialog.value = { ...editor, saving: true };
  try {
    const encoded = Array.from(new TextEncoder().encode(content));
    await uploadSftpFile(tab.connection.config, tab.path, editor.fileName, encoded, tab.sessionId);
    editorDialog.value = { ...editorDialog.value, visible: false, saving: false, content };
    ElMessage.success("文件已保存");
    await loadRemoteFiles(tab, tab.path);
  } catch (error) {
    editorDialog.value = { ...editorDialog.value, saving: false };
    ElMessage.error(`保存失败：${error}`);
  }
}

function handleSftpContextAction(tab, payload) {
  const { action, file } = payload;
  if (action === "download" || action === "download-to") {
    handleSftpDownload(tab, { file, choosePath: action === "download-to" });
  } else if (action === "rename") {
    handleSftpRename(tab, file);
  } else if (action === "delete") {
    handleSftpDelete(tab, file);
  } else if (action === "copy-path") {
    handleSftpCopyPath(tab, file);
  } else if (action === "edit") {
    handleSftpEdit(tab, file);
  } else if (action === "copy") {
    handleSftpCopyFile(tab, file);
  }
}

function scrollSshTabs(event) {
  const target = tabsScroll.value;
  if (!target) {
    return;
  }

  const delta = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY;
  if (!delta) {
    return;
  }

  event.preventDefault();
  target.scrollLeft += delta;
}

async function handleSftpDelete(tab, file) {
  if (!file) {
    return;
  }

  try {
    await ElMessageBox.confirm(`确认删除“${file.name}”？`, "删除远程文件", {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning",
      customClass: "bruno-message-box",
    });
    beginSftpBusy(tab);
    await deleteSftpPath(tab.connection.config, joinPath(tab.path, file.name), file.folder, tab.sessionId);
    ElMessage.success("删除完成");
    await loadRemoteFiles(tab, tab.path);
  } catch (error) {
    if (error === "cancel" || error === "close") {
      return;
    }
    ElMessage.error(`删除失败：${error}`);
  } finally {
    endSftpBusy(tab);
  }
}

async function handleSftpCreateDir(tab) {
  try {
    const { value } = await ElMessageBox.prompt("文件夹名称", "新建文件夹", {
      confirmButtonText: "创建",
      cancelButtonText: "取消",
      inputPattern: /^(?!\.{1,2}$)[^/\\]+$/,
      inputErrorMessage: "名称不能包含斜杠，也不能是 . 或 ..",
      customClass: "bruno-message-box",
    });
    beginSftpBusy(tab);
    await createSftpDir(tab.connection.config, tab.path, value.trim(), tab.sessionId);
    ElMessage.success("文件夹已创建");
    await loadRemoteFiles(tab, tab.path);
  } catch (error) {
    if (error === "cancel" || error === "close") {
      return;
    }
    ElMessage.error(`创建失败：${error}`);
  } finally {
    endSftpBusy(tab);
  }
}

async function handleSftpRename(tab, file) {
  if (!file) {
    return;
  }

  try {
    const { value } = await ElMessageBox.prompt("新的名称", "重命名", {
      confirmButtonText: "保存",
      cancelButtonText: "取消",
      inputValue: file.name,
      inputPattern: /^(?!\.{1,2}$)[^/\\]+$/,
      inputErrorMessage: "名称不能包含斜杠，也不能是 . 或 ..",
      customClass: "bruno-message-box",
    });
    beginSftpBusy(tab);
    await renameSftpPath(tab.connection.config, joinPath(tab.path, file.name), value.trim(), tab.sessionId);
    ElMessage.success("重命名完成");
    await loadRemoteFiles(tab, tab.path);
  } catch (error) {
    if (error === "cancel" || error === "close") {
      return;
    }
    ElMessage.error(`重命名失败：${error}`);
  } finally {
    endSftpBusy(tab);
  }
}

function joinPath(base, name) {
  if (!base || base === ".") {
    return name;
  }
  return `${base.replace(/\/+$/, "")}/${name}`.replace(/^\/\//, "/");
}

function initialSftpPath(connection) {
  const path = connection?.config?.remotePath?.trim();
  return !path || path === "/" ? "." : path;
}

async function resolveInitialSftpPath(tab) {
  if (tab.path !== ".") {
    return tab.path;
  }

  try {
    return await getSftpRealpath(tab.connection.config, ".", tab.sessionId);
  } catch {
    return ".";
  }
}

function nextCopyName(files, name) {
  const names = new Set(files.map((file) => file.name));
  if (!names.has(name)) {
    return name;
  }

  const dotIndex = name.lastIndexOf(".");
  const stem = dotIndex > 0 ? name.slice(0, dotIndex) : name;
  const extension = dotIndex > 0 ? name.slice(dotIndex) : "";
  let index = 2;
  let candidate = `${stem} copy${extension}`;
  while (names.has(candidate)) {
    candidate = `${stem} copy ${index}${extension}`;
    index += 1;
  }
  return candidate;
}

function createTransferTask(type, name, size = 0) {
  const now = Date.now();
  const task = {
    id: `${type}-${now}-${Math.random().toString(16).slice(2)}`,
    type,
    name,
    size: size ?? 0,
    transferred: 0,
    progress: 0,
    status: "preparing",
    cancelable: type === "upload" || type === "download",
    canceled: false,
    uploadId: "",
    startedAt: now,
    finishedAt: null,
    error: "",
  };
  decorateTransferTask(task);
  return task;
}

function updateTransferTask(tab, taskId, patch) {
  let nextTask = null;
  patchTab(tab, (current) => {
    const tasks = current.tasks.map((task) => {
      if (task.id !== taskId) {
        return task;
      }
      nextTask = { ...task, ...patch };
      decorateTransferTask(nextTask);
      return nextTask;
    });
    return { tasks };
  });
  return nextTask;
}

function beginSftpBusy(tab) {
  patchTab(tab, (current) => ({ busyCount: (current.busyCount ?? 0) + 1 }));
}

function endSftpBusy(tab) {
  patchTab(tab, (current) => ({ busyCount: Math.max((current.busyCount ?? 0) - 1, 0) }));
}

function decorateTransferTask(task) {
  const now = task.finishedAt ?? Date.now();
  const elapsedSeconds = Math.max((now - task.startedAt) / 1000, 0.1);
  const speed = task.transferred / elapsedSeconds;
  task.progress = Math.max(0, Math.min(100, Math.round(task.progress)));
  task.transferredLabel = formatFileSize(task.transferred);
  task.sizeLabel = task.size ? formatFileSize(task.size) : "-";
  if (task.status === "failed") {
    task.speedLabel = task.error || "传输失败";
  } else if (task.status === "canceled") {
    task.speedLabel = "已取消";
  } else {
    task.speedLabel = `${formatFileSize(speed)}/s`;
  }
}

function nextFrame() {
  return new Promise((resolve) => requestAnimationFrame(resolve));
}

async function waitForLoadingPaint() {
  await nextFrame();
  await nextFrame();
}

function handleCancelTransferTask(tab, task) {
  if (!task || task.status === "done" || task.status === "failed" || task.status === "canceled") {
    return;
  }
  updateTransferTask(tab, task.id, { canceled: true, status: "canceled", error: "已取消", finishedAt: Date.now(), cancelable: false });
  if (task.type === "download" && task.downloadId) {
    cancelSftpDownload(task.downloadId).catch(() => {});
  }
}

function handleClearTransferTask(tab, task) {
  if (!task || task.status === "running" || task.status === "preparing" || task.status === "canceling") {
    return;
  }
  patchTab(tab, (current) => ({ tasks: current.tasks.filter((item) => item.id !== task.id) }));
}

function isTaskCanceled(tab, taskId) {
  return Boolean(currentTab(tab).tasks.find((task) => task.id === taskId)?.canceled);
}

function isTransferCanceled(error) {
  return String(error).includes("TRANSFER_CANCELED");
}

function isSshTransportDisconnect(error) {
  const message = String(error);
  return message.includes("transport read") || message.includes("Failure while draining incoming flow");
}

function toggleMonitorPanel() {
  if (activeTab.value?.kind !== "terminal") {
    ElMessage.info("请先切换到终端标签");
    return;
  }

  patchTab(activeTab.value, { monitorVisible: !activeTab.value.monitorVisible });
}

function runCommandInActiveTerminal(command) {
  if (activeTab.value?.kind !== "terminal") {
    ElMessage.info("请先切换到终端标签");
    return;
  }

  if (activeTab.value.state !== "connected") {
    ElMessage.info("终端连接后才能进入容器");
    return;
  }

  handleTerminalInput(activeTab.value, command);
}

function formatFileSize(size) {
  if (size == null) {
    return "-";
  }
  if (size < 1024) {
    return `${size}B`;
  }
  if (size < 1024 * 1024) {
    return `${Math.round(size / 1024)}KB`;
  }
  return `${(size / 1024 / 1024).toFixed(1)}MB`;
}
</script>

<template>
  <section class="terminal-file-workspace">
    <header v-if="tabs.length > 0" class="ssh-tabbar" :class="{ 'without-file-action': !hasTerminalTab }">
      <div ref="tabsScroll" class="ssh-tabs" role="tablist" @wheel="scrollSshTabs">
        <button
          v-for="tab in visibleTabs"
          :key="tab.id"
          class="ssh-tab"
          :class="{ active: activeTabId === tab.id }"
          :title="tab.label"
          role="tab"
          @click="activeTabId = tab.id"
          @contextmenu.prevent="openTabContextMenu($event, tab)"
        >
          <el-icon><MonitorIcon v-if="tab.kind === 'terminal'" /><FolderOpened v-else /></el-icon>
          <span>{{ tab.connection?.name ?? tab.label }}</span>
          <el-icon class="ssh-tab-close" @click.stop="closeTab(tab.id)"><Close /></el-icon>
        </button>
        <el-dropdown v-if="overflowTabs.length" trigger="click" @command="activeTabId = $event">
          <button class="ssh-tab-more" type="button">
            <span>更多</span>
            <strong>{{ overflowTabs.length }}</strong>
          </button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item
                v-for="tab in overflowTabs"
                :key="tab.id"
                :command="tab.id"
                :class="{ active: activeTabId === tab.id }"
                @contextmenu.prevent="openTabContextMenu($event, tab)"
              >
                {{ tab.connection?.name ?? tab.label }}
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </div>
      <ContextMenu
        v-model="tabContextOpen"
        :items="tabContextItems"
        :x="tabContextPosition.x"
        :y="tabContextPosition.y"
        @select="handleTabContextSelect"
      />
      <div v-if="hasTerminalTab" class="ssh-tab-actions">
        <el-tooltip content="打开 SFTP 文件">
          <el-button
            :icon="FolderOpened"
            :disabled="!connection"
            @click.prevent
            @keydown.enter.prevent="openSftpTab"
            @keydown.space.prevent="openSftpTab"
            @pointerdown.prevent="openSftpTab"
          />
        </el-tooltip>
        <el-tooltip content="服务器监控">
          <el-button
            :icon="MonitorIcon"
            :class="{ active: activeTerminalMonitorVisible }"
            @click="toggleMonitorPanel"
          />
        </el-tooltip>
      </div>
    </header>

    <div class="ssh-tab-content" :class="{ 'no-tabs': tabs.length === 0 }">
      <div v-if="activeTab?.kind === 'terminal'" class="ssh-main-pane">
        <TerminalSessionPane
          :connection="activeTab.connection"
          :error="activeTab.error"
          :output="activeTab.output"
          :state="activeTab.state"
          :terminal-theme="props.terminalTheme"
          @input="(data) => handleTerminalInput(activeTab, data)"
          @resize="(size) => handleTerminalResize(activeTab, size)"
        />
        <aside v-if="activeTab.monitorVisible" class="ssh-monitor-panel">
          <MonitorWorkspace
            :connection="activeTab.connection"
            @close="patchTab(activeTab, { monitorVisible: false })"
            @terminal-command="runCommandInActiveTerminal"
          />
        </aside>
      </div>
      <div v-else-if="activeTab?.kind === 'sftp'" class="ssh-main-pane">
        <SftpFilePane
          :connection="activeTab.connection"
          :error="activeTab.error"
          :files="activeTab.files"
          :loading="activeTab.loading"
          :busy="(activeTab.busyCount ?? 0) > 0"
          :path="activeTab.path"
          :tasks="activeTab.tasks"
          :clipboard-file="activeTab.clipboardFile"
          :state="activeTab.connecting ? 'connecting' : 'connected'"
          @cancel-task="(task) => handleCancelTransferTask(activeTab, task)"
          @clear-task="(task) => handleClearTransferTask(activeTab, task)"
          @context-action="(payload) => handleSftpContextAction(activeTab, payload)"
          @create-dir="handleSftpCreateDir(activeTab)"
          @delete="(file) => handleSftpDelete(activeTab, file)"
          @download="(payload) => handleSftpDownload(activeTab, payload)"
          @edit="(file) => handleSftpEdit(activeTab, file)"
          @open-path="(path) => loadRemoteFiles(activeTab, path)"
          @paste="handleSftpPaste(activeTab)"
          @rename="(file) => handleSftpRename(activeTab, file)"
          @refresh="loadRemoteFiles(activeTab, activeTab.path)"
          @upload="(file) => handleSftpUpload(activeTab, file)"
        />
      </div>
      <div v-else class="ssh-empty">
        <div class="ssh-empty__panel">
          <div class="ssh-empty__visual">
            <el-icon><Connection /></el-icon>
          </div>
          <div class="ssh-empty__copy">
            <p>待连接</p>
            <strong>{{ connection?.name ?? "选择一个 SSH 主机" }}</strong>
            <span>{{ selectedSshTarget }}</span>
          </div>
          <div class="ssh-empty__steps">
            <div class="ssh-empty__step active">
              <span>1</span>
              <strong>选择主机</strong>
            </div>
            <el-icon><Right /></el-icon>
            <div class="ssh-empty__step">
              <span>2</span>
              <strong>打开终端</strong>
            </div>
            <el-icon><Right /></el-icon>
            <div class="ssh-empty__step">
              <span>3</span>
              <strong>SFTP 文件</strong>
            </div>
          </div>
          <div class="ssh-empty__actions">
            <el-button
              class="system-button"
              :icon="MonitorIcon"
              :disabled="!connection"
              @click="openTerminalTab(connection)"
            >
              连接终端
            </el-button>
            <el-button class="system-button" :icon="FolderOpened" :disabled="!connection" @click="openSftpTab">
              打开文件
            </el-button>
          </div>
        </div>
      </div>
    </div>
    <SftpEditorDialog
      v-model="editorDialog.visible"
      :content="editorDialog.content"
      :file-name="editorDialog.fileName"
      :loading="editorDialog.loading"
      :path="editorDialog.path"
      :saving="editorDialog.saving"
      @save="saveSftpEditorContent"
    />
  </section>
</template>

<style scoped>
.terminal-file-workspace {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  background: var(--panel);
}

.ssh-tabbar {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 8px;
  align-items: start;
  min-height: 44px;
  max-height: 78px;
  padding: 6px 10px 0;
  border-bottom: 1px solid var(--line);
  background: #fff;
}

.ssh-tabbar.without-file-action {
  grid-template-columns: minmax(0, 1fr);
}

.ssh-tab-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.ssh-tab-actions :deep(.el-button) {
  width: 30px;
  height: 30px;
  margin: 0;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #fff;
  color: var(--muted);
  box-shadow: none;
}

.ssh-tab-actions :deep(.el-button.active),
.ssh-tab-actions :deep(.el-button:hover) {
  border-color: var(--line-strong);
  background: var(--surface-strong);
  color: var(--orange);
}

.ssh-tab-actions :deep(.el-button:active) {
  background: #e8e9ec;
  transform: none;
}

.ssh-tabs {
  position: relative;
  display: flex;
  flex-wrap: wrap;
  align-content: flex-start;
  min-width: 0;
  max-height: 72px;
  overflow: hidden;
  padding: 0 1px;
  gap: 4px;
}

.ssh-tab {
  display: inline-grid;
  grid-template-columns: 16px minmax(0, 1fr) 16px;
  gap: 6px;
  align-items: center;
  width: 128px;
  height: 34px;
  flex: 0 0 128px;
  padding: 0 8px;
  border: 1px solid var(--line);
  border-bottom-color: transparent;
  border-radius: 8px 8px 0 0;
  background: var(--surface-muted);
  color: var(--muted);
  cursor: pointer;
  font-size: 13px;
}

.ssh-tab span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 500;
}

.ssh-tab.active {
  background: var(--panel);
  color: var(--text);
}

.ssh-tab-close {
  color: var(--faint);
}

.ssh-tab-close:hover {
  color: var(--danger);
}

.ssh-tab-more {
  display: inline-grid;
  grid-template-columns: minmax(0, 1fr) 22px;
  gap: 6px;
  align-items: center;
  width: 96px;
  height: 34px;
  flex: 0 0 96px;
  padding: 0 8px;
  border: 1px solid var(--line);
  border-bottom-color: transparent;
  border-radius: 8px 8px 0 0;
  background: #fff;
  color: var(--muted);
  cursor: pointer;
  font-size: 12px;
}

.ssh-tab-more span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ssh-tab-more strong {
  display: grid;
  place-items: center;
  height: 20px;
  border-radius: 999px;
  background: var(--surface-muted);
  color: var(--text);
  font-size: 11px;
  font-weight: 650;
}

.ssh-tab-more:hover {
  border-color: var(--line-strong);
  background: var(--surface-strong);
  color: var(--text);
}

.ssh-tab-content {
  display: flex;
  min-height: 0;
  flex: 1;
  padding: 10px 10px 10px 0;
}

.ssh-tab-content.no-tabs {
  padding-top: 0;
}

.ssh-tab-content > :deep(*),
.ssh-main-pane > :deep(*) {
  flex: 1;
}

.ssh-main-pane {
  display: flex;
  min-width: 0;
  min-height: 0;
  flex: 1;
}

.ssh-monitor-panel {
  display: flex;
  width: 396px;
  min-width: 396px;
  max-width: 396px;
  flex: 0 0 396px;
  min-height: 0;
}

.ssh-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 0;
  min-height: 0;
  padding: 26px;
  background:
    linear-gradient(90deg, rgba(242, 107, 58, 0.05), transparent 34%),
    linear-gradient(180deg, var(--surface-muted), #fff);
  color: var(--muted);
}

.ssh-empty__panel {
  display: grid;
  justify-items: center;
  width: min(520px, 100%);
  gap: 18px;
  text-align: center;
}

.ssh-empty__visual {
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

.ssh-empty__visual .el-icon {
  font-size: 28px;
}

.ssh-empty__copy {
  display: grid;
  gap: 6px;
}

.ssh-empty__copy p {
  margin: 0;
  color: var(--orange);
  font-size: 11px;
  font-weight: 780;
}

.ssh-empty__copy strong {
  color: var(--text);
  font-size: 20px;
  font-weight: 780;
}

.ssh-empty__copy span {
  color: var(--muted);
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
  font-size: 12px;
}

.ssh-empty__steps {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 16px minmax(0, 1fr) 16px minmax(0, 1fr);
  align-items: center;
  width: 100%;
  max-width: 430px;
  gap: 8px;
}

.ssh-empty__steps > .el-icon {
  color: var(--line-strong);
  font-size: 13px;
}

.ssh-empty__step {
  display: grid;
  justify-items: center;
  min-width: 0;
  gap: 7px;
}

.ssh-empty__step span {
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

.ssh-empty__step strong {
  max-width: 100%;
  overflow: hidden;
  color: var(--muted);
  font-size: 12px;
  font-weight: 650;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ssh-empty__step.active span {
  border-color: #f5c5b3;
  background: var(--orange-soft);
  color: var(--orange);
}

.ssh-empty__step.active strong {
  color: var(--text);
}

.ssh-empty__actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 8px;
}

.ssh-empty__actions :deep(.el-button) {
  height: 34px;
  margin: 0;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #fff;
  color: var(--muted);
  font-size: 12px;
  font-weight: 650;
  box-shadow: none;
}

.ssh-empty__actions :deep(.el-button:hover) {
  border-color: var(--line-strong);
  background: var(--surface-strong);
  color: var(--text);
}

.ssh-empty__actions :deep(.el-button:active) {
  background: #e8e9ec;
  transform: none;
}

</style>
