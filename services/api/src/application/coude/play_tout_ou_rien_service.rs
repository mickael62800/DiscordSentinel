//! Service `PlayToutOuRienUseCase` (Phase 2 #1 audit).
//!
//! Centralise cote API la decision RNG + persistance + cooldown +
//! memorial pour la commande `/tout-ou-rien`. Le bot ne fait plus
//! qu'animer + afficher le verdict.

use std::sync::Arc;

use async_trait::async_trait;
use rand::Rng;

use crate::domain::entities::coude::tout_ou_rien::coin_delta as coin_delta;
use crate::domain::entities::coude::tout_ou_rien::resolve_outcome as resolve_outcome;
use crate::domain::entities::coude::tout_ou_rien_log::ToutOuRienLogOutcome;
use crate::domain::entities::coude::tout_ou_rien::ToutOuRienOutcome;
use crate::domain::entities::coude::tout_ou_rien::TOUT_OU_RIEN_COOLDOWN_KEY;
use crate::domain::entities::coude::tout_ou_rien::TOUT_OU_RIEN_COOLDOWN_SECS;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::play_tout_ou_rien::PlayToutOuRienCommand;
use crate::ports::inbound::coude::play_tout_ou_rien::PlayToutOuRienUseCase;
use crate::ports::inbound::coude::play_tout_ou_rien::ToutOuRienResolution;
use crate::ports::inbound::coude::play_tout_ou_rien::MIN_BALANCE_FOR_PLAY;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::outbound::coude::player_repository::PlayerRepository;
use crate::ports::outbound::coude::social_repository::SocialRepository;
use crate::ports::outbound::coude::tout_ou_rien_repository::ToutOuRienRepository;
pub struct PlayToutOuRienService {
    player_repo: Arc<dyn PlayerRepository>,
    wallet_uc: Arc<dyn ManageWalletUseCase>,
    social_repo: Arc<dyn SocialRepository>,
    log_repo: Arc<dyn ToutOuRienRepository>,
}

impl PlayToutOuRienService {
    pub fn new(
        player_repo: Arc<dyn PlayerRepository>,
        wallet_uc: Arc<dyn ManageWalletUseCase>,
        social_repo: Arc<dyn SocialRepository>,
        log_repo: Arc<dyn ToutOuRienRepository>,
    ) -> Self {
        Self { player_repo, wallet_uc, social_repo, log_repo }
    }

    /// Tirage uniforme dans [0, 1). Isole dans une fonction pour faciliter
    /// le scoping du `ThreadRng` (non-Send) avant les await suivants.
    fn roll() -> f64 {
        let mut rng = rand::thread_rng();
        rng.gen_range(0.0..1.0)
    }
}

#[async_trait]
impl PlayToutOuRienUseCase for PlayToutOuRienService {
    async fn play(
        &self,
        cmd: PlayToutOuRienCommand,
    ) -> Result<ToutOuRienResolution, DomainError> {
        let PlayToutOuRienCommand { guild_id, user_id, username } = cmd;

        // 1. Cooldown weekly. Si actif -> RateLimited (mappe 429).
        if let Some(expires_at) = self
            .social_repo
            .get_cooldown(&guild_id, &user_id, TOUT_OU_RIEN_COOLDOWN_KEY)
            .await?
        {
            return Err(DomainError::RateLimited(format!(
                "Tout-ou-rien deja joue cette semaine (jusqu'a {expires_at})"
            )));
        }

        // 2. Lecture solde.
        let player = self
            .player_repo
            .get_or_create(&guild_id, &user_id, &username)
            .await?;
        if player.coins < MIN_BALANCE_FOR_PLAY {
            return Err(DomainError::ValidationError(format!(
                "Solde insuffisant : il te faut au moins {} coins (tu en as {}).",
                MIN_BALANCE_FOR_PLAY, player.coins
            )));
        }
        let initial_coins = player.coins;

        // 3. Tirage RNG (scope ferme avant tout await).
        let outcome = resolve_outcome(Self::roll());

        // 4. Delta domain pur (Win = +balance, Lose = -80%).
        let delta = coin_delta(initial_coins, outcome);

        // 5. Mutation wallet via use case unifie (faillite/jackpot detectes).
        match outcome {
            ToutOuRienOutcome::Win if delta > 0 => {
                self.wallet_uc
                    .credit(
                        &guild_id,
                        &user_id,
                        delta,
                        "tout_ou_rien_win",
                        "TOUT-OU-RIEN — victoire",
                    )
                    .await?;
            }
            ToutOuRienOutcome::Lose if delta < 0 => {
                self.wallet_uc
                    .debit(
                        &guild_id,
                        &user_id,
                        -delta,
                        "tout_ou_rien_lose",
                        "TOUT-OU-RIEN — defaite",
                    )
                    .await?;
            }
            _ => {}
        }

        // 6. Cooldown apres mutation reussie : double-clic genere un seul payout.
        self.social_repo
            .set_cooldown(
                &guild_id,
                &user_id,
                TOUT_OU_RIEN_COOLDOWN_KEY,
                TOUT_OU_RIEN_COOLDOWN_SECS,
            )
            .await?;

        // 7. Memorial des clodos (audit). Best-effort : on log mais on
        //    n'echoue pas la commande si l'insert log foire.
        let log_outcome = match outcome {
            ToutOuRienOutcome::Win => ToutOuRienLogOutcome::Won,
            ToutOuRienOutcome::Lose => ToutOuRienLogOutcome::Lost,
        };
        if let Err(e) = self
            .log_repo
            .record(&guild_id, &user_id, &username, initial_coins, log_outcome, delta)
            .await
        {
            tracing::warn!(error = %e, user_id = %user_id, "Echec record tout-ou-rien log");
        }

        let final_balance = (initial_coins + delta).max(0);
        Ok(ToutOuRienResolution {
            initial_coins,
            outcome,
            delta,
            final_balance,
        })
    }
}

#[cfg(test)]
#[path = "tests/play_tout_ou_rien.rs"]
mod tests;
