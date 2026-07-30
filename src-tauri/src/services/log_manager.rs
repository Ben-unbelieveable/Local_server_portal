use crate::models::LogEntry;
use chrono::Local;
use std::collections::VecDeque;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::{info, warn};

const MAX_BUFFER_LINES: usize = 10_000;
const MAX_LOG_FILE_MB: u64 = 50;
const LOG_RETENTION_DAYS: i64 = 7;

pub struct LogManager {
    /// 内存缓冲区，key = service_id
    buffers: Mutex<std::collections::HashMap<String, VecDeque<LogEntry>>>,
    /// 基础日志目录
    base_dir: PathBuf,
}

impl LogManager {
    pub fn new() -> Self {
        let base_dir = Self::default_log_dir();
        fs::create_dir_all(&base_dir).ok();

        // 启动时清理过期日志
        let dir = base_dir.clone();
        std::thread::spawn(move || {
            Self::prune_old_logs(&dir, LOG_RETENTION_DAYS);
        });

        LogManager {
            buffers: Mutex::new(std::collections::HashMap::new()),
            base_dir,
        }
    }

    fn default_log_dir() -> PathBuf {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(|home| PathBuf::from(home).join(".local-service-manager").join("logs"))
            .unwrap_or_else(|_| PathBuf::from("./logs"))
    }

    /// 追加一条日志
    pub fn append(&self, service_id: &str, stream: &str, line: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        let entry = LogEntry {
            service_id: service_id.to_string(),
            timestamp: timestamp.clone(),
            stream: stream.to_string(),
            line: line.to_string(),
        };

        // 写入内存缓冲区
        {
            let mut buffers = self.buffers.lock().unwrap();
            let buffer = buffers
                .entry(service_id.to_string())
                .or_insert_with(|| VecDeque::with_capacity(MAX_BUFFER_LINES));
            if buffer.len() >= MAX_BUFFER_LINES {
                buffer.pop_front();
            }
            buffer.push_back(entry.clone());
        }

        // 写入磁盘
        self.write_to_disk(service_id, &timestamp, stream, line);
    }

    fn write_to_disk(&self, service_id: &str, timestamp: &str, stream: &str, line: &str) {
        let today = &timestamp[..10]; // YYYY-MM-DD
        let dir = self.base_dir.join(service_id);
        let file_path = dir.join(format!("{}.log", today));

        fs::create_dir_all(&dir).ok();

        // 检查文件大小
        if file_path.exists() {
            if let Ok(meta) = fs::metadata(&file_path) {
                if meta.len() > MAX_LOG_FILE_MB * 1024 * 1024 {
                    // 截断文件
                    let _ = fs::write(&file_path, "");
                }
            }
        }

        let log_line = format!("[{}] [{}] {}\n", timestamp, stream, line);

        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
        {
            let _ = file.write_all(log_line.as_bytes());
        } else {
            warn!("无法写入日志文件: {:?}", file_path);
        }
    }

    /// 获取最近 N 条日志
    pub fn get_recent(&self, service_id: &str, count: usize) -> Vec<LogEntry> {
        let buffers = self.buffers.lock().unwrap();
        if let Some(buffer) = buffers.get(service_id) {
            let skip = if buffer.len() > count {
                buffer.len() - count
            } else {
                0
            };
            buffer.iter().skip(skip).cloned().collect()
        } else {
            vec![]
        }
    }

    /// 搜索日志
    pub fn search(&self, service_id: &str, keyword: &str) -> Vec<LogEntry> {
        let buffers = self.buffers.lock().unwrap();
        if let Some(buffer) = buffers.get(service_id) {
            buffer
                .iter()
                .filter(|e| e.line.contains(keyword))
                .cloned()
                .collect()
        } else {
            vec![]
        }
    }

    /// 获取历史日志（从磁盘文件读取）
    pub fn get_history(&self, service_id: &str, date: &str) -> Vec<LogEntry> {
        let file_path = self.base_dir.join(service_id).join(format!("{}.log", date));
        if !file_path.exists() {
            return vec![];
        }

        match fs::read_to_string(&file_path) {
            Ok(content) => {
                let mut entries = Vec::new();
                for line in content.lines() {
                    // 解析格式: [timestamp] [stream] message
                    if let Some(entry) = Self::parse_log_line(service_id, line) {
                        entries.push(entry);
                    }
                }
                entries
            }
            Err(_) => vec![],
        }
    }

    fn parse_log_line(service_id: &str, line: &str) -> Option<LogEntry> {
        // [2025-07-23 12:00:00.000] [stdout] message
        if !line.starts_with('[') {
            return None;
        }
        let rest = &line[1..]; // 跳过第一个 [
        let close_bracket = rest.find(']')?;
        let timestamp = rest[..close_bracket].to_string();

        let rest = &rest[close_bracket + 1..];
        if !rest.starts_with(" [") {
            return None;
        }
        let rest = &rest[2..];
        let close_bracket2 = rest.find(']')?;
        let stream = rest[..close_bracket2].to_string();

        let message = rest[close_bracket2 + 2..].to_string();

        Some(LogEntry {
            service_id: service_id.to_string(),
            timestamp,
            stream,
            line: message,
        })
    }

    /// 清理过期日志
    fn prune_old_logs(base_dir: &PathBuf, retention_days: i64) {
        let cutoff = Local::now()
            .date_naive()
            .checked_sub_days(chrono::Days::new(retention_days as u64));

        if cutoff.is_none() {
            return;
        }
        let cutoff = cutoff.unwrap();

        if let Ok(entries) = fs::read_dir(base_dir) {
            for service_entry in entries.flatten() {
                if let Ok(log_entries) = fs::read_dir(service_entry.path()) {
                    for log_entry in log_entries.flatten() {
                        let path = log_entry.path();
                        if path.extension().map_or(false, |e| e == "log") {
                            let file_stem = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("");
                            if let Ok(date) =
                                chrono::NaiveDate::parse_from_str(file_stem, "%Y-%m-%d")
                            {
                                if date < cutoff {
                                    let _ = fs::remove_file(&path);
                                    info!("清理过期日志: {:?}", path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
