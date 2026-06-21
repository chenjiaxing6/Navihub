<script setup>
import { DataAnalysis, Right } from "@element-plus/icons-vue";

defineProps({
  connectionName: { type: String, default: "" },
  databaseTarget: { type: String, required: true },
});
</script>

<template>
  <div class="empty-workspace">
    <div class="empty-workspace__panel">
      <div class="empty-workspace__visual">
        <el-icon><DataAnalysis /></el-icon>
      </div>
      <div class="empty-workspace__copy">
        <p>待打开</p>
        <strong>{{ connectionName || "选择一个数据库连接" }}</strong>
        <span>{{ databaseTarget }}</span>
      </div>
      <div class="empty-workspace__steps">
        <div class="empty-workspace__step active">
          <span>1</span>
          <strong>选择连接</strong>
        </div>
        <el-icon><Right /></el-icon>
        <div class="empty-workspace__step">
          <span>2</span>
          <strong>打开库</strong>
        </div>
        <el-icon><Right /></el-icon>
        <div class="empty-workspace__step">
          <span>3</span>
          <strong>查看表</strong>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.empty-workspace {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  min-width: 0;
  min-height: 0;
  padding: 26px;
  background:
    linear-gradient(90deg, rgba(242, 107, 58, 0.05), transparent 34%),
    linear-gradient(180deg, var(--surface-muted), #fff);
  color: var(--muted);
}

.empty-workspace__panel {
  display: grid;
  justify-items: center;
  width: min(520px, 100%);
  gap: 18px;
  text-align: center;
}

.empty-workspace__visual {
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

.empty-workspace__visual .el-icon {
  font-size: 28px;
}

.empty-workspace__copy {
  display: grid;
  gap: 6px;
}

.empty-workspace__copy p {
  margin: 0;
  color: var(--orange);
  font-size: 11px;
  font-weight: 780;
}

.empty-workspace__copy strong {
  color: var(--text);
  font-size: 20px;
  font-weight: 780;
}

.empty-workspace__copy span {
  color: var(--muted);
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
  font-size: 12px;
}

.empty-workspace__steps {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 16px minmax(0, 1fr) 16px minmax(0, 1fr);
  align-items: center;
  width: 100%;
  max-width: 430px;
  gap: 8px;
}

.empty-workspace__steps > .el-icon {
  color: var(--line-strong);
  font-size: 13px;
}

.empty-workspace__step {
  display: grid;
  justify-items: center;
  min-width: 0;
  gap: 7px;
}

.empty-workspace__step span {
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

.empty-workspace__step strong {
  max-width: 100%;
  overflow: hidden;
  color: var(--muted);
  font-size: 12px;
  font-weight: 650;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.empty-workspace__step.active span {
  border-color: #f5c5b3;
  background: var(--orange-soft);
  color: var(--orange);
}

.empty-workspace__step.active strong {
  color: var(--text);
}
</style>
