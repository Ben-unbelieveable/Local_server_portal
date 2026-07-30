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
}
