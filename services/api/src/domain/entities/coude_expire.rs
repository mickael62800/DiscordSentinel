//! Regles metier pour l'expiration d'un combat Coup de Coude pending
//! (defenseur qui n'a pas repondu dans le delai imparti).

/// Penalite de lachete appliquee au defenseur quand son combat
/// expire sans reponse : 20% de la mise, minimum 1 coin.
///
/// Alimente la caisse communautaire (cashbox) via deposit.
pub fn cowardice_penalty(mise: i64) -> i64 {
    ((mise as f64 * 0.20).max(1.0)) as i64
}

#[cfg(test)]
#[path = "tests/coude_expire.rs"]
mod tests;
