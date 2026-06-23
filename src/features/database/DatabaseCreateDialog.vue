<script setup>
import { computed, reactive, ref, watch } from "vue";
import { listMysqlDatabaseOptions } from "./mysqlAdminApi";

const props = defineProps({
  modelValue: { type: Boolean, default: false },
  config: { type: Object, default: null },
  database: { type: Object, default: null },
  loading: { type: Boolean, default: false },
  mode: { type: String, default: "create" },
});

const emit = defineEmits(["submit", "update:modelValue"]);

const form = reactive({
  database: "",
  charset: "utf8mb4",
  collation: "utf8mb4_unicode_ci",
});
const optionsLoading = ref(false);
const databaseOptions = ref({
  charsets: [],
  collations: [],
});

const visible = computed({
  get() {
    return props.modelValue;
  },
  set(value) {
    emit("update:modelValue", value);
  },
});
const isEditMode = computed(() => props.mode === "edit");
const dialogTitle = computed(() => isEditMode.value ? "编辑数据库" : "新建数据库");
const submitText = computed(() => isEditMode.value ? "保存修改" : "创建");

const fallbackCharsets = [
  { name: "utf8mb4", defaultCollation: "utf8mb4_unicode_ci", description: "UTF-8 Unicode" },
  { name: "utf8", defaultCollation: "utf8_general_ci", description: "UTF-8 Unicode" },
  { name: "latin1", defaultCollation: "latin1_swedish_ci", description: "cp1252 West European" },
  { name: "gbk", defaultCollation: "gbk_chinese_ci", description: "GBK Simplified Chinese" },
];
const fallbackCollations = [
  { name: "utf8mb4_unicode_ci", charset: "utf8mb4", isDefault: true },
  { name: "utf8mb4_general_ci", charset: "utf8mb4", isDefault: false },
  { name: "utf8mb4_0900_ai_ci", charset: "utf8mb4", isDefault: false },
  { name: "utf8mb4_bin", charset: "utf8mb4", isDefault: false },
  { name: "utf8_general_ci", charset: "utf8", isDefault: true },
  { name: "utf8_unicode_ci", charset: "utf8", isDefault: false },
  { name: "utf8_bin", charset: "utf8", isDefault: false },
  { name: "latin1_swedish_ci", charset: "latin1", isDefault: true },
  { name: "latin1_general_ci", charset: "latin1", isDefault: false },
  { name: "latin1_bin", charset: "latin1", isDefault: false },
  { name: "gbk_chinese_ci", charset: "gbk", isDefault: true },
  { name: "gbk_bin", charset: "gbk", isDefault: false },
];

const charsetOptions = computed(() => (
  databaseOptions.value.charsets.length > 0 ? databaseOptions.value.charsets : fallbackCharsets
));
const collationOptions = computed(() => {
  const source = databaseOptions.value.collations.length > 0 ? databaseOptions.value.collations : fallbackCollations;
  return source.filter((item) => item.charset === form.charset);
});

watch(
  () => props.modelValue,
  (value) => {
    if (!value) {
      return;
    }

    resetForm();
    loadOptions();
  },
);

watch(
  () => form.charset,
  (charset) => {
    const options = collationOptions.value;
    if (!options.some((item) => item.name === form.collation)) {
      form.collation = options.find((item) => item.isDefault)?.name ?? options[0]?.name ?? "";
    }
  },
);

function resetForm() {
  form.database = props.database?.name ?? "";
  const initialCollation = props.database?.collation ?? "";
  form.charset = initialCollation.split("_")?.[0] || "utf8mb4";
  form.collation = initialCollation || "utf8mb4_unicode_ci";
}

async function loadOptions() {
  if (!props.config) {
    return;
  }

  optionsLoading.value = true;
  try {
    databaseOptions.value = await listMysqlDatabaseOptions(props.config);
    if (!isEditMode.value || !form.collation) {
      const utf8mb4 = charsetOptions.value.find((item) => item.name === "utf8mb4") ?? charsetOptions.value[0];
      if (utf8mb4) {
        form.charset = utf8mb4.name;
        form.collation = utf8mb4.defaultCollation || collationOptions.value.find((item) => item.isDefault)?.name || collationOptions.value[0]?.name || "";
      }
    }
  } catch {
    databaseOptions.value = {
      charsets: fallbackCharsets,
      collations: fallbackCollations,
    };
  } finally {
    optionsLoading.value = false;
  }
}

function close() {
  visible.value = false;
}

function submit() {
  emit("submit", {
    database: form.database.trim(),
    charset: form.charset,
    collation: form.collation,
  });
}
</script>

<template>
  <el-dialog
    v-model="visible"
    class="database-object-dialog"
    custom-class="database-object-dialog"
    width="460px"
    append-to-body
    :close-on-click-modal="!loading"
    :close-on-press-escape="!loading"
    :show-close="false"
  >
    <template #header>
      <div class="database-object-dialog__header">
        <strong>{{ dialogTitle }}</strong>
        <button class="database-object-dialog__close" type="button" aria-label="关闭" :disabled="loading" @click="close">×</button>
      </div>
    </template>

    <el-form class="database-object-form" :model="form" label-position="top">
      <el-form-item label="数据库名称">
        <el-input
          v-model="form.database"
          :disabled="isEditMode"
          autofocus
          placeholder="例如：app_data"
          @keydown.enter.prevent="submit"
        />
      </el-form-item>
      <el-form-item label="字符集">
        <el-select
          v-model="form.charset"
          filterable
          :loading="optionsLoading"
          popper-class="database-object-select-popper"
        >
          <el-option
            v-for="item in charsetOptions"
            :key="item.name"
            :label="item.name"
            :value="item.name"
          >
            <span class="database-object-option">
              <strong>{{ item.name }}</strong>
              <small>{{ item.description }}</small>
            </span>
          </el-option>
        </el-select>
      </el-form-item>
      <el-form-item label="排序规则">
        <el-select
          v-model="form.collation"
          filterable
          :loading="optionsLoading"
          popper-class="database-object-select-popper"
        >
          <el-option
            v-for="item in collationOptions"
            :key="item.name"
            :label="item.name"
            :value="item.name"
          >
            <span class="database-object-option">
              <strong>{{ item.name }}</strong>
              <small>{{ item.isDefault ? '默认' : item.charset }}</small>
            </span>
          </el-option>
        </el-select>
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button :disabled="loading" @click="close">取消</el-button>
      <el-button type="primary" :loading="loading" @click="submit">{{ submitText }}</el-button>
    </template>
  </el-dialog>
</template>
