//! Detection des moments "memorables" d un combat Coup de Coude.
//!
//! Sprint 1 quick win (cf. COUPE_AMELIORATIONS section 2.3) : derriere
//! chaque combat banal se cache un potentiel "moment de l hebdo". On
//! detecte automatiquement les patterns suivants pour les afficher dans
//! l embed de combat et les compter dans les stats du profil.

/// Flags appliques a un combat resolu. Un combat peut cumuler plusieurs
/// flags (ex: Clutch + GiantKiller, ou Ridicule + Bust solo).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CombatOutcomeFlags {
    /// Le gagnant a fini avec moins de 10% de ses HP max.
    pub clutch: bool,
    /// Le gagnant a passe au moins 2 rounds sous 20% HP.
    pub comeback: bool,
    /// Le gagnant a fini au-dessus de 90% HP — domination.
    pub perfect: bool,
    /// Combat termine en 1 round avec un d20 = 1 (critique foire).
    pub ridicule: bool,
    /// Les deux joueurs sont tombes a 0 HP au meme round (rare).
    pub zero_pointe: bool,
}

impl CombatOutcomeFlags {
    pub fn is_any_set(&self) -> bool {
        self.clutch || self.comeback || self.perfect || self.ridicule || self.zero_pointe
    }

    /// Etiquettes humaines pour l affichage dans l embed.
    pub fn labels(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.clutch { out.push("🔥 CLUTCH"); }
        if self.comeback { out.push("⚡ COMEBACK"); }
        if self.perfect { out.push("💎 PERFECT"); }
        if self.ridicule { out.push("🤡 RIDICULE"); }
        if self.zero_pointe { out.push("🪦 ZERO POINTE"); }
        out
    }
}

/// Seuils HP pour la detection (% du HP max).
pub const CLUTCH_HP_PCT_MAX: f64 = 0.10;
pub const COMEBACK_HP_PCT_MAX: f64 = 0.20;
pub const PERFECT_HP_PCT_MIN: f64 = 0.90;
pub const COMEBACK_MIN_ROUNDS_LOW_HP: usize = 2;

/// Detecte les flags d un combat. Toutes les entrees sont des donnees
/// connues a la fin du combat (cf. `CombatLog` ou similaire).
///
/// # Arguments
/// * `winner_hp_remaining` — HP du gagnant a la fin (>= 0)
/// * `winner_hp_max` — HP max du gagnant (>= 1)
/// * `winner_low_hp_rounds` — combien de rounds le gagnant a passe sous COMEBACK_HP_PCT_MAX
/// * `total_rounds` — nombre de rounds total
/// * `winner_first_d20` — premier d20 du gagnant (utile pour Ridicule si == 1)
/// * `loser_hp_remaining` — HP du perdant a la fin (souvent 0, mais peut etre != 0 si forfait)
pub fn detect_outcome_flags(
    winner_hp_remaining: i32,
    winner_hp_max: i32,
    winner_low_hp_rounds: usize,
    total_rounds: usize,
    winner_first_d20: Option<u8>,
    loser_hp_remaining: i32,
) -> CombatOutcomeFlags {
    let mut flags = CombatOutcomeFlags::default();

    if winner_hp_max <= 0 {
        return flags; // garde defensif, ne devrait pas arriver
    }
    let pct = (winner_hp_remaining as f64 / winner_hp_max as f64).clamp(0.0, 1.0);

    if pct <= CLUTCH_HP_PCT_MAX && winner_hp_remaining > 0 {
        flags.clutch = true;
    }
    if pct >= PERFECT_HP_PCT_MIN {
        flags.perfect = true;
    }
    if winner_low_hp_rounds >= COMEBACK_MIN_ROUNDS_LOW_HP {
        flags.comeback = true;
    }
    // Ridicule : KO en 1 round avec d20 = 1
    if total_rounds == 1 && winner_first_d20 == Some(1) {
        flags.ridicule = true;
    }
    // Zero pointe : les deux ont fini a 0 (rare cas de double KO meme round)
    if winner_hp_remaining == 0 && loser_hp_remaining == 0 {
        flags.zero_pointe = true;
    }

    flags
}

#[cfg(test)]
#[path = "tests/outcome_flags.rs"]
mod tests;
