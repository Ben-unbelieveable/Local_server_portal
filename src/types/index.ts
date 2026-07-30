export interface ServiceConfig {
  id: string;
  name: string;
  command: string;
  url?: string;
  work_dir?: string;
  env?: Record<string, string>;
  group?: string;
  description?: string;
  stop_timeout: number;
}

export type ServiceStatus =
  | "stopped"
  | "starting"
  | "running"
  | "stopping"
  | "failed"
  | "error";

export interface ServiceRuntime {
  config: ServiceConfig;
  status: ServiceStatus;
  pid: number | null;
  cpu_percent: number;
  memory_mb: number;
  uptime_secs: number;
  restart_count: number;
  last_error: string | null;
}

export interface SystemResource {
  cpu_percent: number;
  memory_used_gb: number;
  memory_total_gb: number;
  memory_percent: number;
  gpu_name?: string;
  gpu_percent?: number;
  gpu_memory_used_mb?: number;
  gpu_memory_total_mb?: number;
  /** CPU 用户态占比 0~100 */
  cpu_user_percent?: number;
  /** CPU 内核态占比 0~100 */
  cpu_system_percent?: number;
  /** CPU 闲置占比 0~100 */
  cpu_idle_percent?: number;
  /** GPU 渲染利用率 */
  gpu_renderer_percent?: number;
  /** GPU Tiler 利用率 */
  gpu_tiler_percent?: number;
  /** GPU 核心数 */
  gpu_core_count?: number;
}

/** 资源历史采样点（应用启动后累计） */
export interface ResourceHistoryPoint {
  ts: number;
  cpu_percent: number;
  memory_percent: number;
  cpu_user_percent?: number;
  cpu_system_percent?: number;
  cpu_idle_percent?: number;
  gpu_percent?: number;
  gpu_renderer_percent?: number;
  gpu_tiler_percent?: number;
}

export interface ServiceResource {
  service_id: string;
  cpu_percent: number;
  memory_mb: number;
  pid: number;
  uptime_secs: number;
}

export interface LogEntry {
  service_id: string;
  timestamp: string;
  stream: string;
  line: string;
}

export interface BatchResult {
  service_id: string;
  success: boolean;
  message: string;
}

export interface StatusChangeEvent {
  service_id: string;
  status: string;
  pid: number | null;
  error: string | null;
}

export interface ResourceUpdateEvent {
  system: SystemResource;
  services: ServiceResource[];
  /** 启动后采样的资源历史（环形缓冲） */
  history: ResourceHistoryPoint[];
}

/** 应用行为偏好（与服务 YAML 分离） */
export interface AppPreferences {
  /** 关闭主窗口时是否同步退出托盘 */
  quit_when_close_main: boolean;
  /** 开机启动偏好镜像 */
  launch_at_login: boolean;
}
