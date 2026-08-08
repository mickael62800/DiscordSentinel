//! Implementation gRPC du `BumpService`.
//!
//! Wrappe `ManageBumpUseCase`. Remplace les endpoints HTTP `/api/bump/...`
//! appeles par bump-bot (enregistrement d'un bump, rappels, carte de statut).

use std::sync::Arc;

use sentinel_proto::bump::v1 as proto;
use sentinel_proto::bump::v1::bump_service_server::BumpService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use sentinel_core::domain::entities::community::bump::BumpReward;
use sentinel_core::domain::entities::community::bump::BumpState;
use sentinel_core::domain::entities::community::bump::DueReminder;
use sentinel_core::ports::inbound::community::manage_bump::ManageBumpUseCase;
use sentinel_core::ports::inbound::community::manage_bump::RecordBumpCommand;

pub struct BumpGrpc {
    pub uc: Arc<dyn ManageBumpUseCase>,
}

#[tonic::async_trait]
impl BumpService for BumpGrpc {
    async fn record_bump(
        &self,
        request: Request<proto::RecordBumpRequest>,
    ) -> Result<Response<proto::BumpReward>, Status> {
        let req = request.into_inner();
        let reward = self
            .uc
            .record_bump(RecordBumpCommand {
                guild_id: req.guild_id,
                user_id: req.user_id,
                username: req.username,
                channel_id: req.channel_id,
                provider: req.provider,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(reward_to_proto(reward)))
    }

    async fn due_reminders(
        &self,
        _request: Request<proto::DueRemindersRequest>,
    ) -> Result<Response<proto::DueReminderList>, Status> {
        let rows = self.uc.due_reminders().await.map_err(domain_to_status)?;
        Ok(Response::new(proto::DueReminderList {
            reminders: rows.into_iter().map(due_reminder_to_proto).collect(),
        }))
    }

    async fn mark_reminder_sent(
        &self,
        request: Request<proto::MarkReminderSentRequest>,
    ) -> Result<Response<proto::MarkReminderSentResponse>, Status> {
        let req = request.into_inner();
        self.uc
            .mark_reminder_sent(&req.guild_id, req.provider)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::MarkReminderSentResponse {}))
    }

    async fn guild_status(
        &self,
        request: Request<proto::GuildStatusRequest>,
    ) -> Result<Response<proto::BumpStatusList>, Status> {
        let states = self
            .uc
            .guild_status(&request.into_inner().guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::BumpStatusList {
            statuses: states.into_iter().map(state_to_proto).collect(),
        }))
    }
}

fn reward_to_proto(r: BumpReward) -> proto::BumpReward {
    proto::BumpReward {
        rewarded: r.rewarded,
        reward: r.reward,
        weekly_count: r.weekly_count,
        new_balance: r.new_balance,
        vip_role_id: r.vip_role_id,
        vip_just_unlocked: r.vip_just_unlocked,
    }
}

fn due_reminder_to_proto(r: DueReminder) -> proto::DueReminder {
    proto::DueReminder {
        guild_id: r.guild_id,
        channel_id: r.channel_id,
        provider: r.provider,
    }
}

fn state_to_proto(s: BumpState) -> proto::BumpStatus {
    // `ready_at` calcule cote serveur (comme le handler HTTP), pour que le bot
    // n'ait pas a connaitre la regle de cooldown.
    let ready_at = s.last_bump_at + chrono::Duration::minutes(s.cooldown_minutes.max(0));
    proto::BumpStatus {
        provider: s.provider,
        channel_id: s.channel_id,
        last_bump_at: s.last_bump_at.to_rfc3339(),
        cooldown_minutes: s.cooldown_minutes,
        ready_at: ready_at.to_rfc3339(),
    }
}

#[cfg(test)]
#[path = "tests/bump.rs"]
mod tests;
