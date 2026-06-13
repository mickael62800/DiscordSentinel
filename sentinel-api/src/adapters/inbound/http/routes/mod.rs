//! Per-domain route modules. Each exposes `pub fn routes() -> Router<AppState>`
//! returning already-prefixed (nested) routes ready to be merged into the main
//! router. `analytics` is special: its `inner()` is re-used by `build()` to
//! apply the heavy-rate-limiter layer without double-nesting.

pub mod analytics;
pub mod audit;
pub mod automod;
pub mod bot;
pub mod bot_persistence;
pub mod coude;
pub mod dashboard;
pub mod game_portal;
pub mod games;
pub mod members;
pub mod moderation;
pub mod progression;
pub mod security;
pub mod stats;
pub mod system;
pub mod tamagotchi;
pub mod ticket;
pub mod voice_channels;
