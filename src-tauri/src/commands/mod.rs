use crate::models::{
    BatchResult, LogEntry, ServiceConfig, ServiceResource, ServiceRuntime, SystemResource,
};
use crate::services::config_manager;
use crate::services::resource_monitor::ResourceMonitor;
use crate::services::service_manager::AppState;
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
pub async fn save_config_raw(content: String) -> Result<(), String> {
    // 先校验格式
    config_manager::validate_yaml(&content)?;
    std::fs::write(config_manager::config_path(), &content)
        .map_err(|e| format!("保存配置文件失败: {}", e))
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

/// toggle_tray_popup 的核心逻辑（同步），供 command 和 Rust 端 tray event 回调复用。
///
/// 将可见性切换与窗口定位逻辑提取为同步函数，避免在 tray 事件回调中
/// 无法使用 async command 的限制。
pub fn toggle_tray_popup_impl(app: &tauri::AppHandle) -> Result<bool, String> {
    if let Some(win) = app.get_webview_window("tray-popup") {
        let is_visible = win.is_visible().unwrap_or(false);
        if is_visible {
            let _ = win.hide();
            Ok(false)
        } else {
            // 定位到屏幕右上角（macOS menubar 下方）
            if let Ok(Some(monitor)) = app.primary_monitor() {
                let screen_size = monitor.size();
                let scale = monitor.scale_factor();
                let win_size = win
                    .outer_size()
                    .unwrap_or(tauri::PhysicalSize::new(380, 640));
                let x =
                    (screen_size.width as f64 / scale - win_size.width as f64 / scale - 16.0) as i32;
                let y = 36i32; // menubar 高度
                let _ = win.set_position(tauri::PhysicalPosition::new(
                    (x as f64 * scale) as i32,
                    (y as f64 * scale) as i32,
                ));
            }
            let _ = win.show();
            let _ = win.set_focus();
            Ok(true)
        }
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
