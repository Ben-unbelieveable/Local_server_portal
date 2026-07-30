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
        // 传入系统总内存（bytes），用于 Apple Silicon 统一内存架构下设置 GPU 显存上限
        let gpu_info = Self::get_gpu_info(memory_total);

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
    ///
    /// 参数 `memory_total_bytes` 为系统总内存（bytes），用于 Apple Silicon
    /// 统一内存架构下设置 GPU 可用显存上限。
    fn get_gpu_info(memory_total_bytes: f64) -> (Option<String>, Option<f32>, Option<f64>, Option<f64>) {
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

        // macOS GPU 信息采集（Apple Silicon / AMD GPU）
        #[cfg(target_os = "macos")]
        {
            // Step 1: 用 system_profiler 获取 GPU 名称（最可靠）
            let mut gpu_name: Option<String> = None;
            if let Ok(output) = Command::new("system_profiler")
                .args(["SPDisplaysDataType"])
                .output()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                gpu_name = text.lines().find_map(|l| {
                    let l = l.trim();
                    if l.starts_with("Chipset Model:") || l.starts_with("Model:") {
                        l.splitn(2, ':').nth(1).map(|s| s.trim().to_string())
                    } else {
                        None
                    }
                });
            }

            // Step 2: 用 ioreg 解析 PerformanceStatistics，获取 GPU 利用率和显存使用量
            // ioreg 的 PerformanceStatistics 包含 "Device Utilization %" 和
            // "In use system memory" 等字段。不论 system_profiler 是否成功获取到
            // 名称，都执行 ioreg 以获取利用率数据。
            let mut gpu_percent: Option<f32> = None;
            let mut gpu_memory_used_mb: Option<f64> = None;

            if let Ok(output) = Command::new("ioreg")
                .args(["-r", "-d", "1", "-c", "IOAccelerator", "-w", "0"])
                .output()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                let text_str = text.to_string();

                // 解析 PerformanceStatistics 中的 GPU 利用率和显存
                let (percent, mem_used) = Self::parse_ioreg_performance_stats(&text_str);
                gpu_percent = percent;
                gpu_memory_used_mb = mem_used;

                // 如果 system_profiler 未获取到名称，尝试从 ioreg 的 "model" 字段获取
                if gpu_name.is_none() {
                    gpu_name = Self::extract_ioreg_model(&text_str);
                }
            }

            // GPU 总显存：Apple Silicon 采用统一内存架构，GPU 可访问全部系统内存
            let gpu_memory_total_mb = if memory_total_bytes > 0.0 {
                Some(memory_total_bytes / (1024.0 * 1024.0))
            } else {
                None
            };

            if gpu_name.is_some() || gpu_percent.is_some() {
                return (gpu_name, gpu_percent, gpu_memory_used_mb, gpu_memory_total_mb);
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
                    return (Some(name.to_string()), None, None, gpu_memory_total_mb);
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

    /// 从 ioreg 输出中解析 PerformanceStatistics 块，提取 GPU 利用率和已用显存
    ///
    /// ioreg 的 PerformanceStatistics 格式（单行，-w 0）：
    /// ```text
    /// "PerformanceStatistics" = {"Device Utilization %"=0,"In use system memory"=765706240,...}
    /// ```
    ///
    /// 也兼容多行格式：
    /// ```text
    /// "PerformanceStatistics" = {
    ///     "Device Utilization %" = 0
    ///     "In use system memory" = 765706240
    /// }
    /// ```
    fn parse_ioreg_performance_stats(text: &str) -> (Option<f32>, Option<f64>) {
        let mut gpu_percent: Option<f32> = None;
        let mut gpu_memory_used_mb: Option<f64> = None;

        // 将多行输出扁平化为单行，简化解析逻辑
        let flat = text.replace('\n', " ");

        // 定位 PerformanceStatistics 块内容（在 { 与匹配的 } 之间）
        if let Some(perf_pos) = flat.find("\"PerformanceStatistics\"") {
            let after_perf = &flat[perf_pos..];
            if let Some(brace_start) = after_perf.find('{') {
                let after_brace = &after_perf[brace_start + 1..];
                if let Some(brace_end) = after_brace.find('}') {
                    let stats = &after_brace[..brace_end];

                    // 提取 "Device Utilization %"（0~100 的浮点数）
                    if let Some(val) = Self::extract_ioreg_value(stats, "Device Utilization %") {
                        gpu_percent = val.parse::<f32>().ok();
                    }

                    // 提取 "In use system memory"（bytes → MB）
                    if let Some(val) = Self::extract_ioreg_value(stats, "In use system memory") {
                        if let Ok(bytes) = val.parse::<u64>() {
                            gpu_memory_used_mb = Some(bytes as f64 / (1024.0 * 1024.0));
                        }
                    }
                }
            }
        }

        (gpu_percent, gpu_memory_used_mb)
    }

    /// 从 ioreg stats 内容中按 key 提取对应的 value 字符串
    ///
    /// 支持两种格式：
    /// - `"key"=value`（-w 0 紧凑格式）
    /// - `"key" = value`（带空格的可读格式）
    ///
    /// value 以逗号、换行、或下一个引号开头的 key 为终止符。
    fn extract_ioreg_value<'a>(stats: &'a str, key: &str) -> Option<&'a str> {
        let needle = format!("\"{}\"", key);
        let mut search_from = 0;

        while let Some(rel_pos) = stats[search_from..].find(&needle) {
            let abs_pos = search_from + rel_pos;
            let after_key = &stats[abs_pos + needle.len()..];
            let after_ws = after_key.trim_start();

            if let Some(rest) = after_ws.strip_prefix('=') {
                let after_eq = rest.trim_start();
                // value 终止符：逗号、换行、或下一个引号开头的 key
                let mut end = after_eq.len();
                for (i, ch) in after_eq.char_indices() {
                    if ch == ',' || ch == '\n' || ch == '\r' {
                        end = i;
                        break;
                    }
                    // 跳过值首字符后遇到的引号（下一个 key 的开始）
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
    ///
    /// ioreg 输出中包含 `"model" = "Apple M5 Pro"` 这样的行。
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 真实 ioreg 单行输出（-w 0 紧凑格式），包含关键的边界情况：
    /// "In use system memory (driver)" 出现在 "In use system memory" 之前
    const REAL_IOREG_ONELINE: &str = "\
+-o AGXAcceleratorG17X  <class AGXAcceleratorG17X, id 0x1000006ce, registered, matched, active, busy 0 (4406 ms), retain 85>
    {
      \"SchedulerState\" = {\"Stamps\"=(),\"BusyWorkQueues\"=()}
      \"PerformanceStatistics\" = {\"In use system memory (driver)\"=0,\"Alloc system memory\"=4859609088,\"Tiler Utilization %\"=0,\"recoveryCount\"=0,\"lastRecoveryTime\"=0,\"Renderer Utilization %\"=0,\"TiledSceneBytes\"=851968,\"Device Utilization %\"=0,\"SplitSceneCount\"=0,\"Allocated PB Size\"=135266304,\"In use system memory\"=925630464}
      \"model\" = \"Apple M5 Pro\"
      \"gpu-core-count\" = 16
    }";

    /// 多行 ioreg 输出（可读格式），同样包含 "(driver)" 干扰字段
    const REAL_IOREG_MULTILINE: &str = "\
+-o AGXAcceleratorG17X  <class AGXAcceleratorG17X>
    {
      \"PerformanceStatistics\" = {
        \"In use system memory (driver)\" = 0
        \"Device Utilization %\" = 42
        \"In use system memory\" = 1048576
      }
      \"model\" = \"Apple M5 Pro\"
    }";

    // ==================== parse_ioreg_performance_stats ====================

    #[test]
    fn test_parse_stats_oneline_real_output() {
        let (percent, mem) = ResourceMonitor::parse_ioreg_performance_stats(REAL_IOREG_ONELINE);
        // Device Utilization % = 0
        assert_eq!(percent, Some(0.0));
        // In use system memory = 925630464 bytes = 882.75 MB (925630464 / 1048576 = 882.75)
        assert!((mem.unwrap() - 882.75).abs() < 0.001);
    }

    #[test]
    fn test_parse_stats_multiline_format() {
        let (percent, mem) = ResourceMonitor::parse_ioreg_performance_stats(REAL_IOREG_MULTILINE);
        assert_eq!(percent, Some(42.0));
        // 1048576 bytes = 1.0 MB
        assert!((mem.unwrap() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_stats_driver_memory_not_mistaken_for_real() {
        // 关键边界："(driver)" 变体的值是 0，真实字段值是 925630464
        // 如果误匹配 "(driver)"，会得到 mem=0.0 而非 882.75
        let (percent, mem) = ResourceMonitor::parse_ioreg_performance_stats(REAL_IOREG_ONELINE);
        assert_ne!(mem, Some(0.0), "不应误匹配 'In use system memory (driver)'");
        assert_eq!(percent, Some(0.0));
    }

    #[test]
    fn test_parse_stats_no_performance_statistics() {
        let text = "some random ioreg output without the stats block";
        let (percent, mem) = ResourceMonitor::parse_ioreg_performance_stats(text);
        assert_eq!(percent, None);
        assert_eq!(mem, None);
    }

    #[test]
    fn test_parse_stats_missing_keys() {
        // PerformanceStatistics 块存在但缺少目标字段
        let text = "\"PerformanceStatistics\" = {\"recoveryCount\"=5,\"Allocated PB Size\"=1024}";
        let (percent, mem) = ResourceMonitor::parse_ioreg_performance_stats(text);
        assert_eq!(percent, None);
        assert_eq!(mem, None);
    }

    #[test]
    fn test_parse_stats_utilization_with_decimal() {
        let text = "\"PerformanceStatistics\" = {\"Device Utilization %\"=37.5,\"In use system memory\"=2097152}";
        let (percent, mem) = ResourceMonitor::parse_ioreg_performance_stats(text);
        assert_eq!(percent, Some(37.5));
        assert!((mem.unwrap() - 2.0).abs() < 0.001); // 2097152 bytes = 2 MB
    }

    // ==================== extract_ioreg_value ====================

    #[test]
    fn test_extract_value_precise_key_matching() {
        // 关键测试：精确匹配防止前缀误匹配
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
        // 值后紧跟逗号和下一个 key
        let stats = "\"Device Utilization %\"=99,\"Next Key\"=1";
        let val = ResourceMonitor::extract_ioreg_value(stats, "Device Utilization %");
        assert_eq!(val, Some("99"));
    }

    #[test]
    fn test_extract_value_no_mismatch_on_suffix_key() {
        // 确保查找 "Utilization %" 不会误匹配 "Tiler Utilization %" 或 "Renderer Utilization %"
        let stats = "\"Tiler Utilization %\"=10,\"Renderer Utilization %\"=20,\"Device Utilization %\"=30";
        let val = ResourceMonitor::extract_ioreg_value(stats, "Device Utilization %");
        assert_eq!(val, Some("30"));
    }

    // ==================== extract_ioreg_model ====================

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
        let text = "some ioreg output without model field";
        let val = ResourceMonitor::extract_ioreg_model(text);
        assert_eq!(val, None);
    }

    #[test]
    fn test_extract_model_empty_value() {
        let text = "\"model\" = \"\"";
        let val = ResourceMonitor::extract_ioreg_model(text);
        assert_eq!(val, None);
    }

    // ==================== 回归：数据结构契约 ====================

    #[test]
    fn test_system_resource_has_all_gpu_fields() {
        // 验证 SystemResource 结构包含所有 GPU 字段（编译期保证）
        let _resource = SystemResource {
            cpu_percent: 0.0,
            memory_used_gb: 0.0,
            memory_total_gb: 0.0,
            memory_percent: 0.0,
            gpu_name: None,
            gpu_percent: None,
            gpu_memory_used_mb: None,
            gpu_memory_total_mb: None,
        };
        // 如果结构体字段不匹配，编译会失败
    }

    #[test]
    fn test_get_gpu_info_return_arity() {
        // get_gpu_info 返回 4-tuple：(name, percent, mem_used, mem_total)
        // 通过类型系统间接验证：get_system_resources 正确解构该 tuple
        // 这里只验证解析函数的返回类型契约
        let (percent, mem): (Option<f32>, Option<f64>) =
            ResourceMonitor::parse_ioreg_performance_stats(REAL_IOREG_ONELINE);
        assert!(percent.is_some());
        assert!(mem.is_some());
    }
}
