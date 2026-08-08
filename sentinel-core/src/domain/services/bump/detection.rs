//! Détection de bump/vote réussi (Disboard, DiscordL). Logique pure : elle
//! travaille sur un DTO `BumpMessageFacts` que l'adaptateur (le bot) construit
//! depuis le `Message` Serenity — le core ne connaît pas Discord.
//!
//! Multi-provider (DRY) : chaque plateforme de bump est decrite par une entree
//! `BumpProvider` dans le registre `PROVIDERS`. Ajouter une plateforme = une
//! entree ici + un jeu de cles de config cote API.

/// Action recompensee : bump (remontee de serveur) ou vote.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BumpAction {
    Bump,
    Vote,
}

impl BumpAction {
    /// Mot affiche dans les annonces ("pour le **bump**" / "#3 de la semaine").
    pub fn label(self) -> &'static str {
        match self {
            BumpAction::Bump => "bump",
            BumpAction::Vote => "vote",
        }
    }
}

/// Contenu d'un embed, réduit à ce que la détection consomme.
#[derive(Debug, Clone, Default)]
pub struct EmbedFacts {
    pub title: Option<String>,
    pub description: Option<String>,
    /// (name, value) des champs de l'embed.
    pub fields: Vec<(String, String)>,
}

/// Mention utilisateur présente dans le message.
#[derive(Debug, Clone, Copy)]
pub struct UserFacts {
    pub id: u64,
    pub is_bot: bool,
}

/// Faits extraits d'un message Discord, suffisants pour la détection.
#[derive(Debug, Clone, Default)]
pub struct BumpMessageFacts {
    /// User id de l'auteur du message (le bot provider).
    pub author_id: u64,
    pub embeds: Vec<EmbedFacts>,
    /// Texte brut du message (hors embeds). Certaines plateformes (Discadia)
    /// confirment le bump en TEXTE SIMPLE, sans embed : sans ce champ leur
    /// message serait invisible a la detection.
    pub content: String,
    /// Utilisateur de l'interaction `/bump` (interaction_metadata), si présent.
    pub interaction_user: Option<UserFacts>,
    /// Mentions du CONTENU du message (pas celles des embeds).
    pub mentions: Vec<UserFacts>,
}

/// Description d'une plateforme de bump/vote (Disboard, DiscordL bump/vote, ...).
pub struct BumpProvider {
    /// User id du bot qui poste la confirmation.
    pub bot_id: u64,
    /// Identifiant stable envoye a l'API + namespace de config ("disboard").
    pub key: &'static str,
    /// Nom lisible pour les annonces / rappels.
    pub display: &'static str,
    /// Texte de la commande a rappeler (ex: "/bump (Disboard)").
    pub bump_hint: &'static str,
    /// Action recompensee (bump ou vote) pour les annonces/rappels.
    pub action: BumpAction,
    /// Cooldown par defaut en minutes (indicatif ; l'API tranche).
    pub default_cooldown_min: i64,
    /// Discrimine l'ACTION : ce message correspond-il a ce provider ?
    /// (le meme bot DiscordL poste bump ET vote — on tranche sur le titre).
    pub matches: fn(&BumpMessageFacts) -> bool,
    /// Detection d'un SUCCES (et pas un cooldown/echec) pour ce provider.
    pub detect: fn(&BumpMessageFacts) -> bool,
}

/// Disboard (bot historique, bump uniquement).
pub const DISBOARD: BumpProvider = BumpProvider {
    bot_id: 302050872383242240,
    key: "disboard",
    display: "Disboard",
    bump_hint: "/bump (Disboard)",
    action: BumpAction::Bump,
    default_cooldown_min: 120,
    matches: |_| true,
    detect: detect_disboard,
};

/// DiscordL — bump (discordl.org).
pub const DISCORDL: BumpProvider = BumpProvider {
    bot_id: 528557940811104258,
    key: "discordl",
    display: "DiscordL",
    bump_hint: "/bump (DiscordL)",
    action: BumpAction::Bump,
    default_cooldown_min: 240,
    matches: matches_discordl_bump,
    detect: detect_discordl_bump,
};

/// DiscordL — vote (meme bot, action differente).
pub const DISCORDL_VOTE: BumpProvider = BumpProvider {
    bot_id: 528557940811104258,
    key: "discordl_vote",
    display: "DiscordL",
    bump_hint: "/vote (DiscordL)",
    action: BumpAction::Vote,
    default_cooldown_min: 720,
    matches: matches_discordl_vote,
    detect: detect_discordl_vote,
};

// ── Plateformes a bot_id CONFIGURABLE (French GG, Discadia, top.gg, Spacebump) ─
//
// Leur bot_id n'est pas universel : il se regle par serveur (cle `{key}_bot_id`
// dans la config bump-bot). Tant qu'aucun bot_id n'est configure, elles restent
// inertes (`bot_id: 0` -> aucune correspondance). La detection est GENERIQUE :
// un succes = un embed present et aucun mot de cooldown/echec. C'est ce qui
// fiabilise ces plateformes sans parser leur format exact.

/// Texte pertinent d'un message pour la detection : titres + descriptions des
/// embeds ET le contenu brut. Les CHAMPS d'embed sont volontairement exclus :
/// SpaceBump y met « Cooldown applique : 2h » sur un bump REUSSI, ce qui ferait
/// passer un succes pour un cooldown.
fn detection_text(facts: &BumpMessageFacts) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for e in &facts.embeds {
        if let Some(t) = e.title.as_deref() {
            parts.push(t);
        }
        if let Some(d) = e.description.as_deref() {
            parts.push(d);
        }
    }
    if !facts.content.is_empty() {
        parts.push(&facts.content);
    }
    parts.join(" ").to_lowercase()
}

/// `matches` generique : le bot a poste quelque chose d'exploitable — un embed
/// (Disboard, DiscordL, French GG, SpaceBump...) OU du texte simple (Discadia
/// confirme le bump sans embed). Chaque plateforme ayant un bot_id distinct, pas
/// besoin de discriminer l'action ici.
fn generic_matches(facts: &BumpMessageFacts) -> bool {
    !facts.embeds.is_empty() || !facts.content.trim().is_empty()
}

/// Mots/symboles qui signalent un ECHEC ou un cooldown (donc PAS un bump
/// recompensable) : cooldown en cours, votes epuises, simple invitation a voter.
const FAILURE_WORDS: &[&str] = &[
    // Cooldown / attente
    "wait",
    "minute",
    "patient",
    "cooldown",
    "seconde",
    "second",
    "already",
    "try again",
    "deja",
    "déjà",
    "prochain",
    "heure",
    "hour",
    "attends",
    "encore",
    "trop tot",
    "trop tôt",
    "next bump",
    "come back",
    "revenir",
    "reviens",
    // Votes epuises (DiscordL : « Tu n'as plus de votes ... aujourd'hui »)
    "plus de vote",
    "n'as plus",
    "n as plus",
    "no more vote",
    // Invitation a voter, pas une confirmation (top.gg : « Vote is ready »)
    "is ready",
    "vote for",
    // Symboles d'echec/avertissement, fiables cross-plateforme
    "❌",
    "⚠️",
    "🚫",
];

/// `detect` generique : succes = message exploitable ET aucun signal d'echec.
fn generic_success(facts: &BumpMessageFacts) -> bool {
    if !generic_matches(facts) {
        return false;
    }
    let text = detection_text(facts);
    !FAILURE_WORDS.iter().any(|w| text.contains(w))
}

/// French GG (bot_id configurable).
pub const FRENCH_GG: BumpProvider = BumpProvider {
    bot_id: 0,
    key: "frenchgg",
    display: "French GG",
    bump_hint: "/bump (French GG)",
    action: BumpAction::Bump,
    default_cooldown_min: 120,
    matches: generic_matches,
    detect: generic_success,
};

/// Discadia (bot_id configurable).
pub const DISCADIA: BumpProvider = BumpProvider {
    bot_id: 0,
    key: "discadia",
    display: "Discadia",
    bump_hint: "/bump (Discadia)",
    action: BumpAction::Bump,
    default_cooldown_min: 1440,
    matches: generic_matches,
    detect: generic_success,
};

/// top.gg — vote (bot_id configurable).
pub const TOPGG: BumpProvider = BumpProvider {
    bot_id: 0,
    key: "topgg",
    display: "top.gg",
    bump_hint: "/vote (top.gg)",
    action: BumpAction::Vote,
    default_cooldown_min: 720,
    matches: generic_matches,
    detect: generic_success,
};

/// Spacebump (bot_id configurable).
pub const SPACEBUMP: BumpProvider = BumpProvider {
    bot_id: 0,
    key: "spacebump",
    display: "Spacebump",
    bump_hint: "/bump (Spacebump)",
    action: BumpAction::Bump,
    default_cooldown_min: 360,
    matches: generic_matches,
    detect: generic_success,
};

/// Registre des plateformes supportees.
pub static PROVIDERS: &[BumpProvider] = &[
    DISBOARD,
    DISCORDL,
    DISCORDL_VOTE,
    FRENCH_GG,
    DISCADIA,
    TOPGG,
    SPACEBUMP,
];

/// Provider correspondant a CE message, en utilisant les bot_id CONFIGURES par
/// guild. `bot_id_for(key)` renvoie le bot_id configure (0 = non defini -> on
/// retombe sur le bot_id par defaut du provider, non nul pour Disboard/DiscordL).
/// Un provider ne matche que si son bot_id effectif est non nul ET egal a
/// l'auteur du message ET que son `matches` (action) est satisfait.
pub fn provider_for_message_configured<F: Fn(&str) -> u64>(
    facts: &BumpMessageFacts,
    bot_id_for: F,
) -> Option<&'static BumpProvider> {
    PROVIDERS.iter().find(|p| {
        let configured = bot_id_for(p.key);
        let effective = if configured != 0 {
            configured
        } else {
            p.bot_id
        };
        effective != 0 && effective == facts.author_id && (p.matches)(facts)
    })
}

/// Provider correspondant a CE message via les bot_id PAR DEFAUT uniquement
/// (Disboard/DiscordL). Conserve pour compat/tests ; le bot utilise la variante
/// configuree.
pub fn provider_for_message(facts: &BumpMessageFacts) -> Option<&'static BumpProvider> {
    PROVIDERS
        .iter()
        .find(|p| p.bot_id != 0 && p.bot_id == facts.author_id && (p.matches)(facts))
}

/// Un provider a bot_id PAR DEFAUT (Disboard/DiscordL) poste-t-il avec ce bot_id ?
pub fn is_provider_bot(bot_id: u64) -> bool {
    PROVIDERS
        .iter()
        .any(|p| p.bot_id != 0 && p.bot_id == bot_id)
}

pub fn provider_by_key(key: &str) -> Option<&'static BumpProvider> {
    PROVIDERS.iter().find(|p| p.key == key)
}

/// `true` si l'embed Disboard indique un bump REUSSI (et pas un cooldown/echec).
fn detect_disboard(facts: &BumpMessageFacts) -> bool {
    let mut positive = false;
    for e in &facts.embeds {
        let desc = e.description.as_deref().unwrap_or("").to_lowercase();
        // Echec / cooldown Disboard : "please wait ... minutes", "patienter".
        if desc.contains("minutes") || desc.contains("wait") || desc.contains("patient") {
            return false;
        }
        if desc.contains("done")
            || desc.contains("effectu")
            || desc.contains("👍")
            || desc.contains(":thumbsup:")
        {
            positive = true;
        }
    }
    positive
}

/// `true` si un embed DiscordL contient l'un des motifs (titre ou description).
fn dl_has(facts: &BumpMessageFacts, needles: &[&str]) -> bool {
    facts.embeds.iter().any(|e| {
        let title = e.title.as_deref().unwrap_or("").to_lowercase();
        let desc = e.description.as_deref().unwrap_or("").to_lowercase();
        needles
            .iter()
            .any(|n| title.contains(n) || desc.contains(n))
    })
}

/// `true` si le message DiscordL est un cooldown/echec (pas un succes).
fn dl_is_cooldown(facts: &BumpMessageFacts) -> bool {
    dl_has(
        facts,
        &[
            "wait", "patient", "attends", "prochain", "déjà", "reviens", "already", "minute",
        ],
    )
}

/// Descriptions d'embeds concaténées en minuscules. IMPORTANT : DiscordL met son
/// "titre" en MARKDOWN dans la DESCRIPTION (`### [Résultat du Bump...](url)`) ;
/// le champ `title` de l'embed est VIDE. On matche donc sur la description.
fn dl_desc_lower(facts: &BumpMessageFacts) -> String {
    facts
        .embeds
        .iter()
        .filter_map(|e| e.description.as_deref())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

// Le meme bot DiscordL poste bump ET vote : on tranche sur le mot present dans
// la description ("...du **bump**..." vs "...du **vote**...").
fn matches_discordl_bump(facts: &BumpMessageFacts) -> bool {
    let d = dl_desc_lower(facts);
    d.contains("bump") && !d.contains("vote")
}
fn matches_discordl_vote(facts: &BumpMessageFacts) -> bool {
    dl_desc_lower(facts).contains("vote")
}

/// Marqueur de SUCCES DiscordL : coche verte "✅" ou "a bump/voté", cherche dans
/// titre + description + CHAMPS de l'embed (le texte de succes peut vivre dans un
/// field, pas la description). Distingue un vrai bump d'un cooldown au meme titre.
fn dl_is_success(facts: &BumpMessageFacts) -> bool {
    facts.embeds.iter().any(|e| {
        let mut hay = String::new();
        if let Some(t) = &e.title {
            hay.push_str(t);
            hay.push(' ');
        }
        if let Some(d) = &e.description {
            hay.push_str(d);
            hay.push(' ');
        }
        for (name, value) in &e.fields {
            hay.push_str(name);
            hay.push(' ');
            hay.push_str(value);
            hay.push(' ');
        }
        if hay.contains('\u{2705}') {
            return true; // ✅
        }
        let low = hay.to_lowercase();
        low.contains("a bump") || low.contains("a voté") || low.contains("a vote")
    })
}

/// Bump DiscordL reussi : titre "bump" + marqueur de succes + hors cooldown.
fn detect_discordl_bump(facts: &BumpMessageFacts) -> bool {
    matches_discordl_bump(facts) && dl_is_success(facts) && !dl_is_cooldown(facts)
}
/// Vote DiscordL reussi : titre "vote" + marqueur de succes + hors cooldown.
fn detect_discordl_vote(facts: &BumpMessageFacts) -> bool {
    matches_discordl_vote(facts) && dl_is_success(facts) && !dl_is_cooldown(facts)
}

/// Resout l'auteur du /bump : d'abord via interaction_metadata (reponse de
/// commande), sinon en repli via la premiere mention d'un user non-bot dans la
/// description de l'embed (DiscordL mentionne le bumpeur).
pub fn resolve_bumper(facts: &BumpMessageFacts) -> Option<u64> {
    if let Some(user) = facts.interaction_user {
        if !user.is_bot {
            return Some(user.id);
        }
    }
    // Repli : premiere mention non-bot dans l'embed (si presente dans les mentions).
    for e in &facts.embeds {
        let desc = e.description.as_deref().unwrap_or("");
        for m in &facts.mentions {
            if !m.is_bot && desc.contains(&format!("<@{}>", m.id)) {
                return Some(m.id);
            }
        }
    }
    // Repli cle pour DiscordL : les mentions du message n'incluent PAS celles
    // situees DANS les embeds (seulement celles du contenu). On parse donc
    // `<@id>` directement dans la description/titre de l'embed.
    for e in &facts.embeds {
        for s in [e.description.as_deref(), e.title.as_deref()]
            .into_iter()
            .flatten()
        {
            if let Some(id) = first_mention_id(s) {
                return Some(id);
            }
        }
    }
    // Repli ultime : toute mention non-bot du message.
    facts.mentions.iter().find(|m| !m.is_bot).map(|m| m.id)
}

/// Extrait l'ID de la premiere mention utilisateur `<@id>` / `<@!id>` d'un texte.
pub fn first_mention_id(s: &str) -> Option<u64> {
    let start = s.find("<@")?;
    let rest = s[start + 2..].trim_start_matches('!');
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts_with_desc(author_id: u64, desc: &str) -> BumpMessageFacts {
        BumpMessageFacts {
            author_id,
            embeds: vec![EmbedFacts {
                description: Some(desc.to_string()),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    // ── Disboard ──

    #[test]
    fn disboard_success_done() {
        let f = facts_with_desc(DISBOARD.bot_id, "Bump done! 👍");
        assert!((DISBOARD.detect)(&f));
    }

    #[test]
    fn disboard_success_french() {
        let f = facts_with_desc(DISBOARD.bot_id, "Bump effectué !");
        assert!((DISBOARD.detect)(&f));
    }

    #[test]
    fn disboard_cooldown_rejected() {
        let f = facts_with_desc(DISBOARD.bot_id, "Please wait 42 minutes before bumping");
        assert!(!(DISBOARD.detect)(&f));
    }

    #[test]
    fn disboard_no_embed_rejected() {
        let f = BumpMessageFacts {
            author_id: DISBOARD.bot_id,
            ..Default::default()
        };
        assert!(!(DISBOARD.detect)(&f));
    }

    // ── DiscordL bump vs vote ──

    #[test]
    fn discordl_bump_success() {
        let f = facts_with_desc(
            DISCORDL.bot_id,
            "### [Résultat du Bump](https://x) ✅ <@111> a bump le serveur",
        );
        let p = provider_for_message(&f).unwrap();
        assert_eq!(p.key, "discordl");
        assert!((p.detect)(&f));
    }

    #[test]
    fn discordl_vote_success() {
        let f = facts_with_desc(
            DISCORDL.bot_id,
            "### [Résultat du Vote](https://x) ✅ <@111> a voté pour le serveur",
        );
        let p = provider_for_message(&f).unwrap();
        assert_eq!(p.key, "discordl_vote");
        assert!((p.detect)(&f));
    }

    #[test]
    fn discordl_cooldown_rejected() {
        let f = facts_with_desc(
            DISCORDL.bot_id,
            "Résultat du Bump — tu as déjà bump, reviens dans 42 minutes",
        );
        let p = provider_for_message(&f).unwrap();
        assert!(!(p.detect)(&f));
    }

    #[test]
    fn discordl_success_marker_in_field() {
        let f = BumpMessageFacts {
            author_id: DISCORDL.bot_id,
            embeds: vec![EmbedFacts {
                description: Some("Résultat du Bump".into()),
                fields: vec![("Statut".into(), "✅ succès".into())],
                ..Default::default()
            }],
            ..Default::default()
        };
        let p = provider_for_message(&f).unwrap();
        assert!((p.detect)(&f));
    }

    // ── Registre ──

    #[test]
    fn unknown_bot_no_provider() {
        let f = facts_with_desc(999, "Bump done!");
        assert!(provider_for_message(&f).is_none());
        assert!(!is_provider_bot(999));
        assert!(is_provider_bot(DISBOARD.bot_id));
    }

    #[test]
    fn provider_by_key_lookup() {
        assert_eq!(provider_by_key("disboard").unwrap().bot_id, DISBOARD.bot_id);
        assert!(provider_by_key("inconnu").is_none());
    }

    // ── Plateformes generiques (French GG, Discadia, top.gg, Spacebump) ──

    fn facts_text(content: &str) -> BumpMessageFacts {
        BumpMessageFacts {
            content: content.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn discadia_succes_en_texte_simple() {
        // Discadia confirme le bump SANS embed : la detection doit lire le
        // contenu brut, sinon son bump est invisible.
        let f = facts_text("✅ Bumped La Bande du Canapé !");
        assert!((DISCADIA.detect)(&f));
    }

    #[test]
    fn discadia_cooldown_en_texte_simple_rejete() {
        let f = facts_text("⚠️ Already bumped recently, please try again dans 42 minutes.");
        assert!(!(DISCADIA.detect)(&f));
    }

    #[test]
    fn frenchgg_succes_embed() {
        let f = facts_with_desc(0, "Le serveur La Bande du Canapé a été bumpé !");
        assert!((FRENCH_GG.detect)(&f));
    }

    #[test]
    fn spacebump_cooldown_seulement_en_champ_reste_un_succes() {
        // « Cooldown applique : 2h » vit dans un CHAMP sur un bump reussi : il ne
        // doit pas faire passer le succes pour un cooldown.
        let f = BumpMessageFacts {
            embeds: vec![EmbedFacts {
                title: Some("✅ Bump réussi".into()),
                description: Some("Le serveur La Bande du Canapé a été bump sur SpaceBump.".into()),
                fields: vec![("Cooldown appliqué".into(), "2h (base gratuit : 2h)".into())],
            }],
            ..Default::default()
        };
        assert!((SPACEBUMP.detect)(&f));
    }

    #[test]
    fn topgg_vote_is_ready_n_est_pas_un_succes() {
        // top.gg n'affiche qu'une INVITATION a voter, pas une confirmation.
        let f = facts_with_desc(0, "La Bande du Canapé — Vote is ready");
        assert!(!(TOPGG.detect)(&f));
    }

    #[test]
    fn votes_epuises_rejete() {
        let f = facts_with_desc(
            0,
            "❌ Tu n'as plus de votes pour La Bande du Canapé aujourd'hui",
        );
        assert!(!(SPACEBUMP.detect)(&f));
    }

    // ── resolve_bumper ──

    #[test]
    fn bumper_from_interaction() {
        let f = BumpMessageFacts {
            interaction_user: Some(UserFacts {
                id: 42,
                is_bot: false,
            }),
            ..Default::default()
        };
        assert_eq!(resolve_bumper(&f), Some(42));
    }

    #[test]
    fn bumper_interaction_bot_ignored() {
        let f = BumpMessageFacts {
            interaction_user: Some(UserFacts {
                id: 42,
                is_bot: true,
            }),
            ..Default::default()
        };
        assert_eq!(resolve_bumper(&f), None);
    }

    #[test]
    fn bumper_from_embed_mention_in_message_mentions() {
        let f = BumpMessageFacts {
            embeds: vec![EmbedFacts {
                description: Some("<@77> a bump".into()),
                ..Default::default()
            }],
            mentions: vec![UserFacts {
                id: 77,
                is_bot: false,
            }],
            ..Default::default()
        };
        assert_eq!(resolve_bumper(&f), Some(77));
    }

    #[test]
    fn bumper_parsed_from_embed_text_when_not_in_mentions() {
        // Cas DiscordL : la mention n'est QUE dans l'embed.
        let f = BumpMessageFacts {
            embeds: vec![EmbedFacts {
                description: Some("✅ <@123456789> a bump le serveur".into()),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert_eq!(resolve_bumper(&f), Some(123456789));
    }

    #[test]
    fn first_mention_id_variants() {
        assert_eq!(first_mention_id("hello <@123> world"), Some(123));
        assert_eq!(first_mention_id("<@!456>"), Some(456));
        assert_eq!(first_mention_id("no mention"), None);
        assert_eq!(first_mention_id("<@abc>"), None);
    }
}
