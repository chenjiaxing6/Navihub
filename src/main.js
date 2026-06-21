import { createApp } from "vue";
import "element-plus/es/components/message/style/css";
import "element-plus/es/components/message-box/style/css";
import "./styles/index.css";
import App from "./App.vue";
import { setupGlobalMessage } from "./shared/globalMessage";

setupGlobalMessage();
createApp(App).mount("#app");
