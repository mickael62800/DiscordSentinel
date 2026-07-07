//! Service Community : decisions d'eligibilite (role + parrainage). Lit la
//! config serveur via le port sortant `BotConfigRepository`, puis applique les
//! regles PURES du domaine (`domain::entities::community::eligibility`). Aucune
//! dependance Discord : le bot fournit les donnees (roles, dates de join).

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::community::eligibility::{
    check_prerequisites, days_since, evaluate_sponsorship, parse_prerequisites, EligibilityDecision,
};
use crate::domain::entities::system::bot_config::BotGuildConfig;
use crate::domain::errors::DomainError;
use crate::ports::inbound::community::check_eligibility::{
    CheckEligibilityUseCase, CheckRoleEligibilityCommand, ValidateSponsorshipCommand,
};
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

/// Nom du module de config (aligne sur `MODULE_BOT_NAME` cote bot).
const COMMUNITY_BOT: &str = "community-bot";

pub struct CheckEligibilityService {
    config: Arc<dyn BotConfigRepository>,
}

impl CheckEligibilityService {
    pub fn new(config: Arc<dyn BotConfigRepository>) -> Self {
        Self { config }
    }
}

fn cfg_str<'a>(entries: &'a [BotGuildConfig], key: &str) -> Option<&'a str> {
    entries
        .iter()
        .find(|e| e.config_key == key)
        .map(|e| e.config_value.as_str())
}

fn cfg_u64(entries: &[BotGuildConfig], key: &str, default: u64) -> u64 {
    cfg_str(entries, key)
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

/// Horodatage courant (secondes unix). Isole pour la lisibilite/tests.
fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

#[async_trait]
impl CheckEligibilityUseCase for CheckEligibilityService {
    async fn check_role_eligibility(
        &self,
        cmd: CheckRoleEligibilityCommand,
    ) -> Result<EligibilityDecision, DomainError> {
        let cfg = self
            .config
            .get_config(&cmd.guild_id, COMMUNITY_BOT)
            .await
            .unwrap_or_default();

        let raw = cfg_str(&cfg, "role_prerequisites").unwrap_or("");
        let prereqs = parse_prerequisites(raw);

        // `None` => 0 jour (reproduit le `unwrap_or(0)` du bot pour les prereqs).
        let joined_days = cmd
            .joined_at_unix
            .map(|j| days_since(now_unix(), j))
            .unwrap_or(0);

        Ok(check_prerequisites(
            &prereqs,
            cmd.role_id,
            &cmd.user_roles,
            joined_days,
        ))
    }

    async fn validate_sponsorship(
        &self,
        cmd: ValidateSponsorshipCommand,
    ) -> Result<EligibilityDecision, DomainError> {
        let cfg = self
            .config
            .get_config(&cmd.guild_id, COMMUNITY_BOT)
            .await
            .unwrap_or_default();

        let min_parrain_days = cfg_u64(&cfg, "sponsor_min_parrain_days", 7);
        let max_filleul_days = cfg_u64(&cfg, "sponsor_max_filleul_days", 30);

        let now = now_unix();
        // Parrain absent => 0 jour (echoue le min). Filleul absent => u64::MAX
        // (echoue le max). Reproduit exactement les defauts du bot.
        let sponsor_days = cmd
            .sponsor_joined_at_unix
            .map(|j| days_since(now, j))
            .unwrap_or(0);
        let sponsored_days = cmd
            .sponsored_joined_at_unix
            .map(|j| days_since(now, j))
            .unwrap_or(u64::MAX);

        Ok(evaluate_sponsorship(
            cmd.sponsor_id,
            cmd.sponsored_id,
            sponsor_days,
            sponsored_days,
            min_parrain_days,
            max_filleul_days,
        ))
    }
}
