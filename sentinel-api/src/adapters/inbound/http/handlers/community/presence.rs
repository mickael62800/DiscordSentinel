//! Presence en direct, surface publique.
//!
//! # Ce qui est publie et pourquoi
//!
//! Seuls les salons visibles par @everyone remontent ici — le filtrage est
//! fait par le bot, qui seul connait les permissions Discord. L'API ne peut
//! pas le refaire et ne doit surtout pas le contourner.
//!
//! Le DTO expose les pseudos mais PAS les identifiants Discord, comme les
//! autres surfaces publiques : un pseudo suffit a afficher une pastille,
//! l'identifiant permettrait de retrouver la personne hors du serveur.
//!
//! Une section vide est le cas normal (personne en vocal, bot redemarre,
//! Redis indisponible). Elle ne remonte jamais d'erreur : la page membre doit
//! s'afficher entiere meme quand cette brique est muette.

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::handlers::community::public_guard::ensure_guild_id;
use crate::adapters::inbound::http::state::AppState;

/// Salons ecrits remontes. Au-dela, la liste cesse d'informer.
const TEXT_CHANNELS: i64 = 5;

#[derive(Debug, Serialize)]
pub struct VoiceMemberDto {
    pub username: String,
    /// Micro coupe, quelle qu'en soit la cause. La page n'a pas besoin de
    /// distinguer une coupure volontaire d'une sanction — et l'afficher
    /// exposerait une decision de moderation.
    pub muted: bool,
    pub streaming: bool,
    pub video: bool,
}

#[derive(Debug, Serialize)]
pub struct VoiceChannelDto {
    pub channel_name: String,
    pub members: Vec<VoiceMemberDto>,
}

#[derive(Debug, Serialize)]
pub struct TextChannelDto {
    pub channel_name: String,
    pub recent_authors: Vec<String>,
    pub last_message_at: String,
}

#[derive(Debug, Serialize)]
pub struct PresenceDto {
    pub voice: Vec<VoiceChannelDto>,
    pub voice_total: usize,
    pub text: Vec<TextChannelDto>,
}

/// GET /api/public/presence/{guild_id}
pub async fn public_presence(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<PresenceDto>, ApiError> {
    ensure_guild_id(&guild_id)?;

    let presence = state.presence_uc.voice(&guild_id).await?;
    let text = state
        .presence_uc
        .text_activity(&guild_id, TEXT_CHANNELS)
        .await?;

    let (voice, voice_total) = match presence {
        Some(p) => {
            let total = p.total_members();
            let salons = p
                .occupied_channels()
                .into_iter()
                .map(|c| VoiceChannelDto {
                    channel_name: c.channel_name.clone(),
                    members: c
                        .members
                        .iter()
                        .map(|m| VoiceMemberDto {
                            username: m.username.clone(),
                            muted: !m.can_speak(),
                            streaming: m.streaming,
                            video: m.video,
                        })
                        .collect(),
                })
                .collect();
            (salons, total)
        }
        // Instantane absent ou perime : on renvoie du vide plutot qu'une
        // erreur, la page masquera simplement la section.
        None => (vec![], 0),
    };

    Ok(Json(PresenceDto {
        voice,
        voice_total,
        text: text
            .into_iter()
            .map(|t| TextChannelDto {
                channel_name: t.channel_name,
                recent_authors: t.recent_authors,
                last_message_at: t.last_message_at.to_rfc3339(),
            })
            .collect(),
    }))
}
