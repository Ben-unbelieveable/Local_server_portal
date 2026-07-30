pub mod commands;
pub mod models;
pub mod services;

use services::resource_monitor::ResourceMonitor;
use services::service_manager::AppState;
use std::sync::Arc;
use tauri::Manager;
use tauri::Emitter;
use tauri::Listener;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

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

                    // 写入缓存，供 get_system_resources 命令即时返回（避免每次调用都重算约 500ms）
                    {
                        let mut mgr = manager_monitor.lock().await;
                        mgr.set_system_resource(sys_res.clone());
                    }

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
                                mgr.update_resource(&service.config.id, res.cpu_percent, res.memory_mb);
                                svc_resources.push(res);
                            }
                        }
                        // 检测异常退出
                        mgr.check_process_alive();
                    }

                    // 推送资源更新事件
                    let _ = app_handle_monitor.emit(
                        "resource-update",
                        crate::models::ResourceUpdateEvent {
                            system: sys_res,
                            services: svc_resources,
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
                        let _ = handle.emit("show-notification", serde_json::json!({
                            "title": "服务异常",
                            "body": format!("服务异常退出: {}", error_msg),
                        }));
                    });
                }
            });

            // 管理 AppState
            app.manage(state);

            // 创建托盘弹窗窗口（默认隐藏，点击托盘图标时显示）
            let _tray_popup = tauri::WebviewWindowBuilder::new(
                &app_handle,
                "tray-popup",
                tauri::WebviewUrl::App("index.html#/tray-popup".into()),
            )
            .title("服务快速管理")
            .inner_size(380.0, 640.0)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .visible(false)
            .focused(false)
            .build();

            // 给配置自动创建的托盘图标（tauri.conf.json 中 trayIcon.id = "main"）挂上点击事件回调。
            //
            // 之所以在 Rust 端处理点击事件，而不是沿用前端 useSystemTray.ts 中的
            // TrayIcon.removeById + TrayIcon.new 逻辑，是因为：
            //   1. JS 端 defaultWindowIcon() 在未配置应用默认窗口图标时返回 null；
            //   2. 用 null 图标重建托盘后，macOS 不会显示无图标的托盘项；
            //   3. 配置自动创建的托盘本身带有正确图标，因此只需复用它并挂载事件即可。
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
        ])
        .run(tauri::generate_context!())
        .expect("启动应用失败");
}
