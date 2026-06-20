import { invoke } from "@tauri-apps/api/core";

export function loadMonitorSnapshot(target) {
  return invoke("monitor_snapshot", { target });
}

export function killMonitorProcess(target, pid) {
  return invoke("monitor_kill_process", { target, pid });
}

export function stopMonitorContainer(target, containerId) {
  return invoke("monitor_stop_container", { target, containerId });
}

export function startMonitorContainer(target, containerId) {
  return invoke("monitor_start_container", { target, containerId });
}

export function removeMonitorContainer(target, containerId) {
  return invoke("monitor_remove_container", { target, containerId });
}
