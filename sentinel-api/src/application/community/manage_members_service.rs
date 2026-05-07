use std::sync::Arc;

use async_trait::async_trait;

use sentinel_core::domain::entities::community::guild_member::GuildMember;
use sentinel_core::domain::entities::community::guild_member::MemberConduct;
use sentinel_core::domain::entities::community::guild_member::MemberInfractions;
use sentinel_core::domain::entities::community::guild_member::MemberModeration;
use sentinel_core::domain::entities::community::guild_member::MemberStats;
use sentinel_core::domain::entities::community::guild_member::MemberSummary;
use sentinel_core::domain::errors::DomainError;
use crate::ports::inbound::moderation::manage_infractions::InfractionFilters;
use crate::ports::inbound::community::manage_conduct::ManageConductUseCase;
use crate::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use crate::ports::inbound::community::manage_members::ManageMembersUseCase;
use crate::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
use crate::ports::inbound::audit::manage_stats::ManageStatsUseCase;
use crate::ports::inbound::community::manage_members::RegisterMemberCommand;
use crate::ports::inbound::community::manage_members::SyncMembersCommand;
use crate::ports::inbound::community::manage_members::UpdateMemberCommand;
use crate::ports::outbound::community::member_repository::MemberRepository;

pub struct ManageMembersService {
    member_repo: Arc<dyn MemberRepository>,
    infractions_uc: Arc<dyn ManageInfractionsUseCase>,
    moderation_uc: Arc<dyn ManageModerationUseCase>,
    conduct_uc: Arc<dyn ManageConductUseCase>,
    stats_uc: Arc<dyn ManageStatsUseCase>,
}

impl ManageMembersService {
    pub fn new(
        member_repo: Arc<dyn MemberRepository>,
        infractions_uc: Arc<dyn ManageInfractionsUseCase>,
        moderation_uc: Arc<dyn ManageModerationUseCase>,
        conduct_uc: Arc<dyn ManageConductUseCase>,
        stats_uc: Arc<dyn ManageStatsUseCase>,
    ) -> Self {
        Self { member_repo, infractions_uc, moderation_uc, conduct_uc, stats_uc }
    }
}

#[async_trait]
impl ManageMembersUseCase for ManageMembersService {
    async fn list_members(&self, guild_id: &str) -> Result<Vec<GuildMember>, DomainError> {
        self.member_repo.find_by_guild(guild_id).await
    }

    async fn get_member(&self, guild_id: &str, user_id: &str) -> Result<GuildMember, DomainError> {
        self.member_repo.find_one(guild_id, user_id).await?
            .ok_or_else(|| DomainError::NotFound(format!("Membre {user_id} introuvable dans {guild_id}")))
    }

    async fn get_member_summary(&self, guild_id: &str, user_id: &str) -> Result<MemberSummary, DomainError> {
        let member = self.get_member(guild_id, user_id).await?;

        // Infractions
        let infractions_list = self.infractions_uc.list_infractions(guild_id, InfractionFilters {
            user_id: Some(user_id.to_string()),
            action: None,
            limit: 20,
            offset: 0,
        }).await.unwrap_or_else(|e| {
            tracing::warn!(error = %e, guild_id, user_id, "Echec chargement infractions pour summary");
            vec![]
        });
        let infractions_total = infractions_list.len() as i64;
        let infractions_recent: Vec<serde_json::Value> = infractions_list.iter().take(10)
            .map(|i| serde_json::json!({
                "id": i.id.to_string(),
                "created_at": i.created_at.to_rfc3339(),
                "reason": i.reason,
                "score": i.score,
                "action": format!("{:?}", i.action),
                "content": i.content,
            }))
            .collect();

        // Moderation
        let mod_history = match self.moderation_uc.get_history(guild_id, user_id).await {
            Ok(h) => Some(h),
            Err(e) => {
                tracing::warn!(error = %e, guild_id, user_id, "Echec chargement historique moderation pour summary");
                None
            }
        };
        let (total_warns, total_mutes, total_bans, mod_actions) = if let Some(ref h) = mod_history {
            let warns = h.actions.iter().filter(|a| a.action_type == "warn").count() as i64;
            let mutes = h.actions.iter().filter(|a| a.action_type == "mute").count() as i64;
            let bans = h.actions.iter().filter(|a| a.action_type == "ban").count() as i64;
            let actions: Vec<serde_json::Value> = h.actions.iter().take(10)
                .map(|a| serde_json::json!({
                    "action_type": a.action_type,
                    "reason": a.reason,
                    "moderator_name": a.moderator_name,
                    "created_at": a.created_at.to_rfc3339(),
                    "duration": a.duration,
                }))
                .collect();
            (warns, mutes, bans, actions)
        } else {
            (0, 0, 0, vec![])
        };

        // Conduct
        let conduct_points = match self.conduct_uc.get_points(guild_id, user_id).await {
            Ok(p) => Some(p),
            Err(e) => {
                tracing::warn!(error = %e, guild_id, user_id, "Echec chargement points conduite pour summary");
                None
            }
        };
        let conduct_config = match self.conduct_uc.get_config(guild_id).await {
            Ok(cfg) => cfg,
            Err(_) => sentinel_core::domain::entities::community::conduct::ConductConfig::default_for_guild(guild_id),
        };
        let points = conduct_points.as_ref().map(|c| c.points).unwrap_or(conduct_config.max_points);
        let max_points = conduct_config.max_points;
        let conduct_log_entries = self.conduct_uc.get_points_log(guild_id, user_id, 20).await.unwrap_or_else(|e| {
            tracing::warn!(error = %e, guild_id, user_id, "Echec chargement log conduite pour summary");
            vec![]
        });
        let conduct_log: Vec<serde_json::Value> = conduct_log_entries.iter().take(20)
            .map(|l| serde_json::json!({
                "delta": l.delta,
                "reason": l.reason,
                "created_at": l.created_at.to_rfc3339(),
            }))
            .collect();

        // Stats
        let user_stats = self.stats_uc.get_user_stats(guild_id, user_id).await.ok().flatten();
        let (message_count, voice_seconds, last_active) = user_stats
            .map(|s| (s.message_count as i64, s.voice_seconds as i64, Some(s.updated_at)))
            .unwrap_or((0, 0, None));

        Ok(MemberSummary {
            member,
            conduct: MemberConduct { points, max_points, log: conduct_log },
            infractions: MemberInfractions { total: infractions_total, recent: infractions_recent },
            moderation: MemberModeration { total_warns, total_mutes, total_bans, actions: mod_actions },
            stats: MemberStats { message_count, voice_seconds, last_active },
        })
    }

    async fn sync_members(&self, cmd: SyncMembersCommand) -> Result<u64, DomainError> {
        self.member_repo.upsert_many(&cmd.members).await
    }

    async fn register_member(&self, cmd: RegisterMemberCommand) -> Result<(), DomainError> {
        self.member_repo.upsert(&cmd.member).await
    }

    async fn remove_member(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        self.member_repo.delete(guild_id, user_id).await
    }

    async fn update_member(&self, cmd: UpdateMemberCommand) -> Result<(), DomainError> {
        let mut member = self.get_member(&cmd.guild_id, &cmd.user_id).await?;
        if let Some(username) = cmd.username { member.username = username; }
        if let Some(display_name) = cmd.display_name { member.display_name = Some(display_name); }
        if let Some(avatar) = cmd.avatar { member.avatar = Some(avatar); }
        if let Some(roles) = cmd.roles { member.roles = roles; }
        self.member_repo.upsert(&member).await
    }
}

#[cfg(test)]
#[path = "tests/manage_members.rs"]
mod tests;
