//! Orchestration de la resolution batch des combats Coup de Coude en phase
//! de paris. Implementation du use case `ResolveBettingBatchUseCase`.
//!
//! Cette couche application :
//!   - appelle le port outbound `CoudeCombatRepository` pour claim atomique
//!   - charge les joueurs via `CoudePlayerRepository`
//!   - charge les events via `CoudeSocialRepository`
//!   - appelle le domain service `coude_combat_engine::resolve_combat` (pur)
//!   - applique les effets via les repos/use cases existants (wallet, stats,
//!     HP, assurance, paris)
//!   - retourne les metadonnees Discord pour que le worker poste le resultat
//!
//! Respect strict de l'architecture hexagonale : aucun SQL direct ici, tout
//! passe par les ports outbound. Le combat_engine est pur domain (pas d'IO).

use std::sync::Arc;

use async_trait::async_trait;
use tracing::warn;

use crate::domain::entities::{CoudeBalanceParams, CoudeCombat};
use crate::domain::errors::DomainError;
use crate::domain::services::coude_combat_engine::{
    self as engine, PlayerLite, ServerEventLite,
};
use crate::ports::inbound::resolve_betting_batch::{
    ResolveBettingBatchUseCase, ResolvedBettingCombatOutput,
};
use crate::ports::inbound::{
    ManageCoudeBetsUseCase, ManageCoudeInventoryUseCase, ManageCoudeSocialUseCase,
    ManageCoudeTauntsUseCase,
};
use crate::ports::outbound::{
    BotConfigRepository, CoudeCombatRepository, CoudePlayerRepository, WalletRepository,
};

/// Delai de paris par defaut (5 min), override par guild via bot_guild_config.
const DEFAULT_BET_DELAY_SECS: i64 = 300;
/// Au-dela de 120s en 'resolving', on considere le combat stuck et on retry.
const STUCK_THRESHOLD_SECS: i64 = 120;

pub struct ResolveBettingBatchService {
    combat_repo: Arc<dyn CoudeCombatRepository>,
    player_repo: Arc<dyn CoudePlayerRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
    bets_uc: Arc<dyn ManageCoudeBetsUseCase>,
    inventory_uc: Arc<dyn ManageCoudeInventoryUseCase>,
    social_uc: Arc<dyn ManageCoudeSocialUseCase>,
    taunts_uc: Arc<dyn ManageCoudeTauntsUseCase>,
    bot_config_repo: Arc<dyn BotConfigRepository>,
}

impl ResolveBettingBatchService {
    pub fn new(
        combat_repo: Arc<dyn CoudeCombatRepository>,
        player_repo: Arc<dyn CoudePlayerRepository>,
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
    async fn load_balance(&self, guild_id: &str) -> CoudeBalanceParams {
        match self.bot_config_repo.get_config(guild_id, "coude-bot").await {
            Ok(entries) => {
                let map: std::collections::HashMap<String, String> = entries
                    .into_iter()
                    .map(|e| (e.config_key, e.config_value))
                    .collect();
                CoudeBalanceParams::from_config(&map)
            }
            Err(e) => {
                warn!(error = %e, guild_id, "Echec chargement coude balance params — default");
                CoudeBalanceParams::default()
            }
        }
    }

    /// Charge un joueur et le convertit en `PlayerLite` pour le moteur.
    async fn load_player(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<PlayerLite, DomainError> {
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
        combat: &CoudeCombat,
    ) -> Result<ResolvedBettingCombatOutput, DomainError> {
        let attacker = self.load_player(&combat.guild_id, &combat.attacker_id).await?;
        let defender = self.load_player(&combat.guild_id, &combat.defender_id).await?;

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
            .map(|e| ServerEventLite { event_type: e.event_type })
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
                    crate::domain::entities::CombatResolution {
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
            if explosion_loss > 0 {
                let desc = format!("Explosion combat {}", combat.id);
                if let Err(e) = self
                    .wallet_repo
                    .debit(&combat.guild_id, &combat.attacker_id, explosion_loss, "coude_combat_explosion", &desc)
                    .await
                {
                    warn!(error = %e, "Echec debit attacker explosion");
                }
                if let Err(e) = self
                    .wallet_repo
                    .debit(&combat.guild_id, &combat.defender_id, explosion_loss, "coude_combat_explosion", &desc)
                    .await
                {
                    warn!(error = %e, "Echec debit defender explosion");
                }
                if let Err(e) = self
                    .player_repo
                    .record_coins_lost(&combat.guild_id, &combat.attacker_id, explosion_loss)
                    .await
                {
                    warn!(error = %e, "Echec record_coins_lost attacker explosion");
                }
                if let Err(e) = self
                    .player_repo
                    .record_coins_lost(&combat.guild_id, &combat.defender_id, explosion_loss)
                    .await
                {
                    warn!(error = %e, "Echec record_coins_lost defender explosion");
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
        let winner_id = result.winner_id.clone().unwrap();
        let loser_id = result.loser_id.clone().unwrap();

        // Cap sur solde reel du perdant (plus de coins ex-nihilo).
        let loser_wallet = self
            .wallet_repo
            .get(&combat.guild_id, &loser_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Wallet perdant {loser_id} introuvable")))?;
        let loser_balance = loser_wallet.coins;

        // Assurance : cherche et consomme.
        let mut actual_loss = result.coins_lost_by_loser.min(loser_balance);
        let insurance = self
            .inventory_uc
            .get_active_insurance(&combat.guild_id, &loser_id)
            .await
            .unwrap_or(None);
        let insurance_msg = if let Some(ins) = insurance.as_ref() {
            if let Err(e) = self.inventory_uc.expire_insurance(ins.id).await {
                warn!(error = %e, "Echec expire_insurance");
            }
            if ins.is_scam {
                actual_loss = actual_loss.saturating_mul(2).min(loser_balance);
                Some(format!(
                    "\u{1f480} L'assurance de <@{loser_id}> etait une ARNAQUE ! Double perte : -{actual_loss} coins",
                ))
            } else {
                actual_loss /= 2;
                Some(format!(
                    "\u{1f6e1}\u{fe0f} L'assurance amortit le coup pour <@{loser_id}> ! Perte reduite : -{actual_loss} coins",
                ))
            }
        } else {
            None
        };

        let coins_transferred = result.coins_won.min(loser_balance);

        // Update combat row.
        self.combat_repo
            .resolve(
                combat.id,
                crate::domain::entities::CombatResolution {
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
        let combat_desc = format!("Combat {winner_id} vs {loser_id}");
        if coins_transferred > 0 {
            if let Err(e) = self
                .wallet_repo
                .credit(&combat.guild_id, &winner_id, coins_transferred, "coude_combat_win", &combat_desc)
                .await
            {
                warn!(error = %e, "Echec credit winner");
            }
        }
        if actual_loss > 0 {
            if let Err(e) = self
                .wallet_repo
                .debit(&combat.guild_id, &loser_id, actual_loss, "coude_combat_loss", &combat_desc)
                .await
            {
                warn!(error = %e, "Echec debit loser");
            }
        }

        // Stats (wins/losses via record_*)
        if let Err(e) = self
            .player_repo
            .record_win(&combat.guild_id, &winner_id, coins_transferred, result.stolen_bonus)
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
                let _ = self
                    .wallet_repo
                    .debit(&combat.guild_id, &loser_id, vol_capped, "coude_combat_vol_victim", &vol_desc)
                    .await;
                let _ = self
                    .wallet_repo
                    .credit(&combat.guild_id, &winner_id, vol_capped, "coude_combat_vol_bonus", &vol_desc)
                    .await;
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

        // Resolve bets
        if let Err(e) = self
            .bets_uc
            .resolve(combat.id, Some(winner_id.clone()))
            .await
        {
            warn!(error = %e, combat_id = %combat.id, "Echec resolve paris");
        }

        // Message final : on y attache le message assurance si present
        let final_message = if let Some(ins_msg) = insurance_msg {
            format!("{}\n\n{}", result.message, ins_msg)
        } else {
            result.message
        };

        // Phase 9 Part D : track streaks et collecte les taunt events.
        let mut taunt_events = Vec::new();
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
