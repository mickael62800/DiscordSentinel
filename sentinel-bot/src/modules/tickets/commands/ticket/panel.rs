use serenity::all::{
    ChannelId, ComponentInteraction, Context, CreateActionRow, CreateButton, CreateInputText,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateModal, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption, EditChannel, InputTextStyle, ModalInteraction,
    PermissionOverwrite, PermissionOverwriteType,
};
use serenity::builder::{CreateChannel, CreateEmbed, CreateMessage};
use serenity::model::channel::ChannelType;
use serenity::model::Permissions;
use tracing::{error, info, warn};

use crate::shared::heartbeat::ApiClientKey;

use crate::modules::tickets::api_client::{ApiClient, CreateTicketRequest};

use super::constants::*;

use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::sync::Mutex;

/// Garde anti double-submit : empeche deux soumissions concurrentes du modal
/// pour le meme couple (guild, user) de creer deux tickets/salons (race
/// check-then-create lors d'un double-clic). La limite `max_open_per_user`
/// reste geree separement (et garde son defaut illimite).
static OPEN_IN_PROGRESS: Lazy<Mutex<HashSet<(u64, u64)>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

/// Verrou RAII (guild, user) : libere automatiquement a la fin du scope, y
/// compris sur les `return` precoces et en cas d'erreur.
struct OpenGuard {
    key: (u64, u64),
}

impl OpenGuard {
    /// Tente d'acquerir le verrou. Retourne `None` si une ouverture est deja
    /// en cours pour ce couple (guild, user).
    fn try_acquire(guild: u64, user: u64) -> Option<Self> {
        let mut set = OPEN_IN_PROGRESS.lock().unwrap();
        if set.insert((guild, user)) {
            Some(Self { key: (guild, user) })
        } else {
            None
        }
    }
}

impl Drop for OpenGuard {
    fn drop(&mut self) {
        OPEN_IN_PROGRESS.lock().unwrap().remove(&self.key);
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
             > **Probleme avec un moderateur** — Confidentiel, remonte directement aux proprietaires du serveur\n\
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
        .map(|(value, label, desc)| CreateSelectMenuOption::new(*label, *value).description(*desc))
        .collect();

    let select = CreateSelectMenu::new(TYPE_SELECT_ID, CreateSelectMenuKind::String { options })
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

    let modal_id = format!("{}{}", MODAL_ID_PREFIX, ticket_type);

    // Bornes des champs de la modale, reglables par serveur (config `ticket-bot`).
    // Defauts = valeurs historiques -> aucun changement tant que non reconfigure.
    // Gardes : min <= max, sinon on retombe sur les defauts ; max plafonne a la
    // limite modale Discord (4000).
    let (subject_min, subject_max, desc_min, desc_max) = {
        let guild_config = if let Some(gid) = component.guild_id {
            let data = ctx.data.read().await;
            match data.get::<ApiClientKey>() {
                Some(base) => base
                    .get_guild_config_for(
                        &gid.to_string(),
                        crate::modules::tickets::MODULE_BOT_NAME,
                    )
                    .await
                    .unwrap_or_default(),
                None => std::collections::HashMap::new(),
            }
        } else {
            std::collections::HashMap::new()
        };
        let cfg = |key: &str, default: u64, cap: u64| {
            crate::shared::api_client::BaseApiClient::config_u64(&guild_config, key, default)
                .clamp(1, cap)
        };
        let s_min = cfg("ticket_subject_min_len", 5, 4000);
        let s_max = cfg("ticket_subject_max_len", 100, 4000);
        let d_min = cfg("ticket_desc_min_len", 10, 4000);
        let d_max = cfg("ticket_desc_max_len", 2000, 4000);
        let (s_min, s_max) = if s_min <= s_max {
            (s_min, s_max)
        } else {
            (5, 100)
        };
        let (d_min, d_max) = if d_min <= d_max {
            (d_min, d_max)
        } else {
            (10, 2000)
        };
        (s_min as u16, s_max as u16, d_min as u16, d_max as u16)
    };

    let subject_input = CreateInputText::new(InputTextStyle::Short, "Sujet", "ticket_subject")
        .placeholder("Resumez votre demande en quelques mots...")
        .required(true)
        .min_length(subject_min)
        .max_length(subject_max);

    let description_input = CreateInputText::new(
        InputTextStyle::Paragraph,
        "Description",
        "ticket_description",
    )
    .placeholder(
        "Decrivez votre probleme en detail : que s'est-il passe, quand, qui est concerne...",
    )
    .required(true)
    .min_length(desc_min)
    .max_length(desc_max);

    let modal =
        CreateModal::new(&modal_id, format!("Nouveau ticket — {}", type_label)).components(vec![
            CreateActionRow::InputText(subject_input),
            CreateActionRow::InputText(description_input),
        ]);

    if let Err(e) = component
        .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
        .await
    {
        error!(error = %e, "Erreur ouverture modal ticket");
    }

    if let Err(e) = component.delete_response(&ctx.http).await {
        tracing::debug!(error = %e, "Impossible de supprimer le message du dropdown");
    }
}

/// Gere la soumission du modal -> cree le salon prive et le ticket
pub async fn handle_modal_submit(ctx: &Context, modal: &ModalInteraction) {
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

    if let Err(e) = modal
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Failed to defer modal response");
    }

    // Anti double-submit : un second submit concurrent du meme (guild, user)
    // est rejete tant que le premier n'a pas termine. Le verrou est libere
    // automatiquement (RAII) a la sortie de la fonction.
    let _open_guard = match OpenGuard::try_acquire(guild_id.get(), author.id.get()) {
        Some(g) => g,
        None => {
            if let Err(e) = modal
                .edit_response(
                    &ctx.http,
                    serenity::builder::EditInteractionResponse::new()
                        .content("Une ouverture de ticket est deja en cours. Merci de patienter."),
                )
                .await
            {
                warn!(error = %e, "Failed to send double-submit response");
            }
            return;
        }
    };

    // Rate limiting : verifier max_open_per_user
    {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let guild_config = match base
                .get_guild_config_for(
                    &guild_id.to_string(),
                    crate::modules::tickets::MODULE_BOT_NAME,
                )
                .await
            {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                    std::collections::HashMap::new()
                }
            };
            // Defaut NON nul (3) : avant, 0 = illimite -> tout membre pouvait
            // creer des salons en boucle (spam/DoS). 0 reste possible en config
            // explicite si un serveur veut vraiment l'illimite.
            let max_open: u64 = crate::shared::api_client::BaseApiClient::config_u64(
                &guild_config,
                "max_open_per_user",
                3,
            );

            if max_open > 0 {
                let grpc = match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
                    Some(g) => g.clone(),
                    None => return,
                };
                let api = ApiClient::new(grpc);
                if let Ok(tickets) = api.list_tickets().await {
                    // Compteur scope a CE serveur (list_tickets cote bot n'est
                    // pas scope -> sans ce filtre, on comptait les tickets de
                    // toutes les guildes et on bloquait/comptait a tort).
                    let guild_str = guild_id.to_string();
                    let open_count = tickets
                        .iter()
                        .filter(|t| {
                            t.author_id == author.id.to_string()
                                && t.status != "closed"
                                && t.server == guild_str
                        })
                        .count() as u64;

                    if open_count >= max_open {
                        if let Err(e) = modal.edit_response(
                            &ctx.http,
                            serenity::builder::EditInteractionResponse::new()
                                .content(format!(
                                    "Vous avez deja {} ticket(s) ouvert(s). Limite : {} par utilisateur.",
                                    open_count, max_open
                                ))
                        ).await {
                            warn!(error = %e, "Failed to send rate limit response");
                        }
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

    if let Ok(channels) = guild_id.channels(&ctx.http).await {
        let exists = channels.values().any(|c| c.name == channel_name);
        if exists {
            if let Err(e) = modal.delete_response(&ctx.http).await {
                warn!(error = %e, "Failed to delete duplicate ticket response");
            }
            return;
        }
    }

    let everyone_role = guild_id.everyone_role();
    let overwrites = vec![
        PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
            kind: PermissionOverwriteType::Role(everyone_role),
        },
        PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL
                | Permissions::SEND_MESSAGES
                | Permissions::READ_MESSAGE_HISTORY
                | Permissions::ATTACH_FILES,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(author.id),
        },
        PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL
                | Permissions::SEND_MESSAGES
                | Permissions::READ_MESSAGE_HISTORY
                | Permissions::MANAGE_CHANNELS,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(ctx.cache.current_user().id),
        },
    ];

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(b) => b,
        None => {
            error!("ApiClientKey introuvable dans le context");
            return;
        }
    };
    let grpc = match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
        Some(g) => g.clone(),
        None => {
            error!("GrpcClientKey introuvable dans le context");
            return;
        }
    };
    let api = ApiClient::new(grpc);
    let guild_config = match base
        .get_guild_config_for(
            &guild_id.to_string(),
            crate::modules::tickets::MODULE_BOT_NAME,
        )
        .await
    {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
            std::collections::HashMap::new()
        }
    };

    let mut all_overwrites = overwrites;
    let is_admin_only = ADMIN_ONLY_TYPES.contains(&ticket_type.as_str());

    if is_admin_only {
        // « Probleme avec un moderateur » : ca remonte AU PLUS HAUT — les
        // PROPRIETAIRES du serveur (owner Discord + co-fondateurs configures),
        // PAS les admins ni les mods. On accorde l'acces aux proprietaires et on
        // refuse la vue au role moderateur.
        let mut owner_ids: Vec<u64> = Vec::new();
        if let Ok(partial) = guild_id.to_partial_guild(&ctx.http).await {
            owner_ids.push(partial.owner_id.get());
        }
        if let Some(csv) = guild_config.get("ticket_owner_ids") {
            for id in csv.split(',').filter_map(|s| s.trim().parse::<u64>().ok()) {
                owner_ids.push(id);
            }
        }
        owner_ids.sort_unstable();
        owner_ids.dedup();
        for uid in owner_ids {
            all_overwrites.push(PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL
                    | Permissions::SEND_MESSAGES
                    | Permissions::READ_MESSAGE_HISTORY
                    | Permissions::MANAGE_CHANNELS,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(serenity::model::id::UserId::new(uid)),
            });
        }
        if let Some(mod_role_str) = guild_config.get("moderator_role_id") {
            if let Ok(role_id) = mod_role_str.parse::<u64>() {
                all_overwrites.push(PermissionOverwrite {
                    allow: Permissions::empty(),
                    deny: Permissions::VIEW_CHANNEL,
                    kind: PermissionOverwriteType::Role(serenity::model::id::RoleId::new(role_id)),
                });
            }
        }
    } else {
        // Tickets normaux : acces admin + moderateur.
        if let Some(admin_role_str) = guild_config.get("admin_role_id") {
            if let Ok(role_id) = admin_role_str.parse::<u64>() {
                all_overwrites.push(PermissionOverwrite {
                    allow: Permissions::VIEW_CHANNEL
                        | Permissions::SEND_MESSAGES
                        | Permissions::READ_MESSAGE_HISTORY
                        | Permissions::MANAGE_CHANNELS,
                    deny: Permissions::empty(),
                    kind: PermissionOverwriteType::Role(serenity::model::id::RoleId::new(role_id)),
                });
            }
        }
        if let Some(mod_role_str) = guild_config.get("moderator_role_id") {
            if let Ok(role_id) = mod_role_str.parse::<u64>() {
                all_overwrites.push(PermissionOverwrite {
                    allow: Permissions::VIEW_CHANNEL
                        | Permissions::SEND_MESSAGES
                        | Permissions::READ_MESSAGE_HISTORY,
                    deny: Permissions::empty(),
                    kind: PermissionOverwriteType::Role(serenity::model::id::RoleId::new(role_id)),
                });
            }
        }
    }

    let category_id = guild_config
        .get("ticket_category_id")
        .and_then(|v| v.parse::<u64>().ok())
        .or_else(|| {
            std::env::var("TICKET_CATEGORY_ID")
                .ok()
                .and_then(|v| v.parse().ok())
        });

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
            if let Err(edit) = modal
                .edit_response(
                    &ctx.http,
                    serenity::builder::EditInteractionResponse::new().content(
                        "Echec de l'ouverture du ticket (creation du salon). Merci de reessayer.",
                    ),
                )
                .await
            {
                warn!(error = %edit, "Failed to send channel-create failure response");
            }
            return;
        }
    };

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
        guild_id: Some(guild_id.to_string()),
        category: ticket_type.clone(),
        ticket_type: ticket_type.clone(),
        channel_id: Some(channel.id.to_string()),
    };

    let ticket_id = match api.create_ticket(&request).await {
        Ok(t) => t.id.clone(),
        Err(e) => {
            error!(error = %e, "Erreur creation ticket API");
            // Compensation : la ligne DB n'existe pas, on supprime le salon
            // orphelin pour eviter un salon prive sans mapping `[ticket:]`
            // (messages jamais mirrores, close incapable de le retrouver).
            if let Err(del) = channel.id.delete(&ctx.http).await {
                warn!(error = %del, channel = %channel.id, "Echec suppression salon orphelin apres echec API");
            }
            if let Err(edit) = modal
                .edit_response(
                    &ctx.http,
                    serenity::builder::EditInteractionResponse::new().content(
                        "Echec de l'ouverture du ticket (erreur serveur). Merci de reessayer.",
                    ),
                )
                .await
            {
                warn!(error = %edit, "Failed to send API-create failure response");
            }
            return;
        }
    };

    // Enregistrer la creation dans le SLA tracker
    {
        let data = ctx.data.read().await;
        if let Some(sla) = data.get::<crate::modules::tickets::SlaTrackerKey>() {
            sla.record_creation(&ticket_id);
        }
    }

    // Topic critique : sans le marqueur `[ticket:UUID]`, le salon est
    // inutilisable (pas de mirroring des messages, close incapable de mapper).
    // On reessaie une fois ; si l'edit echoue toujours, on rollback (supprime
    // le salon + ferme le ticket cote DB) plutot que de laisser un ticket
    // inutilisable.
    let new_topic = format!(
        "[ticket:{}] [author:{}] {} — {}",
        ticket_id, author.id, type_label, author.name
    );
    let mut topic_ok = channel
        .edit(&ctx.http, EditChannel::new().topic(&new_topic))
        .await
        .is_ok();
    if !topic_ok {
        topic_ok = channel
            .edit(&ctx.http, EditChannel::new().topic(&new_topic))
            .await
            .is_ok();
    }
    if !topic_ok {
        error!(channel = %channel.id, ticket_id = %ticket_id, "Echec critique edit topic ticket : salon inutilisable, rollback");
        if let Err(e) = channel.id.delete(&ctx.http).await {
            warn!(error = %e, channel = %channel.id, "Echec suppression salon apres echec topic");
        }
        if let Err(e) = api.close_ticket(&ticket_id).await {
            warn!(error = %e, ticket_id = %ticket_id, "Echec fermeture ticket apres echec topic");
        }
        if let Err(edit) = modal
            .edit_response(
                &ctx.http,
                serenity::builder::EditInteractionResponse::new().content(
                    "Echec de l'ouverture du ticket (configuration du salon). Merci de reessayer.",
                ),
            )
            .await
        {
            warn!(error = %edit, "Failed to send topic-edit failure response");
        }
        return;
    }

    let staff_line = if is_admin_only {
        "Ce ticket est **strictement confidentiel**. Il remonte directement aux **propriétaires du serveur** — ni les modérateurs ni les administrateurs n'y ont accès.\nUn **propriétaire** vous répondra."
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

    let parse_color =
        |config: &std::collections::HashMap<String, String>, key: &str, default: u32| -> u32 {
            config
                .get(key)
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

    let custom_welcome = guild_config
        .get("welcome_message")
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
        .title(format!(
            "Ticket #{} — {}",
            &ticket_id[..8.min(ticket_id.len())],
            type_label
        ))
        .description(welcome_text)
        .color(embed_color);

    let mut welcome = CreateMessage::new().embed(welcome_embed);
    // Probleme avec un moderateur : on PING les proprietaires pour remonter
    // l'affaire immediatement au plus haut.
    if is_admin_only {
        let mut mentions = String::new();
        if let Ok(partial) = guild_id.to_partial_guild(&ctx.http).await {
            mentions.push_str(&format!("<@{}> ", partial.owner_id));
        }
        if let Some(csv) = guild_config.get("ticket_owner_ids") {
            for id in csv.split(',').filter_map(|s| s.trim().parse::<u64>().ok()) {
                mentions.push_str(&format!("<@{id}> "));
            }
        }
        if !mentions.trim().is_empty() {
            welcome = welcome.content(format!(
                "🚨 {mentions}— signalement **confidentiel** concernant un modérateur."
            ));
        }
    }
    let welcome_posted = match channel.send_message(&ctx.http, welcome).await {
        Ok(msg) => Some(msg),
        Err(e) => {
            error!(error = %e, channel = %channel.id, "Erreur envoi message de bienvenue");
            None
        }
    };

    // Phase 2 sync (cf. SYNC_DISCORD_WEB_DESIGN.md) : enregistre le
    // mapping ticket_uuid <-> message Discord pour permettre les sync
    // bilaterales (close depuis web -> lock channel ; close depuis
    // Discord -> retire de la liste web).
    if let Some(ref welcome_msg) = welcome_posted {
        if let Ok(action_uuid) = uuid::Uuid::parse_str(&ticket_id) {
            let data = ctx.data.read().await;
            if let Some(grpc) = data.get::<crate::shared::grpc_client::GrpcClientKey>() {
                let grpc = std::sync::Arc::clone(grpc);
                let guild_str = guild_id.to_string();
                let ch_str = channel.id.to_string();
                let msg_str = welcome_msg.id.to_string();
                drop(data);
                crate::sync::register_action_message(
                    &grpc,
                    action_uuid,
                    crate::sync::kinds::TICKET,
                    &guild_str,
                    &ch_str,
                    &msg_str,
                )
                .await;
            }
        }
    }

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

    if let Err(e) = modal.delete_response(&ctx.http).await {
        warn!(error = %e, "Failed to delete loading ephemeral response");
    }

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
    let guild_id = component
        .guild_id
        .map(|g| g.to_string())
        .unwrap_or_default();
    let faq_raw = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let gc = match base
                .get_guild_config_for(&guild_id, crate::modules::tickets::MODULE_BOT_NAME)
                .await
            {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                    std::collections::HashMap::new()
                }
            };
            crate::shared::api_client::BaseApiClient::config_or(&gc, "faq_entries", "")
        } else {
            String::new()
        }
    };

    let entries = crate::modules::tickets::faq::parse_faq(&faq_raw);

    if entries.is_empty() {
        handle_panel_click(ctx, component).await;
        return;
    }

    let embed = crate::modules::tickets::faq::build_faq_embed(&entries);
    let row = crate::modules::tickets::faq::build_faq_continue_button();

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
