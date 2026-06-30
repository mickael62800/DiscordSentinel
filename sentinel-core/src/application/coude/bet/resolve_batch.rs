//! Orchestration de la resolution batch des combats Coup de Coude en phase
//! de paris. Implementation du use case `ResolveBettingBatchUseCase`.
//!
//! Cette couche application :
//!   - appelle le port outbound `CombatRepository` pour claim atomique
//!   - charge les joueurs via `PlayerRepository`
//!   - charge les events via `SocialRepository`
//!   - appelle le domain service `coude_combat_engine::resolve_combat` (pur)
//!   - applique les effets via les repos/use cases existants (wallet, stats,
//!     HP, assurance, paris)
//!   - retourne les metadonnees Discord pour que le worker poste le resultat
//!
//! Respect strict de l'architecture hexagonale : aucun SQL direct ici, tout
//! passe par les ports outbound. Le combat_engine est pur domain (pas d'IO).

#[cfg(test)]
#[path = "tests/resolve_batch.rs"]
mod tests;

use std::sync::Arc;

use async_trait::async_trait;
use tracing::warn;

use crate::domain::entities::coude::balance::BalanceParams;
use crate::domain::entities::coude::combat::resolution_rules::apply_insurance_to_loss;
use crate::domain::entities::coude::combat::Combat;
use crate::domain::errors::DomainError;
use crate::domain::services::coude::coude_combat_engine as engine;
use crate::domain::services::coude::coude_combat_engine::PlayerLite;
use crate::domain::services::coude::coude_combat_engine::ServerEventLite;
use crate::ports::inbound::coude::manage_bets::ManageCoudeBetsUseCase;
use crate::ports::inbound::coude::manage_inventory::ManageCoudeInventoryUseCase;
use crate::ports::inbound::coude::manage_social::ManageCoudeSocialUseCase;
use crate::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase;
use crate::ports::inbound::coude::resolve_betting_batch::ResolveBettingBatchUseCase;
use crate::ports::inbound::coude::resolve_betting_batch::ResolvedBettingCombatOutput;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
use crate::ports::outbound::coude::combat_repository::CombatRepository;
use crate::ports::outbound::coude::player_repository::PlayerRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
/// Delai de paris par defaut (5 min), override par guild via bot_guild_config.
const DEFAULT_BET_DELAY_SECS: i64 = 300;
/// Au-dela de 120s en 'resolving', on considere le combat stuck et on retry.
const STUCK_THRESHOLD_SECS: i64 = 120;

pub struct ResolveBettingBatchService {
    combat_repo: Arc<dyn CombatRepository>,
    player_repo: Arc<dyn PlayerRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
    bets_uc: Arc<dyn ManageCoudeBetsUseCase>,
    inventory_uc: Arc<dyn ManageCoudeInventoryUseCase>,
    social_uc: Arc<dyn ManageCoudeSocialUseCase>,
    taunts_uc: Arc<dyn ManageCoudeTauntsUseCase>,
    bot_config_repo: Arc<dyn BotConfigRepository>,
}

impl ResolveBettingBatchService {
    pub fn new(
        combat_repo: Arc<dyn CombatRepository>,
        player_repo: Arc<dyn PlayerRepository>,
        wallet_repo: Arc<dyn WalletRepository>,
        bets_uc: Arc<dyn ManageCoudeBetsUseCase>,
        inventory_uc: Arc<dyn ManageCoudeInventoryUseCase>,
        social_uc: Arc<dyn ManageCoudeSocialUseCase>,
        taunts_uc: Arc<dyn ManageCoudeTauntsUseCase>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self {
            combat_repo,
            player_repo,
            wallet_repo,
            bets_uc,
            inventory_uc,
            social_uc,
            taunts_uc,
            bot_config_repo,
        }
    }

    /// Charge les parametres de balance de la guild ou default.
    async fn load_balance(&self, guild_id: &str) -> BalanceParams {
        crate::application::coude::guild_settings::load_balance_params(
            &*self.bot_config_repo,
            guild_id,
        )
        .await
    }

    /// Charge un joueur et le convertit en `PlayerLite` pour le moteur.
    async fn load_player(&self, guild_id: &str, user_id: &str) -> Result<PlayerLite, DomainError> {
        let p = self
            .player_repo
            .get(guild_id, user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Joueur {user_id} introuvable")))?;
        Ok(PlayerLite {
            user_id: p.user_id,
            class: p.class.as_ref().map(|c| c.as_str().to_string()),
            level: p.level,
            atk: p.atk,
            def: p.def,
            cowardice_count: p.cowardice_count,
            hp_current: Some(p.hp_current),
        })
    }

    /// Resout un combat unique et applique tous les effets de bord.
    async fn resolve_one(
        &self,
        combat: &Combat,
    ) -> Result<ResolvedBettingCombatOutput, DomainError> {
        let attacker = self
            .load_player(&combat.guild_id, &combat.attacker_id)
            .await?;
        let defender = self
            .load_player(&combat.guild_id, &combat.defender_id)
            .await?;

        let events = self
            .social_uc
            .list_active_events(&combat.guild_id)
            .await
            .unwrap_or_else(|e| {
                warn!(error = %e, guild_id = %combat.guild_id, "Echec chargement events actifs");
                vec![]
            });
        let engine_events: Vec<ServerEventLite> = events
            .into_iter()
            .map(|e| ServerEventLite {
                event_type: e.event_type,
            })
            .collect();

        // HP courants (deja lus dans load_player)
        let atk_hp = attacker.hp_current.unwrap_or(100);
        let def_hp = defender.hp_current.unwrap_or(100);

        // Charge params balance pour la guild (Phase 132).
        let balance = self.load_balance(&combat.guild_id).await;

        // Moteur pur (domain).
        let result = engine::combat::resolve_combat(
            &attacker,
            &defender,
            atk_hp,
            def_hp,
            combat.mise,
            combat.special_attack.as_deref(),
            combat.defender_special.as_deref(),
            &engine_events,
            &balance,
        );

        let first_atk_roll = result.rounds.first().map(|r| r.attacker_roll).unwrap_or(0);
        let first_def_roll = result.rounds.first().map(|r| r.defender_roll).unwrap_or(0);
        let chaos_key = result
            .rounds
            .iter()
            .find_map(|r| r.chaos_event)
            .map(|ce| ce.key().to_string());

        // ── Draw / Explosion path ──
        if result.winner_id.is_none() {
            let explosion_loss = result.coins_lost_by_loser;

            // Update combat + HP
            self.combat_repo
                .resolve(
                    combat.id,
                    crate::domain::entities::coude::combat::CombatResolution {
                        status: "accepted".into(),
                        winner_id: None,
                        attacker_roll: Some(first_atk_roll),
                        defender_roll: Some(first_def_roll),
                        chaos_event: chaos_key.clone(),
                        result_message: Some(result.message.clone()),
                        coins_transferred: explosion_loss,
                    },
                )
                .await?;

            self.player_repo
                .update_hp(
                    &combat.guild_id,
                    &combat.attacker_id,
                    result.attacker_hp_final.max(0),
                    result.attacker_hp_max,
                )
                .await
                .ok();
            self.player_repo
                .update_hp(
                    &combat.guild_id,
                    &combat.defender_id,
                    result.defender_hp_final.max(0),
                    result.defender_hp_max,
                )
                .await
                .ok();

            // Explosion : les 2 joueurs perdent explosion_loss chacun.
            // BUG critique fix : les erreurs etaient warn-only, masquant des
            // pertes/credits manques. On passe en error! avec event_type=
            // "combat.wallet_inconsistency" pour visibilite dans Logs systeme.
            if explosion_loss > 0 {
                let desc = format!("Explosion combat {}", combat.id);
                if let Err(e) = self
                    .wallet_repo
                    .debit(
                        &combat.guild_id,
                        &combat.attacker_id,
                        explosion_loss,
                        "coude_combat_explosion",
                        &desc,
                    )
                    .await
                {
                    tracing::error!(
                        event_type = "combat.wallet_inconsistency",
                        combat_id = %combat.id,
                        guild_id = %combat.guild_id,
                        user_id = %combat.attacker_id,
                        op = "debit_explosion_attacker",
                        amount = explosion_loss,
                        error = %e,
                        "Echec debit attacker explosion : combat marque resolu mais joueur n'a pas perdu de pieces"
                    );
                }
                if let Err(e) = self
                    .wallet_repo
                    .debit(
                        &combat.guild_id,
                        &combat.defender_id,
                        explosion_loss,
                        "coude_combat_explosion",
                        &desc,
                    )
                    .await
                {
                    tracing::error!(
                        event_type = "combat.wallet_inconsistency",
                        combat_id = %combat.id,
                        guild_id = %combat.guild_id,
                        user_id = %combat.defender_id,
                        op = "debit_explosion_defender",
                        amount = explosion_loss,
                        error = %e,
                        "Echec debit defender explosion : combat marque resolu mais joueur n'a pas perdu de pieces"
                    );
                }
                // Migration #3 wallet : on utilise record_draw (counter-only
                // apres migration) plutot que record_coins_lost (qui debite
                // encore user_wallets) — evite le double-debit puisque
                // wallet_repo.debit a deja ete applique ci-dessus.
                if let Err(e) = self
                    .player_repo
                    .record_draw(&combat.guild_id, &combat.attacker_id, explosion_loss)
                    .await
                {
                    warn!(error = %e, "Echec record_draw attacker explosion");
                }
                if let Err(e) = self
                    .player_repo
                    .record_draw(&combat.guild_id, &combat.defender_id, explosion_loss)
                    .await
                {
                    warn!(error = %e, "Echec record_draw defender explosion");
                }
            }

            // Refund tous les paris (egalite/explosion).
            if let Err(e) = self.bets_uc.refund(combat.id).await {
                warn!(error = %e, combat_id = %combat.id, "Echec refund paris egalite/explosion");
            }

            // Phase 9 Part D : egalite → reset les streaks de combat des
            // deux joueurs. Pas de taunt event pour un draw.
            if let Err(e) = self
                .taunts_uc
                .on_player_drew(&combat.guild_id, &combat.attacker_id)
                .await
            {
                warn!(error = %e, "Echec reset streaks attacker draw");
            }
            if let Err(e) = self
                .taunts_uc
                .on_player_drew(&combat.guild_id, &combat.defender_id)
                .await
            {
                warn!(error = %e, "Echec reset streaks defender draw");
            }

            return Ok(ResolvedBettingCombatOutput {
                combat_id: combat.id.to_string(),
                guild_id: combat.guild_id.clone(),
                channel_id: combat.channel_id.clone(),
                message_id: combat.message_id.clone(),
                result_message: result.message,
                winner_id: None,
                loser_id: None,
                coins_transferred: explosion_loss,
                is_draw: true,
                taunt_events: vec![],
            });
        }

        // ── Winner path ──
        // Garde-fou : si pour une raison quelconque le resultat n'expose pas
        // de vainqueur/perdant clairs (devrait etre couvert par le chemin
        // egalite plus haut), on traite le combat comme une egalite sans transfert
        // plutot que de paniquer sur un unwrap.
        let (Some(winner_id), Some(loser_id)) = (result.winner_id.clone(), result.loser_id.clone())
        else {
            warn!(
                combat_id = %combat.id,
                "Combat sans vainqueur/perdant hors chemin egalite : traite comme no-op"
            );
            if let Err(e) = self.bets_uc.refund(combat.id).await {
                warn!(error = %e, combat_id = %combat.id, "Echec refund paris (winner/loser absent)");
            }
            return Ok(ResolvedBettingCombatOutput {
                combat_id: combat.id.to_string(),
                guild_id: combat.guild_id.clone(),
                channel_id: combat.channel_id.clone(),
                message_id: combat.message_id.clone(),
                result_message: result.message,
                winner_id: None,
                loser_id: None,
                coins_transferred: 0,
                is_draw: true,
                taunt_events: vec![],
            });
        };

        // Cap sur solde reel du perdant (plus de coins ex-nihilo).
        let loser_wallet = self
            .wallet_repo
            .get(&combat.guild_id, &loser_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("Wallet perdant {loser_id} introuvable"))
            })?;
        let loser_balance = loser_wallet.coins;

        // Assurance : regles pures → domain::apply_insurance_to_loss
        // (clamp-then-apply, sémantique unifiée avec resolve_combat_now).
        let insurance = self
            .inventory_uc
            .get_active_insurance(&combat.guild_id, &loser_id)
            .await
            .unwrap_or(None);
        let adj = apply_insurance_to_loss(
            result.coins_lost_by_loser,
            loser_balance,
            insurance.as_ref(),
            &loser_id,
        );
        if let Some(ins_id) = adj.consumed_insurance_id {
            if let Err(e) = self.inventory_uc.expire_insurance(ins_id).await {
                warn!(error = %e, "Echec expire_insurance");
            }
        }
        let actual_loss = adj.actual_loss;
        let insurance_msg = adj.message;

        let coins_transferred = result.coins_won.min(loser_balance);

        // Update combat row.
        self.combat_repo
            .resolve(
                combat.id,
                crate::domain::entities::coude::combat::CombatResolution {
                    status: "accepted".into(),
                    winner_id: Some(winner_id.clone()),
                    attacker_roll: Some(first_atk_roll),
                    defender_roll: Some(first_def_roll),
                    chaos_event: chaos_key,
                    result_message: Some(result.message.clone()),
                    coins_transferred,
                },
            )
            .await?;

        // HP
        self.player_repo
            .update_hp(
                &combat.guild_id,
                &combat.attacker_id,
                result.attacker_hp_final.max(0),
                result.attacker_hp_max,
            )
            .await
            .ok();
        self.player_repo
            .update_hp(
                &combat.guild_id,
                &combat.defender_id,
                result.defender_hp_final.max(0),
                result.defender_hp_max,
            )
            .await
            .ok();

        // Transferts wallet
        // BUG critique fix (atomicite) : credit winner + debit loser dans la
        // MEME tx Postgres via pay_combat_atomic (identique a resolve_now).
        // Evite la creation/destruction de pieces si une seule des deux
        // operations reussit. error! (et non warn!) avec event_type clair
        // pour detecter les desyncs wallet.
        let combat_desc = format!("Combat {winner_id} vs {loser_id}");
        let mut payout_ok = true;
        if coins_transferred > 0 || actual_loss > 0 {
            if let Err(e) = self
                .wallet_repo
                .pay_combat_atomic(
                    &combat.guild_id,
                    &winner_id,
                    coins_transferred,
                    &loser_id,
                    actual_loss,
                    "coude_combat",
                    &combat_desc,
                )
                .await
            {
                tracing::error!(
                    event_type = "combat.wallet_inconsistency",
                    combat_id = %combat.id,
                    guild_id = %combat.guild_id,
                    winner_id = %winner_id,
                    loser_id = %loser_id,
                    op = "pay_combat_atomic",
                    coins_transferred,
                    actual_loss,
                    error = %e,
                    "Echec payout combat atomique : combat marque resolu mais l'argent n'a pas bouge — stats non enregistrees pour preserver la coherence"
                );
                payout_ok = false;
            }
        }

        // Stats (wins/losses via record_*) : seulement si le payout a reussi,
        // pour eviter un total_won/total_lost incoherent vs wallet.
        if payout_ok {
            if let Err(e) = self
                .player_repo
                .record_win(
                    &combat.guild_id,
                    &winner_id,
                    coins_transferred,
                    result.stolen_bonus,
                )
                .await
            {
                warn!(error = %e, "Echec record_win");
            }
            if let Err(e) = self
                .player_repo
                .record_loss(&combat.guild_id, &loser_id, actual_loss)
                .await
            {
                warn!(error = %e, "Echec record_loss");
            }
        }

        // Vol chaos bonus (cap sur solde restant du perdant).
        if result.vol_coins > 0 {
            let wallet_after = self
                .wallet_repo
                .get(&combat.guild_id, &loser_id)
                .await
                .ok()
                .flatten();
            let available = wallet_after.map(|w| w.coins).unwrap_or(0);
            let vol_capped = result.vol_coins.min(available);
            if vol_capped > 0 {
                let vol_desc = format!("Vol chaos combat {}", combat.id);
                // BUG critique fix (atomicite) : le vol chaos est un transfert
                // winner-credit + loser-debit -> on l'execute via
                // pay_combat_atomic (une seule tx) pour ne plus risquer de
                // debiter la victime sans crediter le winner (ou inversement).
                if let Err(e) = self
                    .wallet_repo
                    .pay_combat_atomic(
                        &combat.guild_id,
                        &winner_id,
                        vol_capped,
                        &loser_id,
                        vol_capped,
                        "coude_combat_vol",
                        &vol_desc,
                    )
                    .await
                {
                    tracing::error!(
                        event_type = "combat.wallet_inconsistency",
                        combat_id = %combat.id,
                        guild_id = %combat.guild_id,
                        winner_id = %winner_id,
                        loser_id = %loser_id,
                        op = "vol_chaos_pay_combat_atomic",
                        amount = vol_capped,
                        error = %e,
                        "Echec vol chaos atomique : transfert annule (ni debit victime ni credit winner)"
                    );
                }
            }
        }

        // Chaos events counter
        if result.chaos_events_count > 0 {
            let _ = self
                .player_repo
                .increment_chaos(&combat.guild_id, &combat.attacker_id)
                .await;
            let _ = self
                .player_repo
                .increment_chaos(&combat.guild_id, &combat.defender_id)
                .await;
        }

        // XP (winner +15 ou +30 giant killer, loser +5)
        let xp_winner = if result.is_giant_killer { 30 } else { 15 };
        let _ = self
            .player_repo
            .add_xp(&combat.guild_id, &winner_id, xp_winner)
            .await;
        let _ = self
            .player_repo
            .add_xp(&combat.guild_id, &loser_id, 5)
            .await;

        // Resolve bets (Migration #7 : collecte aussi les taunts jackpot
        // cote parieurs gagnants + bonus combattants).
        let bets_taunts = match self
            .bets_uc
            .resolve(combat.id, Some(winner_id.clone()))
            .await
        {
            Ok(outcome) => outcome.taunt_events,
            Err(e) => {
                warn!(error = %e, combat_id = %combat.id, "Echec resolve paris");
                Vec::new()
            }
        };

        // Message final : on y attache le message assurance si present
        let final_message = if let Some(ins_msg) = insurance_msg {
            format!("{}\n\n{}", result.message, ins_msg)
        } else {
            result.message
        };

        // Phase 9 Part D : track streaks et collecte les taunt events.
        let mut taunt_events = bets_taunts;
        match self
            .taunts_uc
            .on_player_won(&combat.guild_id, &winner_id)
            .await
        {
            Ok(Some(ev)) => taunt_events.push(ev),
            Ok(None) => {}
            Err(e) => warn!(error = %e, "Echec on_player_won"),
        }
        match self
            .taunts_uc
            .on_player_lost(&combat.guild_id, &loser_id)
            .await
        {
            Ok(Some(ev)) => taunt_events.push(ev),
            Ok(None) => {}
            Err(e) => warn!(error = %e, "Echec on_player_lost"),
        }

        Ok(ResolvedBettingCombatOutput {
            combat_id: combat.id.to_string(),
            guild_id: combat.guild_id.clone(),
            channel_id: combat.channel_id.clone(),
            message_id: combat.message_id.clone(),
            result_message: final_message,
            winner_id: Some(winner_id),
            loser_id: Some(loser_id),
            coins_transferred,
            is_draw: false,
            taunt_events,
        })
    }
}

#[async_trait]
impl ResolveBettingBatchUseCase for ResolveBettingBatchService {
    async fn resolve_batch(&self) -> Result<Vec<ResolvedBettingCombatOutput>, DomainError> {
        // Claim atomique
        let mut combats = self
            .combat_repo
            .claim_due_betting_combats(DEFAULT_BET_DELAY_SECS)
            .await?;
        let stuck = self
            .combat_repo
            .claim_stuck_resolving_combats(STUCK_THRESHOLD_SECS)
            .await?;
        combats.extend(stuck);

        let mut resolved = Vec::with_capacity(combats.len());
        for combat in &combats {
            match self.resolve_one(combat).await {
                Ok(out) => resolved.push(out),
                Err(e) => {
                    warn!(
                        error = %e,
                        combat_id = %combat.id,
                        "Echec resolution combat, passe au suivant"
                    );
                }
            }
        }
        Ok(resolved)
    }
}
