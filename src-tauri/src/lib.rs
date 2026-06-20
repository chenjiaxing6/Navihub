mod database;
mod monitor;
mod ssh;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ssh::session::SshState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            database::mysql::mysql_test_connection,
            database::mysql::mysql_load_schema,
            database::mysql::mysql_execute_query,
            database::mysql::mysql_describe_table,
            monitor::system::monitor_snapshot,
            monitor::system::monitor_kill_process,
            monitor::system::monitor_stop_container,
            monitor::system::monitor_start_container,
            monitor::system::monitor_remove_container,
            ssh::session::ssh_connect,
            ssh::session::ssh_write,
            ssh::session::ssh_resize,
            ssh::session::ssh_disconnect,
            ssh::session::ssh_test_connection,
            ssh::session::sftp_connect,
            ssh::session::sftp_disconnect,
            ssh::session::sftp_list_dir,
            ssh::session::sftp_realpath,
            ssh::session::sftp_upload_file,
            ssh::session::sftp_upload_file_start,
            ssh::session::sftp_upload_file_write,
            ssh::session::sftp_upload_file_finish,
            ssh::session::sftp_download_file,
            ssh::session::sftp_download_file_start,
            ssh::session::sftp_download_file_read,
            ssh::session::sftp_download_file_finish,
            ssh::session::sftp_download_file_cancel,
            ssh::session::sftp_delete_path,
            ssh::session::sftp_create_dir,
            ssh::session::sftp_rename_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
