//! Paramètres ECONOMY réglables par serveur du jeu "Coup de Coude".
//!
//! Le domaine reste PUR : cette structure est passée EN ENTRÉE (as data)
//! aux fonctions domain qui utilisaient auparavant des constantes en dur.
//! Le domaine ne lit JAMAIS la config du serveur lui-même — la couche
//! application construit un `CoudeEconomyConfig` depuis la config
//! `coude-bot` (éditable par serveur) via `from_config`, puis le fournit.
//!
//! L'implémentation `Default` reproduit EXACTEMENT les constantes
//! historiques (cf. `combat::resolution_rules`, `steal::roll`,
//! `tout_ou_rien`, `heist`, `curse`, `tournament`), si bien que le
//! comportement est inchangé tant qu'aucune surcharge n'est configurée.
//!
//! Ce module reflète le pattern `ScoringConfig` (moderation/scoring_service).

use std::collections::HashMap;

use crate::domain::entities::coude::combat::flavor::FLAVOR_LINE_PROBABILITY;
use crate::domain::entities::coude::combat::resolution_rules::COMBAT_XP_LOSER;
use crate::domain::entities::coude::combat::resolution_rules::COMBAT_XP_WINNER_BASE;
use crate::domain::entities::coude::combat::resolution_rules::COMBAT_XP_WINNER_UNDERDOG;
use crate::domain::entities::coude::combat::resolution_rules::UNDERDOG_LEVEL_GAP;
use crate::domain::entities::coude::curse::CURSE_COST_COINS;
use crate::domain::entities::coude::curse::CURSE_LIFT_MULTIPLIER;
use crate::domain::entities::coude::curse::FAUSSE_ASSURANCE_FEE_COINS;
use crate::domain::entities::coude::curse::LEAKY_WALLET_FEE_COINS;
use crate::domain::entities::coude::heist::HEIST_BASE_SUCCESS_PERCENT;
use crate::domain::entities::coude::heist::HEIST_GAIN_MAX_PERCENT;
use crate::domain::entities::coude::heist::HEIST_GAIN_MIN_PERCENT;
use crate::domain::entities::coude::heist::HEIST_MAX_SUCCESS_PERCENT;
use crate::domain::entities::coude::refusal_count::HONOR_DEBT_THRESHOLD;
use crate::domain::entities::coude::social::DAILY_CHAOS_MAX;
use crate::domain::entities::coude::social::MIN_COINS_ELIGIBLE;
use crate::domain::entities::coude::tournament::TOURNAMENT_PRIZE_POOL_PERCENT;
use crate::domain::entities::coude::tout_ou_rien::TOUT_OU_RIEN_LOSS_KEEP_PCT;
use crate::domain::entities::coude::tout_ou_rien::TOUT_OU_RIEN_WIN_MULTIPLIER;
use crate::domain::entities::coude::tout_ou_rien::TOUT_OU_RIEN_WIN_PROBABILITY;

/// Défauts des bornes de vol exprimées en pourcentage (le domaine
/// `steal::roll` travaille en basis points = pct × 100).
const STEAL_AFK_MIN_PCT: u32 = 10;
const STEAL_AFK_MAX_PCT: u32 = 15;
const STEAL_ACTIVE_MIN_PCT: u32 = 15;
const STEAL_ACTIVE_MAX_PCT: u32 = 25;

/// Valeurs de balance ECONOMY réglables par serveur, passées en données
/// aux fonctions domain pures. `Default` == constantes historiques.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoudeEconomyConfig {
    // Combat XP.
    pub combat_xp_winner_base: i64,
    pub combat_xp_winner_underdog: i64,
    pub combat_xp_loser: i64,
    // Vol (% de wallet volé selon statut AFK/actif). En POURCENTAGE.
    pub steal_afk_min_pct: u32,
    pub steal_afk_max_pct: u32,
    pub steal_active_min_pct: u32,
    pub steal_active_max_pct: u32,
    // Tout-ou-rien.
    pub tout_ou_rien_win_probability: f64,
    pub tout_ou_rien_win_multiplier: f64,
    pub tout_ou_rien_loss_keep_pct: f64,
    // Braquage (heist).
    pub heist_base_success_pct: u32,
    pub heist_max_success_pct: u32,
    pub heist_gain_min_pct: u32,
    pub heist_gain_max_pct: u32,
    // Malédictions / frais.
    pub curse_cost_coins: i64,
    pub curse_lift_multiplier: i64,
    pub leaky_wallet_fee_coins: i64,
    pub fausse_assurance_fee_coins: i64,
    // Tournoi (% de la cashbox constituant le prize pool).
    pub tournament_prize_pool_pct: i64,
    // ── Gameplay LOW (réglages d'ambiance / seuils non-monétaires) ──
    /// Cap journalier d'événements daily chaos par guild.
    pub daily_chaos_max_events: i64,
    /// Solde minimum pour qu'un joueur soit éligible au tirage chaos.
    pub min_coins_eligible: i64,
    /// Probabilité (0..1) qu'une ligne de flavor soit injectée par round.
    pub flavor_line_probability: f64,
    /// Nombre de refus au-delà duquel la dette d'honneur est due.
    pub honor_debt_threshold: i32,
    /// Écart de niveaux minimum pour activer le bonus Giant Killer.
    pub underdog_level_gap: i32,
}

impl Default for CoudeEconomyConfig {
    fn default() -> Self {
        Self {
            combat_xp_winner_base: COMBAT_XP_WINNER_BASE,
            combat_xp_winner_underdog: COMBAT_XP_WINNER_UNDERDOG,
            combat_xp_loser: COMBAT_XP_LOSER,
            steal_afk_min_pct: STEAL_AFK_MIN_PCT,
            steal_afk_max_pct: STEAL_AFK_MAX_PCT,
            steal_active_min_pct: STEAL_ACTIVE_MIN_PCT,
            steal_active_max_pct: STEAL_ACTIVE_MAX_PCT,
            tout_ou_rien_win_probability: TOUT_OU_RIEN_WIN_PROBABILITY,
            tout_ou_rien_win_multiplier: TOUT_OU_RIEN_WIN_MULTIPLIER,
            tout_ou_rien_loss_keep_pct: TOUT_OU_RIEN_LOSS_KEEP_PCT,
            heist_base_success_pct: HEIST_BASE_SUCCESS_PERCENT,
            heist_max_success_pct: HEIST_MAX_SUCCESS_PERCENT,
            heist_gain_min_pct: HEIST_GAIN_MIN_PERCENT,
            heist_gain_max_pct: HEIST_GAIN_MAX_PERCENT,
            curse_cost_coins: CURSE_COST_COINS,
            curse_lift_multiplier: CURSE_LIFT_MULTIPLIER,
            leaky_wallet_fee_coins: LEAKY_WALLET_FEE_COINS,
            fausse_assurance_fee_coins: FAUSSE_ASSURANCE_FEE_COINS,
            tournament_prize_pool_pct: TOURNAMENT_PRIZE_POOL_PERCENT,
            daily_chaos_max_events: DAILY_CHAOS_MAX,
            min_coins_eligible: MIN_COINS_ELIGIBLE,
            flavor_line_probability: FLAVOR_LINE_PROBABILITY,
            honor_debt_threshold: HONOR_DEBT_THRESHOLD,
            underdog_level_gap: UNDERDOG_LEVEL_GAP,
        }
    }
}

impl CoudeEconomyConfig {
    /// Construit la config depuis une map clé→valeur (typiquement la config
    /// guild `coude-bot`). Toute clé manquante/malformée retombe sur le
    /// défaut. Des GARDES de sécurité sont appliqués APRÈS parsing pour
    /// garantir des valeurs saines (voir `sanitize`), de sorte qu'une
    /// mauvaise config ne casse jamais l'intégrité monétaire.
    pub fn from_config(cfg: &HashMap<String, String>) -> Self {
        let d = Self::default();
        let raw = Self {
            combat_xp_winner_base: parse_i64(cfg, "combat_xp_winner_base", d.combat_xp_winner_base),
            combat_xp_winner_underdog: parse_i64(
                cfg,
                "combat_xp_winner_underdog",
                d.combat_xp_winner_underdog,
            ),
            combat_xp_loser: parse_i64(cfg, "combat_xp_loser", d.combat_xp_loser),
            steal_afk_min_pct: parse_u32(cfg, "steal_afk_min_pct", d.steal_afk_min_pct),
            steal_afk_max_pct: parse_u32(cfg, "steal_afk_max_pct", d.steal_afk_max_pct),
            steal_active_min_pct: parse_u32(cfg, "steal_active_min_pct", d.steal_active_min_pct),
            steal_active_max_pct: parse_u32(cfg, "steal_active_max_pct", d.steal_active_max_pct),
            tout_ou_rien_win_probability: parse_f64(
                cfg,
                "tout_ou_rien_win_probability",
                d.tout_ou_rien_win_probability,
            ),
            tout_ou_rien_win_multiplier: parse_f64(
                cfg,
                "tout_ou_rien_win_multiplier",
                d.tout_ou_rien_win_multiplier,
            ),
            tout_ou_rien_loss_keep_pct: parse_f64(
                cfg,
                "tout_ou_rien_loss_keep_pct",
                d.tout_ou_rien_loss_keep_pct,
            ),
            heist_base_success_pct: parse_u32(
                cfg,
                "heist_base_success_pct",
                d.heist_base_success_pct,
            ),
            heist_max_success_pct: parse_u32(cfg, "heist_max_success_pct", d.heist_max_success_pct),
            heist_gain_min_pct: parse_u32(cfg, "heist_gain_min_pct", d.heist_gain_min_pct),
            heist_gain_max_pct: parse_u32(cfg, "heist_gain_max_pct", d.heist_gain_max_pct),
            curse_cost_coins: parse_i64(cfg, "curse_cost_coins", d.curse_cost_coins),
            curse_lift_multiplier: parse_i64(cfg, "curse_lift_multiplier", d.curse_lift_multiplier),
            leaky_wallet_fee_coins: parse_i64(
                cfg,
                "leaky_wallet_fee_coins",
                d.leaky_wallet_fee_coins,
            ),
            fausse_assurance_fee_coins: parse_i64(
                cfg,
                "fausse_assurance_fee_coins",
                d.fausse_assurance_fee_coins,
            ),
            tournament_prize_pool_pct: parse_i64(
                cfg,
                "tournament_prize_pool_pct",
                d.tournament_prize_pool_pct,
            ),
            daily_chaos_max_events: parse_i64(
                cfg,
                "daily_chaos_max_events",
                d.daily_chaos_max_events,
            ),
            min_coins_eligible: parse_i64(cfg, "min_coins_eligible", d.min_coins_eligible),
            flavor_line_probability: parse_f64(
                cfg,
                "flavor_line_probability",
                d.flavor_line_probability,
            ),
            honor_debt_threshold: parse_i32(cfg, "honor_debt_threshold", d.honor_debt_threshold),
            underdog_level_gap: parse_i32(cfg, "underdog_level_gap", d.underdog_level_gap),
        };
        raw.sanitize()
    }

    /// Applique les GARDES de sécurité (idempotent). Documenté :
    /// - pourcentages bornés à `0..=100`, probabilités à `0.0..=1.0` ;
    /// - `tout_ou_rien_win_multiplier` planché à `1.0` (évite qu'un "gain"
    ///   devienne une perte), `curse_lift_multiplier` planché à `1` ;
    /// - montants de coins forcés `>= 0` ;
    /// - contraintes min ≤ max (vol AFK, vol actif, gain braquage) et
    ///   base ≤ max pour le braquage : en cas d'inversion, la PAIRE
    ///   entière retombe sur ses défauts (comportement sûr et prévisible
    ///   plutôt qu'un swap silencieux qui masquerait une erreur de config).
    fn sanitize(mut self) -> Self {
        let d = Self::default();

        // Coins >= 0.
        self.combat_xp_winner_base = self.combat_xp_winner_base.max(0);
        self.combat_xp_winner_underdog = self.combat_xp_winner_underdog.max(0);
        self.combat_xp_loser = self.combat_xp_loser.max(0);
        self.curse_cost_coins = self.curse_cost_coins.max(0);
        self.leaky_wallet_fee_coins = self.leaky_wallet_fee_coins.max(0);
        self.fausse_assurance_fee_coins = self.fausse_assurance_fee_coins.max(0);

        // Multiplicateurs planchés. NaN (parse déjà filtré is_finite, mais
        // défensif) ou valeur < 1.0 → planché à 1.0.
        if self.tout_ou_rien_win_multiplier.is_nan() || self.tout_ou_rien_win_multiplier < 1.0 {
            self.tout_ou_rien_win_multiplier = d.tout_ou_rien_win_multiplier.max(1.0);
        }
        self.curse_lift_multiplier = self.curse_lift_multiplier.max(1);

        // Probabilités / ratios bornés [0.0, 1.0], NaN → défaut.
        self.tout_ou_rien_win_probability = clamp_unit(
            self.tout_ou_rien_win_probability,
            d.tout_ou_rien_win_probability,
        );
        self.tout_ou_rien_loss_keep_pct = clamp_unit(
            self.tout_ou_rien_loss_keep_pct,
            d.tout_ou_rien_loss_keep_pct,
        );

        // Pourcentages bornés 0..=100.
        self.steal_afk_min_pct = self.steal_afk_min_pct.min(100);
        self.steal_afk_max_pct = self.steal_afk_max_pct.min(100);
        self.steal_active_min_pct = self.steal_active_min_pct.min(100);
        self.steal_active_max_pct = self.steal_active_max_pct.min(100);
        self.heist_base_success_pct = self.heist_base_success_pct.min(100);
        self.heist_max_success_pct = self.heist_max_success_pct.min(100);
        self.heist_gain_min_pct = self.heist_gain_min_pct.min(100);
        self.heist_gain_max_pct = self.heist_gain_max_pct.min(100);
        self.tournament_prize_pool_pct = self.tournament_prize_pool_pct.clamp(0, 100);

        // Contraintes min ≤ max : la paire entière retombe sur ses défauts
        // si inversée (safe, documenté).
        if self.steal_afk_min_pct > self.steal_afk_max_pct {
            self.steal_afk_min_pct = d.steal_afk_min_pct;
            self.steal_afk_max_pct = d.steal_afk_max_pct;
        }
        if self.steal_active_min_pct > self.steal_active_max_pct {
            self.steal_active_min_pct = d.steal_active_min_pct;
            self.steal_active_max_pct = d.steal_active_max_pct;
        }
        if self.heist_gain_min_pct > self.heist_gain_max_pct {
            self.heist_gain_min_pct = d.heist_gain_min_pct;
            self.heist_gain_max_pct = d.heist_gain_max_pct;
        }
        if self.heist_base_success_pct > self.heist_max_success_pct {
            self.heist_base_success_pct = d.heist_base_success_pct;
            self.heist_max_success_pct = d.heist_max_success_pct;
        }

        // ── Gameplay LOW ──
        // Compteurs / coins >= 0 (0 = feature desactivee, ex. cap chaos 0).
        self.daily_chaos_max_events = self.daily_chaos_max_events.max(0);
        self.min_coins_eligible = self.min_coins_eligible.max(0);
        // Seuils de niveaux / refus >= 0.
        self.honor_debt_threshold = self.honor_debt_threshold.max(0);
        self.underdog_level_gap = self.underdog_level_gap.max(0);
        // Probabilite bornee [0, 1], NaN -> defaut.
        self.flavor_line_probability =
            clamp_unit(self.flavor_line_probability, d.flavor_line_probability);

        self
    }
}

fn clamp_unit(v: f64, default: f64) -> f64 {
    if v.is_nan() {
        default
    } else {
        v.clamp(0.0, 1.0)
    }
}

fn parse_i64(cfg: &HashMap<String, String>, key: &str, default: i64) -> i64 {
    cfg.get(key)
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(default)
}

fn parse_i32(cfg: &HashMap<String, String>, key: &str, default: i32) -> i32 {
    cfg.get(key)
        .and_then(|v| v.trim().parse::<i32>().ok())
        .unwrap_or(default)
}

fn parse_u32(cfg: &HashMap<String, String>, key: &str, default: u32) -> u32 {
    cfg.get(key)
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(default)
}

/// Parse tolérant d'un f64 : accepte la virgule décimale française et
/// les espaces. Retombe sur le défaut si vide/malformé.
fn parse_f64(cfg: &HashMap<String, String>, key: &str, default: f64) -> f64 {
    cfg.get(key)
        .and_then(|v| v.trim().replace(',', ".").parse::<f64>().ok())
        .filter(|f| f.is_finite())
        .unwrap_or(default)
}

#[cfg(test)]
#[path = "tests/economy_config.rs"]
mod tests;
