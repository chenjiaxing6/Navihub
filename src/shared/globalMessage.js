import { ElMessage } from "element-plus/es/components/message/index";

const defaultPlacement = "top-right";
const messageTypes = ["primary", "success", "warning", "info", "error"];

function withDefaultPlacement(options) {
  if (options && typeof options === "object" && !Array.isArray(options)) {
    return { placement: defaultPlacement, ...options };
  }

  return {
    message: options,
    placement: defaultPlacement,
  };
}

function withClosableError(options) {
  const nextOptions = withDefaultPlacement(options);

  if (nextOptions && typeof nextOptions === "object" && !Array.isArray(nextOptions)) {
    return { showClose: true, ...nextOptions };
  }

  return {
    message: nextOptions,
    placement: defaultPlacement,
    showClose: true,
  };
}

export function setupGlobalMessage() {
  const showMessage = ElMessage.bind(ElMessage);
  const showTypedMessages = Object.fromEntries(
    messageTypes.map((type) => [type, ElMessage[type].bind(ElMessage)]),
  );

  const patchedMessage = (options, appContext) => showMessage(withDefaultPlacement(options), appContext);
  Object.assign(ElMessage, patchedMessage);

  for (const type of messageTypes) {
    ElMessage[type] = (options, appContext) => {
      const nextOptions = type === "error" ? withClosableError(options) : withDefaultPlacement(options);
      return showTypedMessages[type](nextOptions, appContext);
    };
  }
}
