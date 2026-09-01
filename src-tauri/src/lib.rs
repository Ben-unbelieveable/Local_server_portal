pub mod commands;
pub mod models;
pub mod services;

use services::local_api;
use services::preferences::{self, QUIT_WHEN_CLOSE_MAIN};
use services::resource_monitor::ResourceMonitor;
use services::service_manager::AppState;
use services::widget_sync;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState};
use tauri::Emitter;
use tauri::Listener;
use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;

/// 托盘弹窗因失焦隐藏后，短暂忽略随后的托盘点击，避免「失焦隐藏 → 托盘点击又立刻显示」竞态。
/// 值为 Unix 毫秒时间戳：在此时间之前的 toggle 若本意是「显示」，将被跳过。
pub static TRAY_IGNORE_SHOW_UNTIL_MS: AtomicU64 = AtomicU64::new(0);

/// 托盘点击处理中：忽略随后的 Reopen（否则会打开主窗并关掉弹窗）。
pub static TRAY_CLICK_UNTIL_MS: AtomicU64 = AtomicU64::new(0);

/// 弹窗刚显示：忽略紧随其后的失焦隐藏（点菜单栏本身会立刻让弹窗失焦）。
pub static TRAY_IGNORE_HIDE_UNTIL_MS: AtomicU64 = AtomicU64::new(0);

/// 上次菜单栏点击时间，用于区分单击 / 双击。
static TRAY_LAST_CLICK_MS: AtomicU64 = AtomicU64::new(0);

/// 单击延迟任务代数：新点击会作废尚未执行的「开弹窗」。
static TRAY_CLICK_GEN: AtomicU64 = AtomicU64::new(0);

/// 双击判定间隔（ms）。间隔内第二下只开主窗。
const TRAY_DOUBLE_CLICK_MS: u64 = 320;

/// 当前 Unix 毫秒时间戳
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        // 开机启动：社区方案 tauri-plugin-autostart（macOS LaunchAgent）
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ));

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_plugin_widgets::init());
    }

    builder
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

                    // macOS 桌面小组件：同步快照；拾取小组件起停请求
                    #[cfg(target_os = "macos")]
                    {
                        if let Some(action) = widget_sync::take_pending_action() {
                            let mgr = manager_monitor.clone();
                            let handle = app_handle_monitor.clone();
                            tauri::async_runtime::spawn(async move {
                                {
                                    let mut m = mgr.lock().await;
                                    let _ = match action.action.as_str() {
                                        "stop" => m.stop_service(&action.id).await,
                                        _ => m.start_service(&action.id).await,
                                    };
                                }
                                let (res, services) = {
                                    let m = mgr.lock().await;
                                    (
                                        ResourceMonitor::get_system_resources(),
                                        m.get_services(),
                                    )
                                };
                                widget_sync::push_snapshot_now(&handle, &res, &services);
                            });
                        }
                        let services_snapshot = {
                            let mgr = manager_monitor.lock().await;
                            mgr.get_services()
                        };
                        widget_sync::push_snapshot(
                            &app_handle_monitor,
                            &sys_res,
                            &services_snapshot,
                        );
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
                        if now_ms() < TRAY_IGNORE_HIDE_UNTIL_MS.load(Ordering::SeqCst) {
                            return;
                        }
                        let _ = hide_win.hide();
                        TRAY_IGNORE_SHOW_UNTIL_MS
                            .store(now_ms().saturating_add(350), Ordering::SeqCst);
                    }
                });
            }

            // 托盘：左键单击/双击；右键菜单（打开主窗口 / 退出）
            if let Some(tray) = app.tray_by_id("main") {
                if let Ok(menu) = build_tray_menu(&app_handle) {
                    let _ = tray.set_menu(Some(menu));
                    let _ = tray.set_show_menu_on_left_click(false);
                }
                tray.on_menu_event(|app, event| match event.id().as_ref() {
                    "tray_show_main" => reveal_main_window(app),
                    "tray_quit" => quit_app_from_tray(app),
                    _ => {}
                });
                let tray_app = app_handle.clone();
                tray.on_tray_icon_event(move |_tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button,
                        button_state,
                        ..
                    } = &event
                    {
                        if *button_state != MouseButtonState::Up {
                            return;
                        }
                        TRAY_CLICK_UNTIL_MS
                            .store(now_ms().saturating_add(800), Ordering::SeqCst);
                        if *button == MouseButton::Left {
                            handle_tray_icon_click(&tray_app);
                        }
                    }
                });
            }

            // 主窗口关闭：默认隐藏主窗 + 从程序坞移除（Accessory），托盘继续跑；
            // 偏好开启时彻底退出。社区方案：set_activation_policy(Accessory/Regular)。
            if let Some(main) = app.get_webview_window("main") {
                let main_for_hide = main.clone();
                let app_for_quit = app_handle.clone();
                main.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        if !QUIT_WHEN_CLOSE_MAIN.load(Ordering::SeqCst) {
                            api.prevent_close();
                            let _ = main_for_hide.hide();
                            // 主窗隐藏后切到 Accessory：程序坞标签消失，菜单栏托盘保留
                            #[cfg(target_os = "macos")]
                            {
                                let _ = app_for_quit
                                    .set_activation_policy(tauri::ActivationPolicy::Accessory);
                            }
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
            commands::get_network_info,
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
        .build(tauri::generate_context!())
        .expect("启动应用失败")
        .run(|app, event| {
            // 点击桌面小组件会触发 Reopen；从托盘隐藏主窗后需重新显示，避免白屏空窗
            if let tauri::RunEvent::Reopen { .. } = event {
                // 菜单栏单击/双击自己处理；这里只响应 Dock / 小组件。
                if now_ms() < TRAY_CLICK_UNTIL_MS.load(Ordering::SeqCst) {
                    return;
                }
                reveal_main_window(app);
            }
        });
}

/// 构建托盘右键菜单：打开主窗口、退出。
///
/// 输入：AppHandle
/// 输出：菜单；失败则调用方跳过挂载
fn build_tray_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let show_main = MenuItem::with_id(app, "tray_show_main", "打开主窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "tray_quit", "退出", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    Menu::with_items(app, &[&show_main, &sep, &quit])
}

/// 右键「退出」：停托管服务后结束进程。
fn quit_app_from_tray(app: &tauri::AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(state) = handle.try_state::<AppState>() {
            let mut mgr = state.manager.lock().await;
            let _ = mgr.shutdown_all().await;
        }
        handle.exit(0);
    });
}

/// 菜单栏单击开弹窗、双击开主窗；并挡住同一次点击触发的 Reopen。
///
/// 输入：AppHandle
/// 输出：无。间隔内第二下取消待执行的开弹窗。
fn handle_tray_icon_click(app: &tauri::AppHandle) {
    let now = now_ms();
    TRAY_CLICK_UNTIL_MS.store(now.saturating_add(800), Ordering::SeqCst);
    TRAY_IGNORE_HIDE_UNTIL_MS.store(
        now.saturating_add(TRAY_DOUBLE_CLICK_MS + 80),
        Ordering::SeqCst,
    );

    let last = TRAY_LAST_CLICK_MS.swap(now, Ordering::SeqCst);
    if last != 0 && now.saturating_sub(last) <= TRAY_DOUBLE_CLICK_MS {
        TRAY_CLICK_GEN.fetch_add(1, Ordering::SeqCst);
        TRAY_LAST_CLICK_MS.store(0, Ordering::SeqCst);
        reveal_main_window(app);
        return;
    }

    let gen = TRAY_CLICK_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(TRAY_DOUBLE_CLICK_MS)).await;
        if TRAY_CLICK_GEN.load(Ordering::SeqCst) != gen {
            return;
        }
        let _ = commands::show_tray_popup_impl(&app);
    });
}

/// 显示并聚焦主窗口（双击菜单栏 / 小组件 / Dock），同时关掉托盘弹窗。
/// 隐藏后再打开时 WKWebView 可能被系统回收，表现为白屏，因此强制 reload。
pub(crate) fn reveal_main_window(app: &tauri::AppHandle) {
    if let Some(popup) = app.get_webview_window("tray-popup") {
        let _ = popup.hide();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.unminimize();
        let _ = main.reload();
        let _ = main.set_focus();
    }
}
