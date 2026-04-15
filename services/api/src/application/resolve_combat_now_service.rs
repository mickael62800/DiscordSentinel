//! Orchestration pour la resolution instantanee d'un combat (attaque
//! surprise / bloodbath / defense via item). Phase 7 refacto.
//!
//! Avant Phase 7 : 450 lignes de logique metier dans
//! `bots/coude-bot/src/commands/accepter.rs::resolve_combat_internal`
//! appelant directement `bots/coude-bot/src/game/combat.rs` (duplique).
//!
//! Apres Phase 7 : toute la logique vit ici (couche application de l'API),
//! le bot appelle juste le RPC `ResolveCombatNow` et poste l'embed
//! retourne pret a l'emploi.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::warn;
use uuid::Uuid;

use crate::domain::errors::DomainError;
use crate::domain::services::coude_combat_engine::{
    self as engine, PlayerLite, ServerEventLite,
};
use crate::ports::inbound::resolve_combat_now::{
    ResolveCombatNowOutput, ResolveCombatNowUseCase, ResolvedCombatEmbedField,
};
use crate::ports::inbound::{
    ManageCoudeBetsUseCase, ManageCoudeCombatsUseCase, ManageCoudeInventoryUseCase,
    ManageCoudePlayersUseCase, ManageCoudeSocialUseCase, ManageCoudeTauntsUseCase,
};
use crate::ports::outbound::{CoudeCombatRepository, WalletRepository};

pub struct ResolveCombatNowService {
    combat_repo: Arc<dyn CoudeCombatRepository>,
    combats_uc: Arc<dyn ManageCoudeCombatsUseCase>,
    players_uc: Arc<dyn ManageCoudePlayersUseCase>,
    wallet_repo: Arc<dyn WalletRepository>,
    bets_uc: Arc<dyn ManageCoudeBetsUseCase>,
    inventory_uc: Arc<dyn ManageCoudeInventoryUseCase>,
    social_uc: Arc<dyn ManageCoudeSocialUseCase>,
    taunts_uc: Arc<dyn ManageCoudeTauntsUseCase>,
}

impl ResolveCombatNowService {
    pub fn new(
        combat_repo: Arc<dyn CoudeCombatRepository>,
        combats_uc: Arc<dyn ManageCoudeCombatsUseCase>,
        players_uc: Arc<dyn ManageCoudePlayersUseCase>,
        wallet_repo: Arc<dyn WalletRepository>,
        bets_uc: Arc<dyn ManageCoudeBetsUseCase>,
        inventory_uc: Arc<dyn ManageCoudeInventoryUseCase>,
        social_uc: Arc<dyn ManageCoudeSocialUseCase>,
        taunts_uc: Arc<dyn ManageCoudeTauntsUseCase>,
    ) -> Self {
        Self {
            combat_repo,
            combats_uc,
            players_uc,
            wallet_repo,
            bets_uc,
            inventory_uc,
            social_uc,
            taunts_uc,
        }
    }
}

#[async_trait]
impl ResolveCombatNowUseCase for ResolveCombatNowService {
    async fn resolve_now(
        &self,
        combat_id: Uuid,
    ) -> Result<ResolveCombatNowOutput, DomainError> {
        // 1. Charger le combat
        let combat = self.combats_uc.get(combat_id).await?;

        // 2. Charger les joueurs
        let attacker = self
            .players_uc
            .get(&combat.guild_id, &combat.attacker_id)
            .await?;
        let defender = self
            .players_uc
            .get(&combat.guild_id, &combat.defender_id)
            .await?;

        // 3. Events actifs
        let events = self
            .social_uc
            .list_active_events(&combat.guild_id)
            .await
            .unwrap_or_default();
        let engine_events: Vec<ServerEventLite> = events
            .into_iter()
            .map(|e| ServerEventLite { event_type: e.event_type })
            .collect();

        // 4. Moteur de combat (pur domain)
        let atk_player = PlayerLite {
            user_id: attacker.user_id.clone(),
            class: attacker.class.as_ref().map(|c| c.as_str().to_string()),
            level: attacker.level,
            atk: attacker.atk,
            def: attacker.def,
            cowardice_count: attacker.cowardice_count,
            hp_current: Some(attacker.hp_current),
        };
        let def_player = PlayerLite {
            user_id: defender.user_id.clone(),
            class: defender.class.as_ref().map(|c| c.as_str().to_string()),
            level: defender.level,
            atk: defender.atk,
            def: defender.def,
            cowardice_count: defender.cowardice_count,
            hp_current: Some(defender.hp_current),
        };

        let result = engine::combat::resolve_combat(
            &atk_player,
            &def_player,
            attacker.hp_current,
            defender.hp_current,
            combat.mise,
            combat.special_attack.as_deref(),
            combat.defender_special.as_deref(),
            &engine_events,
        );

        let first_atk_roll = result.rounds.first().map(|r| r.attacker_roll).unwrap_or(0);
        let first_def_roll = result.rounds.first().map(|r| r.defender_roll).unwrap_or(0);
        let chaos_key = result
            .rounds
            .iter()
            .find_map(|r| r.chaos_event)
            .map(|c| c.key().to_string());

        // 5. Persister le combat
        self.combat_repo
            .resolve(
                combat.id,
                crate::domain::entities::CombatResolution {
                    status: "accepted".into(),
                    winner_id: result.winner_id.clone(),
                    attacker_roll: Some(first_atk_roll),
                    defender_roll: Some(first_def_roll),
                    chaos_event: chaos_key.clone(),
                    result_message: Some(result.message.clone()),
                    coins_transferred: result.coins_won.max(result.coins_lost_by_loser),
                },
            )
            .await?;

        // 6. HP
        let _ = self
            .players_uc
            .update_hp(
                &combat.guild_id,
                &combat.attacker_id,
                result.attacker_hp_final.max(0),
                result.attacker_hp_max,
            )
            .await;
        let _ = self
            .players_uc
            .update_hp(
                &combat.guild_id,
                &combat.defender_id,
                result.defender_hp_final.max(0),
                result.defender_hp_max,
            )
            .await;

        let mut fields: Vec<ResolvedCombatEmbedField> = Vec::new();

        // Champ combat : N rounds | HP...
        fields.push(ResolvedCombatEmbedField {
            name: "Combat".into(),
            value: format!(
                "{} rounds | <@{}> : {}/{} HP | <@{}> : {}/{} HP",
                result.total_rounds,
                combat.attacker_id,
                result.attacker_hp_final,
                result.attacker_hp_max,
                combat.defender_id,
                result.defender_hp_final,
                result.defender_hp_max,
            ),
            inline: false,
        });

        // 7. Winner path / Draw path
        let (mut title_color, mut insurance_msg, mut prime_amount, mut xp_lines) =
            (0x57F287u32, None::<String>, 0i64, Vec::<String>::new());
        if result.chaos_events_count > 0 {
            title_color = 0x9B59B6;
        }

        match (&result.winner_id, &result.loser_id) {
            (Some(winner_id), Some(loser_id)) => {
                // Assurance
                let mut actual_loss = result.coins_lost_by_loser;
                if let Ok(Some(ins)) =
                    self.inventory_uc.get_active_insurance(&combat.guild_id, loser_id).await
                {
                    let _ = self.inventory_uc.expire_insurance(ins.id).await;
                    if ins.is_scam {
                        actual_loss = result.coins_lost_by_loser.saturating_mul(2);
                        insurance_msg = Some(format!(
                            "\u{1f480} L'assurance de <@{}> etait une **ARNAQUE** ! Double perte : **-{} coins** !",
                            loser_id, actual_loss
                        ));
                    } else {
                        actual_loss = result.coins_lost_by_loser / 2;
                        insurance_msg = Some(format!(
                            "\u{1f6e1}\u{fe0f} L'assurance a amorti le coup pour <@{}> ! Perte reduite : **-{} coins** (au lieu de {})",
                            loser_id, actual_loss, result.coins_lost_by_loser
                        ));
                    }
                }

                // Cap sur solde reel du perdant
                let loser_wallet = self
                    .wallet_repo
                    .get(&combat.guild_id, loser_id)
                    .await
                    .ok()
                    .flatten();
                let loser_balance = loser_wallet.map(|w| w.coins).unwrap_or(0);
                let coins_transferred = result.coins_won.min(loser_balance);
                let actual_loss = actual_loss.min(loser_balance);

                // Wallet transfers
                let desc = format!("Combat {} vs {}", winner_id, loser_id);
                if coins_transferred > 0 {
                    if let Err(e) = self
                        .wallet_repo
                        .credit(&combat.guild_id, winner_id, coins_transferred, "coude_combat_win", &desc)
                        .await
                    {
                        warn!(error = %e, "Echec credit winner");
                    }
                }
                if actual_loss > 0 {
                    if let Err(e) = self
                        .wallet_repo
                        .debit(&combat.guild_id, loser_id, actual_loss, "coude_combat_loss", &desc)
                        .await
                    {
                        warn!(error = %e, "Echec debit loser");
                    }
                }

                // Stats
                let _ = self
                    .players_uc
                    .record_win(&combat.guild_id, winner_id, coins_transferred, result.stolen_bonus)
                    .await;
                let _ = self
                    .players_uc
                    .record_loss(&combat.guild_id, loser_id, actual_loss)
                    .await;

                // Primes : si le perdant en a, le gagnant les recupere
                let winner_name = if *winner_id == combat.attacker_id {
                    &combat.attacker_name
                } else {
                    &combat.defender_name
                };
                prime_amount = self
                    .inventory_uc
                    .claim_primes(&combat.guild_id, loser_id, winner_id, winner_name)
                    .await
                    .unwrap_or(0);
                if prime_amount > 0 {
                    let _ = self
                        .players_uc
                        .record_coins_earned(&combat.guild_id, winner_id, prime_amount)
                        .await;
                }

                // Chaos events count
                if result.chaos_events_count > 0 {
                    let _ = self
                        .players_uc
                        .increment_chaos(&combat.guild_id, &combat.attacker_id)
                        .await;
                    let _ = self
                        .players_uc
                        .increment_chaos(&combat.guild_id, &combat.defender_id)
                        .await;
                }

                // XP
                let level_gap = (attacker.level - defender.level).abs();
                let winner_is_underdog = level_gap >= 3 && result.is_giant_killer;
                let winner_xp = if winner_is_underdog { 30 } else { 15 };
                let loser_xp = 5i64;

                if let Ok(xp) = self
                    .players_uc
                    .add_xp(&combat.guild_id, winner_id, winner_xp)
                    .await
                {
                    xp_lines.push(format!(
                        "\u{2b06}\u{fe0f} <@{}> gagne **+{} XP**{}",
                        winner_id,
                        winner_xp,
                        if winner_is_underdog { " (Giant Killer x2 !)" } else { "" }
                    ));
                    if xp.leveled_up {
                        let title = title_for_level(xp.new_level);
                        xp_lines.push(format!(
                            "\u{1f31f} **LEVEL UP !** <@{}> passe niveau **{}** \u{300c}{}\u{300d} ! (+{} points de stats)",
                            winner_id, xp.new_level, title, xp.stat_points_gained
                        ));
                    }
                }
                if let Ok(xp) = self
                    .players_uc
                    .add_xp(&combat.guild_id, loser_id, loser_xp)
                    .await
                {
                    xp_lines.push(format!(
                        "\u{2b06}\u{fe0f} <@{}> gagne **+{} XP**",
                        loser_id, loser_xp
                    ));
                    if xp.leveled_up {
                        let title = title_for_level(xp.new_level);
                        xp_lines.push(format!(
                            "\u{1f31f} **LEVEL UP !** <@{}> passe niveau **{}** \u{300c}{}\u{300d} ! (+{} points de stats)",
                            loser_id, xp.new_level, title, xp.stat_points_gained
                        ));
                    }
                }

                // Paris
                let plan = self
                    .bets_uc
                    .resolve(combat.id, Some(winner_id.clone()))
                    .await
                    .ok();
                if let Some(plan) = plan {
                    if !plan.payouts.is_empty() {
                        let mut lines = vec!["\u{1f3b2} **Resultats des paris :**".to_string()];
                        for p in &plan.payouts {
                            if p.won {
                                lines.push(format!(
                                    "\u{2705} **{}** gagne **{} coins** !",
                                    p.bettor_name, p.payout
                                ));
                            } else {
                                lines.push(format!(
                                    "\u{274c} **{}** perd sa mise de **{} coins**",
                                    p.bettor_name, p.amount_bet
                                ));
                            }
                        }
                        if let Some(bonus) = plan.fighter_bonus {
                            lines.push(String::new());
                            lines.push(format!(
                                "\u{1f4b0} **Pot des paris : {} coins**",
                                bonus.total_pot
                            ));
                            lines.push(format!(
                                "\u{1f451} <@{}> recoit **+{} coins** (10% du pot)",
                                winner_id, bonus.winner_bonus
                            ));
                            let loser_display = if *winner_id == combat.attacker_id {
                                &combat.defender_id
                            } else {
                                &combat.attacker_id
                            };
                            lines.push(format!(
                                "\u{1f3c5} <@{}> recoit **+{} coins** (5% du pot, merci d'avoir participe)",
                                loser_display, bonus.loser_bonus
                            ));
                        }
                        fields.push(ResolvedCombatEmbedField {
                            name: "\u{1f3b2} Paris".into(),
                            value: lines.join("\n"),
                            inline: false,
                        });
                    }
                }
            }
            _ => {
                // Draw / accident_debile
                let had_accident = result.rounds.iter().any(|r| {
                    matches!(
                        r.chaos_event,
                        Some(engine::chaos::ChaosEvent::AccidentDebile)
                    )
                });
                if had_accident {
                    let _ = self
                        .players_uc
                        .record_draw(&combat.guild_id, &combat.attacker_id, combat.mise)
                        .await;
                    let _ = self
                        .players_uc
                        .record_draw(&combat.guild_id, &combat.defender_id, combat.mise)
                        .await;
                    let _ = self
                        .players_uc
                        .increment_chaos(&combat.guild_id, &combat.attacker_id)
                        .await;
                    let _ = self
                        .players_uc
                        .increment_chaos(&combat.guild_id, &combat.defender_id)
                        .await;
                }

                // Paris (refund tout le monde)
                let plan = self.bets_uc.resolve(combat.id, None).await.ok();
                if let Some(plan) = plan {
                    if !plan.payouts.is_empty() {
                        let mut lines = vec!["\u{1f3b2} **Resultats des paris :**".to_string()];
                        for p in &plan.payouts {
                            lines.push(format!(
                                "\u{274c} **{}** perd sa mise de **{} coins**",
                                p.bettor_name, p.amount_bet
                            ));
                        }
                        fields.push(ResolvedCombatEmbedField {
                            name: "\u{1f3b2} Paris".into(),
                            value: lines.join("\n"),
                            inline: false,
                        });
                    }
                }
                title_color = 0x9B59B6; // draw = violet
            }
        }

        // Ajouter les champs XP / primes / assurance en ordre
        if !xp_lines.is_empty() {
            fields.push(ResolvedCombatEmbedField {
                name: "\u{1f4ca} Experience".into(),
                value: xp_lines.join("\n"),
                inline: false,
            });
        }
        if prime_amount > 0 {
            if let Some(winner_id) = &result.winner_id {
                fields.push(ResolvedCombatEmbedField {
                    name: "\u{1f4b0} Primes recuperees !".into(),
                    value: format!("<@{}> empoche {} coins de primes !", winner_id, prime_amount),
                    inline: false,
                });
            }
        }
        if let Some(ins_msg) = insurance_msg {
            fields.push(ResolvedCombatEmbedField {
                name: "\u{1f6e1}\u{fe0f} Assurance".into(),
                value: ins_msg,
                inline: false,
            });
        }

        // Phase 9 Part D : track streaks + collecte taunt events.
        let mut taunt_events = Vec::new();
        match (&result.winner_id, &result.loser_id) {
            (Some(winner_id), Some(loser_id)) => {
                if let Ok(Some(ev)) = self
                    .taunts_uc
                    .on_player_won(&combat.guild_id, winner_id)
                    .await
                {
                    taunt_events.push(ev);
                }
                if let Ok(Some(ev)) = self
                    .taunts_uc
                    .on_player_lost(&combat.guild_id, loser_id)
                    .await
                {
                    taunt_events.push(ev);
                }
            }
            _ => {
                // Draw : reset les deux streaks de combat.
                let _ = self
                    .taunts_uc
                    .on_player_drew(&combat.guild_id, &combat.attacker_id)
                    .await;
                let _ = self
                    .taunts_uc
                    .on_player_drew(&combat.guild_id, &combat.defender_id)
                    .await;
            }
        }

        Ok(ResolveCombatNowOutput {
            combat_id: combat.id.to_string(),
            title: "\u{2694}\u{fe0f} Resultat du Coup de Coude !".into(),
            description: result.message,
            color: title_color,
            fields,
            taunt_events,
        })
    }
}

/// Pure helper (duplicata volontaire avec domain/entities/coude_player.rs
/// `title_for_level`) pour rester self-contained dans l'orchestration.
fn title_for_level(level: i32) -> &'static str {
    match level {
        1..=4 => "Debutant",
        5..=9 => "Bagarreur",
        10..=14 => "Guerrier",
        15..=19 => "Veteran",
        20..=24 => "Champion",
        25 => "Inarretable",
        _ => "Debutant",
    }
}
