use crate::models::{ServiceResource, SystemResource};
use std::process::Command;
use sysinfo::System;

/// GPU 采集结果（名称、利用率、显存、渲染/Tiler、核心数）
struct GpuInfo {
    name: Option<String>,
    percent: Option<f32>,
    memory_used_mb: Option<f64>,
    memory_total_mb: Option<f64>,
    renderer_percent: Option<f32>,
    tiler_percent: Option<f32>,
    core_count: Option<u32>,
}

impl GpuInfo {
    fn empty() -> Self {
        Self {
            name: None,
            percent: None,
            memory_used_mb: None,
            memory_total_mb: None,
            renderer_percent: None,
            tiler_percent: None,
            core_count: None,
        }
    }
}

/// ioreg PerformanceStatistics 解析结果
struct IoregPerfStats {
    device_percent: Option<f32>,
    memory_used_mb: Option<f64>,
    renderer_percent: Option<f32>,
    tiler_percent: Option<f32>,
}

pub struct ResourceMonitor;

impl ResourceMonitor {
    /// 获取系统资源快照（含 CPU/GPU 细项，供历史面积图使用）
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
        let cpu_percent = sys.global_cpu_usage().min(100.0).max(0.0);

        // CPU 用户/系统/闲置拆分（平台可用时填充；否则用总量推导闲置）
        let (cpu_user, cpu_system, cpu_idle) = Self::get_cpu_breakdown(cpu_percent);

        // GPU 信息采集
        let gpu_info = Self::get_gpu_info(memory_total);

        SystemResource {
            cpu_percent,
            memory_used_gb,
            memory_total_gb,
            memory_percent: memory_percent as f32,
            gpu_name: gpu_info.name,
            gpu_percent: gpu_info.percent,
            gpu_memory_used_mb: gpu_info.memory_used_mb,
            gpu_memory_total_mb: gpu_info.memory_total_mb,
            cpu_user_percent: cpu_user,
            cpu_system_percent: cpu_system,
            cpu_idle_percent: cpu_idle,
            gpu_renderer_percent: gpu_info.renderer_percent,
            gpu_tiler_percent: gpu_info.tiler_percent,
            gpu_core_count: gpu_info.core_count,
        }
    }

    /// 获取 CPU 用户/系统/闲置占比。
    ///
    /// 输入：`fallback_used` — sysinfo 全局 CPU 使用率，拆分失败时用于推导闲置
    /// 输出：(user, system, idle)，单位均为 0~100
    ///
    /// macOS 优先解析 `top -l 1` 的 "CPU usage: x% user, y% sys, z% idle"；
    /// 其他平台用 fallback_used 与 100-fallback 填充 used/idle。
    fn get_cpu_breakdown(fallback_used: f32) -> (Option<f32>, Option<f32>, Option<f32>) {
        #[cfg(target_os = "macos")]
        {
            if let Some(breakdown) = Self::parse_macos_top_cpu() {
                return breakdown;
            }
        }

        let used = fallback_used.min(100.0).max(0.0);
        let idle = (100.0 - used).max(0.0);
        // 无细项时：把 used 记为用户态，系统态为 0，便于前端仍能画 used/idle 堆叠
        (Some(used), Some(0.0), Some(idle))
    }

    /// 解析 macOS `top -l 1 -n 0` 输出中的 CPU 行
    #[cfg(target_os = "macos")]
    fn parse_macos_top_cpu() -> Option<(Option<f32>, Option<f32>, Option<f32>)> {
        let output = Command::new("top")
            .args(["-l", "1", "-n", "0", "-s", "0"])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        Self::parse_top_cpu_line(&text)
    }

    /// 从 top 文本中提取 "CPU usage: a% user, b% sys, c% idle"
    ///
    /// 输入：top 命令完整输出
    /// 输出：Some((user, system, idle)) 或 None
    fn parse_top_cpu_line(text: &str) -> Option<(Option<f32>, Option<f32>, Option<f32>)> {
        for line in text.lines() {
            let lower = line.to_ascii_lowercase();
            if !lower.contains("cpu usage") && !lower.contains("cpu:") {
                continue;
            }
            let mut user = None;
            let mut system = None;
            let mut idle = None;
            for part in line.split(',') {
                let p = part.trim().to_ascii_lowercase();
                if let Some(v) = Self::extract_percent_before_label(&p, "user") {
                    user = Some(v);
                } else if let Some(v) = Self::extract_percent_before_label(&p, "sys") {
                    system = Some(v);
                } else if let Some(v) = Self::extract_percent_before_label(&p, "idle") {
                    idle = Some(v);
                }
            }
            if user.is_some() || system.is_some() || idle.is_some() {
                return Some((user, system, idle));
            }
        }
        None
    }

    /// 从形如 "5.26% user" 或 "cpu usage: 5.26% user" 的片段提取百分比数值
    fn extract_percent_before_label(part: &str, label: &str) -> Option<f32> {
        let idx = part.find(label)?;
        let before = &part[..idx];
        let pct_idx = before.rfind('%')?;
        let num_start = before[..pct_idx]
            .rfind(|c: char| !(c.is_ascii_digit() || c == '.' || c == ' '))
            .map(|i| i + 1)
            .unwrap_or(0);
        before[num_start..pct_idx].trim().parse::<f32>().ok()
    }

    /// 跨平台 GPU 信息采集
    ///
    /// 参数 `memory_total_bytes` 为系统总内存（bytes），用于 Apple Silicon
    /// 统一内存架构下设置 GPU 可用显存上限。
    fn get_gpu_info(memory_total_bytes: f64) -> GpuInfo {
        // 尝试 nvidia-smi（NVIDIA GPU，跨平台）
        if let Ok(output) = Command::new("nvidia-smi")
            .args([
                "--query-gpu=name,utilization.gpu,memory.used,memory.total",
                "--format=csv,noheader,nounits",
            ])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = text.lines().next() {
                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 4 {
                    return GpuInfo {
                        name: Some(parts[0].to_string()),
                        percent: parts[1].parse::<f32>().ok(),
                        memory_used_mb: parts[2].parse::<f64>().ok(),
                        memory_total_mb: parts[3].parse::<f64>().ok(),
                        renderer_percent: None,
                        tiler_percent: None,
                        core_count: None,
                    };
                }
            }
        }

        // macOS GPU 信息采集（Apple Silicon / AMD GPU）
        #[cfg(target_os = "macos")]
        {
            let mut info = GpuInfo::empty();

            if let Ok(output) = Command::new("system_profiler")
                .args(["SPDisplaysDataType"])
                .output()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                info.name = text.lines().find_map(|l| {
                    let l = l.trim();
                    if l.starts_with("Chipset Model:") || l.starts_with("Model:") {
                        l.splitn(2, ':').nth(1).map(|s| s.trim().to_string())
                    } else {
                        None
                    }
                });
            }

            if let Ok(output) = Command::new("ioreg")
                .args(["-r", "-d", "1", "-c", "IOAccelerator", "-w", "0"])
                .output()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                let text_str = text.to_string();

                let stats = Self::parse_ioreg_performance_stats(&text_str);
                info.percent = stats.device_percent;
                info.memory_used_mb = stats.memory_used_mb;
                info.renderer_percent = stats.renderer_percent;
                info.tiler_percent = stats.tiler_percent;
                info.core_count = Self::extract_ioreg_core_count(&text_str);

                if info.name.is_none() {
                    info.name = Self::extract_ioreg_model(&text_str);
                }
            }

            info.memory_total_mb = if memory_total_bytes > 0.0 {
                Some(memory_total_bytes / (1024.0 * 1024.0))
            } else {
                None
            };

            if info.name.is_some() || info.percent.is_some() {
                return info;
            }

            if let Ok(output) = Command::new("sysctl")
                .args(["-n", "machdep.cpu.brand_string"])
                .output()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                let name = text.trim();
                if !name.is_empty() {
                    info.name = Some(name.to_string());
                    return info;
                }
            }
        }

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
                    return GpuInfo {
                        name: gpu_name,
                        percent: None,
                        memory_used_mb: None,
                        memory_total_mb: None,
                        renderer_percent: None,
                        tiler_percent: None,
                        core_count: None,
                    };
                }
            }
        }

        GpuInfo::empty()
    }

    /// 从 ioreg 输出中解析 PerformanceStatistics 块
    ///
    /// 提取 Device / Renderer / Tiler 利用率与已用显存。
    fn parse_ioreg_performance_stats(text: &str) -> IoregPerfStats {
        let mut stats = IoregPerfStats {
            device_percent: None,
            memory_used_mb: None,
            renderer_percent: None,
            tiler_percent: None,
        };

        let flat = text.replace('\n', " ");

        if let Some(perf_pos) = flat.find("\"PerformanceStatistics\"") {
            let after_perf = &flat[perf_pos..];
            if let Some(brace_start) = after_perf.find('{') {
                let after_brace = &after_perf[brace_start + 1..];
                if let Some(brace_end) = after_brace.find('}') {
                    let block = &after_brace[..brace_end];

                    if let Some(val) = Self::extract_ioreg_value(block, "Device Utilization %") {
                        stats.device_percent = val.parse::<f32>().ok();
                    }
                    if let Some(val) = Self::extract_ioreg_value(block, "Renderer Utilization %") {
                        stats.renderer_percent = val.parse::<f32>().ok();
                    }
                    if let Some(val) = Self::extract_ioreg_value(block, "Tiler Utilization %") {
                        stats.tiler_percent = val.parse::<f32>().ok();
                    }
                    if let Some(val) = Self::extract_ioreg_value(block, "In use system memory") {
                        if let Ok(bytes) = val.parse::<u64>() {
                            stats.memory_used_mb = Some(bytes as f64 / (1024.0 * 1024.0));
                        }
                    }
                }
            }
        }

        stats
    }

    /// 从 ioreg stats 内容中按 key 提取对应的 value 字符串
    ///
    /// 支持 `"key"=value` 与 `"key" = value` 两种格式。
    fn extract_ioreg_value<'a>(stats: &'a str, key: &str) -> Option<&'a str> {
        let needle = format!("\"{}\"", key);
        let mut search_from = 0;

        while let Some(rel_pos) = stats[search_from..].find(&needle) {
            let abs_pos = search_from + rel_pos;
            let after_key = &stats[abs_pos + needle.len()..];
            let after_ws = after_key.trim_start();

            if let Some(rest) = after_ws.strip_prefix('=') {
                let after_eq = rest.trim_start();
                let mut end = after_eq.len();
                for (i, ch) in after_eq.char_indices() {
                    if ch == ',' || ch == '\n' || ch == '\r' || ch == '}' {
                        end = i;
                        break;
                    }
                    if i > 0 && ch == '"' {
                        end = i;
                        break;
                    }
                }
                let val_str = after_eq[..end].trim();
                if !val_str.is_empty() {
                    return Some(val_str);
                }
            }

            search_from = abs_pos + needle.len();
        }

        None
    }

    /// 从 ioreg 输出中提取 GPU model 名称
    fn extract_ioreg_model(text: &str) -> Option<String> {
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("\"model\"") {
                if let Some(eq_pos) = trimmed.find('=') {
                    let val = trimmed[eq_pos + 1..].trim().trim_matches('"').to_string();
                    if !val.is_empty() {
                        return Some(val);
                    }
                }
            }
        }
        None
    }

    /// 从 ioreg 输出提取 gpu-core-count
    fn extract_ioreg_core_count(text: &str) -> Option<u32> {
        let flat = text.replace('\n', " ");
        let val = Self::extract_ioreg_value(&flat, "gpu-core-count")?;
        val.parse::<u32>().ok()
    }

    /// 获取指定 PID 进程的资源
    pub fn get_process_resource(service_id: &str, pid: u32, uptime_secs: u64) -> ServiceResource {
        let mut sys = System::new_all();
        sys.refresh_all();

        if let Some(process) = sys.process(sysinfo::Pid::from(pid as usize)) {
            let cpu = process.cpu_usage();
            let memory = process.memory() as f64 / (1024.0 * 1024.0);

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

#[cfg(test)]
mod tests {
    use super::*;

    /// 真实 ioreg 单行输出（-w 0 紧凑格式）
    const REAL_IOREG_ONELINE: &str = "\
+-o AGXAcceleratorG17X  <class AGXAcceleratorG17X, id 0x1000006ce, registered, matched, active, busy 0 (4406 ms), retain 85>
    {
      \"SchedulerState\" = {\"Stamps\"=(),\"BusyWorkQueues\"=()}
      \"PerformanceStatistics\" = {\"In use system memory (driver)\"=0,\"Alloc system memory\"=4859609088,\"Tiler Utilization %\"=0,\"recoveryCount\"=0,\"lastRecoveryTime\"=0,\"Renderer Utilization %\"=0,\"TiledSceneBytes\"=851968,\"Device Utilization %\"=0,\"SplitSceneCount\"=0,\"Allocated PB Size\"=135266304,\"In use system memory\"=925630464}
      \"model\" = \"Apple M5 Pro\"
      \"gpu-core-count\" = 16
    }";

    const REAL_IOREG_MULTILINE: &str = "\
+-o AGXAcceleratorG17X  <class AGXAcceleratorG17X>
    {
      \"PerformanceStatistics\" = {
        \"In use system memory (driver)\" = 0
        \"Device Utilization %\" = 42
        \"Renderer Utilization %\" = 30
        \"Tiler Utilization %\" = 20
        \"In use system memory\" = 1048576
      }
      \"model\" = \"Apple M5 Pro\"
      \"gpu-core-count\" = 16
    }";

    #[test]
    fn test_parse_stats_oneline_real_output() {
        let stats = ResourceMonitor::parse_ioreg_performance_stats(REAL_IOREG_ONELINE);
        assert_eq!(stats.device_percent, Some(0.0));
        assert!((stats.memory_used_mb.unwrap() - 882.75).abs() < 0.001);
        assert_eq!(stats.renderer_percent, Some(0.0));
        assert_eq!(stats.tiler_percent, Some(0.0));
    }

    #[test]
    fn test_parse_stats_multiline_format() {
        let stats = ResourceMonitor::parse_ioreg_performance_stats(REAL_IOREG_MULTILINE);
        assert_eq!(stats.device_percent, Some(42.0));
        assert_eq!(stats.renderer_percent, Some(30.0));
        assert_eq!(stats.tiler_percent, Some(20.0));
        assert!((stats.memory_used_mb.unwrap() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_stats_driver_memory_not_mistaken_for_real() {
        let stats = ResourceMonitor::parse_ioreg_performance_stats(REAL_IOREG_ONELINE);
        assert_ne!(stats.memory_used_mb, Some(0.0));
        assert_eq!(stats.device_percent, Some(0.0));
    }

    #[test]
    fn test_parse_stats_no_performance_statistics() {
        let stats = ResourceMonitor::parse_ioreg_performance_stats("random text");
        assert_eq!(stats.device_percent, None);
        assert_eq!(stats.memory_used_mb, None);
    }

    #[test]
    fn test_parse_stats_missing_keys() {
        let text = "\"PerformanceStatistics\" = {\"recoveryCount\"=5,\"Allocated PB Size\"=1024}";
        let stats = ResourceMonitor::parse_ioreg_performance_stats(text);
        assert_eq!(stats.device_percent, None);
        assert_eq!(stats.memory_used_mb, None);
    }

    #[test]
    fn test_parse_stats_utilization_with_decimal() {
        let text = "\"PerformanceStatistics\" = {\"Device Utilization %\"=37.5,\"In use system memory\"=2097152}";
        let stats = ResourceMonitor::parse_ioreg_performance_stats(text);
        assert_eq!(stats.device_percent, Some(37.5));
        assert!((stats.memory_used_mb.unwrap() - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_extract_value_precise_key_matching() {
        let stats = "\"In use system memory (driver)\"=0,\"In use system memory\"=925630464";
        let val = ResourceMonitor::extract_ioreg_value(stats, "In use system memory");
        assert_eq!(val, Some("925630464"));
    }

    #[test]
    fn test_extract_value_compact_format() {
        let stats = "\"Device Utilization %\"=42,\"recoveryCount\"=5";
        let val = ResourceMonitor::extract_ioreg_value(stats, "Device Utilization %");
        assert_eq!(val, Some("42"));
    }

    #[test]
    fn test_extract_value_spaced_format() {
        let stats = "\"Device Utilization %\" = 42";
        let val = ResourceMonitor::extract_ioreg_value(stats, "Device Utilization %");
        assert_eq!(val, Some("42"));
    }

    #[test]
    fn test_extract_value_key_not_found() {
        let stats = "\"recoveryCount\"=5,\"Allocated PB Size\"=1024";
        let val = ResourceMonitor::extract_ioreg_value(stats, "Device Utilization %");
        assert_eq!(val, None);
    }

    #[test]
    fn test_extract_value_decimal_value() {
        let stats = "\"Device Utilization %\"=37.5";
        let val = ResourceMonitor::extract_ioreg_value(stats, "Device Utilization %");
        assert_eq!(val, Some("37.5"));
    }

    #[test]
    fn test_extract_value_terminates_at_comma() {
        let stats = "\"Device Utilization %\"=99,\"Next Key\"=1";
        let val = ResourceMonitor::extract_ioreg_value(stats, "Device Utilization %");
        assert_eq!(val, Some("99"));
    }

    #[test]
    fn test_extract_value_no_mismatch_on_suffix_key() {
        let stats =
            "\"Tiler Utilization %\"=10,\"Renderer Utilization %\"=20,\"Device Utilization %\"=30";
        let val = ResourceMonitor::extract_ioreg_value(stats, "Device Utilization %");
        assert_eq!(val, Some("30"));
    }

    #[test]
    fn test_extract_model_real_output() {
        let val = ResourceMonitor::extract_ioreg_model(REAL_IOREG_ONELINE);
        assert_eq!(val, Some("Apple M5 Pro".to_string()));
    }

    #[test]
    fn test_extract_model_spaced_format() {
        let text = "      \"model\" = \"Apple M2\"\n";
        let val = ResourceMonitor::extract_ioreg_model(text);
        assert_eq!(val, Some("Apple M2".to_string()));
    }

    #[test]
    fn test_extract_model_compact_format() {
        let text = "\"model\"=\"Apple M3 Max\"";
        let val = ResourceMonitor::extract_ioreg_model(text);
        assert_eq!(val, Some("Apple M3 Max".to_string()));
    }

    #[test]
    fn test_extract_model_not_found() {
        let val = ResourceMonitor::extract_ioreg_model("no model");
        assert_eq!(val, None);
    }

    #[test]
    fn test_extract_model_empty_value() {
        let text = "\"model\" = \"\"";
        let val = ResourceMonitor::extract_ioreg_model(text);
        assert_eq!(val, None);
    }

    #[test]
    fn test_extract_core_count() {
        assert_eq!(
            ResourceMonitor::extract_ioreg_core_count(REAL_IOREG_ONELINE),
            Some(16)
        );
        assert_eq!(
            ResourceMonitor::extract_ioreg_core_count(REAL_IOREG_MULTILINE),
            Some(16)
        );
    }

    #[test]
    fn test_parse_top_cpu_line() {
        let text = "CPU usage: 14.0% user, 7.0% sys, 79.0% idle\nProcesses: 500";
        let (u, s, i) = ResourceMonitor::parse_top_cpu_line(text).unwrap();
        assert_eq!(u, Some(14.0));
        assert_eq!(s, Some(7.0));
        assert_eq!(i, Some(79.0));
    }

    #[test]
    fn test_system_resource_has_all_gpu_fields() {
        let _resource = SystemResource {
            cpu_percent: 0.0,
            memory_used_gb: 0.0,
            memory_total_gb: 0.0,
            memory_percent: 0.0,
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
        };
    }
}
