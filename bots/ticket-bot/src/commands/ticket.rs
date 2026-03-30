use serenity::all::{
    CommandDataOption, CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, ChannelId,
    CreateActionRow, CreateButton, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
    ComponentInteraction, PermissionOverwrite, PermissionOverwriteType, EditChannel,
    CreateModal, CreateInputText, InputTextStyle, ModalInteraction,
};
use serenity::builder::{CreateChannel, CreateMessage};
use serenity::model::channel::ChannelType;
use serenity::model::Permissions;
use tracing::{error, info, warn};

use sentinel_shared::heartbeat::ApiClientKey;

use crate::api_client::{ApiClient, CreateTicketRequest};

// ── Constantes pour les custom_id des boutons/menus ──
pub const PANEL_BUTTON_ID: &str = "sentinel_ticket_create";
pub const TYPE_SELECT_ID: &str = "sentinel_ticket_type";
pub const MODAL_ID_PREFIX: &str = "sentinel_ticket_modal:";
pub const CLOSE_BUTTON_ID: &str = "sentinel_ticket_close";
pub const INVITE_BUTTON_ID: &str = "sentinel_ticket_invite";
pub const INVITE_SELECT_ID: &str = "sentinel_ticket_invite_select";
pub const VOCAL_BUTTON_ID: &str = "sentinel_ticket_vocal";
pub const VOCAL_USER_ACCEPT_ID: &str = "sentinel_ticket_vocal_user_accept";
pub const VOCAL_USER_DECLINE_ID: &str = "sentinel_ticket_vocal_user_decline";
pub const CLOSE_CONFIRM_ID: &str = "sentinel_ticket_close_confirm";
pub const CLOSE_CANCEL_ID: &str = "sentinel_ticket_close_cancel";

/// Types de tickets disponibles
/// Types de tickets qui restreignent la visibilite aux admins uniquement (pas les modos)
const ADMIN_ONLY_TYPES: &[&str] = &["probleme_moderateur"];

/// Types de tickets a priorite urgente automatique
const URGENT_TYPES: &[&str] = &["urgence_detresse"];

const TICKET_TYPES: &[(&str, &str, &str)] = &[
    ("probleme_serveur", "Probleme serveur", "Un souci technique ou de configuration du serveur"),
    ("probleme_membre", "Probleme avec un membre", "Signaler le comportement d'un membre"),
    ("probleme_moderateur", "Probleme avec un moderateur", "Signaler un abus ou probleme avec un moderateur (confidentiel, admins uniquement)"),
    ("appel_sanction", "Appel de sanction", "Contester une sanction recue"),
    ("urgence_detresse", "Situation urgente / detresse", "Vous traversez une situation grave et avez besoin d'aide rapidement"),
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

/// Construit le message du panel de creation de ticket (reutilisable)
pub fn build_panel_message() -> CreateMessage {
    let button = CreateButton::new(PANEL_BUTTON_ID)
        .label("Creer un ticket")
        .style(serenity::all::ButtonStyle::Success);

    let row = CreateActionRow::Buttons(vec![button]);

    CreateMessage::new()
        .content(
            "**Assistance & Support**\n\n\
             Besoin d'aide ? Cliquez sur le bouton ci-dessous pour ouvrir un ticket.\n\
             Un salon prive sera cree pour vous permettre d'echanger avec le staff.\n\n\
             **Types de demandes disponibles :**\n\
             > **Probleme serveur** — Un souci technique ou de configuration\n\
             > **Probleme avec un membre** — Signaler un comportement inapproprie\n\
             > **Probleme avec un moderateur** — Confidentiel, visible uniquement par les admins\n\
             > **Appel de sanction** — Contester une sanction recue\n\
             > **Situation urgente / detresse** — Besoin d'aide rapide dans une situation grave\n\
             > **Question** — Poser une question au staff\n\
             > **Autre** — Toute autre demande\n\n\
             Choisissez le type de demande, puis decrivez votre situation dans le formulaire.",
        )
        .components(vec![row])
}

/// /ticket panel — Envoie le message permanent avec le bouton "Creer un ticket"
async fn handle_panel(
    ctx: &Context,
    command: &CommandInteraction,
) -> Result<(), serenity::Error> {
    command.channel_id.send_message(&ctx.http, build_panel_message()).await?;
    reply(ctx, command, "Panneau de tickets deploye !").await
}

/// Gere le clic sur le bouton "Creer un ticket" du panel
pub async fn handle_panel_click(ctx: &Context, component: &ComponentInteraction) {
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

/// Gere la selection du type de ticket -> ouvre un modal pour decrire le probleme
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

    let type_label = TICKET_TYPES
        .iter()
        .find(|(v, _, _)| *v == ticket_type)
        .map(|(_, l, _)| *l)
        .unwrap_or("Ticket");

    // Ouvrir un modal pour que l'utilisateur decrive son probleme
    let modal_id = format!("{}{}", MODAL_ID_PREFIX, ticket_type);

    let subject_input = CreateInputText::new(InputTextStyle::Short, "Sujet", "ticket_subject")
        .placeholder("Resumez votre demande en quelques mots...")
        .required(true)
        .min_length(5)
        .max_length(100);

    let description_input = CreateInputText::new(
        InputTextStyle::Paragraph,
        "Description",
        "ticket_description",
    )
    .placeholder("Decrivez votre probleme en detail : que s'est-il passe, quand, qui est concerne...")
    .required(true)
    .min_length(10)
    .max_length(2000);

    let modal = CreateModal::new(&modal_id, format!("Nouveau ticket — {}", type_label))
        .components(vec![
            CreateActionRow::InputText(subject_input),
            CreateActionRow::InputText(description_input),
        ]);

    if let Err(e) = component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await {
        error!(error = %e, "Erreur ouverture modal ticket");
    }

    // Supprimer le message ephemeral du dropdown (celui de handle_panel_click)
    // Le message du select menu est un follow-up du bouton, on le supprime via le message source
    if let Err(e) = component.delete_response(&ctx.http).await {
        // Pas grave si ca echoue, c'est du nettoyage
        tracing::debug!(error = %e, "Impossible de supprimer le message du dropdown");
    }
}

/// Gere la soumission du modal -> cree le salon prive et le ticket
pub async fn handle_modal_submit(ctx: &Context, modal: &ModalInteraction) {
    // Extraire le type de ticket depuis le custom_id du modal
    let ticket_type = match modal.data.custom_id.strip_prefix(MODAL_ID_PREFIX) {
        Some(t) => t.to_string(),
        None => return,
    };

    let guild_id = match modal.guild_id {
        Some(id) => id,
        None => return,
    };

    let author = &modal.user;
    let type_label = TICKET_TYPES
        .iter()
        .find(|(v, _, _)| *v == ticket_type)
        .map(|(_, l, _)| *l)
        .unwrap_or("Ticket");

    // Extraire les champs du modal
    let mut subject = String::new();
    let mut description = String::new();

    for row in &modal.data.components {
        for component in &row.components {
            if let serenity::all::ActionRowComponent::InputText(input) = component {
                match input.custom_id.as_str() {
                    "ticket_subject" => subject = input.value.clone().unwrap_or_default(),
                    "ticket_description" => description = input.value.clone().unwrap_or_default(),
                    _ => {}
                }
            }
        }
    }

    // Defer la reponse (ephemeral)
    let _ = modal.create_response(
        &ctx.http,
        CreateInteractionResponse::Defer(
            CreateInteractionResponseMessage::new()
                .ephemeral(true),
        ),
    ).await;

    // Rate limiting : verifier max_open_per_user
    {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let guild_config = base.get_guild_config(&guild_id.to_string()).await.unwrap_or_default();
            let max_open: u64 = sentinel_shared::api_client::BaseApiClient::config_u64(&guild_config, "max_open_per_user", 0);

            if max_open > 0 {
                let api = ApiClient::new(base.clone());
                if let Ok(tickets) = api.list_tickets().await {
                    let open_count = tickets.iter().filter(|t| {
                        t.author_id == author.id.to_string() && t.status != "closed"
                    }).count() as u64;

                    if open_count >= max_open {
                        let _ = modal.edit_response(
                            &ctx.http,
                            serenity::builder::EditInteractionResponse::new()
                                .content(format!(
                                    "Vous avez deja {} ticket(s) ouvert(s). Limite : {} par utilisateur.",
                                    open_count, max_open
                                ))
                        ).await;
                        return;
                    }
                }
            }
        }
    }

    // Creer le salon textuel prive
    let channel_name = format!(
        "ticket-{}-{}",
        &author.name.chars().take(10).collect::<String>(),
        &author.id.get().to_string()[..4]
    );

    // Anti-doublon : verifier qu'un salon avec ce nom n'existe pas deja
    if let Ok(channels) = guild_id.channels(&ctx.http).await {
        let exists = channels.values().any(|c| c.name == channel_name);
        if exists {
            let _ = modal.delete_response(&ctx.http).await;
            return;
        }
    }

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
    let base = match data.get::<ApiClientKey>() {
        Some(b) => b,
        None => {
            error!("ApiClientKey introuvable dans le context");
            return;
        }
    };
    let api = ApiClient::new(base.clone());
    let guild_config = base.get_guild_config(&guild_id.to_string()).await.unwrap_or_default();

    let mut all_overwrites = overwrites;
    let is_admin_only = ADMIN_ONLY_TYPES.contains(&ticket_type.as_str());

    // Admin role — toujours ajoute
    if let Some(admin_role_str) = guild_config.get("admin_role_id") {
        if let Ok(role_id) = admin_role_str.parse::<u64>() {
            all_overwrites.push(PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY | Permissions::MANAGE_CHANNELS,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Role(serenity::model::id::RoleId::new(role_id)),
            });
        }
    }

    // Moderator role — seulement si le ticket n'est PAS reserve aux admins
    if !is_admin_only {
        if let Some(mod_role_str) = guild_config.get("moderator_role_id") {
            if let Ok(role_id) = mod_role_str.parse::<u64>() {
                all_overwrites.push(PermissionOverwrite {
                    allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
                    deny: Permissions::empty(),
                    kind: PermissionOverwriteType::Role(serenity::model::id::RoleId::new(role_id)),
                });
            }
        }
    } else {
        // Pour les tickets admin-only, bloquer explicitement le role moderateur
        if let Some(mod_role_str) = guild_config.get("moderator_role_id") {
            if let Ok(role_id) = mod_role_str.parse::<u64>() {
                all_overwrites.push(PermissionOverwrite {
                    allow: Permissions::empty(),
                    deny: Permissions::VIEW_CHANNEL,
                    kind: PermissionOverwriteType::Role(serenity::model::id::RoleId::new(role_id)),
                });
            }
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
        .permissions(all_overwrites.clone());

    if let Some(cat_id) = category_id {
        create_channel = create_channel.category(ChannelId::new(cat_id));
    }

    let mut channel = match guild_id.create_channel(&ctx.http, create_channel).await {
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

    let title = if subject.is_empty() {
        format!("{} — {}", type_label, author.name)
    } else {
        subject.clone()
    };

    let priority = if URGENT_TYPES.contains(&ticket_type.as_str()) {
        "urgent"
    } else {
        "medium"
    };

    let request = CreateTicketRequest {
        title: title.clone(),
        priority: priority.to_string(),
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

    // Mettre a jour le topic du salon avec l'UUID du ticket
    if ticket_id != "???" {
        let new_topic = format!("[ticket:{}] {} — {}", ticket_id, type_label, author.name);
        if let Err(e) = channel.edit(&ctx.http, EditChannel::new().topic(&new_topic)).await {
            warn!(error = %e, "Impossible de mettre a jour le topic du salon ticket");
        }
    }

    // Message de bienvenue adapte selon le type de ticket
    let staff_line = if is_admin_only {
        "Ce ticket est **confidentiel**. Seuls les administrateurs peuvent le voir.\nUn administrateur vous repondra sous peu."
    } else if URGENT_TYPES.contains(&ticket_type.as_str()) {
        "**PRIORITE URGENTE** — Un membre du staff va vous repondre le plus rapidement possible.\nVous n'etes pas seul(e), nous sommes la pour vous aider."
    } else {
        "Un membre du staff vous repondra sous peu."
    };

    let welcome_content = format!(
        "**Ticket #{ticket_short}** — {type_label}\n\
         ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n\
         **Auteur :** <@{author_id}>\n\
         **Type :** {type_label}\n\
         **Priorite :** {priority}\n\
         **Sujet :** {subject}\n\n\
         **Description :**\n\
         > {description}\n\n\
         ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
         {staff_line}",
        ticket_short = &ticket_id[..8.min(ticket_id.len())],
        author_id = author.id,
        subject = title,
        description = description.replace('\n', "\n> "),
    );

    // Message de bienvenue (sans boutons)
    let welcome = CreateMessage::new().content(welcome_content);
    if let Err(e) = channel.send_message(&ctx.http, welcome).await {
        error!(error = %e, channel = %channel.id, "Erreur envoi message de bienvenue");
    }

    // Boutons utilisateur
    let user_close_btn = CreateButton::new(CLOSE_BUTTON_ID)
        .label("Fermer le ticket")
        .style(serenity::all::ButtonStyle::Danger);
    let invite_btn = CreateButton::new(INVITE_BUTTON_ID)
        .label("Inviter quelqu'un")
        .style(serenity::all::ButtonStyle::Secondary);
    let user_row = CreateActionRow::Buttons(vec![user_close_btn, invite_btn]);

    let user_msg = CreateMessage::new()
        .content("**Commandes utilisateur :**")
        .components(vec![user_row]);
    if let Err(e) = channel.send_message(&ctx.http, user_msg).await {
        error!(error = %e, channel = %channel.id, "Erreur envoi commandes utilisateur");
    }

    // Boutons staff (admin / moderateur)
    let staff_close_btn = CreateButton::new(CLOSE_BUTTON_ID)
        .label("Fermer le ticket")
        .style(serenity::all::ButtonStyle::Danger);
    let vocal_btn = CreateButton::new(VOCAL_BUTTON_ID)
        .label("Proposer un vocal")
        .style(serenity::all::ButtonStyle::Primary);
    let staff_row = CreateActionRow::Buttons(vec![staff_close_btn, vocal_btn]);

    let staff_msg = CreateMessage::new()
        .content("**Commandes staff :**")
        .components(vec![staff_row]);
    if let Err(e) = channel.send_message(&ctx.http, staff_msg).await {
        error!(error = %e, channel = %channel.id, "Erreur envoi commandes staff");
    }

    // Supprimer la reponse ephemeral (le "en chargement...") pour ne pas polluer
    let _ = modal.delete_response(&ctx.http).await;

    info!(
        ticket_id = %ticket_id,
        author = %author.name,
        channel = %channel.name,
        ticket_type = %ticket_type,
        "Ticket cree (salon isole)"
    );
}

/// Bouton "Fermer le ticket"
/// - Utilisateur : envoie une demande visible, en attente de validation du staff
/// - Admin/modo : affiche les boutons de confirmation en ephemeral (lui seul les voit)
pub async fn handle_close_button(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(id) => id,
        None => return,
    };

    // Verifier si c'est un admin/moderateur
    let is_staff = match guild_id.member(&ctx.http, component.user.id).await {
        Ok(member) => {
            if let Some(guild) = guild_id.to_guild_cached(&ctx.cache) {
                let permissions = guild.member_permissions(&member);
                permissions.manage_messages() || permissions.administrator()
            } else {
                false
            }
        }
        Err(_) => false,
    };

    if is_staff {
        // Staff : boutons de confirmation en ephemeral
        let confirm_btn = CreateButton::new(CLOSE_CONFIRM_ID)
            .label("Valider la fermeture")
            .style(serenity::all::ButtonStyle::Danger);
        let cancel_btn = CreateButton::new(CLOSE_CANCEL_ID)
            .label("Annuler")
            .style(serenity::all::ButtonStyle::Secondary);

        let row = CreateActionRow::Buttons(vec![confirm_btn, cancel_btn]);

        let _ = component.create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("**Voulez-vous fermer ce ticket ?**\nLe salon sera supprime apres validation.")
                    .components(vec![row])
                    .ephemeral(true),
            ),
        ).await;
    } else {
        // Utilisateur : reponse ephemeral + message visible sans boutons
        let _ = component.create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Votre demande de fermeture a ete envoyee au staff.")
                    .ephemeral(true),
            ),
        ).await;

        let msg = CreateMessage::new()
            .content(format!(
                "**<@{}> souhaite fermer ce ticket.**\n\
                 En attente de validation d'un administrateur ou moderateur.",
                component.user.id
            ));

        let _ = component.channel_id.send_message(&ctx.http, msg).await;
    }
}

/// Un admin/modo valide la fermeture du ticket
pub async fn handle_close_confirm(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(id) => id,
        None => return,
    };

    // Verifier que c'est bien un admin ou moderateur
    let is_staff = match guild_id.member(&ctx.http, component.user.id).await {
        Ok(member) => {
            if let Some(guild) = guild_id.to_guild_cached(&ctx.cache) {
                let permissions = guild.member_permissions(&member);
                permissions.manage_messages() || permissions.administrator()
            } else {
                false
            }
        }
        Err(_) => false,
    };

    if !is_staff {
        let _ = component.create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Seuls les administrateurs et moderateurs peuvent valider la fermeture.")
                    .ephemeral(true),
            ),
        ).await;
        return;
    }

    let channel_id = component.channel_id;
    let channel_name = channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild())
        .map(|c| c.name.clone())
        .unwrap_or_default();

    let ticket_id = get_ticket_id_from_channel(ctx, channel_id).await;

    // Repondre
    let _ = component.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(format!(
                    "Ticket ferme par <@{}>. Ce salon sera supprime dans 5 secondes.",
                    component.user.id
                ))
        ),
    ).await;

    // Fermer via API
    let data = ctx.data.read().await;
    if let Some(base) = data.get::<ApiClientKey>() {
        if let Some(ref id) = ticket_id {
            let api = ApiClient::new(base.clone());
            if let Err(e) = api.close_ticket(id).await {
                error!(error = %e, ticket_id = %id, "Erreur fermeture ticket API");
            }
        } else {
            warn!(channel = %channel_name, "Impossible de trouver l'UUID du ticket dans le topic du salon");
        }

        base.send_log(
            "info",
            &component.guild_id.map(|g| g.to_string()).unwrap_or_default(),
            &format!(
                "Ticket ferme : {} (id: {}) par {}",
                channel_name,
                ticket_id.as_deref().unwrap_or("inconnu"),
                component.user.name
            ),
        );
    }

    info!(
        channel = %channel_name,
        ticket_id = %ticket_id.as_deref().unwrap_or("inconnu"),
        user = %component.user.name,
        "Ticket ferme (valide par le staff)"
    );

    // Supprimer le salon vocal associe s'il existe
    let vocal_name = format!("vocal-{}", channel_name);
    if let Ok(channels) = guild_id.channels(&ctx.http).await {
        for (ch_id, ch) in &channels {
            if ch.kind == ChannelType::Voice && ch.name == vocal_name {
                if let Err(e) = ch_id.delete(&ctx.http).await {
                    warn!(error = %e, vocal = %vocal_name, "Impossible de supprimer le salon vocal du ticket");
                } else {
                    info!(vocal = %vocal_name, "Salon vocal du ticket supprime");
                }
            }
        }
    }

    // Envoyer le transcript en DM a l'auteur du ticket
    if let Some(ref id) = ticket_id {
        let data2 = ctx.data.read().await;
        if let Some(base) = data2.get::<ApiClientKey>() {
            let api = ApiClient::new(base.clone());
            if let Ok(detail) = api.get_ticket(id).await {
                // Trouver l'auteur pour lui envoyer le DM
                if let Ok(author_id) = detail.ticket.author_id.parse::<u64>() {
                    let user_id = serenity::model::id::UserId::new(author_id);
                    if let Ok(dm_channel) = user_id.create_dm_channel(&ctx.http).await {
                        let mut transcript = format!(
                            "**Transcript du ticket #{short_id}**\n\
                             **Sujet :** {title}\n\
                             **Type :** {category}\n\
                             **Statut :** Ferme\n\
                             ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n",
                            short_id = &id[..8.min(id.len())],
                            title = detail.ticket.title,
                            category = detail.ticket.category,
                        );

                        for msg in &detail.messages {
                            transcript.push_str(&format!(
                                "**[{}]** {} :\n> {}\n\n",
                                msg.author_role, msg.author_name, msg.content
                            ));
                        }

                        if detail.messages.is_empty() {
                            transcript.push_str("_Aucun message dans ce ticket._\n");
                        }

                        // Discord limite a 2000 caracteres par message
                        for chunk in transcript.as_bytes().chunks(1900) {
                            let text = String::from_utf8_lossy(chunk);
                            let _ = dm_channel.say(&ctx.http, &*text).await;
                        }
                    }
                }
            }
        }
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    let _ = channel_id.delete(&ctx.http).await;
}

/// Un admin/modo refuse la fermeture — le ticket reste ouvert
pub async fn handle_close_cancel(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(id) => id,
        None => return,
    };

    // Verifier que c'est bien un admin ou moderateur
    let is_staff = match guild_id.member(&ctx.http, component.user.id).await {
        Ok(member) => {
            if let Some(guild) = guild_id.to_guild_cached(&ctx.cache) {
                let permissions = guild.member_permissions(&member);
                permissions.manage_messages() || permissions.administrator()
            } else {
                false
            }
        }
        Err(_) => false,
    };

    if !is_staff {
        let _ = component.create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Seuls les administrateurs et moderateurs peuvent gerer la fermeture.")
                    .ephemeral(true),
            ),
        ).await;
        return;
    }

    let _ = component.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(format!(
                    "<@{}> a decide de garder ce ticket ouvert. La discussion continue.",
                    component.user.id
                ))
        ),
    ).await;
}

/// Bouton "Inviter quelqu'un" — reserve a l'utilisateur (pas le staff)
pub async fn handle_invite_button(ctx: &Context, component: &ComponentInteraction) {
    // Seul l'utilisateur du ticket peut inviter, pas le staff
    if let Some(guild_id) = component.guild_id {
        let is_staff = match guild_id.member(&ctx.http, component.user.id).await {
            Ok(member) => {
                if let Some(guild) = guild_id.to_guild_cached(&ctx.cache) {
                    let permissions = guild.member_permissions(&member);
                    permissions.manage_messages() || permissions.administrator()
                } else {
                    false
                }
            }
            Err(_) => false,
        };

        if is_staff {
            let _ = component.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Seul l'utilisateur du ticket peut inviter des personnes.")
                        .ephemeral(true),
                ),
            ).await;
            return;
        }
    }

    let select = CreateSelectMenu::new(
        INVITE_SELECT_ID,
        CreateSelectMenuKind::User { default_users: None },
    )
    .placeholder("Selectionnez un membre a inviter...");

    let row = CreateActionRow::SelectMenu(select);

    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("**Qui souhaitez-vous inviter dans ce ticket ?**")
            .components(vec![row])
            .ephemeral(true),
    );

    if let Err(e) = component.create_response(&ctx.http, response).await {
        error!(error = %e, "Erreur affichage menu invitation");
    }
}

/// Gere la selection d'un utilisateur a inviter via le UserSelect
pub async fn handle_invite_select(ctx: &Context, component: &ComponentInteraction) {
    let user_ids = match &component.data.kind {
        serenity::all::ComponentInteractionDataKind::UserSelect { values } => values.clone(),
        _ => return,
    };

    let user_id = match user_ids.first() {
        Some(id) => *id,
        None => return,
    };

    // Ne pas inviter un bot
    if let Ok(user) = user_id.to_user(&ctx.http).await {
        if user.bot {
            let _ = component.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Impossible d'inviter un bot dans le ticket.")
                        .ephemeral(true),
                ),
            ).await;
            return;
        }
    }

    // Ajouter la permission VIEW_CHANNEL pour cet utilisateur
    let overwrite = PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(user_id),
    };

    if let Err(e) = component.channel_id.create_permission(&ctx.http, overwrite).await {
        error!(error = %e, "Impossible d'inviter l'utilisateur");
        let _ = component.create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Erreur lors de l'invitation.")
                    .ephemeral(true),
            ),
        ).await;
        return;
    }

    // Message visible dans le salon
    let _ = component.channel_id.say(
        &ctx.http,
        format!("<@{}> a ete invite dans ce ticket.", user_id),
    ).await;

    let _ = component.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(format!("<@{}> a ete invite avec succes !", user_id))
                .ephemeral(true),
        ),
    ).await;

    // Sync invited_user_id vers l'API
    if let Some(ref ticket_id) = get_ticket_id_from_channel(ctx, component.channel_id).await {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let api = ApiClient::new(base.clone());
            if let Err(e) = api.update_ticket_channel(ticket_id, None, Some(user_id.to_string())).await {
                error!(error = %e, ticket_id = %ticket_id, "Erreur sync invited_user_id vers API");
            }
        }
    }

    info!(user_id = %user_id, channel = %component.channel_id, "Utilisateur invite dans le ticket via menu");
}

/// Bouton "Passer en vocal" — reserve aux admins/modos
/// Propose directement a l'utilisateur (pas d'etape de confirmation staff)
pub async fn handle_vocal_button(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(id) => id,
        None => return,
    };

    // Verifier que l'utilisateur est admin ou moderateur
    let is_staff = match guild_id.member(&ctx.http, component.user.id).await {
        Ok(member) => {
            if let Some(guild) = guild_id.to_guild_cached(&ctx.cache) {
                let permissions = guild.member_permissions(&member);
                permissions.manage_messages() || permissions.administrator()
            } else {
                false
            }
        }
        Err(_) => false,
    };

    if !is_staff {
        let _ = component.create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Seuls les administrateurs et moderateurs peuvent proposer un vocal.")
                    .ephemeral(true),
            ),
        ).await;
        return;
    }

    // Verifier qu'un salon vocal n'existe pas deja
    let channel_name = component.channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild())
        .map(|c| c.name.clone())
        .unwrap_or_default();

    let vocal_name = format!("vocal-{}", channel_name);
    if let Ok(channels) = guild_id.channels(&ctx.http).await {
        if channels.values().any(|c| c.kind == ChannelType::Voice && c.name == vocal_name) {
            let _ = component.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Un salon vocal existe deja pour ce ticket.")
                        .ephemeral(true),
                ),
            ).await;
            return;
        }
    }

    // Verifier qu'il n'y a pas deja une proposition en attente
    let bot_id = ctx.cache.current_user().id;
    if let Ok(messages) = component.channel_id.messages(&ctx.http, serenity::all::GetMessages::new().limit(10)).await {
        let pending = messages.iter().any(|m| {
            m.author.id == bot_id
                && m.content.contains("Souhaitez-vous passer en vocal")
                && !m.components.is_empty()
        });

        if pending {
            let _ = component.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Une proposition de vocal est deja en attente de reponse.")
                        .ephemeral(true),
                ),
            ).await;
            return;
        }
    }

    // Repondre au staff en ephemeral (seul le staff voit ca)
    let _ = component.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Proposition de vocal envoyee.")
                .ephemeral(true),
        ),
    ).await;

    // Message visible avec boutons pour l'utilisateur
    let accept_btn = CreateButton::new(VOCAL_USER_ACCEPT_ID)
        .label("Oui, passer en vocal")
        .style(serenity::all::ButtonStyle::Success);
    let decline_btn = CreateButton::new(VOCAL_USER_DECLINE_ID)
        .label("Non merci")
        .style(serenity::all::ButtonStyle::Secondary);

    let row = CreateActionRow::Buttons(vec![accept_btn, decline_btn]);

    let msg = CreateMessage::new()
        .content(
            "Le staff vous propose de passer en **vocal** pour echanger plus facilement.\n\
             Un salon vocal prive sera cree si vous acceptez.\n\n\
             Souhaitez-vous passer en vocal ?"
        )
        .components(vec![row]);

    let _ = component.channel_id.send_message(&ctx.http, msg).await;
}

/// L'utilisateur accepte le passage en vocal — creation du salon
pub async fn handle_vocal_user_accept(ctx: &Context, component: &ComponentInteraction) {
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

    // Verifier qu'un salon vocal n'existe pas deja pour ce ticket
    if let Ok(channels) = guild_id.channels(&ctx.http).await {
        let already_exists = channels.values().any(|c| {
            c.kind == ChannelType::Voice && c.name == vocal_name
        });

        if already_exists {
            let _ = component.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Le salon vocal existe deja pour ce ticket.")
                        .ephemeral(true),
                ),
            ).await;
            return;
        }
    }

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
            // Remplacer le message de proposition par le resultat (supprime les boutons)
            let _ = component.create_response(
                &ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .content(format!(
                            "Salon vocal cree ! Rejoignez <#{}> pour discuter.",
                            vc.id
                        ))
                        .components(vec![])
                ),
            ).await;

            info!(vocal = %vc.name, ticket = %channel_name, "Salon vocal cree pour ticket (accepte par l'utilisateur)");

            // Sync voice_channel_id vers l'API
            if let Some(ref ticket_id) = get_ticket_id_from_channel(ctx, component.channel_id).await {
                let data = ctx.data.read().await;
                if let Some(base) = data.get::<ApiClientKey>() {
                    let api = ApiClient::new(base.clone());
                    if let Err(e) = api.update_ticket_channel(ticket_id, Some(vc.id.to_string()), None).await {
                        error!(error = %e, ticket_id = %ticket_id, "Erreur sync voice_channel_id vers API");
                    }
                }
            }
        }
        Err(e) => {
            error!(error = %e, "Impossible de creer le salon vocal");
            let _ = component.create_response(
                &ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .content("Impossible de creer le salon vocal.")
                        .components(vec![])
                ),
            ).await;
        }
    }
}

/// L'utilisateur refuse le passage en vocal
pub async fn handle_vocal_user_decline(ctx: &Context, component: &ComponentInteraction) {
    // Remplacer le message de proposition par le refus (supprime les boutons)
    let _ = component.create_response(
        &ctx.http,
        CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new()
                .content(format!(
                    "<@{}> a decline la discussion vocale. La conversation continue a l'ecrit.",
                    component.user.id
                ))
                .components(vec![])
        ),
    ).await;
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

    // Recuperer l'UUID du ticket depuis le topic du salon
    let ticket_id = get_ticket_id_from_channel(ctx, command.channel_id).await;

    reply(ctx, command, "Fermeture du ticket...").await?;

    let data = ctx.data.read().await;
    if let Some(base) = data.get::<ApiClientKey>() {
        if let Some(ref id) = ticket_id {
            let api = ApiClient::new(base.clone());
            if let Err(e) = api.close_ticket(id).await {
                error!(error = %e, ticket_id = %id, "Erreur fermeture ticket API");
            }
        } else {
            warn!(channel = %channel_name, "Impossible de trouver l'UUID du ticket dans le topic du salon");
        }

        base.send_log(
            "info",
            &command.guild_id.map(|g| g.to_string()).unwrap_or_default(),
            &format!(
                "Ticket ferme : {} (id: {}) par {}",
                channel_name,
                ticket_id.as_deref().unwrap_or("inconnu"),
                command.user.name
            ),
        );
    }

    // Supprimer le salon vocal associe s'il existe
    if let Some(guild_id) = command.guild_id {
        let vocal_name = format!("vocal-{}", channel_name);
        if let Ok(channels) = guild_id.channels(&ctx.http).await {
            for (ch_id, ch) in &channels {
                if ch.kind == ChannelType::Voice && ch.name == vocal_name {
                    if let Err(e) = ch_id.delete(&ctx.http).await {
                        warn!(error = %e, vocal = %vocal_name, "Impossible de supprimer le salon vocal du ticket");
                    } else {
                        info!(vocal = %vocal_name, "Salon vocal du ticket supprime");
                    }
                }
            }
        }
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

/// Extrait l'UUID du ticket depuis le topic du salon.
/// Le topic contient `[ticket:UUID]` au debut.
pub fn extract_ticket_id_from_topic(topic: &str) -> Option<&str> {
    let start = topic.find("[ticket:")? + "[ticket:".len();
    let end = topic[start..].find(']')? + start;
    let id = &topic[start..end];
    if id.is_empty() { None } else { Some(id) }
}

/// Recupere l'UUID du ticket depuis le topic d'un salon Discord.
pub async fn get_ticket_id_from_channel(ctx: &Context, channel_id: ChannelId) -> Option<String> {
    let channel = channel_id.to_channel(&ctx.http).await.ok()?;
    let guild_channel = channel.guild()?;
    let topic = guild_channel.topic.as_deref()?;
    extract_ticket_id_from_topic(topic).map(|s| s.to_string())
}

/// Verifie si un custom_id correspond a un modal de ticket
pub fn is_ticket_modal(custom_id: &str) -> bool {
    custom_id.starts_with(MODAL_ID_PREFIX)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Tests extract_ticket_id_from_topic ──

    #[test]
    fn test_extract_ticket_id_valid() {
        let topic = "[ticket:550e8400-e29b-41d4-a716-446655440000] Question — testuser";
        let id = extract_ticket_id_from_topic(topic).unwrap();
        assert_eq!(id, "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_extract_ticket_id_no_bracket() {
        let topic = "Juste un topic normal sans ticket";
        assert!(extract_ticket_id_from_topic(topic).is_none());
    }

    #[test]
    fn test_extract_ticket_id_empty() {
        let topic = "[ticket:] Vide";
        assert!(extract_ticket_id_from_topic(topic).is_none());
    }

    #[test]
    fn test_extract_ticket_id_malformed() {
        let topic = "[ticket:abc";
        assert!(extract_ticket_id_from_topic(topic).is_none());
    }

    #[test]
    fn test_extract_ticket_id_middle_of_topic() {
        let topic = "Prefix [ticket:my-uuid-123] Suffix";
        let id = extract_ticket_id_from_topic(topic).unwrap();
        assert_eq!(id, "my-uuid-123");
    }

    // ── Tests is_ticket_modal ──

    #[test]
    fn test_is_ticket_modal_valid() {
        assert!(is_ticket_modal("sentinel_ticket_modal:probleme_serveur"));
        assert!(is_ticket_modal("sentinel_ticket_modal:question"));
    }

    #[test]
    fn test_is_ticket_modal_invalid() {
        assert!(!is_ticket_modal("sentinel_ticket_create"));
        assert!(!is_ticket_modal("other_modal"));
        assert!(!is_ticket_modal(""));
    }

    // ── Tests constantes types ──

    #[test]
    fn test_admin_only_types() {
        assert!(ADMIN_ONLY_TYPES.contains(&"probleme_moderateur"));
        assert!(!ADMIN_ONLY_TYPES.contains(&"question"));
        assert!(!ADMIN_ONLY_TYPES.contains(&"probleme_serveur"));
    }

    #[test]
    fn test_urgent_types() {
        assert!(URGENT_TYPES.contains(&"urgence_detresse"));
        assert!(!URGENT_TYPES.contains(&"question"));
        assert!(!URGENT_TYPES.contains(&"probleme_serveur"));
    }

    // ── Tests TICKET_TYPES ──

    #[test]
    fn test_ticket_types_count() {
        assert_eq!(TICKET_TYPES.len(), 7);
    }

    #[test]
    fn test_ticket_types_no_suggestion() {
        assert!(!TICKET_TYPES.iter().any(|(v, _, _)| *v == "suggestion"));
    }

    #[test]
    fn test_ticket_types_has_moderateur() {
        assert!(TICKET_TYPES.iter().any(|(v, _, _)| *v == "probleme_moderateur"));
    }

    #[test]
    fn test_ticket_types_has_urgence() {
        assert!(TICKET_TYPES.iter().any(|(v, _, _)| *v == "urgence_detresse"));
    }

    #[test]
    fn test_ticket_types_all_have_labels_and_descriptions() {
        for (value, label, desc) in TICKET_TYPES {
            assert!(!value.is_empty(), "value vide");
            assert!(!label.is_empty(), "label vide pour {value}");
            assert!(!desc.is_empty(), "description vide pour {value}");
        }
    }

    // ── Tests custom_id constants ──

    #[test]
    fn test_custom_ids_are_unique() {
        let ids = vec![
            PANEL_BUTTON_ID,
            TYPE_SELECT_ID,
            CLOSE_BUTTON_ID,
            INVITE_BUTTON_ID,
            INVITE_SELECT_ID,
            VOCAL_BUTTON_ID,
            VOCAL_USER_ACCEPT_ID,
            VOCAL_USER_DECLINE_ID,
            CLOSE_CONFIRM_ID,
            CLOSE_CANCEL_ID,
        ];
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(ids.len(), unique.len(), "Des custom_id sont en doublon");
    }

    #[test]
    fn test_custom_ids_start_with_sentinel() {
        let ids = vec![
            PANEL_BUTTON_ID, TYPE_SELECT_ID, CLOSE_BUTTON_ID,
            INVITE_BUTTON_ID, INVITE_SELECT_ID, VOCAL_BUTTON_ID,
            VOCAL_USER_ACCEPT_ID, VOCAL_USER_DECLINE_ID,
            CLOSE_CONFIRM_ID, CLOSE_CANCEL_ID,
        ];
        for id in ids {
            assert!(id.starts_with("sentinel_"), "'{id}' ne commence pas par sentinel_");
        }
    }
}
