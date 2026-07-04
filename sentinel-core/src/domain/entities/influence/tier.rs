//! Paliers narratifs — cœur du principe « stocke chiffre / expose narratif »
//! (cf. ARCHITECTURE.md §1). Fonctions PURES du domaine, sans I/O, testables
//! unitairement (meme esprit que `coude/tout_ou_rien.rs`).
//!
//! Le proprietaire d'un profil voit ses chiffres exacts ; les tiers ne voient
//! qu'un palier (Influence) ou un libelle (Reputation). La decision
//! chiffre/palier est prise dans le service application selon le viewer.

/// Palier narratif generique (Influence, et capitaux positifs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NarrativeTier {
    Negligeable,
    Faible,
    Moyenne,
    Elevee,
    TresElevee,
    Legendaire,
}

impl NarrativeTier {
    /// Libelle francais affichable.
    pub fn label(self) -> &'static str {
        match self {
            NarrativeTier::Negligeable => "Négligeable",
            NarrativeTier::Faible => "Faible",
            NarrativeTier::Moyenne => "Moyenne",
            NarrativeTier::Elevee => "Élevée",
            NarrativeTier::TresElevee => "Très élevée",
            NarrativeTier::Legendaire => "Légendaire",
        }
    }

    /// Rendu en 5 etoiles (cf. profil 03.md §4 : `★★★★☆`).
    pub fn stars(self) -> &'static str {
        match self {
            NarrativeTier::Negligeable => "☆☆☆☆☆",
            NarrativeTier::Faible => "★☆☆☆☆",
            NarrativeTier::Moyenne => "★★☆☆☆",
            NarrativeTier::Elevee => "★★★☆☆",
            NarrativeTier::TresElevee => "★★★★☆",
            NarrativeTier::Legendaire => "★★★★★",
        }
    }
}

/// Seuils (croissants) separant les paliers. Donnee de config passee en
/// parametre (domaine pur, aucune I/O) — cf. `CoudeEconomyConfig`.
///
/// `bounds[i]` = valeur minimale pour atteindre le palier `i+1`. Il faut donc
/// 5 bornes pour 6 paliers.
#[derive(Debug, Clone, Copy)]
pub struct TierThresholds {
    pub bounds: [i64; 5],
}

impl Default for TierThresholds {
    /// Progression geometrique par defaut (reglable via config).
    fn default() -> Self {
        Self {
            bounds: [100, 500, 2_000, 10_000, 50_000],
        }
    }
}

/// Convertit une valeur entiere en palier narratif selon les seuils fournis.
pub fn to_tier(value: i64, thresholds: &TierThresholds) -> NarrativeTier {
    let b = &thresholds.bounds;
    if value < b[0] {
        NarrativeTier::Negligeable
    } else if value < b[1] {
        NarrativeTier::Faible
    } else if value < b[2] {
        NarrativeTier::Moyenne
    } else if value < b[3] {
        NarrativeTier::Elevee
    } else if value < b[4] {
        NarrativeTier::TresElevee
    } else {
        NarrativeTier::Legendaire
    }
}

/// Palier de reputation — echelle qualitative dediee (04.md §5), centree sur 0
/// (la reputation peut etre negative).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReputationTier {
    Desastreuse,
    Mauvaise,
    Neutre,
    Bonne,
    Excellente,
}

impl ReputationTier {
    pub fn label(self) -> &'static str {
        match self {
            ReputationTier::Desastreuse => "Désastreuse",
            ReputationTier::Mauvaise => "Mauvaise",
            ReputationTier::Neutre => "Neutre",
            ReputationTier::Bonne => "Bonne",
            ReputationTier::Excellente => "Excellente",
        }
    }
}

/// Convertit une reputation entiere (positive ou negative) en palier.
pub fn to_reputation_tier(value: i64) -> ReputationTier {
    if value <= -500 {
        ReputationTier::Desastreuse
    } else if value < -50 {
        ReputationTier::Mauvaise
    } else if value <= 50 {
        ReputationTier::Neutre
    } else if value < 500 {
        ReputationTier::Bonne
    } else {
        ReputationTier::Excellente
    }
}

#[cfg(test)]
#[path = "tests/tier.rs"]
mod tests;
