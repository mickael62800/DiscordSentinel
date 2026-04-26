//! Maledictions sociales (cf. COUPE_AMELIORATIONS section 5.1).
//!
//! Permet a un joueur de poser une "malediction" ridicule sur un autre
//! pendant 24h. Cout : 300c. Une seule malediction active par cible.
//! La cible peut "lever" la malediction en payant le double a l auteur.
//!
//! Logique purement domaine : effets pondereux + selection aleatoire.
//! L application des effets est branchee dans les services concernes
//! (combat pour la banane, transactions pour le portefeuille, taunts
//! pour l insomnie, etc.).

use std::fmt;

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Une malediction posee (persistance + lecture).
#[derive(Debug, Clone)]
pub struct ActiveCurse {
    pub id: Uuid,
    pub guild_id: String,
    pub target_id: String,
    pub source_id: String,
    pub kind: CurseKind,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub lifted_at: Option<DateTime<Utc>>,
    pub lifted_by: Option<String>,
    /// Some(n) si la curse a un compteur d utilisations (cf. Empoisonner).
    /// None pour les curses purement temporelles.
    pub uses_remaining: Option<i32>,
}

impl ActiveCurse {
    /// Vraie si la curse n est ni levee ni expiree a `now`.
    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        self.lifted_at.is_none() && self.expires_at > now
    }
}

/// Cout d une malediction (en coins, paye par l auteur).
pub const CURSE_COST_COINS: i64 = 300;

/// Multiplicateur du cout pour lever sa propre malediction (cible -> auteur).
pub const CURSE_LIFT_MULTIPLIER: i64 = 2;

/// Duree de vie d une malediction (heures).
pub const CURSE_DURATION_HOURS: i64 = 24;

/// Probabilite de rater son d20 sous l effet "Peau de banane".
pub const BANANA_FAIL_PROBABILITY: f64 = 0.30;

/// Frais fixes (en coins) preleves sur chaque transaction sous "Portefeuille troue".
pub const LEAKY_WALLET_FEE_COINS: i64 = 10;

/// Multiplicateur de poids des taunts de defaite sous "Insomnie".
pub const INSOMNIA_TAUNT_MULTIPLIER: f64 = 1.5;

/// Retard d affichage (secondes) pour les messages de combat sous "Lenteur".
pub const SLOWNESS_DELAY_SECS: u64 = 10;

/// Les sept "maledictions" / "sabotages" disponibles. Les 6 premiers sont
/// les vraies maledictions de /maudire (24h, 300c, tirage aleatoire).
/// Pancarte est un sabotage de /saboter (7 jours, 150c, pure cosmetique
/// — affichage du saboteur sous le profil de la cible).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CurseKind {
    /// Pseudo renomme "@X le Poulet" pendant 24h.
    Chicken,
    /// 30% de chance de rater chaque d20 (relance a 1).
    Banana,
    /// 10c de frais sur chaque transaction.
    LeakyWallet,
    /// Messages de combat avec 10s de retard.
    Slowness,
    /// Taunts de defaite +50%.
    Insomnia,
    /// Au prochain combat, la licorne ne peut PAS tomber.
    Heartbreak,
    /// Pancarte "Rival officiel de @X" affichee sous le profil 7 jours.
    Pancarte,
    /// Sabotage "Graisser les armes" (cf. COUPE_AMELIORATIONS 5.2) :
    /// la prochaine attaque speciale de la cible foire automatiquement
    /// (override a `None`). Consume on use. Cout : 200c. Expire en 24h
    /// si jamais declenchee.
    Graisser,
    /// Sabotage "Empoisonner le wallet" (cf. COUPE_AMELIORATIONS 5.2) :
    /// sur les 3 prochains gains de combat de la cible, 10% sont redirige
    /// vers le saboteur. Cout : 400c. Expire en 7 jours ou apres 3 uses.
    Empoisonner,
}

impl CurseKind {
    /// Identifiant texte stable pour la persistance (DB / API).
    pub fn as_db_str(self) -> &'static str {
        match self {
            CurseKind::Chicken => "chicken",
            CurseKind::Banana => "banana",
            CurseKind::LeakyWallet => "leaky_wallet",
            CurseKind::Slowness => "slowness",
            CurseKind::Insomnia => "insomnia",
            CurseKind::Heartbreak => "heartbreak",
            CurseKind::Pancarte => "pancarte",
            CurseKind::Graisser => "graisser",
            CurseKind::Empoisonner => "empoisonner",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "chicken" => Some(CurseKind::Chicken),
            "banana" => Some(CurseKind::Banana),
            "leaky_wallet" => Some(CurseKind::LeakyWallet),
            "slowness" => Some(CurseKind::Slowness),
            "insomnia" => Some(CurseKind::Insomnia),
            "heartbreak" => Some(CurseKind::Heartbreak),
            "pancarte" => Some(CurseKind::Pancarte),
            "graisser" => Some(CurseKind::Graisser),
            "empoisonner" => Some(CurseKind::Empoisonner),
            _ => None,
        }
    }

    /// Emoji associe — utilise dans les embeds bot.
    pub fn emoji(self) -> &'static str {
        match self {
            CurseKind::Chicken => "🐔",
            CurseKind::Banana => "🍌",
            CurseKind::LeakyWallet => "💸",
            CurseKind::Slowness => "🐌",
            CurseKind::Insomnia => "🧛",
            CurseKind::Heartbreak => "💔",
            CurseKind::Pancarte => "🪧",
            CurseKind::Graisser => "🛢️",
            CurseKind::Empoisonner => "☠️",
        }
    }

    /// Libelle court francais.
    pub fn label(self) -> &'static str {
        match self {
            CurseKind::Chicken => "Malediction du poulet",
            CurseKind::Banana => "Peau de banane",
            CurseKind::LeakyWallet => "Portefeuille troue",
            CurseKind::Slowness => "Lenteur",
            CurseKind::Insomnia => "Insomnie",
            CurseKind::Heartbreak => "Malchance amoureuse",
            CurseKind::Pancarte => "Pancarte Rival officiel",
            CurseKind::Graisser => "Armes graissees",
            CurseKind::Empoisonner => "Wallet empoisonne",
        }
    }

    /// Cout en coins paye par l auteur. Specifique aux sabotages /
    /// curses non-classiques. Defaut = CURSE_COST_COINS (300c).
    pub fn cost_coins(self) -> i64 {
        match self {
            CurseKind::Pancarte => 150,
            CurseKind::Graisser => 200,
            CurseKind::Empoisonner => 400,
            _ => CURSE_COST_COINS,
        }
    }

    /// Duree de vie en heures. Defaut = CURSE_DURATION_HOURS (24h).
    pub fn duration_hours(self) -> i64 {
        match self {
            CurseKind::Pancarte => 24 * 7,
            CurseKind::Empoisonner => 24 * 7,
            // Graisser = 24h fallback ; en pratique consume au 1er combat.
            _ => CURSE_DURATION_HOURS,
        }
    }

    /// Nombre d utilisations initiales pour les curses "consume on use".
    /// None = curse purement temporelle (la duree gere son extinction).
    /// Some(n) = la curse expire automatiquement apres n declenchements.
    pub fn initial_uses(self) -> Option<i32> {
        match self {
            CurseKind::Empoisonner => Some(3),
            _ => None,
        }
    }

    /// Catalogue des maledictions tirables au sort par /maudire.
    /// Pancarte EXCLUE : c est un sabotage explicite via /saboter.
    pub const ALL: [CurseKind; 6] = [
        CurseKind::Chicken,
        CurseKind::Banana,
        CurseKind::LeakyWallet,
        CurseKind::Slowness,
        CurseKind::Insomnia,
        CurseKind::Heartbreak,
    ];
}

impl fmt::Display for CurseKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.emoji(), self.label())
    }
}

/// Tirage aleatoire d une malediction parmi les 6.
///
/// `rand_index` doit etre un entier dans [0, 6). En pratique on appelle
/// `pick_random_curse(rng.gen_range(0..6))` cote service.
pub fn pick_curse_by_index(rand_index: usize) -> CurseKind {
    CurseKind::ALL[rand_index % CurseKind::ALL.len()]
}

/// Cout pour lever une malediction (paye par la cible a l auteur).
pub const fn lift_cost(_kind: CurseKind) -> i64 {
    CURSE_COST_COINS * CURSE_LIFT_MULTIPLIER
}

/// Applique l effet "Peau de banane" a un jet de d20.
///
/// Si la cible est sous banane et que le tirage de probabilite tombe sous
/// `BANANA_FAIL_PROBABILITY`, le d20 est ramene a 1 (echec critique).
///
/// `proba_roll` doit etre dans [0.0, 1.0).
pub fn apply_banana_to_d20(raw_d20: u8, has_banana: bool, proba_roll: f64) -> u8 {
    if has_banana && proba_roll < BANANA_FAIL_PROBABILITY {
        1
    } else {
        raw_d20
    }
}

/// Calcule le montant net + les frais d une transaction sous l effet
/// "Portefeuille troue". Retourne `(net, fee)`.
///
/// Si le montant est trop petit pour absorber les frais, on retourne
/// `(0, amount)` — la cible perd tout en frais.
pub fn apply_leaky_wallet(amount: i64, has_leaky: bool) -> (i64, i64) {
    if !has_leaky || amount <= 0 {
        return (amount, 0);
    }
    if amount <= LEAKY_WALLET_FEE_COINS {
        return (0, amount);
    }
    (amount - LEAKY_WALLET_FEE_COINS, LEAKY_WALLET_FEE_COINS)
}

/// Pondere un poids de taunt de defaite sous l effet "Insomnie".
pub fn apply_insomnia_to_taunt_weight(base_weight: f64, has_insomnia: bool) -> f64 {
    if has_insomnia {
        base_weight * INSOMNIA_TAUNT_MULTIPLIER
    } else {
        base_weight
    }
}

/// Pourcentage redirige vers le saboteur sous l effet "Empoisonner".
pub const POISON_GAIN_REDIRECT_PCT: f64 = 0.10;

/// Calcule le montant a rediriger vers le saboteur sur un gain donne.
/// Floor a 0 si pas empoisonne ou gain non-positif.
pub fn poison_redirect_amount(gain: i64, has_poison: bool) -> i64 {
    if !has_poison || gain <= 0 {
        return 0;
    }
    ((gain as f64) * POISON_GAIN_REDIRECT_PCT) as i64
}

#[cfg(test)]
#[path = "tests/curse.rs"]
mod tests;
