import { httpGet, httpPatch } from "@/api/http";

export interface AlertRule {
  id: string;
  label: string;
  metric: string;
  comparator: string; // 'gt' | 'lt'
  threshold: number | null;
  enabled: boolean;
  severity: string; // 'info' | 'warning' | 'critical'
  cooldown_secs: number;
}

export interface UpdateAlertRule {
  enabled?: boolean;
  threshold?: number;
  severity?: string;
  cooldown_secs?: number;
}

export const alertRulesService = {
  /** GET /api/alert-rules — liste des règles de supervision (superadmin). */
  list(): Promise<AlertRule[]> {
    return httpGet("/api/alert-rules");
  },
  /** PATCH /api/alert-rules/{id} — met à jour une règle. */
  update(id: string, patch: UpdateAlertRule): Promise<AlertRule> {
    return httpPatch(`/api/alert-rules/${id}`, patch);
  },
};
