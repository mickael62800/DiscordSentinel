//! Service `ResolveStealUseCase` : resolution serveur-side du vol.
//!
//! Portage FIDELE de l'ancien `voler.rs::resolve_steal_attempt` (bot),
//! desormais autorite serveur. Les regles (bonus classe `fourbe` +4,
//! `DEF/10`, malus AFK, penalite d'echec %) vivent ICI ou dans la config
//! serveur (`GuildSettings`), plus dans le bot. Mirror du pattern
//! `ResolveCombatNowService`.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::warn;

use crate::application::coude::guild_settings::GuildSettings;
use crate::domain::entities::coude::player::title_for_level;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_economy::ManageCoudeEconomyUseCase;
use crate::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase;
use crate::ports::inbound::coude::manage_steal_boosts::ManageCoudeStealBoostsUseCase;
use crate::ports::inbound::coude::manage_steal_protections::ManageCoudeStealProtectionsUseCase;
use crate::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase;
use crate::ports::inbound::coude::resolve_steal::ResolveStealCommand;
use crate::ports::inbound::coude::resolve_steal::ResolveStealOutput;
use crate::ports::inbound::coude::resolve_steal::ResolveStealUseCase;
use crate::ports::inbound::coude::resolve_steal::StealResolutionOutcome;
use crate::ports::inbound::coude::roll_steal::RollStealCommand;
use crate::ports::inbound::coude::roll_steal::RollStealUseCase;
use crate::ports::outbound::coude::flavor_templates_repository::FlavorTemplatesRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

/// Bonus de roll pour un voleur de classe "fourbe" (regle de combat, CORE).
const FOURBE_CLASS_BONUS: i32 = 4;

/// Diviseur du bonus defensif issu de la stat DEF (`def / 10`).
const DEF_BONUS_DIVISOR: i32 = 10;

/// Cles/defauts de config serveur (miroir exact du bot `guild_config`).
const KEY_FAILURE_PENALTY_PCT: &str = "steal_failure_penalty_pct";
const DEFAULT_FAILURE_PENALTY_PCT: i64 = 20;
const KEY_AFK_DEFENDER_MALUS: &str = "afk_defender_malus";
const DEFAULT_AFK_DEFENDER_MALUS: i32 = 8;

pub struct ResolveStealService {
    roll_uc: Arc<dyn RollStealUseCase>,
    players_uc: Arc<dyn ManageCoudePlayersUseCase>,
    economy_uc: Arc<dyn ManageCoudeEconomyUseCase>,
    taunts_uc: Arc<dyn ManageCoudeTauntsUseCase>,
    protections_uc: Arc<dyn ManageCoudeStealProtectionsUseCase>,
    boosts_uc: Arc<dyn ManageCoudeStealBoostsUseCase>,
    flavor_repo: Arc<dyn FlavorTemplatesRepository>,
    bot_config_repo: Arc<dyn BotConfigRepository>,
}

impl ResolveStealService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        roll_uc: Arc<dyn RollStealUseCase>,
        players_uc: Arc<dyn ManageCoudePlayersUseCase>,
        economy_uc: Arc<dyn ManageCoudeEconomyUseCase>,
        taunts_uc: Arc<dyn ManageCoudeTauntsUseCase>,
        protections_uc: Arc<dyn ManageCoudeStealProtectionsUseCase>,
        boosts_uc: Arc<dyn ManageCoudeStealBoostsUseCase>,
        flavor_repo: Arc<dyn FlavorTemplatesRepository>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self {
            roll_uc,
            players_uc,
            economy_uc,
            taunts_uc,
            protections_uc,
            boosts_uc,
            flavor_repo,
            bot_config_repo,
        }
    }

    /// Ligne de detail XP (miroir du bot). Ajoute "+N XP" et l'eventuel
    /// LEVEL UP au buffer. Best-effort : les erreurs sont ignorees.
    async fn append_xp_line(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        prefix: &str,
        out: &mut String,
    ) {
        if let Ok(xp) = self.players_uc.add_xp(guild_id, user_id, amount).await {
            out.push_str(prefix);
            if xp.leveled_up {
                let title = title_for_level(xp.new_level);
                out.push_str(&format!(
                    "\n\u{1f31f} **LEVEL UP !** Niveau **{}** \u{300c}{}\u{300d} ! (+{} points de stats)",
                    xp.new_level, title, xp.stat_points_gained
                ));
            }
        }
    }
}

/// Formatte un template flavor (`{voleur}`/`{victime}`/`{montant}`).
fn format_msg(template: &str, voleur: &str, victime: &str, montant: i64) -> String {
    template
        .replace("{voleur}", voleur)
        .replace("{victime}", victime)
        .replace("{montant}", &montant.to_string())
}

/// Embed de repli quand un template flavor est introuvable / l'API est
/// indisponible (miroir exact du bot).
fn api_unavailable(
    outcome: StealResolutionOutcome,
    stolen: i64,
    lost: i64,
    thief_roll: i32,
    victim_roll: i32,
    taunt_events: Vec<crate::domain::entities::coude::taunt::TauntEvent>,
) -> ResolveStealOutput {
    ResolveStealOutput {
        outcome,
        title: "\u{26a0}\u{fe0f} API indisponible".to_string(),
        description: "Veuillez reessayer plus tard.".to_string(),
        color: 0x95A5A6,
        stolen,
        lost,
        thief_roll,
        victim_roll,
        taunt_events,
    }
}

#[async_trait]
impl ResolveStealUseCase for ResolveStealService {
    async fn resolve_steal(
        &self,
        cmd: ResolveStealCommand,
    ) -> Result<ResolveStealOutput, DomainError> {
        let ResolveStealCommand {
            guild_id,
            thief_id,
            target_id,
            afk,
        } = cmd;

        // Config serveur (penalite d'echec + malus AFK) — mêmes cles/defauts
        // que l'ancien `load_guild_config` cote bot.
        let settings = GuildSettings::load(self.bot_config_repo.as_ref(), &guild_id).await;
        let failure_penalty_pct =
            settings.get_i64(KEY_FAILURE_PENALTY_PCT, DEFAULT_FAILURE_PENALTY_PCT);
        let afk_defender_malus =
            settings.get_i32(KEY_AFK_DEFENDER_MALUS, DEFAULT_AFK_DEFENDER_MALUS);

        // Joueurs (autorite serveur : on relit le solde/DEF/classe).
        let thief = self.players_uc.get(&guild_id, &thief_id).await?;
        let target = self.players_uc.get(&guild_id, &target_id).await?;

        // 1. Tirage RNG serveur (d20 thief/victim + % wallet vole).
        let roll = self
            .roll_uc
            .roll(RollStealCommand {
                guild_id: guild_id.clone(),
                afk,
            })
            .await?;
        let thief_roll = roll.thief_d20;
        let target_roll = roll.victim_d20;

        // 2. Bonus voleur : classe fourbe + somme des boosts actifs.
        let class_bonus = if thief.class.as_ref().map(|c| c.as_str()) == Some("fourbe") {
            FOURBE_CLASS_BONUS
        } else {
            0
        };
        let boost_bonus = self
            .boosts_uc
            .total_bonus(&guild_id, &thief_id)
            .await
            .unwrap_or(0);
        let thief_bonus = class_bonus + boost_bonus;

        // 3. Bonus defenseur : DEF/10, moins le malus si la cible est AFK.
        let mut target_bonus = target.def / DEF_BONUS_DIVISOR;
        if afk {
            target_bonus -= afk_defender_malus;
        }
        let thief_total = thief_roll + thief_bonus;
        let target_total = target_roll + target_bonus;

        // Detail du roll (miroir exact du bot : on ne leake le boost que
        // s'il est non nul).
        let thief_detail = if boost_bonus > 0 {
            format!(
                "d20: {} + class: {} + boost: {}",
                thief_roll, class_bonus, boost_bonus
            )
        } else {
            format!("d20: {} + bonus: {}", thief_roll, class_bonus)
        };
        let roll_detail = format!(
            "\n\n\u{1f3b2} Voleur: {} ({}) vs Victime: {} (d20: {} + DEF bonus: {}{})",
            thief_total,
            thief_detail,
            target_total,
            target_roll,
            target_bonus + if afk { afk_defender_malus } else { 0 },
            if afk {
                format!(" - AFK: {}", afk_defender_malus)
            } else {
                String::new()
            },
        );

        let mut taunt_events: Vec<crate::domain::entities::coude::taunt::TauntEvent> = Vec::new();

        if thief_total > target_total {
            // Le voleur a gagne le roll — une protection active peut encore
            // bloquer le vol (abonnements temps-base, pas de consommation).
            if let Ok(Some(trigger)) = self.protections_uc.try_trigger(&guild_id, &target_id).await
            {
                // Blocage reussi → reset le victim streak.
                if let Err(e) = self
                    .taunts_uc
                    .on_player_defended_steal(&guild_id, &target_id)
                    .await
                {
                    warn!(error = %e, "Echec on_player_defended_steal");
                }

                let protection_detail = format!(
                    "\n\u{1f3b2} Le voleur avait gagne le combat ({} > {}), mais la protection a fait un jet de **{}/100** (seuil **{}%**) → \u{2705} bloque !",
                    thief_total, target_total, trigger.rolled_value, trigger.block_chance_percent
                );
                let block_msg = format!(
                    "\u{1f6e1}\u{fe0f} <@{}> etait protege par **{}** qui a bloque la tentative de vol de <@{}> !",
                    target_id, trigger.item_name, thief_id
                );
                // La victime gagne +3 XP comme pour une defense reussie.
                let mut xp_line = String::new();
                self.append_xp_line(
                    &guild_id,
                    &target_id,
                    3,
                    &format!("\n\u{2b06}\u{fe0f} +3 XP pour <@{}>", target_id),
                    &mut xp_line,
                )
                .await;

                return Ok(ResolveStealOutput {
                    outcome: StealResolutionOutcome::Blocked,
                    title: "\u{1f6e1}\u{fe0f} Vol bloque !".to_string(),
                    description: format!(
                        "{}{}{}{}",
                        block_msg, roll_detail, protection_detail, xp_line
                    ),
                    color: 0x3498DB,
                    stolen: 0,
                    lost: 0,
                    thief_roll,
                    victim_roll: target_roll,
                    taunt_events,
                });
            }

            // Pas de protection : le vol reussit. % vole tire plus haut.
            let steal_pct: f64 = (roll.steal_pct_bp as f64) / 10_000.0;
            let stolen_base = ((target.coins as f64 * steal_pct) as i64).max(1);

            // Mutation wallet atomique (clamp serveur au solde victime) +
            // taunts (faillite victime / jackpot voleur).
            let stolen = match self
                .economy_uc
                .steal(&guild_id, &thief_id, &target_id, stolen_base)
                .await
            {
                Ok(outcome) => {
                    taunt_events.extend(outcome.taunt_events);
                    outcome.stolen
                }
                Err(e) => {
                    warn!(error = %e, "Echec economy.steal");
                    stolen_base
                }
            };

            // Incremente le victim streak + collecte taunt event.
            match self
                .taunts_uc
                .on_player_stolen_from(&guild_id, &target_id)
                .await
            {
                Ok(Some(ev)) => taunt_events.push(ev),
                Ok(None) => {}
                Err(e) => warn!(error = %e, "Echec on_player_stolen_from"),
            }

            let mut xp_line = String::new();
            self.append_xp_line(
                &guild_id,
                &thief_id,
                5,
                "\n\u{2b06}\u{fe0f} +5 XP pour le voleur",
                &mut xp_line,
            )
            .await;

            let key = if afk {
                "steal_success_afk"
            } else {
                "steal_success_fight"
            };
            let template_str = match self.flavor_repo.random_by_key(key, "fr").await {
                Ok(Some(s)) => s,
                Ok(None) | Err(_) => {
                    return Ok(api_unavailable(
                        StealResolutionOutcome::Success,
                        stolen,
                        0,
                        thief_roll,
                        target_roll,
                        taunt_events,
                    ));
                }
            };
            let msg_text = format_msg(
                &template_str,
                &format!("<@{}>", thief_id),
                &format!("<@{}>", target_id),
                stolen,
            );

            Ok(ResolveStealOutput {
                outcome: StealResolutionOutcome::Success,
                title: "\u{1f4b0} Vol reussi !".to_string(),
                description: format!("{}{}{}", msg_text, roll_detail, xp_line),
                color: 0x57F287,
                stolen,
                lost: 0,
                thief_roll,
                victim_roll: target_roll,
                taunt_events,
            })
        } else {
            // Vol echoue : le voleur perd `failure_penalty_pct`% de ses coins.
            let lost_base =
                ((thief.coins as f64 * (failure_penalty_pct as f64 / 100.0)) as i64).max(1);

            let lost = match self
                .economy_uc
                .steal_fail_penalty(&guild_id, &thief_id, lost_base)
                .await
            {
                Ok((actual_lost, wallet_taunts)) => {
                    taunt_events.extend(wallet_taunts);
                    actual_lost.max(1)
                }
                Err(e) => {
                    warn!(error = %e, "Echec economy.steal_fail_penalty");
                    lost_base
                }
            };

            // Vol rate = victime a "resiste", reset son streak.
            if let Err(e) = self
                .taunts_uc
                .on_player_defended_steal(&guild_id, &target_id)
                .await
            {
                warn!(error = %e, "Echec on_player_defended_steal (fail path)");
            }

            let mut xp_line = String::new();
            self.append_xp_line(
                &guild_id,
                &target_id,
                3,
                &format!("\n\u{2b06}\u{fe0f} +3 XP pour <@{}>", target_id),
                &mut xp_line,
            )
            .await;

            let template_str = match self.flavor_repo.random_by_key("steal_fail", "fr").await {
                Ok(Some(s)) => s,
                Ok(None) | Err(_) => {
                    return Ok(api_unavailable(
                        StealResolutionOutcome::Failed,
                        0,
                        lost,
                        thief_roll,
                        target_roll,
                        taunt_events,
                    ));
                }
            };
            let msg_text = format_msg(
                &template_str,
                &format!("<@{}>", thief_id),
                &format!("<@{}>", target_id),
                lost,
            );

            Ok(ResolveStealOutput {
                outcome: StealResolutionOutcome::Failed,
                title: "\u{1f6a8} Vol rate !".to_string(),
                description: format!("{}{}{}", msg_text, roll_detail, xp_line),
                color: 0xED4245,
                stolen: 0,
                lost,
                thief_roll,
                victim_roll: target_roll,
                taunt_events,
            })
        }
    }
}
