//! 本机网络信息：局域网 IPv4 与公网 IP。
//!
//! 局域网：社区方案 `if-addrs` 枚举非 loopback 接口。
//! 公网：`curl` 请求 ipify / ifconfig.me（与 resource_monitor 一致走系统命令），结果缓存 5 分钟。
//!
//! 风险：公网查询依赖外网可达；无网或代理异常时 `public_ip` 为 None。

use crate::models::NetworkInfo;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 公网 IP 缓存 TTL
const PUBLIC_IP_TTL: Duration = Duration::from_secs(300);

struct PublicIpCache {
    ip: Option<String>,
    error: Option<String>,
    fetched_at: Option<Instant>,
}

static PUBLIC_IP_CACHE: Mutex<PublicIpCache> = Mutex::new(PublicIpCache {
    ip: None,
    error: None,
    fetched_at: None,
});

/// 枚举本机局域网 IPv4（排除 loopback、link-local、未指定地址）
///
/// 输出：去重排序后的 IPv4 字符串列表
pub fn get_lan_ips() -> Vec<String> {
    let mut ips: Vec<String> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|iface| match iface.addr {
            if_addrs::IfAddr::V4(v4) => {
                let ip = v4.ip;
                if ip.is_loopback() || ip.is_unspecified() || ip.is_link_local() {
                    return None;
                }
                Some(ip.to_string())
            }
            _ => None,
        })
        .collect();
    ips.sort();
    ips.dedup();
    ips
}

/// 从外网 API 拉取公网 IP（无缓存，阻塞 IO 在 spawn_blocking 中执行）
async fn fetch_public_ip_raw() -> (Option<String>, Option<String>) {
    let urls = ["https://api.ipify.org", "https://ifconfig.me/ip"];

    for url in urls {
        let url = url.to_string();
        let url_for_log = url.clone();
        let result = tokio::task::spawn_blocking(move || {
            std::process::Command::new("curl")
                .args(["-sS", "--max-time", "5", &url])
                .output()
        })
        .await;

        match result {
            Ok(Ok(output)) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !text.is_empty() {
                    return (Some(text), None);
                }
            }
            Ok(Ok(output)) => {
                tracing::debug!(
                    "公网 IP 查询 {} 失败: curl exit {:?}",
                    url_for_log,
                    output.status.code()
                );
            }
            Ok(Err(e)) => {
                tracing::debug!("公网 IP 查询 {} 启动 curl 失败: {}", url_for_log, e);
            }
            Err(e) => {
                tracing::debug!(
                    "公网 IP 查询 {} spawn_blocking 失败: {}",
                    url_for_log,
                    e
                );
            }
        }
    }

    (
        None,
        Some("无法获取公网 IP（请检查网络连接或 curl 是否可用）".to_string()),
    )
}

/// 带 TTL 缓存的公网 IP 查询
async fn fetch_public_ip_cached() -> (Option<String>, Option<String>) {
    {
        let cache = PUBLIC_IP_CACHE.lock().unwrap();
        if let Some(at) = cache.fetched_at {
            if at.elapsed() < PUBLIC_IP_TTL {
                return (cache.ip.clone(), cache.error.clone());
            }
        }
    }

    let (ip, error) = fetch_public_ip_raw().await;

    {
        let mut cache = PUBLIC_IP_CACHE.lock().unwrap();
        cache.ip = ip.clone();
        cache.error = error.clone();
        cache.fetched_at = Some(Instant::now());
    }

    (ip, error)
}

/// 聚合局域网与公网 IP 信息
///
/// 输入：无
/// 输出：`NetworkInfo`（公网 IP 异步查询，失败时字段为 None 并附带 error 说明）
pub async fn fetch_network_info() -> NetworkInfo {
    let lan_ips = get_lan_ips();
    let (public_ip, public_ip_error) = fetch_public_ip_cached().await;
    NetworkInfo {
        lan_ips,
        public_ip,
        public_ip_error,
    }
}
