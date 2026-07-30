use crate::models::{ServiceResource, SystemResource};
use std::process::Command;
use sysinfo::System;

pub struct ResourceMonitor;

impl ResourceMonitor {
    /// 获取系统资源快照
    pub fn get_system_resources() -> SystemResource {
        let mut sys = System::new_all();
        sys.refresh_all();

        let memory_total = sys.total_memory() as f64;
        let memory_used = sys.used_memory() as f64;

        let memory_total_gb = memory_total / (1024.0 * 1024.0 * 1024.0);
        let memory_used_gb = memory_used / (1024.0 * 1024.0 * 1024.0);
        let memory_percent = if memory_total > 0.0 {
            (memory_used / memory_total) * 100.0
        } else {
            0.0
        };

        // CPU 使用率：global_cpu_usage() 返回的是百分比(0~100)。
        // 必须在两次 refresh_all() 之间留出时间窗口，否则会基于极短窗口
        // 返回异常高值（甚至误报 100%）。
        sys.refresh_all();
        std::thread::sleep(std::time::Duration::from_millis(500));
        sys.refresh_all();
        let cpu_percent = sys.global_cpu_usage();

        // GPU 信息采集
        let gpu_info = Self::get_gpu_info();

        SystemResource {
            cpu_percent: cpu_percent.min(100.0).max(0.0),
            memory_used_gb,
            memory_total_gb,
            memory_percent: memory_percent as f32,
            gpu_name: gpu_info.0,
            gpu_percent: gpu_info.1,
            gpu_memory_used_mb: gpu_info.2,
            gpu_memory_total_mb: gpu_info.3,
        }
    }

    /// 跨平台 GPU 信息采集
    fn get_gpu_info() -> (Option<String>, Option<f32>, Option<f64>, Option<f64>) {
        // 尝试 nvidia-smi（NVIDIA GPU，跨平台）
        if let Ok(output) = Command::new("nvidia-smi")
            .args(["--query-gpu=name,utilization.gpu,memory.used,memory.total", "--format=csv,noheader,nounits"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = text.lines().next() {
                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 4 {
                    let name = Some(parts[0].to_string());
                    let util = parts[1].parse::<f32>().ok();
                    let mem_used = parts[2].parse::<f64>().ok();
                    let mem_total = parts[3].parse::<f64>().ok();
                    return (name, util, mem_used, mem_total);
                }
            }
        }

        // 尝试 macOS system_profiler（Apple Silicon / AMD GPU，最可靠地拿到型号）
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("system_profiler")
                .args(["SPDisplaysDataType"])
                .output()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                let gpu_name = text.lines().find_map(|l| {
                    let l = l.trim();
                    if l.starts_with("Chipset Model:") || l.starts_with("Model:") {
                        l.splitn(2, ':').nth(1).map(|s| s.trim().to_string())
                    } else {
                        None
                    }
                });
                if gpu_name.is_some() {
                    // macOS 没有公开的 GPU 利用率 API，返回型号即可
                    return (gpu_name, None, None, None);
                }
            }

            // 回退：ioreg 提取 model
            if let Ok(output) = Command::new("ioreg")
                .args(["-r", "-d", "1", "-c", "IOAccelerator", "-w", "0"])
                .output()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                let gpu_name = text
                    .lines()
                    .find(|l| l.contains("model"))
                    .and_then(|l| {
                        let parts: Vec<&str> = l.split('=').collect();
                        parts.last().map(|s| s.trim().trim_matches('"').to_string())
                    });
                if gpu_name.is_some() {
                    return (gpu_name, None, None, None);
                }
            }

            // 回退：sysctl 读取 CPU/芯片型号（Apple Silicon 上 CPU 与 GPU 共享芯片，
            // "Apple M1/M2/M3/M5 Pro" 既是 CPU 也是 GPU 名称）
            if let Ok(output) = Command::new("sysctl")
                .args(["-n", "machdep.cpu.brand_string"])
                .output()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                let name = text.trim();
                if !name.is_empty() {
                    return (Some(name.to_string()), None, None, None);
                }
            }
        }

        // 尝试 Windows WMI
        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = Command::new("wmic")
                .args(["path", "Win32_VideoController", "get", "name", "/format:csv"])
                .output()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                let gpu_name = text
                    .lines()
                    .nth(2)
                    .and_then(|l| l.split(',').nth(1))
                    .map(|s| s.trim().to_string());

                if gpu_name.is_some() {
                    return (gpu_name, None, None, None);
                }
            }
        }

        (None, None, None, None)
    }

    /// 获取指定 PID 进程的资源
    pub fn get_process_resource(service_id: &str, pid: u32, uptime_secs: u64) -> ServiceResource {
        let mut sys = System::new_all();
        sys.refresh_all();

        if let Some(process) = sys.process(sysinfo::Pid::from(pid as usize)) {
            let cpu = process.cpu_usage(); // 已是百分比(0~100)
            let memory = process.memory() as f64 / (1024.0 * 1024.0); // bytes -> MB

            ServiceResource {
                service_id: service_id.to_string(),
                cpu_percent: cpu.min(100.0).max(0.0),
                memory_mb: memory,
                pid,
                uptime_secs,
            }
        } else {
            ServiceResource {
                service_id: service_id.to_string(),
                cpu_percent: 0.0,
                memory_mb: 0.0,
                pid,
                uptime_secs,
            }
        }
    }
}
