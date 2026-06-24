use serde::{Deserialize, Serialize};
use ssh2::{BlockDirections, FileStat, OpenFlags, OpenType, RenameFlags, Session};
use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, State};

const SSH_RETRY_DELAY: Duration = Duration::from_millis(20);
const SSH_RETRY_ATTEMPTS: u32 = 500;
const SSH_POLL_TIMEOUT_MS: i32 = 25;
const SSH_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone, Default)]
pub struct SshState {
    sessions: Arc<Mutex<HashMap<String, SshSessionHandle>>>,
    sftp_sessions: Arc<Mutex<HashMap<String, SftpSessionHandle>>>,
    sftp_uploads: Arc<Mutex<HashMap<String, SftpUploadHandle>>>,
    sftp_downloads: Arc<Mutex<HashMap<String, SftpDownloadHandle>>>,
    pending_disconnects: Arc<Mutex<HashSet<String>>>,
}

struct SshSessionHandle {
    sender: mpsc::Sender<SshIoCommand>,
}

struct SftpSessionHandle {
    session: Session,
    sftp: ssh2::Sftp,
    _socket: TcpStream,
}

struct SftpUploadHandle {
    file: ssh2::File,
}

struct SftpDownloadHandle {
    receiver: mpsc::Receiver<Result<Vec<u8>, String>>,
    canceled: Arc<AtomicBool>,
    finished: bool,
}

enum SshIoCommand {
    Write(String),
    Resize { cols: u32, rows: u32 },
    Disconnect,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalEvent {
    pub session_id: String,
    pub kind: String,
    pub data: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpEntry {
    pub name: String,
    pub size: Option<u64>,
    pub folder: bool,
    pub permissions: Option<u32>,
    pub modified_time: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpDownloadChunk {
    pub data: Vec<u8>,
    pub done: bool,
}

#[tauri::command]
pub fn ssh_connect(
    app: AppHandle,
    state: State<'_, SshState>,
    session_id: String,
    config: SshConfig,
) -> Result<(), String> {
    let state = state.inner().clone();
    state
        .pending_disconnects
        .lock()
        .map_err(|_| "SSH session state is unavailable".to_string())?
        .remove(&session_id);

    thread::spawn(move || {
        if let Err(error) = connect_terminal_session(app.clone(), state, session_id.clone(), config)
        {
            emit_terminal(&app, &session_id, "error", &error);
        }
    });

    Ok(())
}

#[tauri::command]
pub fn ssh_write(
    state: State<'_, SshState>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    let sender = {
        let sessions = state
            .sessions
            .lock()
            .map_err(|_| "SSH session state is unavailable".to_string())?;
        sessions
            .get(&session_id)
            .map(|session| session.sender.clone())
    }
    .ok_or_else(|| "SSH session is not connected".to_string())?;

    sender
        .send(SshIoCommand::Write(data))
        .map_err(|_| "SSH session is not connected".to_string())
}

#[tauri::command]
pub fn ssh_resize(
    state: State<'_, SshState>,
    session_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    let sender = {
        let sessions = state
            .sessions
            .lock()
            .map_err(|_| "SSH session state is unavailable".to_string())?;
        sessions
            .get(&session_id)
            .map(|session| session.sender.clone())
    }
    .ok_or_else(|| "SSH session is not connected".to_string())?;

    sender
        .send(SshIoCommand::Resize { cols, rows })
        .map_err(|_| "SSH session is not connected".to_string())
}

#[tauri::command]
pub fn ssh_disconnect(state: State<'_, SshState>, session_id: String) -> Result<(), String> {
    state
        .pending_disconnects
        .lock()
        .map_err(|_| "SSH session state is unavailable".to_string())?
        .insert(session_id.clone());

    let session = state
        .sessions
        .lock()
        .map_err(|_| "SSH session state is unavailable".to_string())?
        .remove(&session_id);

    if let Some(session) = session {
        let _ = session.sender.send(SshIoCommand::Disconnect);
    }

    Ok(())
}

#[tauri::command]
pub fn ssh_test_connection(config: SshConfig) -> Result<String, String> {
    let (session, _) = connect_session(&config)?;
    session
        .disconnect(None, "test complete", None)
        .map_err(to_string)?;
    Ok("连接成功".to_string())
}

#[tauri::command]
pub fn sftp_connect(
    state: State<'_, SshState>,
    session_id: String,
    config: SshConfig,
) -> Result<(), String> {
    let (session, socket) = connect_session(&config)?;
    let sftp = session.sftp().map_err(to_string)?;
    state
        .sftp_sessions
        .lock()
        .map_err(|_| "SFTP session state is unavailable".to_string())?
        .insert(
            session_id,
            SftpSessionHandle {
                session,
                sftp,
                _socket: socket,
            },
        );

    Ok(())
}

#[tauri::command]
pub fn sftp_disconnect(state: State<'_, SshState>, session_id: String) -> Result<(), String> {
    let session = state
        .sftp_sessions
        .lock()
        .map_err(|_| "SFTP session state is unavailable".to_string())?
        .remove(&session_id);

    if let Some(handle) = session {
        let _ = handle.session.disconnect(None, "sftp closed", None);
    }

    Ok(())
}

#[tauri::command]
pub fn sftp_list_dir(
    state: State<'_, SshState>,
    session_id: Option<String>,
    config: SshConfig,
    path: String,
) -> Result<Vec<SftpEntry>, String> {
    if let Some(session_id) = session_id {
        return with_sftp_session(&state, &session_id, |sftp| list_sftp_entries(sftp, &path));
    }

    let (session, _) = connect_session(&config)?;
    let sftp = session.sftp().map_err(to_string)?;
    list_sftp_entries(&sftp, &path)
}

#[tauri::command]
pub fn sftp_realpath(
    state: State<'_, SshState>,
    session_id: Option<String>,
    config: SshConfig,
    path: String,
) -> Result<String, String> {
    if let Some(session_id) = session_id {
        return with_sftp_session(&state, &session_id, |sftp| realpath_sftp(sftp, &path));
    }

    let (session, _) = connect_session(&config)?;
    let sftp = session.sftp().map_err(to_string)?;
    realpath_sftp(&sftp, &path)
}

fn realpath_sftp(sftp: &ssh2::Sftp, path: &str) -> Result<String, String> {
    sftp.realpath(Path::new(path))
        .map_err(to_string)?
        .to_str()
        .map(|value| value.to_string())
        .ok_or_else(|| "Remote path is not valid UTF-8".to_string())
}

fn list_sftp_entries(sftp: &ssh2::Sftp, path: &str) -> Result<Vec<SftpEntry>, String> {
    let mut entries = Vec::new();

    for item in sftp.readdir(Path::new(&path)).map_err(to_string)? {
        let (path, stat) = item;
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        entries.push(SftpEntry {
            name: name.to_string(),
            size: stat.size,
            folder: is_directory(&stat),
            permissions: stat.perm,
            modified_time: stat.mtime,
        });
    }

    entries.sort_by(|left, right| {
        right
            .folder
            .cmp(&left.folder)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(entries)
}

#[tauri::command]
pub fn sftp_upload_file(
    state: State<'_, SshState>,
    session_id: Option<String>,
    config: SshConfig,
    path: String,
    file_name: String,
    data: Vec<u8>,
) -> Result<(), String> {
    if file_name.contains('/') || file_name == "." || file_name == ".." {
        return Err("Invalid file name".to_string());
    }

    if let Some(session_id) = session_id {
        return with_sftp_session(&state, &session_id, |sftp| {
            upload_sftp_file(sftp, &path, &file_name, &data)
        });
    }

    let (session, _) = connect_session(&config)?;
    let sftp = session.sftp().map_err(to_string)?;
    upload_sftp_file(&sftp, &path, &file_name, &data)
}

fn upload_sftp_file(
    sftp: &ssh2::Sftp,
    path: &str,
    file_name: &str,
    data: &[u8],
) -> Result<(), String> {
    let remote_path = join_remote_path(&path, &file_name);
    let mut remote_file = sftp
        .open_mode(
            &remote_path,
            OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
            0o644,
            OpenType::File,
        )
        .map_err(to_string)?;

    remote_file.write_all(data).map_err(to_string)
}

#[tauri::command]
pub fn sftp_upload_file_start(
    state: State<'_, SshState>,
    session_id: Option<String>,
    config: SshConfig,
    upload_id: String,
    path: String,
    file_name: String,
) -> Result<(), String> {
    if file_name.contains('/') || file_name == "." || file_name == ".." {
        return Err("Invalid file name".to_string());
    }

    let remote_path = join_remote_path(&path, &file_name);
    let file = if let Some(session_id) = session_id {
        with_sftp_session(&state, &session_id, |sftp| {
            sftp.open_mode(
                &remote_path,
                OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
                0o644,
                OpenType::File,
            )
            .map_err(to_string)
        })?
    } else {
        let (session, _) = connect_session(&config)?;
        let sftp = session.sftp().map_err(to_string)?;
        sftp.open_mode(
            &remote_path,
            OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
            0o644,
            OpenType::File,
        )
        .map_err(to_string)?
    };

    state
        .sftp_uploads
        .lock()
        .map_err(|_| "SFTP upload state is unavailable".to_string())?
        .insert(upload_id, SftpUploadHandle { file });
    Ok(())
}

#[tauri::command]
pub fn sftp_upload_file_write(
    state: State<'_, SshState>,
    upload_id: String,
    data: Vec<u8>,
) -> Result<(), String> {
    let mut uploads = state
        .sftp_uploads
        .lock()
        .map_err(|_| "SFTP upload state is unavailable".to_string())?;
    let handle = uploads
        .get_mut(&upload_id)
        .ok_or_else(|| "SFTP upload is not active".to_string())?;
    handle.file.write_all(&data).map_err(to_string)
}

#[tauri::command]
pub fn sftp_upload_file_finish(
    state: State<'_, SshState>,
    upload_id: String,
) -> Result<(), String> {
    let mut uploads = state
        .sftp_uploads
        .lock()
        .map_err(|_| "SFTP upload state is unavailable".to_string())?;
    if let Some(mut handle) = uploads.remove(&upload_id) {
        handle.file.flush().map_err(to_string)?;
    }
    Ok(())
}

#[tauri::command]
pub fn sftp_download_file(
    state: State<'_, SshState>,
    session_id: Option<String>,
    config: SshConfig,
    path: String,
) -> Result<Vec<u8>, String> {
    if let Some(session_id) = session_id {
        return with_sftp_session(&state, &session_id, |sftp| download_sftp_file(sftp, &path));
    }

    let (session, _) = connect_session(&config)?;
    let sftp = session.sftp().map_err(to_string)?;
    download_sftp_file(&sftp, &path)
}

fn download_sftp_file(sftp: &ssh2::Sftp, path: &str) -> Result<Vec<u8>, String> {
    let mut remote_file = sftp.open(Path::new(&path)).map_err(to_string)?;
    let mut data = Vec::new();
    remote_file.read_to_end(&mut data).map_err(to_string)?;
    Ok(data)
}

#[tauri::command]
pub fn sftp_download_file_start(
    state: State<'_, SshState>,
    session_id: Option<String>,
    config: SshConfig,
    download_id: String,
    path: String,
) -> Result<(), String> {
    let mut file = if let Some(session_id) = session_id {
        with_sftp_session(&state, &session_id, |sftp| {
            sftp.open(Path::new(&path)).map_err(to_string)
        })?
    } else {
        let (session, _) = connect_session(&config)?;
        let sftp = session.sftp().map_err(to_string)?;
        sftp.open(Path::new(&path)).map_err(to_string)?
    };
    let (sender, receiver) = mpsc::channel();
    let canceled = Arc::new(AtomicBool::new(false));
    let worker_canceled = canceled.clone();

    thread::spawn(move || {
        let mut buffer = vec![0; 32 * 1024];
        while !worker_canceled.load(Ordering::Relaxed) {
            match file.read(&mut buffer) {
                Ok(0) => {
                    let _ = sender.send(Ok(Vec::new()));
                    return;
                }
                Ok(bytes_read) => {
                    if sender.send(Ok(buffer[..bytes_read].to_vec())).is_err() {
                        return;
                    }
                }
                Err(error) => {
                    let _ = sender.send(Err(to_string(error)));
                    return;
                }
            }
        }
    });

    state
        .sftp_downloads
        .lock()
        .map_err(|_| "SFTP download state is unavailable".to_string())?
        .insert(
            download_id,
            SftpDownloadHandle {
                receiver,
                canceled,
                finished: false,
            },
        );
    Ok(())
}

#[tauri::command]
pub fn sftp_download_file_read(
    state: State<'_, SshState>,
    download_id: String,
    _chunk_size: usize,
) -> Result<SftpDownloadChunk, String> {
    let mut downloads = state
        .sftp_downloads
        .lock()
        .map_err(|_| "SFTP download state is unavailable".to_string())?;
    let handle = downloads
        .get_mut(&download_id)
        .ok_or_else(|| "SFTP download is not active".to_string())?;
    if handle.finished {
        return Ok(SftpDownloadChunk {
            data: Vec::new(),
            done: true,
        });
    }

    match handle.receiver.recv_timeout(Duration::from_millis(20)) {
        Ok(Ok(chunk)) => {
            let done = chunk.is_empty();
            if done {
                handle.finished = true;
            }
            Ok(SftpDownloadChunk { data: chunk, done })
        }
        Ok(Err(error)) => {
            handle.finished = true;
            Err(error)
        }
        Err(mpsc::RecvTimeoutError::Timeout) => Ok(SftpDownloadChunk {
            data: Vec::new(),
            done: false,
        }),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            handle.finished = true;
            Ok(SftpDownloadChunk {
                data: Vec::new(),
                done: true,
            })
        }
    }
}

#[tauri::command]
pub fn sftp_download_file_finish(
    state: State<'_, SshState>,
    download_id: String,
) -> Result<(), String> {
    if let Some(handle) = state
        .sftp_downloads
        .lock()
        .map_err(|_| "SFTP download state is unavailable".to_string())?
        .remove(&download_id)
    {
        handle.canceled.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
pub fn sftp_download_file_cancel(
    state: State<'_, SshState>,
    download_id: String,
) -> Result<(), String> {
    if let Some(handle) = state
        .sftp_downloads
        .lock()
        .map_err(|_| "SFTP download state is unavailable".to_string())?
        .remove(&download_id)
    {
        handle.canceled.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
pub fn sftp_delete_path(
    state: State<'_, SshState>,
    session_id: Option<String>,
    config: SshConfig,
    path: String,
    is_dir: bool,
) -> Result<(), String> {
    let target = Path::new(&path);
    if path.trim().is_empty() || target == Path::new("/") {
        return Err("Refusing to delete this path".to_string());
    }

    if let Some(session_id) = session_id {
        return with_sftp_session(&state, &session_id, |sftp| {
            delete_sftp_path(sftp, target, is_dir)
        });
    }

    let (session, _) = connect_session(&config)?;
    let sftp = session.sftp().map_err(to_string)?;
    delete_sftp_path(&sftp, target, is_dir)
}

fn delete_sftp_path(sftp: &ssh2::Sftp, target: &Path, is_dir: bool) -> Result<(), String> {
    if is_dir {
        sftp.rmdir(target).map_err(to_string)
    } else {
        sftp.unlink(target).map_err(to_string)
    }
}

#[tauri::command]
pub fn sftp_create_dir(
    state: State<'_, SshState>,
    session_id: Option<String>,
    config: SshConfig,
    path: String,
    name: String,
) -> Result<(), String> {
    validate_remote_name(&name)?;
    if let Some(session_id) = session_id {
        return with_sftp_session(&state, &session_id, |sftp| {
            create_sftp_dir(sftp, &path, &name)
        });
    }

    let (session, _) = connect_session(&config)?;
    let sftp = session.sftp().map_err(to_string)?;
    create_sftp_dir(&sftp, &path, &name)
}

fn create_sftp_dir(sftp: &ssh2::Sftp, path: &str, name: &str) -> Result<(), String> {
    let remote_path = join_remote_path(&path, &name);
    sftp.mkdir(&remote_path, 0o755).map_err(to_string)
}

#[tauri::command]
pub fn sftp_rename_path(
    state: State<'_, SshState>,
    session_id: Option<String>,
    config: SshConfig,
    old_path: String,
    new_name: String,
) -> Result<(), String> {
    validate_remote_name(&new_name)?;
    if let Some(session_id) = session_id {
        return with_sftp_session(&state, &session_id, |sftp| {
            rename_sftp_path(sftp, &old_path, &new_name)
        });
    }

    let (session, _) = connect_session(&config)?;
    let sftp = session.sftp().map_err(to_string)?;
    rename_sftp_path(&sftp, &old_path, &new_name)
}

fn rename_sftp_path(sftp: &ssh2::Sftp, old_path: &str, new_name: &str) -> Result<(), String> {
    let old = Path::new(old_path);
    let parent = old.parent().unwrap_or_else(|| Path::new("/"));
    let mut new_path = PathBuf::from(parent);
    new_path.push(new_name);
    sftp.rename(old, &new_path, Some(RenameFlags::OVERWRITE))
        .map_err(to_string)
}

fn with_sftp_session<T>(
    state: &State<'_, SshState>,
    session_id: &str,
    operation: impl FnOnce(&ssh2::Sftp) -> Result<T, String>,
) -> Result<T, String> {
    let sessions = state
        .sftp_sessions
        .lock()
        .map_err(|_| "SFTP session state is unavailable".to_string())?;
    let handle = sessions
        .get(session_id)
        .ok_or_else(|| "SFTP session is not connected".to_string())?;

    operation(&handle.sftp)
}

fn validate_remote_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains('/')
        || trimmed.contains('\\')
    {
        return Err("Invalid name".to_string());
    }

    Ok(())
}

fn join_remote_path(path: &str, file_name: &str) -> PathBuf {
    let mut base = PathBuf::from(path);
    base.push(file_name);
    base
}

fn connect_terminal_session(
    app: AppHandle,
    state: SshState,
    session_id: String,
    config: SshConfig,
) -> Result<(), String> {
    let (session, socket) = connect_session(&config)?;
    let mut channel = session.channel_session().map_err(to_string)?;
    channel
        .request_pty("xterm-256color", None, Some((120, 32, 0, 0)))
        .map_err(to_string)?;
    set_terminal_env(&mut channel);
    channel.shell().map_err(to_string)?;
    prepare_terminal_socket(&socket, &session)?;
    session.set_blocking(false);

    let (sender, receiver) = mpsc::channel();
    {
        let mut pending_disconnects = state
            .pending_disconnects
            .lock()
            .map_err(|_| "SSH session state is unavailable".to_string())?;
        if pending_disconnects.remove(&session_id) {
            let _ = channel.close();
            let _ = session.disconnect(None, "connection cancelled", None);
            return Ok(());
        }
    }

    state
        .sessions
        .lock()
        .map_err(|_| "SSH session state is unavailable".to_string())?
        .insert(session_id.clone(), SshSessionHandle { sender });

    emit_terminal(&app, &session_id, "connected", "");
    spawn_ssh_io_worker(app, session_id, session, channel, receiver);
    Ok(())
}

fn set_terminal_env(channel: &mut ssh2::Channel) {
    for (key, value) in [
        ("TERM", "xterm-256color"),
        ("COLORTERM", "truecolor"),
        ("CLICOLOR", "1"),
        ("FORCE_COLOR", "1"),
    ] {
        let _ = channel.setenv(key, value);
    }
}

pub(crate) fn connect_session(config: &SshConfig) -> Result<(Session, TcpStream), String> {
    if config.host.trim().is_empty() {
        return Err("Host is required".to_string());
    }

    let tcp = connect_tcp(&config.host, config.port)?;
    let socket_options = tcp.try_clone().map_err(to_string)?;
    tcp.set_nodelay(true).map_err(to_string)?;
    socket_options.set_nodelay(true).map_err(to_string)?;
    tcp.set_read_timeout(Some(Duration::from_secs(20)))
        .map_err(to_string)?;
    tcp.set_write_timeout(Some(Duration::from_secs(20)))
        .map_err(to_string)?;

    let mut session = Session::new().map_err(to_string)?;
    session.set_timeout(20_000);
    session.set_tcp_stream(tcp);
    session.handshake().map_err(to_string)?;

    if let Some(private_key) = config
        .private_key
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        let private_key_path = Path::new(private_key);
        if private_key_path.exists() {
            session
                .userauth_pubkey_file(
                    &config.username,
                    None,
                    private_key_path,
                    config.password.as_deref(),
                )
                .map_err(to_string)?;
        } else {
            session
                .userauth_pubkey_memory(
                    &config.username,
                    None,
                    private_key,
                    config.password.as_deref(),
                )
                .map_err(to_string)?;
        }
    } else {
        session
            .userauth_password(&config.username, config.password.as_deref().unwrap_or(""))
            .map_err(to_string)?;
    }

    if !session.authenticated() {
        return Err("SSH authentication failed".to_string());
    }

    Ok((session, socket_options))
}

fn prepare_terminal_socket(socket: &TcpStream, session: &Session) -> Result<(), String> {
    socket.set_read_timeout(None).map_err(to_string)?;
    socket.set_write_timeout(None).map_err(to_string)?;
    session.set_timeout(0);
    Ok(())
}

fn connect_tcp(host: &str, port: u16) -> Result<TcpStream, String> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| format!("Unable to resolve {host}:{port}: {error}"))?;

    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, Duration::from_secs(10)) {
            Ok(stream) => return Ok(stream),
            Err(error) => {
                last_error = Some(format!("{address}: {error}"));
            }
        }
    }

    Err(last_error
        .map(|error| format!("Unable to connect to {host}:{port} ({error})"))
        .unwrap_or_else(|| format!("Unable to resolve {host}:{port}")))
}

fn spawn_ssh_io_worker(
    app: AppHandle,
    session_id: String,
    session: Session,
    mut channel: ssh2::Channel,
    receiver: mpsc::Receiver<SshIoCommand>,
) {
    thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        let mut last_keepalive = Instant::now();

        loop {
            if !drain_channel_output(&app, &session_id, &mut channel, &mut buffer) {
                break;
            }

            if !drain_ssh_commands(&app, &session_id, &session, &mut channel, &receiver) {
                break;
            }

            if !drain_channel_output(&app, &session_id, &mut channel, &mut buffer) {
                break;
            }

            if last_keepalive.elapsed() >= SSH_KEEPALIVE_INTERVAL {
                let _ = retry_session_operation(&session, || session.keepalive_send());
                last_keepalive = Instant::now();
            }

            if channel.eof() {
                emit_terminal(&app, &session_id, "disconnected", "");
                break;
            }

            waitsocket(&session);
        }

        let _ = channel.close();
    });
}

fn drain_channel_output(
    app: &AppHandle,
    session_id: &str,
    channel: &mut ssh2::Channel,
    buffer: &mut [u8],
) -> bool {
    loop {
        match channel.read(buffer) {
            Ok(0) => return true,
            Ok(size) => {
                let output = String::from_utf8_lossy(&buffer[..size]).to_string();
                emit_terminal(app, session_id, "data", &output);
            }
            Err(error) if is_retryable_ssh_io_error(&error) => return true,
            Err(error) => {
                emit_terminal_error(app, session_id, &error.to_string());
                return false;
            }
        }
    }
}

fn drain_ssh_commands(
    app: &AppHandle,
    session_id: &str,
    session: &Session,
    channel: &mut ssh2::Channel,
    receiver: &mpsc::Receiver<SshIoCommand>,
) -> bool {
    loop {
        match receiver.try_recv() {
            Ok(SshIoCommand::Write(data)) => {
                if let Err(error) = write_channel_all(session, channel, data.as_bytes()) {
                    emit_terminal_error(app, session_id, &error);
                    return false;
                }
            }
            Ok(SshIoCommand::Resize { cols, rows }) => {
                if let Err(error) = retry_channel_operation(session, || {
                    channel
                        .request_pty_size(cols, rows, None, None)
                        .map_err(to_io_error)
                }) {
                    emit_terminal_error(app, session_id, &error.to_string());
                    return false;
                }
            }
            Ok(SshIoCommand::Disconnect) | Err(mpsc::TryRecvError::Disconnected) => return false,
            Err(mpsc::TryRecvError::Empty) => return true,
        }
    }
}

fn emit_terminal(app: &AppHandle, session_id: &str, kind: &str, data: &str) {
    let _ = app.emit(
        "ssh://terminal",
        TerminalEvent {
            session_id: session_id.to_string(),
            kind: kind.to_string(),
            data: data.to_string(),
        },
    );
}

fn emit_terminal_error(app: &AppHandle, session_id: &str, error: &str) {
    if is_ssh_transport_disconnect_message(error) {
        emit_terminal(app, session_id, "disconnected", "");
    } else {
        emit_terminal(app, session_id, "error", error);
    }
}

fn write_channel_all(
    session: &Session,
    channel: &mut ssh2::Channel,
    data: &[u8],
) -> Result<(), String> {
    let mut written = 0;
    let mut zero_write_attempts = 0;

    while written < data.len() {
        let size = retry_channel_operation(session, || {
            channel.write(&data[written..]).map_err(to_io_error)
        })
        .map_err(to_string)?;

        if size == 0 {
            if zero_write_attempts >= SSH_RETRY_ATTEMPTS {
                return Err("SSH channel accepted no data".to_string());
            }

            zero_write_attempts += 1;
            waitsocket(session);
            continue;
        }

        zero_write_attempts = 0;
        written += size;
    }

    retry_channel_operation(session, || channel.flush().map_err(to_io_error)).map_err(to_string)
}

fn retry_channel_operation<T>(
    session: &Session,
    mut operation: impl FnMut() -> std::io::Result<T>,
) -> std::io::Result<T> {
    let mut attempts = 0;

    loop {
        let result = operation();

        match result {
            Ok(value) => return Ok(value),
            Err(error) if is_retryable_ssh_io_error(&error) && attempts < SSH_RETRY_ATTEMPTS => {
                attempts += 1;
                waitsocket(session);
            }
            Err(error) => return Err(error),
        }
    }
}

fn retry_session_operation<T>(
    session: &Session,
    mut operation: impl FnMut() -> Result<T, ssh2::Error>,
) -> Result<T, ssh2::Error> {
    let mut attempts = 0;

    loop {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error)
                if error.code() == ssh2::ErrorCode::Session(-37)
                    && attempts < SSH_RETRY_ATTEMPTS =>
            {
                attempts += 1;
                waitsocket(session);
            }
            Err(error) => return Err(error),
        }
    }
}

fn waitsocket(session: &Session) {
    let directions = session.block_directions();
    let mut events = 0;

    if matches!(directions, BlockDirections::Inbound | BlockDirections::Both) {
        events |= libc::POLLIN;
    }

    if matches!(
        directions,
        BlockDirections::Outbound | BlockDirections::Both
    ) {
        events |= libc::POLLOUT;
    }

    if events == 0 {
        thread::sleep(SSH_RETRY_DELAY);
        return;
    }

    let mut poll_fd = libc::pollfd {
        fd: session.as_raw_fd(),
        events,
        revents: 0,
    };

    unsafe {
        let _ = libc::poll(&mut poll_fd, 1, SSH_POLL_TIMEOUT_MS);
    }
}

fn is_retryable_ssh_io_error(error: &std::io::Error) -> bool {
    if matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
    ) {
        return true;
    }

    if error.kind() != std::io::ErrorKind::Other {
        return false;
    }

    is_ssh_transport_disconnect_message(&error.to_string())
}

fn is_ssh_transport_disconnect_message(message: &str) -> bool {
    message.contains("transport read") || message.contains("Failure while draining incoming flow")
}

fn to_io_error(error: impl Into<std::io::Error>) -> std::io::Error {
    error.into()
}

fn is_directory(stat: &FileStat) -> bool {
    stat.perm
        .map(|permissions| permissions & libc_s_ifmt() == libc_s_ifdir())
        .unwrap_or(false)
}

fn libc_s_ifmt() -> u32 {
    0o170000
}

fn libc_s_ifdir() -> u32 {
    0o040000
}

fn to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}
