//! Service maledictions (cf. COUPE_AMELIORATIONS section 5.1).

use std::sync::Arc;

use async_trait::async_trait;
use rand::Rng;

use crate::domain::entities::{
    lift_cost, pick_curse_by_index, ActiveCurse, CurseKind,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_curses::{CastedCurse, ManageCoudeCursesUseCase};
use crate::ports::outbound::{CoudeCursesRepository, WalletRepository};

const CAST_SOURCE: &str = "curse_cast";
const LIFT_SOURCE: &str = "curse_lift";

pub struct ManageCoudeCursesService {
    curses_repo: Arc<dyn CoudeCursesRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
}

impl ManageCoudeCursesService {
    pub fn new(
        curses_repo: Arc<dyn CoudeCursesRepository>,
        wallet_repo: Arc<dyn WalletRepository>,
    ) -> Self {
        Self {
            curses_repo,
            wallet_repo,
        }
    }
}

#[async_trait]
impl ManageCoudeCursesUseCase for ManageCoudeCursesService {
    async fn cast(
        &self,
        guild_id: &str,
        source_id: &str,
        _source_username: &str,
        target_id: &str,
        kind: Option<CurseKind>,
    ) -> Result<CastedCurse, DomainError> {
        if source_id == target_id {
            return Err(DomainError::ValidationError(
                "Tu ne peux pas te maudire toi-meme.".into(),
            ));
        }

        // Verifie qu il n y a pas deja une malediction active sur la cible.
        if self
            .curses_repo
            .get_active_for_target(guild_id, target_id)
            .await?
            .is_some()
        {
            return Err(DomainError::Conflict(
                "Une malediction est deja active sur cette cible.".into(),
            ));
        }

        let chosen = match kind {
            Some(k) => k,
            None => {
                let idx: usize = rand::thread_rng().gen_range(0..CurseKind::ALL.len());
                pick_curse_by_index(idx)
            }
        };

        let cost = chosen.cost_coins();
        let duration = chosen.duration_hours();

        // Debit du wallet de l auteur — leve l erreur si solde insuffisant.
        self.wallet_repo
            .debit(
                guild_id,
                source_id,
                cost,
                CAST_SOURCE,
                &format!("{} sur {}", chosen.label(), target_id),
            )
            .await?;

        // Insertion. La contrainte unique partial sert de garde-fou en cas
        // de race entre le check + l insert.
        let id = self
            .curses_repo
            .cast(
                guild_id,
                target_id,
                source_id,
                chosen,
                duration,
            )
            .await
            .map_err(|e| {
                // Si la curse a echoue apres le debit, on ne re-credite pas
                // automatiquement — c est rare (race window minuscule), mais
                // c est tracable via wallet_transactions.
                e
            })?;

        Ok(CastedCurse {
            id,
            kind: chosen,
            cost_paid: cost,
        })
    }

    async fn get_active(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Option<ActiveCurse>, DomainError> {
        self.curses_repo
            .get_active_for_target(guild_id, target_id)
            .await
    }

    async fn lift_own(
        &self,
        guild_id: &str,
        target_id: &str,
        _target_username: &str,
    ) -> Result<ActiveCurse, DomainError> {
        let curse = self
            .curses_repo
            .get_active_for_target(guild_id, target_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Aucune malediction active.".into()))?;

        let cost = lift_cost(curse.kind);
        // La cible paye le cout, transfere integralement a l auteur initial.
        self.wallet_repo
            .transfer(
                guild_id,
                target_id,
                &curse.source_id,
                cost,
                LIFT_SOURCE,
                &format!("Levee malediction {}", curse.kind.label()),
            )
            .await?;

        self.curses_repo.lift(curse.id, target_id).await?;
        // Re-fetch pour avoir lifted_at/lifted_by a jour. Si la malediction
        // a expire entre temps, on retourne quand meme l ancien snapshot
        // marque comme leve.
        let updated = self
            .curses_repo
            .get_active_for_target(guild_id, target_id)
            .await?;
        // get_active filtre lifted_at IS NULL -> toujours None ici.
        // On recompose donc manuellement le snapshot leve.
        Ok(updated.unwrap_or(ActiveCurse {
            lifted_at: Some(chrono::Utc::now()),
            lifted_by: Some(target_id.to_string()),
            ..curse
        }))
    }
}

#[cfg(test)]
#[path = "tests/manage_coude_curses.rs"]
mod tests;
