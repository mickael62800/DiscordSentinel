//! Railleries automatiques sur series (Phase 9 Part D).
//!
//! Toute la "logique metier" des railleries vit ici :
//! - les seuils de declenchement (3/5/10)
//! - les catalogues de messages moqueurs par type de serie
//! - les suffixes progressifs appliques au pseudo Discord
//!
//! Le bot ne fait que poster dans un salon et renommer — il n'a aucune
//! regle a appliquer.

use rand::seq::SliceRandom;
use rand::Rng;

/// Seuils auxquels on declenche un taunt. Toute nouvelle valeur de
/// streak qui matche un seuil provoque un `TauntEvent`.
pub const TAUNT_THRESHOLDS: &[i32] = &[3, 5, 10];

/// Type de serie trackee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreakKind {
    /// Victoires consecutives en combat.
    Win,
    /// Defaites consecutives en combat.
    Loss,
    /// Fois d'affilee ou le joueur s'est fait voler (victime).
    StealVictim,
}

impl StreakKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Win => "win",
            Self::Loss => "loss",
            Self::StealVictim => "steal_victim",
        }
    }
}

/// Config de la feature par guild. Le channel peut etre None (feature
/// disabled ou non configuree). `enabled` permet de couper global sans
/// perdre le channel_id configure.
#[derive(Debug, Clone)]
pub struct CoudeTauntsConfig {
    pub guild_id: String,
    pub channel_id: Option<String>,
    pub enabled: bool,
}

/// Evenement emis par l'API quand un seuil est franchi. Tout est deja
/// calcule et pret a etre poste tel quel par le bot.
#[derive(Debug, Clone)]
pub struct TauntEvent {
    /// Salon Discord ou poster le message (ID brut, le bot fait le cast).
    pub channel_id: String,
    /// Joueur cible du taunt (pour mention + rename).
    pub target_user_id: String,
    /// Message moqueur deja compose avec mention du joueur.
    pub message: String,
    /// Suffixe (ou prefixe) a ajouter au pseudo Discord. Le bot prend
    /// le display_name courant et applique ce suffixe, en gardant un
    /// nickname final sous 32 caracteres (limite Discord).
    pub nickname_suffix: String,
    /// Pour le log (non utilise par le bot).
    pub streak_kind: &'static str,
    pub streak_value: i32,
}

// ── Catalogues de messages par kind x threshold ──
//
// Placeholder `{user}` remplace par la mention <@user_id>.

const WIN_MESSAGES_3: &[&str] = &[
    "\u{1f525} {user} enchaine 3 victoires ! L'arene tremble.",
    "\u{2694}\u{fe0f} Triple victoire pour {user} — quelqu'un ose encore ?",
    "\u{1f3c6} {user} commence a prendre gout au sang. 3 d'affilee.",
];

const WIN_MESSAGES_5: &[&str] = &[
    "\u{1f525}\u{1f525} 5 victoires consecutives pour {user}. C'est plus un joueur, c'est un raz-de-maree.",
    "\u{1f451} {user} regne sur le serveur avec 5 W d'affilee.",
    "\u{26a1} {user} a trouve un bug : personne n'arrive a le battre. 5 victoires.",
];

const WIN_MESSAGES_10: &[&str] = &[
    "\u{1f31f} LEGENDAIRE. {user} atteint 10 victoires sans interruption. Arretez-le quelqu'un !",
    "\u{1f9b8} {user} est officiellement **imbattable** avec 10 W. Priere de signaler le hack.",
    "\u{1f3f0} {user} a construit un empire sur 10 victoires. Les plebeiens tremblent.",
];

const LOSS_MESSAGES_3: &[&str] = &[
    "\u{1f62d} 3 defaites pour {user}. On appelle ca une mauvaise passe.",
    "\u{1f480} {user} encha\u{00ee}ne 3 defaites — peut-etre essayer le mode facile ?",
    "\u{1f97a} Triple KO pour {user}. Le tapis est confortable ?",
];

const LOSS_MESSAGES_5: &[&str] = &[
    "\u{1f926} 5 defaites d'affilee pour {user}. Tu es sur que tu joues du bon cote de l'ecran ?",
    "\u{1f6aa} {user} a pris la porte 5 fois. La sortie est toujours au meme endroit.",
    "\u{1f4a9} {user} vient de perdre son 5e combat consecutif. Impressionnant de constance.",
];

const LOSS_MESSAGES_10: &[&str] = &[
    "\u{1f3c6} {user} remporte le trophee de la **10e defaite consecutive**. Bravo champion.",
    "\u{1f4c9} {user} a defini un nouveau standard pour la mediocrite : 10 losses d'affilee.",
    "\u{1faa6} {user}, on va devoir ecrire un livre sur toi : \"Comment perdre 10 combats sans essayer\".",
];

const STEAL_VICTIM_MESSAGES_3: &[&str] = &[
    "\u{1f45c} {user} s'est fait vider 3 fois. Ta cagnotte fuit comme une passoire.",
    "\u{1fa99} 3 vols subis pour {user}. Ton compte en banque est une porte ouverte.",
    "\u{1f439} Les voleurs ont fait le tour de {user} 3 fois. C'est devenu un distributeur.",
];

const STEAL_VICTIM_MESSAGES_5: &[&str] = &[
    "\u{1f92f} {user} est officiellement la vache a lait du serveur. 5 vols.",
    "\u{1f4b8} {user} a finance la moitie du serveur en se faisant voler 5 fois.",
    "\u{1f911} 5 vols subis. {user}, il faudrait peut-etre investir dans une protection, non ?",
];

const STEAL_VICTIM_MESSAGES_10: &[&str] = &[
    "\u{1f4c8} {user} atteint le palier **10 vols** — c'est une carriere.",
    "\u{1f4dd} Note officielle : {user} est la caisse enregistreuse du serveur. 10 vols.",
    "\u{1f621} 10 fois. DIX FOIS. {user}, comment c'est encore possible ?",
];

fn messages_for(kind: StreakKind, threshold: i32) -> &'static [&'static str] {
    match (kind, threshold) {
        (StreakKind::Win, 3) => WIN_MESSAGES_3,
        (StreakKind::Win, 5) => WIN_MESSAGES_5,
        (StreakKind::Win, 10) => WIN_MESSAGES_10,
        (StreakKind::Loss, 3) => LOSS_MESSAGES_3,
        (StreakKind::Loss, 5) => LOSS_MESSAGES_5,
        (StreakKind::Loss, 10) => LOSS_MESSAGES_10,
        (StreakKind::StealVictim, 3) => STEAL_VICTIM_MESSAGES_3,
        (StreakKind::StealVictim, 5) => STEAL_VICTIM_MESSAGES_5,
        (StreakKind::StealVictim, 10) => STEAL_VICTIM_MESSAGES_10,
        _ => &[],
    }
}

/// Suffixe de pseudo par kind x threshold. Volontairement court pour
/// rester sous les 32 chars de Discord meme avec des pseudos longs.
pub fn nickname_suffix_for(kind: StreakKind, threshold: i32) -> &'static str {
    match (kind, threshold) {
        (StreakKind::Win, 3) => " (en feu)",
        (StreakKind::Win, 5) => " (tyran)",
        (StreakKind::Win, 10) => " le Legende",
        (StreakKind::Loss, 3) => " (KO)",
        (StreakKind::Loss, 5) => " le Pouf",
        (StreakKind::Loss, 10) => " le Paillasson",
        (StreakKind::StealVictim, 3) => " (vide)",
        (StreakKind::StealVictim, 5) => " le Pigeon",
        (StreakKind::StealVictim, 10) => " la Tirelire",
        _ => "",
    }
}

/// Retourne le palier franchi si `new_streak` en est un, sinon None.
pub fn crossed_threshold(new_streak: i32) -> Option<i32> {
    TAUNT_THRESHOLDS
        .iter()
        .copied()
        .find(|&t| t == new_streak)
}

/// Construit un TauntEvent pret a etre poste par le bot. Renvoie None
/// si :
///   - le seuil n'est pas franchi
///   - aucun channel n'est configure / feature disabled
///   - le joueur a opt-out
pub fn build_taunt_event(
    config: &CoudeTauntsConfig,
    target_user_id: &str,
    kind: StreakKind,
    new_streak: i32,
    user_opted_out: bool,
) -> Option<TauntEvent> {
    if user_opted_out {
        return None;
    }
    if !config.enabled {
        return None;
    }
    let channel_id = config.channel_id.clone()?;
    let threshold = crossed_threshold(new_streak)?;
    let messages = messages_for(kind, threshold);
    if messages.is_empty() {
        return None;
    }

    // Tire un message aleatoire dans un bloc scope pour que le
    // ThreadRng (non-Send) soit drop avant qu'on rende l'Option.
    let chosen = {
        let mut rng = rand::thread_rng();
        // unwrap() safe : on a verifie qu'on avait au moins un message.
        *messages.choose(&mut rng).unwrap_or(&"")
    };
    let message = chosen.replace("{user}", &format!("<@{target_user_id}>"));
    let nickname_suffix = nickname_suffix_for(kind, threshold).to_string();

    Some(TauntEvent {
        channel_id,
        target_user_id: target_user_id.to_string(),
        message,
        nickname_suffix,
        streak_kind: kind.as_str(),
        streak_value: new_streak,
    })
}

/// Pour les tests : force une selection deterministe (first message).
#[cfg(test)]
pub fn build_taunt_event_deterministic(
    config: &CoudeTauntsConfig,
    target_user_id: &str,
    kind: StreakKind,
    new_streak: i32,
    user_opted_out: bool,
) -> Option<TauntEvent> {
    if user_opted_out || !config.enabled {
        return None;
    }
    let channel_id = config.channel_id.clone()?;
    let threshold = crossed_threshold(new_streak)?;
    let messages = messages_for(kind, threshold);
    let first = *messages.first()?;
    let message = first.replace("{user}", &format!("<@{target_user_id}>"));
    Some(TauntEvent {
        channel_id,
        target_user_id: target_user_id.to_string(),
        message,
        nickname_suffix: nickname_suffix_for(kind, threshold).to_string(),
        streak_kind: kind.as_str(),
        streak_value: new_streak,
    })
}

// Helper pour eviter `#[allow(dead_code)]` sur Rng — garde l'import used.
#[inline]
fn _keep_rng_used<R: Rng>(_: &mut R) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_with_channel() -> CoudeTauntsConfig {
        CoudeTauntsConfig {
            guild_id: "g1".into(),
            channel_id: Some("123".into()),
            enabled: true,
        }
    }

    #[test]
    fn thresholds_contain_3_5_10() {
        assert_eq!(TAUNT_THRESHOLDS, &[3, 5, 10]);
    }

    #[test]
    fn crossed_threshold_detects_exact_matches_only() {
        assert_eq!(crossed_threshold(3), Some(3));
        assert_eq!(crossed_threshold(5), Some(5));
        assert_eq!(crossed_threshold(10), Some(10));
        // Les valeurs intermediaires ne declenchent pas.
        assert_eq!(crossed_threshold(1), None);
        assert_eq!(crossed_threshold(2), None);
        assert_eq!(crossed_threshold(4), None);
        assert_eq!(crossed_threshold(6), None);
        assert_eq!(crossed_threshold(11), None);
    }

    #[test]
    fn build_none_when_user_opted_out() {
        let ev =
            build_taunt_event_deterministic(&cfg_with_channel(), "u1", StreakKind::Win, 3, true);
        assert!(ev.is_none());
    }

    #[test]
    fn build_none_when_feature_disabled() {
        let mut cfg = cfg_with_channel();
        cfg.enabled = false;
        let ev = build_taunt_event_deterministic(&cfg, "u1", StreakKind::Win, 3, false);
        assert!(ev.is_none());
    }

    #[test]
    fn build_none_when_no_channel_configured() {
        let mut cfg = cfg_with_channel();
        cfg.channel_id = None;
        let ev = build_taunt_event_deterministic(&cfg, "u1", StreakKind::Win, 3, false);
        assert!(ev.is_none());
    }

    #[test]
    fn build_none_when_below_threshold() {
        let ev =
            build_taunt_event_deterministic(&cfg_with_channel(), "u1", StreakKind::Win, 2, false);
        assert!(ev.is_none());
    }

    #[test]
    fn build_success_substitutes_user_mention() {
        let ev = build_taunt_event_deterministic(
            &cfg_with_channel(),
            "u1",
            StreakKind::Win,
            3,
            false,
        )
        .expect("should build event");
        assert!(ev.message.contains("<@u1>"));
        assert!(!ev.message.contains("{user}"));
        assert_eq!(ev.channel_id, "123");
        assert_eq!(ev.target_user_id, "u1");
        assert_eq!(ev.streak_kind, "win");
        assert_eq!(ev.streak_value, 3);
    }

    #[test]
    fn all_kind_threshold_combinations_have_messages_and_suffix() {
        for kind in [StreakKind::Win, StreakKind::Loss, StreakKind::StealVictim] {
            for &t in TAUNT_THRESHOLDS {
                let msgs = messages_for(kind, t);
                assert!(!msgs.is_empty(), "missing messages for {:?}/{}", kind, t);
                let suffix = nickname_suffix_for(kind, t);
                assert!(!suffix.is_empty(), "missing suffix for {:?}/{}", kind, t);
                assert!(
                    suffix.len() <= 24,
                    "suffix too long (pseudo+suffix risk >32 chars): {:?}/{}",
                    kind,
                    t
                );
            }
        }
    }

    #[test]
    fn random_selection_picks_from_catalog() {
        // Smoke test : le chemin non-deterministe ne panic pas et renvoie Some
        // pour une config valide + seuil franchi.
        let ev =
            build_taunt_event(&cfg_with_channel(), "u42", StreakKind::Loss, 5, false);
        assert!(ev.is_some());
    }
}
