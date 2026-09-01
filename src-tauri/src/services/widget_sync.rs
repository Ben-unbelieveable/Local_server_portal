//! macOS 桌面小组件数据同步。
//!
//! 写入 WidgetKit 扩展沙盒中的 `widget_snapshot.json`（扁平快照，由原生 SwiftUI 渲染）。
//! 不使用 tauri-plugin-widgets 的 JSON UI / LazyVGrid，避免 WidgetKit 白屏。

#[cfg(target_os = "macos")]
use crate::models::{NetworkInfo, ServiceRuntime, ServiceStatus, SystemResource};
#[cfg(target_os = "macos")]
use serde::{Deserialize, Serialize};
#[cfg(target_os = "macos")]
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::sync::Mutex;
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};
#[cfg(target_os = "macos")]
use tauri::AppHandle;
#[cfg(target_os = "macos")]
use tauri_plugin_widgets::WidgetExt;

/// WidgetKit 扩展 bundle id，对应 ~/Library/Containers/<id>/Data
#[cfg(target_os = "macos")]
const WIDGET_EXTENSION_BUNDLE_ID: &str = "com.local-service-manager.app.widgetkit";

#[cfg(target_os = "macos")]
static LAST_WIDGET_RELOAD: Mutex<Option<Instant>> = Mutex::new(None);

#[cfg(target_os = "macos")]
static CACHED_NETWORK: Mutex<Option<NetworkInfo>> = Mutex::new(None);

#[cfg(target_os = "macos")]
static LAST_NETWORK_FETCH: Mutex<Option<Instant>> = Mutex::new(None);

#[cfg(target_os = "macos")]
static LAST_STATUS_SIG: Mutex<String> = Mutex::new(String::new());

/// 小组件 App Intent 写入的起停请求（主应用每秒拾取）
#[cfg(target_os = "macos")]
#[derive(Debug, Deserialize)]
pub struct WidgetPendingAction {
    pub id: String,
    pub action: String,
}

/// 小组件 Timeline 刷新间隔
#[cfg(target_os = "macos")]
const WIDGET_RELOAD_INTERVAL: Duration = Duration::from_secs(30);

/// 小组件展示用的服务摘要
#[cfg(target_os = "macos")]
#[derive(Serialize)]
struct WidgetServiceItem {
    id: String,
    name: String,
    status: String,
    url: Option<String>,
}

/// 小组件快照（Swift Codable 对应字段）
#[cfg(target_os = "macos")]
#[derive(Serialize)]
struct WidgetSnapshot {
    cpu: f32,
    memory: f32,
    gpu: Option<f32>,
    lan: String,
    public_ip: String,
    running: usize,
    total: usize,
    services: Vec<WidgetServiceItem>,
}

#[cfg(not(target_os = "macos"))]
pub fn push_snapshot(
    _app: &tauri::AppHandle,
    _resource: &crate::models::SystemResource,
    _services: &[crate::models::ServiceRuntime],
) {
}

/// 将当前资源与服务快照写入小组件容器，并按间隔触发 WidgetKit reload。
///
/// 输入：AppHandle、系统资源、服务列表
/// 输出：无；失败仅打 debug 日志
#[cfg(target_os = "macos")]
pub fn push_snapshot(
    app: &AppHandle,
    resource: &SystemResource,
    services: &[ServiceRuntime],
) {
    push_snapshot_inner(app, resource, services, false);
}

/// 立即写入快照并刷新 Timeline（小组件点起停后使用）。
///
/// 输入：与 `push_snapshot` 相同；输出：无
#[cfg(target_os = "macos")]
pub fn push_snapshot_now(
    app: &AppHandle,
    resource: &SystemResource,
    services: &[ServiceRuntime],
) {
    push_snapshot_inner(app, resource, services, true);
}

/// 读取并删除小组件写入的起停请求。
///
/// 输入：无（读扩展容器 `widget_action.json`）
/// 输出：有待处理动作时返回 `Some`，否则 `None`
#[cfg(target_os = "macos")]
pub fn take_pending_action() -> Option<WidgetPendingAction> {
    let dir = container_data_dir()?;
    let path = dir.join("widget_action.json");
    let data = std::fs::read(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    serde_json::from_slice(&data).ok()
}

/// 写入快照：force、到点，或服务状态变化时才落盘。
#[cfg(target_os = "macos")]
fn push_snapshot_inner(
    app: &AppHandle,
    resource: &SystemResource,
    services: &[ServiceRuntime],
    force: bool,
) {
    if !widget_extension_present() {
        return;
    }

    let sig = services_sig(services);
    let sig_changed = {
        let mut last = LAST_STATUS_SIG.lock().unwrap();
        if *last != sig {
            *last = sig;
            true
        } else {
            false
        }
    };

    let due = {
        let mut last = LAST_WIDGET_RELOAD.lock().unwrap();
        let now = Instant::now();
        let due = last
            .map(|t| now.duration_since(t) >= WIDGET_RELOAD_INTERVAL)
            .unwrap_or(true);
        if due {
            *last = Some(now);
        }
        due
    };
    if !force && !due && !sig_changed {
        return;
    }

    refresh_network_cache_async();
    let network = network_snapshot();
    let snapshot = build_snapshot(resource, services, &network);
    if !write_snapshot(&snapshot) {
        tracing::debug!("小组件容器尚未创建，跳过写入");
        return;
    }
    let _ = app.widget().reload_all_timelines();
}

/// 组装快照：最多 12 条服务（小组件竖排展示，名称由 Swift 截断）
#[cfg(target_os = "macos")]
fn build_snapshot(
    resource: &SystemResource,
    services: &[ServiceRuntime],
    network: &NetworkInfo,
) -> WidgetSnapshot {
    let gpu = resource
        .gpu_percent
        .or(resource.gpu_renderer_percent)
        .or(resource.gpu_tiler_percent);
    let lan = network
        .lan_ips
        .first()
        .cloned()
        .unwrap_or_else(|| "—".into());
    let public_ip = network
        .public_ip
        .clone()
        .unwrap_or_else(|| "—".into());
    let running = services
        .iter()
        .filter(|s| s.status == ServiceStatus::Running)
        .count();
    let items = services
        .iter()
        .take(12)
        .map(|svc| WidgetServiceItem {
            id: svc.config.id.clone(),
            name: svc.config.name.clone(),
            status: svc.status.as_str().to_string(),
            url: svc.config.url.clone().filter(|u| !u.is_empty()),
        })
        .collect();
    WidgetSnapshot {
        cpu: resource.cpu_percent,
        memory: resource.memory_percent,
        gpu,
        lan,
        public_ip,
        running,
        total: services.len(),
        services: items,
    }
}

/// 写入扩展沙盒 Data/widget_snapshot.json（目录必须已存在）
#[cfg(target_os = "macos")]
fn write_snapshot(snapshot: &WidgetSnapshot) -> bool {
    let Some(dir) = container_data_dir() else {
        return false;
    };
    let Ok(body) = serde_json::to_string(snapshot) else {
        return false;
    };
    std::fs::write(dir.join("widget_snapshot.json"), body).is_ok()
}

/// 扩展容器 Data 目录；不存在则返回 None（不创建，避免 TCC）。
fn container_data_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let dir = PathBuf::from(home)
        .join("Library/Containers")
        .join(WIDGET_EXTENSION_BUNDLE_ID)
        .join("Data");
    dir.is_dir().then_some(dir)
}

/// 服务 id+状态签名，用于判断是否需要立刻刷新小组件。
fn services_sig(services: &[ServiceRuntime]) -> String {
    services
        .iter()
        .map(|s| format!("{}:{}", s.config.id, s.status.as_str()))
        .collect::<Vec<_>>()
        .join("|")
}

/// 当前 .app 是否已嵌入 WidgetKit 扩展
#[cfg(target_os = "macos")]
fn widget_extension_present() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let plugins = exe
        .parent()
        .and_then(|macos| macos.parent())
        .map(|contents| contents.join("PlugIns"));
    let Some(plugins) = plugins else {
        return false;
    };
    let Ok(entries) = std::fs::read_dir(plugins) else {
        return false;
    };
    entries.filter_map(|e| e.ok()).any(|e| {
        e.path()
            .extension()
            .is_some_and(|ext| ext == "appex")
    })
}

#[cfg(target_os = "macos")]
fn refresh_network_cache_async() {
    let now = Instant::now();
    let need_fetch = {
        let last = LAST_NETWORK_FETCH.lock().unwrap();
        last.map(|t| now.duration_since(t) >= Duration::from_secs(60))
            .unwrap_or(true)
    };
    if !need_fetch {
        return;
    }
    *LAST_NETWORK_FETCH.lock().unwrap() = Some(now);
    tauri::async_runtime::spawn(async {
        let info = crate::services::network_info::fetch_network_info().await;
        *CACHED_NETWORK.lock().unwrap() = Some(info);
    });
}

#[cfg(target_os = "macos")]
fn network_snapshot() -> NetworkInfo {
    let cached = CACHED_NETWORK.lock().unwrap().clone();
    NetworkInfo {
        lan_ips: crate::services::network_info::get_lan_ips(),
        public_ip: cached.as_ref().and_then(|c| c.public_ip.clone()),
        public_ip_error: cached.as_ref().and_then(|c| c.public_ip_error.clone()),
    }
}
