//! Railleries automatiques sur series (Phase 9 Part D + extensions blackjack/eco).
//!
//! Toute la "logique metier" des railleries vit ici :
//! - les seuils de declenchement (3/5/10)
//! - les catalogues de messages moqueurs par type de serie
//! - les suffixes progressifs appliques au pseudo Discord
//!
//! Le bot ne fait que poster dans un salon et renommer — il n'a aucune
//! regle a appliquer.
//!
//! # Extensions (migration 139)
//!
//! Ajout de 6 nouveaux `StreakKind` :
//! - blackjack : `BjNatural21` (one-shot), `BjBustStreak` (3/5/10),
//!   `BjWinStreak` (3/5/10)
//! - economie  : `EcoBankruptcy`, `EcoJackpot`, `EcoGenerousDonor` (one-shots)
//!
//! Les "one-shot" passent par `build_taunt_event_single` qui court-circuite
//! le check de palier.

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

    // ── Blackjack ──
    /// Blackjack naturel (21 en 2 cartes). One-shot, pas de palier.
    BjNatural21,
    /// Bust (depassement de 21) consecutifs. Palier 3/5/10.
    BjBustStreak,
    /// Mains blackjack gagnees consecutives. Palier 3/5/10.
    BjWinStreak,

    // ── Economie ──
    /// Balance du wallet passe de >0 a 0 (faillite). One-shot.
    EcoBankruptcy,
    /// Gain enorme en une op (> seuil configurable, default 10_000). One-shot.
    EcoJackpot,
    /// Don a un autre joueur > seuil (default 1_000). One-shot.
    EcoGenerousDonor,
}

impl StreakKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Win => "win",
            Self::Loss => "loss",
            Self::StealVictim => "steal_victim",
            Self::BjNatural21 => "bj_natural21",
            Self::BjBustStreak => "bj_bust_streak",
            Self::BjWinStreak => "bj_win_streak",
            Self::EcoBankruptcy => "eco_bankruptcy",
            Self::EcoJackpot => "eco_jackpot",
            Self::EcoGenerousDonor => "eco_generous_donor",
        }
    }

    /// True si ce kind fonctionne par palier (streak qui atteint 3/5/10),
    /// false pour les one-shots (naturel 21, faillite, jackpot, don).
    pub fn is_threshold_based(self) -> bool {
        matches!(
            self,
            Self::Win
                | Self::Loss
                | Self::StealVictim
                | Self::BjBustStreak
                | Self::BjWinStreak
        )
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
    /// Suffixe (ou prefixe) a ajouter au pseudo Discord.
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

// ── Blackjack : Natural 21 (one-shot) ──

const BJ_NATURAL_MESSAGES: &[&str] = &[
    "\u{1f3b0} {user} tire un blackjack naturel ! 21 en 2 cartes. La chance incarnee.",
    "\u{1f3b2} {user} fait 21 d'entree. Le croupier s'incline.",
    "\u{1f451} Blackjack naturel pour {user} — tapis rouge deroule.",
    "\u{2728} {user} : As + tete. Propre, efficace, insolent.",
    "\u{1f4b0} {user} empoche le jackpot direct — blackjack naturel.",
    "\u{1f60e} {user} n'a meme pas eu besoin de tirer. 21 direct.",
    "\u{1f525} {user} sort un blackjack naturel. Le casino tremble.",
    "\u{1f3af} Bullseye : {user} fait 21 en 2 cartes.",
    "\u{1f340} La chance sourit a {user} : blackjack naturel !",
    "\u{1f947} {user} ne joue pas, il collectionne les blackjacks naturels.",
    "\u{1f921} {user} vient de sortir un 21 naturel. On dirait un scenario truque.",
    "\u{1f3c6} {user} decroche le saint Graal du blackjack : le naturel.",
    "\u{1f379} Cocktail parfait pour {user} : As + 10. 21 sans effort.",
    "\u{1f4ab} {user} brille avec un blackjack naturel. Eblouissant.",
    "\u{1f984} Licorne sauvage : {user} fait 21 en 2 cartes.",
];

// ── Blackjack : bust streaks ──

const BJ_BUST_3: &[&str] = &[
    "\u{1f4a5} {user} bust 3 fois de suite. Tirer n'est pas une strategie.",
    "\u{1f915} 3 bust d'affilee pour {user}. Le 21 s'eloigne a chaque carte.",
    "\u{1f614} {user} depasse 21 pour la 3e fois. Peut-etre s'arreter plus tot ?",
    "\u{1f4c9} Triple bust pour {user}. La gravite s'en melerait presque.",
    "\u{1f643} {user} collectionne les bust : 3 d'un coup.",
    "\u{1f92a} {user} a tire 3 fois la carte de trop. Maths difficiles.",
    "\u{1f4a8} 3 bust consecutifs pour {user}. Le deck te deteste.",
    "\u{1f4ad} {user} se demande toujours pourquoi il a dit \"hit\". 3 bust.",
    "\u{1f635} Triple bust pour {user}. Les cartes sont cruelles.",
    "\u{1f926} {user} bust 3 fois. Le croupier n'en revient pas.",
    "\u{1f4a6} {user} coule 3 mains d'affilee. Bust bust bust.",
    "\u{1f94a} 3 KO auto-inflige pour {user}. Impressionnant.",
    "\u{1f331} {user} plante 3 mains. Peut-etre apprendre le stand ?",
    "\u{1f4a5} Bust triple pour {user}. Le chat noir est dans la manche.",
    "\u{1f644} {user} repete 3 fois la meme erreur. Definition de folie.",
];

const BJ_BUST_5: &[&str] = &[
    "\u{1f480} {user} bust 5 fois de suite. C'est devenu un art.",
    "\u{1f921} 5 bust consecutifs pour {user}. On peut parler d'autodestruction.",
    "\u{1f525} {user} brule sa main 5 fois d'affilee. Torche vivante.",
    "\u{1f4c9} {user} atteint 5 bust. Le compteur explose, pas le score.",
    "\u{1f97a} 5 bust pour {user}. Peut-etre changer de jeu ?",
    "\u{1f6ae} {user} jette 5 mains a la poubelle. Recyclage intensif.",
    "\u{1f92f} 5 depassements consecutifs pour {user}. Le chaos incarne.",
    "\u{1f3b2} {user} bust 5 fois. Le RNG a rendu un verdict.",
    "\u{1f47b} {user} est hante par le 21. 5 bust a la suite.",
    "\u{1f635} 5 bust. {user}, le stand existe aussi comme option.",
    "\u{1f4a2} {user} enchaine 5 bust. La table n'en peut plus.",
    "\u{1f4a9} 5 mains ratees pour {user}. Triste spectacle.",
    "\u{2620}\u{fe0f} {user} signe son 5e bust d'affilee. RIP.",
    "\u{1f94a} 5e bust consecutif pour {user}. Masochisme validate.",
    "\u{1f922} {user} ecoeure la table avec 5 bust.",
];

const BJ_BUST_10: &[&str] = &[
    "\u{1f4c9} 10 bust d'affilee. {user}, apprends a compter jusqu'a 21.",
    "\u{1f921} {user} atteint 10 bust consecutifs. Record du monde de la betise.",
    "\u{1f9ee} {user}, l'addition c'est 21, pas 37. 10 bust.",
    "\u{1f480} 10 bust pour {user}. Le casino t'interdit l'entree.",
    "\u{1f3c6} Trophee du pire joueur decerne a {user} : 10 bust.",
    "\u{1f92f} 10 bust consecutifs. {user}, c'est une performance artistique ?",
    "\u{1f4a5} {user} atomise 10 mains d'affilee. Kaboom fois 10.",
    "\u{1f47b} {user} est maudit. 10 bust a la suite, c'est statistiquement improbable.",
    "\u{1f3b4} {user}, change de metier. 10 bust c'est un signe.",
    "\u{1f4d6} Manuel du blackjack offert a {user} apres 10 bust.",
    "\u{1f3af} {user} vise la fosse septique : 10 bust dans le mille.",
    "\u{1faa6} 10 bust. {user} est officiellement nul.",
    "\u{1f6d1} Stop. {user} a bust 10 fois. Quelqu'un lui confisque les cartes.",
    "\u{1f4ad} {user} ne sait pas lire les chiffres. 10 bust le prouvent.",
    "\u{1f921} 10 bust. {user}, Las Vegas t'envoie un cadeau de remerciement.",
];

// ── Blackjack : win streaks ──

const BJ_WIN_3: &[&str] = &[
    "\u{1f0cf} {user} gagne 3 mains de suite. Le croupier transpire.",
    "\u{1f3b0} Triple win blackjack pour {user}. La table est froide.",
    "\u{1f60e} {user} enchaine 3 victoires. Le croupier commence a douter.",
    "\u{1f4b0} 3 mains gagnees pour {user}. Le pactole grossit.",
    "\u{1f947} {user} rafle 3 mains. En douceur.",
    "\u{1f3af} 3 wins blackjack pour {user}. Viseur calibre.",
    "\u{1f525} {user} est lance : 3 victoires de blackjack d'affilee.",
    "\u{1f9ca} {user} joue de sang-froid. 3 mains dans la poche.",
    "\u{1f44c} 3 wins pour {user}. Tranquille, pas de stress.",
    "\u{1f3af} {user} place 3 mains gagnantes. Sniper du blackjack.",
    "\u{1f9e0} {user} compte les cartes ? 3 wins consecutifs.",
    "\u{1f31f} Triple victoire blackjack pour {user}. Etoile montante.",
    "\u{1f4a5} {user} explose 3 mains. Le croupier note dans son carnet.",
    "\u{1f3b2} 3 wins pour {user}. Les des lui sourient.",
    "\u{1f3c5} Medaille de bronze pour 3 wins consecutifs : {user}.",
];

const BJ_WIN_5: &[&str] = &[
    "\u{1f3b0} 5 wins blackjack pour {user}. Le croupier appelle le manager.",
    "\u{1f451} {user} domine la table : 5 victoires consecutives.",
    "\u{1f525} {user} est en feu. 5 mains blackjack remportees d'affilee.",
    "\u{1f4b0} 5 wins pour {user}. La banque commence a trembler.",
    "\u{1f3af} {user} shoote 5 mains parfaites. Que dire de plus ?",
    "\u{1f9ca} 5 wins avec sang-froid pour {user}. Maitre des nerfs.",
    "\u{1f9e0} {user} lit le deck. 5 victoires blackjack consecutives.",
    "\u{1f4ab} {user} aligne 5 wins. Constellation de victoires.",
    "\u{1f3c6} 5 mains blackjack, 5 victoires. {user} signe un carton plein.",
    "\u{1f60e} {user} repousse la chance a 5 reprises. Talent pur.",
    "\u{26a1} 5 wins blackjack. {user} court-circuite le RNG.",
    "\u{1f947} {user} rafle 5 mains. Le croupier envisage la reconversion.",
    "\u{1f3b2} {user} gagne 5 fois d'affilee. La chance c'est pour les autres.",
    "\u{1f4c8} Courbe ascendante pour {user} : 5 wins consecutifs.",
    "\u{1f53a} {user} est au top : 5 victoires blackjack non-stop.",
];

const BJ_WIN_10: &[&str] = &[
    "\u{1f988} {user} a atteint 10 victoires blackjack consecutives. C'est un requin.",
    "\u{1f451} {user} a mis la couronne : 10 wins blackjack d'affilee.",
    "\u{1f3b0} {user}, la legende vivante du blackjack. 10 wins.",
    "\u{1f4a3} {user} fait sauter la banque : 10 victoires consecutives.",
    "\u{1f31f} 10 wins blackjack ! {user} entre dans le hall of fame.",
    "\u{1f525} {user} est incandescent. 10 victoires blackjack d'affilee.",
    "\u{1f9e0} {user} compte chaque carte. 10 wins c'est un exploit.",
    "\u{1f3c6} Trophee ultime pour {user} : 10 wins blackjack consecutifs.",
    "\u{1f4b0} {user} vide les coffres : 10 victoires blackjack.",
    "\u{1f3b2} 10 wins. Le RNG est au service exclusif de {user}.",
    "\u{1f440} Le casino surveille {user}. 10 wins, trop suspect.",
    "\u{1f451} {user} est roi du tapis : 10 wins blackjack consecutifs.",
    "\u{1f3ad} {user} joue a un autre jeu : 10 wins blackjack.",
    "\u{2b50} 10 wins blackjack ! {user} brille plus fort qu'une enseigne Vegas.",
    "\u{1f6b8} Attention : {user} est officiellement dangereux. 10 wins blackjack.",
];

// ── Eco : Bankruptcy (one-shot) ──

const ECO_BANKRUPTCY_MESSAGES: &[&str] = &[
    "\u{1f4b8} {user} est en faillite. Zero coin au compteur.",
    "\u{1faa6} {user} a tout perdu. Le compte est a sec.",
    "\u{1f4c9} Faillite pour {user}. La courbe touche le fond.",
    "\u{1f480} {user} voit sa fortune partir en fumee. Zero.",
    "\u{1f6ab} {user}, plus un sou. Time to grind.",
    "\u{1f4b0} 0 coin pour {user}. Les temps sont durs.",
    "\u{1f4ad} {user} medite sur son passif. Faillite officielle.",
    "\u{1f61e} {user} rejoint le club des fauches. Bienvenue.",
    "\u{1f4e6} {user} a vendu ses meubles. Faillite complete.",
    "\u{1f4ab} {user}, disparition totale du solde. Pouf, zero.",
    "\u{1f6b7} Banqueroute. {user} est hors jeu financierement.",
    "\u{1fab0} {user} se retrouve a sec. Zero coin.",
    "\u{1f914} {user}, peut-etre eviter le blackjack la prochaine fois ?",
    "\u{1f32a} Tempete financiere pour {user}. 0 coin.",
    "\u{1f525} {user} a cramer toute sa fortune. Pheonix des fauches.",
];

// ── Eco : Jackpot (one-shot) ──

const ECO_JACKPOT_MESSAGES: &[&str] = &[
    "\u{1f4b0} {user} empoche un jackpot monstrueux ! La caisse explose.",
    "\u{1f911} {user} vient de devenir riche. Attention aux voleurs.",
    "\u{1f3b0} Jackpot ! {user} fait sauter la banque.",
    "\u{1f4b8} {user} rafle un paquet enorme. Les yeux brillent.",
    "\u{1f4a5} Gain massif pour {user}. La fortune sourit.",
    "\u{1f947} {user} decroche la cagnotte. Bravo champion.",
    "\u{1f31f} {user} flamboie : jackpot inscrit au palmares.",
    "\u{1f4b2} {user} encaisse une pluie de coins.",
    "\u{1f3c6} Jackpot legendaire pour {user}. On parle chiffres.",
    "\u{1f48e} {user} se pave de diamants. Gain colossal.",
    "\u{1f9e8} {user} fait tilt. Jackpot garanti.",
    "\u{1f525} {user} brule la caisse, mais dans le bon sens. Jackpot.",
    "\u{1f3ad} {user} joue au theatre des millionnaires. Jackpot.",
    "\u{1f4a1} {user} illumine le serveur avec un gain enorme.",
    "\u{1f680} {user} decolle. Jackpot ! Direction la lune.",
];

// ── Eco : Generous donor (one-shot) ──

const ECO_DONOR_MESSAGES: &[&str] = &[
    "\u{1f381} {user} fait un don genereux. Mere Teresa du serveur.",
    "\u{1f64f} {user} partage sa fortune. Un cas rare.",
    "\u{1f496} {user} donne sans compter. L'ame caritative.",
    "\u{1f338} {user} offre des coins. Le serveur applaudit.",
    "\u{1f31f} {user} illumine la journee d'un autre. Don massif.",
    "\u{1f3f5}\u{fe0f} {user} fait pleuvoir les coins sur un heureux.",
    "\u{1f973} Fete pour le beneficiaire : {user} a donne gros.",
    "\u{1f49d} {user}, generosite validee. Don confirme.",
    "\u{1f33c} {user} seme des coins. Fleurs de bonte.",
    "\u{1f485} {user} a le coeur sur la main. Don monumental.",
    "\u{1f64b} {user} partage, {user} est beni.",
    "\u{1f955} {user} nourrit les autres. Don genereux.",
    "\u{1f942} {user} leve son verre a sa generosite.",
    "\u{1f940} {user} depose un bouquet de coins. Magnifique.",
    "\u{1f31e} {user} reveille la bonte : don enorme.",
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
        (StreakKind::BjBustStreak, 3) => BJ_BUST_3,
        (StreakKind::BjBustStreak, 5) => BJ_BUST_5,
        (StreakKind::BjBustStreak, 10) => BJ_BUST_10,
        (StreakKind::BjWinStreak, 3) => BJ_WIN_3,
        (StreakKind::BjWinStreak, 5) => BJ_WIN_5,
        (StreakKind::BjWinStreak, 10) => BJ_WIN_10,
        // One-shot : ignore le threshold
        (StreakKind::BjNatural21, _) => BJ_NATURAL_MESSAGES,
        (StreakKind::EcoBankruptcy, _) => ECO_BANKRUPTCY_MESSAGES,
        (StreakKind::EcoJackpot, _) => ECO_JACKPOT_MESSAGES,
        (StreakKind::EcoGenerousDonor, _) => ECO_DONOR_MESSAGES,
        _ => &[],
    }
}

/// Suffixe de pseudo par kind x threshold.
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
        // Blackjack
        (StreakKind::BjNatural21, _) => " \u{1f3b0}",
        (StreakKind::BjBustStreak, 3) => " \u{1f921}",
        (StreakKind::BjBustStreak, 5) => " \u{1f480}",
        (StreakKind::BjBustStreak, 10) => " \u{1f4c9}",
        (StreakKind::BjWinStreak, 3) => " \u{1f0cf}",
        (StreakKind::BjWinStreak, 5) => " \u{1f3b2}",
        (StreakKind::BjWinStreak, 10) => " \u{1f988}",
        // Eco (one-shot)
        (StreakKind::EcoBankruptcy, _) => " \u{1faa6}",
        (StreakKind::EcoJackpot, _) => " \u{1f4b0}",
        (StreakKind::EcoGenerousDonor, _) => " \u{1f381}",
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
///   - le seuil n'est pas franchi (pour les kinds threshold-based)
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

    // Pour les kinds threshold-based : verifier qu'on a franchi 3/5/10.
    // Pour les one-shots : threshold ignore, on tire direct.
    let threshold = if kind.is_threshold_based() {
        crossed_threshold(new_streak)?
    } else {
        0 // valeur neutre : messages_for ignore le threshold pour one-shots
    };
    let messages = messages_for(kind, threshold);
    if messages.is_empty() {
        return None;
    }

    let chosen = {
        let mut rng = rand::thread_rng();
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

/// Version one-shot pour les kinds sans palier (naturel 21, faillite,
/// jackpot, don). Pas de check de streak, juste config + opt-out.
pub fn build_taunt_event_single(
    config: &CoudeTauntsConfig,
    target_user_id: &str,
    kind: StreakKind,
    user_opted_out: bool,
) -> Option<TauntEvent> {
    if kind.is_threshold_based() {
        return None;
    }
    build_taunt_event(config, target_user_id, kind, 0, user_opted_out)
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
    let threshold = if kind.is_threshold_based() {
        crossed_threshold(new_streak)?
    } else {
        0
    };
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
    fn all_combat_kind_threshold_combinations_have_messages_and_suffix() {
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
    fn all_bj_threshold_kinds_have_messages_and_suffix() {
        for kind in [StreakKind::BjBustStreak, StreakKind::BjWinStreak] {
            for &t in TAUNT_THRESHOLDS {
                let msgs = messages_for(kind, t);
                assert!(!msgs.is_empty(), "missing bj messages {:?}/{}", kind, t);
                let suffix = nickname_suffix_for(kind, t);
                assert!(!suffix.is_empty(), "missing bj suffix {:?}/{}", kind, t);
            }
        }
    }

    #[test]
    fn one_shot_kinds_have_catalog_and_suffix() {
        for kind in [
            StreakKind::BjNatural21,
            StreakKind::EcoBankruptcy,
            StreakKind::EcoJackpot,
            StreakKind::EcoGenerousDonor,
        ] {
            assert!(!kind.is_threshold_based());
            let msgs = messages_for(kind, 0);
            assert!(!msgs.is_empty(), "missing one-shot messages {:?}", kind);
            let suffix = nickname_suffix_for(kind, 0);
            assert!(!suffix.is_empty(), "missing one-shot suffix {:?}", kind);
        }
    }

    #[test]
    fn random_selection_picks_from_catalog() {
        let ev =
            build_taunt_event(&cfg_with_channel(), "u42", StreakKind::Loss, 5, false);
        assert!(ev.is_some());
    }

    #[test]
    fn build_single_one_shot_success() {
        let ev = build_taunt_event_single(
            &cfg_with_channel(),
            "u1",
            StreakKind::BjNatural21,
            false,
        )
        .expect("one-shot should build");
        assert!(ev.message.contains("<@u1>"));
        assert_eq!(ev.streak_kind, "bj_natural21");
    }

    #[test]
    fn build_single_rejects_threshold_kind() {
        let ev = build_taunt_event_single(&cfg_with_channel(), "u1", StreakKind::Win, false);
        assert!(ev.is_none());
    }

    #[test]
    fn bj_bust_catalogs_have_at_least_15_variants() {
        assert!(BJ_BUST_3.len() >= 15);
        assert!(BJ_BUST_5.len() >= 15);
        assert!(BJ_BUST_10.len() >= 15);
        assert!(BJ_WIN_3.len() >= 15);
        assert!(BJ_WIN_5.len() >= 15);
        assert!(BJ_WIN_10.len() >= 15);
        assert!(BJ_NATURAL_MESSAGES.len() >= 15);
        assert!(ECO_BANKRUPTCY_MESSAGES.len() >= 15);
        assert!(ECO_JACKPOT_MESSAGES.len() >= 15);
        assert!(ECO_DONOR_MESSAGES.len() >= 15);
    }
}
