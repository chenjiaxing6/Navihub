<script setup>
const localFiles = [
  { icon: "▣", name: "myhub-setup.dmg", meta: "184 MB", selected: true },
  { icon: "▤", name: "release-notes.md", meta: "12 KB" },
  { icon: "▧", name: "assets", meta: "28 项" },
];

const remoteFiles = [
  { icon: "▧", name: "images", meta: "146 项" },
  { icon: "▣", name: "myhub-setup.dmg", meta: "上传中", selected: true },
  { icon: "▤", name: "index.json", meta: "4 KB" },
];
</script>

<template>
  <section class="sftp-workspace">
    <section class="file-pane">
      <div class="file-path">本地 · ~/Downloads/release</div>
      <div
        v-for="file in localFiles"
        :key="file.name"
        class="file-row"
        :class="{ selected: file.selected }"
      >
        <span>{{ file.icon }}</span>
        <strong>{{ file.name }}</strong>
        <small>{{ file.meta }}</small>
      </div>
    </section>

    <section class="transfer-stack">
      <el-button>上传 →</el-button>
      <el-button>← 下载</el-button>
      <el-progress :percentage="68" :show-text="false" />
      <small>2 个任务 · 18.4 MB/s</small>
    </section>

    <section class="file-pane">
      <div class="file-path">远程 · /var/www/assets</div>
      <div
        v-for="file in remoteFiles"
        :key="file.name"
        class="file-row"
        :class="{ selected: file.selected }"
      >
        <span>{{ file.icon }}</span>
        <strong>{{ file.name }}</strong>
        <small>{{ file.meta }}</small>
      </div>
    </section>
  </section>
</template>

<style scoped>
.sftp-workspace {
  display: grid;
  grid-template-columns: 1fr 140px 1fr;
  gap: 8px;
  min-height: 0;
  flex: 1;
  padding: 10px;
  background: var(--panel);
}

.file-pane {
  min-width: 0;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: #fff;
  box-shadow: none;
}

.file-path {
  display: flex;
  align-items: center;
  height: 38px;
  padding: 0 12px;
  border-bottom: 1px solid var(--line);
  background: var(--surface-muted);
  color: var(--muted);
  font-size: 12px;
}

.file-row {
  display: grid;
  grid-template-columns: 28px 1fr auto;
  align-items: center;
  height: 34px;
  padding: 0 12px;
  border-bottom: 1px solid var(--line);
  color: var(--text);
  font-size: 13px;
}

.file-row small {
  color: var(--muted);
}

.file-row.selected {
  background: var(--blue-soft);
}

.file-row:hover {
  background: #fafafa;
}

.transfer-stack {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  justify-content: center;
  gap: 10px;
  color: var(--muted);
  text-align: center;
}

.transfer-stack :deep(.el-button) {
  margin: 0;
}
</style>
