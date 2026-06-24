mod database;
mod monitor;
mod ssh;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ssh::session::SshState::default())
        .manage(database::mysql::MysqlState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            database::mysql::mysql_test_connection,
            database::mysql::mysql_load_schema,
            database::mysql::mysql_execute_query,
            database::mysql::mysql_describe_table,
            database::mysql_admin::mysql_create_database,
            database::mysql_admin::mysql_alter_database_options,
            database::mysql_admin::mysql_list_database_options,
            database::mysql_admin::mysql_drop_database,
            database::mysql_admin::mysql_create_table,
            database::mysql_admin::mysql_copy_table,
            database::mysql_admin::mysql_rename_table,
            database::mysql_admin::mysql_drop_table,
            database::mysql_admin::mysql_empty_table,
            database::mysql_admin::mysql_truncate_table,
            database::mysql_admin::mysql_export_tables_sql,
            database::mysql_admin::mysql_export_database_sql,
            database::mysql_admin::mysql_import_sql,
            database::sqlite::sqlite_test_connection,
            database::sqlite::sqlite_load_schema,
            database::sqlite::sqlite_execute_query,
            database::sqlite::sqlite_describe_table,
            database::sqlite_admin::sqlite_create_table,
            database::sqlite_admin::sqlite_copy_table,
            database::sqlite_admin::sqlite_rename_table,
            database::sqlite_admin::sqlite_drop_table,
            database::sqlite_admin::sqlite_empty_table,
            database::sqlite_admin::sqlite_export_tables_sql,
            database::sqlite_admin::sqlite_export_database_sql,
            database::sqlite_admin::sqlite_import_sql,
            database::sqlite_admin::sqlite_vacuum,
            database::sqlite_admin::sqlite_integrity_check,
            database::sqlite_admin::sqlite_analyze,
            database::sqlite_admin::sqlite_reindex,
            database::sqlite_admin::sqlite_database_info,
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
