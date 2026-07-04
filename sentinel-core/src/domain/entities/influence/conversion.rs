//! Conversion de capitaux — cœur du gameplay (04.md §2/§10). Fonction PURE.
//!
//! « Le jeu consiste a transformer un capital en un autre. » On modelise un
//! ensemble fini de conversions dirigees, chacune avec un cout entier (unites
//! de capital SOURCE par unite de capital CIBLE), regle par la config.

use crate::domain::entities::influence::capital::Capital;

/// Les conversions autorisees (chaine 04.md §10 : Argent -> Reputation ->
/// Influence, et Argent -> Information).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionKind {
    MoneyToReputation,
    ReputationToInfluence,
    MoneyToInformation,
}

impl ConversionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConversionKind::MoneyToReputation => "money_reputation",
            ConversionKind::ReputationToInfluence => "reputation_influence",
            ConversionKind::MoneyToInformation => "money_information",
        }
    }

    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "money_reputation" => Some(Self::MoneyToReputation),
            "reputation_influence" => Some(Self::ReputationToInfluence),
            "money_information" => Some(Self::MoneyToInformation),
            _ => None,
        }
    }

    pub fn source(&self) -> Capital {
        match self {
            ConversionKind::MoneyToReputation => Capital::Money,
            ConversionKind::ReputationToInfluence => Capital::Reputation,
            ConversionKind::MoneyToInformation => Capital::Money,
        }
    }

    pub fn target(&self) -> Capital {
        match self {
            ConversionKind::MoneyToReputation => Capital::Reputation,
            ConversionKind::ReputationToInfluence => Capital::Influence,
            ConversionKind::MoneyToInformation => Capital::Information,
        }
    }
}

/// Couts de conversion (unites de source par unite de cible), regles par config.
#[derive(Debug, Clone, Copy)]
pub struct ConversionRates {
    pub money_to_reputation: i64,
    pub reputation_to_influence: i64,
    pub money_to_information: i64,
}

impl Default for ConversionRates {
    fn default() -> Self {
        Self {
            money_to_reputation: 10,
            reputation_to_influence: 5,
            money_to_information: 20,
        }
    }
}

impl ConversionRates {
    pub fn cost_of(&self, kind: ConversionKind) -> i64 {
        match kind {
            ConversionKind::MoneyToReputation => self.money_to_reputation,
            ConversionKind::ReputationToInfluence => self.reputation_to_influence,
            ConversionKind::MoneyToInformation => self.money_to_information,
        }
    }
}

/// Resultat d'une conversion : combien depenser (source) et gagner (cible).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversionResult {
    pub spent: i64,  // debite de la source (multiple exact du cout)
    pub gained: i64, // credite a la cible
}

/// Erreurs de conversion (traduites en `DomainError` par le service).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionError {
    /// Cout de conversion non configure / invalide (<= 0).
    InvalidRate,
    /// Le montant a depenser est inferieur au cout d'un point.
    BelowMinimum { cost: i64 },
    /// Solde de capital source insuffisant.
    Insufficient { available: i64, needed: i64 },
}

/// Convertit `budget` unites de capital source en capital cible.
///
/// On produit `gained = budget / cost` points (division entiere) et on ne
/// debite que `gained * cost` (le reste non convertible n'est pas preleve).
pub fn convert(
    kind: ConversionKind,
    budget: i64,
    available_source: i64,
    rates: &ConversionRates,
) -> Result<ConversionResult, ConversionError> {
    let cost = rates.cost_of(kind);
    if cost <= 0 {
        return Err(ConversionError::InvalidRate);
    }
    if budget < cost {
        return Err(ConversionError::BelowMinimum { cost });
    }
    let gained = budget / cost;
    let spent = gained * cost;
    if spent > available_source {
        return Err(ConversionError::Insufficient {
            available: available_source,
            needed: spent,
        });
    }
    Ok(ConversionResult { spent, gained })
}

#[cfg(test)]
#[path = "tests/conversion.rs"]
mod tests;
