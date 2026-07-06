//! Vote binaire simple sur une motion (cf. 06.md — version MVP).

/// Choix possible d'un bulletin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteChoice {
    Pour,
    Contre,
    Abstention,
}

impl VoteChoice {
    pub fn as_str(&self) -> &'static str {
        match self {
            VoteChoice::Pour => "pour",
            VoteChoice::Contre => "contre",
            VoteChoice::Abstention => "abstention",
        }
    }

    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "pour" => Some(Self::Pour),
            "contre" => Some(Self::Contre),
            "abstention" => Some(Self::Abstention),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            VoteChoice::Pour => "Pour",
            VoteChoice::Contre => "Contre",
            VoteChoice::Abstention => "Abstention",
        }
    }
}

/// Decompte des bulletins d'une motion. `pour`/`contre`/`abstention` sont les
/// NOMBRES de bulletins (affichage) ; `pour_weight`/`contre_weight` sont les
/// memes bulletins PONDERES par l'influence du votant (adoption). Le capital
/// central « influence » pese donc dans le resultat (cf. 04.md §3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    pub pour: i64,
    pub contre: i64,
    pub abstention: i64,
    pub pour_weight: i64,
    pub contre_weight: i64,
}

impl Tally {
    /// Resultat a la cloture : adoptee si le POIDS « pour » depasse le poids
    /// « contre » (egalite = rejetee, abstentions ignorees). Repli sur les
    /// comptes bruts si aucun poids n'est renseigne (tests / anciennes donnees).
    pub fn is_adopted(&self) -> bool {
        if self.pour_weight != 0 || self.contre_weight != 0 {
            self.pour_weight > self.contre_weight
        } else {
            self.pour > self.contre
        }
    }
}
