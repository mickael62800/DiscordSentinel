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

/// Decompte des bulletins d'une motion.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    pub pour: i64,
    pub contre: i64,
    pub abstention: i64,
}

impl Tally {
    /// Resultat a la cloture : adoptee si strictement plus de « pour » que de
    /// « contre » (egalite = rejetee). Les abstentions ne comptent pas.
    pub fn is_adopted(&self) -> bool {
        self.pour > self.contre
    }
}
