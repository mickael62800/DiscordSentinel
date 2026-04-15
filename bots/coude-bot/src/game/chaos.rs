use rand::Rng;

/// Evenement chaotique pouvant survenir lors d'un round de combat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaosEvent {
    /// 2% par round — L'attaquant inflige x2 degats ce round
    CritiqueSauvage,
    /// 2% par round — Le defenseur esquive et contre-attaque +50% degats
    EsquiveDivine,
    /// 1.5% par round — Les deux prennent 10% de leurs HP max en degats
    AccidentDebile,
    /// 1% par round — L'attaquant se frappe lui-meme
    Glissade,
    /// 1.5% par round — Le gagnant du round vole 5% des coins de l'adversaire
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
}

/// Tire un evenement chaos pour un round (8% de chance total par round).
/// Sur un combat de 5 rounds, probabilite d'au moins un event : ~34%.
pub fn roll_chaos() -> Option<ChaosEvent> {
    let mut rng = rand::thread_rng();
    let roll: u32 = rng.gen_range(1..=1000);

    match roll {
        1..=20 => Some(ChaosEvent::CritiqueSauvage),   // 2%
        21..=40 => Some(ChaosEvent::EsquiveDivine),     // 2%
        41..=55 => Some(ChaosEvent::AccidentDebile),     // 1.5%
        56..=65 => Some(ChaosEvent::Glissade),           // 1%
        66..=80 => Some(ChaosEvent::Vol),                // 1.5%
        _ => None,                                        // 92%
    }
}

