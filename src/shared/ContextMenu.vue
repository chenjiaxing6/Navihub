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
  if (item.disabled || item.children?.length) {
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
      <component
        :is="item.children?.length ? 'div' : 'button'"
        v-for="item in visibleItems"
        :key="item.key"
        class="context-menu__item"
        :class="{
          'context-menu__item--danger': item.danger,
          'context-menu__item--divided': item.divided,
          'context-menu__item--submenu': item.children?.length,
          'context-menu__item--submenu-disabled': item.disabled && item.children?.length,
        }"
        :disabled="item.disabled"
        role="menuitem"
        @click="selectItem(item)"
      >
        <span>{{ item.label }}</span>
        <span v-if="item.children?.length" class="context-menu__arrow">›</span>
        <div v-if="item.children?.length" class="context-menu__submenu" role="menu">
          <button
            v-for="child in item.children.filter((childItem) => !childItem.hidden)"
            :key="child.key"
            class="context-menu__item"
            :class="{ 'context-menu__item--danger': child.danger, 'context-menu__item--divided': child.divided }"
            :disabled="child.disabled"
            role="menuitem"
            @click.stop="selectItem(child)"
          >
            <span>{{ child.label }}</span>
          </button>
        </div>
      </component>
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
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
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

.context-menu__arrow {
  color: var(--faint);
  font-size: 18px;
  line-height: 1;
}

.context-menu__submenu {
  display: none;
  position: absolute;
  top: -5px;
  left: 100%;
  min-width: 214px;
  padding: 5px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #fff;
  box-shadow: 0 8px 24px rgba(24, 27, 35, 0.10);
}

.context-menu__item--submenu::after {
  position: absolute;
  top: -8px;
  right: -14px;
  bottom: -8px;
  width: 18px;
  content: "";
}

.context-menu__item--submenu:hover .context-menu__submenu,
.context-menu__item--submenu:focus-within .context-menu__submenu {
  display: block;
}

.context-menu__item--submenu-disabled:hover .context-menu__submenu,
.context-menu__item--submenu-disabled:focus-within .context-menu__submenu {
  display: none;
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
