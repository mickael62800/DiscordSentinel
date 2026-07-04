//! Type d'organisation (cf. 05.md §3). Stocke en TEXT cote Postgres.

/// Les 5 familles d'organisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrganizationKind {
    /// Creer de la richesse (banque, industrie, commerce...).
    Entreprise,
    /// Obtenir le pouvoir : candidats, lois, campagnes.
    Parti,
    /// Controler l'information : articles, enquetes, scandales.
    Media,
    /// Defendre un groupe : greves, negociations, pression.
    Syndicat,
    /// Influencer sans etre visible (mafia, espions, societe secrete).
    Secrete,
}

impl OrganizationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrganizationKind::Entreprise => "entreprise",
            OrganizationKind::Parti => "parti",
            OrganizationKind::Media => "media",
            OrganizationKind::Syndicat => "syndicat",
            OrganizationKind::Secrete => "secrete",
        }
    }

    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "entreprise" => Some(Self::Entreprise),
            "parti" => Some(Self::Parti),
            "media" => Some(Self::Media),
            "syndicat" => Some(Self::Syndicat),
            "secrete" => Some(Self::Secrete),
            _ => None,
        }
    }

    /// Libelle francais affichable.
    pub fn label(&self) -> &'static str {
        match self {
            OrganizationKind::Entreprise => "Entreprise",
            OrganizationKind::Parti => "Parti politique",
            OrganizationKind::Media => "Média",
            OrganizationKind::Syndicat => "Syndicat",
            OrganizationKind::Secrete => "Organisation secrète",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            OrganizationKind::Entreprise => "🏢",
            OrganizationKind::Parti => "🏛️",
            OrganizationKind::Media => "📰",
            OrganizationKind::Syndicat => "✊",
            OrganizationKind::Secrete => "🕵️",
        }
    }
}
