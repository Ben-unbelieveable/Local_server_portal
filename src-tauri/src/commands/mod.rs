use crate::models::{
    BatchResult, LogEntry, NetworkInfo, ResourceHistoryPoint, ServiceConfig, ServiceResource,
    ServiceRuntime, SystemResource,
};
use crate::services::config_manager;
use crate::services::network_info;
use crate::services::preferences::{self, AppPreferences};
use crate::services::resource_monitor::ResourceMonitor;
use crate::services::service_manager::AppState;
use crate::TRAY_IGNORE_HIDE_UNTIL_MS;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, State};

// ==================== 服务管理 Commands ====================

#[tauri::command]
pub async fn get_services(state: State<'_, AppState>) -> Result<Vec<ServiceRuntime>, String> {
    let manager = state.manager.lock().await;
    Ok(manager.get_services())
}

#[tauri::command]
pub async fn add_service(
    state: State<'_, AppState>,
    config: ServiceConfig,
) -> Result<ServiceRuntime, String> {
    let mut manager = state.manager.lock().await;
    manager.add_service(config)
}

#[tauri::command]
pub async fn update_service(
    state: State<'_, AppState>,
    id: String,
    config: ServiceConfig,
) -> Result<ServiceRuntime, String> {
    let mut manager = state.manager.lock().await;
    manager.update_service(&id, config)
}

#[tauri::command]
pub async fn remove_service(
    state: State<'_, AppState>,
    id: String,
    stop_first: bool,
) -> Result<(), String> {
    let mut manager = state.manager.lock().await;
    manager.remove_service(&id, stop_first)
}

#[tauri::command]
pub async fn start_service(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut manager = state.manager.lock().await;
    manager.start_service(&id).await
}

#[tauri::command]
pub async fn stop_service(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut manager = state.manager.lock().await;
    manager.stop_service(&id).await
}

#[tauri::command]
pub async fn restart_service(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut manager = state.manager.lock().await;
    manager.restart_service(&id).await
}

#[tauri::command]
pub async fn batch_start(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<Vec<BatchResult>, String> {
    let mut manager = state.manager.lock().await;
    Ok(manager.batch_start(&ids).await)
}

#[tauri::command]
pub async fn batch_stop(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<Vec<BatchResult>, String> {
    let mut manager = state.manager.lock().await;
    Ok(manager.batch_stop(&ids).await)
}

// ==================== 资源监控 Commands ====================

#[tauri::command]
pub async fn get_system_resources(state: State<'_, AppState>) -> Result<SystemResource, String> {
    let manager = state.manager.lock().await;
    match manager.get_system_resource() {
        Some(res) => Ok(res),
        None => Ok(ResourceMonitor::get_system_resources()), // 监控循环首 tick 前的兜底
    }
}

#[tauri::command]
pub async fn get_service_resources(
    state: State<'_, AppState>,
) -> Result<Vec<ServiceResource>, String> {
    let manager = state.manager.lock().await;
    let services = manager.get_services();
    let mut resources = Vec::new();

    for service in services {
        if let Some(pid) = service.pid {
            let res = ResourceMonitor::get_process_resource(&service.config.id, pid, service.uptime_secs);
            resources.push(res);
        }
    }

    Ok(resources)
}

/// 获取应用启动后累计的资源历史采样（环形缓冲，供面积图首屏渲染）
#[tauri::command]
pub async fn get_resource_history(
    state: State<'_, AppState>,
) -> Result<Vec<ResourceHistoryPoint>, String> {
    let manager = state.manager.lock().await;
    Ok(manager.get_resource_history())
}

/// 获取本机局域网 IPv4 与公网 IP
#[tauri::command]
pub async fn get_network_info() -> Result<NetworkInfo, String> {
    Ok(network_info::fetch_network_info().await)
}

// ==================== 日志 Commands ====================

#[tauri::command]
pub async fn get_recent_logs(
    state: State<'_, AppState>,
    service_id: String,
    count: u32,
) -> Result<Vec<LogEntry>, String> {
    let manager = state.manager.lock().await;
    let log_mgr = manager.log_manager();
    Ok(log_mgr.get_recent(&service_id, count as usize))
}

#[tauri::command]
pub async fn search_logs(
    state: State<'_, AppState>,
    service_id: String,
    keyword: String,
) -> Result<Vec<LogEntry>, String> {
    let manager = state.manager.lock().await;
    let log_mgr = manager.log_manager();
    Ok(log_mgr.search(&service_id, &keyword))
}

#[tauri::command]
pub async fn get_history_logs(
    state: State<'_, AppState>,
    service_id: String,
    date: String,
) -> Result<Vec<LogEntry>, String> {
    let manager = state.manager.lock().await;
    let log_mgr = manager.log_manager();
    Ok(log_mgr.get_history(&service_id, &date))
}

// ==================== 配置 Commands ====================

#[tauri::command]
pub async fn get_config_raw() -> Result<String, String> {
    let path = config_manager::config_path();
    if path.exists() {
        std::fs::read_to_string(&path).map_err(|e| format!("读取配置文件失败: {}", e))
    } else {
        Ok(String::new())
    }
}

#[tauri::command]
pub async fn save_config_raw(
    state: State<'_, AppState>,
    content: String,
) -> Result<(), String> {
    config_manager::validate_yaml(&content)?;
    std::fs::write(config_manager::config_path(), &content)
        .map_err(|e| format!("保存配置文件失败: {}", e))?;
    let mut manager = state.manager.lock().await;
    manager.reload_config_from_disk()
}

#[tauri::command]
pub async fn validate_config(content: String) -> Result<String, String> {
    match config_manager::validate_yaml(&content) {
        Ok(_) => Ok("ok".to_string()),
        Err(e) => Err(e),
    }
}

// ==================== 内部事件处理 Commands ====================

#[tauri::command]
pub async fn handle_process_started(
    state: State<'_, AppState>,
    service_id: String,
    pid: u32,
) -> Result<(), String> {
    let mut manager = state.manager.lock().await;
    manager.on_process_started(&service_id, pid);
    Ok(())
}

#[tauri::command]
pub async fn handle_process_stopped(
    state: State<'_, AppState>,
    service_id: String,
) -> Result<(), String> {
    let mut manager = state.manager.lock().await;
    manager.on_process_stopped(&service_id);
    Ok(())
}

#[tauri::command]
pub async fn handle_process_error(
    state: State<'_, AppState>,
    service_id: String,
    error: String,
) -> Result<(), String> {
    let mut manager = state.manager.lock().await;
    manager.on_process_error(&service_id, &error);
    Ok(())
}

#[tauri::command]
pub async fn shutdown_all_services(state: State<'_, AppState>) -> Result<Vec<BatchResult>, String> {
    let mut manager = state.manager.lock().await;
    Ok(manager.shutdown_all().await)
}

// ==================== 托盘弹窗 Commands ====================

/// 显示托盘弹窗（不关闭、不打开主窗）。菜单栏单击确认后调用。
///
/// 输入：AppHandle
/// 输出：成功 `Ok(())`；窗口未创建则 `Err`
pub fn show_tray_popup_impl(app: &tauri::AppHandle) -> Result<(), String> {
    let win = app
        .get_webview_window("tray-popup")
        .ok_or_else(|| "托盘弹窗窗口未创建".to_string())?;
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let screen_size = monitor.size();
        let scale = monitor.scale_factor();
        let win_size = win
            .outer_size()
            .unwrap_or(tauri::PhysicalSize::new(380, 720));
        let x = (screen_size.width as f64 / scale - win_size.width as f64 / scale - 16.0) as i32;
        let y = 36i32;
        let _ = win.set_position(tauri::PhysicalPosition::new(
            (x as f64 * scale) as i32,
            (y as f64 * scale) as i32,
        ));
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    TRAY_IGNORE_HIDE_UNTIL_MS.store(now.saturating_add(400), Ordering::SeqCst);
    let _ = win.show();
    let _ = win.set_focus();
    Ok(())
}

/// 切换托盘弹窗显示/隐藏。
///
/// 输入：AppHandle
/// 输出：切换后是否可见
pub fn toggle_tray_popup_impl(app: &tauri::AppHandle) -> Result<bool, String> {
    if let Some(win) = app.get_webview_window("tray-popup") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
            return Ok(false);
        }
        show_tray_popup_impl(app)?;
        Ok(true)
    } else {
        Err("托盘弹窗窗口未创建".to_string())
    }
}

/// 切换托盘弹窗显示/隐藏，返回弹窗当前是否可见
#[tauri::command]
pub async fn toggle_tray_popup(app: tauri::AppHandle) -> Result<bool, String> {
    toggle_tray_popup_impl(&app)
}

/// 隐藏托盘弹窗
#[tauri::command]
pub async fn hide_tray_popup(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("tray-popup") {
        let _ = win.hide();
    }
    Ok(())
}

/// 显示并聚焦主窗口，同时隐藏托盘弹窗。
///
/// 从托盘 Webview 直接调用 `Window.getByLabel("main").show()` 常因 ACL
///（权限仅作用于当前窗口）失败；改为 Rust 侧操作可可靠打开主窗口。
/// macOS：先切回 Regular，让程序坞重新出现该应用标签。
#[tauri::command]
pub async fn show_main_window(app: tauri::AppHandle) -> Result<(), String> {
    // 先藏托盘，避免失焦竞态把用户注意力拉回弹窗
    if let Some(popup) = app.get_webview_window("tray-popup") {
        let _ = popup.hide();
        crate::TRAY_IGNORE_SHOW_UNTIL_MS.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0)
                .saturating_add(350),
            Ordering::SeqCst,
        );
    }

    // 恢复程序坞可见（关主窗时曾切到 Accessory）
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    }

    crate::reveal_main_window(&app);
    if app.get_webview_window("main").is_some() {
        Ok(())
    } else {
        Err("主窗口未创建".to_string())
    }
}

// ==================== 应用偏好 / 退出 ====================

/// 读取应用偏好（关主窗是否退托盘、开机启动镜像）
#[tauri::command]
pub async fn get_app_preferences() -> Result<AppPreferences, String> {
    Ok(preferences::load_preferences())
}

/// 设置：关闭 Dock 主窗口时是否同步退出托盘/进程
#[tauri::command]
pub async fn set_quit_when_close_main(enabled: bool) -> Result<AppPreferences, String> {
    preferences::set_quit_when_close_main(enabled)
}

/// 写入开机启动偏好镜像（OS 登录项由前端 autostart 插件实际开关）
#[tauri::command]
pub async fn set_launch_at_login_pref(enabled: bool) -> Result<AppPreferences, String> {
    preferences::set_launch_at_login_pref(enabled)
}

/// 彻底退出应用：先尝试停止全部托管服务，再结束进程（含托盘）。
///
/// 输入：无；输出：Ok 后进程即将退出。
#[tauri::command]
pub async fn quit_app(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut manager = state.manager.lock().await;
        let _ = manager.shutdown_all().await;
    }
    app.exit(0);
    Ok(())
}

