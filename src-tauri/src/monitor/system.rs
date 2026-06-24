use crate::ssh::session::{connect_session, SshConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorTarget {
    pub kind: String,
    pub ssh: Option<SshConfig>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSnapshot {
    pub timestamp: u64,
    pub host: HostInfo,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub network_summary: NetworkSummary,
    pub networks: Vec<NetworkInterface>,
    pub processes: Vec<ProcessInfo>,
    pub docker: DockerInfo,
    pub mounts: Vec<MountInfo>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HostInfo {
    pub username: String,
    pub host: String,
    pub uptime_seconds: u64,
    pub os: String,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub total_usage: f64,
    pub user_usage: f64,
    pub system_usage: f64,
    pub iowait_usage: f64,
    pub cores: Vec<f64>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    pub total: u64,
    pub used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSummary {
    pub rx_total: u64,
    pub tx_total: u64,
    pub rx_rate: f64,
    pub tx_rate: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInterface {
    pub name: String,
    pub ip: String,
    pub rx_total: u64,
    pub tx_total: u64,
    pub rx_rate: f64,
    pub tx_rate: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu: f64,
    pub memory: u64,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DockerInfo {
    pub available: bool,
    pub containers: Vec<DockerContainer>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerContainer {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MountInfo {
    pub name: String,
    pub path: String,
    pub total: u64,
    pub used: u64,
    pub available: u64,
}

struct CommandOutput {
    stdout: String,
}

#[tauri::command]
pub async fn monitor_snapshot(target: MonitorTarget) -> Result<MonitorSnapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let runner = Runner::new(target)?;
        runner.snapshot()
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn monitor_kill_process(target: MonitorTarget, pid: u32) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let runner = Runner::new(target)?;
        runner.run(&format!("kill -TERM {}", shell_arg(&pid.to_string())))?;
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn monitor_stop_container(
    target: MonitorTarget,
    container_id: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let runner = Runner::new(target)?;
        runner.run(&format!("docker stop {}", shell_arg(&container_id)))?;
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn monitor_start_container(
    target: MonitorTarget,
    container_id: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let runner = Runner::new(target)?;
        runner.run(&format!("docker start {}", shell_arg(&container_id)))?;
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn monitor_remove_container(
    target: MonitorTarget,
    container_id: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let runner = Runner::new(target)?;
        runner.run(&format!("docker rm {}", shell_arg(&container_id)))?;
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

struct Runner {
    target: MonitorTarget,
}

impl Runner {
    fn new(target: MonitorTarget) -> Result<Self, String> {
        if target.kind == "ssh" && target.ssh.is_none() {
            return Err("缺少 SSH 连接配置".to_string());
        }
        Ok(Self { target })
    }

    fn snapshot(&self) -> Result<MonitorSnapshot, String> {
        let output = self.run(SNAPSHOT_SCRIPT)?;
        let sections = parse_sections(&output.stdout);

        let host = self.host_info(&sections);
        let cpu = parse_cpu_samples(section(&sections, "CPU"));
        let memory = parse_memory(section(&sections, "MEMORY"));
        let networks = parse_networks(section(&sections, "NETWORK"));
        let network_summary = summarize_networks(&networks);
        let processes = parse_processes(section(&sections, "PROCESSES"));
        let docker = self.docker(&sections);
        let mounts = parse_mounts(section(&sections, "MOUNTS"));

        Ok(MonitorSnapshot {
            timestamp: now_ms(),
            host,
            cpu,
            memory,
            network_summary,
            networks,
            processes,
            docker,
            mounts,
        })
    }

    fn run(&self, script: &str) -> Result<CommandOutput, String> {
        if self.target.kind == "ssh" {
            let config = self
                .target
                .ssh
                .as_ref()
                .ok_or_else(|| "缺少 SSH 连接配置".to_string())?;
            run_ssh_command(config, script)
        } else {
            let mut command = Command::new("sh");
            command.arg("-lc").arg(script);
            run_command(command)
        }
    }

    fn host_info(&self, sections: &HashMap<String, String>) -> HostInfo {
        let host_section = section(sections, "HOST");
        let mut lines = host_section.lines();
        let username = lines.next().unwrap_or("").trim().to_string();
        let host = lines.next().unwrap_or("").trim().to_string();
        let uptime_seconds = lines.next().unwrap_or("").trim().parse().unwrap_or(0);
        let os = lines.next().unwrap_or("").trim().to_string();

        HostInfo {
            username,
            host,
            uptime_seconds,
            os,
        }
    }

    fn docker(&self, sections: &HashMap<String, String>) -> DockerInfo {
        let status = section(sections, "DOCKER_STATUS").trim();
        let output = section(sections, "DOCKER");
        match status {
            "ok" => DockerInfo {
                available: true,
                containers: parse_docker(output),
            },
            "empty" => DockerInfo {
                available: true,
                containers: Vec::new(),
            },
            _ => DockerInfo::default(),
        }
    }
}

const SNAPSHOT_SCRIPT: &str = r#"
printf '__MYHUB_SECTION__:HOST\n'
whoami 2>/dev/null || true
hostname -I 2>/dev/null | awk '{print $1}' || hostname 2>/dev/null || true
awk '{print int($1)}' /proc/uptime 2>/dev/null || echo 0
. /etc/os-release 2>/dev/null && echo "$PRETTY_NAME" || uname -sr 2>/dev/null || true
printf '__MYHUB_SECTION__:CPU\n'
cat /proc/stat 2>/dev/null | awk '/^cpu/ {print}'
sleep 0.25
echo --NEXT--
cat /proc/stat 2>/dev/null | awk '/^cpu/ {print}'
printf '__MYHUB_SECTION__:MEMORY\n'
cat /proc/meminfo 2>/dev/null || true
printf '__MYHUB_SECTION__:NETWORK\n'
cat /proc/net/dev 2>/dev/null || true
printf '\n--ADDR--\n'
ip -o -4 addr show 2>/dev/null || true
printf '__MYHUB_SECTION__:PROCESSES\n'
ps -eo pid=,pcpu=,rss=,comm= --sort=-pcpu 2>/dev/null | head -30 || true
printf '__MYHUB_SECTION__:DOCKER_STATUS\n'
if docker ps -a --format '{{.ID}}\t{{.Names}}\t{{.Image}}\t{{.Status}}' >/tmp/myhub-docker-ps.$$ 2>/dev/null; then
  if [ -s /tmp/myhub-docker-ps.$$ ]; then echo ok; else echo empty; fi
else
  echo unavailable
fi
printf '__MYHUB_SECTION__:DOCKER\n'
cat /tmp/myhub-docker-ps.$$ 2>/dev/null || true
rm -f /tmp/myhub-docker-ps.$$ 2>/dev/null || true
printf '__MYHUB_SECTION__:MOUNTS\n'
df -B1 -P 2>/dev/null | tail -n +2 || true
"#;

fn run_command(mut command: Command) -> Result<CommandOutput, String> {
    let output = command.output().map_err(|error| error.to_string())?;
    Ok(CommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
    })
}

fn run_ssh_command(config: &SshConfig, script: &str) -> Result<CommandOutput, String> {
    let (session, _socket) = connect_session(config)?;
    let mut channel = session
        .channel_session()
        .map_err(|error| error.to_string())?;
    channel.exec(script).map_err(|error| error.to_string())?;

    let mut stdout = String::new();
    let mut stderr = String::new();
    channel
        .read_to_string(&mut stdout)
        .map_err(|error| error.to_string())?;
    channel
        .stderr()
        .read_to_string(&mut stderr)
        .map_err(|error| error.to_string())?;
    channel.wait_close().map_err(|error| error.to_string())?;
    let _ = channel.exit_status();
    let _ = session.disconnect(None, "monitor command complete", None);

    Ok(CommandOutput { stdout })
}

fn parse_cpu_samples(raw: &str) -> CpuInfo {
    let Some((first, second)) = raw.split_once("--NEXT--") else {
        return parse_cpu(first_cpu_rows(raw));
    };
    let previous = cpu_rows(first);
    let current = cpu_rows(second);
    let mut rows = String::new();

    for (name, values) in current {
        let Some(before) = previous.get(&name) else {
            continue;
        };
        let diffs = values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                value
                    .saturating_sub(*before.get(index).unwrap_or(&0))
                    .to_string()
            })
            .collect::<Vec<String>>()
            .join(" ");
        rows.push_str(&format!("{name} {diffs}\n"));
    }

    parse_cpu(&rows)
}

fn parse_sections(raw: &str) -> HashMap<String, String> {
    let mut sections = HashMap::new();
    let mut current = String::new();
    let mut body = String::new();

    for line in raw.lines() {
        if let Some(name) = line.strip_prefix("__MYHUB_SECTION__:") {
            if !current.is_empty() {
                sections.insert(current.clone(), body.trim_matches('\n').to_string());
            }
            current = name.trim().to_string();
            body.clear();
        } else if !current.is_empty() {
            body.push_str(line);
            body.push('\n');
        }
    }

    if !current.is_empty() {
        sections.insert(current, body.trim_matches('\n').to_string());
    }

    sections
}

fn section<'a>(sections: &'a HashMap<String, String>, name: &str) -> &'a str {
    sections.get(name).map(String::as_str).unwrap_or("")
}

fn cpu_rows(raw: &str) -> HashMap<String, Vec<u64>> {
    raw.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 8 || !fields[0].starts_with("cpu") {
                return None;
            }
            Some((
                fields[0].to_string(),
                fields[1..]
                    .iter()
                    .filter_map(|value| value.parse::<u64>().ok())
                    .collect(),
            ))
        })
        .collect()
}

fn first_cpu_rows(raw: &str) -> &str {
    raw
}

fn parse_cpu(raw: &str) -> CpuInfo {
    let mut total = CpuInfo::default();
    for line in raw.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 8 {
            continue;
        }
        let values: Vec<f64> = fields[1..]
            .iter()
            .filter_map(|value| value.parse::<f64>().ok())
            .collect();
        if values.len() < 7 {
            continue;
        }
        let sum: f64 = values.iter().sum();
        if sum <= 0.0 {
            continue;
        }
        let idle = values.get(3).copied().unwrap_or(0.0) + values.get(4).copied().unwrap_or(0.0);
        let usage = clamp_percent((sum - idle) / sum * 100.0);
        if fields[0] == "cpu" {
            total.total_usage = usage;
            total.user_usage = clamp_percent(values[0] / sum * 100.0);
            total.system_usage = clamp_percent(values[2] / sum * 100.0);
            total.iowait_usage = clamp_percent(values.get(4).copied().unwrap_or(0.0) / sum * 100.0);
        } else {
            total.cores.push(usage);
        }
    }
    total
}

fn parse_memory(raw: &str) -> MemoryInfo {
    let mut values = HashMap::new();
    for line in raw.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() >= 2 {
            values.insert(
                fields[0].trim_end_matches(':').to_string(),
                fields[1].parse::<u64>().unwrap_or(0) * 1024,
            );
        }
    }
    let total = *values.get("MemTotal").unwrap_or(&0);
    let available = *values.get("MemAvailable").unwrap_or(&0);
    let swap_total = *values.get("SwapTotal").unwrap_or(&0);
    let swap_free = *values.get("SwapFree").unwrap_or(&0);
    MemoryInfo {
        total,
        used: total.saturating_sub(available),
        swap_total,
        swap_used: swap_total.saturating_sub(swap_free),
    }
}

fn parse_networks(raw: &str) -> Vec<NetworkInterface> {
    let mut parts = raw.split("\n--ADDR--\n");
    let dev = parts.next().unwrap_or("");
    let addr = parts.next().unwrap_or("");
    let mut ips = HashMap::new();
    for line in addr.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() >= 4 {
            ips.insert(
                fields[1].to_string(),
                fields[3].split('/').next().unwrap_or("").to_string(),
            );
        }
    }
    dev.lines()
        .filter_map(|line| {
            let (name, rest) = line.split_once(':')?;
            let name = name.trim().to_string();
            let fields: Vec<&str> = rest.split_whitespace().collect();
            if fields.len() < 16 {
                return None;
            }
            Some(NetworkInterface {
                ip: ips.get(&name).cloned().unwrap_or_default(),
                name,
                rx_total: fields[0].parse().unwrap_or(0),
                tx_total: fields[8].parse().unwrap_or(0),
                rx_rate: 0.0,
                tx_rate: 0.0,
            })
        })
        .collect()
}

fn parse_processes(raw: &str) -> Vec<ProcessInfo> {
    raw.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            Some(ProcessInfo {
                pid: parts.next()?.parse().ok()?,
                cpu: parts.next()?.parse().unwrap_or(0.0),
                memory: parts.next()?.parse::<u64>().unwrap_or(0) * 1024,
                name: parts.collect::<Vec<&str>>().join(" "),
            })
        })
        .collect()
}

fn parse_docker(raw: &str) -> Vec<DockerContainer> {
    raw.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() < 4 {
                return None;
            }
            Some(DockerContainer {
                id: fields[0].to_string(),
                name: fields[1].to_string(),
                image: fields[2].to_string(),
                status: fields[3].to_string(),
            })
        })
        .collect()
}

fn parse_mounts(raw: &str) -> Vec<MountInfo> {
    raw.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 6 {
                return None;
            }
            let total = fields[1].parse().unwrap_or(0);
            let used = fields[2].parse().unwrap_or(0);
            Some(MountInfo {
                name: fields[0].to_string(),
                total,
                used,
                available: fields[3].parse().unwrap_or(0),
                path: fields[5].to_string(),
            })
        })
        .collect()
}

fn summarize_networks(networks: &[NetworkInterface]) -> NetworkSummary {
    NetworkSummary {
        rx_total: networks.iter().map(|item| item.rx_total).sum(),
        tx_total: networks.iter().map(|item| item.tx_total).sum(),
        rx_rate: 0.0,
        tx_rate: 0.0,
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

fn shell_arg(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn clamp_percent(value: f64) -> f64 {
    value.max(0.0).min(100.0)
}
