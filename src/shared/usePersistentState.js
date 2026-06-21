import { ref, watch } from "vue";

export function readStoredValue(key, fallback, normalize = (value) => value) {
  try {
    const stored = JSON.parse(localStorage.getItem(key) || "null");
    return normalize(stored ?? fallback);
  } catch {
    return normalize(fallback);
  }
}

export function usePersistentState(key, fallback, normalize = (value) => value) {
  const state = ref(readStoredValue(key, fallback, normalize));

  watch(
    state,
    (value) => {
      localStorage.setItem(key, JSON.stringify(value));
    },
    { deep: true },
  );

  return state;
}
