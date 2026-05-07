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

    #[allow(dead_code)]
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
    roll_chaos_with_multiplier(1.0)
}

/// Variante avec multiplicateur applique aux probabilites — utilise par
/// les saisons thematiques (cf. COUPE_AMELIORATIONS 6.3) :
/// - "Saison du Chaos" -> multiplier=2.0 (events x2)
/// - autres saisons -> multiplier=1.0 (neutre)
///
/// Le multiplicateur est applique aux seuils 1..=80 ; le reste reste
/// "no event". Clamp implicite : si le multiplicateur depasse 1000/80
/// (~12.5), on satureait — improbable en pratique (multiplicateur max
/// prevu = 2.0).
pub fn roll_chaos_with_multiplier(multiplier: f64) -> Option<ChaosEvent> {
    let mut rng = rand::thread_rng();
    let roll: u32 = rng.gen_range(1..=1000);

    let scaled = |upper: u32| -> u32 { ((upper as f64) * multiplier) as u32 };
    let t1 = scaled(20);
    let t2 = scaled(40);
    let t3 = scaled(55);
    let t4 = scaled(65);
    let t5 = scaled(80);

    if roll <= t1 {
        Some(ChaosEvent::CritiqueSauvage)
    } else if roll <= t2 {
        Some(ChaosEvent::EsquiveDivine)
    } else if roll <= t3 {
        Some(ChaosEvent::AccidentDebile)
    } else if roll <= t4 {
        Some(ChaosEvent::Glissade)
    } else if roll <= t5 {
        Some(ChaosEvent::Vol)
    } else {
        None
    }
}


#[cfg(test)]
#[path = "tests/chaos.rs"]
mod tests;
