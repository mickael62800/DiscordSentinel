//! Spectateurs fictifs (cf. COUPE_AMELIORATIONS section 2.5).
//!
//! A la fin d un combat, le bot poste 3-5 faux commentaires de "spectateurs"
//! avec des pseudos absurdes. Zero mecanique : pure ambiance pour donner
//! l illusion qu il y a une foule qui regarde le combat.
//!
//! Logique pure — testable sans IO.

use rand::seq::SliceRandom;
use rand::Rng;

pub const SPECTATOR_COUNT_MIN: usize = 3;
pub const SPECTATOR_COUNT_MAX: usize = 5;

/// Catalogue de pseudos absurdes.
pub const SPECTATOR_USERNAMES: &[&str] = &[
    "Kevin_Le_Hooligan", "LaReine_du_Troll", "Max_69_420", "GrandMama",
    "Sheep_Lv_1", "Jean-Mi_69", "BobLeBricoleur", "Fortnite_GOAT_07",
    "Marie-Cherie", "Dudule_94", "TacosForever", "PapyDescentes",
    "ClovisLeChat", "MamieBingo", "Saucisse_Mvp", "GibierDuDimanche",
    "Rocket_Dinosaur", "Nicolas_Sarkozy", "VivienneLaPunaise", "Cucuthe6",
    "Jeremy_Vegan", "BernardLaPlante", "MikeAuBifteck", "PetiteCanaille",
    "Roger_Rabbit_42", "BrunoLeFort", "Doudou_McRib", "Lola_Soigneuse",
    "PoulpyLaMain", "Genevieve_Lol", "Mickey_Mouseries", "Sandwich_Boy",
    "MisterTaupe", "ChrisFromMars", "TheoXBox", "BambineEclair",
    "GabrielleAnxieuse", "PrudenceTheCat", "Banjo69", "ZazaTroFort",
    "Kevin_v2", "Alphonse_Patapouf", "MiniGreg", "AmandineFougasse",
    "RogerToiTrop", "Lulu_GTA6", "Gabin_Macaroni", "Laurine_Pinkmoon",
    "Brutus_Ier", "Patrick_Etoile", "Honey_Bnouille", "Mathilde_Latte",
    "PiwiCrispy", "VladTheTiktoker", "Cocci_Nelle", "Brice_de_Nice",
    "Jambonette42", "DanielLeForain", "Kiwi_Bandit", "Jean-Claude_Rip",
    "Steeve_Pignouf", "GogoLeMagnifique", "Ronaldo_Le_Frigo", "MorpionDuVar",
    "BarbieDuPS5", "Maurice_Stronk", "RobertaBzz", "Toto_LeChef",
    "Fanfan_la_Tulipe", "Bambou_Croquant", "PetainsPasFrais", "Jean-Phi_Twitch",
    "VivianeMarmot", "RaphaelLeFlegme", "MikadoLeBeau", "Suzy_Brocoli",
    "Pierrot_Vegeta", "FlorianLaGalette", "Coquin_DuMidi", "Alpha_Patapouf",
    "Doris_Gluten", "Tortue_Crispy", "EnricoMacareux", "Marlon_Foxnews",
    "Christine_Boomy", "Rambo_Le_Trotro", "Julie_Le_Dunk", "GilouLeChiqueur",
    "Sherlock_Bruh", "BlanchetteLourd", "Gandalf_Le_69", "Yves_Le_Diabolo",
    "Mireille_Pompon", "Domino_Croute", "Steeve_Tortilla", "BernieDuStream",
    "Madame_Mim", "Marcel_Nutella", "ZouzouLazer", "PrunelleLeRogue",
];

/// Catalogue de phrases types — `{atk}` / `{def}` substitues, `{winner}` /
/// `{loser}` aussi (peuvent etre vides en cas de match nul).
pub const SPECTATOR_LINES: &[&str] = &[
    "MDRRRR j avais mise sur {atk} 😭",
    "{def} c est un clodo on le savait",
    "KEKW",
    "j ai gagne 40 coins merci",
    "je retire tout",
    "OMEGALUL",
    "ce combat c est un televrac",
    "{winner} GOAT",
    "{loser} c est honteux la",
    "qui a vu le bouton « parier sur le perdant »",
    "j ai mise toute ma retraite sur {def}",
    "putain les gars c etait epique",
    "moi j etais la quand {atk} a fait nimporte quoi en round 2",
    "PEPELAUGH",
    "j ai re-mise tout pour la prochaine",
    "le bot triche, il sucre les jets",
    "mon medecin dit que je dois arreter",
    "MAMAN J AI GAGNE 200 COINS",
    "je depose plainte au tribunal",
    "5 etoiles je recommande ce combat",
    "le pop-corn etait bon en tout cas",
    "{atk} stp arrete de jouer tu nous fais honte",
    "cest moi ou {def} avait l air motive cette fois",
    "wp les gars",
    "premier degat de la decennie",
    "qui a parie 1 coin svp 😂",
    "j hesite a quitter ce serveur honnetement",
    "vous valez rien franchement",
    "GG WP",
    "MEILLEUR COMBAT DE L ANNEE",
    "un combat sponsorise par Boursorama",
    "le commentateur a abandonne au round 3",
    "{winner} je veux tes coordonnees IBAN",
    "perso je le savais",
    "POG",
    "ALLEZ COMME D HAB",
    "{loser} repos demain",
    "F dans le chat pour {loser}",
    "j ai jamais autant ri",
    "je hais ce serveur",
];

/// Tirage stable de N pseudos distincts (entre MIN et MAX) avec leur ligne.
/// Retourne Vec<(username, message)>.
pub fn pick_spectator_chat(
    rng: &mut impl Rng,
    attacker_name: &str,
    defender_name: &str,
    winner_name: Option<&str>,
    loser_name: Option<&str>,
) -> Vec<(String, String)> {
    let count = rng.gen_range(SPECTATOR_COUNT_MIN..=SPECTATOR_COUNT_MAX);
    let mut shuffled_users: Vec<&&str> = SPECTATOR_USERNAMES.iter().collect();
    shuffled_users.shuffle(rng);

    let win = winner_name.unwrap_or("");
    let lose = loser_name.unwrap_or("");

    (0..count)
        .map(|i| {
            let user = shuffled_users
                .get(i)
                .map(|s| (**s).to_string())
                .unwrap_or_else(|| format!("Spectateur_{i}"));
            let line_tmpl = SPECTATOR_LINES[rng.gen_range(0..SPECTATOR_LINES.len())];
            let line = line_tmpl
                .replace("{atk}", attacker_name)
                .replace("{def}", defender_name)
                .replace("{winner}", win)
                .replace("{loser}", lose);
            (user, line)
        })
        .collect()
}

/// Formatage du chat pour insertion dans un embed Discord.
pub fn format_spectator_chat(chat: &[(String, String)]) -> String {
    chat.iter()
        .map(|(user, line)| format!("💬 [{}] : {}", user, line))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
#[path = "tests/fake_spectators.rs"]
mod tests;
