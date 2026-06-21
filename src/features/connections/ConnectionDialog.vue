<script setup>
import { computed, reactive, ref, watch } from "vue";
import { ElMessage } from "element-plus/es/components/message/index";
import { testMysqlConnection } from "../database/mysqlApi";
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
    host: "127.0.0.1",
    port: 3306,
    username: "root",
    password: "",
    database: "",
    privateKey: "",
    remotePath: "",
  },
  ssh: {
    name: "new-host",
    host: "",
    port: 22,
    username: "root",
    password: "",
    database: "",
    privateKey: "",
    remotePath: "",
  },
};

const form = reactive({ ...defaultsByWorkspace.database });
const testing = ref(false);

const effectiveWorkspace = computed(() => props.connection?.workspace ?? props.workspace);
const isEditing = computed(() => Boolean(props.connection));
const isSsh = computed(() => effectiveWorkspace.value === "ssh");
const dialogTitle = computed(() => {
  if (isEditing.value) {
    return isSsh.value ? "编辑主机" : "编辑 MySQL 连接";
  }

  return isSsh.value ? "添加主机" : "新建 MySQL 连接";
});
const submitText = computed(() => isEditing.value ? "保存修改" : (isSsh.value ? "保存主机" : "保存连接"));

watch(
  () => [props.modelValue, effectiveWorkspace.value, props.connection?.id],
  ([visible]) => {
    if (!visible) {
      return;
    }

    Object.assign(form, getInitialForm());
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
    name: form.name || "new-mysql",
    config: getMysqlConfig(),
  });
}

function getInitialForm() {
  const defaults = defaultsByWorkspace[effectiveWorkspace.value] ?? defaultsByWorkspace.database;
  const config = props.connection?.config ?? {};

  return {
    ...defaults,
    name: props.connection?.name ?? defaults.name,
    host: config.host ?? defaults.host,
    port: config.port ?? defaults.port,
    username: config.username ?? defaults.username,
    password: config.password ?? defaults.password,
    database: config.database ?? defaults.database,
    privateKey: config.privateKey ?? defaults.privateKey,
    remotePath: config.remotePath ?? defaults.remotePath,
  };
}

async function handleTestConnection() {
  testing.value = true;

  try {
    const message = isSsh.value
      ? await testSshConnection(getSshConfig())
      : await testMysqlConnection(getMysqlConfig());
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

function getMysqlConfig() {
  return {
    host: form.host.trim() || "127.0.0.1",
    port: Number(form.port) || 3306,
    username: form.username.trim() || "root",
    password: form.password,
    database: form.database.trim(),
  };
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

    <el-form class="connection-form" :model="form" label-position="top">
      <el-form-item class="span-2" label="连接名">
        <el-input v-model="form.name" :placeholder="isSsh ? '例如：prod-web-01' : '例如：prod-mysql-01'" />
      </el-form-item>
      <el-form-item label="主机">
        <el-input v-model="form.host" :placeholder="isSsh ? '192.168.1.10' : '127.0.0.1'" />
      </el-form-item>
      <el-form-item label="端口">
        <el-input-number v-model="form.port" :min="1" :max="65535" controls-position="right" />
      </el-form-item>
      <el-form-item label="用户">
        <el-input v-model="form.username" placeholder="root" />
      </el-form-item>
      <el-form-item label="密码">
        <el-input v-model="form.password" type="password" show-password />
      </el-form-item>
      <el-form-item v-if="!isSsh" class="span-2" label="默认库">
        <el-input v-model="form.database" placeholder="可为空" />
      </el-form-item>
      <el-form-item v-if="isSsh" class="span-2" label="私钥">
        <el-input v-model="form.privateKey" placeholder="可为空，后续可接入密钥文件选择" />
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="handleClose">取消</el-button>
      <el-button :loading="testing" @click="handleTestConnection">测试连接</el-button>
      <el-button type="primary" @click="handleSubmit">{{ submitText }}</el-button>
    </template>
  </el-dialog>
</template>
