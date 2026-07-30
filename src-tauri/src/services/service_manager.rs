use crate::models::{
    AppConfig, BatchResult, ServiceConfig, ServiceRuntime, ServiceStatus, StatusChangeEvent,
    SystemResource,
};
use crate::services::config_manager::{load_config, save_config};
use crate::services::log_manager::LogManager;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

pub struct ServiceManager {
    services: HashMap<String, ServiceRuntime>,
    config: AppConfig,
    log_manager: Arc<LogManager>,
    app_handle: AppHandle,
    /// 系统资源快照缓存，由监控循环每 tick 写入，供 get_system_resources 命令即时返回
    system_resource: Option<SystemResource>,
}

impl ServiceManager {
    pub fn new(app_handle: AppHandle) -> Result<Self, String> {
        let config = load_config()?;
        let log_manager = Arc::new(LogManager::new());

        let mut services = HashMap::new();
        for svc in &config.services {
            services.insert(
                svc.id.clone(),
                ServiceRuntime {
                    config: svc.clone(),
                    status: ServiceStatus::Stopped,
                    pid: None,
                    cpu_percent: 0.0,
                    memory_mb: 0.0,
                    uptime_secs: 0,
                    restart_count: 0,
                    last_error: None,
                },
            );
        }

        // 启动期一次性探测：若服务配置的端口已经在 LISTEN，
        // 说明该服务在应用启动前就已经在运行（例如用户在终端手动拉起）。
        // 直接标记为 Running 并写入监听进程 PID，避免首屏仪表盘把运行中的
        // 服务误显示成"已停止"，以及首屏 CPU/内存为 0 的问题。
        // 注意：这里只做"启动期"探测，运行期不把 Stopped 复活成 Running，
        // 否则会和"用户主动停止服务"的端口 TIME_WAIT 窗口冲突（停止后被误判复活）。
        for svc in &config.services {
            if let Some(port) = svc.port {
                if Self::port_is_listening(port) {
                    if let Some(runtime) = services.get_mut(&svc.id) {
                        runtime.status = ServiceStatus::Running;
                        runtime.pid = Self::pid_listening_on_port(port);
                        runtime.uptime_secs = 0;
                        runtime.last_error = None;
                    }
                }
            }
        }

        Ok(ServiceManager {
            services,
            config,
            log_manager,
            app_handle,
            system_resource: None,
        })
    }

    /// 获取所有服务运行时状态
    pub fn get_services(&self) -> Vec<ServiceRuntime> {
        let mut list: Vec<ServiceRuntime> = self.services.values().cloned().collect();
        list.sort_by(|a, b| a.config.name.cmp(&b.config.name));
        list
    }

    /// 获取单个服务
    pub fn get_service(&self, id: &str) -> Option<&ServiceRuntime> {
        self.services.get(id)
    }

    /// 获取缓存的系统资源快照（由监控循环每 tick 写入）。
    /// 命令层优先返回缓存，避免每次调用都触发约 500ms 的阻塞采集。
    pub fn get_system_resource(&self) -> Option<SystemResource> {
        self.system_resource.clone()
    }

    /// 监控循环写入最新系统资源快照。
    pub fn set_system_resource(&mut self, res: SystemResource) {
        self.system_resource = Some(res);
    }

    /// 添加服务
    pub fn add_service(&mut self, config: ServiceConfig) -> Result<ServiceRuntime, String> {
        if self.services.contains_key(&config.id) {
            return Err(format!("服务 ID '{}' 已存在，请更换", config.id));
        }

        if config.command.trim().is_empty() {
            return Err("启动命令不能为空".to_string());
        }

        let runtime = ServiceRuntime {
            config: config.clone(),
            status: ServiceStatus::Stopped,
            pid: None,
            cpu_percent: 0.0,
            memory_mb: 0.0,
            uptime_secs: 0,
            restart_count: 0,
            last_error: None,
        };

        self.services.insert(config.id.clone(), runtime.clone());
        self.config.services.push(config);
        save_config(&self.config)?;

        Ok(runtime)
    }

    /// 编辑服务
    pub fn update_service(
        &mut self,
        id: &str,
        config: ServiceConfig,
    ) -> Result<ServiceRuntime, String> {
        let existing = self
            .services
            .get(id)
            .ok_or_else(|| format!("服务 '{}' 不存在", id))?;

        let was_running = existing.status == ServiceStatus::Running;

        let runtime = ServiceRuntime {
            config: config.clone(),
            status: if was_running {
                ServiceStatus::Running
            } else {
                ServiceStatus::Stopped
            },
            pid: existing.pid,
            cpu_percent: existing.cpu_percent,
            memory_mb: existing.memory_mb,
            uptime_secs: existing.uptime_secs,
            restart_count: existing.restart_count,
            last_error: None,
        };

        self.services.insert(id.to_string(), runtime.clone());

        // 更新配置
        if let Some(svc) = self.config.services.iter_mut().find(|s| s.id == id) {
            *svc = config;
        }
        save_config(&self.config)?;

        Ok(runtime)
    }

    /// 删除服务
    pub fn remove_service(&mut self, id: &str, stop_first: bool) -> Result<(), String> {
        if stop_first {
            let _ = self.stop_service_internal(id, true);
        }

        self.services.remove(id);
        self.config.services.retain(|s| s.id != id);
        save_config(&self.config)?;

        Ok(())
    }

    /// 启动服务（异步）
    pub async fn start_service(&mut self, id: &str) -> Result<(), String> {
        // 先提取需要的数据，避免借用冲突
        let (command, work_dir, env, _name) = {
            let service = self
                .services
                .get(id)
                .ok_or_else(|| format!("服务 '{}' 不存在", id))?;

            if service.status == ServiceStatus::Running
                || service.status == ServiceStatus::Starting
            {
                return Err(format!("服务 '{}' 已在运行中", service.config.name));
            }

            // 检查启动脚本是否存在
            if let Some(script_path) = Self::extract_script_path(&service.config.command) {
                if !std::path::Path::new(&script_path).exists() {
                    let error_msg = format!("启动脚本不存在: {}", script_path);
                    self.set_service_error(id, error_msg.clone());
                    self.emit_status_change(id, ServiceStatus::Failed, None);
                    return Err(error_msg);
                }
            }

            (
                service.config.command.clone(),
                service.config.work_dir.clone(),
                service.config.env.clone(),
                service.config.name.clone(),
            )
        };

        // 更新状态
        self.set_service_status(id, ServiceStatus::Starting, None);
        self.emit_status_change(id, ServiceStatus::Starting, None);

        // 解析命令
        let (program, args) = Self::parse_command(&command);

        let service_id = id.to_string();
        let app_handle = self.app_handle.clone();
        let log_manager = self.log_manager.clone();

        // 在后台线程中启动进程
        tokio::spawn(async move {
            let result = Self::spawn_process(
                &program,
                &args,
                work_dir.as_deref(),
                &env,
                &service_id,
                app_handle.clone(),
                log_manager.clone(),
            )
            .await;

            match result {
                Ok(pid) => {
                    let _ = app_handle.emit(
                        "service-process-started",
                        serde_json::json!({
                            "service_id": service_id,
                            "pid": pid,
                        }),
                    );
                }
                Err(e) => {
                    let _ = app_handle.emit(
                        "service-process-error",
                        serde_json::json!({
                            "service_id": service_id,
                            "error": e,
                        }),
                    );
                }
            }
        });

        Ok(())
    }

    /// 处理进程启动成功事件
    pub fn on_process_started(&mut self, id: &str, pid: u32) {
        if let Some(service) = self.services.get_mut(id) {
            service.pid = Some(pid);
            service.status = ServiceStatus::Running;
            service.uptime_secs = 0;
            service.last_error = None;
        }
        self.emit_status_change(id, ServiceStatus::Running, Some(pid));
    }

    /// 处理进程启动失败事件
    pub fn on_process_error(&mut self, id: &str, error: &str) {
        self.set_service_error(id, error.to_string());
        self.emit_status_change(id, ServiceStatus::Failed, None);
    }

    /// 停止服务（异步）
    pub async fn stop_service(&mut self, id: &str) -> Result<(), String> {
        self.stop_service_internal(id, false)
    }

    fn stop_service_internal(&mut self, id: &str, force: bool) -> Result<(), String> {
        let service = self
            .services
            .get(id)
            .ok_or_else(|| format!("服务 '{}' 不存在", id))?;

        if service.status == ServiceStatus::Stopped
            || service.status == ServiceStatus::Failed
            || service.status == ServiceStatus::Error
        {
            if !force {
                return Err(format!("服务 '{}' 未在运行", service.config.name));
            }
            return Ok(());
        }

        let pid = match service.pid {
            Some(p) => p,
            None => {
                self.set_service_status(id, ServiceStatus::Stopped, None);
                return Ok(());
            }
        };

        let timeout = service.config.stop_timeout;
        let service_id = id.to_string();
        let app_handle = self.app_handle.clone();

        self.set_service_status(id, ServiceStatus::Stopping, None);
        self.emit_status_change(id, ServiceStatus::Stopping, Some(pid));

        // 在后台停止进程
        tokio::spawn(async move {
            let result = Self::kill_process(pid, timeout).await;
            match result {
                Ok(killed) => {
                    let _ = app_handle.emit(
                        "service-process-stopped",
                        serde_json::json!({
                            "service_id": service_id,
                            "forced": !killed,
                        }),
                    );
                }
                Err(e) => {
                    let _ = app_handle.emit(
                        "service-process-error",
                        serde_json::json!({
                            "service_id": service_id,
                            "error": format!("停止失败: {}", e),
                        }),
                    );
                }
            }
        });

        Ok(())
    }

    /// 处理进程停止事件
    pub fn on_process_stopped(&mut self, id: &str) {
        if let Some(service) = self.services.get_mut(id) {
            service.pid = None;
            service.status = ServiceStatus::Stopped;
            service.cpu_percent = 0.0;
            service.memory_mb = 0.0;
            service.uptime_secs = 0;
        }
        self.emit_status_change(id, ServiceStatus::Stopped, None);
    }

    /// 重启服务
    pub async fn restart_service(&mut self, id: &str) -> Result<(), String> {
        let service = self
            .services
            .get(id)
            .ok_or_else(|| format!("服务 '{}' 不存在", id))?;

        if service.status == ServiceStatus::Running {
            self.stop_service_internal(id, true)?;
            // 等待停止完成（简单延迟）
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        self.start_service(id).await
    }

    /// 批量启动
    pub async fn batch_start(&mut self, ids: &[String]) -> Vec<BatchResult> {
        let mut results = Vec::new();
        for id in ids {
            match self.start_service(id).await {
                Ok(()) => results.push(BatchResult {
                    service_id: id.clone(),
                    success: true,
                    message: "启动成功".to_string(),
                }),
                Err(e) => {
                    if e.contains("已在运行中") {
                        results.push(BatchResult {
                            service_id: id.clone(),
                            success: true,
                            message: "已在运行中，跳过".to_string(),
                        });
                    } else {
                        results.push(BatchResult {
                            service_id: id.clone(),
                            success: false,
                            message: e,
                        });
                    }
                }
            }
        }
        results
    }

    /// 批量停止
    pub async fn batch_stop(&mut self, ids: &[String]) -> Vec<BatchResult> {
        let mut results = Vec::new();
        for id in ids {
            match self.stop_service(id).await {
                Ok(()) => results.push(BatchResult {
                    service_id: id.clone(),
                    success: true,
                    message: "停止成功".to_string(),
                }),
                Err(e) => results.push(BatchResult {
                    service_id: id.clone(),
                    success: false,
                    message: e,
                }),
            }
        }
        results
    }

    /// 停止所有运行中的服务（退出时调用）
    pub async fn shutdown_all(&mut self) -> Vec<BatchResult> {
        let running_ids: Vec<String> = self
            .services
            .iter()
            .filter(|(_, s)| s.status == ServiceStatus::Running)
            .map(|(id, _)| id.clone())
            .collect();

        self.batch_stop(&running_ids).await
    }

    /// 更新服务资源数据
    pub fn update_resource(&mut self, id: &str, cpu: f32, mem: f64) {
        if let Some(service) = self.services.get_mut(id) {
            service.cpu_percent = cpu;
            service.memory_mb = mem;
            service.uptime_secs += 1; // 监控循环每 1 秒刷新一次
        }
    }

    /// 检测异常退出 / 复活误判的服务
    ///
    /// 状态机判定优先级（按"是否反映服务真实可达性"由强到弱）：
    /// 1. 端口监听（lsof）—— 最强证据，反映服务真实可达
    /// 2. 进程 PID 存活
    /// 3. 进程 PID 死 + 端口死 → 真正异常退出
    ///
    /// 历史背景：很多启动脚本是 `bash deploy.sh` 这种 short-lived shell，
    /// 脚本内部用 `nohup ... &` 启动真实服务后立即退出。父 shell PID 死
    /// 了之后，被 init 收养的子进程 PPID 变 1，PPID 链完全断了，单纯
    /// 看 PID 会把活着的服务误判为 Error。所以改用端口探测。
    pub fn check_process_alive(&mut self) {
        let mut recovered: Vec<(String, u32)> = Vec::new(); // pid 修正
        let mut promoted: Vec<(String, u32)> = Vec::new();  // Error -> Running 复活
        let mut errored: Vec<(String, String)> = Vec::new();

        for (id, service) in self.services.iter() {
            let port = service.config.port;

            // 1) 端口监听：判定服务真实可达
            let port_alive = port
                .map(|p| Self::port_is_listening(p))
                .unwrap_or(false);
            let port_pid = port.and_then(Self::pid_listening_on_port);

            // 状态：Error / Failed —— 之前误判的，端口若又监听了 → 复活
            if service.status == ServiceStatus::Error
                || service.status == ServiceStatus::Failed
            {
                if port_alive {
                    if let Some(new_pid) = port_pid {
                        promoted.push((id.clone(), new_pid));
                    }
                }
                continue;
            }

            // 状态：仅对 Running 做存活检测
            if service.status != ServiceStatus::Running {
                continue;
            }

            // 2) PID 活着 → 没事
            if let Some(pid) = service.pid {
                if Self::is_process_alive(pid) {
                    continue;
                }
            }

            // 3) PID 死了。看看端口是不是还在监听。
            if port_alive {
                if let Some(new_pid) = port_pid {
                    recovered.push((id.clone(), new_pid));
                } else {
                    // 端口在但拿不到 PID（权限/瞬态），仍然算活着
                    recovered.push((id.clone(), service.pid.unwrap_or(0)));
                }
                continue;
            }

            // 4) 真正死了
            errored.push((
                id.clone(),
                format!(
                    "进程异常退出 (PID: {})",
                    service.pid.unwrap_or(0)
                ),
            ));
        }

        for (id, new_pid) in recovered {
            if let Some(svc) = self.services.get_mut(&id) {
                svc.pid = Some(new_pid);
                svc.last_error = None;
            }
            // pid 修正（保持 Running 状态），无需 status change 事件
        }
        for (id, new_pid) in promoted {
            if let Some(svc) = self.services.get_mut(&id) {
                svc.pid = Some(new_pid);
                svc.status = ServiceStatus::Running;
                svc.uptime_secs = 0;
                svc.last_error = None;
            }
            self.emit_status_change(&id, ServiceStatus::Running, Some(new_pid));
        }
        for (id, msg) in errored {
            self.set_service_error(&id, msg);
            self.emit_status_change(&id, ServiceStatus::Error, None);
        }
    }

    /// 用 lsof 探测指定端口是否在 LISTEN（macOS / Linux 通用）
    pub fn port_is_listening(port: u16) -> bool {
        use std::process::Command;
        // BSD lsof 要求 -i 参数整体作为一项（"-iTCP:5173"），不能拆成 "-iTCP" + "5173"
        let out = Command::new("lsof")
            .args([
                "-nP",
                &format!("-iTCP:{}", port),
                "-sTCP:LISTEN",
                "-t",
            ])
            .output();
        match out {
            Ok(o) => !String::from_utf8_lossy(&o.stdout).trim().is_empty(),
            Err(_) => false,
        }
    }

    /// 获取占用该端口的进程 PID（取第一个 LISTEN 进程）
    pub fn pid_listening_on_port(port: u16) -> Option<u32> {
        use std::process::Command;
        let out = Command::new("lsof")
            .args([
                "-nP",
                &format!("-iTCP:{}", port),
                "-sTCP:LISTEN",
                "-t",
            ])
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&out.stdout);
        s.trim()
            .lines()
            .next()
            .and_then(|line| line.trim().parse::<u32>().ok())
    }

    /// 获取日志管理器引用
    pub fn log_manager(&self) -> Arc<LogManager> {
        self.log_manager.clone()
    }

    // --- 内部辅助方法 ---

    fn set_service_status(&mut self, id: &str, status: ServiceStatus, pid: Option<u32>) {
        if let Some(service) = self.services.get_mut(id) {
            service.status = status;
            if pid.is_some() {
                service.pid = pid;
            }
        }
    }

    fn set_service_error(&mut self, id: &str, error: String) {
        if let Some(service) = self.services.get_mut(id) {
            service.status = ServiceStatus::Error;
            service.last_error = Some(error.clone());
            service.pid = None;
            service.cpu_percent = 0.0;
            service.memory_mb = 0.0;
        }
    }

    fn emit_status_change(&self, id: &str, status: ServiceStatus, pid: Option<u32>) {
        let event = StatusChangeEvent {
            service_id: id.to_string(),
            status: status.as_str().to_string(),
            pid,
            error: if status == ServiceStatus::Failed || status == ServiceStatus::Error {
                self.services
                    .get(id)
                    .and_then(|s| s.last_error.clone())
            } else {
                None
            },
        };
        let _ = self.app_handle.emit("service-status-changed", event);
    }

    /// 解析命令字符串为程序和参数
    fn parse_command(command: &str) -> (String, Vec<String>) {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return (String::new(), vec![]);
        }
        let program = parts[0].to_string();
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
        (program, args)
    }

    /// 从 bash/sh 命令中提取脚本路径
    fn extract_script_path(command: &str) -> Option<String> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.len() >= 2 {
            let prog = parts[0];
            if prog == "bash" || prog == "sh" || prog == "zsh" {
                // 找到 .sh 结尾的参数
                for arg in &parts[1..] {
                    if arg.ends_with(".sh") {
                        return Some(arg.to_string());
                    }
                }
            }
        }
        None
    }

    /// 启动子进程
    async fn spawn_process(
        program: &str,
        args: &[String],
        work_dir: Option<&str>,
        env: &HashMap<String, String>,
        service_id: &str,
        _app_handle: AppHandle,
        log_manager: Arc<LogManager>,
    ) -> Result<u32, String> {
        use std::process::{Command, Stdio};

        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Some(dir) = work_dir {
            cmd.current_dir(dir);
        }

        for (key, value) in env {
            cmd.env(key, value);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("启动进程失败: {}", e))?;

        let pid = child.id();

        // 异步读取 stdout/stderr
        let sid = service_id.to_string();
        if let Some(stdout) = child.stdout.take() {
            let lm = log_manager.clone();
            let id = sid.clone();
            tokio::spawn(async move {
                use std::io::{BufRead, BufReader};
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(text) = line {
                        lm.append(&id, "stdout", &text);
                    }
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let lm = log_manager.clone();
            let id = sid.clone();
            tokio::spawn(async move {
                use std::io::{BufRead, BufReader};
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(text) = line {
                        lm.append(&id, "stderr", &text);
                    }
                }
            });
        }

        // 后台等待进程退出（检测异常退出）
        let id = sid.clone();
        tokio::spawn(async move {
            let status = child.wait();
            match status {
                Ok(exit_status) => {
                    if !exit_status.success() {
                        log_manager.append(
                            &id,
                            "stderr",
                            &format!(
                                "进程退出，退出码: {}",
                                exit_status.code().unwrap_or(-1)
                            ),
                        );
                    }
                }
                Err(e) => {
                    log_manager.append(&id, "stderr", &format!("等待进程退出时出错: {}", e));
                }
            }
        });

        Ok(pid)
    }

    /// 终止进程
    async fn kill_process(pid: u32, timeout_secs: u64) -> Result<bool, String> {
        // 先发送 SIGTERM
        #[cfg(unix)]
        {
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }
        #[cfg(windows)]
        {
            // Windows: 使用 taskkill
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string()])
                .output();
        }

        // 等待进程退出
        let start = std::time::Instant::now();
        loop {
            if !Self::is_process_alive(pid) {
                return Ok(true);
            }
            if start.elapsed().as_secs() >= timeout_secs {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        // 超时，SIGKILL
        #[cfg(unix)]
        {
            unsafe {
                libc::kill(pid as i32, libc::SIGKILL);
            }
        }
        #[cfg(windows)]
        {
            let _ = Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .output();
        }

        // 递归清理子进程
        Self::kill_child_processes(pid);

        Ok(false) // false 表示经过了强制终止
    }

    /// 清理子进程
    fn kill_child_processes(pid: u32) {
        #[cfg(unix)]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("pgrep").args(["-P", &pid.to_string()]).output() {
                let child_pids: Vec<u32> = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter_map(|l| l.trim().parse().ok())
                    .collect();
                for child_pid in child_pids {
                    unsafe {
                        libc::kill(child_pid as i32, libc::SIGKILL);
                    }
                }
            }
        }
        #[cfg(windows)]
        {
            // Windows 上用 taskkill /T 会杀进程树
            use std::process::Command;
            let _ = Command::new("taskkill")
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .output();
        }
    }

    /// 检查进程是否存活
    fn is_process_alive(pid: u32) -> bool {
        #[cfg(unix)]
        {
            unsafe { libc::kill(pid as i32, 0) == 0 }
        }
        #[cfg(windows)]
        {
            use std::process::Command;
            Command::new("tasklist")
                .args(["/FI", &format!("PID eq {}", pid)])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
                .unwrap_or(false)
        }
    }
}

/// 全局状态包装
pub struct AppState {
    pub manager: Arc<Mutex<ServiceManager>>,
}

impl AppState {
    pub fn new(app_handle: AppHandle) -> Result<Self, String> {
        let manager = ServiceManager::new(app_handle)?;
        Ok(AppState {
            manager: Arc::new(Mutex::new(manager)),
        })
    }
}
