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

#[cfg(test)]
#[path = "tests/resolve_combat_now.rs"]
mod tests;

use std::sync::Arc;

use async_trait::async_trait;
use tracing::warn;
use uuid::Uuid;

use crate::domain::entities::coude::combat_resolution_rules::apply_insurance_to_loss;
use crate::domain::entities::coude::combat_resolution_rules::compute_combat_xp;
use crate::domain::entities::coude::combat_outcome_flags::detect_outcome_flags;
use crate::domain::entities::coude::combat_resolution_rules::format_bet_payout_lines;
use crate::domain::entities::coude::combat_outcome_flags::CombatOutcomeFlags;
use crate::domain::entities::coude::combat_outcome_flags::COMEBACK_HP_PCT_MAX;
use crate::domain::errors::DomainError;
use crate::domain::services::coude::coude_combat_engine as engine;
use crate::domain::services::coude::coude_combat_engine::PlayerLite;
use crate::domain::services::coude::coude_combat_engine::ServerEventLite;
use crate::ports::inbound::coude::resolve_combat_now::ResolveCombatNowOutput;
use crate::ports::inbound::coude::resolve_combat_now::ResolveCombatNowUseCase;
use crate::ports::inbound::coude::resolve_combat_now::ResolvedCombatEmbedField;
use crate::ports::inbound::coude::manage_bets::ManageCoudeBetsUseCase;
use crate::ports::inbound::coude::manage_combats::ManageCoudeCombatsUseCase;
use crate::ports::inbound::coude::manage_inventory::ManageCoudeInventoryUseCase;
use crate::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase;
use crate::ports::inbound::coude::manage_social::ManageCoudeSocialUseCase;
use crate::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::ports::outbound::coude::bounty_repository::BountyRepository;
use crate::ports::outbound::coude::coalition_repository::CoalitionRepository;
use crate::ports::outbound::coude::combat_repository::CombatRepository;
use crate::ports::outbound::coude::curses_repository::CursesRepository;
use crate::ports::outbound::coude::player_repository::PlayerRepository;
use crate::ports::outbound::coude::safety_net_repository::SafetyNetRepository;
use crate::ports::outbound::coude::ultimate_repository::UltimateRepository;
use crate::ports::outbound::coude::vendetta_repository::VendettaRepository;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
pub struct ResolveCombatNowService {
    combat_repo: Arc<dyn CombatRepository>,
    combats_uc: Arc<dyn ManageCoudeCombatsUseCase>,
    players_uc: Arc<dyn ManageCoudePlayersUseCase>,
    wallet_repo: Arc<dyn WalletRepository>,
    bets_uc: Arc<dyn ManageCoudeBetsUseCase>,
    inventory_uc: Arc<dyn ManageCoudeInventoryUseCase>,
    social_uc: Arc<dyn ManageCoudeSocialUseCase>,
    taunts_uc: Arc<dyn ManageCoudeTauntsUseCase>,
    bot_config_repo: Arc<dyn BotConfigRepository>,
    curses_repo: Option<Arc<dyn CursesRepository>>,
    safety_net_repo: Option<Arc<dyn SafetyNetRepository>>,
    vendetta_repo: Option<Arc<dyn VendettaRepository>>,
    player_repo: Option<Arc<dyn PlayerRepository>>,
    bounty_repo: Option<Arc<dyn BountyRepository>>,
    coalition_repo: Option<Arc<dyn CoalitionRepository>>,
    ultimate_repo: Option<Arc<dyn UltimateRepository>>,
}

impl ResolveCombatNowService {
    pub fn new(
        combat_repo: Arc<dyn CombatRepository>,
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
            curses_repo: None,
            safety_net_repo: None,
            vendetta_repo: None,
            player_repo: None,
            bounty_repo: None,
            coalition_repo: None,
            ultimate_repo: None,
        }
    }

    /// Branche le repo coalition (cf. COUPE_AMELIORATIONS 5.3).
    pub fn with_coalition_repo(mut self, repo: Arc<dyn CoalitionRepository>) -> Self {
        self.coalition_repo = Some(repo);
        self
    }

    /// Branche le repo ultimate (cf. COUPE_AMELIORATIONS 3.1).
    pub fn with_ultimate_repo(mut self, repo: Arc<dyn UltimateRepository>) -> Self {
        self.ultimate_repo = Some(repo);
        self
    }

    /// Branche le repo player (cf. COUPE_AMELIORATIONS 5.3) pour
    /// detecter quand un winner casse une streak >= 5 du loser et
    /// declencher la "Prime collective" (regicide bonus 1000c).
    pub fn with_player_repo(mut self, repo: Arc<dyn PlayerRepository>) -> Self {
        self.player_repo = Some(repo);
        self
    }

    /// Branche le repo bounty (cf. COUPE_AMELIORATIONS 5.3) pour gerer
    /// les primes collectives auto-ouvertes / claimees lors des combats.
    pub fn with_bounty_repo(mut self, repo: Arc<dyn BountyRepository>) -> Self {
        self.bounty_repo = Some(repo);
        self
    }

    /// Branche le repo des maledictions pour activer Banana
    /// (cf. COUPE_AMELIORATIONS 5.1) sur les d20 du combat.
    pub fn with_curses_repo(mut self, repo: Arc<dyn CursesRepository>) -> Self {
        self.curses_repo = Some(repo);
        self
    }

    /// Branche le repo du filet de securite (cf. COUPE_AMELIORATIONS 4.4)
    /// pour reduire les pertes du perdant et activer le filet quand son
    /// solde tombe sous le seuil.
    pub fn with_safety_net_repo(mut self, repo: Arc<dyn SafetyNetRepository>) -> Self {
        self.safety_net_repo = Some(repo);
        self
    }

    /// Branche le repo des vendettas (cf. COUPE_AMELIORATIONS 5.3) pour
    /// detecter les revanches en cours et appliquer le bonus +100% au
    /// gain du challenger qui gagne sa revanche, ou marquer la vendetta
    /// comme perdue dans le cas inverse.
    pub fn with_vendetta_repo(mut self, repo: Arc<dyn VendettaRepository>) -> Self {
        self.vendetta_repo = Some(repo);
        self
    }

    async fn loser_has_safety_net(&self, guild_id: &str, user_id: &str) -> bool {
        let Some(repo) = &self.safety_net_repo else { return false; };
        matches!(repo.get_active(guild_id, user_id).await, Ok(Some(_)))
    }

    async fn try_activate_safety_net_after(&self, guild_id: &str, user_id: &str) {
        let Some(repo) = &self.safety_net_repo else { return; };
        let balance = match self.wallet_repo.get(guild_id, user_id).await {
            Ok(Some(w)) => w.coins,
            _ => return,
        };
        let settings = crate::application::coude::guild_settings::GuildSettings::load(
            self.bot_config_repo.as_ref(),
            guild_id,
        )
        .await;
        let trigger = settings.get_i64("safety_net_trigger_coins", 50);
        let duration = settings.get_i64("safety_net_duration_hours", 72);
        if balance >= trigger {
            return;
        }
        // Skip si deja un filet actif (evite cumul).
        if matches!(repo.get_active(guild_id, user_id).await, Ok(Some(_))) {
            return;
        }
        if let Err(e) = repo.activate(guild_id, user_id, duration).await {
            warn!(error = %e, %user_id, "Echec activation safety_net");
        }
    }

    async fn fetch_banana(&self, guild_id: &str, user_id: &str) -> bool {
        let Some(repo) = &self.curses_repo else { return false; };
        use crate::domain::entities::coude::curse::CurseKind;
        matches!(
            repo.get_active_for_target(guild_id, user_id).await,
            Ok(Some(c)) if c.kind == CurseKind::Banana
        )
    }

    /// Consume Graisser si actif sur la cible (cf. COUPE_AMELIORATIONS 5.2).
    /// Retourne `true` si la malediction a ete trouvee + levee, ce qui doit
    /// faire foirer la prochaine attaque speciale.
    async fn consume_graisser_if_active(&self, guild_id: &str, user_id: &str) -> bool {
        let Some(repo) = &self.curses_repo else { return false; };
        use crate::domain::entities::coude::curse::CurseKind;
        match repo.get_active_for_target(guild_id, user_id).await {
            Ok(Some(c)) if c.kind == CurseKind::Graisser => {
                // Consume the curse — log et continue meme si lift echoue
                // (best-effort, le combat doit aboutir).
                if let Err(e) = repo.lift(c.id, user_id).await {
                    warn!(error = %e, %user_id, "Echec lift Graisser apres consumption");
                }
                true
            }
            _ => false,
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

        // 4. Moteur de combat (pur domain).
        //
        // Roll des mythiques (cf. COUPE_AMELIORATIONS 2.1) en amont pour
        // pouvoir appliquer les effets qui modifient les inputs du moteur
        // (ex: Magicien -> swap classes).
        let mythic_event: Option<crate::domain::entities::coude::mythic_events::MythicEvent> = {
            use crate::domain::entities::coude::mythic_events::roll_mythic_event;
            use rand::rngs::StdRng;
            use rand::SeedableRng;
            let mut myth_rng = StdRng::from_entropy();
            roll_mythic_event(&mut myth_rng)
        };

        // Mythique "Le Magicien" : echange les classes des deux combattants
        // pour ce combat seulement. Les autres stats (level, atk, def,
        // hp) restent celles du joueur d origine — seul le passif de
        // classe (Berserker/Blindage/Esquive/Vampirisme/...) est swap.
        let magician_active = matches!(&mythic_event, Some(ev) if ev.key == "magicien");
        let (atk_class_for_engine, def_class_for_engine) = if magician_active {
            (
                defender.class.as_ref().map(|c| c.as_str().to_string()),
                attacker.class.as_ref().map(|c| c.as_str().to_string()),
            )
        } else {
            (
                attacker.class.as_ref().map(|c| c.as_str().to_string()),
                defender.class.as_ref().map(|c| c.as_str().to_string()),
            )
        };
        let atk_player = PlayerLite {
            user_id: attacker.user_id.clone(),
            class: atk_class_for_engine,
            level: attacker.level,
            atk: attacker.atk,
            def: attacker.def,
            cowardice_count: attacker.cowardice_count,
            hp_current: Some(attacker.hp_current),
        };
        let def_player = PlayerLite {
            user_id: defender.user_id.clone(),
            class: def_class_for_engine,
            level: defender.level,
            atk: defender.atk,
            def: defender.def,
            cowardice_count: defender.cowardice_count,
            hp_current: Some(defender.hp_current),
        };

        // Charge les parametres de balance de la guild (fallback default
        // si bot_guild_config indispo ou vide).
        let balance = load_balance_params(self.bot_config_repo.as_ref(), &combat.guild_id).await;
        // Settings pour les features 4.1 / 3.3 / 4.4 (config par-guild,
        // cf. migration 170).
        let settings = crate::application::coude::guild_settings::GuildSettings::load(
            self.bot_config_repo.as_ref(),
            &combat.guild_id,
        )
        .await;

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

        // Saisons thematiques (cf. COUPE_AMELIORATIONS 6.3) : on derive le
        // theme du numero de saison de l attaquant (rotation deterministe).
        // - "Saison du Chaos" -> chaos events x2
        // - "Saison du Tank"  -> +20% DEF pour les Tanks
        let (season_chaos_multiplier, season_tank_def_bonus) = {
            use crate::domain::entities::coude::season_theme::season_chaos_multiplier as season_chaos;
            use crate::domain::entities::coude::season_theme::season_tank_def_bonus_pct as season_tank;
            (season_chaos(attacker.season), season_tank(attacker.season))
        };
        // Palier "Riposte fulgurante" (cf. COUPE_AMELIORATIONS 3.2) :
        // un defenseur de niveau 20+ qui se fait attaquer par un joueur
        // de niveau strictement inferieur frappe en premier au round 1.
        let defender_riposte_first_round = defender.level >= 20 && defender.level > attacker.level;

        let curses = engine::combat::CombatCurses {
            attacker_has_banana: self.fetch_banana(&combat.guild_id, &combat.attacker_id).await,
            defender_has_banana: self.fetch_banana(&combat.guild_id, &combat.defender_id).await,
            chaos_multiplier: season_chaos_multiplier,
            tank_def_bonus_pct: season_tank_def_bonus,
            defender_riposte_first_round,
        };

        // Sabotage "Graisser les armes" (cf. COUPE_AMELIORATIONS 5.2) :
        // si l attaquant ou le defenseur est sous l effet, sa prochaine
        // attaque speciale foire (override a None) et le sabotage est
        // consume.
        let attacker_special_raw = combat.special_attack.as_deref();
        let defender_special_raw = combat.defender_special.as_deref();
        let mut graisser_msgs: Vec<String> = Vec::new();
        let attacker_special_effective = if attacker_special_raw.is_some()
            && self.consume_graisser_if_active(&combat.guild_id, &combat.attacker_id).await
        {
            graisser_msgs.push(format!(
                "\u{1f6e2}\u{fe0f} **{}** : son arme etait graissee, l attaque speciale foire !",
                combat.attacker_name
            ));
            None
        } else {
            attacker_special_raw
        };
        let defender_special_effective = if defender_special_raw.is_some()
            && self.consume_graisser_if_active(&combat.guild_id, &combat.defender_id).await
        {
            graisser_msgs.push(format!(
                "\u{1f6e2}\u{fe0f} **{}** : son arme etait graissee, la riposte foire !",
                combat.defender_name
            ));
            None
        } else {
            defender_special_raw
        };

        // Ultimates pre-combat (cf. COUPE_AMELIORATIONS 3.1) :
        // - Bourrin : swap HP avant l engine
        // - Tank : court-circuit, victoire forfait
        // - Agile : court-circuit, 50/50 pur
        let mut ultimate_msg: Option<String> = None;
        let mut shortcut_result: Option<engine::combat::CombatResult> = None;
        let (atk_hp_for_engine, def_hp_for_engine) = {
            let mut a = attacker.hp_current;
            let mut d = defender.hp_current;
            if let Some(ult_repo) = &self.ultimate_repo {
                use crate::domain::entities::coude::ultimate::UltimateKind;
                if let Ok(state) = ult_repo.get(&combat.guild_id, &combat.attacker_id).await {
                    match state.pending_kind {
                        Some(UltimateKind::Bourrin) => {
                            std::mem::swap(&mut a, &mut d);
                            ultimate_msg = Some(format!(
                                "\u{1f504} **Echange de carcasses** : <@{}> swap son HP ({} -> {}) avec <@{}> ({} -> {}) !",
                                combat.attacker_id, attacker.hp_current, d,
                                combat.defender_id, defender.hp_current, a
                            ));
                            if let Err(e) = ult_repo
                                .consume_pending(&combat.guild_id, &combat.attacker_id)
                                .await
                            {
                                warn!(error = %e, "Echec consume ultimate Bourrin");
                            }
                        }
                        Some(UltimateKind::Tank) => {
                            shortcut_result = Some(engine::combat::CombatResult {
                                winner_id: Some(combat.attacker_id.clone()),
                                loser_id: Some(combat.defender_id.clone()),
                                rounds: vec![],
                                total_rounds: 10,
                                attacker_hp_final: attacker.hp_current,
                                defender_hp_final: defender.hp_current,
                                attacker_hp_max: attacker.hp_max,
                                defender_hp_max: defender.hp_max,
                                chaos_events_count: 0,
                                coins_won: combat.mise,
                                coins_lost_by_loser: combat.mise,
                                stolen_bonus: 0,
                                vol_coins: 0,
                                message: format!(
                                    "\u{1f9f1} **STATUE** : <@{}> ne bouge pas. <@{}> attend... attend... finit par partir chercher un sandwich. Forfait au bout de 10 rounds.",
                                    combat.attacker_id, combat.defender_id
                                ),
                                is_giant_killer: false,
                                attacker_class_revealed: Some("tank".into()),
                                defender_class_revealed: None,
                            });
                            if let Err(e) = ult_repo
                                .consume_pending(&combat.guild_id, &combat.attacker_id)
                                .await
                            {
                                warn!(error = %e, "Echec consume ultimate Tank");
                            }
                        }
                        Some(UltimateKind::Agile) => {
                            use rand::Rng;
                            let heads = {
                                let mut rng = rand::thread_rng();
                                rng.gen_bool(0.5)
                            };
                            let (winner_id, loser_id) = if heads {
                                (combat.attacker_id.clone(), combat.defender_id.clone())
                            } else {
                                (combat.defender_id.clone(), combat.attacker_id.clone())
                            };
                            shortcut_result = Some(engine::combat::CombatResult {
                                winner_id: Some(winner_id.clone()),
                                loser_id: Some(loser_id.clone()),
                                rounds: vec![],
                                total_rounds: 1,
                                attacker_hp_final: if heads { attacker.hp_current } else { 0 },
                                defender_hp_final: if heads { 0 } else { defender.hp_current },
                                attacker_hp_max: attacker.hp_max,
                                defender_hp_max: defender.hp_max,
                                chaos_events_count: 0,
                                coins_won: combat.mise,
                                coins_lost_by_loser: combat.mise,
                                stolen_bonus: 0,
                                vol_coins: 0,
                                message: format!(
                                    "\u{1fa99} **PILE OU FACE** : la piece tombe. <@{}> gagne. <@{}> a juste regarde la piece tomber. Pas de combat, pas d explication.",
                                    winner_id, loser_id
                                ),
                                is_giant_killer: false,
                                attacker_class_revealed: Some("agile".into()),
                                defender_class_revealed: None,
                            });
                            if let Err(e) = ult_repo
                                .consume_pending(&combat.guild_id, &combat.attacker_id)
                                .await
                            {
                                warn!(error = %e, "Echec consume ultimate Agile");
                            }
                        }
                        Some(UltimateKind::Fourbe) => {
                            // Le Fuyard : vol la mise du defenseur AVANT le
                            // combat et te barre. Transfer atomique
                            // defender -> attacker pour le montant de la
                            // mise. Combat marque resolu sans HP perdu.
                            let stolen = combat.mise;
                            let transfer_ok = self
                                .wallet_repo
                                .transfer(
                                    &combat.guild_id,
                                    &combat.defender_id,
                                    &combat.attacker_id,
                                    stolen,
                                    "ultimate_fourbe_fuyard",
                                    "Ultimate Le Fuyard — vol pre-combat",
                                )
                                .await
                                .is_ok();
                            // Si le transfer echoue (solde insuffisant
                            // defendeur), on annule l ultimate et on
                            // laisse le combat se derouler normalement.
                            if transfer_ok {
                                shortcut_result = Some(engine::combat::CombatResult {
                                    winner_id: Some(combat.attacker_id.clone()),
                                    loser_id: Some(combat.defender_id.clone()),
                                    rounds: vec![],
                                    total_rounds: 0,
                                    attacker_hp_final: attacker.hp_current,
                                    defender_hp_final: defender.hp_current,
                                    attacker_hp_max: attacker.hp_max,
                                    defender_hp_max: defender.hp_max,
                                    chaos_events_count: 0,
                                    // Le transfer a deja eu lieu hors-engine.
                                    // On force a 0 pour eviter un double payout
                                    // dans le pipeline post-engine.
                                    coins_won: 0,
                                    coins_lost_by_loser: 0,
                                    stolen_bonus: 0,
                                    vol_coins: 0,
                                    message: format!(
                                        "\u{1f3c3} **LE FUYARD** : <@{}> rafle la mise ({}c) AVANT le combat et se barre. <@{}> recoit un message « ton adversaire a fui avec la caisse ».",
                                        combat.attacker_id, stolen, combat.defender_id
                                    ),
                                    is_giant_killer: false,
                                    attacker_class_revealed: Some("fourbe".into()),
                                    defender_class_revealed: None,
                                });
                                if let Err(e) = ult_repo
                                    .consume_pending(&combat.guild_id, &combat.attacker_id)
                                    .await
                                {
                                    warn!(error = %e, "Echec consume ultimate Fourbe");
                                }
                            } else {
                                // Transfer echoue : log et on n active PAS
                                // l ultimate (le combat continue normalement,
                                // pendant_kind reste posee pour reessai).
                                warn!("Echec transfer Fourbe : solde defenseur insuffisant ?");
                            }
                        }
                        _ => {}
                    }
                }
            }
            (a, d)
        };

        let mut result = if let Some(short) = shortcut_result {
            short
        } else {
            engine::combat::resolve_combat_with_curses(
                &atk_player,
                &def_player,
                atk_hp_for_engine,
                def_hp_for_engine,
                combat.mise,
                attacker_special_effective,
                defender_special_effective,
                &engine_events,
                &balance,
                curses,
            )
        };

        // Prefix les messages Graisser + ultimate dans la description.
        if !graisser_msgs.is_empty() {
            result.message = format!("{}\n\n{}", graisser_msgs.join("\n"), result.message);
        }
        if let Some(u_msg) = &ultimate_msg {
            result.message = format!("{}\n\n{}", u_msg, result.message);
        }

        // Effets mecaniques des mythiques (cf. COUPE_AMELIORATIONS 2.1).
        // Le roll a deja eu lieu plus haut pour permettre Magicien
        // (swap classes pre-engine).
        if let Some(ev) = &mythic_event {
            match ev.key {
                // Invasion de poulets : combat annule, match nul force,
                // zero transfert de coins. Le pipeline plus bas gere deja
                // le cas (None, None) comme un draw.
                "invasion_poulets" => {
                    result.winner_id = None;
                    result.loser_id = None;
                    result.coins_won = 0;
                    result.coins_lost_by_loser = 0;
                    result.stolen_bonus = 0;
                    result.vol_coins = 0;
                }
                // Aliens (light) : abduction brievement, les deux reviennent
                // sonnes a 1 HP. Match nul force, zero transfert. Le report
                // 24h promis dans le folklore reste a faire (out of scope).
                "aliens" => {
                    result.winner_id = None;
                    result.loser_id = None;
                    result.coins_won = 0;
                    result.coins_lost_by_loser = 0;
                    result.stolen_bonus = 0;
                    result.vol_coins = 0;
                    result.attacker_hp_final = 1;
                    result.defender_hp_final = 1;
                }
                // Distributeur de PQ : un gagnant et un perdant sont bien
                // declares mais le pot devient du PQ — aucun transfert.
                "distributeur_pq" => {
                    result.coins_won = 0;
                    result.coins_lost_by_loser = 0;
                    result.stolen_bonus = 0;
                    result.vol_coins = 0;
                }
                // Licorne rose : match nul force + 500c bonus pour chaque
                // combattant (cree de la masse monetaire — le serveur paye).
                // Le credit wallet est applique en best-effort, hors-tx :
                // si l API casse, on n a pas perdu d argent (les wallets
                // d origine sont intacts).
                "licorne_rose" => {
                    result.winner_id = None;
                    result.loser_id = None;
                    result.coins_won = 0;
                    result.coins_lost_by_loser = 0;
                    result.stolen_bonus = 0;
                    result.vol_coins = 0;
                    for uid in [&combat.attacker_id, &combat.defender_id] {
                        if let Err(e) = self
                            .wallet_repo
                            .credit(
                                &combat.guild_id,
                                uid,
                                500,
                                "mythic_licorne_rose",
                                "Bonus Licorne rose",
                            )
                            .await
                        {
                            warn!(error = %e, user_id = %uid, "Echec credit Licorne rose");
                        }
                    }
                }
                // Revanche d outre-tombe : le perdant ressuscite et vole
                // 30% des coins du gagnant. On force un match nul
                // (skip le pipeline atomique de paiement combat) et on
                // execute directement un transfert 30%-balance du
                // gagnant vers le perdant.
                "revanche_outre_tombe" => {
                    let original_winner = result.winner_id.clone();
                    let original_loser = result.loser_id.clone();
                    result.winner_id = None;
                    result.loser_id = None;
                    result.coins_won = 0;
                    result.coins_lost_by_loser = 0;
                    result.stolen_bonus = 0;
                    result.vol_coins = 0;
                    if let (Some(w_id), Some(l_id)) = (original_winner, original_loser) {
                        let w_balance = match self.wallet_repo.get(&combat.guild_id, &w_id).await {
                            Ok(Some(w)) => w.coins,
                            _ => 0,
                        };
                        let stolen = (w_balance as f64 * 0.30) as i64;
                        if stolen > 0 {
                            if let Err(e) = self
                                .wallet_repo
                                .transfer(
                                    &combat.guild_id,
                                    &w_id,
                                    &l_id,
                                    stolen,
                                    "mythic_revanche_outre_tombe",
                                    "Revanche d outre-tombe (vol 30%)",
                                )
                                .await
                            {
                                warn!(error = %e, "Echec transfert Revanche outre-tombe");
                            }
                        }
                    }
                }
                // Jackpot divin : le combat est resolu normalement (winner /
                // loser conserves) mais le winner touche en plus un bonus
                // de 10 * mise depuis le neant (le serveur paye, pas la
                // cagnotte — interpretation simplifiee). Best-effort credit.
                "jackpot_divin" => {
                    if let Some(winner_id) = result.winner_id.clone() {
                        let bonus = combat.mise.saturating_mul(10);
                        if bonus > 0 {
                            if let Err(e) = self
                                .wallet_repo
                                .credit(
                                    &combat.guild_id,
                                    &winner_id,
                                    bonus,
                                    "mythic_jackpot_divin",
                                    "Jackpot divin x10 mise",
                                )
                                .await
                            {
                                warn!(error = %e, user_id = %winner_id, "Echec credit Jackpot divin");
                            }
                        }
                    }
                }
                // Etoile filante : les deux combattants ressuscitent a 100%
                // HP — interpretation simplifiee, pas de sudden death
                // re-execute (necessiterait de rerun le moteur). On force
                // un match nul, on retablit les HP finaux a hp_max et zero
                // transfert. Le narratif annonce explique la "magie".
                "etoile_filante" => {
                    result.winner_id = None;
                    result.loser_id = None;
                    result.coins_won = 0;
                    result.coins_lost_by_loser = 0;
                    result.stolen_bonus = 0;
                    result.vol_coins = 0;
                    result.attacker_hp_final = result.attacker_hp_max;
                    result.defender_hp_final = result.defender_hp_max;
                }
                // Trefle a 4 feuilles : le combat est resolu normalement
                // (winner_id / loser_id conserves, win streak incremente),
                // mais le perdant recupere 150% de sa mise au lieu d en
                // perdre. Le gagnant ne touche rien (la cagnotte cosmique
                // sponsorise). Operationnellement : zero transfert atomique
                // (coins_won / coins_lost forces a 0), puis credit du loser
                // 1.5 * mise depuis le neant.
                "trefle_quatre_feuilles" => {
                    let bonus = (combat.mise as f64 * 1.5) as i64;
                    result.coins_won = 0;
                    result.coins_lost_by_loser = 0;
                    result.stolen_bonus = 0;
                    result.vol_coins = 0;
                    if let Some(loser_id) = result.loser_id.clone() {
                        if bonus > 0 {
                            if let Err(e) = self
                                .wallet_repo
                                .credit(
                                    &combat.guild_id,
                                    &loser_id,
                                    bonus,
                                    "mythic_trefle",
                                    "Trefle a 4 feuilles",
                                )
                                .await
                            {
                                warn!(error = %e, user_id = %loser_id, "Echec credit Trefle");
                            }
                        }
                    }
                }
                // Bombe nucleaire : annihilation totale, les deux perdent
                // 50% de leur wallet, le combat est marque match nul. Read
                // le solde courant pour calculer 50% precis. Best-effort.
                "bombe_nucleaire" => {
                    result.winner_id = None;
                    result.loser_id = None;
                    result.coins_won = 0;
                    result.coins_lost_by_loser = 0;
                    result.stolen_bonus = 0;
                    result.vol_coins = 0;
                    for uid in [&combat.attacker_id, &combat.defender_id] {
                        let balance = match self.wallet_repo.get(&combat.guild_id, uid).await {
                            Ok(Some(w)) => w.coins,
                            _ => 0,
                        };
                        let to_debit = balance / 2;
                        if to_debit > 0 {
                            if let Err(e) = self
                                .wallet_repo
                                .debit(
                                    &combat.guild_id,
                                    uid,
                                    to_debit,
                                    "mythic_bombe_nucleaire",
                                    "Bombe nucleaire mythique",
                                )
                                .await
                            {
                                warn!(error = %e, user_id = %uid, "Echec debit Bombe nucleaire");
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let first_atk_roll = result.rounds.first().map(|r| r.attacker_roll).unwrap_or(0);
        let first_def_roll = result.rounds.first().map(|r| r.defender_roll).unwrap_or(0);
        let chaos_key = result
            .rounds
            .iter()
            .find_map(|r| r.chaos_event)
            .map(|c| c.key().to_string());

        // Sprint 1 (2.3) — Detection des moments memorables (Clutch /
        // Comeback / Perfect / Ridicule / Zero pointe).
        let outcome_flags = compute_outcome_flags_from_result(&result, &combat);

        // 5. Persister le combat
        self.combat_repo
            .resolve(
                combat.id,
                crate::domain::entities::coude::combat::CombatResolution {
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
        let mut bets_draw_taunts: Vec<crate::domain::entities::coude::taunt::TauntEvent> = Vec::new();
        if result.chaos_events_count > 0 {
            title_color = 0x9B59B6;
        }

        let mut vendetta_msg: Option<String> = None;
        let mut vendetta_humiliation: Option<crate::ports::inbound::coude::resolve_combat_now::VendettaHumiliation> = None;
        // Bouclier malchance (4.1) : true si la 1ere defaite du jour a
        // ete adoucie. Visible aux deux match blocks (payout + streaks).
        let mut shield_active = false;
        match (&result.winner_id, &result.loser_id) {
            (Some(winner_id), Some(loser_id)) => {
                // Cap sur solde reel du perdant (pre-requis pour l'assurance
                // qui clamp d'abord, cf. Flow B dans apply_insurance_to_loss).
                let loser_wallet = self
                    .wallet_repo
                    .get(&combat.guild_id, loser_id)
                    .await
                    .ok()
                    .flatten();
                let loser_balance = loser_wallet.map(|w| w.coins).unwrap_or(0);
                let coins_transferred_nominal = result.coins_won.min(loser_balance);

                // Vendetta (cf. COUPE_AMELIORATIONS 5.3) : si le winner a
                // une vendetta active contre le loser, c est sa revanche
                // — gain double. Inversement si le loser avait declare une
                // vendetta contre le winner, on la resout comme perdue.
                use crate::domain::entities::coude::vendetta::apply_revenge_bonus;
                let coins_transferred = if let Some(repo) = &self.vendetta_repo {
                    if let Ok(Some(v)) = repo.get_active(&combat.guild_id, winner_id, loser_id).await {
                        let boosted = apply_revenge_bonus(coins_transferred_nominal, true);
                        if let Err(e) = repo.resolve(v.id, true).await {
                            warn!(error = %e, "Echec resolve vendetta won");
                        }
                        if boosted > coins_transferred_nominal {
                            vendetta_msg = Some(format!(
                                "\u{2694}\u{fe0f} **VENDETTA ACCOMPLIE !** Gain double : {} -> {} coins.",
                                coins_transferred_nominal, boosted
                            ));
                        }
                        boosted
                    } else {
                        // Verifie si le perdant avait une vendetta contre
                        // le gagnant : il vient de la perdre.
                        if let Ok(Some(v)) = repo
                            .get_active(&combat.guild_id, loser_id, winner_id)
                            .await
                        {
                            if let Err(e) = repo.resolve(v.id, false).await {
                                warn!(error = %e, "Echec resolve vendetta lost");
                            }
                            vendetta_msg = Some(format!(
                                "\u{1faa6} Vendetta de <@{}> ECHOUEE — il est de nouveau ecrase par <@{}>.",
                                loser_id, winner_id
                            ));
                            // Le bot va renommer le winner "le Bourreau
                            // de @loser" pendant 7 jours (cf. roadmap 5.3).
                            vendetta_humiliation = Some(
                                crate::ports::inbound::coude::resolve_combat_now::VendettaHumiliation {
                                    target_user_id: winner_id.clone(),
                                    challenger_user_id: loser_id.clone(),
                                },
                            );
                        }
                        coins_transferred_nominal
                    }
                } else {
                    coins_transferred_nominal
                };

                // Coalition active contre le winner (cf. COUPE_AMELIORATIONS
                // 5.3) : ses gains de combat sont reduits a 80% (perte
                // de 20% absorbee par le neant). Si le loser est membre
                // d une coalition contre le winner, on flag aussi pour
                // potentiel auto-break ci-dessous.
                let mut coalition_msg: Option<String> = None;
                let mut break_coalition_id: Option<uuid::Uuid> = None;
                let coins_transferred = if let Some(coalition_repo) = &self.coalition_repo {
                    use crate::domain::entities::coude::coalition::apply_coalition_penalty;
                    use crate::domain::entities::coude::coalition::COALITION_GAIN_MULTIPLIER;
                    // Penalite si winner est cible d une coalition active.
                    let winner_in_coalition = coalition_repo
                        .get_active(&combat.guild_id, winner_id)
                        .await
                        .ok()
                        .flatten()
                        .filter(|c| c.is_active_at(chrono::Utc::now()));
                    let after_coalition = if let Some(c) = &winner_in_coalition {
                        // Le winner est cible. Loser membre ? -> auto-break.
                        if c.members.iter().any(|m| m.member_id == *loser_id) {
                            break_coalition_id = Some(c.id);
                        }
                        let reduced = apply_coalition_penalty(coins_transferred, true);
                        if reduced < coins_transferred {
                            coalition_msg = Some(format!(
                                "\u{1f5e1}\u{fe0f} Coalition active : gain reduit a {}% ({} -> {} coins).",
                                (COALITION_GAIN_MULTIPLIER * 100.0) as i32,
                                coins_transferred,
                                reduced
                            ));
                        }
                        reduced
                    } else {
                        coins_transferred
                    };
                    after_coalition
                } else {
                    coins_transferred
                };
                if let Some(c_msg) = coalition_msg {
                    insurance_msg = match insurance_msg {
                        Some(prev) => Some(format!("{prev}\n{c_msg}")),
                        None => Some(c_msg),
                    };
                }

                // Bonus de prestige (cf. COUPE_AMELIORATIONS 3.3) :
                // +5% par prestige du gagnant sur le payout combat.
                // Lecture via player_repo si dispo.
                let coins_transferred = if let Some(prepo) = &self.player_repo {
                    use crate::domain::entities::coude::prestige::prestige_gain_multiplier_with_params;
                    let prestige_count = prepo
                        .get_prestige_count(&combat.guild_id, winner_id)
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or(0);
                    let bonus_pct = settings.get_percent_ratio("prestige_gain_bonus_percent", 5);
                    let max_count = settings.get_i32("prestige_max_count", 5);
                    let mult = prestige_gain_multiplier_with_params(
                        prestige_count, bonus_pct, max_count,
                    );
                    if mult > 1.0 && coins_transferred > 0 {
                        let boosted =
                            ((coins_transferred as f64) * mult).round() as i64;
                        let prestige_msg = format!(
                            "\u{2728} Bonus prestige (x{:.2}) : {} -> {} coins.",
                            mult, coins_transferred, boosted
                        );
                        insurance_msg = match insurance_msg {
                            Some(prev) => Some(format!("{prev}\n{prestige_msg}")),
                            None => Some(prestige_msg),
                        };
                        boosted
                    } else {
                        coins_transferred
                    }
                } else {
                    coins_transferred
                };
                // Auto-break de la coalition : la cible vient de battre
                // un de ses conspirateurs.
                if let (Some(coalition_repo), Some(coalition_id)) =
                    (&self.coalition_repo, break_coalition_id)
                {
                    if let Err(e) = coalition_repo.mark_broken(coalition_id, winner_id).await {
                        warn!(error = %e, "Echec mark_broken coalition");
                    } else {
                        let break_msg = format!(
                            "\u{1f4a5} **Coalition brisee !** <@{}> a battu <@{}> et leve la coalition contre lui. Les gains reprennent a 100%.",
                            winner_id, loser_id
                        );
                        insurance_msg = match insurance_msg {
                            Some(prev) => Some(format!("{prev}\n{break_msg}")),
                            None => Some(break_msg),
                        };
                    }
                }

                // Sabotage Empoisonner (cf. COUPE_AMELIORATIONS 5.2) :
                // si le winner est sous l effet, 10% de son gain est
                // redirige vers le saboteur. Consume une utilisation.
                let mut poison_msg: Option<String> = None;
                if let Some(curses_repo) = &self.curses_repo {
                    use crate::domain::entities::coude::curse::poison_redirect_amount;
                    use crate::domain::entities::coude::curse::CurseKind;
                    if let Ok(Some(c)) = curses_repo
                        .get_active_for_target(&combat.guild_id, winner_id)
                        .await
                    {
                        if c.kind == CurseKind::Empoisonner {
                            let redirect = poison_redirect_amount(coins_transferred, true);
                            if redirect > 0 {
                                if let Err(e) = self
                                    .wallet_repo
                                    .credit(
                                        &combat.guild_id,
                                        &c.source_id,
                                        redirect,
                                        "poison_wallet_redirect",
                                        "Sabotage Empoisonner — gain redirige",
                                    )
                                    .await
                                {
                                    warn!(error = %e, "Echec credit Empoisonner");
                                }
                                if let Err(e) = curses_repo.consume_one_use(c.id).await {
                                    warn!(error = %e, "Echec consume Empoisonner");
                                }
                                poison_msg = Some(format!(
                                    "\u{2620}\u{fe0f} **Wallet empoisonne** : {}c (10%) du gain de <@{}> redirige vers <@{}>.",
                                    redirect, winner_id, c.source_id
                                ));
                            }
                        }
                    }
                }
                if let Some(p_msg) = poison_msg {
                    insurance_msg = match insurance_msg {
                        Some(prev) => Some(format!("{prev}\n{p_msg}")),
                        None => Some(p_msg),
                    };
                }

                // Assurance : clamp-then-apply dans le domain pour que les
                // joueurs fauches beneficient effectivement de la protection.
                let mut active_insurance = self
                    .inventory_uc
                    .get_active_insurance(&combat.guild_id, loser_id)
                    .await
                    .ok()
                    .flatten();

                // Sabotage Fausse assurance (cf. COUPE_AMELIORATIONS 5.2) :
                // si le loser est sous l effet ET avait une assurance
                // active, l assurance est annulee + 200c additionnels
                // sont preleves vers le saboteur. La curse est consumee.
                if let (Some(curses_repo), Some(_ins)) = (&self.curses_repo, &active_insurance) {
                    use crate::domain::entities::coude::curse::CurseKind;
                    use crate::domain::entities::coude::curse::FAUSSE_ASSURANCE_FEE_COINS;
                    if let Ok(Some(c)) = curses_repo
                        .get_active_for_target(&combat.guild_id, loser_id)
                        .await
                    {
                        if c.kind == CurseKind::FausseAssurance {
                            // Nullifie la protection : le clamp-then-apply
                            // partira en flux "no insurance".
                            active_insurance = None;
                            // Frais additionnels via transfer atomique
                            // loser -> saboteur. Hors-tx mais ok : si
                            // echec on a juste pas de tax (loser garde
                            // ses coins).
                            if let Err(e) = self
                                .wallet_repo
                                .transfer(
                                    &combat.guild_id,
                                    loser_id,
                                    &c.source_id,
                                    FAUSSE_ASSURANCE_FEE_COINS,
                                    "fausse_assurance_fee",
                                    "Sabotage Fausse assurance — frais",
                                )
                                .await
                            {
                                warn!(error = %e, "Echec transfer fausse assurance");
                            }
                            if let Err(e) = curses_repo.consume_one_use(c.id).await {
                                warn!(error = %e, "Echec consume FausseAssurance");
                            }
                            let scam_msg = format!(
                                "\u{1f3ad} **Fausse assurance** : <@{}> decouvre que son contrat etait un scam de <@{}> ! Aucune reduction + {}c de frais preleves.",
                                loser_id, c.source_id, FAUSSE_ASSURANCE_FEE_COINS
                            );
                            insurance_msg = match insurance_msg {
                                Some(prev) => Some(format!("{prev}\n{scam_msg}")),
                                None => Some(scam_msg),
                            };
                        }
                    }
                }

                let mut adj = apply_insurance_to_loss(
                    result.coins_lost_by_loser,
                    loser_balance,
                    active_insurance.as_ref(),
                    loser_id,
                );
                if let Some(ins_id) = adj.consumed_insurance_id {
                    if let Err(e) = self.inventory_uc.expire_insurance(ins_id).await {
                        warn!(error = %e, insurance_id = %ins_id, "Echec expire_insurance : reduction non appliquee");
                        adj = apply_insurance_to_loss(
                            result.coins_lost_by_loser,
                            loser_balance,
                            None,
                            loser_id,
                        );
                    }
                }
                // Bug fix : concatener le message scam FausseAssurance (mis
                // plus haut quand le malus est actif) avec le message de
                // reduction d'assurance. L'ecrasement precedent perdait le
                // scam alert quand les deux survenaient sur le meme combat.
                insurance_msg = match (insurance_msg.take(), adj.message) {
                    (Some(prev), Some(adj_msg)) => Some(format!("{prev}\n{adj_msg}")),
                    (Some(prev), None) => Some(prev),
                    (None, adj_msg) => adj_msg,
                };

                // Sprint 1 (4.1) — bouclier malchance : si c est la 1ere
                // defaite du jour, perte * 0.5. Recommande pour eviter la
                // spirale "j ai perdu une fois, je quitte".
                let is_first_defeat_today = self
                    .combat_repo
                    .count_defeats_today(&combat.guild_id, loser_id)
                    .await
                    .unwrap_or(0)
                    == 0;
                // Bouclier malchance : enabled + multiplicateur configurables.
                let shield_enabled = settings.get_bool("lucky_shield_enabled", true);
                let shield_mult = settings.get_percent_ratio("lucky_shield_loss_percent", 50);
                let actual_loss = if shield_enabled {
                    crate::domain::entities::coude::lucky_shield::apply_lucky_shield_with_multiplier(
                        adj.actual_loss, is_first_defeat_today, shield_mult,
                    )
                } else {
                    adj.actual_loss
                };
                shield_active = is_first_defeat_today && actual_loss < adj.actual_loss;
                if shield_active {
                    let shield_msg = format!(
                        "\u{1f49a} Bouclier malchance du jour : perte reduite de {} a {} (win streak preservee).",
                        adj.actual_loss, actual_loss
                    );
                    insurance_msg = match insurance_msg {
                        Some(prev) => Some(format!("{prev}\n{shield_msg}")),
                        None => Some(shield_msg),
                    };
                }

                // Filet de securite (cf. COUPE_AMELIORATIONS 4.4) : si le
                // perdant a un filet actif, sa perte est divisee par 2.
                let has_safety_net = self.loser_has_safety_net(&combat.guild_id, loser_id).await;
                let actual_loss = if has_safety_net {
                    use crate::domain::entities::coude::safety_net::reduce_loss_with_multiplier as safety_net_reduce_loss_with_multiplier;
                    let net_mult = settings.get_percent_ratio("safety_net_loss_percent", 50);
                    let reduced = safety_net_reduce_loss_with_multiplier(actual_loss, true, net_mult);
                    if reduced < actual_loss {
                        let net_msg = format!(
                            "\u{1f49a} Filet de securite : perte reduite de {} a {}.",
                            actual_loss, reduced
                        );
                        insurance_msg = match insurance_msg {
                            Some(prev) => Some(format!("{prev}\n{net_msg}")),
                            None => Some(net_msg),
                        };
                    }
                    reduced
                } else {
                    actual_loss
                };

                // Payout atomique : credit winner + debit loser dans la meme
                // tx Postgres. Evite les etats partiels si le processus crash
                // entre les deux operations (bug #1 de l audit).
                let desc = format!("Combat {} vs {}", winner_id, loser_id);
                let mut payout_ok = true;
                if coins_transferred > 0 || actual_loss > 0 {
                    if let Err(e) = self
                        .wallet_repo
                        .pay_combat_atomic(
                            &combat.guild_id,
                            winner_id,
                            coins_transferred,
                            loser_id,
                            actual_loss,
                            "coude_combat",
                            &desc,
                        )
                        .await
                    {
                        // error! (et non warn!) : c est une desync wallet
                        // critique. Le combat est pose mais l argent n a pas
                        // bouge. On evite d enregistrer les stats pour ne pas
                        // creer un total_won/total_lost incoherent vs wallet.
                        tracing::error!(
                            error = %e,
                            combat_id = %combat.id,
                            "Echec payout combat atomique — stats non enregistrees pour preserver la coherence"
                        );
                        payout_ok = false;
                    }
                }

                // Filet de securite (cf. COUPE_AMELIORATIONS 4.4) : apres
                // le payout, si le solde du perdant tombe sous 50c et qu il
                // n a pas deja un filet actif, on l active. Best-effort.
                self.try_activate_safety_net_after(&combat.guild_id, loser_id).await;

                // Stats : seulement si le payout a reussi (eviter desync).
                if payout_ok {
                    if let Err(e) = self
                        .players_uc
                        .record_win(&combat.guild_id, winner_id, coins_transferred, result.stolen_bonus)
                        .await
                    {
                        tracing::error!(error = %e, "Echec record_win");
                    }
                    if let Err(e) = self
                        .players_uc
                        .record_loss(&combat.guild_id, loser_id, actual_loss)
                        .await
                    {
                        tracing::error!(error = %e, "Echec record_loss");
                    }
                }

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
                    let loser_display: &str = if *winner_id == combat.attacker_id {
                        &combat.defender_id
                    } else {
                        &combat.attacker_id
                    };
                    if let Some(lines) = format_bet_payout_lines(
                        &outcome.plan, Some(winner_id), Some(loser_display),
                    ) {
                        fields.push(ResolvedCombatEmbedField {
                            name: "\u{1f3b2} Paris".into(),
                            value: lines,
                            inline: false,
                        });
                    }
                }
            }
            _ => {
                // Draw / accident_debile / explosion
                let had_accident = result.rounds.iter().any(|r| {
                    matches!(
                        r.chaos_event,
                        Some(engine::chaos::ChaosEvent::AccidentDebile)
                    )
                });
                let is_explosion = combat.defender_special.as_deref() == Some("explosion");

                if is_explosion && result.coins_lost_by_loser > 0 {
                    // Explosion : les deux joueurs perdent `coins_lost_by_loser`
                    // (calcule par le moteur : 50% de la mise). On debite le
                    // wallet explicitement — sans ce debit, le message "EXPLOSION,
                    // les deux perdent X coins" ne correspondait a rien en BDD.
                    let desc = format!("Explosion combat {}", combat.id);
                    let loss = result.coins_lost_by_loser;
                    if let Err(e) = self
                        .wallet_repo
                        .debit(&combat.guild_id, &combat.attacker_id, loss, "coude_combat_explosion", &desc)
                        .await
                    {
                        tracing::error!(error = %e, attacker = %combat.attacker_id, %loss, "Echec debit explosion attaquant — desync embed/wallet");
                    }
                    if let Err(e) = self
                        .wallet_repo
                        .debit(&combat.guild_id, &combat.defender_id, loss, "coude_combat_explosion", &desc)
                        .await
                    {
                        tracing::error!(error = %e, defender = %combat.defender_id, %loss, "Echec debit explosion defenseur — desync embed/wallet");
                    }
                    if let Err(e) = self
                        .players_uc
                        .record_draw(&combat.guild_id, &combat.attacker_id, loss)
                        .await
                    {
                        tracing::error!(error = %e, "Echec record_draw explosion attaquant");
                    }
                    if let Err(e) = self
                        .players_uc
                        .record_draw(&combat.guild_id, &combat.defender_id, loss)
                        .await
                    {
                        tracing::error!(error = %e, "Echec record_draw explosion defenseur");
                    }
                }

                if had_accident {
                    // Accident debile : les deux joueurs sont penalises de
                    // `combat.mise`. On debite le wallet explicitement (avant,
                    // record_draw faisait le debit en interne — plus le cas
                    // depuis la migration #3 wallet).
                    let desc = format!("Accident debile combat {}", combat.id);
                    if combat.mise > 0 {
                        if let Err(e) = self
                            .wallet_repo
                            .debit(&combat.guild_id, &combat.attacker_id, combat.mise, "coude_combat_draw", &desc)
                            .await
                        {
                            tracing::error!(error = %e, "Echec debit accident attaquant — desync embed/wallet");
                        }
                        if let Err(e) = self
                            .wallet_repo
                            .debit(&combat.guild_id, &combat.defender_id, combat.mise, "coude_combat_draw", &desc)
                            .await
                        {
                            tracing::error!(error = %e, "Echec debit accident defenseur — desync embed/wallet");
                        }
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
                    if let Some(lines) = format_bet_payout_lines(&outcome.plan, None, None) {
                        fields.push(ResolvedCombatEmbedField {
                            name: "\u{1f3b2} Paris".into(),
                            value: lines,
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
        if let Some(v_msg) = vendetta_msg {
            fields.push(ResolvedCombatEmbedField {
                name: "\u{2694}\u{fe0f} Vendetta".into(),
                value: v_msg,
                inline: false,
            });
        }
        // Bug fix : le field Regicide etait pousse AVANT que `regicide_msg`
        // soit calcule (Phase 9 Part D, plus bas), donc jamais affiche. On
        // pousse maintenant directement apres le calcul, voir plus bas.

        // Phase 9 Part D : track streaks + collecte taunt events.
        let mut taunt_events = Vec::new();
        match (&result.winner_id, &result.loser_id) {
            (Some(winner_id), Some(loser_id)) => {
                // Prime collective / Regicide (cf. COUPE_AMELIORATIONS 5.3).
                //
                // Etape 1 (pre-touch) : si le perdant avait une streak >= 5
                // ET une bounty ouverte sur sa tete, on la claim et on
                // credit le total au gagnant. Si pas de bounty mais
                // streak >= 5 (cas legacy / pre-bounty), fallback bonus
                // fixe pour preserver le comportement.
                use crate::domain::entities::coude::bounty::BOUNTY_AUTO_OPEN_STREAK_THRESHOLD;
                use crate::domain::entities::coude::bounty::BOUNTY_INITIAL_AMOUNT;
                let mut loser_pre_streak: Option<i32> = None;
                if let Some(repo) = &self.player_repo {
                    if let Ok(Some((win_streak, _loss_streak))) = repo
                        .get_combat_streaks(&combat.guild_id, loser_id)
                        .await
                    {
                        loser_pre_streak = Some(win_streak);
                        if win_streak >= BOUNTY_AUTO_OPEN_STREAK_THRESHOLD {
                            // Cherche une bounty ouverte (theoriquement
                            // auto-creee a streak=5 dans un combat
                            // precedent). Sinon fallback BOUNTY_INITIAL.
                            // Si claim() echoue, on retombe sur le fallback
                            // BOUNTY_INITIAL_AMOUNT (montant fixe) plutot que
                            // de crediter `total` : sinon la bounty reste
                            // ouverte en DB et un autre joueur pourrait la
                            // re-claim plus tard -> double-paiement.
                            let bounty_amount: i64 = if let Some(brepo) = &self.bounty_repo {
                                match brepo.get_open(&combat.guild_id, loser_id).await {
                                    Ok(Some(b)) => {
                                        match brepo.claim(b.id, winner_id).await {
                                            Ok(_) => b.total_amount,
                                            Err(e) => {
                                                tracing::error!(
                                                    error = %e,
                                                    bounty_id = %b.id,
                                                    "Echec claim bounty — fallback initial pour eviter double-paiement"
                                                );
                                                BOUNTY_INITIAL_AMOUNT
                                            }
                                        }
                                    }
                                    _ => BOUNTY_INITIAL_AMOUNT,
                                }
                            } else {
                                BOUNTY_INITIAL_AMOUNT
                            };
                            if let Err(e) = self
                                .wallet_repo
                                .credit(
                                    &combat.guild_id,
                                    winner_id,
                                    bounty_amount,
                                    "regicide_bounty",
                                    "Prime collective Regicide",
                                )
                                .await
                            {
                                warn!(error = %e, "Echec credit regicide");
                            }
                            fields.push(ResolvedCombatEmbedField {
                                name: "\u{1f451} Regicide".into(),
                                value: format!(
                                    "\u{1f451} **REGICIDE !** <@{}> casse la serie de {} victoires de <@{}> et empoche **+{} coins** de prime collective.",
                                    winner_id, win_streak, loser_id, bounty_amount
                                ),
                                inline: false,
                            });
                        }
                    }
                }
                // (Etape 2 — post-touch — vit plus bas, voir
                // bloc `if let Some(brepo)` apres on_player_won).
                let _ = loser_pre_streak;

                if let Ok(Some(ev)) = self
                    .taunts_uc
                    .on_player_won(&combat.guild_id, winner_id)
                    .await
                {
                    taunt_events.push(ev);
                }
                // Bouclier malchance (4.1) : sous shield, on saute le
                // touch_loss_streak qui resetterait la win streak. Pas
                // d incrementation de loss_streak non plus — c est le prix
                // de la "1ere defaite adoucie".
                if !shield_active {
                    if let Ok(Some(ev)) = self
                        .taunts_uc
                        .on_player_lost(&combat.guild_id, loser_id)
                        .await
                    {
                        taunt_events.push(ev);
                    }
                }

                // Etape 2 (post-touch) : si le gagnant vient d atteindre
                // la 5e victoire consecutive, on auto-ouvre une prime
                // de 1000c sur sa tete (cf. COUPE_AMELIORATIONS 5.3).
                if let (Some(prepo), Some(brepo)) = (&self.player_repo, &self.bounty_repo) {
                    if let Ok(Some((winner_streak, _))) = prepo
                        .get_combat_streaks(&combat.guild_id, winner_id)
                        .await
                    {
                        // >= et non == : sous shield ou autre edge case, la
                        // streak peut sauter le seuil exact (ex. 4 -> 6) sans
                        // qu on ait jamais ouvert la bounty. open() est
                        // idempotente (Conflict ignore si deja ouverte).
                        if winner_streak >= BOUNTY_AUTO_OPEN_STREAK_THRESHOLD {
                            // open echoue avec Conflict si une bounty est
                            // deja ouverte — c est OK, no-op silent.
                            if let Err(e) = brepo
                                .open(
                                    &combat.guild_id,
                                    winner_id,
                                    BOUNTY_INITIAL_AMOUNT,
                                )
                                .await
                            {
                                if !matches!(e, DomainError::Conflict(_)) {
                                    warn!(error = %e, "Echec auto-open bounty");
                                }
                            } else {
                                taunt_events.push(crate::domain::entities::coude::taunt::TauntEvent {
                                    channel_id: combat.guild_id.clone(),
                                    target_user_id: winner_id.clone(),
                                    message: format!(
                                        "\u{1f48e} Une **prime collective de {}c** vient de s ouvrir sur la tete de <@{}> ! Tout le monde peut contribuer via `/contribuer-prime` jusqu a ce qu il soit battu.",
                                        BOUNTY_INITIAL_AMOUNT, winner_id
                                    ),
                                    nickname_suffix: String::new(),
                                    streak_kind: "win",
                                    streak_value: winner_streak,
                                });
                            }
                        }
                    }
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

        // Mythiques (cf. COUPE_AMELIORATIONS 2.1) — annonce de l event
        // deja roll au debut de la resolution (cf. plus haut).
        let mythic_announce: Option<String> = mythic_event.map(|ev| {
            use crate::domain::entities::coude::mythic_events::format_mythic_announce;
            let winner_name = result.winner_id.as_deref().and_then(|id| {
                if id == combat.attacker_id { Some(combat.attacker_name.as_str()) }
                else if id == combat.defender_id { Some(combat.defender_name.as_str()) }
                else { None }
            });
            let loser_name = result.loser_id.as_deref().and_then(|id| {
                if id == combat.attacker_id { Some(combat.attacker_name.as_str()) }
                else if id == combat.defender_id { Some(combat.defender_name.as_str()) }
                else { None }
            });
            format_mythic_announce(
                &ev,
                &combat.attacker_name,
                &combat.defender_name,
                winner_name,
                loser_name,
            )
        });

        // Spectateurs fictifs (cf. COUPE_AMELIORATIONS 2.5) — 3-5 faux
        // commentaires de "spectateurs" injectes en fin d embed pour
        // donner l illusion d une foule. Zero mecanique.
        {
            use crate::domain::entities::coude::fake_spectators::format_spectator_chat;
            use crate::domain::entities::coude::fake_spectators::pick_spectator_chat;
            use rand::rngs::StdRng;
            use rand::SeedableRng;
            let mut chat_rng = StdRng::from_entropy();
            let winner_name = result.winner_id.as_deref().and_then(|id| {
                if id == combat.attacker_id { Some(combat.attacker_name.as_str()) }
                else if id == combat.defender_id { Some(combat.defender_name.as_str()) }
                else { None }
            });
            let loser_name = result.loser_id.as_deref().and_then(|id| {
                if id == combat.attacker_id { Some(combat.attacker_name.as_str()) }
                else if id == combat.defender_id { Some(combat.defender_name.as_str()) }
                else { None }
            });
            let chat = pick_spectator_chat(
                &mut chat_rng,
                &combat.attacker_name,
                &combat.defender_name,
                winner_name,
                loser_name,
            );
            fields.push(ResolvedCombatEmbedField {
                name: "\u{1f4e3} Tribune".into(),
                value: format_spectator_chat(&chat),
                inline: false,
            });
        }

        // Pretix les badges memorables au debut de la description si applicable.
        let description = if outcome_flags.is_any_set() {
            let labels = outcome_flags.labels().join(" · ");
            format!("**{}**\n\n{}", labels, result.message)
        } else {
            result.message
        };
        // Prefix mythique en tete de description si tombe.
        let description = if let Some(annonce) = mythic_announce {
            format!("{}\n\n{}", annonce, description)
        } else {
            description
        };

        Ok(ResolveCombatNowOutput {
            combat_id: combat.id.to_string(),
            title: "\u{2694}\u{fe0f} Resultat du Coup de Coude !".into(),
            description,
            color: title_color,
            fields,
            taunt_events,
            vendetta_humiliation,
        })
    }
}

/// Calcule les flags memorables d un combat resolu.
/// Identifie le gagnant + ses HP finaux + son d20 du 1er round + les rounds
/// passes en bas HP, puis appelle `detect_outcome_flags`.
fn compute_outcome_flags_from_result(
    result: &crate::domain::services::coude::coude_combat_engine::combat::CombatResult,
    combat: &crate::domain::entities::coude::combat::Combat,
) -> CombatOutcomeFlags {
    let Some(winner_id) = result.winner_id.as_ref() else {
        // Match nul : pas de "winner" -> seul potentiel flag = zero_pointe
        return detect_outcome_flags(0, 1, 0, result.total_rounds as usize, None, 0);
    };

    let winner_is_attacker = *winner_id == combat.attacker_id;
    let (winner_hp_remaining, winner_hp_max, loser_hp_remaining, winner_first_d20) =
        if winner_is_attacker {
            (
                result.attacker_hp_final,
                result.attacker_hp_max,
                result.defender_hp_final,
                result.rounds.first().map(|r| r.attacker_roll as u8),
            )
        } else {
            (
                result.defender_hp_final,
                result.defender_hp_max,
                result.attacker_hp_final,
                result.rounds.first().map(|r| r.defender_roll as u8),
            )
        };

    // Compte les rounds ou le gagnant est passe sous le seuil COMEBACK_HP_PCT_MAX.
    let low_hp_threshold = (winner_hp_max as f64 * COMEBACK_HP_PCT_MAX) as i32;
    let winner_low_hp_rounds: usize = result
        .rounds
        .iter()
        .filter(|r| {
            let hp = if winner_is_attacker { r.attacker_hp_after } else { r.defender_hp_after };
            hp <= low_hp_threshold && hp > 0
        })
        .count();

    detect_outcome_flags(
        winner_hp_remaining,
        winner_hp_max,
        winner_low_hp_rounds,
        result.total_rounds as usize,
        winner_first_d20,
        loser_hp_remaining,
    )
}

// `load_balance_params` deplace dans `application::guild_settings`
// (cf. API P0 #3 audit). Le wrapper local conserve l'usage existant.
use crate::application::coude::guild_settings::load_balance_params;

use crate::domain::entities::coude::player::title_for_level as title_for_level;
