import { ElMessage } from "element-plus";

function withClosableError(options) {
  if (options && typeof options === "object" && !Array.isArray(options)) {
    return { showClose: true, ...options };
  }

  return {
    message: options,
    showClose: true,
  };
}

export function setupGlobalMessage() {
  const showError = ElMessage.error.bind(ElMessage);

  ElMessage.error = (options, appContext) => showError(withClosableError(options), appContext);
}
