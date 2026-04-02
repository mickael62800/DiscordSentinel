use serenity::all::{
    Context, CreateActionRow, CreateButton, CreateSelectMenu, CreateSelectMenuKind,
    CreateSelectMenuOption, ComponentInteraction, PermissionOverwrite, PermissionOverwriteType,
    EditChannel, CreateModal, CreateInputText, InputTextStyle, ModalInteraction, ChannelId,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::builder::{CreateChannel, CreateEmbed, CreateMessage};
use serenity::model::channel::ChannelType;
use serenity::model::Permissions;
use tracing::{error, info, warn};

use sentinel_shared::heartbeat::ApiClientKey;

use crate::api_client::{ApiClient, CreateTicketRequest};

use super::constants::*;

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
        "**Auteur :** <@{author_id}>\n\
         **Type :** {type_label}\n\
         **Priorite :** {priority}\n\
         **Sujet :** {subject}\n\n\
         **Description :**\n\
         > {description}\n\n\
         {staff_line}",
        author_id = author.id,
        subject = title,
        description = description.replace('\n', "\n> "),
    );

    // Lire les couleurs depuis la config (ou valeurs par defaut)
    let parse_color = |config: &std::collections::HashMap<String, String>, key: &str, default: u32| -> u32 {
        config.get(key)
            .and_then(|v| u32::from_str_radix(v.trim_start_matches('#'), 16).ok())
            .unwrap_or(default)
    };

    let color_normal = parse_color(&guild_config, "color_normal", 0x2ecc71);
    let color_urgent = parse_color(&guild_config, "color_urgent", 0xff6600);
    let color_confidential = parse_color(&guild_config, "color_confidential", 0xe74c3c);
    let color_staff = parse_color(&guild_config, "color_staff", 0xe67e22);
    let color_user = parse_color(&guild_config, "color_user", 0x3498db);

    let embed_color: u32 = if is_admin_only {
        color_confidential
    } else if URGENT_TYPES.contains(&ticket_type.as_str()) {
        color_urgent
    } else {
        color_normal
    };

    // Message d'accueil personnalise ou par defaut
    let custom_welcome = guild_config.get("welcome_message")
        .filter(|v| !v.is_empty())
        .cloned();

    let welcome_text = if let Some(ref custom) = custom_welcome {
        format!(
            "**Auteur :** <@{author_id}>\n\
             **Type :** {type_label}\n\
             **Priorite :** {priority}\n\
             **Sujet :** {subject}\n\n\
             **Description :**\n\
             > {description}\n\n\
             {custom}",
            author_id = author.id,
            subject = title,
            description = description.replace('\n', "\n> "),
        )
    } else {
        welcome_content
    };

    let welcome_embed = CreateEmbed::new()
        .title(format!("Ticket #{} — {}", &ticket_id[..8.min(ticket_id.len())], type_label))
        .description(welcome_text)
        .color(embed_color);

    let welcome = CreateMessage::new().embed(welcome_embed);
    if let Err(e) = channel.send_message(&ctx.http, welcome).await {
        error!(error = %e, channel = %channel.id, "Erreur envoi message de bienvenue");
    }

    // Boutons staff (embed) — en premier
    let staff_close_btn = CreateButton::new(CLOSE_BUTTON_ID)
        .label("Fermer le ticket")
        .style(serenity::all::ButtonStyle::Danger);
    let vocal_btn = CreateButton::new(VOCAL_BUTTON_ID)
        .label("Proposer un vocal")
        .style(serenity::all::ButtonStyle::Primary);
    let staff_row = CreateActionRow::Buttons(vec![staff_close_btn, vocal_btn]);

    let staff_embed = CreateEmbed::new()
        .title("Commandes staff")
        .description("Reserve aux administrateurs et moderateurs.")
        .color(color_staff);

    let staff_msg = CreateMessage::new()
        .embed(staff_embed)
        .components(vec![staff_row]);
    if let Err(e) = channel.send_message(&ctx.http, staff_msg).await {
        error!(error = %e, channel = %channel.id, "Erreur envoi commandes staff");
    }

    // Boutons utilisateur (embed) — en dessous
    let user_close_btn = CreateButton::new(CLOSE_BUTTON_ID)
        .label("Fermer le ticket")
        .style(serenity::all::ButtonStyle::Danger);
    let invite_btn = CreateButton::new(INVITE_BUTTON_ID)
        .label("Inviter quelqu'un")
        .style(serenity::all::ButtonStyle::Secondary);
    let user_row = CreateActionRow::Buttons(vec![user_close_btn, invite_btn]);

    let user_embed = CreateEmbed::new()
        .title("Commandes utilisateur")
        .description("Utilisez les boutons ci-dessous pour gerer votre ticket.")
        .color(color_user);

    let user_msg = CreateMessage::new()
        .embed(user_embed)
        .components(vec![user_row]);
    if let Err(e) = channel.send_message(&ctx.http, user_msg).await {
        error!(error = %e, channel = %channel.id, "Erreur envoi commandes utilisateur");
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

// ── FAQ : affiche les FAQ avant la creation du ticket ──

/// Gere le clic sur le bouton "Creer un ticket" — avec FAQ intercalee si configuree.
pub async fn handle_panel_click_with_faq(ctx: &Context, component: &ComponentInteraction) {
    // Lire les FAQ depuis la config guild
    let guild_id = component.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let faq_raw = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let gc = base.get_guild_config(&guild_id).await.unwrap_or_default();
            sentinel_shared::api_client::BaseApiClient::config_or(&gc, "faq_entries", "")
        } else {
            String::new()
        }
    };

    let entries = crate::faq::parse_faq(&faq_raw);

    if entries.is_empty() {
        // Pas de FAQ → afficher directement le selecteur de type
        handle_panel_click(ctx, component).await;
        return;
    }

    // Afficher les FAQ + bouton "Creer un ticket quand meme"
    let embed = crate::faq::build_faq_embed(&entries);
    let row = crate::faq::build_faq_continue_button();

    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(vec![row])
            .ephemeral(true),
    );

    if let Err(e) = component.create_response(&ctx.http, response).await {
        error!(error = %e, "Erreur envoi FAQ");
    }
}

/// Gere le clic sur "Ma question n'est pas dans la FAQ — Creer un ticket"
pub async fn handle_faq_continue(ctx: &Context, component: &ComponentInteraction) {
    handle_panel_click(ctx, component).await;
}
