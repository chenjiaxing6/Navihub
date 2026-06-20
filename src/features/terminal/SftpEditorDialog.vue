<script setup>
import { computed, nextTick, onBeforeUnmount, ref, shallowRef, watch } from "vue";

const props = defineProps({
  modelValue: { type: Boolean, default: false },
  content: { type: String, default: "" },
  fileName: { type: String, default: "" },
  loading: { type: Boolean, default: false },
  path: { type: String, default: "" },
  saving: { type: Boolean, default: false },
});

const emit = defineEmits(["update:modelValue", "save"]);

const editorRoot = ref(null);
const editorView = shallowRef(null);
const editorLoading = ref(false);
const editorReady = ref(false);
const detectedLanguage = ref("");
const editorContent = ref("");
let editorModules = null;
let editorModulesPromise = null;
let languageCompartment = null;
let applyingExternalContent = false;

const dialogTitle = computed(() => (props.fileName ? `在线编辑 · ${props.fileName}` : "在线编辑"));
const hasChanges = computed(() => editorContent.value !== props.content);
const loadingEditor = computed(() => props.loading || props.saving || editorLoading.value);
const languageLabel = computed(() => detectedLanguage.value || "Plain Text");
const shouldEnhanceEditor = computed(() => !props.loading && editorContent.value.length <= 256 * 1024);

watch(
  () => props.modelValue,
  async (visible) => {
    if (!visible) {
      destroyEditor();
      return;
    }

    editorContent.value = props.content;
    detectedLanguage.value = detectLanguageName(props.fileName);
    await nextTick();
    if (shouldEnhanceEditor.value) {
      scheduleEditorEnhancement();
    }
  },
);

watch(
  () => props.content,
  (content) => {
    editorContent.value = content;
    if (props.loading) {
      return;
    }
    if (!editorView.value || editorView.value.state.doc.toString() === content) {
      return;
    }

    applyingExternalContent = true;
    editorView.value.dispatch({
      changes: { from: 0, to: editorView.value.state.doc.length, insert: content },
    });
    applyingExternalContent = false;
  },
);

watch(
  () => props.loading,
  async (loading) => {
    if (loading || !props.modelValue) {
      return;
    }

    editorContent.value = props.content;
    await nextTick();
    if (shouldEnhanceEditor.value) {
      scheduleEditorEnhancement();
    }
  },
);

watch(
  () => props.fileName,
  async () => {
    detectedLanguage.value = detectLanguageName(props.fileName);
    if (props.modelValue && editorView.value) {
      await applyLanguage();
    }
  },
);

onBeforeUnmount(() => {
  destroyEditor();
});

async function ensureEditorModules() {
  if (editorModules) {
    return editorModules;
  }

  editorModulesPromise ??= Promise.all([
    import("codemirror"),
    import("@codemirror/state"),
  ]).then(([codemirror, state]) => ({
    basicSetup: codemirror.basicSetup,
    EditorView: codemirror.EditorView,
    Compartment: state.Compartment,
  }));
  editorModules = await editorModulesPromise;
  return editorModules;
}

async function createEditor() {
  if (!editorRoot.value || editorView.value) {
    return;
  }

  editorLoading.value = true;
  await nextTick();
  try {
    if (!editorRoot.value || editorView.value) {
      return;
    }
    const { basicSetup, EditorView, Compartment } = await ensureEditorModules();
    languageCompartment ??= new Compartment();

    editorView.value = new EditorView({
      doc: editorContent.value,
      parent: editorRoot.value,
      extensions: [
        basicSetup,
        EditorView.lineWrapping,
        languageCompartment.of([]),
        EditorView.updateListener.of((update) => {
          if (update.docChanged && !applyingExternalContent) {
            editorContent.value = update.state.doc.toString();
          }
        }),
        EditorView.theme({
          "&": {
            height: "100%",
            fontSize: "12px",
            backgroundColor: "#fbfbfa",
          },
          ".cm-scroller": {
            fontFamily: '"SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace',
            lineHeight: "1.55",
          },
          ".cm-gutters": {
            backgroundColor: "#f4f3f1",
            borderRight: "1px solid #e6e1dc",
            color: "#8f8580",
          },
          ".cm-activeLine": {
            backgroundColor: "#fff4ed",
          },
          ".cm-activeLineGutter": {
            backgroundColor: "#f8e7dc",
          },
          ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": {
            backgroundColor: "#f5c7af",
          },
          "&.cm-focused": {
            outline: "none",
          },
        }),
      ],
    });

    editorReady.value = true;
  } finally {
    editorLoading.value = false;
  }
}

async function applyLanguage() {
  const language = languageLoaderForFile(props.fileName);
  detectedLanguage.value = language?.name ?? detectLanguageName(props.fileName);
  if (!editorView.value || !languageCompartment) {
    return;
  }

  let extension = [];
  if (language) {
    try {
      extension = await language.load();
    } catch {
      detectedLanguage.value = "";
    }
  }
  editorView.value.dispatch({
    effects: languageCompartment.reconfigure(extension),
  });
}

function destroyEditor() {
  editorView.value?.destroy();
  editorView.value = null;
  editorReady.value = false;
  editorLoading.value = false;
  detectedLanguage.value = "";
}

function scheduleEditorEnhancement() {
  window.setTimeout(async () => {
    if (!props.modelValue || props.loading || editorView.value || !editorRoot.value) {
      return;
    }

    await createEditor();
    await applyLanguage();
  }, 120);
}

function languageLoaderForFile(fileName) {
  const normalized = fileName.toLowerCase();
  const extension = normalized.includes(".") ? normalized.slice(normalized.lastIndexOf(".") + 1) : "";
  const baseName = normalized.slice(normalized.lastIndexOf("/") + 1);

  if (
    ["sh", "bash", "zsh", "fish", "ksh"].includes(extension) ||
    ["bashrc", "zshrc", "profile", "bash_profile", "bash_login", "zprofile", "zshenv", "zlogin"].includes(baseName.replace(/^\./, ""))
  ) {
    return {
      name: "Shell",
      load: async () => {
        const [{ StreamLanguage }, { shell }] = await Promise.all([
          import("@codemirror/language"),
          import("@codemirror/legacy-modes/mode/shell"),
        ]);
        return StreamLanguage.define(shell);
      },
    };
  }

  if (["js", "jsx", "mjs", "cjs", "ts", "tsx"].includes(extension)) {
    return {
      name: ["ts", "tsx"].includes(extension) ? "TypeScript" : "JavaScript",
      load: async () => {
        const { javascript } = await import("@codemirror/lang-javascript");
        return javascript({ typescript: ["ts", "tsx"].includes(extension), jsx: ["jsx", "tsx"].includes(extension) });
      },
    };
  }

  const loaders = {
    json: { name: "JSON", load: async () => (await import("@codemirror/lang-json")).json() },
    css: { name: "CSS", load: async () => (await import("@codemirror/lang-css")).css() },
    scss: { name: "Sass", load: async () => (await import("@codemirror/lang-sass")).sass({ indented: false }) },
    sass: { name: "Sass", load: async () => (await import("@codemirror/lang-sass")).sass({ indented: true }) },
    html: { name: "HTML", load: async () => (await import("@codemirror/lang-html")).html() },
    htm: { name: "HTML", load: async () => (await import("@codemirror/lang-html")).html() },
    md: { name: "Markdown", load: async () => (await import("@codemirror/lang-markdown")).markdown() },
    markdown: { name: "Markdown", load: async () => (await import("@codemirror/lang-markdown")).markdown() },
    py: { name: "Python", load: async () => (await import("@codemirror/lang-python")).python() },
    sql: { name: "SQL", load: async () => (await import("@codemirror/lang-sql")).sql() },
    xml: { name: "XML", load: async () => (await import("@codemirror/lang-xml")).xml() },
    svg: { name: "XML", load: async () => (await import("@codemirror/lang-xml")).xml() },
    yaml: { name: "YAML", load: async () => (await import("@codemirror/lang-yaml")).yaml() },
    yml: { name: "YAML", load: async () => (await import("@codemirror/lang-yaml")).yaml() },
    vue: { name: "Vue", load: async () => (await import("@codemirror/lang-vue")).vue() },
    php: { name: "PHP", load: async () => (await import("@codemirror/lang-php")).php() },
    java: { name: "Java", load: async () => (await import("@codemirror/lang-java")).java() },
    c: { name: "C/C++", load: async () => (await import("@codemirror/lang-cpp")).cpp() },
    h: { name: "C/C++", load: async () => (await import("@codemirror/lang-cpp")).cpp() },
    cpp: { name: "C/C++", load: async () => (await import("@codemirror/lang-cpp")).cpp() },
    cc: { name: "C/C++", load: async () => (await import("@codemirror/lang-cpp")).cpp() },
    cxx: { name: "C/C++", load: async () => (await import("@codemirror/lang-cpp")).cpp() },
    rs: { name: "Rust", load: async () => (await import("@codemirror/lang-rust")).rust() },
    go: { name: "Go", load: async () => (await import("@codemirror/lang-go")).go() },
  };

  if (normalized === "dockerfile") {
    return {
      name: "Dockerfile",
      load: async () => {
        const [{ StreamLanguage }, { dockerFile }] = await Promise.all([
          import("@codemirror/language"),
          import("@codemirror/legacy-modes/mode/dockerfile"),
        ]);
        return StreamLanguage.define(dockerFile);
      },
    };
  }

  return loaders[extension] ?? null;
}

function detectLanguageName(fileName) {
  return languageLoaderForFile(fileName)?.name ?? "";
}

function closeDialog() {
  emit("update:modelValue", false);
}

function saveContent() {
  emit("save", editorContent.value);
}
</script>

<template>
  <el-dialog
    class="sftp-editor-dialog"
    :model-value="modelValue"
    :title="dialogTitle"
    width="min(1040px, calc(100vw - 48px))"
    :close-on-click-modal="false"
    destroy-on-close
    @update:model-value="emit('update:modelValue', $event)"
  >
    <div class="editor-shell">
      <div v-if="loadingEditor" class="editor-loading-bar" aria-hidden="true">
        <span />
      </div>
      <header class="editor-meta">
        <span class="editor-path">{{ path }}</span>
        <span class="editor-language">{{ languageLabel }}</span>
      </header>
      <div class="editor-body">
        <textarea
          v-model="editorContent"
          class="editor-fallback"
          :class="{ 'editor-fallback--hidden': editorReady }"
          :disabled="loading"
          spellcheck="false"
        />
        <div ref="editorRoot" class="editor-host" :class="{ 'editor-host--hidden': !editorReady }" />
        <div v-if="loading" class="editor-loading-cover">
          <span>正在加载远程文件...</span>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="editor-actions">
        <el-button :disabled="saving" @click="closeDialog">取消</el-button>
        <el-button type="primary" :loading="saving" :disabled="loading || !hasChanges" @click="saveContent">保存</el-button>
      </div>
    </template>
  </el-dialog>
</template>

<style scoped>
.editor-shell {
  position: relative;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #fbfbfa;
}

.editor-loading-bar {
  position: absolute;
  z-index: 3;
  top: 0;
  right: 8px;
  left: 8px;
  height: 2px;
  overflow: hidden;
  border-radius: 999px;
  background: transparent;
}

.editor-loading-bar span {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 28%;
  border-radius: inherit;
  background: #1f2937;
  animation: editor-loading 1.05s ease-in-out infinite;
}

.editor-meta {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 12px;
  align-items: center;
  min-height: 34px;
  padding: 0 10px;
  border-bottom: 1px solid var(--line);
  background: #fff;
  color: var(--muted);
  font-size: 12px;
}

.editor-path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.editor-language {
  color: var(--text);
  font-weight: 650;
}

.editor-body {
  position: relative;
}

.editor-host,
.editor-fallback {
  height: min(62vh, 620px);
  min-height: 380px;
}

.editor-host {
  position: relative;
  z-index: 1;
}

.editor-host--hidden {
  position: absolute;
  inset: 0;
  z-index: 0;
  visibility: hidden;
  pointer-events: none;
}

.editor-fallback {
  position: relative;
  z-index: 1;
  width: 100%;
  min-width: 0;
  resize: none;
  padding: 10px 12px;
  border: 0;
  outline: none;
  background: #fbfbfa;
  color: var(--text);
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
  font-size: 12px;
  line-height: 1.55;
}

.editor-fallback--hidden {
  position: absolute;
  inset: 0;
  z-index: 0;
  visibility: hidden;
  pointer-events: none;
}

.editor-loading-cover {
  position: absolute;
  inset: 0;
  z-index: 2;
  display: grid;
  place-items: center;
  background: rgba(255, 255, 255, 0.72);
  color: var(--muted);
  font-size: 13px;
  font-weight: 650;
}

.editor-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.editor-actions :deep(.el-button) {
  height: 31px;
  min-width: 70px;
  padding: 0 13px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #fff;
  color: var(--muted);
  font-size: 12px;
  font-weight: 650;
}

.editor-actions :deep(.el-button:hover) {
  border-color: var(--line-strong);
  background: var(--surface-strong);
  color: var(--text);
}

.editor-actions :deep(.el-button--primary) {
  border-color: var(--orange);
  background: var(--orange);
  color: #fff;
}

.editor-actions :deep(.el-button--primary:hover) {
  border-color: #e65d2e;
  background: #e65d2e;
  color: #fff;
}

.editor-actions :deep(.el-button.is-disabled),
.editor-actions :deep(.el-button.is-disabled:hover) {
  opacity: 0.58;
}

@keyframes editor-loading {
  0% {
    transform: translateX(-120%);
  }

  100% {
    transform: translateX(460%);
  }
}
</style>
