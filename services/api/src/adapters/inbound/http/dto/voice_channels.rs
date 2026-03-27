use serde::{Deserialize, Serialize};

use crate::domain::entities::{
    VoiceChannel, VoiceChannelBan, VoiceChannelCoAdmin, VoiceChannelDetail,
    VoiceChannelWhitelistEntry,
};
use crate::ports::inbound::CreateVoiceChannelCommand;

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
pub struct VoiceChannelDetailDto {
    pub channel: VoiceChannelResponseDto,
    pub co_admins: Vec<CoAdminResponseDto>,
    pub bans: Vec<BanResponseDto>,
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
            kind: c.kind,
            visibility: c.visibility,
            queue_enabled: c.queue_enabled,
            locked: c.locked,
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

impl From<VoiceChannelDetail> for VoiceChannelDetailDto {
    fn from(d: VoiceChannelDetail) -> Self {
        Self {
            channel: VoiceChannelResponseDto::from(d.channel),
            co_admins: d.co_admins.into_iter().map(CoAdminResponseDto::from).collect(),
            bans: d.bans.into_iter().map(BanResponseDto::from).collect(),
        }
    }
}
