use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::{VoiceChannel, VoiceChannelBan, VoiceChannelCoAdmin, VoiceChannelInviteLink, VoiceChannelTheme, VoiceChannelWhitelistEntry};
use crate::domain::errors::DomainError;
use crate::domain::value_objects::VoiceChannelKind;
use crate::ports::outbound::VoiceChannelRepository;

pub struct PgVoiceChannelRepository {
    pool: PgPool,
}

impl PgVoiceChannelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct VoiceChannelRow {
    id: Uuid,
    guild_id: String,
    owner_id: String,
    owner_name: String,
    channel_id: String,
    text_channel_id: Option<String>,
    members_channel_id: Option<String>,
    queue_channel_id: Option<String>,
    category_id: Option<String>,
    channel_name: String,
    kind: VoiceChannelKind,
    visibility: String,
    queue_enabled: bool,
    locked: bool,
    stage_enabled: bool,
    member_limit: Option<i32>,
    status: Option<String>,
    channel_status: String,
    closed_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<VoiceChannelRow> for VoiceChannel {
    fn from(row: VoiceChannelRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id,
            owner_id: row.owner_id,
            owner_name: row.owner_name,
            channel_id: row.channel_id,
            text_channel_id: row.text_channel_id,
            members_channel_id: row.members_channel_id,
            queue_channel_id: row.queue_channel_id,
            category_id: row.category_id,
            channel_name: row.channel_name,
            kind: row.kind,
            visibility: row.visibility,
            queue_enabled: row.queue_enabled,
            locked: row.locked,
            stage_enabled: row.stage_enabled,
            member_limit: row.member_limit,
            status: row.status,
            channel_status: row.channel_status,
            closed_at: row.closed_at,
            created_at: row.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct CoAdminRow {
    id: Uuid,
    voice_channel_id: Uuid,
    user_id: String,
    user_name: String,
    granted_at: chrono::DateTime<chrono::Utc>,
}

impl From<CoAdminRow> for VoiceChannelCoAdmin {
    fn from(row: CoAdminRow) -> Self {
        Self {
            id: row.id,
            voice_channel_id: row.voice_channel_id,
            user_id: row.user_id,
            user_name: row.user_name,
            granted_at: row.granted_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct WhitelistRow {
    id: Uuid,
    guild_id: String,
    owner_id: String,
    target_id: String,
    target_name: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<WhitelistRow> for VoiceChannelWhitelistEntry {
    fn from(row: WhitelistRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id,
            owner_id: row.owner_id,
            target_id: row.target_id,
            target_name: row.target_name,
            created_at: row.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct BanRow {
    id: Uuid,
    voice_channel_id: Uuid,
    user_id: String,
    user_name: String,
    banned_by: String,
    reason: Option<String>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<BanRow> for VoiceChannelBan {
    fn from(row: BanRow) -> Self {
        Self {
            id: row.id,
            voice_channel_id: row.voice_channel_id,
            user_id: row.user_id,
            user_name: row.user_name,
            banned_by: row.banned_by,
            reason: row.reason,
            expires_at: row.expires_at,
            created_at: row.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct InviteLinkRow {
    id: Uuid,
    voice_channel_id: Uuid,
    guild_id: String,
    channel_id: String,
    created_by: String,
    created_by_name: String,
    code: String,
    max_uses: Option<i32>,
    current_uses: i32,
    expires_at: chrono::DateTime<chrono::Utc>,
    revoked: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<InviteLinkRow> for VoiceChannelInviteLink {
    fn from(row: InviteLinkRow) -> Self {
        Self {
            id: row.id,
            voice_channel_id: row.voice_channel_id,
            guild_id: row.guild_id,
            channel_id: row.channel_id,
            created_by: row.created_by,
            created_by_name: row.created_by_name,
            code: row.code,
            max_uses: row.max_uses,
            current_uses: row.current_uses,
            expires_at: row.expires_at,
            revoked: row.revoked,
            created_at: row.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ThemeRow {
    id: Uuid,
    guild_id: String,
    name: String,
    emoji: Option<String>,
    channel_name_template: String,
    member_limit: Option<i32>,
    visibility: String,
    locked: bool,
    queue_enabled: bool,
    bitrate: Option<i32>,
    slowmode_secs: Option<i32>,
    stage_enabled: bool,
    is_default: bool,
    sort_order: i32,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<ThemeRow> for VoiceChannelTheme {
    fn from(row: ThemeRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id,
            name: row.name,
            emoji: row.emoji,
            channel_name_template: row.channel_name_template,
            member_limit: row.member_limit,
            visibility: row.visibility,
            locked: row.locked,
            queue_enabled: row.queue_enabled,
            bitrate: row.bitrate,
            slowmode_secs: row.slowmode_secs,
            stage_enabled: row.stage_enabled,
            is_default: row.is_default,
            sort_order: row.sort_order,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl VoiceChannelRepository for PgVoiceChannelRepository {
    // ── Channels ──

    async fn find_all(&self) -> Result<Vec<VoiceChannel>, DomainError> {
        let rows = sqlx::query_as::<_, VoiceChannelRow>(
            "SELECT * FROM voice_channels WHERE channel_status = 'open' ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(VoiceChannel::from).collect())
    }

    async fn find_all_by_guild(&self, guild_id: &str) -> Result<Vec<VoiceChannel>, DomainError> {
        let rows = sqlx::query_as::<_, VoiceChannelRow>(
            "SELECT * FROM voice_channels WHERE guild_id = $1 AND channel_status = 'open' ORDER BY created_at DESC",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(VoiceChannel::from).collect())
    }

    async fn find_by_channel_id(&self, channel_id: &str) -> Result<Option<VoiceChannel>, DomainError> {
        let row = sqlx::query_as::<_, VoiceChannelRow>(
            "SELECT * FROM voice_channels WHERE channel_id = $1 AND channel_status = 'open'",
        )
        .bind(channel_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.map(VoiceChannel::from))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<VoiceChannel>, DomainError> {
        let row = sqlx::query_as::<_, VoiceChannelRow>(
            "SELECT * FROM voice_channels WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.map(VoiceChannel::from))
    }

    async fn save(&self, channel: &VoiceChannel) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO voice_channels (id, guild_id, owner_id, owner_name, channel_id, text_channel_id, members_channel_id, queue_channel_id, category_id, channel_name, kind, visibility, queue_enabled, locked, stage_enabled, member_limit, status, channel_status, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::voice_channel_kind, $12, $13, $14, $15, $16, $17, $18, $19)
            "#,
        )
        .bind(channel.id)
        .bind(&channel.guild_id)
        .bind(&channel.owner_id)
        .bind(&channel.owner_name)
        .bind(&channel.channel_id)
        .bind(&channel.text_channel_id)
        .bind(&channel.members_channel_id)
        .bind(&channel.queue_channel_id)
        .bind(&channel.category_id)
        .bind(&channel.channel_name)
        .bind(channel.kind)
        .bind(&channel.visibility)
        .bind(channel.queue_enabled)
        .bind(channel.locked)
        .bind(channel.stage_enabled)
        .bind(channel.member_limit)
        .bind(&channel.status)
        .bind(&channel.channel_status)
        .bind(channel.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn close(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE voice_channels SET channel_status = 'closed', closed_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn close_by_channel_id(&self, channel_id: &str) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE voice_channels SET channel_status = 'closed', closed_at = NOW() WHERE channel_id = $1 AND channel_status = 'open'",
        )
        .bind(channel_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        // Soft-delete : close au lieu de delete
        self.close(id).await
    }

    async fn update_visibility(&self, id: Uuid, visibility: &str) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET visibility = $1 WHERE id = $2")
            .bind(visibility)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update_locked(&self, id: Uuid, locked: bool) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET locked = $1 WHERE id = $2")
            .bind(locked)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update_queue_enabled(&self, id: Uuid, queue_enabled: bool) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET queue_enabled = $1 WHERE id = $2")
            .bind(queue_enabled)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update_name(&self, id: Uuid, name: &str) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET channel_name = $1 WHERE id = $2")
            .bind(name)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update_status(&self, id: Uuid, status: Option<&str>) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET status = $1 WHERE id = $2")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update_member_limit(&self, id: Uuid, limit: Option<i32>) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET member_limit = $1 WHERE id = $2")
            .bind(limit)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update_owner(&self, id: Uuid, owner_id: &str, owner_name: &str) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET owner_id = $1, owner_name = $2 WHERE id = $3")
            .bind(owner_id)
            .bind(owner_name)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update_queue_channel(&self, id: Uuid, queue_channel_id: Option<&str>) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET queue_channel_id = $1 WHERE id = $2")
            .bind(queue_channel_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update_stage(&self, id: Uuid, stage_enabled: bool) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET stage_enabled = $1 WHERE id = $2")
            .bind(stage_enabled)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    // ── Co-admins ──

    async fn find_co_admins(&self, voice_channel_id: Uuid) -> Result<Vec<VoiceChannelCoAdmin>, DomainError> {
        let rows = sqlx::query_as::<_, CoAdminRow>(
            "SELECT * FROM voice_channel_co_admins WHERE voice_channel_id = $1 ORDER BY granted_at ASC",
        )
        .bind(voice_channel_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(VoiceChannelCoAdmin::from).collect())
    }

    async fn add_co_admin(&self, co_admin: &VoiceChannelCoAdmin) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO voice_channel_co_admins (id, voice_channel_id, user_id, user_name, granted_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (voice_channel_id, user_id) DO NOTHING
            "#,
        )
        .bind(co_admin.id)
        .bind(co_admin.voice_channel_id)
        .bind(&co_admin.user_id)
        .bind(&co_admin.user_name)
        .bind(co_admin.granted_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn remove_co_admin(&self, voice_channel_id: Uuid, user_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM voice_channel_co_admins WHERE voice_channel_id = $1 AND user_id = $2")
            .bind(voice_channel_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    // ── Whitelists ──

    async fn find_whitelist(&self, guild_id: &str, owner_id: &str) -> Result<Vec<VoiceChannelWhitelistEntry>, DomainError> {
        let rows = sqlx::query_as::<_, WhitelistRow>(
            "SELECT * FROM voice_channel_whitelists WHERE guild_id = $1 AND owner_id = $2 ORDER BY created_at ASC",
        )
        .bind(guild_id)
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(VoiceChannelWhitelistEntry::from).collect())
    }

    async fn add_to_whitelist(&self, entry: &VoiceChannelWhitelistEntry) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO voice_channel_whitelists (id, guild_id, owner_id, target_id, target_name, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (guild_id, owner_id, target_id) DO NOTHING
            "#,
        )
        .bind(entry.id)
        .bind(&entry.guild_id)
        .bind(&entry.owner_id)
        .bind(&entry.target_id)
        .bind(&entry.target_name)
        .bind(entry.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn remove_from_whitelist(&self, guild_id: &str, owner_id: &str, target_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM voice_channel_whitelists WHERE guild_id = $1 AND owner_id = $2 AND target_id = $3")
            .bind(guild_id)
            .bind(owner_id)
            .bind(target_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    // ── Bans ──

    async fn find_bans(&self, voice_channel_id: Uuid) -> Result<Vec<VoiceChannelBan>, DomainError> {
        let rows = sqlx::query_as::<_, BanRow>(
            "SELECT * FROM voice_channel_bans WHERE voice_channel_id = $1 ORDER BY created_at DESC",
        )
        .bind(voice_channel_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(VoiceChannelBan::from).collect())
    }

    async fn find_active_ban(&self, voice_channel_id: Uuid, user_id: &str) -> Result<Option<VoiceChannelBan>, DomainError> {
        let row = sqlx::query_as::<_, BanRow>(
            "SELECT * FROM voice_channel_bans WHERE voice_channel_id = $1 AND user_id = $2 AND (expires_at IS NULL OR expires_at > NOW())",
        )
        .bind(voice_channel_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.map(VoiceChannelBan::from))
    }

    async fn save_ban(&self, ban: &VoiceChannelBan) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO voice_channel_bans (id, voice_channel_id, user_id, user_name, banned_by, reason, expires_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (voice_channel_id, user_id) DO UPDATE SET
                banned_by = EXCLUDED.banned_by,
                reason = EXCLUDED.reason,
                expires_at = EXCLUDED.expires_at,
                created_at = EXCLUDED.created_at
            "#,
        )
        .bind(ban.id)
        .bind(ban.voice_channel_id)
        .bind(&ban.user_id)
        .bind(&ban.user_name)
        .bind(&ban.banned_by)
        .bind(&ban.reason)
        .bind(ban.expires_at)
        .bind(ban.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn remove_ban(&self, voice_channel_id: Uuid, user_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM voice_channel_bans WHERE voice_channel_id = $1 AND user_id = $2")
            .bind(voice_channel_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn cleanup_expired_bans(&self) -> Result<u64, DomainError> {
        let result = sqlx::query("DELETE FROM voice_channel_bans WHERE expires_at IS NOT NULL AND expires_at <= NOW()")
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(result.rows_affected())
    }

    // ── Invite Links ──

    async fn find_invite_links(&self, voice_channel_id: Uuid) -> Result<Vec<VoiceChannelInviteLink>, DomainError> {
        let rows = sqlx::query_as::<_, InviteLinkRow>(
            "SELECT * FROM voice_channel_invite_links WHERE voice_channel_id = $1 AND revoked = false AND expires_at > NOW() ORDER BY created_at DESC",
        )
        .bind(voice_channel_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(VoiceChannelInviteLink::from).collect())
    }

    async fn find_invite_by_code(&self, code: &str) -> Result<Option<VoiceChannelInviteLink>, DomainError> {
        let row = sqlx::query_as::<_, InviteLinkRow>(
            "SELECT * FROM voice_channel_invite_links WHERE code = $1",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.map(VoiceChannelInviteLink::from))
    }

    async fn save_invite_link(&self, link: &VoiceChannelInviteLink) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO voice_channel_invite_links (id, voice_channel_id, guild_id, channel_id, created_by, created_by_name, code, max_uses, current_uses, expires_at, revoked, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(link.id)
        .bind(link.voice_channel_id)
        .bind(&link.guild_id)
        .bind(&link.channel_id)
        .bind(&link.created_by)
        .bind(&link.created_by_name)
        .bind(&link.code)
        .bind(link.max_uses)
        .bind(link.current_uses)
        .bind(link.expires_at)
        .bind(link.revoked)
        .bind(link.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn increment_invite_uses(&self, id: Uuid) -> Result<bool, DomainError> {
        let result = sqlx::query(
            "UPDATE voice_channel_invite_links SET current_uses = current_uses + 1 WHERE id = $1 AND revoked = false AND expires_at > NOW() AND (max_uses IS NULL OR current_uses < max_uses)",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    async fn revoke_invite_link(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channel_invite_links SET revoked = true WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    // ── Themes ──

    async fn find_themes(&self, guild_id: &str) -> Result<Vec<VoiceChannelTheme>, DomainError> {
        let rows = sqlx::query_as::<_, ThemeRow>(
            "SELECT * FROM voice_channel_themes WHERE guild_id = $1 ORDER BY sort_order ASC, name ASC",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(VoiceChannelTheme::from).collect())
    }

    async fn find_theme(&self, id: Uuid) -> Result<Option<VoiceChannelTheme>, DomainError> {
        let row = sqlx::query_as::<_, ThemeRow>(
            "SELECT * FROM voice_channel_themes WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.map(VoiceChannelTheme::from))
    }

    async fn save_theme(&self, theme: &VoiceChannelTheme) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO voice_channel_themes (id, guild_id, name, emoji, channel_name_template, member_limit, visibility, locked, queue_enabled, bitrate, slowmode_secs, stage_enabled, is_default, sort_order, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            "#,
        )
        .bind(theme.id)
        .bind(&theme.guild_id)
        .bind(&theme.name)
        .bind(&theme.emoji)
        .bind(&theme.channel_name_template)
        .bind(theme.member_limit)
        .bind(&theme.visibility)
        .bind(theme.locked)
        .bind(theme.queue_enabled)
        .bind(theme.bitrate)
        .bind(theme.slowmode_secs)
        .bind(theme.stage_enabled)
        .bind(theme.is_default)
        .bind(theme.sort_order)
        .bind(theme.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update_theme(&self, theme: &VoiceChannelTheme) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE voice_channel_themes SET
                name = $2, emoji = $3, channel_name_template = $4, member_limit = $5,
                visibility = $6, locked = $7, queue_enabled = $8, bitrate = $9,
                slowmode_secs = $10, stage_enabled = $11, is_default = $12, sort_order = $13
            WHERE id = $1
            "#,
        )
        .bind(theme.id)
        .bind(&theme.name)
        .bind(&theme.emoji)
        .bind(&theme.channel_name_template)
        .bind(theme.member_limit)
        .bind(&theme.visibility)
        .bind(theme.locked)
        .bind(theme.queue_enabled)
        .bind(theme.bitrate)
        .bind(theme.slowmode_secs)
        .bind(theme.stage_enabled)
        .bind(theme.is_default)
        .bind(theme.sort_order)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn delete_theme(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM voice_channel_themes WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn clear_default_themes(&self, guild_id: &str) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channel_themes SET is_default = false WHERE guild_id = $1 AND is_default = true")
            .bind(guild_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }
}
