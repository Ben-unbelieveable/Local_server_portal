import type {
  ServiceRuntime,
  ServiceConfig,
  SystemResource,
  ServiceResource,
  ResourceHistoryPoint,
  LogEntry,
  BatchResult,
  ServiceStatus,
} from "../types";

// ==================== 环境检测 ====================
function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI__" in window;
}

// ==================== Mock 数据 ====================

const mockServices: ServiceRuntime[] = [
  {
    config: {
      id: "antibody_annotation",
      name: "Antibody Annotation",
      command: "bash /Users/liubo/github/antibody_annotation/deploy.sh",
      url: "http://localhost:3000",
      work_dir: "/Users/liubo/github/antibody_annotation",
      env: { PORT: "3000", DEBUG: "false" },
      group: "Web服务",
      description: "抗体标注 Web 服务",
      stop_timeout: 10,
    },
    status: "running" as ServiceStatus,
    pid: 48210,
    cpu_percent: 2.3,
    memory_mb: 156.7,
    uptime_secs: 3720,
    restart_count: 0,
    last_error: null,
  },
  {
    config: {
      id: "argo_portal",
      name: "Argo Portal",
      command: "bash /Users/liubo/github/argo_portal/deploy.sh",
      url: "http://localhost:8080",
      work_dir: "/Users/liubo/github/argo_portal",
      group: "Web服务",
      description: "Argo 门户网站",
      stop_timeout: 10,
    },
    status: "running" as ServiceStatus,
    pid: 48245,
    cpu_percent: 0.8,
    memory_mb: 89.2,
    uptime_secs: 5400,
    restart_count: 0,
    last_error: null,
  },
  {
    config: {
      id: "model_graph_web",
      name: "Model Graph Web",
      command: "bash /Users/liubo/github/model_graph_web/deploy.sh",
      url: "http://localhost:5000",
      work_dir: "/Users/liubo/github/model_graph_web",
      group: "Web服务",
      description: "模型图谱可视化",
      stop_timeout: 10,
    },
    status: "stopped" as ServiceStatus,
    pid: null,
    cpu_percent: 0,
    memory_mb: 0,
    uptime_secs: 0,
    restart_count: 0,
    last_error: null,
  },
  {
    config: {
      id: "ollama",
      name: "Ollama AI",
      command: "ollama serve",
      group: "AI模型",
      description: "Ollama 大模型推理服务（高资源消耗）",
      stop_timeout: 15,
    },
    status: "stopped" as ServiceStatus,
    pid: null,
    cpu_percent: 0,
    memory_mb: 0,
    uptime_secs: 0,
    restart_count: 0,
    last_error: null,
  },
  {
    config: {
      id: "start_failed_demo",
      name: "启动失败示例",
      command: "bash /nonexistent/deploy.sh",
      group: "示例",
      description: "演示启动失败的场景",
      stop_timeout: 10,
    },
    status: "failed" as ServiceStatus,
    pid: null,
    cpu_percent: 0,
    memory_mb: 0,
    uptime_secs: 0,
    restart_count: 1,
    last_error: "启动脚本不存在: /nonexistent/deploy.sh",
  },
];

let _mockState = {
  services: [...mockServices],
  nextPid: 50000,
};

function delay<T>(data: T, ms = 150): Promise<T> {
  return new Promise((resolve) => setTimeout(() => resolve(data), ms));
}

const mockApi = {
  getServices: () => delay([..._mockState.services]),
  addService: (config: ServiceConfig) => {
    const runtime: ServiceRuntime = {
      config,
      status: "stopped" as ServiceStatus,
      pid: null,
      cpu_percent: 0,
      memory_mb: 0,
      uptime_secs: 0,
      restart_count: 0,
      last_error: null,
    };
    _mockState.services.push(runtime);
    return delay(runtime);
  },
  updateService: (id: string, config: ServiceConfig) => {
    const idx = _mockState.services.findIndex((s) => s.config.id === id);
    if (idx === -1) return Promise.reject("服务不存在");
    _mockState.services[idx] = { ..._mockState.services[idx], config };
    return delay(_mockState.services[idx]);
  },
  removeService: (id: string, _stopFirst: boolean) => {
    _mockState.services = _mockState.services.filter((s) => s.config.id !== id);
    return delay<void>(undefined);
  },
  startService: (id: string) => {
    const svc = _mockState.services.find((s) => s.config.id === id);
    if (!svc) return Promise.reject("服务不存在");
    if (svc.status === "running") return Promise.reject("已在运行中");
    svc.status = "starting";
    setTimeout(() => {
      svc.status = "running";
      svc.pid = _mockState.nextPid++;
      svc.uptime_secs = 0;
    }, 1000);
    return delay<void>(undefined);
  },
  stopService: (id: string) => {
    const svc = _mockState.services.find((s) => s.config.id === id);
    if (!svc) return Promise.reject("服务不存在");
    svc.status = "stopping";
    setTimeout(() => {
      svc.status = "stopped";
      svc.pid = null;
      svc.cpu_percent = 0;
      svc.memory_mb = 0;
      svc.uptime_secs = 0;
    }, 800);
    return delay<void>(undefined);
  },
  restartService: (_id: string) => delay<void>(undefined, 1200),
  batchStart: (ids: string[]) => delay(ids.map((id) => ({ service_id: id, success: true, message: "启动成功" }))),
  batchStop: (ids: string[]) => delay(ids.map((id) => ({ service_id: id, success: true, message: "停止成功" }))),
  getSystemResources: () =>
    delay<SystemResource>({
      cpu_percent: 23.5,
      memory_used_gb: 12.5,
      memory_total_gb: 32.0,
      memory_percent: 39.1,
      gpu_name: "Apple M2 Pro",
      gpu_percent: 18.0,
      gpu_memory_used_mb: 2048.0,
      gpu_memory_total_mb: 21845.0,
      cpu_user_percent: 14,
      cpu_system_percent: 9.5,
      cpu_idle_percent: 76.5,
      gpu_renderer_percent: 12,
      gpu_tiler_percent: 8,
      gpu_core_count: 16,
    }),
  getResourceHistory: (): Promise<ResourceHistoryPoint[]> => {
    const now = Date.now();
    const points: ResourceHistoryPoint[] = Array.from({ length: 30 }, (_, i) => {
      const t = now - (29 - i) * 2000;
      const cpu = 15 + Math.sin(i / 3) * 10 + (i % 5);
      const mem = 35 + Math.cos(i / 4) * 5;
      const gpu = Math.max(0, 20 + Math.sin(i / 2) * 25);
      return {
        ts: t,
        cpu_percent: cpu,
        memory_percent: mem,
        cpu_user_percent: cpu * 0.6,
        cpu_system_percent: cpu * 0.4,
        cpu_idle_percent: 100 - cpu,
        gpu_percent: gpu,
        gpu_renderer_percent: gpu * 0.9,
        gpu_tiler_percent: gpu * 0.7,
      };
    });
    return delay(points);
  },
  getServiceResources: () =>
    delay<ServiceResource[]>([
      { service_id: "antibody_annotation", cpu_percent: 2.3, memory_mb: 156.7, pid: 48210, uptime_secs: 3720 },
      { service_id: "argo_portal", cpu_percent: 0.8, memory_mb: 89.2, pid: 48245, uptime_secs: 5400 },
    ]),
  getRecentLogs: (_sid: string, _count: number) =>
    delay<LogEntry[]>([
      { service_id: "antibody_annotation", timestamp: "2025-07-24 10:30:01.123", stream: "stdout", line: "Starting Antibody Annotation Server..." },
      { service_id: "antibody_annotation", timestamp: "2025-07-24 10:30:01.456", stream: "stdout", line: "Loading configuration from config.toml" },
      { service_id: "antibody_annotation", timestamp: "2025-07-24 10:30:02.001", stream: "stdout", line: "Database connection established: postgresql://localhost:5432/antibody" },
      { service_id: "antibody_annotation", timestamp: "2025-07-24 10:30:02.234", stream: "stdout", line: "Redis connection established: redis://localhost:6379" },
      { service_id: "antibody_annotation", timestamp: "2025-07-24 10:30:02.567", stream: "stdout", line: "Server listening on http://0.0.0.0:3000" },
      { service_id: "antibody_annotation", timestamp: "2025-07-24 10:30:02.890", stream: "stdout", line: "Worker pool initialized with 4 workers" },
      { service_id: "antibody_annotation", timestamp: "2025-07-24 10:30:03.100", stream: "stderr", line: "WARNING: Debug mode is disabled in production" },
      { service_id: "antibody_annotation", timestamp: "2025-07-24 10:30:15.200", stream: "stdout", line: "GET /api/annotations 200 OK - 45ms" },
      { service_id: "antibody_annotation", timestamp: "2025-07-24 10:30:18.450", stream: "stdout", line: "POST /api/annotations 201 Created - 120ms" },
      { service_id: "antibody_annotation", timestamp: "2025-07-24 10:30:22.100", stream: "stdout", line: "GET /api/annotations/123 200 OK - 12ms" },
      { service_id: "antibody_annotation", timestamp: "2025-07-24 10:30:25.300", stream: "stderr", line: "ERROR: Connection timeout on external API call (retrying...)" },
      { service_id: "antibody_annotation", timestamp: "2025-07-24 10:30:28.700", stream: "stdout", line: "External API call succeeded after retry" },
      { service_id: "antibody_annotation", timestamp: "2025-07-24 10:30:30.000", stream: "stdout", line: "Health check: OK" },
    ]),
  searchLogs: (_sid: string, keyword: string) =>
    delay<LogEntry[]>([
      { service_id: "antibody_annotation", timestamp: "2025-07-24 10:30:25.300", stream: "stderr", line: "ERROR: Connection timeout on external API call (retrying...)" },
    ]),
  getHistoryLogs: (_sid: string, _date: string) => delay<LogEntry[]>([]),
  getConfigRaw: () =>
    delay(`# 本地服务管理平台 - 服务配置文件
services:
  - id: antibody_annotation
    name: Antibody Annotation
    command: bash /Users/liubo/github/antibody_annotation/deploy.sh
    work_dir: /Users/liubo/github/antibody_annotation
    env:
      PORT: "3000"
    group: Web服务
    description: 抗体标注 Web 服务
    stop_timeout: 10

  - id: argo_portal
    name: Argo Portal
    command: bash /Users/liubo/github/argo_portal/deploy.sh
    work_dir: /Users/liubo/github/argo_portal
    group: Web服务
    stop_timeout: 10

  - id: ollama
    name: Ollama AI
    command: ollama serve
    group: AI模型
    description: Ollama 大模型推理服务
    stop_timeout: 15
`),
  saveConfigRaw: (_content: string) => delay<void>(undefined),
  validateConfig: (_content: string) => delay("ok"),
  shutdownAll: () => delay<BatchResult[]>([]),
};

// ==================== 统一导出 ====================

// 环境检测：在 Tauri 环境下使用真实后端，否则使用 mock
const isTauriEnv = isTauri();

// 缓存真实 API 的加载结果（确保 loadRealApi 只会被调用一次）
let _realApiPromise: Promise<typeof mockApi> | null = null;

async function loadRealApi(): Promise<typeof mockApi> {
  const { invoke } = await import("@tauri-apps/api/core");
  return {
    getServices: () => invoke<ServiceRuntime[]>("get_services"),
    addService: (config: ServiceConfig) => invoke<ServiceRuntime>("add_service", { config }),
    updateService: (id: string, config: ServiceConfig) => invoke<ServiceRuntime>("update_service", { id, config }),
    removeService: (id: string, stopFirst: boolean) => invoke<void>("remove_service", { id, stopFirst }),
    startService: (id: string) => invoke<void>("start_service", { id }),
    stopService: (id: string) => invoke<void>("stop_service", { id }),
    restartService: (id: string) => invoke<void>("restart_service", { id }),
    batchStart: (ids: string[]) => invoke<BatchResult[]>("batch_start", { ids }),
    batchStop: (ids: string[]) => invoke<BatchResult[]>("batch_stop", { ids }),
    getSystemResources: () => invoke<SystemResource>("get_system_resources"),
    getResourceHistory: () => invoke<ResourceHistoryPoint[]>("get_resource_history"),
    getServiceResources: () => invoke<ServiceResource[]>("get_service_resources"),
    getRecentLogs: (serviceId: string, count: number) => invoke<LogEntry[]>("get_recent_logs", { serviceId, count }),
    searchLogs: (serviceId: string, keyword: string) => invoke<LogEntry[]>("search_logs", { serviceId, keyword }),
    getHistoryLogs: (serviceId: string, date: string) => invoke<LogEntry[]>("get_history_logs", { serviceId, date }),
    getConfigRaw: () => invoke<string>("get_config_raw"),
    saveConfigRaw: (content: string) => invoke<void>("save_config_raw", { content }),
    validateConfig: (content: string) => invoke<string>("validate_config", { content }),
    shutdownAll: () => invoke<BatchResult[]>("shutdown_all_services"),
  };
}

// 获取当前应使用的 API：
// - 非 Tauri 环境：直接返回 mockApi；
// - Tauri 环境：始终等待真实后端 API 就绪后再返回，绝不回退到 mock。
function getApi(): Promise<typeof mockApi> {
  if (!isTauriEnv) return Promise.resolve(mockApi);
  if (!_realApiPromise) _realApiPromise = loadRealApi();
  return _realApiPromise;
}

// 导出的 api 对象 — 所有调用都通过此代理。
// 每个方法都会先 await 真实 API 就绪后再执行（调用方本就是 await api.xxx()，无需改动）。
export const api = new Proxy({} as typeof mockApi, {
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

export { isTauriEnv };
