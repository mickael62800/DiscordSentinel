import { httpDelete, httpGet, httpPatch, httpPost } from "@/api/http";

export type RecurrenceType = "once" | "daily" | "weekly" | "monthly" | "yearly";
export type ContentType = "text" | "embed";
export type RunStatus = "pending" | "success" | "partial" | "error";

export interface ScheduledAnnouncement {
  id: string;
  guild_id: string;
  name: string;
  enabled: boolean;
  recurrence_type: RecurrenceType;
  recurrence_hour: number;
  recurrence_minute: number;
  recurrence_day_of_week: number | null;
  recurrence_day_of_month: number | null;
  recurrence_month: number | null;
  scheduled_at: string | null;
  start_date: string;
  end_date: string | null;
  content_type: ContentType;
  content_text: string;
  embed_title: string | null;
  embed_color: number | null;
  embed_image_url: string | null;
  embed_thumbnail_url: string | null;
  embed_footer_text: string | null;
  mention_everyone: boolean;
  mention_here: boolean;
  mention_role_ids: string[];
  channel_ids: string[];
  buttons: AnnouncementButton[];
  auto_reactions: string[];
  created_by: string;
  created_at: string;
  updated_at: string;
  last_run_at: string | null;
  next_run_at: string;
}

export interface RenderedEmbed {
  title: string | null;
  description: string;
  color: number | null;
  image_url: string | null;
  thumbnail_url: string | null;
  footer_text: string | null;
}

export interface AnnouncementButton {
  label: string;
  style: "primary" | "secondary" | "success" | "danger" | "link";
  custom_id?: string | null;
  url?: string | null;
  emoji?: string | null;
}

export interface RenderedAnnouncement {
  announcement_id: string;
  run_id: string;
  guild_id: string;
  channel_ids: string[];
  content_text: string;
  embed: RenderedEmbed | null;
  mentions_prefix: string;
  buttons: AnnouncementButton[];
  auto_reactions: string[];
}

export interface ButtonInteraction {
  id: string;
  announcement_id: string;
  run_id: string | null;
  user_id: string;
  user_name: string | null;
  button_custom_id: string;
  button_label: string | null;
  clicked_at: string;
}

export interface ChannelPostResult {
  channel_id: string;
  message_id: string | null;
  success: boolean;
  error: string | null;
}

export interface AnnouncementRun {
  id: string;
  announcement_id: string;
  guild_id: string;
  ran_at: string;
  channels_posted: ChannelPostResult[];
  status: RunStatus;
  error: string | null;
}

export interface CreateAnnouncementBody {
  guild_id: string;
  name: string;
  recurrence_type: RecurrenceType;
  recurrence_hour: number;
  recurrence_minute?: number;
  recurrence_day_of_week?: number | null;
  recurrence_day_of_month?: number | null;
  recurrence_month?: number | null;
  scheduled_at?: string | null;
  end_date?: string | null;
  content_type: ContentType;
  content_text?: string;
  embed_title?: string | null;
  embed_color?: number | null;
  embed_image_url?: string | null;
  embed_thumbnail_url?: string | null;
  embed_footer_text?: string | null;
  mention_everyone?: boolean;
  mention_here?: boolean;
  mention_role_ids?: string[];
  channel_ids: string[];
  buttons?: AnnouncementButton[];
  auto_reactions?: string[];
}

export type UpdateAnnouncementBody = Omit<CreateAnnouncementBody, "guild_id">;

export const announcementsService = {
  list(guildId: string): Promise<ScheduledAnnouncement[]> {
    return httpGet(`/api/announcements/${guildId}`);
  },
  get(id: string): Promise<ScheduledAnnouncement> {
    return httpGet(`/api/announcements/by-id/${id}`);
  },
  create(body: CreateAnnouncementBody): Promise<ScheduledAnnouncement> {
    return httpPost("/api/announcements", body);
  },
  update(id: string, body: UpdateAnnouncementBody): Promise<ScheduledAnnouncement> {
    return httpPatch(`/api/announcements/by-id/${id}`, body);
  },
  delete(id: string): Promise<void> {
    return httpDelete(`/api/announcements/by-id/${id}`);
  },
  toggle(id: string, enabled: boolean): Promise<boolean> {
    return httpPost(`/api/announcements/${id}/toggle`, { enabled });
  },
  preview(id: string): Promise<RenderedAnnouncement> {
    return httpGet(`/api/announcements/${id}/preview`);
  },
  listRuns(id: string, limit = 50): Promise<AnnouncementRun[]> {
    return httpGet(`/api/announcements/${id}/runs?limit=${limit}`);
  },
  listButtonInteractions(id: string, limit = 100): Promise<ButtonInteraction[]> {
    return httpGet(`/api/announcements/${id}/interactions?limit=${limit}`);
  },
};
