use serde::{Deserialize, Serialize};

use crate::domain::entities::{
    VoiceChannel, VoiceChannelBan, VoiceChannelCoAdmin, VoiceChannelDetail,
    VoiceChannelInviteLink, VoiceChannelTheme, VoiceChannelWhitelistEntry,
};
use crate::ports::inbound::{CreateInviteLinkCommand, CreateThemeCommand, CreateVoiceChannelCommand};

// ── Request DTOs ──

#[derive(Debug, Deserialize)]
pub struct CreateVoiceChannelDto {
    pub guild_id: String,
    pub owner_id: String,
    pub owner_name: String,
    pub channel_id: String,
    pub text_channel_id: Option<String>,
    pub members_channel_id: Option<String>,
    pub queue_channel_id: Option<String>,
    pub category_id: Option<String>,
    pub channel_name: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default = "default_visibility")]
    pub visibility: String,
    #[serde(default)]
    pub queue_enabled: bool,
    #[serde(default)]
    pub stage_enabled: bool,
}

fn default_kind() -> String {
    "public".to_string()
}

fn default_visibility() -> String {
    "visible".to_string()
}

#[derive(Debug, Deserialize)]
pub struct UpdateVoiceChannelDto {
    pub visibility: Option<String>,
    pub locked: Option<bool>,
    pub queue_enabled: Option<bool>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub member_limit: Option<Option<i32>>,
    pub queue_channel_id: Option<Option<String>>,
    pub stage_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct TransferOwnershipDto {
    pub new_owner_id: String,
    pub new_owner_name: String,
}

#[derive(Debug, Deserialize)]
pub struct AddCoAdminDto {
    pub user_id: String,
    pub user_name: String,
}

#[derive(Debug, Deserialize)]
pub struct AddWhitelistDto {
    pub guild_id: String,
    pub owner_id: String,
    pub target_id: String,
    pub target_name: String,
}

#[derive(Debug, Deserialize)]
pub struct BanFromChannelDto {
    pub user_id: String,
    pub user_name: String,
    pub banned_by: String,
    pub reason: Option<String>,
    pub duration_secs: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInviteLinkDto {
    pub created_by: String,
    pub created_by_name: String,
    pub duration_secs: Option<i64>,
    pub max_uses: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UseInviteLinkDto {
    pub user_id: String,
    pub user_name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateThemeDto {
    pub name: String,
    pub emoji: Option<String>,
    #[serde(default = "default_channel_name_template")]
    pub channel_name_template: String,
    pub member_limit: Option<i32>,
    #[serde(default = "default_visibility")]
    pub visibility: String,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub queue_enabled: bool,
    pub bitrate: Option<i32>,
    pub slowmode_secs: Option<i32>,
    #[serde(default)]
    pub stage_enabled: bool,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub sort_order: i32,
}

fn default_channel_name_template() -> String {
    "{user}".to_string()
}

// ── Response DTOs ──

#[derive(Debug, Serialize)]
pub struct VoiceChannelResponseDto {
    pub id: String,
    pub guild_id: String,
    pub owner_id: String,
    pub owner_name: String,
    pub channel_id: String,
    pub text_channel_id: Option<String>,
    pub members_channel_id: Option<String>,
    pub queue_channel_id: Option<String>,
    pub category_id: Option<String>,
    pub channel_name: String,
    pub kind: String,
    pub visibility: String,
    pub queue_enabled: bool,
    pub locked: bool,
    pub stage_enabled: bool,
    pub member_limit: Option<i32>,
    pub status: Option<String>,
    pub channel_status: String,
    pub closed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct CoAdminResponseDto {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    pub granted_at: String,
}

#[derive(Debug, Serialize)]
pub struct WhitelistEntryResponseDto {
    pub id: String,
    pub owner_id: String,
    pub target_id: String,
    pub target_name: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct BanResponseDto {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    pub banned_by: String,
    pub reason: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct InviteLinkResponseDto {
    pub id: String,
    pub channel_id: String,
    pub guild_id: String,
    pub created_by: String,
    pub created_by_name: String,
    pub code: String,
    pub max_uses: Option<i32>,
    pub current_uses: i32,
    pub expires_at: String,
    pub revoked: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct VoiceChannelDetailDto {
    pub channel: VoiceChannelResponseDto,
    pub co_admins: Vec<CoAdminResponseDto>,
    pub bans: Vec<BanResponseDto>,
    pub invite_links: Vec<InviteLinkResponseDto>,
}

// ── From impls ──

impl From<CreateVoiceChannelDto> for CreateVoiceChannelCommand {
    fn from(dto: CreateVoiceChannelDto) -> Self {
        Self {
            guild_id: dto.guild_id,
            owner_id: dto.owner_id,
            owner_name: dto.owner_name,
            channel_id: dto.channel_id,
            text_channel_id: dto.text_channel_id,
            members_channel_id: dto.members_channel_id,
            queue_channel_id: dto.queue_channel_id,
            category_id: dto.category_id,
            channel_name: dto.channel_name,
            kind: dto.kind,
            visibility: dto.visibility,
            queue_enabled: dto.queue_enabled,
            stage_enabled: dto.stage_enabled,
        }
    }
}

impl From<VoiceChannel> for VoiceChannelResponseDto {
    fn from(c: VoiceChannel) -> Self {
        Self {
            id: c.id.to_string(),
            guild_id: c.guild_id,
            owner_id: c.owner_id,
            owner_name: c.owner_name,
            channel_id: c.channel_id,
            text_channel_id: c.text_channel_id,
            members_channel_id: c.members_channel_id,
            queue_channel_id: c.queue_channel_id,
            category_id: c.category_id,
            channel_name: c.channel_name,
            kind: c.kind.as_str().to_string(),
            visibility: c.visibility,
            queue_enabled: c.queue_enabled,
            locked: c.locked,
            stage_enabled: c.stage_enabled,
            member_limit: c.member_limit,
            status: c.status,
            channel_status: c.channel_status,
            closed_at: c.closed_at.map(|t| t.to_rfc3339()),
            created_at: c.created_at.to_rfc3339(),
        }
    }
}

impl From<VoiceChannelCoAdmin> for CoAdminResponseDto {
    fn from(ca: VoiceChannelCoAdmin) -> Self {
        Self {
            id: ca.id.to_string(),
            user_id: ca.user_id,
            user_name: ca.user_name,
            granted_at: ca.granted_at.to_rfc3339(),
        }
    }
}

impl From<VoiceChannelWhitelistEntry> for WhitelistEntryResponseDto {
    fn from(w: VoiceChannelWhitelistEntry) -> Self {
        Self {
            id: w.id.to_string(),
            owner_id: w.owner_id,
            target_id: w.target_id,
            target_name: w.target_name,
            created_at: w.created_at.to_rfc3339(),
        }
    }
}

impl From<VoiceChannelBan> for BanResponseDto {
    fn from(b: VoiceChannelBan) -> Self {
        Self {
            id: b.id.to_string(),
            user_id: b.user_id,
            user_name: b.user_name,
            banned_by: b.banned_by,
            reason: b.reason,
            expires_at: b.expires_at.map(|t| t.to_rfc3339()),
            created_at: b.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ThemeResponseDto {
    pub id: String,
    pub guild_id: String,
    pub name: String,
    pub emoji: Option<String>,
    pub channel_name_template: String,
    pub member_limit: Option<i32>,
    pub visibility: String,
    pub locked: bool,
    pub queue_enabled: bool,
    pub bitrate: Option<i32>,
    pub slowmode_secs: Option<i32>,
    pub stage_enabled: bool,
    pub is_default: bool,
    pub sort_order: i32,
    pub created_at: String,
}

impl From<VoiceChannelTheme> for ThemeResponseDto {
    fn from(t: VoiceChannelTheme) -> Self {
        Self {
            id: t.id.to_string(),
            guild_id: t.guild_id,
            name: t.name,
            emoji: t.emoji,
            channel_name_template: t.channel_name_template,
            member_limit: t.member_limit,
            visibility: t.visibility,
            locked: t.locked,
            queue_enabled: t.queue_enabled,
            bitrate: t.bitrate,
            slowmode_secs: t.slowmode_secs,
            stage_enabled: t.stage_enabled,
            is_default: t.is_default,
            sort_order: t.sort_order,
            created_at: t.created_at.to_rfc3339(),
        }
    }
}

impl From<CreateThemeDto> for CreateThemeCommand {
    fn from(dto: CreateThemeDto) -> Self {
        Self {
            guild_id: String::new(), // set by handler from path
            name: dto.name,
            emoji: dto.emoji,
            channel_name_template: dto.channel_name_template,
            member_limit: dto.member_limit,
            visibility: dto.visibility,
            locked: dto.locked,
            queue_enabled: dto.queue_enabled,
            bitrate: dto.bitrate,
            slowmode_secs: dto.slowmode_secs,
            stage_enabled: dto.stage_enabled,
            is_default: dto.is_default,
            sort_order: dto.sort_order,
        }
    }
}

impl From<VoiceChannelInviteLink> for InviteLinkResponseDto {
    fn from(l: VoiceChannelInviteLink) -> Self {
        Self {
            id: l.id.to_string(),
            channel_id: l.channel_id,
            guild_id: l.guild_id,
            created_by: l.created_by,
            created_by_name: l.created_by_name,
            code: l.code,
            max_uses: l.max_uses,
            current_uses: l.current_uses,
            expires_at: l.expires_at.to_rfc3339(),
            revoked: l.revoked,
            created_at: l.created_at.to_rfc3339(),
        }
    }
}

impl From<CreateInviteLinkDto> for CreateInviteLinkCommand {
    fn from(dto: CreateInviteLinkDto) -> Self {
        Self {
            channel_id: String::new(), // set by handler from path
            created_by: dto.created_by,
            created_by_name: dto.created_by_name,
            duration_secs: dto.duration_secs,
            max_uses: dto.max_uses,
        }
    }
}

impl From<VoiceChannelDetail> for VoiceChannelDetailDto {
    fn from(d: VoiceChannelDetail) -> Self {
        Self {
            channel: VoiceChannelResponseDto::from(d.channel),
            co_admins: d.co_admins.into_iter().map(CoAdminResponseDto::from).collect(),
            bans: d.bans.into_iter().map(BanResponseDto::from).collect(),
            invite_links: d.invite_links.into_iter().map(InviteLinkResponseDto::from).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_channel() -> VoiceChannel {
        VoiceChannel {
            id: Uuid::new_v4(),
            guild_id: "guild1".into(),
            owner_id: "owner1".into(),
            owner_name: "Owner".into(),
            channel_id: "chan1".into(),
            text_channel_id: Some("text1".into()),
            members_channel_id: Some("mem1".into()),
            queue_channel_id: None,
            category_id: Some("cat1".into()),
            channel_name: "Salon de Owner".into(),
            kind: crate::domain::value_objects::VoiceChannelKind::Private,
            visibility: "visible".into(),
            queue_enabled: false,
            locked: false,
            stage_enabled: false,
            member_limit: Some(10),
            status: Some("Cool".into()),
            channel_status: "open".into(),
            closed_at: None,
            created_at: Utc::now(),
        }
    }

    fn make_theme() -> VoiceChannelTheme {
        VoiceChannelTheme {
            id: Uuid::new_v4(),
            guild_id: "guild1".into(),
            name: "Gaming".into(),
            emoji: Some("🎮".into()),
            channel_name_template: "{user} Gaming".into(),
            member_limit: Some(5),
            visibility: "visible".into(),
            locked: false,
            queue_enabled: false,
            bitrate: Some(64000),
            slowmode_secs: Some(10),
            stage_enabled: true,
            is_default: true,
            sort_order: 0,
            created_at: Utc::now(),
        }
    }

    fn make_invite_link() -> VoiceChannelInviteLink {
        VoiceChannelInviteLink {
            id: Uuid::new_v4(),
            voice_channel_id: Uuid::new_v4(),
            guild_id: "guild1".into(),
            channel_id: "chan1".into(),
            created_by: "user1".into(),
            created_by_name: "User".into(),
            code: "ABCD1234".into(),
            max_uses: Some(5),
            current_uses: 2,
            expires_at: Utc::now() + chrono::Duration::hours(1),
            revoked: false,
            created_at: Utc::now(),
        }
    }

    // ── VoiceChannel → VoiceChannelResponseDto ──

    #[test]
    fn channel_to_dto_preserves_fields() {
        let ch = make_channel();
        let id = ch.id;
        let dto = VoiceChannelResponseDto::from(ch);
        assert_eq!(dto.id, id.to_string());
        assert_eq!(dto.guild_id, "guild1");
        assert_eq!(dto.kind, "private");
        assert_eq!(dto.member_limit, Some(10));
        assert!(!dto.stage_enabled);
    }

    #[test]
    fn channel_to_dto_formats_dates() {
        let ch = make_channel();
        let dto = VoiceChannelResponseDto::from(ch);
        assert!(dto.created_at.contains("T")); // RFC3339 format
        assert!(dto.closed_at.is_none());
    }

    #[test]
    fn channel_to_dto_closed_at_some() {
        let mut ch = make_channel();
        ch.closed_at = Some(Utc::now());
        let dto = VoiceChannelResponseDto::from(ch);
        assert!(dto.closed_at.is_some());
    }

    // ── VoiceChannelTheme → ThemeResponseDto ──

    #[test]
    fn theme_to_dto_preserves_all_fields() {
        let theme = make_theme();
        let dto = ThemeResponseDto::from(theme);
        assert_eq!(dto.name, "Gaming");
        assert_eq!(dto.emoji, Some("🎮".into()));
        assert_eq!(dto.member_limit, Some(5));
        assert_eq!(dto.bitrate, Some(64000));
        assert_eq!(dto.slowmode_secs, Some(10));
        assert!(dto.stage_enabled);
        assert!(dto.is_default);
    }

    #[test]
    fn theme_to_dto_none_optionals() {
        let mut theme = make_theme();
        theme.emoji = None;
        theme.member_limit = None;
        theme.bitrate = None;
        theme.slowmode_secs = None;
        let dto = ThemeResponseDto::from(theme);
        assert!(dto.emoji.is_none());
        assert!(dto.member_limit.is_none());
    }

    // ── VoiceChannelInviteLink → InviteLinkResponseDto ──

    #[test]
    fn invite_link_to_dto_preserves_fields() {
        let link = make_invite_link();
        let dto = InviteLinkResponseDto::from(link);
        assert_eq!(dto.code, "ABCD1234");
        assert_eq!(dto.max_uses, Some(5));
        assert_eq!(dto.current_uses, 2);
        assert!(!dto.revoked);
    }

    #[test]
    fn invite_link_to_dto_formats_dates() {
        let link = make_invite_link();
        let dto = InviteLinkResponseDto::from(link);
        assert!(dto.expires_at.contains("T"));
        assert!(dto.created_at.contains("T"));
    }

    // ── CreateThemeDto → CreateThemeCommand ──

    #[test]
    fn theme_dto_to_command_sets_empty_guild() {
        let dto = CreateThemeDto {
            name: "Test".into(),
            emoji: None,
            channel_name_template: "{user}".into(),
            member_limit: None,
            visibility: "visible".into(),
            locked: false,
            queue_enabled: false,
            bitrate: None,
            slowmode_secs: None,
            stage_enabled: false,
            is_default: false,
            sort_order: 0,
        };
        let cmd: CreateThemeCommand = dto.into();
        assert_eq!(cmd.guild_id, ""); // set by handler
        assert_eq!(cmd.name, "Test");
    }

    // ── VoiceChannelDetail → VoiceChannelDetailDto ──

    #[test]
    fn detail_to_dto_aggregates_all() {
        let detail = VoiceChannelDetail {
            channel: make_channel(),
            co_admins: vec![],
            bans: vec![],
            invite_links: vec![make_invite_link()],
        };
        let dto = VoiceChannelDetailDto::from(detail);
        assert!(dto.co_admins.is_empty());
        assert!(dto.bans.is_empty());
        assert_eq!(dto.invite_links.len(), 1);
        assert_eq!(dto.invite_links[0].code, "ABCD1234");
    }

    // ── Default functions ──

    #[test]
    fn default_kind_is_public() {
        assert_eq!(default_kind(), "public");
    }

    #[test]
    fn default_visibility_is_visible() {
        assert_eq!(default_visibility(), "visible");
    }

    #[test]
    fn default_channel_name_template_is_user() {
        assert_eq!(default_channel_name_template(), "{user}");
    }
}
