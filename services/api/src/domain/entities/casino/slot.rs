//! Domaine pur du jeu "slot machine" / "tirette".
//!
//! Mecanique :
//!   1. Le joueur mise X coins (debit wallet)
//!   2. RNG ponderee tire 3 symboles parmi N (cf. `spin_with_rng`)
//!   3. Evaluation (`evaluate_spin`) :
//!      - 3 fois le dernier symbole (jackpot)  -> Jackpot (multiplier + pool entier)
//!      - 3 fois autre symbole identique       -> ThreeOfAKind (mise * multiplier)
//!      - 2 identiques + `payout_2x_enabled`   -> RefundTwoOfAKind (mise rendue)
//!      - sinon                                -> Loss (rien)
//!   4. Pool jackpot alimente par `jackpot_pool_share_pct` % de la mise
//!
//! La fonction `spin_with_rng` accepte un `RngCore` -> seedable et donc
//! testable de maniere deterministe (cf. tests).

use chrono::{DateTime, Utc};
use rand::{distributions::WeightedIndex, prelude::Distribution, RngCore};
use uuid::Uuid;

/// Entree persistee dans `slot_spin_log` : trace d un spin.
#[derive(Debug, Clone)]
pub struct SlotSpin {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub mise: i64,
    pub symbols: Vec<String>, // 3 elements
    pub payout: i64,
    pub multiplier: f64,
    pub is_jackpot: bool,
    pub is_free: bool,
    pub created_at: DateTime<Utc>,
}

/// Etat du pool jackpot pour une guild.
#[derive(Debug, Clone)]
pub struct SlotJackpotPool {
    pub guild_id: String,
    pub current_pool: i64,
    pub last_won_by: Option<String>,
    pub last_won_at: Option<DateTime<Utc>>,
    pub last_won_amount: Option<i64>,
}

/// Top winner pour le leaderboard.
#[derive(Debug, Clone)]
pub struct SlotTopWinner {
    pub user_id: String,
    pub username: String,
    pub total_payout: i64,
    pub jackpot_count: u32,
    pub spin_count: u32,
}

/// Configuration d'une machine a sous pour une guild.
/// Les vecteurs `symbols`, `weights`, `multipliers_3x` doivent avoir la meme
/// longueur. Le **dernier** index est par convention le symbole jackpot.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotConfig {
    pub symbols: Vec<String>,
    pub weights: Vec<u32>,
    pub multipliers_3x: Vec<f64>,
    pub payout_2x_enabled: bool,
    pub jackpot_pool_share_pct: f64,
    pub jackpot_starting_pool: i64,
    pub min_bet: i64,
    pub max_bet: i64,
    pub default_bet: i64,
    pub cooldown_secs: u64,
    pub daily_bonus_enabled: bool,
    pub daily_bonus_mise: i64,
}

impl Default for SlotConfig {
    fn default() -> Self {
        Self {
            symbols: vec![
                "🍒".into(), "🍋".into(), "🍊".into(), "🍇".into(),
                "🔔".into(), "⭐".into(), "7️⃣".into(),
            ],
            weights: vec![30, 25, 20, 15, 7, 2, 1],
            multipliers_3x: vec![2.0, 3.0, 5.0, 8.0, 12.0, 25.0, 100.0],
            payout_2x_enabled: true,
            jackpot_pool_share_pct: 1.0,
            jackpot_starting_pool: 1000,
            min_bet: 10,
            max_bet: 1000,
            default_bet: 50,
            cooldown_secs: 5,
            daily_bonus_enabled: true,
            daily_bonus_mise: 100,
        }
    }
}

/// Resultat metier d'un spin (avant calcul du payout final).
#[derive(Debug, Clone, PartialEq)]
pub enum SpinOutcome {
    /// Aucun match.
    Loss,
    /// 2 symboles identiques sur 3 -> remboursement de la mise.
    RefundTwoOfAKind,
    /// 3 symboles identiques (non-jackpot) -> mise * multiplier.
    ThreeOfAKind { symbol_index: usize, multiplier: f64 },
    /// 3 fois le symbole jackpot (= dernier index) -> mise * multiplier + pool.
    Jackpot { multiplier: f64 },
}

/// Erreurs de validation de la SlotConfig (purement metier).
#[derive(Debug, Clone, PartialEq)]
pub enum SlotConfigError {
    LengthsMismatch,
    EmptySymbols,
    AllWeightsZero,
    BetRangeInvalid,
    SharePctOutOfRange,
}

impl SlotConfigError {
    pub fn as_str(&self) -> &'static str {
        match self {
            SlotConfigError::LengthsMismatch => "symbols, weights et multipliers_3x doivent avoir la meme longueur",
            SlotConfigError::EmptySymbols => "il faut au moins 2 symboles",
            SlotConfigError::AllWeightsZero => "la somme des poids doit etre > 0",
            SlotConfigError::BetRangeInvalid => "min_bet > 0 et min_bet <= max_bet requis",
            SlotConfigError::SharePctOutOfRange => "jackpot_pool_share_pct doit etre entre 0 et 100",
        }
    }
}

/// Verifie l invariant interne de la config. Appelee a chaque chargement
/// (le service refuse de servir une config invalide).
pub fn validate_slot_config(c: &SlotConfig) -> Result<(), SlotConfigError> {
    if c.symbols.is_empty() || c.symbols.len() < 2 {
        return Err(SlotConfigError::EmptySymbols);
    }
    if c.symbols.len() != c.weights.len() || c.symbols.len() != c.multipliers_3x.len() {
        return Err(SlotConfigError::LengthsMismatch);
    }
    if c.weights.iter().sum::<u32>() == 0 {
        return Err(SlotConfigError::AllWeightsZero);
    }
    if c.min_bet <= 0 || c.min_bet > c.max_bet {
        return Err(SlotConfigError::BetRangeInvalid);
    }
    if !(0.0..=100.0).contains(&c.jackpot_pool_share_pct) {
        return Err(SlotConfigError::SharePctOutOfRange);
    }
    Ok(())
}

/// Tire 3 symboles selon les poids de la config. RNG injectee -> seedable
/// (cf. `rand::rngs::StdRng::from_seed` dans les tests).
///
/// Retourne un array de 3 indices (vers `config.symbols`).
pub fn spin_with_rng(rng: &mut impl RngCore, config: &SlotConfig) -> [usize; 3] {
    let dist = WeightedIndex::new(&config.weights).expect("validate_slot_config doit etre appele avant");
    [dist.sample(rng), dist.sample(rng), dist.sample(rng)]
}

/// Evalue le resultat metier d un spin (3 indices).
pub fn evaluate_spin(symbols: &[usize; 3], config: &SlotConfig) -> SpinOutcome {
    let jackpot_idx = config.symbols.len().saturating_sub(1);

    // 3 identiques ?
    if symbols[0] == symbols[1] && symbols[1] == symbols[2] {
        let idx = symbols[0];
        let multiplier = *config.multipliers_3x.get(idx).unwrap_or(&0.0);
        return if idx == jackpot_idx {
            SpinOutcome::Jackpot { multiplier }
        } else {
            SpinOutcome::ThreeOfAKind { symbol_index: idx, multiplier }
        };
    }

    // 2 identiques sur 3 ?
    if config.payout_2x_enabled
        && (symbols[0] == symbols[1] || symbols[1] == symbols[2] || symbols[0] == symbols[2])
    {
        return SpinOutcome::RefundTwoOfAKind;
    }

    SpinOutcome::Loss
}

/// Calcule le payout final (montant credit au joueur) selon l outcome et la mise.
/// Le pool jackpot (en cas de Jackpot) est passe separement et s ajoute au gain.
pub fn compute_payout(mise: i64, outcome: &SpinOutcome, current_jackpot_pool: i64) -> i64 {
    match outcome {
        SpinOutcome::Loss => 0,
        SpinOutcome::RefundTwoOfAKind => mise,
        SpinOutcome::ThreeOfAKind { multiplier, .. } => {
            ((mise as f64) * multiplier).round() as i64
        }
        SpinOutcome::Jackpot { multiplier } => {
            ((mise as f64) * multiplier).round() as i64 + current_jackpot_pool
        }
    }
}

/// Contribution au pool jackpot pour une mise donnee.
pub fn compute_jackpot_contribution(mise: i64, share_pct: f64) -> i64 {
    ((mise as f64) * (share_pct / 100.0)).floor() as i64
}

// ── Parseurs CSV (utilises par le service pour decoder bot_guild_config) ──

/// Parse "🍒,🍋,🍊" en vec de strings (trim, vides supprimees).
pub fn parse_csv_symbols(s: &str) -> Vec<String> {
    s.split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

/// Parse "30,25,20" en vec d entiers (parse fail = 0).
pub fn parse_csv_weights(s: &str) -> Vec<u32> {
    s.split(',')
        .map(|p| p.trim().parse::<u32>().unwrap_or(0))
        .collect()
}

/// Parse "2.0,3,5.5" en vec de floats (parse fail = 0.0).
pub fn parse_csv_multipliers(s: &str) -> Vec<f64> {
    s.split(',')
        .map(|p| p.trim().parse::<f64>().unwrap_or(0.0))
        .collect()
}

#[cfg(test)]
#[path = "tests/slot.rs"]
mod tests;
