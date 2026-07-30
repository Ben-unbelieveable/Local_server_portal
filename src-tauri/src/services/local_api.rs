//! 本机 HTTP 开发桥：仅绑定 `127.0.0.1`，让浏览器 Vite 预览与 Tauri 主窗/托盘共用同一 `AppState`。
//!
//! 社区常见做法是桌面壳内嵌 localhost API（axum / tiny_http）；浏览器无法走 `invoke`，
//! 通过同源 CORS + 固定端口拉取真实服务列表与资源数据。
//!
//! 风险：端口占用时桥无法启动；仅限本机回环，勿改绑 `0.0.0.0`。

use crate::models::{
    BatchResult, LogEntry, ResourceHistoryPoint, ServiceConfig, ServiceResource, ServiceRuntime,
    SystemResource,
};
use crate::services::config_manager;
use crate::services::resource_monitor::ResourceMonitor;
use crate::services::service_manager::ServiceManager;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderValue, Method, Request, StatusCode};
use axum::middleware::{from_fn, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 本机桥监听地址（固定端口，便于前端硬编码 / 环境变量覆盖）
pub const LOCAL_API_ADDR: &str = "127.0.0.1:17830";

type SharedManager = Arc<Mutex<ServiceManager>>;

/// 将错误字符串映射为 HTTP 文本响应
fn err_response(status: StatusCode, msg: String) -> Response {
    (status, msg).into_response()
}

/// CORS 中间件：允许 Vite 等本机源跨域访问（仅开发桥）
async fn cors_layer(req: Request<Body>, next: Next) -> Response {
    if req.method() == Method::OPTIONS {
        let mut res = Response::new(Body::empty());
        *res.status_mut() = StatusCode::NO_CONTENT;
        attach_cors(res.headers_mut());
        return res;
    }
    let mut res = next.run(req).await;
    attach_cors(res.headers_mut());
    res
}

fn attach_cors(headers: &mut axum::http::HeaderMap) {
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Content-Type"),
    );
}

/// 健康检查：确认桌面应用已启动且桥可用
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "ok": true, "service": "local-server-portal" }))
}

async fn get_services(State(mgr): State<SharedManager>) -> Result<Json<Vec<ServiceRuntime>>, Response> {
    let manager = mgr.lock().await;
    Ok(Json(manager.get_services()))
}

async fn add_service(
    State(mgr): State<SharedManager>,
    Json(config): Json<ServiceConfig>,
) -> Result<Json<ServiceRuntime>, Response> {
    let mut manager = mgr.lock().await;
    manager
        .add_service(config)
        .map(Json)
        .map_err(|e| err_response(StatusCode::BAD_REQUEST, e))
}

async fn update_service(
    State(mgr): State<SharedManager>,
    Path(id): Path<String>,
    Json(config): Json<ServiceConfig>,
) -> Result<Json<ServiceRuntime>, Response> {
    let mut manager = mgr.lock().await;
    manager
        .update_service(&id, config)
        .map(Json)
        .map_err(|e| err_response(StatusCode::BAD_REQUEST, e))
}

#[derive(Deserialize)]
struct RemoveQuery {
    stop_first: Option<bool>,
}

async fn remove_service(
    State(mgr): State<SharedManager>,
    Path(id): Path<String>,
    Query(q): Query<RemoveQuery>,
) -> Result<StatusCode, Response> {
    let mut manager = mgr.lock().await;
    manager
        .remove_service(&id, q.stop_first.unwrap_or(false))
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| err_response(StatusCode::BAD_REQUEST, e))
}

async fn start_service(
    State(mgr): State<SharedManager>,
    Path(id): Path<String>,
) -> Result<StatusCode, Response> {
    let mut manager = mgr.lock().await;
    manager
        .start_service(&id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| err_response(StatusCode::BAD_REQUEST, e))
}

async fn stop_service(
    State(mgr): State<SharedManager>,
    Path(id): Path<String>,
) -> Result<StatusCode, Response> {
    let mut manager = mgr.lock().await;
    manager
        .stop_service(&id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| err_response(StatusCode::BAD_REQUEST, e))
}

async fn restart_service(
    State(mgr): State<SharedManager>,
    Path(id): Path<String>,
) -> Result<StatusCode, Response> {
    let mut manager = mgr.lock().await;
    manager
        .restart_service(&id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| err_response(StatusCode::BAD_REQUEST, e))
}

#[derive(Deserialize)]
struct IdsBody {
    ids: Vec<String>,
}

async fn batch_start(
    State(mgr): State<SharedManager>,
    Json(body): Json<IdsBody>,
) -> Json<Vec<BatchResult>> {
    let mut manager = mgr.lock().await;
    Json(manager.batch_start(&body.ids).await)
}

async fn batch_stop(
    State(mgr): State<SharedManager>,
    Json(body): Json<IdsBody>,
) -> Json<Vec<BatchResult>> {
    let mut manager = mgr.lock().await;
    Json(manager.batch_stop(&body.ids).await)
}

async fn get_system_resources(
    State(mgr): State<SharedManager>,
) -> Json<SystemResource> {
    let manager = mgr.lock().await;
    Json(match manager.get_system_resource() {
        Some(res) => res,
        None => ResourceMonitor::get_system_resources(),
    })
}

async fn get_service_resources(
    State(mgr): State<SharedManager>,
) -> Json<Vec<ServiceResource>> {
    let manager = mgr.lock().await;
    let services = manager.get_services();
    let mut resources = Vec::new();
    for service in services {
        if let Some(pid) = service.pid {
            resources.push(ResourceMonitor::get_process_resource(
                &service.config.id,
                pid,
                service.uptime_secs,
            ));
        }
    }
    Json(resources)
}

async fn get_resource_history(
    State(mgr): State<SharedManager>,
) -> Json<Vec<ResourceHistoryPoint>> {
    let manager = mgr.lock().await;
    Json(manager.get_resource_history())
}

#[derive(Deserialize)]
struct LogsQuery {
    count: Option<u32>,
}

async fn get_recent_logs(
    State(mgr): State<SharedManager>,
    Path(service_id): Path<String>,
    Query(q): Query<LogsQuery>,
) -> Json<Vec<LogEntry>> {
    let manager = mgr.lock().await;
    let count = q.count.unwrap_or(100) as usize;
    Json(manager.log_manager().get_recent(&service_id, count))
}

#[derive(Deserialize)]
struct SearchQuery {
    keyword: String,
}

async fn search_logs(
    State(mgr): State<SharedManager>,
    Path(service_id): Path<String>,
    Query(q): Query<SearchQuery>,
) -> Json<Vec<LogEntry>> {
    let manager = mgr.lock().await;
    Json(manager.log_manager().search(&service_id, &q.keyword))
}

#[derive(Deserialize)]
struct HistoryQuery {
    date: String,
}

async fn get_history_logs(
    State(mgr): State<SharedManager>,
    Path(service_id): Path<String>,
    Query(q): Query<HistoryQuery>,
) -> Json<Vec<LogEntry>> {
    let manager = mgr.lock().await;
    Json(manager.log_manager().get_history(&service_id, &q.date))
}

async fn get_config_raw() -> Result<String, Response> {
    let path = config_manager::config_path();
    if path.exists() {
        std::fs::read_to_string(&path).map_err(|e| {
            err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("读取配置失败: {}", e))
        })
    } else {
        Ok(String::new())
    }
}

#[derive(Deserialize)]
struct ContentBody {
    content: String,
}

async fn save_config_raw(Json(body): Json<ContentBody>) -> Result<StatusCode, Response> {
    config_manager::validate_yaml(&body.content)
        .map_err(|e| err_response(StatusCode::BAD_REQUEST, e))?;
    std::fs::write(config_manager::config_path(), &body.content).map_err(|e| {
        err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("保存配置失败: {}", e),
        )
    })?;
    Ok(StatusCode::NO_CONTENT)
}

async fn validate_config(Json(body): Json<ContentBody>) -> Result<Json<String>, Response> {
    config_manager::validate_yaml(&body.content)
        .map(|_| Json("ok".to_string()))
        .map_err(|e| err_response(StatusCode::BAD_REQUEST, e))
}

async fn shutdown_all(State(mgr): State<SharedManager>) -> Json<Vec<BatchResult>> {
    let mut manager = mgr.lock().await;
    Json(manager.shutdown_all().await)
}

fn build_router(manager: SharedManager) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/services", get(get_services).post(add_service))
        .route(
            "/api/services/{id}",
            put(update_service).delete(remove_service),
        )
        .route("/api/services/{id}/start", post(start_service))
        .route("/api/services/{id}/stop", post(stop_service))
        .route("/api/services/{id}/restart", post(restart_service))
        .route("/api/services/batch-start", post(batch_start))
        .route("/api/services/batch-stop", post(batch_stop))
        .route("/api/system-resources", get(get_system_resources))
        .route("/api/service-resources", get(get_service_resources))
        .route("/api/resource-history", get(get_resource_history))
        .route("/api/logs/{service_id}/recent", get(get_recent_logs))
        .route("/api/logs/{service_id}/search", get(search_logs))
        .route("/api/logs/{service_id}/history", get(get_history_logs))
        .route("/api/config", get(get_config_raw).put(save_config_raw))
        .route("/api/config/validate", post(validate_config))
        .route("/api/shutdown-all", post(shutdown_all))
        .layer(from_fn(cors_layer))
        .with_state(manager)
}

/// 在后台启动本机 HTTP 桥；失败只打日志，不阻断桌面应用。
///
/// 输入：`ServiceManager` 的共享句柄（与 Tauri commands 同一份）。
pub fn spawn(manager: SharedManager) {
    tauri::async_runtime::spawn(async move {
        let addr: SocketAddr = match LOCAL_API_ADDR.parse() {
            Ok(a) => a,
            Err(e) => {
                tracing::error!("本地 API 地址解析失败: {}", e);
                return;
            }
        };
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                tracing::error!(
                    "本地 HTTP API 绑定 {} 失败（浏览器预览将无法同步数据）: {}",
                    LOCAL_API_ADDR,
                    e
                );
                return;
            }
        };
        tracing::info!("本地 HTTP API 已启动: http://{}", LOCAL_API_ADDR);
        let app = build_router(manager);
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("本地 HTTP API 异常退出: {}", e);
        }
    });
}
