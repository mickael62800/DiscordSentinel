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
  workers_online: number;
  workers_total: number;
  postgres_online: boolean;
  redis_online: boolean;
}

export interface LogEntry {
  id: string;
  timestamp: string;
  level: string;
  bot: string;
  server: string;
  message: string;
  category: string;
  details: Record<string, unknown>;
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

export interface ConfirmedBan {
  id: string;
  guild_id: string;
  target_id: string;
  target_name: string;
  moderator_name: string;
  action_type: string;
  reason: string;
  created_at: string;
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

// ── Role Panels ──

export interface RolePanel {
  id: string;
  guild_id: string;
  channel_id: string;
  message_id: string | null;
  title: string;
  description: string;
  mode: string;
  max_roles: number | null;
  enabled: boolean;
  created_at: string;
}

export interface RolePanelEntry {
  id: string;
  role_id: string;
  role_name: string;
  emoji: string | null;
  label: string;
  style: string;
  position: number;
}

export interface RolePanelDetail {
  panel: RolePanel;
  entries: RolePanelEntry[];
}

export interface AutoRoleConfig {
  id: string;
  guild_id: string;
  role_id: string;
  role_name: string;
  delay_secs: number;
  enabled: boolean;
}

// ── Dashboard Charts ──

export interface DailyActivity {
  day: string;
  messages: number;
  voice_minutes: number;
  active_members: number;
  new_members: number;
  leaves: number;
  infractions: number;
  warns: number;
  mutes: number;
  bans: number;
}

export interface TopUser {
  user_id: string;
  username: string;
  message_count: number;
  voice_seconds: number;
  voice_hours: number;
}

// ── Levels / XP ──

export interface LevelConfig {
  guild_id: string;
  xp_per_message: number;
  xp_per_voice_minute: number;
  xp_cooldown_secs: number;
  level_up_channel_id: string | null;
  level_up_message: string;
  excluded_channels: string[];
  enabled: boolean;
}

export interface UserLevel {
  id: string;
  guild_id: string;
  user_id: string;
  username: string;
  xp: number;
  level: number;
  xp_current: number;
  xp_needed: number;
  last_xp_at: string;
}

export interface LevelReward {
  id: string;
  guild_id: string;
  level: number;
  role_id: string;
}

// ── Audit Logs ──

export interface AuditLog {
  id: string;
  guild_id: string;
  event_type: string;
  actor_id: string | null;
  actor_name: string | null;
  target_id: string | null;
  target_name: string | null;
  channel_id: string | null;
  channel_name: string | null;
  details: Record<string, unknown>;
  created_at: string;
}

// ── Watched Users (Surveillance) ──

export interface WatchedUser {
  user_id: string;
  username: string;
  guild_id: string;
  guild_name: string;
  risk_level: string;
  total_warns: number;
  total_mutes: number;
  total_bans: number;
  conduct_points: number | null;
  max_conduct_points: number | null;
  last_incident_at: string | null;
  security_events_count: number;
  first_seen_at: string;
}

export interface UserDossier {
  user: WatchedUser;
  infractions: Infraction[];
  moderation_actions: ModerationActionResponse[];
  security_events: SecurityEvent[];
  conduct_log: ConductPointsLog[];
}

// ── IA Config (seuils confiance per-guild) ──

export interface IaConfig {
  guild_id: string;
  text_enabled: boolean;
  text_threshold: number;
  vision_enabled: boolean;
  vision_threshold: number;
}

export interface SaveIaConfigParams {
  text_enabled: boolean;
  text_threshold: number;
  vision_enabled: boolean;
  vision_threshold: number;
}

// ── Analytics ──

export interface HeatmapPoint {
  hour: number;
  day_of_week: number;
  day_name: string;
  messages: number;
  infractions: number;
}

export interface ActionDistribution {
  action: string;
  count: number;
  percentage: number;
}

export interface TopInfractor {
  user_id: string;
  username: string;
  total_infractions: number;
  warns: number;
  deletes: number;
  mutes: number;
  bans: number;
}

export interface ModerationTrend {
  day: string;
  total: number;
  warns: number;
  deletes: number;
  mutes: number;
  bans: number;
}

export interface PeakHour {
  hour: number;
  label: string;
  avg_messages: number;
  avg_infractions: number;
}

export interface FullAnalytics {
  heatmap: HeatmapPoint[];
  action_distribution: ActionDistribution[];
  top_infractors: TopInfractor[];
  moderation_trend: ModerationTrend[];
  peak_hours: PeakHour[];
}
