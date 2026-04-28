//! Mise par defaut intelligente (cf. COUPE_AMELIORATIONS section 1.2).
//!
//! Quand un joueur tape `/coude @cible` sans mise ou clique sur "Tirer"
//! sans choisir, on suggere automatiquement **20% de son wallet** clampe
//! dans la fourchette serveur [min_bet, max_bet].
//!
//! Logique pure, testable sans contexte serveur.

/// Pourcentage du wallet utilise pour la mise par defaut.
pub const DEFAULT_BET_PCT: f64 = 0.20;

/// Calcule la mise suggeree :
/// - 20% du wallet du joueur
/// - clampe a [min_bet, max_bet]
/// - si wallet < min_bet : retourne min_bet (le joueur prendra l erreur
///   "solde insuffisant" cote service, c est intentionnel — il sait au moins
///   ce qui est attendu).
///
/// # Arguments
/// * `wallet_balance` — solde courant du joueur (peut etre 0 ou negatif)
/// * `min_bet` — borne min serveur (typiquement 10)
/// * `max_bet` — borne max serveur (typiquement 1000+)
pub fn suggest_default_bet(wallet_balance: i64, min_bet: i64, max_bet: i64) -> i64 {
    if max_bet < min_bet {
        // Config invalide -> fallback sur min_bet
        return min_bet.max(1);
    }
    let raw = ((wallet_balance.max(0) as f64) * DEFAULT_BET_PCT).round() as i64;
    raw.clamp(min_bet, max_bet)
}

/// Boutons rapides a proposer : ratios fixes (ex: 1×, 2×, 5×, all-in).
/// Retourne les valeurs effectives (deja clampees).
///
/// `multipliers` est typiquement `&[1, 2, 5]` (interpretation : 1× / 2× / 5×
/// la mise suggeree). `all_in` ajoute le solde complet en bouton supplementaire.
pub fn quick_bet_buttons(
    suggested: i64,
    wallet_balance: i64,
    min_bet: i64,
    max_bet: i64,
    multipliers: &[i64],
    include_all_in: bool,
) -> Vec<i64> {
    let mut out: Vec<i64> = multipliers
        .iter()
        .map(|m| (suggested.saturating_mul(*m)).clamp(min_bet, max_bet))
        .collect();

    if include_all_in {
        let all_in = wallet_balance.clamp(min_bet, max_bet);
        out.push(all_in);
    }

    // Dedup conserve l ordre.
    let mut seen = std::collections::HashSet::new();
    out.retain(|v| seen.insert(*v));
    out
}

#[cfg(test)]
#[path = "tests/smart_default_bet.rs"]
mod tests;
