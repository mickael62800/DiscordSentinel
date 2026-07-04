//! Les 5 capitaux d'un citoyen (cf. docs/Nouveau jeux/04.md).
//!
//! Stockes en entiers (connus du seul proprietaire) ; exposes aux tiers sous
//! forme de paliers narratifs (cf. `tier.rs`). Le Reseau et l'Information sont
//! decrits comme non-scalaires dans les specs ; pour le MVP on les modelise en
//! score entier (ecart assume, aligne sur le schema BIGINT de la migration).

/// Les cinq capitaux d'un citoyen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capitals {
    /// Capacite a peser sur la societe — ressource centrale (04.md §3).
    pub influence: i64,
    /// Ressources financieres (04.md §4).
    pub money: i64,
    /// Image publique — peut etre negative (04.md §5).
    pub reputation: i64,
    /// Connaissances detenues (04.md §6).
    pub information: i64,
    /// Liens avec les autres joueurs (04.md §7).
    pub network: i64,
}

impl Capitals {
    /// Capitaux d'un nouveau citoyen : tout a zero sauf l'Argent de depart.
    pub fn starting(start_money: i64) -> Self {
        Self {
            influence: 0,
            money: start_money,
            reputation: 0,
            information: 0,
            network: 0,
        }
    }

    /// Valeur d'un capital donne.
    pub fn get(&self, capital: Capital) -> i64 {
        match capital {
            Capital::Influence => self.influence,
            Capital::Money => self.money,
            Capital::Reputation => self.reputation,
            Capital::Information => self.information,
            Capital::Network => self.network,
        }
    }
}

/// Designe l'un des 5 capitaux (pour les conversions, le registre, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capital {
    Influence,
    Money,
    Reputation,
    Information,
    Network,
}

impl Capital {
    pub fn as_str(&self) -> &'static str {
        match self {
            Capital::Influence => "influence",
            Capital::Money => "money",
            Capital::Reputation => "reputation",
            Capital::Information => "information",
            Capital::Network => "network",
        }
    }

    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "influence" => Some(Self::Influence),
            "money" => Some(Self::Money),
            "reputation" => Some(Self::Reputation),
            "information" => Some(Self::Information),
            "network" => Some(Self::Network),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Capital::Influence => "Influence",
            Capital::Money => "Argent",
            Capital::Reputation => "Réputation",
            Capital::Information => "Information",
            Capital::Network => "Réseau",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Capital::Influence => "🏛️",
            Capital::Money => "💰",
            Capital::Reputation => "⭐",
            Capital::Information => "🕵️",
            Capital::Network => "🤝",
        }
    }
}
