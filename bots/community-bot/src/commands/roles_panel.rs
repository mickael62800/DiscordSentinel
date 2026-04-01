use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::{error, info};

use sentinel_shared::embeds::{info_embed, success_embed};

use crate::handler::{send_role_panel, RolesApiKey};

pub fn register() -> CreateCommand {
    CreateCommand::new("roles-panel")
        .description("Gerer les panels de roles")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "deploy",
                "Deployer un panel de roles dans ce salon",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "panel_id",
                    "ID du panel (depuis le dashboard desktop)",
                )
                .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "list",
                "Lister les panels de roles du serveur",
            ),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            respond(ctx, command, "Cette commande doit etre utilisee dans un serveur.").await;
            return;
        }
    };

    let sub = &command.data.options[0];

    match sub.name.as_str() {
        "deploy" => handle_deploy(ctx, command, &guild_id.to_string()).await,
        "list" => handle_list(ctx, command, &guild_id.to_string()).await,
        _ => {}
    }
}

async fn handle_deploy(ctx: &Context, command: &CommandInteraction, _guild_id: &str) {
    let sub_options = match &command.data.options[0].value {
        CommandDataOptionValue::SubCommand(opts) => opts,
        _ => return,
    };

    let panel_id = match sub_options.iter().find(|o| o.name == "panel_id") {
        Some(o) => match &o.value {
            CommandDataOptionValue::String(s) => s.clone(),
            _ => return,
        },
        None => return,
    };

    let data = ctx.data.read().await;
    let api = match data.get::<RolesApiKey>() {
        Some(a) => a,
        None => return,
    };

    let panel = match api.get_panel(&panel_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            respond(ctx, command, "Panel introuvable. Verifiez l'ID.").await;
            return;
        }
        Err(e) => {
            error!(error = %e, "Erreur API get_panel");
            respond(ctx, command, "Erreur lors de la recuperation du panel.").await;
            return;
        }
    };

    // Envoyer le panel dans le channel
    match send_role_panel(ctx, command.channel_id, &panel).await {
        Ok(msg) => {
            // Sauvegarder le message_id dans l'API
            let _ = api.set_message_id(&panel_id, &msg.id.to_string()).await;
            info!(panel_id = %panel_id, message_id = %msg.id, "Panel de roles deploye");
            respond_embed(
                ctx,
                command,
                success_embed("\u{2705} Panel deploye")
                    .description(format!("Le panel **{}** a ete deploye dans ce salon.", panel.panel.title)),
            )
            .await;
        }
        Err(e) => {
            error!(error = %e, "Erreur envoi panel");
            respond(ctx, command, "Erreur lors de l'envoi du panel.").await;
        }
    }
}

async fn handle_list(ctx: &Context, command: &CommandInteraction, guild_id: &str) {
    let data = ctx.data.read().await;
    let api = match data.get::<RolesApiKey>() {
        Some(a) => a,
        None => return,
    };

    match api.list_panels(guild_id).await {
        Ok(panels) => {
            if panels.is_empty() {
                respond(ctx, command, "Aucun panel de roles configure. Creez-en un depuis le dashboard desktop.").await;
                return;
            }

            let mut desc = String::new();
            for p in &panels {
                let status = if p.message_id.is_some() { "deploye" } else { "non deploye" };
                desc.push_str(&format!("- **{}** (`{}`) — {}\n", p.title, p.id, status));
            }

            respond_embed(
                ctx,
                command,
                info_embed("\u{1f4cb} Panels de roles").description(desc),
            )
            .await;
        }
        Err(e) => {
            error!(error = %e, "Erreur API list_panels");
            respond(ctx, command, "Erreur lors de la recuperation des panels.").await;
        }
    }
}

async fn respond(ctx: &Context, command: &CommandInteraction, content: &str) {
    let msg = CreateInteractionResponseMessage::new()
        .content(content)
        .ephemeral(true);
    let response = CreateInteractionResponse::Message(msg);
    let _ = command.create_response(&ctx.http, response).await;
}

async fn respond_embed(ctx: &Context, command: &CommandInteraction, embed: CreateEmbed) {
    let msg = CreateInteractionResponseMessage::new()
        .embed(embed)
        .ephemeral(true);
    let response = CreateInteractionResponse::Message(msg);
    let _ = command.create_response(&ctx.http, response).await;
}
