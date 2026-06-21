<script setup>
import { ref } from "vue";
import { Close, Search } from "@element-plus/icons-vue";

defineProps({
  modelValue: { type: String, default: "" },
  placeholder: { type: String, required: true },
  matchedCount: { type: Number, required: true },
  totalCount: { type: Number, required: true },
});

const emit = defineEmits(["update:modelValue", "close", "keydown"]);
const inputRef = ref(null);

defineExpose({
  focus: () => inputRef.value?.focus(),
  select: () => inputRef.value?.select(),
});
</script>

<template>
  <div class="table-search">
    <el-icon><Search /></el-icon>
    <input
      ref="inputRef"
      :value="modelValue"
      type="search"
      :placeholder="placeholder"
      @input="emit('update:modelValue', $event.target.value)"
      @keydown="emit('keydown', $event)"
    />
    <span>{{ matchedCount }} / {{ totalCount }}</span>
    <button type="button" aria-label="关闭搜索" @click="emit('close')">
      <el-icon><Close /></el-icon>
    </button>
  </div>
</template>

<style scoped>
.table-search {
  display: grid;
  grid-template-columns: 18px minmax(0, 1fr) auto 28px;
  gap: 7px;
  align-items: center;
  min-height: 38px;
  padding: 5px 7px;
  border-bottom: 1px solid var(--line);
  background: #fff;
  color: var(--muted);
  font-size: 12px;
}

.table-search input {
  width: 100%;
  height: 28px;
  min-width: 0;
  padding: 0 9px;
  border: 1px solid var(--line);
  border-radius: 8px;
  outline: none;
  background: #fff;
  color: var(--text);
  font: inherit;
}

.table-search input:focus {
  border-color: var(--orange);
  box-shadow: 0 0 0 3px rgba(242, 107, 58, 0.13);
}

.table-search span {
  color: var(--faint);
  white-space: nowrap;
}

.table-search button {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #fff;
  color: var(--muted);
  cursor: pointer;
}

.table-search button:hover {
  border-color: var(--line-strong);
  background: var(--surface-strong);
  color: var(--text);
}
</style>
