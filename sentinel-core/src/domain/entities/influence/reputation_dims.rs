//! Reputation multi-dimensionnelle (03.md §10). Complete le capital scalaire
//! `reputation` : quatre axes affectes par des actions distinctes.

/// Les quatre dimensions de reputation d'un citoyen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReputationDims {
    pub reliability: i64,
    pub popularity: i64,
    pub notoriety: i64,
    pub transparency: i64,
}

/// Un axe de reputation ciblable par un ajustement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReputationDim {
    Reliability,
    Popularity,
    Notoriety,
    Transparency,
}

impl ReputationDim {
    /// Nom de colonne SQL (enum ferme -> pas d'injection).
    pub fn column(self) -> &'static str {
        match self {
            ReputationDim::Reliability => "reliability",
            ReputationDim::Popularity => "popularity",
            ReputationDim::Notoriety => "notoriety",
            ReputationDim::Transparency => "transparency",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ReputationDim::Reliability => "Fiabilité",
            ReputationDim::Popularity => "Popularité",
            ReputationDim::Notoriety => "Notoriété",
            ReputationDim::Transparency => "Transparence",
        }
    }
}
