use crate::models::{AppConfig, ServiceConfig};
use std::fs;
use std::path::PathBuf;

/// 获取配置目录路径
pub fn config_dir() -> PathBuf {
    dirs_next().unwrap_or_else(|| PathBuf::from("."))
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(|home| PathBuf::from(home).join(".local-service-manager"))
}

/// 获取 config.yaml 文件路径
pub fn config_path() -> PathBuf {
    config_dir().join("config.yaml")
}

/// 获取日志目录路径
pub fn logs_dir() -> PathBuf {
    config_dir().join("logs")
}

/// 获取数据目录路径
pub fn data_dir() -> PathBuf {
    config_dir().join("data")
}

/// 确保所有必要目录存在
pub fn ensure_dirs() -> std::io::Result<()> {
    let dirs = vec![config_dir(), logs_dir(), data_dir()];
    for dir in dirs {
        fs::create_dir_all(&dir)?;
    }
    Ok(())
}

/// 加载配置
pub fn load_config() -> Result<AppConfig, String> {
    ensure_dirs().map_err(|e| format!("无法创建配置目录: {}", e))?;

    let path = config_path();
    if !path.exists() {
        // 首次启动，创建默认空配置
        let default_config = AppConfig {
            services: vec![
                ServiceConfig {
                    id: "example".to_string(),
                    name: "示例服务".to_string(),
                    command: "echo 'Hello from LocalServiceManager'".to_string(),
                    url: None,
                    port: None,
                    work_dir: None,
                    env: Default::default(),
                    group: Some("示例".to_string()),
                    description: Some("这是一个示例服务配置，你可以删除它并添加自己的服务".to_string()),
                    stop_timeout: 10,
                }
            ],
        };
        save_config(&default_config)?;
        return Ok(default_config);
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("读取配置文件失败: {}", e))?;

    if content.trim().is_empty() {
        return Ok(AppConfig { services: vec![] });
    }

    serde_yaml::from_str(&content)
        .map_err(|e| format!("配置文件格式错误: {}", e))
}

/// 保存配置
pub fn save_config(config: &AppConfig) -> Result<(), String> {
    ensure_dirs().map_err(|e| format!("无法创建配置目录: {}", e))?;

    let content = serde_yaml::to_string(config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;

    fs::write(config_path(), content)
        .map_err(|e| format!("写入配置文件失败: {}", e))
}

/// 校验 YAML 内容格式
pub fn validate_yaml(content: &str) -> Result<AppConfig, String> {
    serde_yaml::from_str::<AppConfig>(content)
        .map_err(|e| format!("YAML 格式错误: {}", e))
}
