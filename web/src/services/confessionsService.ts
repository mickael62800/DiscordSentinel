import { httpDelete, httpGet, httpPost } from "@/api/http";

export type ReportStatus = "pending" | "resolved" | "dismissed";

export interface Confession {
  id: string;
  guild_id: string;
  public_number: number;
  author_user_id: string;
  content: string;
  message_id: string | null;
  channel_id: string | null;
  thread_id: string | null;
  deleted_at: string | null;
  deleted_by: string | null;
  deleted_reason: string | null;
  edited_at: string | null;
  created_at: string;
}

export interface ConfessionReply {
  id: string;
  confession_id: string;
  public_number: number;
  author_user_id: string;
  content: string;
  is_anonymous: boolean;
  message_id: string | null;
  deleted_at: string | null;
  deleted_by: string | null;
  edited_at: string | null;
  created_at: string;
}

export interface ConfessionReport {
  id: string;
  guild_id: string;
  confession_id: string | null;
  reply_id: string | null;
  reporter_user_id: string;
  reason: string;
  status: ReportStatus;
  resolved_by: string | null;
  resolved_at: string | null;
  created_at: string;
}

export const confessionsService = {
  list(guildId: string, includeDeleted = false, limit = 100): Promise<Confession[]> {
    return httpGet(
      `/api/confessions/${guildId}/list?limit=${limit}&include_deleted=${includeDeleted}`,
    );
  },
  delete(id: string, deletedBy: string, reason?: string): Promise<Confession> {
    return httpDelete(`/api/confessions/by-id/${id}`, { deleted_by: deletedBy, reason });
  },
  listReplies(confessionId: string): Promise<ConfessionReply[]> {
    return httpGet(`/api/confessions/by-id/${confessionId}/replies`);
  },
  deleteReply(id: string, deletedBy: string): Promise<ConfessionReply> {
    return httpDelete(`/api/confessions/replies/${id}`, { deleted_by: deletedBy });
  },
  listReports(guildId: string, status?: ReportStatus, limit = 50): Promise<ConfessionReport[]> {
    const q = status ? `?status=${status}&limit=${limit}` : `?limit=${limit}`;
    return httpGet(`/api/confessions/${guildId}/reports${q}`);
  },
  resolveReport(id: string, status: "resolved" | "dismissed", resolvedBy: string): Promise<void> {
    return httpPost(`/api/confessions/reports/${id}/resolve`, { status, resolved_by: resolvedBy });
  },
};
