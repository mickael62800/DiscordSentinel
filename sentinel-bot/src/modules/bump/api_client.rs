//! Client gRPC du module bump (`BumpService`). Le module n'a besoin que du
//! client gRPC (la config guild passe par `guild_config_or_default` + les
//! helpers statiques `BaseApiClient::config_*`, sans instance HTTP).

use std::sync::Arc;

use crate::shared::grpc_client::{grpc_err_to_string, SentinelGrpcClient};
use sentinel_proto::bump::v1 as proto;

/// Recompense d'un bump (sous-ensemble consomme par le bot).
#[derive(Debug, Clone, Default)]
pub struct BumpReward {
    pub rewarded: bool,
    pub reward: i64,
    pub weekly_count: i64,
    pub vip_role_id: Option<String>,
    pub vip_just_unlocked: bool,
}

/// Etat d'un provider pour la carte de statut.
pub struct BumpStatusInfo {
    pub provider: String,
    pub ready_at: chrono::DateTime<chrono::Utc>,
}

/// Un rappel du : (guild, salon, plateforme).
pub struct DueReminder {
    pub guild_id: String,
    pub channel_id: String,
    pub provider: String,
}

pub struct BumpApi {
    grpc: Arc<SentinelGrpcClient>,
}

impl BumpApi {
    pub fn new(grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { grpc }
    }

    /// Enregistre un bump constate (cooldown + recompense + VIP cote serveur).
    pub async fn record_bump(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        channel_id: &str,
        provider: &str,
    ) -> Result<BumpReward, String> {
        let req = proto::RecordBumpRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            channel_id: channel_id.to_string(),
            provider: provider.to_string(),
        };
        let r = crate::grpc_call!(self.grpc, bump, record_bump, req)?;
        Ok(BumpReward {
            rewarded: r.rewarded,
            reward: r.reward,
            weekly_count: r.weekly_count,
            vip_role_id: r.vip_role_id,
            vip_just_unlocked: r.vip_just_unlocked,
        })
    }

    /// Etats bump d'une guild (carte de statut). Les entrees au `ready_at`
    /// illisible sont ignorees.
    pub async fn guild_status(&self, guild_id: &str) -> Result<Vec<BumpStatusInfo>, String> {
        let req = proto::GuildStatusRequest {
            guild_id: guild_id.to_string(),
        };
        let list = crate::grpc_call!(self.grpc, bump, guild_status, req)?;
        Ok(list
            .statuses
            .into_iter()
            .filter_map(|s| {
                let ready_at = chrono::DateTime::parse_from_rfc3339(&s.ready_at)
                    .ok()?
                    .with_timezone(&chrono::Utc);
                Some(BumpStatusInfo {
                    provider: s.provider,
                    ready_at,
                })
            })
            .collect())
    }

    /// Rappels dus (cooldown ecoule, non envoye).
    pub async fn due_reminders(&self) -> Result<Vec<DueReminder>, String> {
        let req = proto::DueRemindersRequest {};
        let list = crate::grpc_call!(self.grpc, bump, due_reminders, req)?;
        Ok(list
            .reminders
            .into_iter()
            .map(|r| DueReminder {
                guild_id: r.guild_id,
                channel_id: r.channel_id,
                provider: r.provider,
            })
            .collect())
    }

    /// Marque le rappel envoye pour un provider.
    pub async fn mark_reminder_sent(&self, guild_id: &str, provider: &str) -> Result<(), String> {
        let req = proto::MarkReminderSentRequest {
            guild_id: guild_id.to_string(),
            provider: Some(provider.to_string()),
        };
        crate::grpc_call!(@unit self.grpc, bump, mark_reminder_sent, req)
    }
}
