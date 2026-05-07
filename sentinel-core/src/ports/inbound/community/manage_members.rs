use async_trait::async_trait;

use crate::domain::entities::community::guild_member::GuildMember;
use crate::domain::entities::community::guild_member::MemberSummary;
use crate::domain::errors::DomainError;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::entities::system::discord_ids::GuildId;

pub struct SyncMembersCommand {
    pub guild_id: GuildId,
    pub members: Vec<GuildMember>,
}

pub struct RegisterMemberCommand {
    pub member: GuildMember,
}

pub struct UpdateMemberCommand {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub roles: Option<serde_json::Value>,
}

#[async_trait]
pub trait ManageMembersUseCase: Send + Sync {
    async fn list_members(&self, guild_id: &str) -> Result<Vec<GuildMember>, DomainError>;
    async fn get_member(&self, guild_id: &str, user_id: &str) -> Result<GuildMember, DomainError>;
    async fn get_member_summary(&self, guild_id: &str, user_id: &str) -> Result<MemberSummary, DomainError>;
    async fn sync_members(&self, cmd: SyncMembersCommand) -> Result<u64, DomainError>;
    async fn register_member(&self, cmd: RegisterMemberCommand) -> Result<(), DomainError>;
    async fn remove_member(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
    async fn update_member(&self, cmd: UpdateMemberCommand) -> Result<(), DomainError>;
}
