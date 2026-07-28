//! # nexus-bot — bot Discord de la plateforme jeux Nexus
//!
//! Serenity minimal, calque sur l'architecture de `sentinel-bot` : le bot
//! n'a AUCUN acces DB, il passe par nexus-api (client HTTP Bearer).
//!
//! Commandes :
//!   - `/roue` — 1 spin de la Roue du Destin par joueur par jour.
//!   - `/solde [membre]` — consulte son portefeuille (ou celui d'un autre).
//!   - `/donner <membre> <montant> [raison]` — transfert de coins.
//!   - `/classement` — top 10 des plus riches du serveur.
//!
//! Env :
//!   - NEXUS_DISCORD_TOKEN (sans lui : log + exit propre, comme le scaffold)
//!   - NEXUS_API_URL (defaut http://localhost:3100)
//!   - NEXUS_API_KEY (Bearer vers nexus-api)

mod api_client;
mod embeds;

use std::sync::Arc;

use serenity::all::Command;
use serenity::all::CommandDataOptionValue;
use serenity::all::CommandInteraction;
use serenity::all::CommandOptionType;
use serenity::all::Context;
use serenity::all::CreateCommand;
use serenity::all::CreateCommandOption;
use serenity::all::CreateInteractionResponse;
use serenity::all::CreateInteractionResponseMessage;
use serenity::all::UserId;
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

/// Extrait une option user par nom.
fn option_user(cmd: &CommandInteraction, name: &str) -> Option<UserId> {
    cmd.data.options.iter().find(|o| o.name == name).and_then(|o| match &o.value {
        CommandDataOptionValue::User(id) => Some(*id),
        _ => None,
    })
}

/// Extrait une option entiere par nom.
fn option_integer(cmd: &CommandInteraction, name: &str) -> Option<i64> {
    cmd.data.options.iter().find(|o| o.name == name).and_then(|o| match &o.value {
        CommandDataOptionValue::Integer(v) => Some(*v),
        _ => None,
    })
}

/// Extrait une option string par nom.
fn option_string(cmd: &CommandInteraction, name: &str) -> Option<String> {
    cmd.data.options.iter().find(|o| o.name == name).and_then(|o| match &o.value {
        CommandDataOptionValue::String(s) => Some(s.clone()),
        _ => None,
    })
}

impl Handler {
    /// Reponse ephemere avec l'embed d'erreur standard.
    async fn reply_error(&self, ctx: &Context, cmd: &CommandInteraction, message: &str) {
        let msg = CreateInteractionResponseMessage::new()
            .embed(embeds::build_error_embed(message))
            .ephemeral(true);
        let _ = cmd
            .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
            .await;
    }

    /// Exige un serveur : retourne le guild_id ou repond une erreur ephemere.
    async fn require_guild(&self, ctx: &Context, cmd: &CommandInteraction) -> Option<String> {
        match cmd.guild_id {
            Some(g) => Some(g.to_string()),
            None => {
                self.reply_error(ctx, cmd, "Cette commande s'utilise sur un serveur, pas en MP.")
                    .await;
                None
            }
        }
    }

    async fn handle_solde(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild_id) = self.require_guild(ctx, cmd).await else {
            return;
        };

        let target_id = option_user(cmd, "membre").unwrap_or(cmd.user.id);
        let display_name = if target_id == cmd.user.id {
            cmd.user.display_name().to_string()
        } else {
            cmd.data
                .resolved
                .users
                .get(&target_id)
                .map(|u| u.display_name().to_string())
                .unwrap_or_else(|| format!("<@{target_id}>"))
        };

        if let Err(e) = cmd.defer(&ctx.http).await {
            tracing::error!("defer /solde impossible: {e}");
            return;
        }

        let response = self.api.get_wallet(&guild_id, &target_id.to_string()).await;
        let embed = match &response {
            Ok(w) => embeds::build_wallet_embed(w, &display_name),
            Err(msg) => embeds::build_error_embed(msg),
        };
        let builder = serenity::all::CreateInteractionResponseFollowup::new().embed(embed);
        if let Err(e) = cmd.create_followup(&ctx.http, builder).await {
            tracing::error!("followup /solde impossible: {e}");
        }
    }

    async fn handle_donner(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild_id) = self.require_guild(ctx, cmd).await else {
            return;
        };

        let Some(target_id) = option_user(cmd, "membre") else {
            self.reply_error(ctx, cmd, "Indique le membre a qui donner.").await;
            return;
        };
        let Some(amount) = option_integer(cmd, "montant") else {
            self.reply_error(ctx, cmd, "Indique le montant du don.").await;
            return;
        };
        let reason = option_string(cmd, "raison");

        // Pre-checks UI rapides (la regle de verite reste cote core/API :
        // auto-transfert, montant > 0, solde suffisant sans clamp).
        if target_id == cmd.user.id {
            self.reply_error(ctx, cmd, "Tu ne peux pas te donner a toi-meme !").await;
            return;
        }
        let target_user = cmd.data.resolved.users.get(&target_id);
        if target_user.is_some_and(|u| u.bot) {
            self.reply_error(ctx, cmd, "Tu ne peux pas donner a un bot !").await;
            return;
        }
        let target_username = target_user
            .map(|u| u.display_name().to_string())
            .unwrap_or_default();

        if let Err(e) = cmd.defer(&ctx.http).await {
            tracing::error!("defer /donner impossible: {e}");
            return;
        }

        let response = self
            .api
            .transfer_coins(
                &guild_id,
                &api_client::TransferRequest {
                    from_user_id: cmd.user.id.to_string(),
                    from_username: cmd.user.display_name().to_string(),
                    to_user_id: target_id.to_string(),
                    to_username: target_username,
                    amount,
                    reason: reason.clone(),
                },
            )
            .await;

        let embed = match &response {
            Ok(res) => embeds::build_transfer_embed(
                cmd.user.id.get(),
                target_id.get(),
                res.amount,
                res.from_balance,
                reason.as_deref(),
            ),
            Err(msg) => embeds::build_error_embed(msg),
        };
        let builder = serenity::all::CreateInteractionResponseFollowup::new().embed(embed);
        if let Err(e) = cmd.create_followup(&ctx.http, builder).await {
            tracing::error!("followup /donner impossible: {e}");
        }
    }

    async fn handle_classement(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild_id) = self.require_guild(ctx, cmd).await else {
            return;
        };

        if let Err(e) = cmd.defer(&ctx.http).await {
            tracing::error!("defer /classement impossible: {e}");
            return;
        }

        let response = self.api.wallet_leaderboard(&guild_id, 10).await;
        let embed = match &response {
            Ok(entries) => embeds::build_leaderboard_embed(entries),
            Err(msg) => embeds::build_error_embed(msg),
        };
        let builder = serenity::all::CreateInteractionResponseFollowup::new().embed(embed);
        if let Err(e) = cmd.create_followup(&ctx.http, builder).await {
            tracing::error!("followup /classement impossible: {e}");
        }
    }

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
        let commands = vec![
            CreateCommand::new("roue")
                .description("Tire la Roue du Destin — 1 spin par jour, le destin decide."),
            CreateCommand::new("solde")
                .description("Affiche ton portefeuille (ou celui d'un autre membre)")
                .add_option(CreateCommandOption::new(
                    CommandOptionType::User,
                    "membre",
                    "Le membre dont voir le solde (defaut : toi)",
                )),
            CreateCommand::new("donner")
                .description("Donne des coins a un autre membre")
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::User,
                        "membre",
                        "Le membre a qui donner",
                    )
                    .required(true),
                )
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::Integer,
                        "montant",
                        "Le nombre de coins a donner",
                    )
                    .required(true)
                    .min_int_value(1),
                )
                .add_option(CreateCommandOption::new(
                    CommandOptionType::String,
                    "raison",
                    "Raison du don (optionnelle)",
                )),
            CreateCommand::new("classement")
                .description("Top 10 des plus riches du serveur"),
        ];
        for command in commands {
            if let Err(e) = Command::create_global_command(&ctx.http, command).await {
                tracing::error!("enregistrement d'une commande slash impossible: {e}");
            }
        }
        tracing::info!("commandes slash /roue /solde /donner /classement enregistrees (globales)");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(cmd) = interaction {
            match cmd.data.name.as_str() {
                "roue" => self.handle_roue(&ctx, &cmd).await,
                "solde" => self.handle_solde(&ctx, &cmd).await,
                "donner" => self.handle_donner(&ctx, &cmd).await,
                "classement" => self.handle_classement(&ctx, &cmd).await,
                _ => {}
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
