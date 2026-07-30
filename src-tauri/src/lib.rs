pub mod commands;
pub mod models;
pub mod services;

use services::local_api;
use services::preferences::{self, QUIT_WHEN_CLOSE_MAIN};
use services::resource_monitor::ResourceMonitor;
use services::service_manager::AppState;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Emitter;
use tauri::Listener;
use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;

/// 托盘弹窗因失焦隐藏后，短暂忽略随后的托盘点击，避免「失焦隐藏 → 托盘点击又立刻显示」竞态。
/// 值为 Unix 毫秒时间戳：在此时间之前的 toggle 若本意是「显示」，将被跳过。
pub static TRAY_IGNORE_SHOW_UNTIL_MS: AtomicU64 = AtomicU64::new(0);

/// 当前 Unix 毫秒时间戳
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        // 开机启动：社区方案 tauri-plugin-autostart（macOS LaunchAgent）
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let app_handle = app.handle().clone();

            // 加载托盘行为偏好（关主窗是否退托盘等）
            let _ = preferences::load_preferences();

            // 初始化应用状态
            let state = AppState::new(app_handle.clone())
                .expect("无法初始化应用状态");

            // 共享给异步任务用的 Arc 句柄（AppState.manager 本身就是 Arc<Mutex<...>>）
            let manager = Arc::clone(&state.manager);

            // 启动资源监控定时器
            let app_handle_monitor = app_handle.clone();
            let manager_monitor = Arc::clone(&manager);
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
                loop {
                    interval.tick().await;

                    // 采集系统资源
                    let sys_res = ResourceMonitor::get_system_resources();

                    // 写入缓存 + 历史环形缓冲，并取出 history 供事件推送
                    let history = {
                        let mut mgr = manager_monitor.lock().await;
                        mgr.set_system_resource(sys_res.clone());
                        mgr.get_resource_history()
                    };

                    // 采集服务资源
                    let mut svc_resources = Vec::new();
                    {
                        let mut mgr = manager_monitor.lock().await;
                        let services = mgr.get_services();
                        for service in services {
                            if let Some(pid) = service.pid {
                                let res = ResourceMonitor::get_process_resource(
                                    &service.config.id,
                                    pid,
                                    service.uptime_secs,
                                );
                                mgr.update_resource(
                                    &service.config.id,
                                    res.cpu_percent,
                                    res.memory_mb,
                                );
                                svc_resources.push(res);
                            }
                        }
                        // 检测异常退出
                        mgr.check_process_alive();
                    }

                    // 推送资源更新事件（含启动后历史曲线）
                    let _ = app_handle_monitor.emit(
                        "resource-update",
                        crate::models::ResourceUpdateEvent {
                            system: sys_res,
                            services: svc_resources,
                            history,
                        },
                    );
                }
            });

            // 处理进程启动事件
            let manager1 = Arc::clone(&manager);
            app_handle.listen("service-process-started", move |event: tauri::Event| {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(event.payload()) {
                    let service_id = data["service_id"].as_str().unwrap_or("").to_string();
                    let pid = data["pid"].as_u64().unwrap_or(0) as u32;
                    let mgr = manager1.clone();
                    tauri::async_runtime::spawn(async move {
                        let mut m = mgr.lock().await;
                        m.on_process_started(&service_id, pid);
                    });
                }
            });

            // 处理进程停止事件
            let manager2 = Arc::clone(&manager);
            app_handle.listen("service-process-stopped", move |event: tauri::Event| {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(event.payload()) {
                    let service_id = data["service_id"].as_str().unwrap_or("").to_string();
                    let mgr = manager2.clone();
                    tauri::async_runtime::spawn(async move {
                        let mut m = mgr.lock().await;
                        m.on_process_stopped(&service_id);
                    });
                }
            });

            // 处理进程错误事件
            let app_handle3 = app_handle.clone();
            let manager3 = Arc::clone(&manager);
            app_handle.listen("service-process-error", move |event: tauri::Event| {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(event.payload()) {
                    let service_id = data["service_id"].as_str().unwrap_or("").to_string();
                    let error_msg = data["error"].as_str().unwrap_or("未知错误").to_string();
                    let mgr = manager3.clone();
                    let handle = app_handle3.clone();
                    tauri::async_runtime::spawn(async move {
                        {
                            let mut m = mgr.lock().await;
                            m.on_process_error(&service_id, &error_msg);
                        }
                        // 发送系统通知
                        let _ = handle.emit(
                            "show-notification",
                            serde_json::json!({
                                "title": "服务异常",
                                "body": format!("服务异常退出: {}", error_msg),
                            }),
                        );
                    });
                }
            });

            // 本机 HTTP 桥：浏览器 Vite 预览与主窗/托盘共用同一 AppState
            local_api::spawn(Arc::clone(&manager));

            // 管理 AppState
            app.manage(state);

            // 创建托盘弹窗窗口（默认隐藏，点击托盘图标时显示）
            let tray_popup = tauri::WebviewWindowBuilder::new(
                &app_handle,
                "tray-popup",
                tauri::WebviewUrl::App("index.html#/tray-popup".into()),
            )
            .title("服务快速管理")
            .inner_size(380.0, 720.0)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .visible(false)
            .focused(false)
            .build();

            // 点击屏幕其他位置（弹窗失焦）时自动隐藏托盘详情窗。
            // 社区标准做法：WindowEvent::Focused(false) → hide。
            // 与托盘再次点击存在竞态：失焦先触发 hide，随后托盘 Click 又会 show；
            // 因此失焦隐藏后设置 350ms 忽略「显示」窗口。
            if let Ok(win) = &tray_popup {
                let hide_win = win.clone();
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(false) = event {
                        let _ = hide_win.hide();
                        TRAY_IGNORE_SHOW_UNTIL_MS
                            .store(now_ms().saturating_add(350), Ordering::SeqCst);
                    }
                });
            }

            // 给配置自动创建的托盘图标挂上点击事件回调。
            if let Some(tray) = app.tray_by_id("main") {
                let tray_app = app_handle.clone();
                tray.on_tray_icon_event(move |_tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { button_state, .. } = &event {
                        if *button_state == tauri::tray::MouseButtonState::Up {
                            let _ = commands::toggle_tray_popup_impl(&tray_app);
                        }
                    }
                });
            }

            // 主窗口关闭：默认隐藏到托盘（不退出）；偏好开启时彻底退出（含托盘）。
            // 注意：隐藏中的 tray-popup 仍算存活窗口，仅 allow close 主窗往往不会结束进程。
            if let Some(main) = app.get_webview_window("main") {
                let main_for_hide = main.clone();
                let app_for_quit = app_handle.clone();
                main.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        if !QUIT_WHEN_CLOSE_MAIN.load(Ordering::SeqCst) {
                            api.prevent_close();
                            let _ = main_for_hide.hide();
                        } else {
                            api.prevent_close();
                            let handle = app_for_quit.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Some(state) = handle.try_state::<AppState>() {
                                    let mut mgr = state.manager.lock().await;
                                    let _ = mgr.shutdown_all().await;
                                }
                                handle.exit(0);
                            });
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_services,
            commands::add_service,
            commands::update_service,
            commands::remove_service,
            commands::start_service,
            commands::stop_service,
            commands::restart_service,
            commands::batch_start,
            commands::batch_stop,
            commands::get_system_resources,
            commands::get_service_resources,
            commands::get_resource_history,
            commands::get_recent_logs,
            commands::search_logs,
            commands::get_history_logs,
            commands::get_config_raw,
            commands::save_config_raw,
            commands::validate_config,
            commands::handle_process_started,
            commands::handle_process_stopped,
            commands::handle_process_error,
            commands::shutdown_all_services,
            commands::toggle_tray_popup,
            commands::hide_tray_popup,
            commands::show_main_window,
            commands::get_app_preferences,
            commands::set_quit_when_close_main,
            commands::set_launch_at_login_pref,
            commands::quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("启动应用失败");
}
