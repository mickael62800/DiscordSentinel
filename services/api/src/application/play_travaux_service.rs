//! Service `PlayTravauxUseCase` (Phase 2 #2 audit).

use std::sync::Arc;

use async_trait::async_trait;
use rand::Rng;

use crate::domain::entities::{
    fail_flavor_at, success_flavor_at, task_at, TRAVAUX_COINS_MAX, TRAVAUX_COINS_MIN,
    TRAVAUX_COOLDOWN_KEY, TRAVAUX_COOLDOWN_SECS, TRAVAUX_FAIL_FLAVORS, TRAVAUX_SUCCESS_FLAVORS,
    TRAVAUX_SUCCESS_PCT, TRAVAUX_TASKS, TRAVAUX_XP_PER_TASK,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::play_travaux::{
    PlayTravauxCommand, PlayTravauxUseCase, TravauxResolution,
};
use crate::ports::inbound::ManageWalletUseCase;
use crate::ports::outbound::{
    CoudeHeistRepository, CoudePlayerRepository, CoudeSocialRepository,
};

pub struct PlayTravauxService {
    heist_repo: Arc<dyn CoudeHeistRepository>,
    player_repo: Arc<dyn CoudePlayerRepository>,
    wallet_uc: Arc<dyn ManageWalletUseCase>,
    social_repo: Arc<dyn CoudeSocialRepository>,
}

impl PlayTravauxService {
    pub fn new(
        heist_repo: Arc<dyn CoudeHeistRepository>,
        player_repo: Arc<dyn CoudePlayerRepository>,
        wallet_uc: Arc<dyn ManageWalletUseCase>,
        social_repo: Arc<dyn CoudeSocialRepository>,
    ) -> Self {
        Self { heist_repo, player_repo, wallet_uc, social_repo }
    }

    /// Roll des 3 valeurs RNG : index tache, succes, montant + index flavor.
    /// `ThreadRng` n'etant pas Send, scope ferme avant tout await suivant.
    fn roll() -> (usize, bool, i64, usize) {
        let mut rng = rand::thread_rng();
        let task_idx = rng.gen_range(0..TRAVAUX_TASKS.len());
        let success = rng.gen_bool(TRAVAUX_SUCCESS_PCT);
        let coins = if success {
            rng.gen_range(TRAVAUX_COINS_MIN..=TRAVAUX_COINS_MAX)
        } else {
            0
        };
        let flavor_idx = if success {
            rng.gen_range(0..TRAVAUX_SUCCESS_FLAVORS.len())
        } else {
            rng.gen_range(0..TRAVAUX_FAIL_FLAVORS.len())
        };
        (task_idx, success, coins, flavor_idx)
    }
}

#[async_trait]
impl PlayTravauxUseCase for PlayTravauxService {
    async fn play(&self, cmd: PlayTravauxCommand) -> Result<TravauxResolution, DomainError> {
        let PlayTravauxCommand { guild_id, user_id, username } = cmd;

        // 1. Prison check : on lit l'etat brut, on filtre `released_at > now`
        //    nous-meme (la repo ne filtre pas).
        let in_prison = match self.heist_repo.get_prison(&guild_id, &user_id).await? {
            Some(state) => state.is_active(),
            None => false,
        };
        if !in_prison {
            return Err(DomainError::Forbidden(
                "Tu n es pas en prison. /travaux est reserve aux detenus apres un braquage rate."
                    .into(),
            ));
        }

        // 2. Cooldown 2h.
        if let Some(expires_at) = self
            .social_repo
            .get_cooldown(&guild_id, &user_id, TRAVAUX_COOLDOWN_KEY)
            .await?
        {
            return Err(DomainError::RateLimited(format!(
                "Tu dois encore te reposer (jusqu'a {expires_at})."
            )));
        }

        // 3. Roll RNG (scope ferme avant les await suivants).
        let (task_idx, success, coins_gain, flavor_idx) = Self::roll();
        let task = task_at(task_idx);
        let flavor = if success {
            success_flavor_at(flavor_idx)
        } else {
            fail_flavor_at(flavor_idx)
        };

        // 4. Credit + XP si succes (best-effort, on continue meme si echec
        //    pour poser le cooldown — sinon le user retente en boucle).
        if success && coins_gain > 0 {
            // get_or_create avant credit (le wallet doit exister).
            let _ = self
                .player_repo
                .get_or_create(&guild_id, &user_id, &username)
                .await?;
            if let Err(e) = self
                .wallet_uc
                .credit(
                    &guild_id,
                    &user_id,
                    coins_gain,
                    "travaux",
                    "Travaux de prison",
                )
                .await
            {
                tracing::warn!(error = %e, user_id, "Echec credit wallet travaux");
            }
            if let Err(e) = self
                .player_repo
                .add_xp(&guild_id, &user_id, TRAVAUX_XP_PER_TASK)
                .await
            {
                tracing::warn!(error = %e, user_id, "Echec add_xp travaux");
            }
        }

        // 5. Cooldown 2h (meme en cas d echec — pas de spam).
        self.social_repo
            .set_cooldown(
                &guild_id,
                &user_id,
                TRAVAUX_COOLDOWN_KEY,
                TRAVAUX_COOLDOWN_SECS,
            )
            .await?;

        Ok(TravauxResolution {
            task_key: task.key,
            task_label: task.label,
            task_description: task.description,
            success,
            flavor,
            coins_gain,
            xp_gain: if success { TRAVAUX_XP_PER_TASK } else { 0 },
        })
    }
}

#[cfg(test)]
#[path = "tests/play_travaux.rs"]
mod tests;
