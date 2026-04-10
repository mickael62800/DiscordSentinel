//! Helpers pour configurer le cache Serenity de manière restrictive.
//!
//! Phase 1 — Quick wins (cf. `docs/ROADMAP.md` § Phase 1) : par défaut Serenity
//! cache **toutes** les guilds + leurs channels + tous les messages vus, ce qui
//! fait exploser la RAM des bots qui n'ont pas besoin de tout ça (presque tous).
//!
//! Ces helpers exposent des `CacheSettings` plus parcimonieux selon le profil
//! du bot. Gain typique : **-30 à -60 % de RAM** par bot Discord.
//!
//! Usage type dans le `main.rs` du bot :
//!
//! ```ignore
//! use sentinel_shared::cache_settings;
//!
//! let mut client = serenity::Client::builder(token, intents)
//!     .event_handler(Handler)
//!     .cache_settings(cache_settings::minimal())
//!     .await
//!     .expect("Erreur creation client");
//! ```

use serenity::cache::Settings as CacheSettings;

/// Cache **minimal** : convient à 90% des bots qui ne lisent presque jamais
/// le cache (la plupart de nos bots font un round-trip API à chaque action).
///
/// - `max_messages = 0` : ne cache aucun message (gain énorme — un message
///   moyen pèse ~200 octets, multiplié par des milliers de canaux ça monte vite)
/// - `cache_guilds = true` : on garde les guilds (nécessaire pour le routing)
/// - `cache_channels = false` : pas de cache channels (on les fetch à la demande)
/// - `cache_users = false` : pas de cache users (on les fetch à la demande)
///
/// À utiliser pour : audit, blackjack, cleanup, game, image, progression,
/// roles, security, ticket, welcome.
pub fn minimal() -> CacheSettings {
    let mut s = CacheSettings::default();
    s.max_messages = 0;
    s.cache_guilds = true;
    s.cache_channels = false;
    s.cache_users = false;
    s
}

/// Cache **petit** : pour les bots qui lisent occasionnellement les channels
/// (ex : automod a besoin du nom du channel pour les logs).
///
/// - `max_messages = 0`
/// - `cache_guilds = true`
/// - `cache_channels = true`
/// - `cache_users = false`
pub fn small() -> CacheSettings {
    let mut s = CacheSettings::default();
    s.max_messages = 0;
    s.cache_guilds = true;
    s.cache_channels = true;
    s.cache_users = false;
    s
}

/// Cache **moyen** : pour les bots qui ont besoin du cache messages pour
/// reconstituer le contexte d'une suppression / édition (audit-bot, automod-bot
/// pour la fenêtre de spam-detection, voice-bot pour les session cards).
///
/// - `max_messages = 100` (par channel) : permet de retrouver le contenu des
///   messages supprimés très récents sans appel API
/// - `cache_guilds = true`
/// - `cache_channels = true`
/// - `cache_users = false`
pub fn medium() -> CacheSettings {
    let mut s = CacheSettings::default();
    s.max_messages = 100;
    s.cache_guilds = true;
    s.cache_channels = true;
    s.cache_users = false;
    s
}

/// Cache **complet** : laisse le défaut Serenity. Pour les bots qui ont
/// vraiment besoin de tout (voice-bot pour résoudre les `voice_states`,
/// moderation-bot pour les permissions des modérateurs).
///
/// Préfère `medium()` ou `small()` quand c'est possible — `full()` est le plus
/// gourmand.
pub fn full() -> CacheSettings {
    CacheSettings::default()
}
