//! Use case Bump : cooldown atomique, recompense graduee de la semaine, credit
//! du wallet et evaluation du seuil VIP. Toute la regle metier vit ici (le
//! handler HTTP ne fait que parser/mapper), le SQL vit dans `BumpRepository`.

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::community::bump::{
    bump_reward, sanitize_provider, BumpReward, DueReminder,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::community::manage_bump::{ManageBumpUseCase, RecordBumpCommand};
use crate::ports::outbound::community::bump_repository::BumpRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

/// Plafond dur d'une recompense de bump (anti-overflow + anti-truncation i32 du
/// journal, et anti-config abusive).
const MAX_BUMP_REWARD: i64 = 100_000_000;

pub struct ManageBumpService {
    config: Arc<dyn BotConfigRepository>,
    repo: Arc<dyn BumpRepository>,
}

impl ManageBumpService {
    pub fn new(config: Arc<dyn BotConfigRepository>, repo: Arc<dyn BumpRepository>) -> Self {
        Self { config, repo }
    }
}

use crate::domain::entities::system::bot_config::{cfg_bool, cfg_i64, cfg_str};

#[async_trait]
impl ManageBumpUseCase for ManageBumpService {
    async fn record_bump(&self, cmd: RecordBumpCommand) -> Result<BumpReward, DomainError> {
        let provider = sanitize_provider(&cmd.provider);
        let cfg = self
            .config
            .get_config(&cmd.guild_id, "bump-bot")
            .await
            .unwrap_or_default();

        // Master switch + interrupteur par provider (defaut on).
        let provider_enabled = cfg_bool(&cfg, &format!("{provider}_enabled"), true);
        if !cfg_bool(&cfg, "enabled", false) || !provider_enabled {
            return Ok(BumpReward::none());
        }

        let base = cfg_i64(&cfg, "bump_reward_base", 100).clamp(0, MAX_BUMP_REWARD);
        let step = cfg_i64(&cfg, "bump_reward_step", 50).clamp(0, MAX_BUMP_REWARD);
        let max = cfg_i64(&cfg, "bump_reward_max", 500).clamp(0, MAX_BUMP_REWARD);

        // Cooldown par provider, avec repli retrocompat.
        let cooldown_default = match provider.as_str() {
            "discordl" => 240,
            "discordl_vote" => 720,
            "frenchgg" => 120,
            "discadia" => 1440,
            "topgg" => 720,
            "spacebump" => 360,
            _ => cfg_i64(&cfg, "bump_cooldown_minutes", 120),
        };
        let cooldown = cfg_i64(&cfg, &format!("{provider}_cooldown_minutes"), cooldown_default)
            .clamp(1, 1440);
        let reminder_enabled = cfg_bool(&cfg, "bump_reminder_enabled", true);
        let channel = {
            let c = cfg_str(&cfg, "bump_channel_id").unwrap_or("").trim().to_string();
            if c.is_empty() {
                cmd.channel_id.clone()
            } else {
                c
            }
        };

        // Garde de cooldown ATOMIQUE (CAS) : ne recompense que si le creneau est
        // libre (dernier bump du (guild, provider) > cooldown).
        let slot_won = self
            .repo
            .try_claim_slot(&cmd.guild_id, &provider, &channel, cooldown, reminder_enabled)
            .await?;
        if !slot_won {
            return Ok(BumpReward::none());
        }

        // Nieme bump de la semaine + recompense graduee.
        let week_count = self.repo.weekly_count(&cmd.guild_id, &cmd.user_id).await?;
        let n = week_count + 1;
        let reward = bump_reward(n, base, step, max);

        self.repo
            .record_event(
                &cmd.guild_id,
                &cmd.user_id,
                &cmd.username,
                reward,
                n,
                &provider,
            )
            .await?;

        // L'economie de coins partagee a ete retiree avec les jeux : plus de
        // credit de wallet. La recompense reste journalisee (compteur bump).
        let new_balance = None;

        // Role VIP : seuil de bumps CUMULES (all-time).
        let mut vip_role_id: Option<String> = None;
        let mut vip_just_unlocked = false;
        if cfg_bool(&cfg, "vip_enabled", false) {
            let vip_role = cfg_str(&cfg, "vip_role_id").unwrap_or("").trim().to_string();
            let vip_threshold = cfg_i64(&cfg, "vip_bump_threshold", 10).max(1);
            if !vip_role.is_empty() {
                let total_bumps = self
                    .repo
                    .total_count(&cmd.guild_id, &cmd.user_id)
                    .await
                    .unwrap_or(0);
                if total_bumps >= vip_threshold {
                    vip_role_id = Some(vip_role);
                    // Le total inclut le bump qu'on vient d'inserer : franchissement
                    // exact quand le total atteint le seuil.
                    vip_just_unlocked = total_bumps == vip_threshold;
                }
            }
        }

        Ok(BumpReward {
            rewarded: true,
            reward,
            weekly_count: n,
            new_balance,
            vip_role_id,
            vip_just_unlocked,
        })
    }

    async fn due_reminders(&self) -> Result<Vec<DueReminder>, DomainError> {
        self.repo.due_reminders().await
    }

    async fn mark_reminder_sent(
        &self,
        guild_id: &str,
        provider: Option<String>,
    ) -> Result<(), DomainError> {
        let normalized = provider.map(|p| sanitize_provider(&p));
        self.repo
            .mark_reminder_sent(guild_id, normalized.as_deref())
            .await
    }

    async fn guild_status(
        &self,
        guild_id: &str,
    ) -> Result<Vec<crate::domain::entities::community::bump::BumpState>, DomainError> {
        self.repo.guild_states(guild_id).await
    }
}
