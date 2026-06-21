import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import AutoImport from "unplugin-auto-import/vite";
import Components from "unplugin-vue-components/vite";
import { ElementPlusResolver } from "unplugin-vue-components/resolvers";

export default defineConfig({
  plugins: [
    vue(),
    AutoImport({
      resolvers: [ElementPlusResolver()],
      dts: false,
    }),
    Components({
      resolvers: [ElementPlusResolver()],
      dts: false,
    }),
  ],
  clearScreen: false,
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes("node_modules")) {
            return undefined;
          }

          if (id.includes("/@codemirror/") || id.includes("/codemirror/")) {
            return "vendor-codemirror";
          }

          if (id.includes("/@xterm/")) {
            return "vendor-xterm";
          }

          if (id.includes("/element-plus/") || id.includes("/@element-plus/")) {
            return "vendor-element-plus";
          }

          if (id.includes("/@vueuse/")) {
            return "vendor-vueuse";
          }

          if (id.includes("/dayjs/")) {
            return "vendor-dayjs";
          }

          if (
            id.includes("/lodash") ||
            id.includes("/async-validator/") ||
            id.includes("/@floating-ui/") ||
            id.includes("/memoize-one/")
          ) {
            return "vendor-utils";
          }

          if (id.includes("/@tauri-apps/")) {
            return "vendor-tauri";
          }

          if (id.includes("/vue/") || id.includes("/@vue/")) {
            return "vendor-vue";
          }

          return "vendor";
        },
      },
    },
  },
  server: {
    strictPort: true,
    host: "127.0.0.1",
    port: 1420,
  },
});
