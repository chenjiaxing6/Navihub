<script setup>
import { nextTick, onBeforeUnmount, ref, watch } from "vue";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { getTerminalTheme } from "../settings/terminalThemes";
import "@xterm/xterm/css/xterm.css";

const props = defineProps({
  connection: { type: Object, default: null },
  error: { type: String, default: "" },
  output: { type: String, default: "" },
  state: { type: String, default: "idle" },
  terminalTheme: { type: Object, default: () => getTerminalTheme() },
});

const emit = defineEmits(["input", "resize"]);

const terminalElement = ref(null);
let terminal = null;
let fitAddon = null;
let dataDisposable = null;
let resizeObserver = null;
let resizeFrame = 0;
let lastTerminalSize = null;
let writtenLength = 0;

watch(
  () => terminalElement.value,
  () => {
    initTerminal();
  },
  { flush: "post" },
);

watch(
  () => props.connection?.id,
  () => {
    writtenLength = 0;
    terminal?.reset();
    writeStatusLine();
  },
);

watch(
  () => props.output,
  (output) => {
    if (!terminal || output.length < writtenLength) {
      writtenLength = 0;
      terminal?.clear();
    }

    const nextChunk = output.slice(writtenLength);
    if (nextChunk) {
      terminal.write(nextChunk);
      writtenLength = output.length;
    }
  },
);

watch(
  () => props.terminalTheme,
  (theme) => {
    if (terminal) {
      terminal.options.theme = { ...theme.theme };
    }
  },
  { deep: true },
);

watch(
  () => [props.state, props.error],
  () => {
    if (props.state === "connecting") {
      terminal?.reset();
      writtenLength = 0;
      terminal?.writeln("Connecting...");
    } else if (props.state === "connected" && writtenLength === 0) {
      terminal?.reset();
    } else if (props.state === "error" && props.error) {
      terminal?.writeln(`\r\n\x1b[31m${props.error}\x1b[0m`);
    } else if (props.state === "disconnected") {
      terminal?.writeln("\r\nDisconnected.");
    }
  },
);

onBeforeUnmount(() => {
  dataDisposable?.dispose();
  resizeObserver?.disconnect();
  if (resizeFrame) {
    cancelAnimationFrame(resizeFrame);
  }
  terminal?.dispose();
});

async function initTerminal() {
  if (!terminalElement.value || terminal) {
    return;
  }

  terminal = new Terminal({
    cursorBlink: true,
    fontFamily: "SFMono-Regular, Consolas, Liberation Mono, monospace",
    fontSize: 13,
    lineHeight: 1.35,
    theme: { ...props.terminalTheme.theme },
  });
  fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);
  terminal.open(terminalElement.value);
  dataDisposable = terminal.onData((data) => emit("input", data));
  resizeObserver = new ResizeObserver(() => fitTerminal());
  resizeObserver.observe(terminalElement.value);
  await nextTick();
  fitTerminal();
  if (props.output) {
    terminal.write(props.output);
    writtenLength = props.output.length;
  } else if (props.state === "connecting") {
    terminal.writeln("Connecting...");
  } else {
    writeStatusLine();
  }
}

function fitTerminal() {
  if (!terminal || !fitAddon) {
    return;
  }

  if (resizeFrame) {
    cancelAnimationFrame(resizeFrame);
  }

  resizeFrame = requestAnimationFrame(() => {
    resizeFrame = 0;
    fitAddon.fit();

    const size = { cols: terminal.cols, rows: terminal.rows };
    if (size.cols === lastTerminalSize?.cols && size.rows === lastTerminalSize?.rows) {
      return;
    }

    lastTerminalSize = size;
    emit("resize", size);
  });
}

function writeStatusLine() {
  if (!terminal || props.state === "connected") {
    return;
  }

  const title = props.connection?.name ?? "未选择主机";
  const hint = props.connection ? "等待连接" : "添加主机后双击连接";
  terminal.writeln(`\x1b[90m${title} · ${hint}\x1b[0m`);
}
</script>

<template>
  <section class="terminal-pane">
    <div ref="terminalElement" class="terminal-canvas" />
  </section>
</template>

<style scoped>
.terminal-pane {
  display: flex;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  flex-direction: column;
  border: 0;
  border-radius: 0;
  background: v-bind("props.terminalTheme.background");
}

.terminal-canvas {
  min-height: 0;
  flex: 1;
  overflow: hidden;
  padding: 8px;
}

.terminal-canvas :deep(.xterm) {
  height: 100%;
}

.terminal-canvas :deep(.xterm-viewport),
.terminal-canvas :deep(.xterm-screen) {
  height: 100%;
}
</style>
