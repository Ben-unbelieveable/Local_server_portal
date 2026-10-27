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
    /// 兼容旧配置：未提供访问地址时用于存活检测；有地址时自动解析其端口。
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

impl ServiceConfig {
    /// 从本机 HTTP(S) 访问地址解析监控端口；无地址时兼容旧 port 配置。
    /// 远端地址不用于本机 PID 探测，避免接管本机同端口的其他进程。
    pub fn monitoring_port(&self) -> Option<u16> {
        match self.url.as_deref().filter(|url| !url.trim().is_empty()) {
            Some(url) => {
                let url = tauri::Url::parse(url.trim()).ok()?;
                if !matches!(url.scheme(), "http" | "https") {
                    return None;
                }
                let host = url.host_str()?;
                let local = host.eq_ignore_ascii_case("localhost")
                    || host
                        .trim_matches(['[', ']'])
                        .parse::<std::net::IpAddr>()
                        .map(|ip| {
                            ip.is_loopback()
                                || ip.is_unspecified()
                                || if_addrs::get_if_addrs()
                                    .unwrap_or_default()
                                    .iter()
                                    .any(|iface| iface.ip() == ip)
                        })
                        .unwrap_or(false);
                if local {
                    url.port_or_known_default().filter(|port| *port != 0)
                } else {
                    None
                }
            }
            None => self.port,
        }
    }
}

#[cfg(test)]
mod service_monitoring_tests {
    use super::ServiceConfig;

    #[test]
    fn monitoring_port_uses_local_url_and_preserves_legacy_configs() {
        let mut config: ServiceConfig = serde_json::from_value(serde_json::json!({
            "id": "test", "name": "test", "command": "test", "port": 9000
        })).unwrap();
        assert_eq!(config.monitoring_port(), Some(9000));
        for (url, expected) in [
            ("http://127.0.0.1:8091/", Some(8091)),
            ("http://localhost:8091/index.html?test=1#top", Some(8091)),
            ("http://[::1]:8091/", Some(8091)),
            ("http://0.0.0.0:8091/", Some(8091)),
            ("https://localhost:443/", Some(443)),
            ("http://localhost/", Some(80)),
            ("https://example.com:8091/", None),
            ("file:///tmp/test.html", None),
            ("invalid", None),
            ("http://localhost:0/", None),
        ] {
            config.url = Some(url.to_owned());
            assert_eq!(config.monitoring_port(), expected, "{url}");
        }
        config.port = None;
        config.url = Some(" http://127.0.0.1:8091/ ".to_owned());
        assert_eq!(config.monitoring_port(), Some(8091));
        config.url = None;
        assert_eq!(config.monitoring_port(), None);
    }
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

/// 常用入口只描述访问目标，不参与服务进程管理。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutConfig {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

impl ShortcutConfig {
    /// 输入入口字段，校验名称和可访问的页面 URL；非法协议或非 HTML 文件返回说明。
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() || self.name.trim().is_empty() {
            return Err("入口 ID 和名称不能为空".to_string());
        }
        let url =
            tauri::Url::parse(&self.url).map_err(|_| "请输入完整的有效访问地址".to_string())?;
        let address = self.url.to_ascii_lowercase();
        match url.scheme() {
            "http" | "https"
                if url.host_str().is_some()
                    && (address.starts_with("http://") || address.starts_with("https://")) =>
            {
                Ok(())
            }
            "file" if address.starts_with("file:///") => {
                let path = url
                    .to_file_path()
                    .map_err(|_| "请使用本机 HTML 文件地址".to_string())?;
                let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                if (extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm"))
                    && (url.path().to_ascii_lowercase().ends_with(".html")
                        || url.path().to_ascii_lowercase().ends_with(".htm"))
                {
                    Ok(())
                } else {
                    Err("本地页面仅支持 .html 或 .htm 文件".to_string())
                }
            }
            _ => Err("仅支持 http://、https:// 和本地 HTML 的 file:/// 地址".to_string()),
        }
    }
}

/// 服务及常用入口共享的 YAML 配置；旧文件缺省为空入口列表。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub services: Vec<ServiceConfig>,
    #[serde(default)]
    pub shortcuts: Vec<ShortcutConfig>,
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
