import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export function connectSshTerminal(sessionId, config) {
  return invoke("ssh_connect", { sessionId, config });
}

export function writeSshTerminal(sessionId, data) {
  return invoke("ssh_write", { sessionId, data });
}

export function resizeSshTerminal(sessionId, cols, rows) {
  return invoke("ssh_resize", { sessionId, cols, rows });
}

export function disconnectSshTerminal(sessionId) {
  return invoke("ssh_disconnect", { sessionId });
}

export function testSshConnection(config) {
  return invoke("ssh_test_connection", { config });
}

export function connectSftpSession(sessionId, config) {
  return invoke("sftp_connect", { sessionId, config });
}

export function disconnectSftpSession(sessionId) {
  return invoke("sftp_disconnect", { sessionId });
}

export function listSftpDir(config, path, sessionId = null) {
  return invoke("sftp_list_dir", { config, path, sessionId });
}

export function getSftpRealpath(config, path, sessionId = null) {
  return invoke("sftp_realpath", { config, path, sessionId });
}

export function uploadSftpFile(config, path, fileName, data, sessionId = null) {
  return invoke("sftp_upload_file", { config, path, fileName, data, sessionId });
}

export function startSftpUpload(config, path, fileName, uploadId, sessionId = null) {
  return invoke("sftp_upload_file_start", { config, path, fileName, uploadId, sessionId });
}

export function writeSftpUpload(uploadId, data) {
  return invoke("sftp_upload_file_write", { uploadId, data });
}

export function finishSftpUpload(uploadId) {
  return invoke("sftp_upload_file_finish", { uploadId });
}

export function downloadSftpFile(config, path, sessionId = null) {
  return invoke("sftp_download_file", { config, path, sessionId });
}

export function startSftpDownload(config, path, downloadId, sessionId = null) {
  return invoke("sftp_download_file_start", { config, path, downloadId, sessionId });
}

export function readSftpDownload(downloadId, chunkSize) {
  return invoke("sftp_download_file_read", { downloadId, chunkSize });
}

export function finishSftpDownload(downloadId) {
  return invoke("sftp_download_file_finish", { downloadId });
}

export function cancelSftpDownload(downloadId) {
  return invoke("sftp_download_file_cancel", { downloadId });
}

export function deleteSftpPath(config, path, isDir, sessionId = null) {
  return invoke("sftp_delete_path", { config, path, isDir, sessionId });
}

export function createSftpDir(config, path, name, sessionId = null) {
  return invoke("sftp_create_dir", { config, path, name, sessionId });
}

export function renameSftpPath(config, oldPath, newName, sessionId = null) {
  return invoke("sftp_rename_path", { config, oldPath, newName, sessionId });
}

export function onSshTerminalEvent(handler) {
  return listen("ssh://terminal", (event) => handler(event.payload));
}
