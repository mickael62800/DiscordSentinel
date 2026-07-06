//! Loi — objet du cycle legislatif (depot -> vote -> application). Phase 3.
//!
//! Pour le MVP : une loi proposee ouvre immediatement un vote binaire de tous
//! les citoyens, cloture a l'echeance par le worker (adoptee si pour>contre).

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Statut du cycle de vie d'une loi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LawStatus {
    /// En cours de vote.
    Vote,
    Adoptee,
    Rejetee,
}

impl LawStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            LawStatus::Vote => "vote",
            LawStatus::Adoptee => "adoptee",
            LawStatus::Rejetee => "rejetee",
        }
    }

    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "vote" | "depot" | "debat" => Some(Self::Vote),
            "adoptee" => Some(Self::Adoptee),
            "rejetee" => Some(Self::Rejetee),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            LawStatus::Vote => "En vote",
            LawStatus::Adoptee => "Adoptée",
            LawStatus::Rejetee => "Rejetée",
        }
    }
}

/// Une loi soumise au vote des citoyens du serveur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Law {
    pub id: Uuid,
    pub guild_id: String,
    pub title: String,
    pub body: String,
    pub status: LawStatus,
    pub author_id: Uuid,
    pub closes_at: Option<DateTime<Utc>>,
    pub channel_id: Option<String>,
    pub message_id: Option<String>,
    /// Effet mecanique optionnel : cle de config (influence-bot) a fixer si la
    /// loi est ADOPTEE. `None` = loi purement narrative (aucun effet).
    pub effect_key: Option<String>,
    pub effect_value: Option<i64>,
    /// Poids de FINANCEMENT (lobbying des orgs) ajoute a chaque camp, en plus
    /// des votes, au moment de la cloture.
    pub funding_pour: i64,
    pub funding_contre: i64,
}

/// Reglages gameplay qu'une loi peut modifier (whitelist stricte : param public
/// -> cle de config -> libelle). Empeche une loi de toucher une config arbitraire.
pub const LAW_EFFECTS: &[(&str, &str, &str)] = &[
    ("cout_enquete", "influence_investigation_cost", "Coût d'une enquête"),
    ("cout_creation_org", "influence_org_creation_cost", "Coût de création d'organisation"),
    ("cout_role_org", "influence_org_role_cost", "Coût du rôle d'organisation"),
    (
        "perte_reputation_scandale",
        "influence_scandal_reputation_loss",
        "Réputation perdue par scandale",
    ),
    (
        "proba_enquete",
        "influence_investigation_success_pct",
        "Probabilité de réussite d'enquête (%)",
    ),
    ("duree_debat_loi", "influence_law_debate_hours", "Durée de débat d'une loi (h)"),
];

/// Cle de config correspondant a un parametre public de loi (whitelist).
pub fn law_effect_key(param: &str) -> Option<&'static str> {
    LAW_EFFECTS.iter().find(|(p, _, _)| *p == param).map(|(_, k, _)| *k)
}

/// Libelle humain d'une cle d'effet (pour les annonces).
pub fn law_effect_label(config_key: &str) -> Option<&'static str> {
    LAW_EFFECTS.iter().find(|(_, k, _)| *k == config_key).map(|(_, _, l)| *l)
}

/// Borne la valeur d'un effet selon la cle (pas de negatif ; % clampe 0..=100).
pub fn clamp_effect_value(config_key: &str, value: i64) -> i64 {
    if config_key == "influence_investigation_success_pct" {
        value.clamp(0, 100)
    } else if config_key == "influence_law_debate_hours" {
        value.max(1)
    } else {
        value.max(0)
    }
}
