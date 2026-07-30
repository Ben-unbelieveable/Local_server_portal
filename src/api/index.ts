import type {
  ServiceRuntime,
  ServiceConfig,
  SystemResource,
  ServiceResource,
  ResourceHistoryPoint,
  LogEntry,
  BatchResult,
  AppPreferences,
} from "../types";

// ==================== 环境检测 ====================
function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI__" in window;
}

/** 本机桥默认地址（与 Rust `local_api::LOCAL_API_ADDR` 一致）；可用 VITE_LOCAL_API_URL 覆盖 */
const LOCAL_API_BASE =
  (import.meta as ImportMeta & { env?: { VITE_LOCAL_API_URL?: string } }).env
    ?.VITE_LOCAL_API_URL ?? "http://127.0.0.1:17830";

/**
 * 浏览器环境：通过本机 HTTP 桥调用与桌面壳相同的后端。
 * 输入：path / fetch init；输出：解析后的 JSON 或 void；失败抛 Error（含服务端文本）。
 */
async function bridgeFetch<T>(path: string, init?: RequestInit): Promise<T> {
  let res: Response;
  try {
    res = await fetch(`${LOCAL_API_BASE}${path}`, {
      ...init,
      headers: {
        ...(init?.body ? { "Content-Type": "application/json" } : {}),
        ...init?.headers,
      },
    });
  } catch {
    throw new Error(
      `无法连接本机 API（${LOCAL_API_BASE}）。请先启动桌面应用（pnpm tauri dev），浏览器才能与主窗/托盘同步数据。`
    );
  }
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || `HTTP ${res.status}`);
  }
  if (res.status === 204) {
    return undefined as T;
  }
  const ct = res.headers.get("content-type") || "";
  if (ct.includes("application/json")) {
    return (await res.json()) as T;
  }
  return (await res.text()) as T;
}

/** API 方法集合形状（Tauri invoke / HTTP 桥共用） */
type AppApi = {
  getServices: () => Promise<ServiceRuntime[]>;
  addService: (config: ServiceConfig) => Promise<ServiceRuntime>;
  updateService: (id: string, config: ServiceConfig) => Promise<ServiceRuntime>;
  removeService: (id: string, stopFirst: boolean) => Promise<void>;
  startService: (id: string) => Promise<void>;
  stopService: (id: string) => Promise<void>;
  restartService: (id: string) => Promise<void>;
  batchStart: (ids: string[]) => Promise<BatchResult[]>;
  batchStop: (ids: string[]) => Promise<BatchResult[]>;
  getSystemResources: () => Promise<SystemResource>;
  getResourceHistory: () => Promise<ResourceHistoryPoint[]>;
  getServiceResources: () => Promise<ServiceResource[]>;
  getRecentLogs: (serviceId: string, count: number) => Promise<LogEntry[]>;
  searchLogs: (serviceId: string, keyword: string) => Promise<LogEntry[]>;
  getHistoryLogs: (serviceId: string, date: string) => Promise<LogEntry[]>;
  getConfigRaw: () => Promise<string>;
  saveConfigRaw: (content: string) => Promise<void>;
  validateConfig: (content: string) => Promise<string>;
  shutdownAll: () => Promise<BatchResult[]>;
  getAppPreferences: () => Promise<AppPreferences>;
  setQuitWhenCloseMain: (enabled: boolean) => Promise<AppPreferences>;
  setLaunchAtLoginPref: (enabled: boolean) => Promise<AppPreferences>;
  quitApp: () => Promise<void>;
};

/**
 * 浏览器桥 API：走 127.0.0.1 HTTP，与主窗/托盘共享 Rust AppState。
 * 不再使用硬编码假服务列表。
 */
const bridgeApi: AppApi = {
  getServices: () => bridgeFetch<ServiceRuntime[]>("/api/services"),
  addService: (config) =>
    bridgeFetch<ServiceRuntime>("/api/services", {
      method: "POST",
      body: JSON.stringify(config),
    }),
  updateService: (id, config) =>
    bridgeFetch<ServiceRuntime>(`/api/services/${encodeURIComponent(id)}`, {
      method: "PUT",
      body: JSON.stringify(config),
    }),
  removeService: (id, stopFirst) =>
    bridgeFetch<void>(
      `/api/services/${encodeURIComponent(id)}?stop_first=${stopFirst}`,
      { method: "DELETE" }
    ),
  startService: (id) =>
    bridgeFetch<void>(`/api/services/${encodeURIComponent(id)}/start`, {
      method: "POST",
    }),
  stopService: (id) =>
    bridgeFetch<void>(`/api/services/${encodeURIComponent(id)}/stop`, {
      method: "POST",
    }),
  restartService: (id) =>
    bridgeFetch<void>(`/api/services/${encodeURIComponent(id)}/restart`, {
      method: "POST",
    }),
  batchStart: (ids) =>
    bridgeFetch<BatchResult[]>("/api/services/batch-start", {
      method: "POST",
      body: JSON.stringify({ ids }),
    }),
  batchStop: (ids) =>
    bridgeFetch<BatchResult[]>("/api/services/batch-stop", {
      method: "POST",
      body: JSON.stringify({ ids }),
    }),
  getSystemResources: () =>
    bridgeFetch<SystemResource>("/api/system-resources"),
  getResourceHistory: () =>
    bridgeFetch<ResourceHistoryPoint[]>("/api/resource-history"),
  getServiceResources: () =>
    bridgeFetch<ServiceResource[]>("/api/service-resources"),
  getRecentLogs: (serviceId, count) =>
    bridgeFetch<LogEntry[]>(
      `/api/logs/${encodeURIComponent(serviceId)}/recent?count=${count}`
    ),
  searchLogs: (serviceId, keyword) =>
    bridgeFetch<LogEntry[]>(
      `/api/logs/${encodeURIComponent(serviceId)}/search?keyword=${encodeURIComponent(keyword)}`
    ),
  getHistoryLogs: (serviceId, date) =>
    bridgeFetch<LogEntry[]>(
      `/api/logs/${encodeURIComponent(serviceId)}/history?date=${encodeURIComponent(date)}`
    ),
  getConfigRaw: () => bridgeFetch<string>("/api/config"),
  saveConfigRaw: (content) =>
    bridgeFetch<void>("/api/config", {
      method: "PUT",
      body: JSON.stringify({ content }),
    }),
  validateConfig: (content) =>
    bridgeFetch<string>("/api/config/validate", {
      method: "POST",
      body: JSON.stringify({ content }),
    }),
  shutdownAll: () =>
    bridgeFetch<BatchResult[]>("/api/shutdown-all", { method: "POST" }),
  getAppPreferences: async () => ({
    quit_when_close_main: false,
    launch_at_login: false,
  }),
  setQuitWhenCloseMain: async () => ({
    quit_when_close_main: false,
    launch_at_login: false,
  }),
  setLaunchAtLoginPref: async () => ({
    quit_when_close_main: false,
    launch_at_login: false,
  }),
  quitApp: async () => {
    throw new Error("浏览器预览无法退出桌面应用，请在托盘内操作");
  },
};

const isTauriEnv = isTauri();

let _realApiPromise: Promise<AppApi> | null = null;

async function loadRealApi(): Promise<AppApi> {
  const { invoke } = await import("@tauri-apps/api/core");
  return {
    getServices: () => invoke<ServiceRuntime[]>("get_services"),
    addService: (config: ServiceConfig) =>
      invoke<ServiceRuntime>("add_service", { config }),
    updateService: (id: string, config: ServiceConfig) =>
      invoke<ServiceRuntime>("update_service", { id, config }),
    removeService: (id: string, stopFirst: boolean) =>
      invoke<void>("remove_service", { id, stopFirst }),
    startService: (id: string) => invoke<void>("start_service", { id }),
    stopService: (id: string) => invoke<void>("stop_service", { id }),
    restartService: (id: string) => invoke<void>("restart_service", { id }),
    batchStart: (ids: string[]) =>
      invoke<BatchResult[]>("batch_start", { ids }),
    batchStop: (ids: string[]) => invoke<BatchResult[]>("batch_stop", { ids }),
    getSystemResources: () => invoke<SystemResource>("get_system_resources"),
    getResourceHistory: () =>
      invoke<ResourceHistoryPoint[]>("get_resource_history"),
    getServiceResources: () =>
      invoke<ServiceResource[]>("get_service_resources"),
    getRecentLogs: (serviceId: string, count: number) =>
      invoke<LogEntry[]>("get_recent_logs", { serviceId, count }),
    searchLogs: (serviceId: string, keyword: string) =>
      invoke<LogEntry[]>("search_logs", { serviceId, keyword }),
    getHistoryLogs: (serviceId: string, date: string) =>
      invoke<LogEntry[]>("get_history_logs", { serviceId, date }),
    getConfigRaw: () => invoke<string>("get_config_raw"),
    saveConfigRaw: (content: string) =>
      invoke<void>("save_config_raw", { content }),
    validateConfig: (content: string) =>
      invoke<string>("validate_config", { content }),
    shutdownAll: () => invoke<BatchResult[]>("shutdown_all_services"),
    getAppPreferences: () => invoke<AppPreferences>("get_app_preferences"),
    setQuitWhenCloseMain: (enabled: boolean) =>
      invoke<AppPreferences>("set_quit_when_close_main", { enabled }),
    setLaunchAtLoginPref: (enabled: boolean) =>
      invoke<AppPreferences>("set_launch_at_login_pref", { enabled }),
    quitApp: () => invoke<void>("quit_app"),
  };
}

/**
 * 获取当前 API：
 * - Tauri：invoke 真实后端
 * - 浏览器：本机 HTTP 桥（需桌面应用运行）
 */
function getApi(): Promise<AppApi> {
  if (!isTauriEnv) return Promise.resolve(bridgeApi);
  if (!_realApiPromise) _realApiPromise = loadRealApi();
  return _realApiPromise;
}

export const api = new Proxy({} as AppApi, {
  get(_target, prop: string) {
    return (...args: unknown[]) => {
      return getApi().then((target) => {
        const fn = (target as Record<string, Function>)[prop];
        if (!fn) throw new Error(`Unknown API method: ${prop}`);
        return fn(...args);
      });
    };
  },
});

export { isTauriEnv, LOCAL_API_BASE };
