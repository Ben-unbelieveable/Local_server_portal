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

/// 系统资源快照（含可选细项，供堆叠面积图图例使用）
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
    /// CPU 用户态占比（0~100），不可用时为 None
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_user_percent: Option<f32>,
    /// CPU 内核态占比（0~100）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_system_percent: Option<f32>,
    /// CPU 闲置占比（0~100）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_idle_percent: Option<f32>,
    /// GPU 渲染利用率（Apple Silicon ioreg Renderer Utilization %）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_renderer_percent: Option<f32>,
    /// GPU Tiler 利用率（Apple Silicon ioreg Tiler Utilization %）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_tiler_percent: Option<f32>,
    /// GPU 核心数（如 Apple Silicon 的 gpu-core-count）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_core_count: Option<u32>,
}

/// 资源历史采样点（应用启动后由监控循环追加，环形缓冲）
#[derive(Debug, Clone, Serialize)]
pub struct ResourceHistoryPoint {
    /// Unix 毫秒时间戳
    pub ts: u64,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_user_percent: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_system_percent: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_idle_percent: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_percent: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_renderer_percent: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_tiler_percent: Option<f32>,
}

impl ResourceHistoryPoint {
    /// 从当前系统资源快照构造历史点
    ///
    /// 输入：`SystemResource` 快照
    /// 输出：带当前时间戳的历史采样点
    pub fn from_system(res: &SystemResource) -> Self {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            ts,
            cpu_percent: res.cpu_percent,
            memory_percent: res.memory_percent,
            cpu_user_percent: res.cpu_user_percent,
            cpu_system_percent: res.cpu_system_percent,
            cpu_idle_percent: res.cpu_idle_percent,
            gpu_percent: res.gpu_percent,
            gpu_renderer_percent: res.gpu_renderer_percent,
            gpu_tiler_percent: res.gpu_tiler_percent,
        }
    }
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

/// 本机网络信息（局域网 IPv4 + 公网 IP）
#[derive(Debug, Clone, Serialize)]
pub struct NetworkInfo {
    /// 局域网 IPv4 列表（非 loopback）
    pub lan_ips: Vec<String>,
    /// 公网 IP；未联网或查询失败时为 None
    pub public_ip: Option<String>,
    /// 公网 IP 查询失败时的说明
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_ip_error: Option<String>,
}

/// 资源更新事件 payload（含启动后累计的历史曲线）
#[derive(Debug, Clone, Serialize)]
pub struct ResourceUpdateEvent {
    pub system: SystemResource,
    pub services: Vec<ServiceResource>,
    /// 启动后采样的资源历史（环形缓冲，最新在末尾）
    pub history: Vec<ResourceHistoryPoint>,
}
