use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::{info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::discord_helpers::reply_ephemeral;
use sentinel_shared::embeds::success_embed;
use sentinel_shared::heartbeat::ApiClientKey;

use crate::handler::{CooldownKey, SponsorshipKey};

pub fn register() -> CreateCommand {
    CreateCommand::new("parrain")
        .description("Parrainer un nouveau membre du serveur")
        .default_member_permissions(serenity::all::Permissions::MANAGE_GUILD)
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "membre", "Membre a parrainer")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    // Check permission serveur (default_member_permissions est un hint UI).
    let has_manage_guild = command
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| {
            p.contains(serenity::all::Permissions::MANAGE_GUILD)
                || p.contains(serenity::all::Permissions::ADMINISTRATOR)
        })
        .unwrap_or(false);

    if !has_manage_guild {
        reply_ephemeral(ctx, command, "❌ Permission MANAGE_GUILD requise pour /parrain.").await;
        warn!(user = %command.user.name, "Tentative /parrain sans permission");
        return;
    }

    // Rate limit anti-spam : 30s par user. Sans ca, un admin (ou un compte
    // compromis avec les droits) pourrait spammer des parrainages.
    {
        let data = ctx.data.read().await;
        if let Some(cooldown) = data.get::<CooldownKey>() {
            if let Some(remaining) = cooldown.check_and_set(command.user.id.get(), "parrain", 30) {
                reply_ephemeral(
                    ctx, command,
                    &format!("⏱️ Cooldown actif. Attends {remaining}s avant de parrainer a nouveau."),
                ).await;
                return;
            }
        }
    }

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

    // H6 — Check LOCAL d'abord (limite de parrainages). Si le local passe,
    // on appelle l'API AVANT de modifier le tracker definitivement. Si l'API
    // echoue on abort sans avoir perturbe le tracker.
    // Note : tracker.sponsor() modifie le DashMap. Si l'API echoue apres, on
    // doit revert via tracker.remove_sponsor(). L'ordre est :
    //   1. Validation locale (limite)
    //   2. tracker.sponsor() (tentative)
    //   3. API create_sponsorship()
    //   4. Si API KO → tracker.remove_sponsor() (rollback)
    match tracker.sponsor(guild_id.get(), command.user.id.get(), target_id.get(), max_sponsorships) {
        Ok(()) => {
            // Persister via l'API, avec rollback si echec
            let api_result = if let Some(api) = data.get::<crate::handler::RolesApiKey>() {
                api.create_sponsorship(
                    &guild_id.to_string(),
                    &command.user.id.to_string(),
                    &target_id.to_string(),
                ).await
            } else {
                Ok(())
            };

            if let Err(e) = api_result {
                // Rollback du tracker en memoire
                tracker.remove_sponsor(guild_id.get(), command.user.id.get(), target_id.get());
                warn!(error = %e, "Echec API create_sponsorship — rollback tracker local");
                reply_ephemeral(ctx, command, "Echec d'enregistrement du parrainage cote serveur. Rien n'a ete applique.").await;
                return;
            }

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

