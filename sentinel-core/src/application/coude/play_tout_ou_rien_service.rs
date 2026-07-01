//! Service `PlayToutOuRienUseCase` (Phase 2 #1 audit).
//!
//! Centralise cote API la decision RNG + persistance + cooldown +
//! memorial pour la commande `/tout-ou-rien`. Le bot ne fait plus
//! qu'animer + afficher le verdict.

use std::sync::Arc;

use async_trait::async_trait;
use rand::Rng;

use crate::domain::entities::coude::tout_ou_rien::coin_delta;
use crate::domain::entities::coude::tout_ou_rien::resolve_outcome;
use crate::domain::entities::coude::tout_ou_rien::ToutOuRienOutcome;
use crate::domain::entities::coude::tout_ou_rien::TOUT_OU_RIEN_COOLDOWN_KEY;
use crate::domain::entities::coude::tout_ou_rien::TOUT_OU_RIEN_COOLDOWN_SECS;
use crate::domain::entities::coude::tout_ou_rien_log::ToutOuRienLogOutcome;
use crate::domain::errors::DomainError;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::coude::play_tout_ou_rien::PlayToutOuRienCommand;
use crate::ports::inbound::coude::play_tout_ou_rien::PlayToutOuRienUseCase;
use crate::ports::inbound::coude::play_tout_ou_rien::ToutOuRienResolution;
use crate::ports::inbound::coude::play_tout_ou_rien::MIN_BALANCE_FOR_PLAY;
use crate::ports::outbound::coude::player_repository::PlayerRepository;
use crate::ports::outbound::coude::social_repository::SocialRepository;
use crate::ports::outbound::coude::tout_ou_rien_repository::ToutOuRienRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
pub struct PlayToutOuRienService {
    player_repo: Arc<dyn PlayerRepository>,
    wallet_uc: Arc<dyn ManageWalletUseCase>,
    social_repo: Arc<dyn SocialRepository>,
    log_repo: Arc<dyn ToutOuRienRepository>,
    bot_config_repo: Option<Arc<dyn BotConfigRepository>>,
}

impl PlayToutOuRienService {
    pub fn new(
        player_repo: Arc<dyn PlayerRepository>,
        wallet_uc: Arc<dyn ManageWalletUseCase>,
        social_repo: Arc<dyn SocialRepository>,
        log_repo: Arc<dyn ToutOuRienRepository>,
    ) -> Self {
        Self {
            player_repo,
            wallet_uc,
            social_repo,
            log_repo,
            bot_config_repo: None,
        }
    }

    /// Branche le repo de config bot pour rendre les paramètres du
    /// tout-ou-rien (probabilité de gain, multiplicateur, % conservé en
    /// cas de défaite) réglables par serveur via `coude-bot`. Sans repo :
    /// valeurs par défaut historiques.
    pub fn with_bot_config_repo(mut self, repo: Arc<dyn BotConfigRepository>) -> Self {
        self.bot_config_repo = Some(repo);
        self
    }

    /// Charge la config ECONOMY de la guild (fallback default sans repo).
    async fn load_economy(
        &self,
        guild_id: &str,
    ) -> crate::domain::entities::coude::economy_config::CoudeEconomyConfig {
        match &self.bot_config_repo {
            Some(repo) => {
                crate::application::coude::guild_settings::load_economy_config(&**repo, guild_id)
                    .await
            }
            None => crate::domain::entities::coude::economy_config::CoudeEconomyConfig::default(),
        }
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
    async fn play(&self, cmd: PlayToutOuRienCommand) -> Result<ToutOuRienResolution, DomainError> {
        let PlayToutOuRienCommand {
            guild_id,
            user_id,
            username,
        } = cmd;

        // 1. Lecture solde EN PREMIER (read-only) : un joueur au solde
        //    insuffisant ne doit PAS etre verrouille pour la semaine.
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

        // 2. Claim atomique du cooldown weekly (fix TOCTOU). Pose le verrou
        //    AVANT toute mutation wallet : deux plays concurrents -> un seul
        //    gagne le claim, l'autre recoit RateLimited. Plus de double payout.
        let claimed = self
            .social_repo
            .try_claim_cooldown(
                &guild_id,
                &user_id,
                TOUT_OU_RIEN_COOLDOWN_KEY,
                TOUT_OU_RIEN_COOLDOWN_SECS,
            )
            .await?;
        if !claimed {
            return Err(DomainError::RateLimited(
                "Tout-ou-rien deja joue cette semaine".to_string(),
            ));
        }

        // Config ECONOMY réglable par serveur (probabilité/multiplicateur/
        // % conservé). Domaine PUR : passée en donnée aux fns.
        let econ = self.load_economy(&guild_id).await;

        // 3. Tirage RNG (scope ferme avant tout await).
        let outcome = resolve_outcome(Self::roll(), &econ);

        // 4. Delta domain pur (Win = +balance, Lose = -80% par défaut).
        let delta = coin_delta(initial_coins, outcome, &econ);

        // 5. Mutation wallet via use case unifie (faillite/jackpot detectes).
        //    En cas d'echec : on RELACHE le claim (best-effort) pour ne pas
        //    verrouiller le joueur une semaine sur un play qui n'a rien paye.
        let mutation = match outcome {
            ToutOuRienOutcome::Win if delta > 0 => self
                .wallet_uc
                .credit(
                    &guild_id,
                    &user_id,
                    delta,
                    "tout_ou_rien_win",
                    "TOUT-OU-RIEN — victoire",
                )
                .await
                .map(|_| ()),
            ToutOuRienOutcome::Lose if delta < 0 => self
                .wallet_uc
                .debit(
                    &guild_id,
                    &user_id,
                    -delta,
                    "tout_ou_rien_lose",
                    "TOUT-OU-RIEN — defaite",
                )
                .await
                .map(|_| ()),
            _ => Ok(()),
        };
        if let Err(e) = mutation {
            if let Err(clear_err) = self
                .social_repo
                .clear_cooldown(&guild_id, &user_id, TOUT_OU_RIEN_COOLDOWN_KEY)
                .await
            {
                tracing::warn!(
                    error = %clear_err,
                    user_id = %user_id,
                    "Echec liberation cooldown tout-ou-rien apres echec mutation wallet"
                );
            }
            return Err(e);
        }

        // 6. Memorial des clodos (audit). Best-effort : on log mais on
        //    n'echoue pas la commande si l'insert log foire.
        let log_outcome = match outcome {
            ToutOuRienOutcome::Win => ToutOuRienLogOutcome::Won,
            ToutOuRienOutcome::Lose => ToutOuRienLogOutcome::Lost,
        };
        if let Err(e) = self
            .log_repo
            .record(
                &guild_id,
                &user_id,
                &username,
                initial_coins,
                log_outcome,
                delta,
            )
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
