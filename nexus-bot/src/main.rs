//! # nexus-bot — bot Discord de la plateforme jeux Nexus
//!
//! Serenity minimal, calque sur l'architecture de `sentinel-bot` : le bot
//! n'a AUCUN acces DB, il passe par nexus-api (client HTTP Bearer).
//!
//! Commande : `/roue` — 1 spin de la Roue du Destin par joueur par jour.
//!
//! Env :
//!   - NEXUS_DISCORD_TOKEN (sans lui : log + exit propre, comme le scaffold)
//!   - NEXUS_API_URL (defaut http://localhost:3100)
//!   - NEXUS_API_KEY (Bearer vers nexus-api)

mod api_client;
mod embeds;

use std::sync::Arc;

use serenity::all::Command;
use serenity::all::CommandInteraction;
use serenity::all::Context;
use serenity::all::CreateCommand;
use serenity::all::CreateInteractionResponse;
use serenity::all::CreateInteractionResponseMessage;
use serenity::all::EventHandler;
use serenity::all::GatewayIntents;
use serenity::all::Interaction;
use serenity::all::Ready;
use serenity::async_trait;
use serenity::Client;

use api_client::ApiClient;

struct Handler {
    api: Arc<ApiClient>,
}

impl Handler {
    async fn handle_roue(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild_id) = cmd.guild_id else {
            let msg = CreateInteractionResponseMessage::new()
                .embed(embeds::build_error_embed("La Roue se tire sur un serveur, pas en MP."))
                .ephemeral(true);
            let _ = cmd
                .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
                .await;
            return;
        };

        // Defer : l'appel API peut prendre > 3s (cold start).
        if let Err(e) = cmd.defer(&ctx.http).await {
            tracing::error!("defer /roue impossible: {e}");
            return;
        }

        let username = cmd.user.display_name().to_string();
        let response = self
            .api
            .spin_wheel(&guild_id.to_string(), &cmd.user.id.to_string(), &username)
            .await;

        let embed = match &response {
            Ok(resp) => embeds::build_result_embed(resp, &username),
            Err(msg) => embeds::build_error_embed(msg),
        };
        let builder = serenity::all::CreateInteractionResponseFollowup::new().embed(embed);
        if let Err(e) = cmd.create_followup(&ctx.http, builder).await {
            tracing::error!("followup /roue impossible: {e}");
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!("nexus-bot connecte en tant que {}", ready.user.name);
        let roue = CreateCommand::new("roue")
            .description("Tire la Roue du Destin — 1 spin par jour, le destin decide.");
        if let Err(e) = Command::create_global_command(&ctx.http, roue).await {
            tracing::error!("enregistrement /roue impossible: {e}");
        } else {
            tracing::info!("commande slash /roue enregistree (globale)");
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(cmd) = interaction {
            if cmd.data.name == "roue" {
                self.handle_roue(&ctx, &cmd).await;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let Ok(token) = std::env::var("NEXUS_DISCORD_TOKEN") else {
        tracing::info!("NEXUS_DISCORD_TOKEN absent — arret (pas de connexion Discord)");
        return;
    };

    let api_url =
        std::env::var("NEXUS_API_URL").unwrap_or_else(|_| "http://localhost:3100".to_string());
    let api_key = std::env::var("NEXUS_API_KEY").ok().filter(|k| !k.is_empty());
    let api = Arc::new(ApiClient::new(api_url, api_key));

    let mut client = Client::builder(&token, GatewayIntents::non_privileged())
        .event_handler(Handler { api })
        .await
        .expect("creation du client serenity");
    if let Err(e) = client.start().await {
        tracing::error!("erreur client nexus-bot: {e}");
    }
}
