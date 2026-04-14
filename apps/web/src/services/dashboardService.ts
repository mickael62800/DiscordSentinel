import { httpGet } from "@/api/http";
import type { ServerStats } from "@/types";

export const dashboardService = {
  getStats(): Promise<ServerStats> { return httpGet("/api/stats"); },
};
