//! macOS 桌面小组件数据同步。
//!
//! 写入 WidgetKit 扩展沙盒中的 `widget_snapshot.json`（扁平快照，由原生 SwiftUI 渲染）。
//! 不使用 tauri-plugin-widgets 的 JSON UI / LazyVGrid，避免 WidgetKit 白屏。

#[cfg(target_os = "macos")]
use crate::models::{NetworkInfo, ServiceRuntime, ServiceStatus, ShortcutConfig, SystemResource};
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
#[cfg(target_os = "macos")]
use tauri_plugin_shell::ShellExt;

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
static LAST_CONTENT_SIG: Mutex<String> = Mutex::new(String::new());

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
    shortcuts: Vec<ShortcutConfig>,
    shortcut_total: usize,
}

#[cfg(not(target_os = "macos"))]
pub fn push_snapshot(
    _app: &tauri::AppHandle,
    _resource: &crate::models::SystemResource,
    _services: &[crate::models::ServiceRuntime],
    _shortcuts: &[crate::models::ShortcutConfig],
) {
}

/// 将当前资源与服务快照写入小组件容器，并按间隔触发 WidgetKit reload。
///
/// 输入：AppHandle、系统资源、服务列表与常用入口
/// 输出：无；失败仅打 debug 日志
#[cfg(target_os = "macos")]
pub fn push_snapshot(
    app: &AppHandle,
    resource: &SystemResource,
    services: &[ServiceRuntime],
    shortcuts: &[ShortcutConfig],
) {
    push_snapshot_inner(app, resource, services, shortcuts, false);
}

/// 立即写入快照并刷新 Timeline（小组件点起停后使用）。
///
/// 输入：与 `push_snapshot` 相同；输出：无
#[cfg(target_os = "macos")]
pub fn push_snapshot_now(
    app: &AppHandle,
    resource: &SystemResource,
    services: &[ServiceRuntime],
    shortcuts: &[ShortcutConfig],
) {
    push_snapshot_inner(app, resource, services, shortcuts, true);
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

/// 输入资源、服务和入口；强制、到点或显示内容变化时写入快照并刷新。
#[cfg(target_os = "macos")]
fn push_snapshot_inner(
    app: &AppHandle,
    resource: &SystemResource,
    services: &[ServiceRuntime],
    shortcuts: &[ShortcutConfig],
    force: bool,
) {
    if !widget_extension_present() {
        return;
    }

    let sig = widget_sig(services, shortcuts);
    let sig_changed = {
        let mut last = LAST_CONTENT_SIG.lock().unwrap();
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
    let snapshot = build_snapshot(resource, services, shortcuts, &network);
    if !write_snapshot(&snapshot) {
        tracing::debug!("小组件容器尚未创建，跳过写入");
        return;
    }
    let _ = app.widget().reload_all_timelines();
}

/// 输入资源、服务、入口和网络；输出各保留前 12 条及真实总数的快照。
#[cfg(target_os = "macos")]
fn build_snapshot(
    resource: &SystemResource,
    services: &[ServiceRuntime],
    shortcuts: &[ShortcutConfig],
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
        shortcuts: shortcuts.iter().take(12).cloned().collect(),
        shortcut_total: shortcuts.len(),
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

/// 输入服务与入口；输出内容签名，使入口增删改在下一次监控循环触发刷新。
fn widget_sig(services: &[ServiceRuntime], shortcuts: &[ShortcutConfig]) -> String {
    let service_states: Vec<_> = services
        .iter()
        .map(|s| (&s.config.id, s.status.as_str()))
        .collect();
    serde_json::to_string(&(service_states, shortcuts)).unwrap_or_default()
}

/// 输入当前入口和被点击的 ID；只返回经过校验的已保存地址，拒绝失效入口。
#[cfg(target_os = "macos")]
pub fn shortcut_url(shortcuts: &[ShortcutConfig], id: &str) -> Result<String, String> {
    let shortcut = shortcuts
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| "入口已删除，请刷新小组件".to_string())?;
    shortcut.validate()?;
    Ok(shortcut.url.clone())
}

/// 输入当前服务和点击 ID；解析当前 HTTP(S) 地址，兼容省略协议的本机地址。
#[cfg(target_os = "macos")]
pub fn service_url(services: &[ServiceRuntime], id: &str) -> Result<String, String> {
    let raw = services.iter().find(|s| s.config.id == id)
        .and_then(|s| s.config.url.as_deref())
        .map(str::trim).filter(|url| !url.is_empty())
        .ok_or_else(|| "服务已删除或未配置访问地址".to_string())?;
    let address = if raw.contains("://") { raw.to_owned() } else { format!("http://{}", raw) };
    let url = tauri::Url::parse(&address).map_err(|_| "服务访问地址无效".to_string())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("服务访问地址仅支持 HTTP(S)".to_string());
    }
    Ok(url.to_string())
}

/// 输入服务及点击 ID；经校验后交给默认浏览器打开，不影响服务运行。
#[cfg(target_os = "macos")]
#[allow(deprecated)]
pub fn open_service(app: &AppHandle, services: &[ServiceRuntime], id: &str) -> Result<(), String> {
    app.shell().open(service_url(services, id)?, None)
        .map_err(|e| format!("打开服务失败: {}", e))
}

/// 输入当前入口及点击 ID；校验后复用主界面已有的 shell 插件打开目标。
#[cfg(target_os = "macos")]
#[allow(deprecated)] // 沿用项目已有 shell 插件，避免为单个访问动作引入新的插件。
pub fn open_shortcut(
    app: &AppHandle,
    shortcuts: &[ShortcutConfig],
    id: &str,
) -> Result<(), String> {
    let url = shortcut_url(shortcuts, id)?;
    app.shell()
        .open(url, None)
        .map_err(|e| format!("打开入口失败: {}", e))
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

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    /// 服务按钮按当前 ID 打开 URL；拒绝已删除、空地址及不受支持的协议。
    #[test]
    fn service_visit_uses_current_url() {
        let config = serde_json::from_value(serde_json::json!({
            "id": "Prompt_WorkBench", "name": "Prompt_WorkBench", "command": "test",
            "url": "http://127.0.0.1:8091/"
        })).unwrap();
        let mut service = ServiceRuntime {
            config, status: ServiceStatus::Running, pid: None,
            cpu_percent: 0.0, memory_mb: 0.0, uptime_secs: 0,
            restart_count: 0, last_error: None,
        };
        let id = "Prompt_WorkBench";
        assert_eq!(service_url(&[service.clone()], id).unwrap(), "http://127.0.0.1:8091/");
        service.config.url = Some("localhost:8092/page?q=1#top".into());
        assert_eq!(service_url(&[service.clone()], id).unwrap(), "http://localhost:8092/page?q=1#top");
        assert!(service_url(&[service.clone()], "deleted").is_err());
        for url in ["", "javascript:alert(1)", "file:///tmp/test.html", "ftp://localhost/test"] {
            service.config.url = Some(url.into());
            assert!(service_url(&[service.clone()], id).is_err(), "{url}");
        }
    }

    /// 输入序号；构造不依赖真实配置和进程的入口样本。
    fn shortcut(index: usize) -> ShortcutConfig {
        ShortcutConfig {
            id: format!("shortcut-{}", index),
            name: format!("入口 {}", index),
            url: format!("http://localhost:3000/{}", index),
            group: None,
            description: None,
        }
    }

    /// 无输入；构造快照测试所需的固定资源数据。
    fn resource() -> SystemResource {
        SystemResource {
            cpu_percent: 25.0,
            memory_used_gb: 4.0,
            memory_total_gb: 16.0,
            memory_percent: 25.0,
            gpu_name: None,
            gpu_percent: None,
            gpu_memory_used_mb: None,
            gpu_memory_total_mb: None,
            cpu_user_percent: None,
            cpu_system_percent: None,
            cpu_idle_percent: None,
            gpu_renderer_percent: None,
            gpu_tiler_percent: None,
            gpu_core_count: None,
        }
    }

    /// 仅有常用入口、没有服务时，快照仍携带完整入口及总数。
    #[test]
    fn snapshot_includes_shortcuts_without_services() {
        let shortcuts = vec![shortcut(0)];
        let network = NetworkInfo {
            lan_ips: vec![],
            public_ip: None,
            public_ip_error: None,
        };
        let snapshot = build_snapshot(&resource(), &[], &shortcuts, &network);
        let json = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(json["total"], 0);
        assert_eq!(json["shortcut_total"], 1);
        assert_eq!(json["shortcuts"][0]["id"], "shortcut-0");
        assert_eq!(json["shortcuts"][0]["url"], "http://localhost:3000/0");
    }

    /// 超过快照条数上限时保留真实总数，使 Swift 显示准确溢出计数。
    #[test]
    fn snapshot_caps_entries_but_preserves_total() {
        let shortcuts: Vec<_> = (0..18).map(shortcut).collect();
        let network = NetworkInfo {
            lan_ips: vec![],
            public_ip: None,
            public_ip_error: None,
        };
        let snapshot = build_snapshot(&resource(), &[], &shortcuts, &network);
        assert_eq!(snapshot.shortcuts.len(), 12);
        assert_eq!(snapshot.shortcut_total, 18);
    }

    /// 增删以及同 ID 的名称/地址修改都会改变刷新签名。
    #[test]
    fn shortcut_edits_trigger_refresh() {
        let entry = shortcut(0);
        let before = widget_sig(&[], &[entry.clone()]);
        assert_ne!(before, widget_sig(&[], &[]));
        assert_ne!(before, widget_sig(&[], &[entry.clone(), shortcut(1)]));
        let mut renamed = entry.clone();
        renamed.name = "更新名称".into();
        assert_ne!(before, widget_sig(&[], &[renamed]));
        let mut moved = entry;
        moved.url = "https://example.com".into();
        assert_ne!(before, widget_sig(&[], &[moved]));
    }

    /// 按当前保存的入口 ID 解析访问目标，不执行服务命令；失效/危险目标被拒绝。
    #[test]
    fn open_uses_current_validated_shortcut() {
        let entry = shortcut(0);
        assert_eq!(
            shortcut_url(&[entry.clone()], &entry.id).unwrap(),
            entry.url
        );
        assert!(shortcut_url(&[entry.clone()], "deleted").is_err());
        let mut file = entry.clone();
        file.url = "file:///tmp/local%20page.html".into();
        assert_eq!(shortcut_url(&[file.clone()], &file.id).unwrap(), file.url);
        let mut invalid = entry;
        invalid.url = "javascript:alert(1)".into();
        assert!(shortcut_url(&[invalid.clone()], &invalid.id).is_err());
    }
}
