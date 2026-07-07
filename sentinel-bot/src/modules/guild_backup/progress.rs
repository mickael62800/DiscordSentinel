//! Abstraction de feedback de progression pour capture / restore / wipe.
//!
//! Deux impls partagent la meme surface `set(text)` :
//! - [`ProgressSink::Interaction`] : chemin slash-command — edite le message
//!   deferre de l'interaction Discord (comportement historique INCHANGE).
//! - [`ProgressSink::Headless`] : chemin event-driven (pilotage web via Redis)
//!   — aucune interaction Discord disponible, on logge simplement l'avancement
//!   via tracing (best-effort).

use serenity::all::{ComponentInteraction, Context, EditInteractionResponse};
use tracing::info;

/// Puits de progression : abstrait la destination du feedback d'avancement.
pub enum ProgressSink<'a> {
    /// Slash-command : edite le message deferre de l'interaction.
    Interaction {
        ctx: &'a Context,
        component: &'a ComponentInteraction,
    },
    /// Event-driven (pilotage web) : log tracing, pas de feedback Discord live.
    Headless { guild_id: String },
}

impl<'a> ProgressSink<'a> {
    /// Construit un sink attache a une interaction Discord (chemin slash-command).
    pub fn interaction(ctx: &'a Context, component: &'a ComponentInteraction) -> Self {
        Self::Interaction { ctx, component }
    }

    /// Construit un sink headless (chemin event-driven) pour une guild.
    pub fn headless(guild_id: impl Into<String>) -> Self {
        Self::Headless {
            guild_id: guild_id.into(),
        }
    }

    /// Publie une ligne de progression (best-effort, jamais bloquant).
    pub async fn set(&self, text: &str) {
        match self {
            Self::Interaction { ctx, component } => {
                let _ = component
                    .edit_response(&ctx.http, EditInteractionResponse::new().content(text))
                    .await;
            }
            Self::Headless { guild_id } => {
                info!(guild = %guild_id, progress = %text, "guild_backup(headless): progression");
            }
        }
    }
}
