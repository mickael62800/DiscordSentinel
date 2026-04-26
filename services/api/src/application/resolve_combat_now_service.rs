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

use crate::domain::entities::{
    apply_insurance_to_loss, apply_lucky_shield, compute_combat_xp, detect_outcome_flags,
    format_bet_payout_lines, CoudeBalanceParams, CombatOutcomeFlags, COMEBACK_HP_PCT_MAX,
};
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
use crate::ports::outbound::{
    BotConfigRepository, CoudeCombatRepository, CoudeCursesRepository, CoudeSafetyNetRepository,
    CoudeVendettaRepository, WalletRepository,
};

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
    curses_repo: Option<Arc<dyn CoudeCursesRepository>>,
    safety_net_repo: Option<Arc<dyn CoudeSafetyNetRepository>>,
    vendetta_repo: Option<Arc<dyn CoudeVendettaRepository>>,
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
            curses_repo: None,
            safety_net_repo: None,
            vendetta_repo: None,
        }
    }

    /// Branche le repo des maledictions pour activer Banana
    /// (cf. COUPE_AMELIORATIONS 5.1) sur les d20 du combat.
    pub fn with_curses_repo(mut self, repo: Arc<dyn CoudeCursesRepository>) -> Self {
        self.curses_repo = Some(repo);
        self
    }

    /// Branche le repo du filet de securite (cf. COUPE_AMELIORATIONS 4.4)
    /// pour reduire les pertes du perdant et activer le filet quand son
    /// solde tombe sous le seuil.
    pub fn with_safety_net_repo(mut self, repo: Arc<dyn CoudeSafetyNetRepository>) -> Self {
        self.safety_net_repo = Some(repo);
        self
    }

    /// Branche le repo des vendettas (cf. COUPE_AMELIORATIONS 5.3) pour
    /// detecter les revanches en cours et appliquer le bonus +100% au
    /// gain du challenger qui gagne sa revanche, ou marquer la vendetta
    /// comme perdue dans le cas inverse.
    pub fn with_vendetta_repo(mut self, repo: Arc<dyn CoudeVendettaRepository>) -> Self {
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
        use crate::domain::entities::{safety_net_should_trigger, SAFETY_NET_DURATION_HOURS};
        if !safety_net_should_trigger(balance) {
            return;
        }
        // Skip si deja un filet actif (evite cumul).
        if matches!(repo.get_active(guild_id, user_id).await, Ok(Some(_))) {
            return;
        }
        if let Err(e) = repo.activate(guild_id, user_id, SAFETY_NET_DURATION_HOURS).await {
            warn!(error = %e, %user_id, "Echec activation safety_net");
        }
    }

    async fn fetch_banana(&self, guild_id: &str, user_id: &str) -> bool {
        let Some(repo) = &self.curses_repo else { return false; };
        use crate::domain::entities::CurseKind;
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
        use crate::domain::entities::CurseKind;
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

        let curses = engine::combat::CombatCurses {
            attacker_has_banana: self.fetch_banana(&combat.guild_id, &combat.attacker_id).await,
            defender_has_banana: self.fetch_banana(&combat.guild_id, &combat.defender_id).await,
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

        let mut result = engine::combat::resolve_combat_with_curses(
            &atk_player,
            &def_player,
            attacker.hp_current,
            defender.hp_current,
            combat.mise,
            attacker_special_effective,
            defender_special_effective,
            &engine_events,
            &balance,
            curses,
        );

        // Prefix les messages Graisser dans la description du combat.
        if !graisser_msgs.is_empty() {
            result.message = format!("{}\n\n{}", graisser_msgs.join("\n"), result.message);
        }

        // Mythiques (cf. COUPE_AMELIORATIONS 2.1) — roll au tout debut pour
        // pouvoir appliquer les effets mecaniques avant les paiements.
        let mythic_event: Option<crate::domain::entities::MythicEvent> = {
            use crate::domain::entities::roll_mythic_event;
            use rand::rngs::StdRng;
            use rand::SeedableRng;
            let mut myth_rng = StdRng::from_entropy();
            roll_mythic_event(&mut myth_rng)
        };

        // Effets mecaniques des mythiques (cf. COUPE_AMELIORATIONS 2.1).
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

        let mut vendetta_msg: Option<String> = None;
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
                use crate::domain::entities::apply_revenge_bonus;
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
                        }
                        coins_transferred_nominal
                    }
                } else {
                    coins_transferred_nominal
                };

                // Assurance : clamp-then-apply dans le domain pour que les
                // joueurs fauches beneficient effectivement de la protection.
                let active_insurance = self
                    .inventory_uc
                    .get_active_insurance(&combat.guild_id, loser_id)
                    .await
                    .ok()
                    .flatten();
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
                insurance_msg = adj.message;

                // Sprint 1 (4.1) — bouclier malchance : si c est la 1ere
                // defaite du jour, perte * 0.5. Recommande pour eviter la
                // spirale "j ai perdu une fois, je quitte".
                let is_first_defeat_today = self
                    .combat_repo
                    .count_defeats_today(&combat.guild_id, loser_id)
                    .await
                    .unwrap_or(0)
                    == 0;
                let actual_loss = apply_lucky_shield(adj.actual_loss, is_first_defeat_today);
                if is_first_defeat_today && actual_loss < adj.actual_loss {
                    let shield_msg = format!(
                        "\u{1f49a} Bouclier malchance du jour : perte reduite de {} a {}.",
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
                    use crate::domain::entities::safety_net_reduce_loss;
                    let reduced = safety_net_reduce_loss(actual_loss, true);
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
                        warn!(error = %e, "Echec payout combat atomique");
                    }
                }

                // Filet de securite (cf. COUPE_AMELIORATIONS 4.4) : apres
                // le payout, si le solde du perdant tombe sous 50c et qu il
                // n a pas deja un filet actif, on l active. Best-effort.
                self.try_activate_safety_net_after(&combat.guild_id, loser_id).await;

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
                    let _ = self
                        .wallet_repo
                        .debit(&combat.guild_id, &combat.attacker_id, loss, "coude_combat_explosion", &desc)
                        .await;
                    let _ = self
                        .wallet_repo
                        .debit(&combat.guild_id, &combat.defender_id, loss, "coude_combat_explosion", &desc)
                        .await;
                    let _ = self
                        .players_uc
                        .record_draw(&combat.guild_id, &combat.attacker_id, loss)
                        .await;
                    let _ = self
                        .players_uc
                        .record_draw(&combat.guild_id, &combat.defender_id, loss)
                        .await;
                }

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

        // Mythiques (cf. COUPE_AMELIORATIONS 2.1) — annonce de l event
        // deja roll au debut de la resolution (cf. plus haut).
        let mythic_announce: Option<String> = mythic_event.map(|ev| {
            use crate::domain::entities::format_mythic_announce;
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
            use crate::domain::entities::{format_spectator_chat, pick_spectator_chat};
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
        })
    }
}

/// Calcule les flags memorables d un combat resolu.
/// Identifie le gagnant + ses HP finaux + son d20 du 1er round + les rounds
/// passes en bas HP, puis appelle `detect_outcome_flags`.
fn compute_outcome_flags_from_result(
    result: &crate::domain::services::coude_combat_engine::combat::CombatResult,
    combat: &crate::domain::entities::CoudeCombat,
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
