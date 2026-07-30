use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 服务配置（持久化到 config.yaml）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub id: String,
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub url: Option<String>,
    /// 端口：用于存活检测（启动脚本进程退出后，端口仍能反映服务的真实状态）。
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub work_dir: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_stop_timeout")]
    pub stop_timeout: u64,
}

fn default_stop_timeout() -> u64 {
    10
}

/// 服务运行状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
    Error,
}

impl ServiceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceStatus::Stopped => "stopped",
            ServiceStatus::Starting => "starting",
            ServiceStatus::Running => "running",
            ServiceStatus::Stopping => "stopping",
            ServiceStatus::Failed => "failed",
            ServiceStatus::Error => "error",
        }
    }
}

/// 服务运行时状态（不持久化）
#[derive(Debug, Clone, Serialize)]
pub struct ServiceRuntime {
    pub config: ServiceConfig,
    pub status: ServiceStatus,
    pub pid: Option<u32>,
    pub cpu_percent: f32,
    pub memory_mb: f64,
    pub uptime_secs: u64,
    pub restart_count: u32,
    pub last_error: Option<String>,
}

/// 系统资源快照
#[derive(Debug, Clone, Serialize)]
pub struct SystemResource {
    pub cpu_percent: f32,
    pub memory_used_gb: f64,
    pub memory_total_gb: f64,
    pub memory_percent: f32,
    pub gpu_name: Option<String>,
    pub gpu_percent: Option<f32>,
    pub gpu_memory_used_mb: Option<f64>,
    pub gpu_memory_total_mb: Option<f64>,
}

/// 单个服务的资源快照
#[derive(Debug, Clone, Serialize)]
pub struct ServiceResource {
    pub service_id: String,
    pub cpu_percent: f32,
    pub memory_mb: f64,
    pub pid: u32,
    pub uptime_secs: u64,
}

/// 应用配置（config.yaml 的顶层结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub services: Vec<ServiceConfig>,
}

/// 批量操作结果
#[derive(Debug, Clone, Serialize)]
pub struct BatchResult {
    pub service_id: String,
    pub success: bool,
    pub message: String,
}

/// 日志条目
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub service_id: String,
    pub timestamp: String,
    pub stream: String, // "stdout" or "stderr"
    pub line: String,
}

/// 服务状态变更事件 payload
#[derive(Debug, Clone, Serialize)]
pub struct StatusChangeEvent {
    pub service_id: String,
    pub status: String,
    pub pid: Option<u32>,
    pub error: Option<String>,
}

/// 资源更新事件 payload
#[derive(Debug, Clone, Serialize)]
pub struct ResourceUpdateEvent {
    pub system: SystemResource,
    pub services: Vec<ServiceResource>,
}
