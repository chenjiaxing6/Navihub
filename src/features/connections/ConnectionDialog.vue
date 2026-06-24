<script setup>
import { computed, reactive, ref, watch } from "vue";
import { ElMessage } from "element-plus/es/components/message/index";
import { open, save } from "@tauri-apps/plugin-dialog";
import { testDatabaseConnection } from "../database/databaseApi";
import { MYSQL_LOGO_PATH, SQLITE_LOGO_CUT_PATH, SQLITE_LOGO_FEATHER_PATH, SQLITE_LOGO_LINE_PATH } from "../database/databaseLogos";
import { testSshConnection } from "../terminal/sshApi";

const props = defineProps({
  modelValue: { type: Boolean, required: true },
  workspace: { type: String, default: "database" },
  connection: { type: Object, default: null },
});

const emit = defineEmits(["update:modelValue", "submit"]);

const defaultsByWorkspace = {
  database: {
    name: "new-mysql",
    engine: "mysql",
    host: "127.0.0.1",
    port: 3306,
    username: "root",
    password: "",
    database: "",
    path: "",
    readOnly: false,
    privateKey: "",
    remotePath: "",
  },
  ssh: {
    name: "new-host",
    engine: "mysql",
    host: "",
    port: 22,
    username: "root",
    password: "",
    database: "",
    path: "",
    readOnly: false,
    privateKey: "",
    remotePath: "",
  },
};

const form = reactive({ ...defaultsByWorkspace.database });
const testing = ref(false);
const choosingDatabaseType = ref(false);

const effectiveWorkspace = computed(() => props.connection?.workspace ?? props.workspace);
const isEditing = computed(() => Boolean(props.connection));
const isSsh = computed(() => effectiveWorkspace.value === "ssh");
const isSqlite = computed(() => !isSsh.value && form.engine === "sqlite");
const showDatabaseTypeChooser = computed(() => choosingDatabaseType.value && !isEditing.value && !isSsh.value);
const dialogTitle = computed(() => {
  if (showDatabaseTypeChooser.value) {
    return "选择数据库类型";
  }

  if (isEditing.value) {
    return isSsh.value ? "编辑主机" : `编辑 ${isSqlite.value ? "SQLite" : "MySQL"} 连接`;
  }

  return isSsh.value ? "添加主机" : `新建 ${isSqlite.value ? "SQLite" : "MySQL"} 连接`;
});
const submitText = computed(() => isEditing.value ? "保存修改" : (isSsh.value ? "保存主机" : "保存连接"));

watch(
  () => [props.modelValue, effectiveWorkspace.value, props.connection?.id],
  ([visible]) => {
    if (!visible) {
      return;
    }

    Object.assign(form, getInitialForm());
    choosingDatabaseType.value = !props.connection && effectiveWorkspace.value === "database";
    testing.value = false;
  },
  { immediate: true },
);

function handleClose() {
  emit("update:modelValue", false);
}

function handleSubmit() {
  if (isSsh.value) {
    emit("submit", {
      workspace: "ssh",
      id: props.connection?.id,
      name: form.name.trim() || "new-host",
      config: getSshConfig(),
    });
    return;
  }

  emit("submit", {
    workspace: "database",
    id: props.connection?.id,
    name: form.name || (isSqlite.value ? "new-sqlite" : "new-mysql"),
    config: getDatabaseConfig(),
  });
}

function selectDatabaseEngine(engine) {
  form.engine = engine;
  if (engine === "sqlite") {
    if (!form.name || form.name === "new-mysql") {
      form.name = "new-sqlite";
    }
  } else if (!form.name || form.name === "new-sqlite") {
    form.name = "new-mysql";
  }
  choosingDatabaseType.value = false;
}

function backToDatabaseTypeChooser() {
  if (!isEditing.value && !isSsh.value) {
    choosingDatabaseType.value = true;
  }
}

function getInitialForm() {
  const defaults = defaultsByWorkspace[effectiveWorkspace.value] ?? defaultsByWorkspace.database;
  const config = props.connection?.config ?? {};

  return {
    ...defaults,
    name: props.connection?.name ?? (config.engine === "sqlite" ? "new-sqlite" : defaults.name),
    engine: config.engine ?? defaults.engine,
    host: config.host ?? defaults.host,
    port: config.port ?? defaults.port,
    username: config.username ?? defaults.username,
    password: config.password ?? defaults.password,
    database: config.database ?? defaults.database,
    path: config.path ?? defaults.path,
    readOnly: Boolean(config.readOnly ?? config.read_only ?? defaults.readOnly),
    privateKey: config.privateKey ?? defaults.privateKey,
    remotePath: config.remotePath ?? defaults.remotePath,
  };
}

async function handleTestConnection() {
  testing.value = true;

  try {
    const message = isSsh.value
      ? await testSshConnection(getSshConfig())
      : await testDatabaseConnection(getDatabaseConfig());
    ElMessage.success(message || "连接成功");
  } catch (error) {
    ElMessage.error(`连接失败：${error}`);
  } finally {
    testing.value = false;
  }
}

function getSshConfig() {
  return {
    host: form.host.trim(),
    port: Number(form.port) || 22,
    username: form.username.trim() || "root",
    password: form.password,
    privateKey: form.privateKey.trim(),
    remotePath: form.remotePath.trim(),
  };
}

function getDatabaseConfig() {
  if (isSqlite.value) {
    return {
      engine: "sqlite",
      path: form.path.trim(),
      readOnly: Boolean(form.readOnly),
    };
  }

  return {
    engine: "mysql",
    host: form.host.trim() || "127.0.0.1",
    port: Number(form.port) || 3306,
    username: form.username.trim() || "root",
    password: form.password,
    database: form.database.trim(),
  };
}

async function chooseSqliteFile() {
  const path = await open({
    title: "选择 SQLite 数据库文件",
    multiple: false,
    filters: [
      { name: "SQLite", extensions: ["db", "sqlite", "sqlite3"] },
      { name: "所有文件", extensions: ["*"] },
    ],
  });
  if (typeof path === "string") {
    form.path = path;
    if (!form.name || form.name === "new-sqlite") {
      form.name = path.split(/[\\/]/).pop()?.replace(/\.(db|sqlite3?|database)$/i, "") || "new-sqlite";
    }
  }
}

async function createSqliteFile() {
  const path = await save({
    title: "新建 SQLite 数据库文件",
    defaultPath: "database.sqlite",
    canCreateDirectories: true,
    filters: [{ name: "SQLite", extensions: ["sqlite", "db", "sqlite3"] }],
  });
  if (typeof path === "string") {
    form.path = path;
    form.readOnly = false;
    if (!form.name || form.name === "new-sqlite") {
      form.name = path.split(/[\\/]/).pop()?.replace(/\.(db|sqlite3?|database)$/i, "") || "new-sqlite";
    }
  }
}
</script>

<template>
  <el-dialog
    class="connection-dialog"
    custom-class="connection-dialog"
    :model-value="modelValue"
    :show-close="false"
    width="520px"
    append-to-body
    @update:model-value="emit('update:modelValue', $event)"
  >
    <template #header>
      <div class="connection-dialog__header">
        <strong>{{ dialogTitle }}</strong>
        <button class="connection-dialog__close" type="button" aria-label="关闭" @click="handleClose">×</button>
      </div>
    </template>

    <div v-if="showDatabaseTypeChooser" class="database-type-chooser">
      <button class="database-type-option mysql" type="button" @click="selectDatabaseEngine('mysql')">
        <span class="database-type-icon mysql-icon" aria-hidden="true">
          <svg class="mysql-official-logo" viewBox="0 0 32 32" focusable="false">
            <path :d="MYSQL_LOGO_PATH" />
          </svg>
        </span>
        <strong>MySQL</strong>
      </button>
      <button class="database-type-option sqlite" type="button" @click="selectDatabaseEngine('sqlite')">
        <span class="database-type-icon sqlite-icon" aria-hidden="true">
          <svg class="sqlite-official-logo" viewBox="0 0 96 96" focusable="false">
            <path class="sqlite-logo-feather" :d="SQLITE_LOGO_FEATHER_PATH" />
            <path class="sqlite-logo-cut" :d="SQLITE_LOGO_CUT_PATH" />
            <path class="sqlite-logo-line" :d="SQLITE_LOGO_LINE_PATH" />
          </svg>
        </span>
        <strong>SQLite</strong>
      </button>
    </div>

    <el-form v-else class="connection-form" :model="form" label-position="top">
      <div v-if="!isSsh && !isEditing" class="connection-type-inline span-2">
        <span>{{ isSqlite ? "SQLite" : "MySQL" }}</span>
        <button type="button" @click="backToDatabaseTypeChooser">更换类型</button>
      </div>
      <el-form-item class="span-2" label="连接名">
        <el-input v-model="form.name" :placeholder="isSsh ? '例如：prod-web-01' : isSqlite ? '例如：local-cache' : '例如：prod-mysql-01'" />
      </el-form-item>
      <template v-if="!isSsh && isSqlite">
        <el-form-item class="span-2" label="数据库文件">
          <div class="sqlite-path-row">
            <el-input v-model="form.path" placeholder="/path/to/database.sqlite" />
            <el-button @click="chooseSqliteFile">选择</el-button>
            <el-button @click="createSqliteFile">新建</el-button>
          </div>
        </el-form-item>
        <el-form-item class="span-2" label="打开方式">
          <el-checkbox v-model="form.readOnly">只读打开</el-checkbox>
        </el-form-item>
      </template>
      <el-form-item v-if="isSsh || !isSqlite" label="主机">
        <el-input v-model="form.host" :placeholder="isSsh ? '192.168.1.10' : '127.0.0.1'" />
      </el-form-item>
      <el-form-item v-if="isSsh || !isSqlite" label="端口">
        <el-input-number v-model="form.port" :min="1" :max="65535" controls-position="right" />
      </el-form-item>
      <el-form-item v-if="isSsh || !isSqlite" label="用户">
        <el-input v-model="form.username" placeholder="root" />
      </el-form-item>
      <el-form-item v-if="isSsh || !isSqlite" label="密码">
        <el-input v-model="form.password" type="password" show-password />
      </el-form-item>
      <el-form-item v-if="!isSsh && !isSqlite" class="span-2" label="默认库">
        <el-input v-model="form.database" placeholder="可为空" />
      </el-form-item>
      <el-form-item v-if="isSsh" class="span-2" label="私钥">
        <el-input v-model="form.privateKey" placeholder="可为空，后续可接入密钥文件选择" />
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="handleClose">取消</el-button>
      <template v-if="!showDatabaseTypeChooser">
        <el-button :loading="testing" @click="handleTestConnection">测试连接</el-button>
        <el-button type="primary" @click="handleSubmit">{{ submitText }}</el-button>
      </template>
    </template>
  </el-dialog>
</template>

<style scoped>
.database-type-chooser {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  padding: 4px 0 8px;
}

.database-type-option {
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 68px;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--panel);
  color: var(--text);
  cursor: pointer;
  text-align: left;
  box-shadow: var(--shadow-card);
}

.database-type-option:hover {
  border-color: var(--line-strong);
  background: var(--surface-muted);
}

.database-type-option:focus-visible {
  outline: 2px solid rgba(242, 107, 58, 0.28);
  outline-offset: 2px;
}

.database-type-option:active {
  background: var(--surface-strong);
  transform: none;
}

.database-type-option strong {
  min-width: 0;
  overflow: hidden;
  font-size: 14px;
  font-weight: 720;
  line-height: 1.2;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.database-type-icon {
  position: relative;
  display: grid;
  place-items: center;
  width: 42px;
  height: 42px;
  flex: 0 0 42px;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface-muted);
}

.mysql-official-logo {
  width: 31px;
  height: 31px;
  fill: #00758f;
}

.sqlite-official-logo {
  width: 34px;
  height: 34px;
}

.sqlite-logo-feather {
  fill: #0b80bd;
}

.sqlite-logo-cut {
  fill: #78c7e8;
  opacity: 0.95;
}

.sqlite-logo-line {
  fill: none;
  stroke: #f5fbff;
  stroke-linecap: round;
  stroke-width: 4;
}

.connection-type-inline {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 32px;
  padding: 0 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface-muted);
  color: var(--muted);
  font-size: 12px;
  font-weight: 650;
}

.connection-type-inline button {
  border: 0;
  background: transparent;
  color: var(--orange);
  cursor: pointer;
  font: inherit;
}

.sqlite-path-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto auto;
  gap: 8px;
  width: 100%;
}
</style>
