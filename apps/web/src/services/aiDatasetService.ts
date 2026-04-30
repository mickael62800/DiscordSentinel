import { httpDelete, httpGet } from "@/api/http";

export interface DatasetMessage {
  id: string;
  user_id: string;
  channel_id: string | null;
  channel_name: string | null;
  content: string;
  created_at: string;
}

export interface ListMessagesResponse {
  items: DatasetMessage[];
  total: number;
}

export interface ListMessagesParams {
  channel_id?: string;
  from?: string;
  to?: string;
  min_length?: number;
  limit?: number;
  offset?: number;
}

function buildQuery(params: ListMessagesParams): string {
  const u = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== null && v !== "") u.set(k, String(v));
  }
  const s = u.toString();
  return s ? `?${s}` : "";
}

export const aiDatasetService = {
  listMessages(guildId: string, params: ListMessagesParams = {}): Promise<ListMessagesResponse> {
    return httpGet(`/api/ai-dataset/messages/${guildId}${buildQuery(params)}`);
  },
  bulkDelete(guildId: string, ids: string[]): Promise<{ deleted: number }> {
    return httpDelete(`/api/ai-dataset/messages/${guildId}`, { ids });
  },
};
