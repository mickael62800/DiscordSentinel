//! Phrases "fun" piochees aleatoirement dans les embeds de fin de partie.
//!
//! Les placeholders `{joueur}`, `{total}`, `{croupier}`, `{gain}`, `{mise}`
//! sont remplaces par l'appelant (`embeds::build_game_embed`).

use rand::Rng;

pub(super) const BJ_NATURAL: &[&str] = &[
    "BLACKJACK NATUREL ! {joueur} sort 21 du premier coup ! Legendaire !",
    "21 en deux cartes ! {joueur} est un dieu du Blackjack !",
    "La perfection ! {joueur} pose un Blackjack avec classe !",
    "{joueur} claque un 21 naturel ! Le croupier en pleure !",
];

pub(super) const BJ_WIN: &[&str] = &[
    "{joueur} l'emporte avec {total} contre {croupier} ! +{gain} coins !",
    "La main de maitre ! {joueur} bat le croupier {total} a {croupier} !",
    "{joueur} sourit : {total} contre {croupier}. Le croupier range ses cartes.",
    "Bien joue {joueur} ! {total} points suffisent pour terrasser le croupier ({croupier}) !",
    "{joueur} encaisse avec un {total} solide. Le croupier s'incline a {croupier}.",
];

pub(super) const BJ_BUST: &[&str] = &[
    "BUST ! {joueur} a ete trop gourmand ! {total} points... c'est la cata !",
    "{joueur} depasse 21 avec {total} ! Le croupier ricane.",
    "{joueur} pensait que plus c'est haut mieux c'est... {total} points. Perdu.",
    "Aie ! {joueur} explose a {total}. La gourmandise est un vilain defaut.",
    "{joueur} tire une carte de trop et finit a {total}. Classique.",
];

pub(super) const BJ_LOSE: &[&str] = &[
    "Le croupier gagne avec {croupier} contre {total}. -{mise} coins pour {joueur}.",
    "Pas de chance ! Le croupier avait {croupier}. {joueur} rage.",
    "{joueur} fait {total} mais le croupier sort {croupier}. La maison gagne toujours.",
    "Le croupier pose {croupier} avec un sourire narquois. {joueur} et ses {total} points pleurent.",
    "Dommage {joueur} ! {total} contre {croupier}. Le casino se frotte les mains.",
];

/// Pioche une phrase aleatoire dans un pool.
pub(super) fn pick_random(messages: &[&str]) -> String {
    let mut rng = rand::thread_rng();
    let idx = rng.gen_range(0..messages.len());
    messages[idx].to_string()
}
