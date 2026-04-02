use rand::Rng;

/// Evenement chaotique pouvant survenir lors d'un combat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaosEvent {
    /// 5% — Le gagnant empoche x3
    CritiqueSauvage,
    /// 5% — Le defenseur contre-attaque automatiquement
    EsquiveDivine,
    /// 3% — Les deux perdent toute la mise
    AccidentDebile,
    /// 2% — L'attaquant se frappe lui-meme
    Glissade,
    /// 3% — Le gagnant vole +20% en plus
    Vol,
}

impl ChaosEvent {
    pub fn key(&self) -> &'static str {
        match self {
            Self::CritiqueSauvage => "critique_sauvage",
            Self::EsquiveDivine => "esquive_divine",
            Self::AccidentDebile => "accident_debile",
            Self::Glissade => "glissade",
            Self::Vol => "vol",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Self::CritiqueSauvage => "\u{1f4a5}",
            Self::EsquiveDivine => "\u{2728}",
            Self::AccidentDebile => "\u{1f4a9}",
            Self::Glissade => "\u{1faa4}",
            Self::Vol => "\u{1f4b0}",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::CritiqueSauvage => "CRITIQUE SAUVAGE",
            Self::EsquiveDivine => "ESQUIVE DIVINE",
            Self::AccidentDebile => "ACCIDENT DEBILE",
            Self::Glissade => "GLISSADE",
            Self::Vol => "VOL A LA TIRE",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::CritiqueSauvage => "Le gagnant empoche x3 !",
            Self::EsquiveDivine => "Le defenseur contre-attaque !",
            Self::AccidentDebile => "Les deux joueurs perdent leur mise !",
            Self::Glissade => "L'attaquant se frappe lui-meme !",
            Self::Vol => "Le gagnant vole 20% de bonus !",
        }
    }
}

/// Tire un evenement chaos (18% de chance total).
pub fn roll_chaos() -> Option<ChaosEvent> {
    let mut rng = rand::thread_rng();
    let roll: u32 = rng.gen_range(1..=100);

    match roll {
        1..=5 => Some(ChaosEvent::CritiqueSauvage),
        6..=10 => Some(ChaosEvent::EsquiveDivine),
        11..=13 => Some(ChaosEvent::AccidentDebile),
        14..=15 => Some(ChaosEvent::Glissade),
        16..=18 => Some(ChaosEvent::Vol),
        _ => None,
    }
}
