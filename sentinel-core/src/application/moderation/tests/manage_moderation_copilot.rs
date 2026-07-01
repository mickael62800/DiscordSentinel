//! Tests du ManageModerationCopilotService avec repos MOCKES.
//! Verifie l'assemblage du contexte, la reutilisation du ladder strikes et
//! l'exclusion des reviews `voting` (garantie par le contrat du mock).

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;

use crate::application::moderation::manage_moderation_copilot_service::ManageModerationCopilotService;
use crate::domain::entities::moderation::action::strikes::StrikeConfig;
use crate::domain::entities::moderation::action::strikes::StrikeThreshold;
use crate::domain::entities::moderation::action::strikes::UserStrike;
use crate::domain::entities::moderation::copilot::PrecedentDistribution;
use crate::domain::entities::moderation::copilot::SuggestionBasis;
use crate::domain::entities::moderation::review::automod::AppliedAction;
use crate::domain::errors::DomainError;
use crate::ports::inbound::moderation::manage_strikes::AddStrikeCommand;
use crate::ports::inbound::moderation::manage_strikes::ManageStrikesUseCase;
use crate::ports::inbound::moderation::manage_strikes::SaveStrikeConfigCommand;
use crate::ports::inbound::moderation::moderation_copilot::ModerationCopilotUseCase;
use crate::ports::outbound::moderation::moderation_copilot_repository::ModerationCopilotRepository;

// ── Mock strikes use case ─────────────────────────────────────────────

struct MockStrikesUc {
    active: u32,
    thresholds: Vec<StrikeThreshold>,
}

#[async_trait]
impl ManageStrikesUseCase for MockStrikesUc {
    async fn add_strike(
        &self,
        _cmd: AddStrikeCommand,
    ) -> Result<crate::domain::entities::moderation::action::strikes::StrikeResult, DomainError>
    {
        Err(DomainError::NotImplemented("add_strike".into()))
    }
    async fn get_active_strikes(
        &self,
        guild_id: &str,
        _user_id: &str,
    ) -> Result<Vec<UserStrike>, DomainError> {
        let now = Utc::now();
        Ok((0..self.active)
            .map(|_| UserStrike {
                id: uuid::Uuid::new_v4(),
                guild_id: guild_id.into(),
                user_id: "u".into(),
                reason: String::new(),
                source: "test".into(),
                infraction_id: None,
                expires_at: None,
                created_at: now,
            })
            .collect())
    }
    async fn reset_strikes(&self, _g: &str, _u: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_config(&self, guild_id: &str) -> Result<StrikeConfig, DomainError> {
        let mut c = StrikeConfig::default_for_guild(guild_id);
        c.thresholds = self.thresholds.clone();
        Ok(c)
    }
    async fn save_config(
        &self,
        _cmd: SaveStrikeConfigCommand,
    ) -> Result<StrikeConfig, DomainError> {
        Err(DomainError::NotImplemented("save_config".into()))
    }
}

// ── Mock copilot repository ───────────────────────────────────────────

struct MockCopilotRepo {
    dominant: Option<String>,
    // Precedents renvoyes pour la categorie dominante (deja hors voting).
    distribution: PrecedentDistribution,
    // Trace les appels a aggregate pour verifier le contrat anti-ancrage.
    aggregate_calls: Mutex<Vec<String>>,
}

#[async_trait]
impl ModerationCopilotRepository for MockCopilotRepo {
    async fn count_sanctions_by_type(
        &self,
        _g: &str,
        _u: &str,
        _since: DateTime<Utc>,
    ) -> Result<Vec<(String, u32)>, DomainError> {
        Ok(vec![("warn".into(), 2), ("mute".into(), 1)])
    }
    async fn last_sanction_at(
        &self,
        _g: &str,
        _u: &str,
    ) -> Result<Option<DateTime<Utc>>, DomainError> {
        Ok(Some(Utc::now()))
    }
    async fn count_open_reviews(&self, _g: &str, _u: &str) -> Result<u32, DomainError> {
        Ok(3)
    }
    async fn dominant_flag_category(
        &self,
        _g: &str,
        _u: &str,
        _since: DateTime<Utc>,
    ) -> Result<Option<String>, DomainError> {
        Ok(self.dominant.clone())
    }
    async fn aggregate_decided_by_flag(
        &self,
        _g: &str,
        flag_category: &str,
        _since: DateTime<Utc>,
    ) -> Result<PrecedentDistribution, DomainError> {
        self.aggregate_calls
            .lock()
            .unwrap()
            .push(flag_category.to_string());
        Ok(self.distribution.clone())
    }
}

fn threshold(strikes: u32, action: &str) -> StrikeThreshold {
    StrikeThreshold {
        strikes,
        action: action.to_string(),
        duration: None,
    }
}

#[tokio::test]
async fn assemble_contexte_avec_jurisprudence() {
    let strikes = Arc::new(MockStrikesUc {
        active: 2,
        thresholds: vec![threshold(1, "warn"), threshold(3, "ban")],
    });
    let repo = Arc::new(MockCopilotRepo {
        dominant: Some("spam".into()),
        distribution: PrecedentDistribution {
            flag_category: "spam".into(),
            counts_by_action: vec![("mute".into(), 4)],
            total: 4,
        },
        aggregate_calls: Mutex::new(vec![]),
    });
    let svc = ManageModerationCopilotService::new(strikes, repo.clone());

    let ctx = svc
        .get_member_context("123456789012345678", "u", 30, 3)
        .await
        .unwrap();

    assert_eq!(ctx.active_strikes, 2);
    assert_eq!(ctx.open_reviews, 3);
    assert!(ctx.last_sanction_at.is_some());
    assert_eq!(ctx.sanctions_by_type.len(), 2);
    assert_eq!(ctx.precedents.total, 4);
    // Precedents suffisants (4 >= 3) + escalade (next=3 -> ban) => Both, action modale mute.
    assert_eq!(ctx.suggestion.basis, SuggestionBasis::Both);
    assert_eq!(ctx.suggestion.action, Some(AppliedAction::Mute));
    // Contrat anti-ancrage : l'agregation a bien ete demandee sur la categorie
    // dominante (l'impl SQL exclura status='voting').
    assert_eq!(repo.aggregate_calls.lock().unwrap().as_slice(), &["spam"]);
}

#[tokio::test]
async fn sans_categorie_dominante_pas_d_agregation() {
    let strikes = Arc::new(MockStrikesUc {
        active: 0,
        thresholds: vec![threshold(1, "warn")],
    });
    let repo = Arc::new(MockCopilotRepo {
        dominant: None,
        distribution: PrecedentDistribution::default(),
        aggregate_calls: Mutex::new(vec![]),
    });
    let svc = ManageModerationCopilotService::new(strikes, repo.clone());

    let ctx = svc
        .get_member_context("123456789012345678", "u", 30, 2)
        .await
        .unwrap();

    // Pas de categorie -> pas d'appel a aggregate, precedents vides.
    assert!(repo.aggregate_calls.lock().unwrap().is_empty());
    assert_eq!(ctx.precedents.total, 0);
    // Escalade seule : next=1 -> warn.
    assert_eq!(ctx.suggestion.basis, SuggestionBasis::Escalation);
    assert_eq!(ctx.suggestion.action, Some(AppliedAction::Warn));
}

#[tokio::test]
async fn clamp_lookback_et_min_precedents() {
    let strikes = Arc::new(MockStrikesUc {
        active: 0,
        thresholds: vec![],
    });
    let repo = Arc::new(MockCopilotRepo {
        dominant: Some("spam".into()),
        distribution: PrecedentDistribution {
            flag_category: "spam".into(),
            counts_by_action: vec![("warn".into(), 1)],
            total: 1,
        },
        aggregate_calls: Mutex::new(vec![]),
    });
    let svc = ManageModerationCopilotService::new(strikes, repo);

    // lookback_days=0 clamp a 1 (pas de panic), min_precedents=0 clamp a 1.
    let ctx = svc
        .get_member_context("123456789012345678", "u", 0, 0)
        .await
        .unwrap();
    // 1 precedent >= min clamp(1) => Precedent.
    assert_eq!(ctx.suggestion.basis, SuggestionBasis::Precedent);
}
