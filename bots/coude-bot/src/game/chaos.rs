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

    pub fn description(&self) -> &'static str {
        match self {
            Self::CritiqueSauvage => "L'attaquant inflige le double de degats !",
            Self::EsquiveDivine => "Le defenseur esquive et contre-attaque !",
            Self::AccidentDebile => "Les deux joueurs se cognent la tete !",
            Self::Glissade => "L'attaquant se frappe lui-meme !",
            Self::Vol => "Des coins tombent des poches !",
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

/// Messages fun pour chaque type d'evenement chaos.
pub const CHAOS_CRITIQUE: &[&str] = &[
    "\u{1f4a5} **CRITIQUE SAUVAGE !** {attaquant} met TOUTE sa force dans ce coup !",
    "\u{26a1} Un eclair de puissance ! {attaquant} frappe deux fois plus fort !",
    "\u{1f525} {attaquant} voit rouge et declenche un coup DEVASTATEUR !",
];

pub const CHAOS_ESQUIVE: &[&str] = &[
    "\u{2728} **ESQUIVE DIVINE !** {defenseur} esquive avec grace et contre-attaque !",
    "\u{1f300} {defenseur} disparait comme un ninja et frappe dans le dos !",
    "\u{1fa9e} {defenseur} fait un pas de cote digne d'un film et riposte !",
];

pub const CHAOS_ACCIDENT: &[&str] = &[
    "\u{1f4a9} **ACCIDENT DEBILE !** Les deux joueurs se cognent la tete en meme temps !",
    "\u{1f921} Les deux glissent dans une flaque et se font mal !",
    "\u{1f414} Un poulet traverse l'arene ! Les deux trebuchent !",
];

pub const CHAOS_GLISSADE: &[&str] = &[
    "\u{1faa4} **GLISSADE !** {attaquant} marche sur une peau de banane et se frappe !",
    "\u{1f9ca} {attaquant} glisse sur du verglas et s'auto-KO ce round !",
    "\u{1f938} {attaquant} tente une pirouette... et se met un coup de coude a lui-meme !",
];

pub const CHAOS_VOL: &[&str] = &[
    "\u{1f4b0} **VOL A LA TIRE !** Pendant le chaos, des coins tombent des poches !",
    "\u{1f412} Un singe vole des coins et les donne au plus fort !",
    "\u{1f32a}\u{fe0f} Le vent souffle des coins d'une poche a l'autre !",
];
