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

use crate::domain::entities::{apply_insurance_to_loss, compute_combat_xp, CoudeBalanceParams};
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
use crate::ports::outbound::{BotConfigRepository, CoudeCombatRepository, WalletRepository};

pub struct ResolveCombatNowService {
    combat_repo: Arc<dyn CoudeCombatRepository>,
    combats_uc: Arc<dyn ManageCoudeCombatsUseCase>,
    players_uc: Arc<dyn ManageCoudePlayersUseCase>,
    wallet_repo: Arc<dyn WalletRepository>,
    bets_uc: Arc<dyn ManageCoudeBetsUseCase>,
    inventory_uc: Arc<dyn ManageCoudeInventoryUseCase>,
    social_uc: Arc<dyn ManageCoudeSocialUseCase>,
    taunts_uc: Arc<dyn ManageCoudeTauntsUseCase>,
    bot_config_repo: Arc<dyn BotConfigRepository>,
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
        bot_config_repo: Arc<dyn BotConfigRepository>,
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
            bot_config_repo,
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

        // Charge les parametres de balance de la guild (fallback default
        // si bot_guild_config indispo ou vide).
        let balance = load_balance_params(self.bot_config_repo.as_ref(), &combat.guild_id).await;

        // Gate : si l'attaquant a lance une surprise ET que le defenseur
        // possede Explosion dans son inventaire ET que le flag
        // `surprise_allow_defender_counter` est actif, on refuse l'auto-
        // resolve. Le bot doit basculer sur le flow de defi normal pour
        // laisser le defenseur une chance de riposter.
        if combat.special_attack.as_deref() == Some("surprise")
            && combat.defender_special.is_none()
            && balance.surprise_allow_defender_counter
        {
            let has_explosion = self
                .inventory_uc
                .has_item(&combat.guild_id, &combat.defender_id, "explosion")
                .await
                .unwrap_or(false);
            if has_explosion {
                return Err(DomainError::Conflict(
                    "surprise_defender_can_counter: le defenseur possede Explosion, passer par le flow de defi normal"
                        .into(),
                ));
            }
        }

        let result = engine::combat::resolve_combat(
            &atk_player,
            &def_player,
            attacker.hp_current,
            defender.hp_current,
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
        // Migration #7 : taunts declenches par les mutations wallet des paris
        // (jackpots cote parieurs gagnants + bonus combattants). Fusionnes en
        // fin de fonction avec les taunts streaks win/loss.
        let mut bets_draw_taunts: Vec<crate::domain::entities::TauntEvent> = Vec::new();
        if result.chaos_events_count > 0 {
            title_color = 0x9B59B6;
        }

        match (&result.winner_id, &result.loser_id) {
            (Some(winner_id), Some(loser_id)) => {
                // Assurance (regles pures → domain::apply_insurance_to_loss)
                let active_insurance = self
                    .inventory_uc
                    .get_active_insurance(&combat.guild_id, loser_id)
                    .await
                    .ok()
                    .flatten();
                let adj = apply_insurance_to_loss(
                    result.coins_lost_by_loser,
                    active_insurance.as_ref(),
                    loser_id,
                );
                if let Some(ins_id) = adj.consumed_insurance_id {
                    let _ = self.inventory_uc.expire_insurance(ins_id).await;
                }
                insurance_msg = adj.message;
                let mut actual_loss = adj.actual_loss;

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
                    // Migration wallet finale : `record_coins_earned` est
                    // stats-only (increment total_earned). Le credit reel
                    // du wallet + log wallet_transactions est explicite ici
                    // (pas via wallet_uc pour rester cote WalletRepository
                    // deja injecte — les taunts de jackpot pour les primes
                    // sont hors scope, le montant est typiquement petit).
                    let prime_desc = format!("Primes combat {}", combat.id);
                    if let Err(e) = self
                        .wallet_repo
                        .credit(
                            &combat.guild_id,
                            winner_id,
                            prime_amount,
                            "coude_primes",
                            &prime_desc,
                        )
                        .await
                    {
                        warn!(error = %e, "Echec credit primes winner");
                    }
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

                // XP (regles pures → domain::compute_combat_xp)
                let awards = compute_combat_xp(
                    attacker.level, defender.level, result.is_giant_killer,
                );
                let winner_is_underdog = awards.winner_is_underdog;
                let winner_xp = awards.winner_xp;
                let loser_xp = awards.loser_xp;

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
                let outcome = self
                    .bets_uc
                    .resolve(combat.id, Some(winner_id.clone()))
                    .await
                    .ok();
                if let Some(outcome) = outcome {
                    bets_draw_taunts = outcome.taunt_events;
                    let plan = outcome.plan;
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
                    // Accident debile : les deux joueurs sont penalises de
                    // `combat.mise`. On debite le wallet explicitement (avant,
                    // record_draw faisait le debit en interne — plus le cas
                    // depuis la migration #3 wallet).
                    let desc = format!("Accident debile combat {}", combat.id);
                    if combat.mise > 0 {
                        let _ = self
                            .wallet_repo
                            .debit(&combat.guild_id, &combat.attacker_id, combat.mise, "coude_combat_draw", &desc)
                            .await;
                        let _ = self
                            .wallet_repo
                            .debit(&combat.guild_id, &combat.defender_id, combat.mise, "coude_combat_draw", &desc)
                            .await;
                    }
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
                let outcome = self.bets_uc.resolve(combat.id, None).await.ok();
                if let Some(outcome) = outcome {
                    if !outcome.plan.payouts.is_empty() {
                        let mut lines = vec!["\u{1f3b2} **Resultats des paris :**".to_string()];
                        for p in &outcome.plan.payouts {
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
                    // Taunts declenches lors de l'application des paris
                    // (jackpots parieurs / bonus combattants — pour un draw,
                    // pas de jackpot attendu cote payouts, mais on propage
                    // defensively).
                    // bets_draw_taunts collectes ici, fusionnes plus bas avec
                    // les streaks win/loss/draw.
                    bets_draw_taunts = outcome.taunt_events;
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

        // Migration #7 : fusionne les taunts issus de la resolution paris
        // (jackpots parieurs + bonus combattants) avec ceux des streaks.
        taunt_events.extend(bets_draw_taunts);

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

/// Charge les parametres de balance du jeu Coup de Coude pour une guild
/// depuis `bot_guild_config` (`bot_name = 'coude-bot'`). Retombe sur le
/// default si l'appel echoue — on prefere ne pas bloquer un combat pour
/// une erreur de lecture de config.
async fn load_balance_params(
    repo: &dyn crate::ports::outbound::BotConfigRepository,
    guild_id: &str,
) -> CoudeBalanceParams {
    match repo.get_config(guild_id, "coude-bot").await {
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

use crate::domain::entities::coude_title_for_level as title_for_level;
