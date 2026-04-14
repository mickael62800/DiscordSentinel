import { httpGet } from "@/api/http";

export interface ServiceStatus {
  name: string;
  online: boolean;
}

export interface HostMetrics {
  cpu_percent: number;
  cpu_cores: number;
  mem_used_mb: number;
  mem_total_mb: number;
}

export interface ProcessMetrics {
  cpu_percent: number;
  mem_used_mb: number;
}

export interface RedisMetrics {
  used_memory_mb: number;
  connected_clients: number;
  total_keys: number;
  uptime_seconds: number;
}

export interface SystemInfo {
  bots: ServiceStatus[];
  workers: ServiceStatus[];
  host: HostMetrics;
  process: ProcessMetrics;
  redis: RedisMetrics;
  uptime_seconds: number;
  db_size_mb: number;
}

export const systemService = {
  getInfo(): Promise<SystemInfo> {
    return httpGet("/api/system/info");
  },
};
