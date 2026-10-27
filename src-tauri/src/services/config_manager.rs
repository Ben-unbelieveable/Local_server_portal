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
            shortcuts: vec![],
        };
        save_config(&default_config)?;
        return Ok(default_config);
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("读取配置文件失败: {}", e))?;

    if content.trim().is_empty() {
        return Ok(AppConfig {
            services: vec![],
            shortcuts: vec![],
        });
    }

    validate_yaml(&content)
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
    let config =
        serde_yaml::from_str::<AppConfig>(content).map_err(|e| format!("YAML 格式错误: {}", e))?;
    let mut ids = std::collections::HashSet::new();
    for shortcut in &config.shortcuts {
        shortcut.validate()?;
        if !ids.insert(&shortcut.id) {
            return Err(format!("入口 ID '{}' 已存在，请更换", shortcut.id));
        }
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::validate_yaml;

    /// 输入旧版 YAML；断言无入口字段时仍保留已有服务并默认空入口。
    #[test]
    fn legacy_config_keeps_services() {
        let config =
            validate_yaml("services:\n  - id: api\n    name: API\n    command: echo hello\n")
                .unwrap();
        assert_eq!(config.services.len(), 1);
        assert!(config.shortcuts.is_empty());
    }

    /// 输入仅含页面入口的配置；往返序列化后验证入口和 URL 不丢失。
    #[test]
    fn shortcuts_round_trip_without_commands() {
        let yaml = "services: []\nshortcuts:\n  - id: docs\n    name: 文档\n    url: http://localhost:3000/docs?q=1#top\n    group: 开发\n  - id: page\n    name: 页面\n    url: file:///tmp/index.html\n";
        let config = validate_yaml(yaml).unwrap();
        let saved = serde_yaml::to_string(&config).unwrap();
        let reloaded = validate_yaml(&saved).unwrap();
        assert!(reloaded.services.is_empty());
        assert_eq!(reloaded.shortcuts.len(), 2);
        assert_eq!(
            reloaded.shortcuts[0].url,
            "http://localhost:3000/docs?q=1#top"
        );
        assert_eq!(reloaded.shortcuts[1].url, "file:///tmp/index.html");
    }

    /// 输入恶意/损坏页面地址；验证 YAML 编辑不能绕过页面协议和文件类型约束。
    #[test]
    fn invalid_shortcuts_are_rejected() {
        for url in [
            "javascript:alert(1)",
            "data:text/html,test",
            "ftp://example.com",
            "https://",
            "https:example.com",
            "file:///tmp/run.sh",
            "file:///tmp/index.html.exe",
            "file:///tmp/index%2ehtml",
            "file://server/share/index.html",
        ] {
            let yaml = format!(
                "services: []\nshortcuts:\n  - id: test\n    name: Test\n    url: '{}'\n",
                url
            );
            assert!(validate_yaml(&yaml).is_err(), "unexpected URL: {}", url);
        }
        assert!(validate_yaml("services: []\nshortcuts:\n  - id: test\n    name: '  '\n    url: https://example.com\n").is_err());
    }

    /// 输入重复 ID；验证配置编辑不会形成无法唯一更新和删除的入口。
    #[test]
    fn duplicate_shortcut_ids_are_rejected() {
        assert!(validate_yaml("services: []\nshortcuts:\n  - id: docs\n    name: One\n    url: https://example.com\n  - id: docs\n    name: Two\n    url: http://localhost:3000\n").is_err());
    }
}
