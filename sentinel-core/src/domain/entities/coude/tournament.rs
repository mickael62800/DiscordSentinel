//! Regles metier pour les tournois hebdomadaires "Coup de Coude".

use chrono::DateTime;
use chrono::Datelike;
use chrono::Duration;
use chrono::TimeZone;
use chrono::Utc;
/// Pourcentage du cashbox attribue au prize pool estime du tournoi courant.
/// Regle metier : 10% du solde de la cashbox de la guild constitue le pot.
pub const TOURNAMENT_PRIZE_POOL_PERCENT: i64 = 10;

/// Calcule les bornes de la semaine [lundi 00:00:00 UTC, dimanche 23:59:59 UTC]
/// qui contient `now`. Regle metier : semaine lundi-dimanche en UTC.
pub fn week_bounds_for(now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
    let dow = now.weekday().num_days_from_monday() as i64;
    let start_date = now.date_naive() - Duration::days(dow);
    let start = Utc
        .from_utc_datetime(&start_date.and_hms_opt(0, 0, 0).unwrap())
        .to_utc();
    let end = start + Duration::days(7) - Duration::seconds(1);
    (start, end)
}

/// Bornes de la semaine courante (wrapper pratique qui utilise `Utc::now()`).
/// Non-testable purement mais delegue au `week_bounds_for` qui l'est.
pub fn current_week_bounds() -> (DateTime<Utc>, DateTime<Utc>) {
    week_bounds_for(Utc::now())
}

/// Estime le prize pool d'un tournoi en fonction de la cashbox : 10% du solde.
/// Retourne 0 si cashbox est None (aucune cashbox configuree).
pub fn estimate_tournament_prize_pool(cashbox_balance: Option<i64>) -> i64 {
    cashbox_balance.unwrap_or(0) / TOURNAMENT_PRIZE_POOL_PERCENT
}

#[cfg(test)]
#[path = "tests/tournament.rs"]
mod tests;
