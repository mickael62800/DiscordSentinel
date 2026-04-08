use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::{info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::discord_helpers::reply_ephemeral;
use sentinel_shared::embeds::success_embed;
use sentinel_shared::heartbeat::ApiClientKey;

use crate::handler::SponsorshipKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("parrain")
        .description("Parrainer un nouveau membre du serveur")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "membre", "Membre a parrainer")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let target_id = match command.data.options.iter().find(|o| o.name == "membre")
        .and_then(|o| match &o.value { CommandDataOptionValue::User(id) => Some(*id), _ => None })
    {
        Some(id) => id,
        None => { reply_ephemeral(ctx, command, "Parametre membre requis.").await; return; }
    };

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => { reply_ephemeral(ctx, command, "Commande serveur uniquement.").await; return; }
    };

    // Lire la config
    let max_sponsorships = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let gc = match base.get_guild_config(&guild_id.to_string()).await {
                Ok(c) => c,
                Err(e) => {
                    warn!(error = %e, "Failed to fetch guild config for sponsorship");
                    std::collections::HashMap::new()
                }
            };
            BaseApiClient::config_u64(&gc, "max_sponsorships", 3) as u32
        } else {
            3
        }
    };

    let data = ctx.data.read().await;
    let tracker = match data.get::<SponsorshipKey>() {
        Some(t) => t,
        None => { reply_ephemeral(ctx, command, "Erreur interne.").await; return; }
    };

    match tracker.sponsor(guild_id.get(), command.user.id.get(), target_id.get(), max_sponsorships) {
        Ok(()) => {
            let embed = success_embed("Parrainage enregistre !")
                .description(format!(
                    "<@{}> est maintenant le parrain de <@{}>.\n\
                     Bienvenue dans la communaute !",
                    command.user.id, target_id
                ))
                .field("Parrain", format!("<@{}>", command.user.id), true)
                .field("Filleul", format!("<@{}>", target_id), true);

            if let Err(e) = command.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(embed),
                ),
            ).await {
                warn!(error = %e, "Failed to send sponsorship response");
            }

            // Persister le parrainage via l'API
            if let Some(api) = data.get::<crate::handler::RolesApiKey>() {
                api.create_sponsorship(
                    &guild_id.to_string(),
                    &command.user.id.to_string(),
                    &target_id.to_string(),
                ).await;
            }

            // Log + event temps reel
            if let Some(base) = data.get::<ApiClientKey>() {
                base.send_log(
                    "info",
                    &guild_id.to_string(),
                    &format!("{} a parraine {}", command.user.name, target_id),
                );
                base.publish_event("sponsorship_created", serde_json::json!({
                    "guild_id": guild_id.to_string(),
                    "sponsor_id": command.user.id.to_string(),
                    "sponsor_name": command.user.name,
                    "sponsored_id": target_id.to_string(),
                }));
            }

            info!(
                parrain = %command.user.name,
                filleul = %target_id,
                guild = %guild_id,
                "Parrainage enregistre"
            );
        }
        Err(msg) => {
            reply_ephemeral(ctx, command, msg).await;
        }
    }
}

