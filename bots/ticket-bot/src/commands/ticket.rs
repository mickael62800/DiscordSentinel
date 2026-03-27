use serenity::all::{
    CommandDataOption, CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, ChannelId, GuildId, UserId,
    CreateActionRow, CreateButton, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
    ComponentInteraction, PermissionOverwrite, PermissionOverwriteType,
};
use serenity::builder::{CreateChannel, CreateMessage, EditChannel};
use serenity::model::channel::ChannelType;
use serenity::model::Permissions;
use tracing::{error, info};

use crate::api_client::CreateTicketRequest;
use crate::handler::ApiClientKey;

// ── Constantes pour les custom_id des boutons/menus ──
pub const PANEL_BUTTON_ID: &str = "sentinel_ticket_create";
pub const TYPE_SELECT_ID: &str = "sentinel_ticket_type";
pub const CLOSE_BUTTON_ID: &str = "sentinel_ticket_close";
pub const INVITE_BUTTON_ID: &str = "sentinel_ticket_invite";
pub const VOCAL_BUTTON_ID: &str = "sentinel_ticket_vocal";

/// Types de tickets disponibles
const TICKET_TYPES: &[(&str, &str, &str)] = &[
    ("probleme_serveur", "Probleme serveur", "Un souci technique ou de configuration du serveur"),
    ("probleme_membre", "Probleme avec un membre", "Signaler le comportement d'un membre"),
    ("appel_sanction", "Appel de sanction", "Contester une sanction recue"),
    ("suggestion", "Suggestion", "Proposer une amelioration pour le serveur"),
    ("question", "Question", "Poser une question au staff"),
    ("autre", "Autre", "Demande qui ne rentre pas dans les autres categories"),
];

/// Enregistre la commande /ticket avec ses sous-commandes.
pub fn register() -> CreateCommand {
    CreateCommand::new("ticket")
        .description("Gestion des tickets de support")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "panel",
                "Deployer le panneau de creation de ticket dans ce salon",
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
                "invite",
                "Inviter un membre dans ce ticket",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "membre",
                    "Membre a inviter",
                )
                .required(true),
            ),
        )
}

/// Dispatch la slash command vers la bonne sous-commande.
pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let sub = &command.data.options[0];
    let result = match sub.name.as_str() {
        "panel" => handle_panel(ctx, command).await,
        "close" => handle_close(ctx, command).await,
        "invite" => handle_invite(ctx, command).await,
        _ => reply(ctx, command, "Sous-commande inconnue.").await,
    };

    if let Err(e) = result {
        error!(error = %e, "Erreur commande ticket");
    }
}

/// /ticket panel — Envoie le message permanent avec le bouton "Creer un ticket"
async fn handle_panel(
    ctx: &Context,
    command: &CommandInteraction,
) -> Result<(), serenity::Error> {
    let button = CreateButton::new(PANEL_BUTTON_ID)
        .label("Creer un ticket")
        .style(serenity::all::ButtonStyle::Success);

    let row = CreateActionRow::Buttons(vec![button]);

    let message = CreateMessage::new()
        .content(
            "**Assistance & Support**\n\n\
             Besoin d'aide ? Cliquez sur le bouton ci-dessous pour ouvrir un ticket.\n\
             Un salon prive sera cree pour vous permettre d'echanger avec le staff.\n\n\
             Choisissez ensuite le type de demande dans le menu qui apparaitra.",
        )
        .components(vec![row]);

    command.channel_id.send_message(&ctx.http, message).await?;

    reply(ctx, command, "Panneau de tickets deploye !").await
}

/// Gere le clic sur le bouton "Creer un ticket" du panel
pub async fn handle_panel_click(ctx: &Context, component: &ComponentInteraction) {
    // Envoyer le menu de selection du type de ticket
    let options: Vec<CreateSelectMenuOption> = TICKET_TYPES
        .iter()
        .map(|(value, label, desc)| {
            CreateSelectMenuOption::new(*label, *value).description(*desc)
        })
        .collect();

    let select = CreateSelectMenu::new(
        TYPE_SELECT_ID,
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Choisissez le type de ticket...");

    let row = CreateActionRow::SelectMenu(select);

    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("**Quel type de ticket souhaitez-vous ouvrir ?**")
            .components(vec![row])
            .ephemeral(true),
    );

    if let Err(e) = component.create_response(&ctx.http, response).await {
        error!(error = %e, "Erreur envoi menu type ticket");
    }
}

/// Gere la selection du type de ticket → cree le salon prive
pub async fn handle_type_select(ctx: &Context, component: &ComponentInteraction) {
    let ticket_type = match &component.data.kind {
        serenity::all::ComponentInteractionDataKind::StringSelect { values } => {
            match values.first() {
                Some(v) => v.clone(),
                None => return,
            }
        }
        _ => return,
    };

    let guild_id = match component.guild_id {
        Some(id) => id,
        None => return,
    };

    let author = &component.user;
    let type_label = TICKET_TYPES
        .iter()
        .find(|(v, _, _)| *v == ticket_type)
        .map(|(_, l, _)| *l)
        .unwrap_or("Ticket");

    // Repondre immediatement (ephemeral)
    let _ = component.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Creation du ticket en cours...")
                .ephemeral(true),
        ),
    ).await;

    // Creer le salon textuel prive
    let channel_name = format!("ticket-{}-{}", &author.name.chars().take(10).collect::<String>(), &author.id.get().to_string()[..4]);

    // Permissions: deny @everyone, allow author + bot
    let everyone_role = guild_id.everyone_role();
    let overwrites = vec![
        PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
            kind: PermissionOverwriteType::Role(everyone_role),
        },
        PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY | Permissions::ATTACH_FILES,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(author.id),
        },
        PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY | Permissions::MANAGE_CHANNELS,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(ctx.cache.current_user().id),
        },
    ];

    // Ajouter les permissions pour le role staff si configure
    let data = ctx.data.read().await;
    let api = data.get::<ApiClientKey>().unwrap();
    let guild_config = api.get_guild_config(&guild_id.to_string()).await.unwrap_or_default();

    let mut all_overwrites = overwrites;

    // Admin role
    if let Some(admin_role_str) = guild_config.get("admin_role_id") {
        if let Ok(role_id) = admin_role_str.parse::<u64>() {
            all_overwrites.push(PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY | Permissions::MANAGE_CHANNELS,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Role(serenity::model::id::RoleId::new(role_id)),
            });
        }
    }

    // Moderator role
    if let Some(mod_role_str) = guild_config.get("moderator_role_id") {
        if let Ok(role_id) = mod_role_str.parse::<u64>() {
            all_overwrites.push(PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Role(serenity::model::id::RoleId::new(role_id)),
            });
        }
    }

    // Category ID si configure
    let category_id = guild_config
        .get("ticket_category_id")
        .and_then(|v| v.parse::<u64>().ok())
        .or_else(|| std::env::var("TICKET_CATEGORY_ID").ok().and_then(|v| v.parse().ok()));

    let mut create_channel = CreateChannel::new(&channel_name)
        .kind(ChannelType::Text)
        .topic(format!("Ticket {} — {}", type_label, author.name))
        .permissions(all_overwrites);

    if let Some(cat_id) = category_id {
        create_channel = create_channel.category(ChannelId::new(cat_id));
    }

    let channel = match guild_id.create_channel(&ctx.http, create_channel).await {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Impossible de creer le salon ticket");
            return;
        }
    };

    // Envoyer au backend
    let guild_name = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map(|g| g.name)
        .unwrap_or_else(|_| guild_id.to_string());

    let request = CreateTicketRequest {
        title: format!("{} — {}", type_label, author.name),
        priority: "medium".to_string(),
        author_id: author.id.to_string(),
        author_name: author.name.clone(),
        server: guild_name,
        category: ticket_type.clone(),
        ticket_type: ticket_type.clone(),
        channel_id: Some(channel.id.to_string()),
    };

    let ticket_id = match api.create_ticket(&request).await {
        Ok(t) => t.id.clone(),
        Err(e) => {
            error!(error = %e, "Erreur creation ticket API");
            "???".to_string()
        }
    };

    // Boutons d'action dans le salon ticket
    let close_btn = CreateButton::new(CLOSE_BUTTON_ID)
        .label("Fermer le ticket")
        .style(serenity::all::ButtonStyle::Danger);
    let invite_btn = CreateButton::new(INVITE_BUTTON_ID)
        .label("Inviter quelqu'un")
        .style(serenity::all::ButtonStyle::Secondary);
    let vocal_btn = CreateButton::new(VOCAL_BUTTON_ID)
        .label("Passer en vocal")
        .style(serenity::all::ButtonStyle::Primary);

    let row = CreateActionRow::Buttons(vec![close_btn, invite_btn, vocal_btn]);

    let welcome = CreateMessage::new()
        .content(format!(
            "**Ticket #{ticket_short}** — {type_label}\n\n\
             Bienvenue <@{author_id}> !\n\
             Decrivez votre probleme ici. Un membre du staff vous repondra.\n\n\
             Utilisez les boutons ci-dessous pour gerer votre ticket.",
            ticket_short = &ticket_id[..8.min(ticket_id.len())],
            author_id = author.id,
        ))
        .components(vec![row]);

    channel.send_message(&ctx.http, welcome).await.ok();

    info!(
        ticket_id = %ticket_id,
        author = %author.name,
        channel = %channel.name,
        ticket_type = %ticket_type,
        "Ticket cree (salon isole)"
    );
}

/// Bouton "Fermer le ticket" — supprime le salon
pub async fn handle_close_button(ctx: &Context, component: &ComponentInteraction) {
    let channel_id = component.channel_id;
    let channel_name = channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild())
        .map(|c| c.name.clone())
        .unwrap_or_default();

    // Repondre
    let _ = component.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Ticket ferme. Ce salon sera supprime dans 5 secondes.")
                .ephemeral(true),
        ),
    ).await;

    // Fermer via API (trouver le ticket par channel name)
    let data = ctx.data.read().await;
    if let Some(api) = data.get::<ApiClientKey>() {
        // On cherche le ticket par son ID court dans le nom du channel
        if let Some(short_id) = channel_name.strip_prefix("ticket-") {
            // L'API fermera le ticket si on trouve l'ID
            let _ = api.close_ticket(short_id).await;
        }

        api.send_log(
            "info",
            &component.guild_id.map(|g| g.to_string()).unwrap_or_default(),
            &format!("Ticket ferme : {} par {}", channel_name, component.user.name),
        );
    }

    info!(channel = %channel_name, user = %component.user.name, "Ticket ferme");

    // Supprimer le salon apres un delai
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    let _ = channel_id.delete(&ctx.http).await;
}

/// Bouton "Inviter quelqu'un" — demande de mentionner un utilisateur
pub async fn handle_invite_button(ctx: &Context, component: &ComponentInteraction) {
    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("Mentionnez l'utilisateur a inviter dans ce salon (ex: `@pseudo`).\nLe prochain message mentionnant un utilisateur sera traite.")
            .ephemeral(true),
    );
    let _ = component.create_response(&ctx.http, response).await;
}

/// Bouton "Passer en vocal" — cree un salon vocal lie au ticket
pub async fn handle_vocal_button(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(id) => id,
        None => return,
    };

    let channel_name = component.channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild())
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "ticket".to_string());

    let vocal_name = format!("vocal-{}", channel_name);

    // Copier les permissions du salon texte
    let text_channel = match component.channel_id.to_channel(&ctx.http).await {
        Ok(c) => c,
        Err(_) => return,
    };

    let guild_channel = text_channel.guild();
    let overwrites = guild_channel
        .as_ref()
        .map(|c| c.permission_overwrites.clone())
        .unwrap_or_default();

    // Category
    let category_id = guild_channel
        .as_ref()
        .and_then(|c| c.parent_id);

    let mut create = CreateChannel::new(&vocal_name)
        .kind(ChannelType::Voice)
        .permissions(overwrites);

    if let Some(cat_id) = category_id {
        create = create.category(cat_id);
    }

    match guild_id.create_channel(&ctx.http, create).await {
        Ok(vc) => {
            let _ = component.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!(
                            "Salon vocal cree : <#{}>. Rejoignez-le pour discuter avec le staff !",
                            vc.id
                        ))
                ),
            ).await;

            info!(vocal = %vc.name, ticket = %channel_name, "Salon vocal cree pour ticket");
        }
        Err(e) => {
            error!(error = %e, "Impossible de creer le salon vocal");
            let _ = component.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Impossible de creer le salon vocal.")
                        .ephemeral(true),
                ),
            ).await;
        }
    }
}

/// Commande /ticket close
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

    reply(ctx, command, "Fermeture du ticket...").await?;

    let data = ctx.data.read().await;
    if let Some(api) = data.get::<ApiClientKey>() {
        api.send_log(
            "info",
            &command.guild_id.map(|g| g.to_string()).unwrap_or_default(),
            &format!("Ticket ferme : {} par {}", channel_name, command.user.name),
        );
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    let _ = command.channel_id.delete(&ctx.http).await;

    Ok(())
}

/// Commande /ticket invite <membre>
async fn handle_invite(
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

    let options = get_sub_options(command);
    let user_id = options
        .iter()
        .find(|o| o.name == "membre")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        });

    let user_id = match user_id {
        Some(id) => id,
        None => return reply(ctx, command, "Veuillez specifier un membre.").await,
    };

    // Ajouter la permission VIEW_CHANNEL pour cet utilisateur
    let overwrite = PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(user_id),
    };

    command.channel_id.create_permission(&ctx.http, overwrite).await?;

    reply(
        ctx,
        command,
        &format!("<@{user_id}> a ete invite dans ce ticket."),
    )
    .await
}

// ── Helpers ──

fn get_sub_options(command: &CommandInteraction) -> &[CommandDataOption] {
    match &command.data.options[0].value {
        CommandDataOptionValue::SubCommand(opts) => opts,
        _ => &[],
    }
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
