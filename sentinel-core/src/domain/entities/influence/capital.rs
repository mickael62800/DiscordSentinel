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
}
