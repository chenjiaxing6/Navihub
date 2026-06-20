<script setup>
import { computed, ref, watch } from "vue";
import ContextMenu from "../../shared/ContextMenu.vue";
import {
  ArrowLeft,
  Box,
  Close,
  Coin,
  Cpu,
  Delete,
  Document,
  DocumentChecked,
  Download,
  EditPen,
  FolderAdd,
  Headset,
  Management,
  Memo,
  Monitor,
  Picture,
  Refresh,
  SetUp,
  Upload,
  VideoCamera,
} from "@element-plus/icons-vue";

const props = defineProps({
  connection: { type: Object, default: null },
  error: { type: String, default: "" },
  files: { type: Array, required: true },
  loading: { type: Boolean, default: false },
  busy: { type: Boolean, default: false },
  path: { type: String, default: "/" },
  state: { type: String, default: "idle" },
  tasks: { type: Array, default: () => [] },
  clipboardFile: { type: Object, default: null },
});

const emit = defineEmits([
  "cancel-task",
  "clear-task",
  "context-action",
  "create-dir",
  "delete",
  "download",
  "edit",
  "open-path",
  "paste",
  "refresh",
  "rename",
  "upload",
]);

const fileInput = ref(null);
const selectedName = ref("");
const pathDraft = ref(props.path || "/");
const contextMenuOpen = ref(false);
const contextMenuPosition = ref({ x: 0, y: 0 });
const contextFile = ref(null);

const connected = computed(() => props.state === "connected");
const connecting = computed(() => props.state === "connecting");
const selectedFile = computed(() => props.files.find((file) => file.name === selectedName.value) ?? null);
const remotePath = computed(() => props.path || props.connection?.config?.remotePath || ".");
const contextTarget = computed(() => contextFile.value ?? selectedFile.value);
const visibleTasks = computed(() => props.tasks.slice(0, 6));
const activeTaskCount = computed(() => props.tasks.filter((task) => task.status === "running" || task.status === "preparing" || task.status === "canceling").length);
const completedTaskCount = computed(() => props.tasks.filter((task) => task.status === "done").length);
const contextMenuItems = computed(() => {
  const file = contextTarget.value;
  const hasFile = Boolean(file);
  const fileOnly = hasFile && !file.folder;
  return [
    { key: "refresh", label: "刷新" },
    { key: "download-to", label: "下载至", disabled: !fileOnly },
    { key: "rename", label: "重命名", disabled: !hasFile },
    { key: "delete", label: "删除", disabled: !hasFile, danger: true },
    { key: "copy-path", label: "复制路径", disabled: !hasFile },
    { key: "create-dir", label: "新建文件夹", divided: true },
    { key: "edit", label: "在线编辑", disabled: !fileOnly },
    { key: "copy", label: "复制", disabled: !fileOnly },
    { key: "paste", label: "粘贴", disabled: !props.clipboardFile },
  ];
});
const parentPath = computed(() => {
  if (remotePath.value === "." || remotePath.value === "~") {
    return ".";
  }

  const normalized = remotePath.value.replace(/\/+$/, "") || "/";
  if (normalized === "/") {
    return "/";
  }

  return normalized.slice(0, normalized.lastIndexOf("/")) || "/";
});

const fileIconGroups = [
  {
    type: "image",
    icon: Picture,
    extensions: ["avif", "bmp", "gif", "heic", "ico", "jpeg", "jpg", "png", "svg", "tif", "tiff", "webp"],
  },
  {
    type: "video",
    icon: VideoCamera,
    extensions: ["avi", "flv", "m4v", "mkv", "mov", "mp4", "mpeg", "mpg", "webm", "wmv"],
  },
  {
    type: "audio",
    icon: Headset,
    extensions: ["aac", "aiff", "flac", "m4a", "mp3", "ogg", "opus", "wav", "wma"],
  },
  {
    type: "archive",
    icon: Box,
    extensions: ["7z", "br", "bz2", "gz", "rar", "tar", "tgz", "xz", "zip", "zst"],
  },
  {
    type: "code",
    icon: Cpu,
    extensions: ["c", "cpp", "cs", "css", "go", "h", "html", "java", "js", "jsx", "kt", "php", "py", "rb", "rs", "scss", "ts", "tsx", "vue"],
  },
  {
    type: "script",
    icon: Monitor,
    extensions: ["bat", "cmd", "fish", "ps1", "sh", "zsh"],
  },
  {
    type: "config",
    icon: SetUp,
    extensions: ["conf", "env", "ini", "lock", "properties", "toml", "yaml", "yml"],
  },
  {
    type: "data",
    icon: Coin,
    extensions: ["csv", "db", "json", "jsonl", "parquet", "sql", "sqlite", "tsv", "xml"],
  },
  {
    type: "document",
    icon: DocumentChecked,
    extensions: ["doc", "docx", "key", "numbers", "pages", "pdf", "ppt", "pptx", "rtf", "xls", "xlsx"],
  },
  {
    type: "text",
    icon: Memo,
    extensions: ["log", "md", "markdown", "txt"],
  },
  {
    type: "app",
    icon: Management,
    extensions: ["app", "bin", "deb", "dmg", "exe", "msi", "pkg", "rpm"],
  },
];

watch(
  () => props.path,
  (path) => {
    pathDraft.value = path || "/";
    selectedName.value = "";
  },
);

function childPath(name) {
  if (remotePath.value === "." || remotePath.value === "~") {
    return name;
  }

  return `${remotePath.value.replace(/\/+$/, "")}/${name}`.replace(/^\/\//, "/");
}

function chooseFile(event) {
  const [file] = event.target.files ?? [];
  event.target.value = "";
  if (file) {
    emit("upload", file);
  }
}

function openContextMenu(event, file = null) {
  if (!connected.value) {
    return;
  }

  contextFile.value = file;
  if (file) {
    selectedName.value = file.name;
  }
  contextMenuPosition.value = { x: event.clientX, y: event.clientY };
  contextMenuOpen.value = true;
}

function handleContextSelect(item) {
  const file = contextTarget.value;
  if (item.key === "refresh") {
    emit("refresh");
    return;
  }
  if (item.key === "create-dir") {
    emit("create-dir");
    return;
  }
  if (item.key === "paste") {
    emit("paste");
    return;
  }
  if (!file) {
    return;
  }
  emit("context-action", { action: item.key, file });
}

function openSelected(file) {
  selectedName.value = file.name;
  if (file.folder) {
    emit("open-path", childPath(file.name));
  }
}

function submitPath() {
  const nextPath = pathDraft.value.trim() || "/";
  emit("open-path", nextPath.startsWith("/") ? nextPath : `/${nextPath}`);
}

function fileModifiedTime(file) {
  if (!file.modifiedTime) {
    return "-";
  }

  return new Date(file.modifiedTime * 1000).toLocaleString();
}

function fileExtension(name) {
  const normalized = name.trim().toLowerCase();
  if (!normalized || (normalized.startsWith(".") && normalized.indexOf(".", 1) === -1)) {
    return "";
  }

  return normalized.slice(normalized.lastIndexOf(".") + 1);
}

function fileIconMeta(file) {
  if (file.folder) {
    return { icon: null, type: "folder" };
  }

  const extension = fileExtension(file.name);
  const group = fileIconGroups.find((item) => item.extensions.includes(extension));
  return group ?? { icon: Document, type: "file" };
}

function fileIconClass(file) {
  return `file-icon-${fileIconMeta(file).type}`;
}

function fileIconComponent(file) {
  return fileIconMeta(file).icon;
}

function taskTitle(task) {
  const direction = task.type === "upload" ? "上传" : "下载";
  return `${direction} · ${task.name}`;
}

function taskStatusLabel(task) {
  if (task.status === "preparing") {
    return "准备中";
  }
  if (task.status === "running") {
    return "传输中";
  }
  if (task.status === "done") {
    return "完成";
  }
  if (task.status === "failed") {
    return "失败";
  }
  if (task.status === "canceling") {
    return "取消中";
  }
  if (task.status === "canceled") {
    return "已取消";
  }
  return "等待";
}

function canClearTask(task) {
  return task.status === "done" || task.status === "failed" || task.status === "canceled";
}
</script>

<template>
  <aside class="remote-file-pane">
    <header class="file-toolbar">
      <el-tooltip content="返回上级">
        <el-button :icon="ArrowLeft" text :disabled="!connected || loading" @click="emit('open-path', parentPath)" />
      </el-tooltip>
      <el-tooltip content="刷新">
        <el-button :icon="Refresh" text :disabled="!connected || loading" @click="emit('refresh')" />
      </el-tooltip>
      <input
        v-model="pathDraft"
        class="path-input"
        :disabled="!connected || loading"
        :placeholder="connected ? '/' : '未连接'"
        @keydown.enter="submitPath"
        @blur="pathDraft = remotePath"
      />
      <input ref="fileInput" class="file-input" type="file" @change="chooseFile" />
      <el-tooltip content="上传文件">
        <el-button :icon="Upload" text :disabled="!connected || loading" @click="fileInput?.click()" />
      </el-tooltip>
      <el-tooltip content="下载文件">
        <el-button
          :icon="Download"
          text
          :disabled="!connected || loading || !selectedFile || selectedFile.folder"
          @click="emit('download', { file: selectedFile, choosePath: true })"
        />
      </el-tooltip>
      <el-tooltip content="新建文件夹">
        <el-button :icon="FolderAdd" text :disabled="!connected || loading" @click="emit('create-dir')" />
      </el-tooltip>
      <el-tooltip content="重命名">
        <el-button
          :icon="EditPen"
          text
          :disabled="!connected || loading || !selectedFile"
          @click="emit('rename', selectedFile)"
        />
      </el-tooltip>
      <el-tooltip content="删除选中">
        <el-button
          :icon="Delete"
          text
          :disabled="!connected || loading || !selectedFile"
          @click="emit('delete', selectedFile)"
        />
      </el-tooltip>
      <div v-if="loading || connecting || busy" class="toolbar-loading" aria-hidden="true">
        <span />
      </div>
    </header>

    <template v-if="connected">
      <div v-if="error" class="file-selection error">
        <span v-if="error">{{ error }}</span>
      </div>

      <div class="file-head">
        <span>名称</span>
        <span>大小</span>
        <span>修改时间</span>
      </div>

      <div class="file-list" @contextmenu.prevent="openContextMenu($event)">
        <button
          v-for="file in files"
          :key="file.name"
          class="file-row"
          :class="{ selected: selectedName === file.name }"
          @click="selectedName = file.name"
          @contextmenu.prevent.stop="openContextMenu($event, file)"
          @dblclick="openSelected(file)"
        >
          <span v-if="file.folder" class="file-folder-icon" aria-hidden="true" />
          <el-icon v-else class="file-icon" :class="fileIconClass(file)">
            <component :is="fileIconComponent(file)" />
          </el-icon>
          <span class="file-name">{{ file.name }}</span>
          <span class="file-size">{{ file.displaySize ?? file.size ?? "-" }}</span>
          <span class="file-time">{{ fileModifiedTime(file) }}</span>
        </button>
      </div>

      <section class="transfer-tasks" :class="{ empty: tasks.length === 0 }">
        <header class="transfer-tasks__head">
          <strong>传输任务</strong>
          <span v-if="tasks.length">{{ activeTaskCount }} 进行中 · {{ completedTaskCount }} 完成</span>
          <span v-else>暂无上传或下载</span>
        </header>
        <div v-if="tasks.length" class="transfer-task-list">
          <article
            v-for="task in visibleTasks"
            :key="task.id"
            class="transfer-task"
            :class="`transfer-task--${task.status}`"
          >
            <div class="transfer-task__main">
              <span class="transfer-task__title">{{ taskTitle(task) }}</span>
              <span class="transfer-task__meta">
                {{ task.transferredLabel }} / {{ task.sizeLabel }} · {{ task.speedLabel }} · {{ taskStatusLabel(task) }}
              </span>
            </div>
            <div class="transfer-task__side">
              <el-progress :percentage="task.progress" :show-text="false" />
              <button
                v-if="task.cancelable && (task.status === 'running' || task.status === 'preparing')"
                class="transfer-task__cancel"
                type="button"
                title="取消"
                @click="emit('cancel-task', task)"
              >
                <el-icon><Close /></el-icon>
              </button>
              <button
                v-else-if="canClearTask(task)"
                class="transfer-task__cancel"
                type="button"
                title="删除任务"
                @click="emit('clear-task', task)"
              >
                <el-icon><Delete /></el-icon>
              </button>
            </div>
          </article>
        </div>
      </section>
    </template>

    <div v-else class="file-empty">
      <strong>{{ connecting ? "正在连接 SFTP" : "SFTP 未连接" }}</strong>
      <span>{{ error || (connecting ? "正在建立文件会话..." : "点击文件夹按钮打开文件标签。") }}</span>
    </div>

    <ContextMenu
      v-model="contextMenuOpen"
      :items="contextMenuItems"
      :x="contextMenuPosition.x"
      :y="contextMenuPosition.y"
      @select="handleContextSelect"
    />
  </aside>
</template>

<style scoped>
.remote-file-pane {
  display: flex;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  flex-direction: column;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #fff;
  box-shadow: none;
}

.file-toolbar {
  position: relative;
  display: grid;
  grid-template-columns: 28px 28px minmax(0, 1fr) repeat(5, 28px);
  gap: 4px;
  align-items: center;
  height: 42px;
  padding: 5px 7px;
  border-bottom: 1px solid var(--line);
  background: #fff;
}

.file-toolbar :deep(.el-button) {
  width: 28px;
  height: 28px;
  margin: 0;
  color: var(--text);
}

.file-input {
  display: none;
}

.path-input {
  height: 30px;
  min-width: 0;
  padding: 0 12px;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 8px;
  color: var(--text);
  font-size: 13px;
  line-height: 28px;
  text-overflow: ellipsis;
  white-space: nowrap;
  outline: none;
}

.path-input:disabled {
  background: #fff;
  color: var(--text);
  opacity: 1;
}

.toolbar-loading {
  position: absolute;
  top: 0;
  right: 8px;
  left: 8px;
  height: 2px;
  overflow: hidden;
  border-radius: 999px;
  background: transparent;
}

.toolbar-loading span {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 28%;
  border-radius: inherit;
  background: #1f2937;
  animation: toolbar-loading 1.05s ease-in-out infinite;
}

@keyframes toolbar-loading {
  0% {
    transform: translateX(-120%);
  }

  100% {
    transform: translateX(460%);
  }
}

.file-selection {
  height: 36px;
  padding: 0 14px;
  overflow: hidden;
  border-bottom: 1px solid var(--line);
  color: var(--text);
  font-size: 13px;
  line-height: 36px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-selection.error {
  color: var(--danger);
}

.file-head {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 96px 170px;
  height: 34px;
  border-bottom: 1px solid var(--line);
  background: var(--surface-muted);
  color: var(--muted);
  font-size: 12px;
  line-height: 34px;
}

.file-head span {
  padding: 0 12px;
  border-right: 1px solid var(--line);
}

.file-list {
  position: relative;
  min-height: 0;
  flex: 1;
  overflow: auto;
}

.file-row {
  display: grid;
  grid-template-columns: 24px minmax(0, 1fr) 96px 170px;
  align-items: center;
  width: 100%;
  height: 34px;
  padding: 0 10px;
  border: 0;
  border-radius: 0;
  background: transparent;
  color: var(--text);
  cursor: default;
  font-size: 13px;
  text-align: left;
}

.file-row:nth-child(even) {
  background: #fafafa;
}

.file-row:hover {
  background: #f3f6fa;
}

.file-row.selected {
  background: #dcecff;
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.42),
    inset 0 -1px 0 rgba(96, 137, 183, 0.12);
}

.file-row.selected:hover {
  background: #d5e8ff;
}

.file-row.selected .file-name {
  color: #1f2937;
  font-weight: 560;
}

.file-row.selected .file-size,
.file-row.selected .file-time {
  color: #506070;
}

.file-icon {
  width: 18px;
  height: 18px;
  color: #8b929d;
}

.file-folder-icon {
  position: relative;
  width: 19px;
  height: 14px;
  margin-left: 1px;
  border-radius: 2px;
  background: #ffc529;
  box-shadow: inset 0 -1px 0 rgba(180, 120, 0, 0.12);
}

.file-folder-icon::before {
  position: absolute;
  top: -3px;
  left: 0;
  width: 9px;
  height: 5px;
  border-radius: 2px 2px 0 0;
  background: #ffd85a;
  content: "";
}

.file-folder-icon::after {
  position: absolute;
  inset: 1px 0 auto;
  height: 1px;
  background: rgba(255, 255, 255, 0.34);
  content: "";
}

.file-icon-image {
  color: #2f9e8f;
}

.file-icon-video {
  color: #7c5ce6;
}

.file-icon-audio {
  color: #df6f24;
}

.file-icon-archive {
  color: #9a7a34;
}

.file-icon-code,
.file-icon-script {
  color: #2563eb;
}

.file-icon-config {
  color: #64748b;
}

.file-icon-data {
  color: #168047;
}

.file-icon-document,
.file-icon-text {
  color: #56616f;
}

.file-icon-app {
  color: #8b5cf6;
}

.file-row.selected .file-icon {
  filter: saturate(1.08);
}

.file-row.selected .file-folder-icon {
  background: #ffc529;
  filter: saturate(1.04);
}

.file-row.selected .file-folder-icon::before {
  background: #ffd14a;
}

.file-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-size {
  color: var(--muted);
  text-align: right;
  white-space: nowrap;
}

.file-time {
  overflow: hidden;
  color: var(--muted);
  text-align: right;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-empty {
  display: grid;
  flex: 1;
  place-content: center;
  gap: 7px;
  padding: 20px;
  color: var(--muted);
  text-align: center;
}

.file-empty strong {
  color: var(--text);
  font-size: 14px;
}

.file-empty span {
  font-size: 12px;
}

.transfer-tasks {
  flex: 0 0 auto;
  border-top: 1px solid var(--line);
  background: #fff;
}

.transfer-tasks.empty {
  background: var(--surface-muted);
}

.transfer-tasks__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 32px;
  padding: 0 12px;
  color: var(--muted);
  font-size: 12px;
}

.transfer-tasks__head strong {
  color: var(--text);
  font-size: 12px;
  font-weight: 650;
}

.transfer-task-list {
  max-height: 156px;
  overflow: auto;
  border-top: 1px solid var(--line);
}

.transfer-task {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 142px;
  gap: 12px;
  align-items: center;
  min-height: 42px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--line);
}

.transfer-task:last-child {
  border-bottom: 0;
}

.transfer-task__main {
  display: grid;
  min-width: 0;
  gap: 2px;
}

.transfer-task__title,
.transfer-task__meta {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.transfer-task__title {
  color: var(--text);
  font-size: 12px;
}

.transfer-task__meta {
  color: var(--muted);
  font-size: 11px;
}

.transfer-task__side {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 24px;
  gap: 8px;
  align-items: center;
}

.transfer-task__side :deep(.el-progress) {
  min-width: 0;
}

.transfer-task__cancel {
  display: grid;
  width: 22px;
  height: 22px;
  place-items: center;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--faint);
  cursor: pointer;
  font-size: 13px;
}

.transfer-task__cancel:hover {
  background: var(--surface-strong);
  color: var(--text);
}

.transfer-task :deep(.el-progress-bar__outer) {
  height: 5px !important;
  background: #e8edf3;
}

.transfer-task :deep(.el-progress-bar__inner) {
  background: #4f8fd9;
}

.transfer-task--done :deep(.el-progress-bar__inner) {
  background: var(--green);
}

.transfer-task--failed .transfer-task__meta {
  color: var(--red);
}

.transfer-task--failed :deep(.el-progress-bar__inner) {
  background: var(--red);
}

.transfer-task--canceling :deep(.el-progress-bar__inner),
.transfer-task--canceled :deep(.el-progress-bar__inner) {
  background: #9aa0aa;
}

.transfer-task--canceled .transfer-task__meta {
  color: var(--faint);
}
</style>
