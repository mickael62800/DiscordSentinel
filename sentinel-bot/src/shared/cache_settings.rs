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
//! use crate::shared::cache_settings;
//!
//! let mut client = serenity::Client::builder(token, intents)
//!     .event_handler(Handler)
//!     .cache_settings(cache_settings::minimal())
//!     .await
//!     .expect("Erreur creation client");
//! ```

use serenity::cache::Settings as CacheSettings;

/// Cache **complet** : laisse le défaut Serenity. Pour les bots qui ont
/// vraiment besoin de tout (voice-bot pour résoudre les `voice_states`,
/// moderation-bot pour les permissions des modérateurs).
///
/// Préfère `medium()` ou `small()` quand c'est possible — `full()` est le plus
/// gourmand.
pub fn full() -> CacheSettings {
    CacheSettings::default()
}
