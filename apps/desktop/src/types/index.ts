export interface Guild {
  guild_id: string;
  name: string;
  icon: string | null;
  member_count: number;
}

export interface BotDefinition {
  bot_name: string;
  display_name: string;
  description: string;
  config_schema: ConfigField[];
}

export interface ConfigField {
  key: string;
  label: string;
  type: string;
  required: boolean;
}

export interface BotGuildConfig {
  guild_id: string;
  bot_name: string;
  config_key: string;
  config_value: string;
}

export interface DiscordConfig {
  client_id: string;
  client_secret: string;
}

export interface ApiConfig {
  api_url: string;
  api_key: string;
}

export interface DiscordUser {
  id: string;
  username: string;
  discriminator: string;
  avatar: string | null;
  global_name: string | null;
}

export interface ServerStats {
  total_servers: number;
  total_users: number;
  messages_today: number;
  infractions_today: number;
  bots_online: number;
  bots_total: number;
}

export interface LogEntry {
  id: string;
  timestamp: string;
  level: string;
  bot: string;
  server: string;
  message: string;
}

export interface Infraction {
  id: string;
  user_id: string;
  username: string;
  server: string;
  infraction_type: string;
  reason: string;
  created_at: string;
  moderator: string;
}

export interface ModerationRule {
  id: string;
  name: string;
  enabled: boolean;
  rule_type: string;
  action: string;
  description: string;
}

export interface UpdateRuleParams {
  guild_id: string;
  flag_type: string;
  weight: number;
  threshold_warn: number;
  threshold_delete: number;
  threshold_mute: number;
  threshold_ban: number;
  enabled: boolean;
}

export interface TableColumn {
  key: string;
  label: string;
}

export interface Notification {
  id: string;
  notification_type: string;
  title: string;
  message: string;
  severity: string;
  read: boolean;
  created_at: string;
}

export interface SecurityEvent {
  id: string;
  guild_id: string;
  event_type: string;
  severity: string;
  description: string;
  user_ids: string[];
  created_at: string;
}

export interface ModerationActionRequest {
  guild_id: string;
  channel_id: string;
  moderator_id: string;
  moderator_name: string;
  target_id: string;
  target_name: string;
  action_type: string;
  reason: string;
  gravity?: string;
  duration?: number;
}

export interface ModerationActionResponse {
  id: string;
  action_type: string;
  target_name: string;
  reason: string;
}

export interface UserModerationHistory {
  target_id: string;
  target_name: string;
  total_warns: number;
  total_mutes: number;
  total_bans: number;
  actions: ModerationActionResponse[];
}

export interface SelectOption {
  value: string;
  label: string;
}

export interface Ticket {
  id: string;
  title: string;
  status: string;
  priority: string;
  author_id: string;
  author_name: string;
  assigned_to: string | null;
  server: string;
  category: string;
  created_at: string;
  updated_at: string;
  messages_count: number;
}

export interface TicketMessage {
  id: string;
  ticket_id: string;
  author_name: string;
  author_role: string;
  content: string;
  created_at: string;
}

export interface TicketDetail {
  ticket: Ticket;
  messages: TicketMessage[];
}

// ── Voice Channels ──

export interface VoiceChannel {
  id: string;
  guild_id: string;
  owner_id: string;
  owner_name: string;
  channel_id: string;
  text_channel_id: string | null;
  members_channel_id: string | null;
  queue_channel_id: string | null;
  category_id: string | null;
  channel_name: string;
  kind: string;
  visibility: string;
  queue_enabled: boolean;
  locked: boolean;
  member_limit: number | null;
  status: string | null;
  created_at: string;
}

export interface VoiceChannelCoAdmin {
  id: string;
  user_id: string;
  user_name: string;
  granted_at: string;
}

export interface VoiceChannelBan {
  id: string;
  user_id: string;
  user_name: string;
  banned_by: string;
  reason: string | null;
  expires_at: string | null;
  created_at: string;
}

export interface VoiceChannelDetail {
  channel: VoiceChannel;
  co_admins: VoiceChannelCoAdmin[];
  bans: VoiceChannelBan[];
}

// ── Conduct (points de conduite) ──

export interface ConductConfig {
  guild_id: string;
  max_points: number;
  regen_amount: number;
  regen_interval: string;
  penalty_warn: number;
  penalty_delete: number;
  penalty_mute: number;
  penalty_ban: number;
}

export interface UserConductPoints {
  id: string;
  guild_id: string;
  user_id: string;
  username: string;
  points: number;
  last_regen_at: string;
  created_at: string;
}

export interface ConductPointsLog {
  id: string;
  delta: number;
  reason: string;
  points_before: number;
  points_after: number;
  created_at: string;
}
