<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { ElMessage } from "element-plus/es/components/message/index";
import { ElMessageBox } from "element-plus/es/components/message-box/index";
import {
  Box,
  Close,
  Clock,
  Coin,
  Cpu,
  Delete,
  Monitor,
  Connection,
  Refresh,
  SwitchButton,
  User,
  VideoPlay,
} from "@element-plus/icons-vue";
import {
  killMonitorProcess,
  loadMonitorSnapshot,
  removeMonitorContainer,
  startMonitorContainer,
  stopMonitorContainer,
} from "./monitorApi";

const props = defineProps({
  connection: { type: Object, default: null },
});

const emit = defineEmits(["close", "terminal-command"]);

const snapshot = ref(null);
const previousSnapshot = ref(null);
const history = ref([]);
const loading = ref(false);
const refreshing = ref(false);
const manualRefreshing = ref(false);
const error = ref("");
const showAllContainers = ref(false);
const processLimit = ref(8);
let refreshTimer = null;
let refreshSerial = 0;

const target = computed(() => {
  if (props.connection?.workspace === "ssh") {
    return { kind: "ssh", ssh: props.connection.config };
  }
  return { kind: "local" };
});

const titleHost = computed(() => {
  if (props.connection?.workspace === "ssh") {
    const { username = "root", host = "", port = 22 } = props.connection.config ?? {};
    return `${username}@${host}:${port}`;
  }
  return "本机";
});

const visibleProcesses = computed(() => (snapshot.value?.processes ?? []).slice(0, processLimit.value));
const visibleContainers = computed(() => {
  const containers = snapshot.value?.docker?.containers ?? [];
  return showAllContainers.value ? containers : containers.filter((container) => isContainerRunning(container));
});

const chartPoints = computed(() => {
  const width = 680;
  const height = 164;
  const pad = 10;
  const max = 100;

  function line(key) {
    if (history.value.length === 0) {
      return "";
    }
    return history.value
      .map((item, index) => {
        const x = pad + (index / Math.max(history.value.length - 1, 1)) * (width - pad * 2);
        const y = height - pad - (Math.min(item[key], max) / max) * (height - pad * 2);
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  }

  return {
    cpu: line("cpu"),
    memory: line("memory"),
    upload: line("upload"),
    download: line("download"),
  };
});

watch(
  () => props.connection?.id,
  () => {
    history.value = [];
    previousSnapshot.value = null;
    snapshot.value = null;
    refresh({ visible: true });
  },
);

onMounted(() => {
  refresh({ visible: true });
  refreshTimer = window.setInterval(() => refresh({ visible: false }), 5000);
});

onBeforeUnmount(() => {
  refreshSerial += 1;
  window.clearInterval(refreshTimer);
});

async function refresh(options = {}) {
  const visible = options.visible ?? false;
  if (refreshing.value) {
    return;
  }
  const serial = ++refreshSerial;
  refreshing.value = true;
  if (visible || !snapshot.value) {
    loading.value = true;
    manualRefreshing.value = true;
  }
  try {
    const next = await loadMonitorSnapshot(target.value);
    if (serial !== refreshSerial) {
      return;
    }
    const enriched = enrichSnapshot(next, previousSnapshot.value);
    previousSnapshot.value = enriched;
    snapshot.value = enriched;
    pushHistory(enriched);
    error.value = "";
  } catch (err) {
    error.value = String(err);
  } finally {
    if (serial === refreshSerial) {
      refreshing.value = false;
      manualRefreshing.value = false;
      loading.value = false;
    }
  }
}

function enrichSnapshot(next, previous) {
  const elapsed = Math.max(((next.timestamp ?? Date.now()) - (previous?.timestamp ?? next.timestamp ?? Date.now())) / 1000, 1);
  const previousNetworks = new Map((previous?.networks ?? []).map((item) => [item.name, item]));
  const networks = (next.networks ?? []).map((item) => {
    const before = previousNetworks.get(item.name);
    return {
      ...item,
      rxRate: before ? Math.max((item.rxTotal - before.rxTotal) / elapsed, 0) : 0,
      txRate: before ? Math.max((item.txTotal - before.txTotal) / elapsed, 0) : 0,
    };
  });
  const networkSummary = {
    rxTotal: networks.reduce((sum, item) => sum + item.rxTotal, 0),
    txTotal: networks.reduce((sum, item) => sum + item.txTotal, 0),
    rxRate: networks.reduce((sum, item) => sum + item.rxRate, 0),
    txRate: networks.reduce((sum, item) => sum + item.txRate, 0),
  };
  return { ...next, networks, networkSummary };
}

function pushHistory(item) {
  const memoryPercent = percent(item.memory?.used, item.memory?.total);
  history.value = [
    ...history.value,
    {
      cpu: item.cpu?.totalUsage ?? 0,
      memory: memoryPercent,
      upload: rateToPercent(item.networkSummary?.txRate ?? 0),
      download: rateToPercent(item.networkSummary?.rxRate ?? 0),
    },
  ].slice(-18);
}

async function killProcess(process) {
  await ElMessageBox.confirm(`确认结束进程 ${process.name} (${process.pid})？`, "结束进程", {
    confirmButtonText: "结束",
    cancelButtonText: "取消",
    customClass: "bruno-message-box",
    type: "warning",
  });
  await killMonitorProcess(target.value, process.pid);
  ElMessage.success("已发送结束信号");
  refresh();
}

async function stopContainer(container) {
  await ElMessageBox.confirm(`确认停止容器「${container.name}」？`, "停止容器", {
    confirmButtonText: "停止",
    cancelButtonText: "取消",
    customClass: "bruno-message-box",
    type: "warning",
  });
  await stopMonitorContainer(target.value, container.id);
  ElMessage.success("已停止容器");
  refresh();
}

async function startContainer(container) {
  await ElMessageBox.confirm(`确认启动容器「${container.name}」？`, "启动容器", {
    confirmButtonText: "启动",
    cancelButtonText: "取消",
    customClass: "bruno-message-box",
    type: "warning",
  });
  await startMonitorContainer(target.value, container.id);
  ElMessage.success("已启动容器");
  refresh({ visible: false });
}

async function removeContainer(container) {
  await ElMessageBox.confirm(`确认删除容器「${container.name}」？此操作不可恢复。`, "删除容器", {
    confirmButtonText: "删除",
    cancelButtonText: "取消",
    customClass: "bruno-message-box",
    type: "warning",
  });
  await removeMonitorContainer(target.value, container.id);
  ElMessage.success("已删除容器");
  refresh({ visible: false });
}

function percent(used, total) {
  if (!total) {
    return 0;
  }
  return Math.min((used / total) * 100, 100);
}

function rateToPercent(value) {
  return Math.min((value / (1024 * 1024)) * 25, 100);
}

function formatPercent(value) {
  return `${Number(value ?? 0).toFixed(1)}%`;
}

function formatBytes(value) {
  const bytes = Number(value ?? 0);
  if (bytes >= 1024 ** 3) {
    return `${(bytes / 1024 ** 3).toFixed(1)}GB`;
  }
  if (bytes >= 1024 ** 2) {
    return `${(bytes / 1024 ** 2).toFixed(1)}MB`;
  }
  if (bytes >= 1024) {
    return `${(bytes / 1024).toFixed(1)}KB`;
  }
  return `${bytes.toFixed(0)}B`;
}

function formatRate(value) {
  return `${formatBytes(value)}/s`;
}

function formatUptime(seconds) {
  const value = Number(seconds ?? 0);
  const days = Math.floor(value / 86400);
  const hours = Math.floor((value % 86400) / 3600);
  const minutes = Math.floor((value % 3600) / 60);
  if (days > 0) {
    return `${days}d ${hours}h ${minutes}m`;
  }
  return `${hours}h ${minutes}m`;
}

function openContainerShell(container) {
  emit("terminal-command", `docker exec -it ${shellArg(container.id)} /bin/bash || docker exec -it ${shellArg(container.id)} /bin/sh\n`);
}

function isContainerRunning(container) {
  return String(container?.status ?? "").toLowerCase().startsWith("up");
}

function isContainerStopped(container) {
  return !isContainerRunning(container);
}

function shellArg(value) {
  return `'${String(value ?? "").replaceAll("'", "'\\''")}'`;
}
</script>

<template>
  <section class="monitor-workspace" :class="{ compact: true }">
    <div v-if="loading" class="monitor-loading" />

    <el-alert v-if="error" class="monitor-alert" :title="error" type="error" :closable="false" />

    <div class="monitor-scroll">
      <section class="panel host-panel">
        <div class="monitor-actions">
          <el-tooltip content="刷新">
            <el-button :icon="Refresh" :loading="manualRefreshing" circle @click="refresh({ visible: true })" />
          </el-tooltip>
          <el-tooltip content="关闭">
            <el-button :icon="Close" circle @click="emit('close')" />
          </el-tooltip>
        </div>
        <div class="host-item">
          <el-icon><User /></el-icon>
          <span>用户</span>
          <strong>{{ snapshot?.host?.username || "-" }}</strong>
        </div>
        <div class="host-item">
          <el-icon><Clock /></el-icon>
          <span>运行时间</span>
          <strong>{{ formatUptime(snapshot?.host?.uptimeSeconds) }}</strong>
        </div>
        <div class="host-item">
          <el-icon><Connection /></el-icon>
          <span>Host</span>
          <strong>{{ snapshot?.host?.host || titleHost }}</strong>
        </div>
        <div class="host-item">
          <el-icon><Monitor /></el-icon>
          <span>系统</span>
          <strong>{{ snapshot?.host?.os || "-" }}</strong>
        </div>
      </section>

      <section class="panel metrics-panel">
        <div class="metric-grid">
          <div class="metric-item">
            <strong>{{ formatPercent(snapshot?.cpu?.totalUsage) }}</strong>
            <span>总CPU占用</span>
          </div>
          <div class="metric-item">
            <strong>{{ formatPercent(snapshot?.cpu?.systemUsage) }}</strong>
            <span>内核态</span>
          </div>
          <div class="metric-item">
            <strong>{{ formatPercent(snapshot?.cpu?.userUsage) }}</strong>
            <span>用户态</span>
          </div>
          <div class="metric-item">
            <strong>{{ formatPercent(snapshot?.cpu?.iowaitUsage) }}</strong>
            <span>IO等待</span>
          </div>
        </div>
        <div class="memory-bars">
          <div class="memory-item">
            <span>物理内存</span>
            <div class="usage-bar blue">
              <i :style="{ width: `${percent(snapshot?.memory?.used, snapshot?.memory?.total)}%` }" />
              <strong>{{ formatBytes(snapshot?.memory?.used) }}/{{ formatBytes(snapshot?.memory?.total) }}</strong>
            </div>
          </div>
          <div class="memory-item">
            <span>Swap内存</span>
            <div class="usage-bar teal">
              <i :style="{ width: `${percent(snapshot?.memory?.swapUsed, snapshot?.memory?.swapTotal)}%` }" />
              <strong>{{ formatBytes(snapshot?.memory?.swapUsed) }}/{{ formatBytes(snapshot?.memory?.swapTotal) }}</strong>
            </div>
          </div>
        </div>
      </section>

      <section class="panel chart-panel">
        <div class="traffic-summary">
          <div><span>总上行</span><strong>{{ formatBytes(snapshot?.networkSummary?.txTotal) }}</strong></div>
          <div><span>总下行</span><strong>{{ formatBytes(snapshot?.networkSummary?.rxTotal) }}</strong></div>
          <div class="upload"><span>实时上行</span><strong>{{ formatRate(snapshot?.networkSummary?.txRate) }}</strong></div>
          <div class="download"><span>实时下行</span><strong>{{ formatRate(snapshot?.networkSummary?.rxRate) }}</strong></div>
        </div>
        <div class="legend">
          <span class="cpu">CPU</span>
          <span class="memory">内存</span>
          <span class="upload">上行</span>
          <span class="download">下行</span>
        </div>
        <svg viewBox="0 0 680 164" preserveAspectRatio="none" aria-hidden="true">
          <polyline :points="chartPoints.cpu" class="line cpu" />
          <polyline :points="chartPoints.memory" class="line memory" />
          <polyline :points="chartPoints.upload" class="line upload" />
          <polyline :points="chartPoints.download" class="line download" />
        </svg>
      </section>

      <section class="panel core-panel">
        <el-icon><Cpu /></el-icon>
        <strong>CPU</strong>
        <div class="core-strips">
          <span
            v-for="(core, index) in snapshot?.cpu?.cores ?? []"
            :key="index"
            :class="{ hot: core > 70, warm: core > 35 }"
          />
        </div>
        <b>{{ formatPercent(snapshot?.cpu?.totalUsage) }}</b>
      </section>

      <section class="panel table-panel">
        <header>
          <h3>进程列表</h3>
          <el-select v-model="processLimit" class="panel-select" size="small" popper-class="monitor-select-popper">
            <el-option :value="5" label="5" />
            <el-option :value="8" label="8" />
            <el-option :value="12" label="12" />
          </el-select>
        </header>
        <div class="table-head process">
          <span>进程</span><span>Pid</span><span>%CPU</span><span>内存</span><span>操作</span>
        </div>
        <div v-for="process in visibleProcesses" :key="process.pid" class="table-row process">
          <span class="pill">{{ process.name }}</span>
          <span>{{ process.pid }}</span>
          <strong>{{ formatPercent(process.cpu) }}</strong>
          <span>{{ formatBytes(process.memory) }}</span>
          <el-button text :icon="SwitchButton" @click="killProcess(process)" />
        </div>
      </section>

      <section class="panel table-panel">
        <header>
          <h3><el-icon><Connection /></el-icon>网卡信息</h3>
        </header>
        <div class="table-head network">
          <span>名称</span><span>上/下行</span><span>总流量</span><span>IP</span>
        </div>
        <div v-for="item in snapshot?.networks ?? []" :key="item.name" class="table-row network">
          <span class="pill teal">{{ item.name }}</span>
          <span><b class="upload">↑ {{ formatRate(item.txRate) }}</b><b class="download">↓ {{ formatRate(item.rxRate) }}</b></span>
          <span><b>↑ {{ formatBytes(item.txTotal) }}</b><b>↓ {{ formatBytes(item.rxTotal) }}</b></span>
          <span class="pill">{{ item.ip || "-" }}</span>
        </div>
      </section>

      <section class="panel table-panel">
        <header>
          <h3><el-icon><Box /></el-icon>DOCKER 容器</h3>
          <button
            class="panel-toggle"
            type="button"
            :class="{ active: showAllContainers }"
            @click="showAllContainers = !showAllContainers"
          >
            显示全部
          </button>
        </header>
        <div class="table-head docker">
          <span>名称</span><span>ID</span><span>状态</span><span>Image</span><span>操作</span>
        </div>
        <div v-if="!snapshot?.docker?.available" class="empty-line">未检测到 Docker 或当前用户无权限。</div>
        <div v-for="container in visibleContainers" :key="container.id" class="table-row docker">
          <span class="pill teal">{{ container.name }}</span>
          <span class="pill">{{ container.id }}</span>
          <span>{{ container.status }}</span>
          <span class="pill">{{ container.image }}</span>
          <span class="row-actions">
            <el-button
              v-if="isContainerRunning(container)"
              class="terminal-action"
              text
              @click="openContainerShell(container)"
            >
              <svg class="terminal-action-icon" viewBox="0 0 16 16" aria-hidden="true">
                <rect x="2.25" y="3.25" width="11.5" height="9.5" rx="1.7" />
                <path d="M5.1 6.8L7 8L5.1 9.2" />
                <path d="M8.4 9.4H10.9" />
              </svg>
            </el-button>
            <el-button
              v-if="isContainerRunning(container)"
              text
              :icon="SwitchButton"
              @click="stopContainer(container)"
            />
            <el-button
              v-if="isContainerStopped(container)"
              text
              :icon="VideoPlay"
              @click="startContainer(container)"
            />
            <el-button
              v-if="isContainerStopped(container)"
              text
              :icon="Delete"
              @click="removeContainer(container)"
            />
          </span>
        </div>
      </section>

      <section class="panel table-panel">
        <header>
          <h3><el-icon><Coin /></el-icon>磁盘挂载</h3>
        </header>
        <div class="table-head mount">
          <span>名称</span><span>空间/可用</span><span>Path</span>
        </div>
        <div v-for="mount in snapshot?.mounts ?? []" :key="`${mount.name}-${mount.path}`" class="table-row mount">
          <span class="pill teal">{{ mount.name }}</span>
          <span>
            <div class="usage-bar mini">
              <i :style="{ width: `${percent(mount.used, mount.total)}%` }" />
              <strong>{{ formatBytes(mount.used) }}/{{ formatBytes(mount.available) }}</strong>
            </div>
          </span>
          <span class="pill">{{ mount.path }}</span>
        </div>
      </section>
    </div>
  </section>
</template>

<style scoped>
.monitor-workspace {
  position: relative;
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  background: var(--app-bg);
}

.monitor-loading {
  position: absolute;
  z-index: 3;
  top: 0;
  right: 0;
  left: 0;
  height: 2px;
  overflow: hidden;
  background: var(--blue-soft);
}

.monitor-loading::after {
  display: block;
  width: 42%;
  height: 100%;
  border-radius: 999px;
  background: var(--blue);
  animation: monitor-loading-slide 1s ease-in-out infinite;
  content: "";
}

@keyframes monitor-loading-slide {
  0% {
    transform: translateX(-110%);
  }

  100% {
    transform: translateX(260%);
  }
}

.monitor-workspace.compact {
  width: 396px;
  min-width: 396px;
  max-width: 396px;
  border-left: 1px solid var(--line);
  background: var(--sidebar-bg);
}

.monitor-actions {
  position: absolute;
  top: 7px;
  right: 7px;
  display: flex;
  align-items: center;
  gap: 2px;
}

.monitor-actions :deep(.el-button) {
  width: 26px;
  height: 26px;
  margin: 0;
  border-radius: 8px;
  border-color: transparent;
  background: transparent;
  color: var(--muted);
}

.monitor-alert {
  margin: 10px 10px 0;
}

.monitor-scroll {
  display: grid;
  grid-template-columns: repeat(12, minmax(0, 1fr));
  gap: 10px;
  min-height: 0;
  overflow: auto;
  padding: 10px;
}

.monitor-workspace.compact .monitor-scroll {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 0 8px 8px;
}

.panel {
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--panel);
  box-shadow: none;
}

.monitor-workspace.compact .panel {
  border-radius: 0;
}

.host-panel,
.metrics-panel,
.chart-panel,
.core-panel,
.table-panel {
  grid-column: span 12;
}

.host-panel {
  position: relative;
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
  padding: 16px;
}

.monitor-workspace.compact .host-panel {
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px 12px;
  padding: 10px 12px;
}

.monitor-workspace.compact .host-item {
  grid-template-columns: 20px minmax(0, 1fr);
}

.monitor-workspace.compact .host-item .el-icon {
  font-size: 18px;
}

.monitor-workspace.compact .host-item strong {
  font-size: 12px;
  font-weight: 560;
}

.host-item {
  display: grid;
  grid-template-columns: 28px 1fr;
  gap: 4px 10px;
  min-width: 0;
}

.host-item .el-icon {
  grid-row: span 2;
  color: var(--muted);
  font-size: 24px;
}

.host-item span,
.metric-item span,
.traffic-summary span,
.memory-item > span {
  color: var(--muted);
  font-size: 13px;
}

.host-item strong {
  overflow: hidden;
  font-size: 18px;
  font-weight: 520;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.metrics-panel {
  padding: 18px 24px;
}

.monitor-workspace.compact .metrics-panel,
.monitor-workspace.compact .chart-panel,
.monitor-workspace.compact .table-panel {
  padding: 10px 12px;
}

.metric-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
}

.monitor-workspace.compact .metric-grid {
  grid-template-columns: repeat(4, 1fr);
  gap: 4px;
}

.monitor-workspace.compact .metric-item strong {
  margin-bottom: 2px;
  font-size: 15px;
}

.monitor-workspace.compact .metric-item span {
  font-size: 11px;
}

.metric-item {
  text-align: center;
}

.metric-item strong {
  display: block;
  margin-bottom: 4px;
  font-size: 22px;
  font-weight: 520;
}

.memory-bars {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 26px;
  margin-top: 18px;
}

.monitor-workspace.compact .memory-bars {
  grid-template-columns: 1fr;
  gap: 8px;
  margin-top: 10px;
}

.monitor-workspace.compact .memory-item > span {
  margin-bottom: 4px;
  font-size: 12px;
  text-align: left;
}

.monitor-workspace.compact .usage-bar {
  height: 20px;
  border-radius: 6px;
}

.monitor-workspace.compact .usage-bar strong {
  font-size: 12px;
  line-height: 20px;
}

.memory-item > span {
  display: block;
  margin-bottom: 7px;
  text-align: center;
}

.usage-bar {
  position: relative;
  height: 26px;
  overflow: hidden;
  border-radius: 7px;
  background: #dbeafe;
}

.usage-bar i {
  position: absolute;
  inset: 0 auto 0 0;
  background: var(--blue);
}

.usage-bar.teal {
  background: #b7e1d9;
}

.usage-bar.teal i {
  background: #2da99a;
}

.usage-bar.mini {
  width: 170px;
  height: 24px;
  background: #a9c3f6;
}

.usage-bar.mini i {
  background: var(--blue);
}

.usage-bar strong {
  position: relative;
  z-index: 1;
  display: block;
  color: #fff;
  font-size: 14px;
  font-weight: 520;
  line-height: 26px;
  text-align: center;
}

.chart-panel {
  padding: 16px 28px 18px;
}

.traffic-summary {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 20px;
  margin-bottom: 10px;
}

.monitor-workspace.compact .traffic-summary {
  grid-template-columns: repeat(4, 1fr);
  gap: 6px;
  margin-bottom: 8px;
}

.monitor-workspace.compact .traffic-summary strong {
  margin-top: 2px;
  font-size: 13px;
}

.monitor-workspace.compact .traffic-summary span {
  font-size: 11px;
}

.monitor-workspace.compact .legend {
  gap: 10px;
  margin-bottom: 4px;
  font-size: 12px;
}

.monitor-workspace.compact .legend span::before {
  width: 10px;
  height: 10px;
  margin-right: 4px;
}

.traffic-summary strong {
  display: block;
  margin-top: 5px;
  font-size: 20px;
  font-weight: 520;
}

.upload strong,
.upload {
  color: #ff8500;
}

.download strong,
.download {
  color: #3dab4b;
}

.legend {
  display: flex;
  justify-content: center;
  gap: 16px;
  margin-bottom: 8px;
  color: var(--muted);
  font-size: 13px;
}

.legend span::before {
  display: inline-block;
  width: 13px;
  height: 13px;
  margin-right: 6px;
  border-radius: 4px;
  content: "";
  vertical-align: -2px;
}

.legend .cpu::before {
  background: var(--blue);
}

.legend .memory::before {
  background: #2da99a;
}

.legend .upload::before {
  background: #ff9800;
}

.legend .download::before {
  background: #45ad4f;
}

svg {
  width: 100%;
  height: 170px;
}

.monitor-workspace.compact svg {
  height: 86px;
}

.line {
  fill: none;
  stroke-width: 2;
}

.line.cpu {
  stroke: #93b4ff;
}

.line.memory {
  stroke: #80d4ca;
}

.line.upload {
  stroke: #ffb15f;
}

.line.download {
  stroke: #9dd69f;
}

.core-panel {
  display: grid;
  grid-template-columns: 26px 42px 1fr 68px;
  align-items: center;
  gap: 10px;
  padding: 22px 28px;
}

.monitor-workspace.compact .core-panel {
  grid-template-columns: 20px 34px 1fr 48px;
  gap: 8px;
  padding: 10px 12px;
}

.monitor-workspace.compact .core-strips {
  min-width: 0;
  gap: 4px;
}

.monitor-workspace.compact .core-strips span {
  height: 14px;
}

.monitor-workspace.compact .core-panel b {
  font-size: 15px;
}

.core-panel .el-icon {
  color: var(--muted);
}

.core-strips {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(4px, 1fr));
  gap: 7px;
  min-width: 240px;
}

.core-strips span {
  height: 20px;
  border-radius: 4px;
  background: #e5e7eb;
}

.core-strips span.warm {
  background: #45ad4f;
}

.core-strips span.hot {
  background: #e84a18;
}

.core-panel b {
  font-size: 20px;
  font-weight: 520;
  text-align: right;
}

.table-panel {
  overflow: hidden;
  padding: 16px 22px;
}

.table-panel header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 34px;
  margin-bottom: 14px;
}

.table-panel h3 {
  display: flex;
  align-items: center;
  gap: 7px;
  margin: 0;
  font-size: 16px;
  font-weight: 700;
}

.table-head,
.table-row {
  display: grid;
  align-items: center;
  gap: 10px;
  min-height: 40px;
}

.table-head {
  color: var(--faint);
  font-size: 13px;
}

.table-row {
  color: var(--text);
  font-size: 14px;
}

.table-row.process,
.table-head.process {
  grid-template-columns: 1.8fr 110px 100px 110px 54px;
}

.monitor-workspace.compact .table-row.process,
.monitor-workspace.compact .table-head.process {
  grid-template-columns: minmax(0, 1fr) 52px 50px 64px 28px;
  gap: 6px;
}

.table-row.network,
.table-head.network {
  grid-template-columns: 130px 170px 170px 1fr;
}

.monitor-workspace.compact .table-row.network,
.monitor-workspace.compact .table-head.network {
  grid-template-columns: 72px 104px 104px 64px;
}

.monitor-workspace.compact .table-row.network > :last-child,
.monitor-workspace.compact .table-head.network > :last-child {
  display: block;
}

.table-row.docker,
.table-head.docker {
  grid-template-columns: 1.2fr 110px 120px 1.5fr 90px;
}

.monitor-workspace.compact .table-row.docker,
.monitor-workspace.compact .table-head.docker {
  grid-template-columns: minmax(0, 1fr) 54px 52px 70px;
}

.monitor-workspace.compact .table-row.docker > :nth-child(4),
.monitor-workspace.compact .table-head.docker > :nth-child(4) {
  display: none;
}

.table-row.mount,
.table-head.mount {
  grid-template-columns: 1.2fr 220px 1fr;
}

.monitor-workspace.compact .table-row.mount,
.monitor-workspace.compact .table-head.mount {
  grid-template-columns: 80px 156px minmax(0, 1fr);
}

.monitor-workspace.compact .table-row.mount > :nth-child(3),
.monitor-workspace.compact .table-head.mount > :nth-child(3) {
  display: block;
}

.monitor-workspace.compact .usage-bar.mini {
  width: 148px;
}

.monitor-workspace.compact .table-panel {
  overflow: visible;
}

.monitor-workspace.compact .table-panel header {
  min-height: 26px;
  margin-bottom: 6px;
}

.monitor-workspace.compact .table-panel h3 {
  font-size: 13px;
}

.monitor-workspace.compact .table-head,
.monitor-workspace.compact .table-row {
  min-height: 30px;
  gap: 6px;
  font-size: 12px;
}

.monitor-workspace.compact .table-head {
  font-size: 11px;
}

.monitor-workspace.compact .pill {
  padding: 2px 7px;
}

.monitor-workspace.compact .table-row :deep(.el-button) {
  width: 22px;
  height: 24px;
  padding: 0;
}

.panel-select {
  width: 58px;
}

.panel-select :deep(.el-select__wrapper) {
  min-height: 26px;
  height: 26px;
  padding: 0 7px;
  border-radius: 7px;
  box-shadow: 0 0 0 1px var(--line) inset;
  background: #fff;
}

.panel-select :deep(.el-select__wrapper.is-focused) {
  box-shadow: 0 0 0 1px var(--line-strong) inset;
}

.panel-select :deep(.el-select__placeholder),
.panel-select :deep(.el-select__selected-item) {
  color: var(--text);
  font-size: 12px;
  font-weight: 520;
}

.panel-toggle {
  height: 26px;
  padding: 0 8px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: #fff;
  color: var(--muted);
  cursor: pointer;
  font-size: 12px;
  line-height: 24px;
}

.panel-toggle:hover {
  border-color: var(--line-strong);
  color: var(--text);
}

.panel-toggle.active {
  border-color: rgba(242, 107, 58, 0.35);
  background: var(--orange-soft);
  color: var(--orange);
}

.monitor-workspace.compact .panel + .panel {
  margin-top: 0;
}

.monitor-workspace.compact .host-item span,
.monitor-workspace.compact .metric-item span,
.monitor-workspace.compact .traffic-summary span {
  font-size: 11px;
}

.monitor-workspace.compact .host-item span {
  line-height: 14px;
}

.monitor-workspace.compact .host-item strong {
  line-height: 17px;
}

.monitor-workspace.compact .memory-item {
  min-width: 0;
}

.monitor-workspace.compact .table-row > span,
.monitor-workspace.compact .table-row > strong,
.monitor-workspace.compact .table-head > span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.monitor-workspace.compact .table-row b {
  overflow: hidden;
  font-size: 11px;
  line-height: 15px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.monitor-workspace.compact .row-actions {
  gap: 2px;
  justify-content: flex-start;
}

.terminal-action-icon {
  width: 16px;
  height: 16px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.35;
}

.pill {
  display: inline-block;
  max-width: 100%;
  overflow: hidden;
  padding: 4px 10px;
  border-radius: 999px;
  background: #e7e7e7;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pill.teal {
  background: #e3f2f1;
  color: #12a89e;
}

.table-row b {
  display: block;
  font-weight: 520;
}

.row-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.table-row :deep(.el-button) {
  color: #e13a12;
}

.row-actions :deep(.el-button + .el-button) {
  margin-left: 0;
}

.row-actions :deep(.el-button:first-child) {
  color: #36a24a;
}

.empty-line {
  min-height: 42px;
  color: var(--muted);
  font-size: 13px;
  line-height: 42px;
}

@media (min-width: 1440px) {
  .host-panel,
  .metrics-panel {
    grid-column: span 6;
  }

  .table-panel {
    grid-column: span 6;
  }
}
</style>
