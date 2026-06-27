//! Helpers pour le pattern defer + followup utilise par les slash
//! commands qui enchainent plusieurs appels API (>3s au total).
//!
//! Discord coupe une interaction apres 3 secondes sans reponse et
//! affiche "L'interaction a echoue" — le joueur voit une "erreur".
//! Pour les commandes qui font >= 3 appels API, on defer immediatement
//! (15 min de delai) puis on utilise create_followup pour la reponse
//! finale.
//!
//! Usage type dans un handler :
//!
//! ```ignore
//! pub async fn handle(ctx: &Context, command: &CommandInteraction) {
//!     if !defer_response(ctx, command).await {
//!         return;
//!     }
//!     // … appels API …
//!     if error_case {
//!         followup_text(ctx, command, "Pas assez de coins").await;
//!         return;
//!     }
//!     followup_embed(ctx, command, embed).await;
//! }
//! ```

use serenity::all::{
    CommandInteraction, Context, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
};

/// Defere la reponse a l'interaction (message non-ephemeral par defaut).
/// Retourne `false` si le defer a echoue — dans ce cas le handler doit
/// retourner tout de suite.
pub async fn defer_response(ctx: &Context, command: &CommandInteraction) -> bool {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(false),
            ),
        )
        .await
        .map(|_| true)
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, cmd = %command.data.name, "Echec defer");
            false
        })
}

/// Defere en mode ephemeral (pour les commandes "silent" comme /protection).
pub async fn defer_ephemeral(ctx: &Context, command: &CommandInteraction) -> bool {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
        .map(|_| true)
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, cmd = %command.data.name, "Echec defer ephemeral");
            false
        })
}

/// Followup ephemeral de feedback (erreur/info) apres un defer. N'apparait que
/// pour l'utilisateur qui a lance la commande. Rendu en embed colore selon la
/// nature du message (cf. `embeds::feedback_embed` : ✅ vert / ⚠️ orange /
/// ❌ rouge / neutre), pour distinguer visuellement succes, attente et erreur.
pub async fn followup_text(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new()
                .embed(crate::shared::embeds::feedback_embed(content))
                .ephemeral(true),
        )
        .await
    {
        tracing::warn!(error = %e, cmd = %command.data.name, "Echec followup_text");
    }
}

/// Followup embed public — pour la reponse de succes visible a tous.
pub async fn followup_embed(ctx: &Context, command: &CommandInteraction, embed: CreateEmbed) {
    if let Err(e) = command
        .create_followup(&ctx.http, CreateInteractionResponseFollowup::new().embed(embed))
        .await
    {
        tracing::warn!(error = %e, cmd = %command.data.name, "Echec followup_embed");
    }
}

/// Followup embed ephemeral — pour des reponses detaillees mais cachees.
#[allow(dead_code)]
pub async fn followup_embed_ephemeral(
    ctx: &Context,
    command: &CommandInteraction,
    embed: CreateEmbed,
) {
    if let Err(e) = command
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new()
                .embed(embed)
                .ephemeral(true),
        )
        .await
    {
        tracing::warn!(error = %e, cmd = %command.data.name, "Echec followup_embed_ephemeral");
    }
}
