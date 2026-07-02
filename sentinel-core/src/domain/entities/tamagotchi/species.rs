//! Les especes de compagnons (~6) avec leurs affinites de stats de depart.
//!
//! Les valeurs sont des stats de BASE a la naissance ; le joueur les fait
//! ensuite progresser via l'entrainement. Affinites = identite de l'espece.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Species {
    Sanglier,
    Renard,
    Tortue,
    Loup,
    Lapin,
    Ours,
}

/// Stats de base d'une espece.
#[derive(Debug, Clone, Copy)]
pub struct SpeciesBase {
    pub str_: i32,
    pub vit: i32,
    pub agi: i32,
}

impl Species {
    pub const ALL: [Species; 6] = [
        Species::Sanglier,
        Species::Renard,
        Species::Tortue,
        Species::Loup,
        Species::Lapin,
        Species::Ours,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Species::Sanglier => "sanglier",
            Species::Renard => "renard",
            Species::Tortue => "tortue",
            Species::Loup => "loup",
            Species::Lapin => "lapin",
            Species::Ours => "ours",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "sanglier" => Some(Species::Sanglier),
            "renard" => Some(Species::Renard),
            "tortue" => Some(Species::Tortue),
            "loup" => Some(Species::Loup),
            "lapin" => Some(Species::Lapin),
            "ours" => Some(Species::Ours),
            _ => None,
        }
    }

    /// Nom affiche + emoji.
    pub fn display(&self) -> &'static str {
        match self {
            Species::Sanglier => "Sanglier 🐗",
            Species::Renard => "Renard 🦊",
            Species::Tortue => "Tortue 🐢",
            Species::Loup => "Loup 🐺",
            Species::Lapin => "Lapin 🐰",
            Species::Ours => "Ours 🐻",
        }
    }

    /// Stats de base (affinites). Total ~30 reparti differemment.
    pub fn base_stats(&self) -> SpeciesBase {
        match self {
            // Bourrin : grosse FORCE.
            Species::Sanglier => SpeciesBase {
                str_: 16,
                vit: 10,
                agi: 4,
            },
            // Rapide et fourbe : AGILITE.
            Species::Renard => SpeciesBase {
                str_: 6,
                vit: 8,
                agi: 16,
            },
            // Mur : VITALITE.
            Species::Tortue => SpeciesBase {
                str_: 6,
                vit: 18,
                agi: 6,
            },
            // Equilibre offensif.
            Species::Loup => SpeciesBase {
                str_: 12,
                vit: 10,
                agi: 8,
            },
            // Vif, peu robuste.
            Species::Lapin => SpeciesBase {
                str_: 7,
                vit: 7,
                agi: 16,
            },
            // Tank offensif.
            Species::Ours => SpeciesBase {
                str_: 14,
                vit: 14,
                agi: 2,
            },
        }
    }
}

#[cfg(test)]
#[path = "tests/species.rs"]
mod tests;
