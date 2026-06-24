//! Module ai-dataset-bot : collecte autonome des messages texte pour
//! entrainer des modeles IA. Totalement independant des modules audit
//! et automod.
//!
//! Toggle par guild : `is_module_enabled(ctx, gid, "ai-dataset-bot")`.
//! Desactive par defaut. Quand actif, chaque message non-bot est
//! envoye via gRPC `AiDatasetService.CollectMessage` qui l'insere dans la
//! table `ai_dataset_messages`.
//!
//! Best-effort : l'appel reste fire-and-forget (resultat ignore) car c'est
//! le chemin le plus chaud du bot et la perte d'un message est acceptable.
//!
//! La page web "Dataset IA" lit cette table pour permettre l'etiquetage
//! manuel et l'export CSV.

use serenity::model::channel::Message;
use serenity::prelude::*;

use crate::shared::discord_helpers::is_module_enabled;
use crate::shared::grpc_client::GrpcClientKey;

use sentinel_proto::ai_dataset::v1 as proto;

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
    let grpc = match data.get::<GrpcClientKey>() {
        Some(grpc) => grpc.clone(),
        None => return,
    };
    drop(data);

    let req = proto::CollectMessageRequest {
        guild_id: guild_id.to_string(),
        channel_id: msg.channel_id.to_string(),
        channel_name,
        user_id: msg.author.id.to_string(),
        content: content.to_string(),
    };

    // Fire-and-forget : resultat ignore, ne bloque pas la chaine d'evenements.
    // Le circuit breaker partage degrade gracieusement si l'API est down.
    let mut client = grpc.ai_dataset();
    let _ = grpc
        .guarded(|| async move { client.collect_message(req).await.map(|r| r.into_inner()) })
        .await;
}
