//! Module ai-dataset-bot : collecte autonome des messages texte pour
//! entrainer des modeles IA. Totalement independant des modules audit
//! et automod.
//!
//! Toggle par guild : `is_module_enabled(ctx, gid, "ai-dataset-bot")`.
//! Desactive par defaut. Quand actif, chaque message non-bot est
//! envoye a `POST /api/ai-dataset/collect` qui l'insere dans la table
//! `ai_dataset_messages`.
//!
//! La page web "Dataset IA" lit cette table pour permettre l'etiquetage
//! manuel et l'export CSV.

use serenity::model::channel::Message;
use serenity::prelude::*;

use crate::shared::discord_helpers::is_module_enabled;
use crate::shared::heartbeat::ApiClientKey;

pub const MODULE_BOT_NAME: &str = "ai-dataset-bot";

/// Insere chaque message texte dans la table ai_dataset_messages si le
/// module est active sur la guild. Ignore les messages vides et ceux
/// trop longs (Discord cap deja a 4000 chars, mais on garde une marge).
pub async fn on_message(ctx: &Context, msg: &Message) {
    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return, // Ignorer les DMs
    };

    // Filtre rapide avant de payer le cout de la requete config.
    let content = msg.content.trim();
    if content.is_empty() {
        return;
    }

    if !is_module_enabled(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await {
        return;
    }

    // Resout le nom du salon (best-effort, ne bloque pas si echoue).
    let channel_name = msg
        .channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild())
        .map(|c| c.name.clone());

    let data = ctx.data.read().await;
    let api = match data.get::<ApiClientKey>() {
        Some(api) => api.clone(),
        None => return,
    };
    drop(data);

    let payload = serde_json::json!({
        "guild_id": guild_id.to_string(),
        "channel_id": msg.channel_id.to_string(),
        "channel_name": channel_name,
        "user_id": msg.author.id.to_string(),
        "content": content,
    });

    api.post_fire_and_forget("/api/ai-dataset/collect", &payload).await;
}
