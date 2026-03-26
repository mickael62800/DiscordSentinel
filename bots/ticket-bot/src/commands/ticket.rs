use serenity::all::{
    CommandDataOption, CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use tracing::{error, info};

use crate::api_client::CreateTicketRequest;
use crate::handler::ApiClientKey;

/// Helper : extrait les sous-options d'une sous-commande.
fn get_sub_options(command: &CommandInteraction) -> &[CommandDataOption] {
    match &command.data.options[0].value {
        CommandDataOptionValue::SubCommand(opts) => opts,
        _ => &[],
    }
}

/// Helper : extrait une valeur string d'une option par nom.
fn get_str_option<'a>(options: &'a [CommandDataOption], name: &str) -> Option<&'a str> {
    options
        .iter()
        .find(|o| o.name == name)
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
}

/// Enregistre la commande /ticket avec ses sous-commandes.
pub fn register() -> CreateCommand {
    CreateCommand::new("ticket")
        .description("Gestion des tickets de support")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "create",
                "Créer un nouveau ticket",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "title",
                    "Titre du ticket",
                )
                .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "category",
                    "Catégorie du ticket",
                )
                .required(true)
                .add_string_choice("Signalement", "report")
                .add_string_choice("Appel de sanction", "appeal")
                .add_string_choice("Permissions", "permissions")
                .add_string_choice("Bug", "bug")
                .add_string_choice("Suggestion", "suggestion"),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "priority",
                    "Priorité du ticket",
                )
                .add_string_choice("Urgent", "urgent")
                .add_string_choice("Haute", "high")
                .add_string_choice("Moyenne", "medium")
                .add_string_choice("Basse", "low"),
            ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "close",
            "Fermer le ticket du salon actuel",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "assign",
                "Assigner le ticket à un modérateur",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "moderator",
                    "Modérateur à assigner",
                )
                .required(true),
            ),
        )
}

/// Dispatch la slash command vers la bonne sous-commande.
pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let sub = &command.data.options[0];
    let result = match sub.name.as_str() {
        "create" => handle_create(ctx, command).await,
        "close" => handle_close(ctx, command).await,
        "assign" => handle_assign(ctx, command).await,
        _ => reply(ctx, command, "Sous-commande inconnue.").await,
    };

    if let Err(e) = result {
        error!(error = %e, "Erreur commande ticket");
    }
}

async fn handle_create(
    ctx: &Context,
    command: &CommandInteraction,
) -> Result<(), serenity::Error> {
    let options = get_sub_options(command);

    let title = get_str_option(options, "title").unwrap_or("Sans titre");
    let category = get_str_option(options, "category").unwrap_or("report");
    let priority = get_str_option(options, "priority").unwrap_or("medium");

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return reply(ctx, command, "Cette commande ne fonctionne que dans un serveur.").await,
    };

    let guild_name = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map(|g| g.name)
        .unwrap_or_else(|_| guild_id.to_string());

    let author = &command.user;

    // Envoyer au backend
    let data = ctx.data.read().await;
    let api = data.get::<ApiClientKey>().unwrap();

    let request = CreateTicketRequest {
        title: title.to_string(),
        priority: priority.to_string(),
        author_id: author.id.to_string(),
        author_name: author.name.clone(),
        server: guild_name.clone(),
        category: category.to_string(),
    };

    let ticket = match api.create_ticket(&request).await {
        Ok(t) => t,
        Err(e) => {
            error!(error = %e, "Impossible de créer le ticket via l'API");
            return reply(ctx, command, &format!("Erreur lors de la création du ticket : {e}")).await;
        }
    };

    // Créer un thread privé pour le ticket
    let thread = command
        .channel_id
        .create_thread(
            &ctx.http,
            serenity::builder::CreateThread::new(format!("ticket-{}", &ticket.id[..8]))
                .kind(serenity::model::channel::ChannelType::PrivateThread),
        )
        .await?;

    // Message d'ouverture dans le thread
    thread
        .send_message(
            &ctx.http,
            serenity::builder::CreateMessage::new().content(format!(
                "**Ticket #{}**\n\
                 **Titre** : {}\n\
                 **Catégorie** : {}\n\
                 **Priorité** : {}\n\
                 **Auteur** : <@{}>\n\n\
                 Décrivez votre problème ici. Un modérateur vous répondra.",
                &ticket.id[..8],
                ticket.title,
                ticket.category,
                ticket.priority,
                author.id,
            )),
        )
        .await?;

    info!(
        ticket_id = %ticket.id,
        author = %author.name,
        guild = %guild_name,
        "Ticket créé"
    );

    reply(
        ctx,
        command,
        &format!("Ticket créé ! Rendez-vous dans <#{}>.", thread.id),
    )
    .await
}

async fn handle_close(
    ctx: &Context,
    command: &CommandInteraction,
) -> Result<(), serenity::Error> {
    let channel_name = command
        .channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild())
        .map(|c| c.name.clone())
        .unwrap_or_default();

    if !channel_name.starts_with("ticket-") {
        return reply(ctx, command, "Cette commande ne fonctionne que dans un salon de ticket.").await;
    }

    let ticket_id = channel_name.trim_start_matches("ticket-");

    let data = ctx.data.read().await;
    let api = data.get::<ApiClientKey>().unwrap();

    if let Err(e) = api.close_ticket(ticket_id).await {
        error!(error = %e, "Impossible de fermer le ticket");
        return reply(ctx, command, &format!("Erreur : {e}")).await;
    }

    // Archiver le thread
    command
        .channel_id
        .edit_thread(
            &ctx.http,
            serenity::builder::EditThread::new()
                .archived(true)
                .locked(true),
        )
        .await
        .ok();

    info!(ticket_id = %ticket_id, "Ticket fermé");

    reply(ctx, command, "Ticket fermé et archivé.").await
}

async fn handle_assign(
    ctx: &Context,
    command: &CommandInteraction,
) -> Result<(), serenity::Error> {
    let channel_name = command
        .channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild())
        .map(|c| c.name.clone())
        .unwrap_or_default();

    if !channel_name.starts_with("ticket-") {
        return reply(ctx, command, "Cette commande ne fonctionne que dans un salon de ticket.").await;
    }

    let ticket_id = channel_name.trim_start_matches("ticket-");

    let options = get_sub_options(command);

    let moderator_id = options
        .iter()
        .find(|o| o.name == "moderator")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        })
        .unwrap();

    let moderator_name = moderator_id
        .to_user(&ctx.http)
        .await
        .map(|u| u.name)
        .unwrap_or_else(|_| moderator_id.to_string());

    let data = ctx.data.read().await;
    let api = data.get::<ApiClientKey>().unwrap();

    if let Err(e) = api.assign_ticket(ticket_id, &moderator_name).await {
        error!(error = %e, "Impossible d'assigner le ticket");
        return reply(ctx, command, &format!("Erreur : {e}")).await;
    }

    // Ajouter le modérateur au thread
    command
        .channel_id
        .add_thread_member(&ctx.http, moderator_id)
        .await
        .ok();

    info!(ticket_id = %ticket_id, moderator = %moderator_name, "Ticket assigné");

    reply(
        ctx,
        command,
        &format!("Ticket assigné à <@{moderator_id}>."),
    )
    .await
}

async fn reply(
    ctx: &Context,
    command: &CommandInteraction,
    content: &str,
) -> Result<(), serenity::Error> {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
}
