//! 应用行为偏好：与服务列表 YAML 分离，避免配置编辑器覆盖托盘/启动相关开关。
//!
//! 存储路径：`~/.local-service-manager/preferences.yaml`

use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};

use super::config_manager::{config_dir, ensure_dirs};

/// 关闭主窗口时是否同步退出整个应用（含托盘）。默认 false = 仅隐藏主窗，托盘继续服务。
pub static QUIT_WHEN_CLOSE_MAIN: AtomicBool = AtomicBool::new(false);

/// 应用级偏好（托盘为主的产品行为）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppPreferences {
    /// 关闭 Dock 中主窗口时，是否同步退出托盘/进程
    #[serde(default)]
    pub quit_when_close_main: bool,
    /// 开机启动偏好镜像（界面展示可与 OS 登录项对齐；真源仍以 autostart 插件为准）
    #[serde(default)]
    pub launch_at_login: bool,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            quit_when_close_main: false,
            launch_at_login: false,
        }
    }
}

/// 偏好文件路径
pub fn preferences_path() -> std::path::PathBuf {
    config_dir().join("preferences.yaml")
}

/// 从磁盘加载偏好，并同步到运行时原子开关。
///
/// 输出：当前 `AppPreferences`；文件不存在时返回默认值并落盘。
pub fn load_preferences() -> AppPreferences {
    let _ = ensure_dirs();
    let path = preferences_path();
    let prefs = if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|c| serde_yaml::from_str(&c).ok())
            .unwrap_or_default()
    } else {
        let defaults = AppPreferences::default();
        let _ = save_preferences(&defaults);
        defaults
    };
    QUIT_WHEN_CLOSE_MAIN.store(prefs.quit_when_close_main, Ordering::SeqCst);
    prefs
}

/// 持久化偏好并更新运行时 `QUIT_WHEN_CLOSE_MAIN`
pub fn save_preferences(prefs: &AppPreferences) -> Result<(), String> {
    ensure_dirs().map_err(|e| format!("无法创建配置目录: {}", e))?;
    let content =
        serde_yaml::to_string(prefs).map_err(|e| format!("序列化偏好失败: {}", e))?;
    fs::write(preferences_path(), content).map_err(|e| format!("写入偏好失败: {}", e))?;
    QUIT_WHEN_CLOSE_MAIN.store(prefs.quit_when_close_main, Ordering::SeqCst);
    Ok(())
}

/// 更新「关主窗是否退出托盘」并持久化
pub fn set_quit_when_close_main(value: bool) -> Result<AppPreferences, String> {
    let mut prefs = load_preferences();
    prefs.quit_when_close_main = value;
    save_preferences(&prefs)?;
    Ok(prefs)
}

/// 更新开机启动偏好镜像（不直接改 OS；由前端/autostart 插件负责 enable/disable）
pub fn set_launch_at_login_pref(value: bool) -> Result<AppPreferences, String> {
    let mut prefs = load_preferences();
    prefs.launch_at_login = value;
    save_preferences(&prefs)?;
    Ok(prefs)
}
