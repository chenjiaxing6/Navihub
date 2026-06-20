<script setup>
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";

const props = defineProps({
  items: { type: Array, default: () => [] },
  modelValue: { type: Boolean, default: false },
  x: { type: Number, default: 0 },
  y: { type: Number, default: 0 },
});

const emit = defineEmits(["select", "update:modelValue"]);
const menuRef = ref(null);
const position = ref({ left: 0, top: 0 });

const visibleItems = computed(() => props.items.filter((item) => !item.hidden));

watch(
  () => [props.modelValue, props.x, props.y, visibleItems.value.length],
  async () => {
    if (!props.modelValue) {
      return;
    }

    position.value = { left: props.x, top: props.y };
    await nextTick();
    clampToViewport();
  },
  { immediate: true },
);

function clampToViewport() {
  const menu = menuRef.value;
  if (!menu) {
    return;
  }

  const gap = 8;
  const rect = menu.getBoundingClientRect();
  position.value = {
    left: Math.min(props.x, window.innerWidth - rect.width - gap),
    top: Math.min(props.y, window.innerHeight - rect.height - gap),
  };
}

function close() {
  emit("update:modelValue", false);
}

function selectItem(item) {
  if (item.disabled) {
    return;
  }

  emit("select", item);
  close();
}

function handleDocumentPointerDown(event) {
  if (!props.modelValue || menuRef.value?.contains(event.target)) {
    return;
  }

  close();
}

function handleKeydown(event) {
  if (!props.modelValue || event.key !== "Escape") {
    return;
  }

  close();
}

window.addEventListener("resize", close);
document.addEventListener("pointerdown", handleDocumentPointerDown);
document.addEventListener("keydown", handleKeydown);

onBeforeUnmount(() => {
  window.removeEventListener("resize", close);
  document.removeEventListener("pointerdown", handleDocumentPointerDown);
  document.removeEventListener("keydown", handleKeydown);
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="modelValue"
      ref="menuRef"
      class="context-menu"
      :style="{ left: `${position.left}px`, top: `${position.top}px` }"
      role="menu"
      @contextmenu.prevent
    >
      <button
        v-for="item in visibleItems"
        :key="item.key"
        class="context-menu__item"
        :class="{ 'context-menu__item--danger': item.danger, 'context-menu__item--divided': item.divided }"
        :disabled="item.disabled"
        role="menuitem"
        @click="selectItem(item)"
      >
        {{ item.label }}
      </button>
    </div>
  </Teleport>
</template>

<style scoped>
.context-menu {
  position: fixed;
  z-index: 3000;
  min-width: 132px;
  padding: 5px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #fff;
  box-shadow: 0 8px 24px rgba(24, 27, 35, 0.10);
}

.context-menu__item {
  display: flex;
  align-items: center;
  width: 100%;
  min-height: 30px;
  padding: 0 9px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  font: inherit;
  font-size: 12px;
  font-weight: 400;
  text-align: left;
  appearance: none;
}

.context-menu__item--divided {
  margin-top: 5px;
  border-top: 1px solid var(--line);
  border-radius: 0 0 7px 7px;
}

.context-menu__item--divided::before {
  display: none;
}

.context-menu__item:hover:not(:disabled) {
  background: var(--surface-strong);
  color: var(--text);
}

.context-menu__item--danger:hover:not(:disabled) {
  background: #fef0f0;
  color: var(--red);
}

.context-menu__item:active:not(:disabled) {
  background: #e8e9ec;
}

.context-menu__item:disabled {
  color: var(--faint);
  cursor: default;
}
</style>
