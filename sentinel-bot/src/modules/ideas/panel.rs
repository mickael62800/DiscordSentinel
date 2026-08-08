//! Panneau public + modale de proposition + creation du salon prive.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use serenity::all::{
    ChannelId, ComponentInteraction, Context, CreateActionRow, CreateButton, CreateInputText,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateModal, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption, EditInteractionResponse, InputTextStyle,
    ModalInteraction, PermissionOverwrite, PermissionOverwriteType,
};
use serenity::builder::{CreateChannel, CreateMessage};
use serenity::model::channel::ChannelType;
use serenity::model::Permissions;
use tracing::{error, info, warn};

use crate::modules::ideas::api_client::{ApiClient, CreateIdeaRequest};
use crate::modules::ideas::embed::{build_idea_embed, build_staff_buttons};
use crate::modules::ideas::MODULE_BOT_NAME;
use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

use super::constants::*;

/// Garde anti double-submit : deux soumissions concurrentes du meme couple
/// (guild, user) ne doivent pas creer deux salons. Meme mecanique que les
/// tickets, verrou RAII libere a la sortie du scope.
static OPEN_IN_PROGRESS: Lazy<Mutex<HashSet<(u64, u64)>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

struct OpenGuard {
    key: (u64, u64),
}

impl OpenGuard {
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

/// Message du panneau public (reutilise par la commande `/idee panneau`).
pub fn build_panel_message() -> CreateMessage {
    let button = CreateButton::new(PANEL_BUTTON_ID)
        .label("Proposer une idee")
        .style(serenity::all::ButtonStyle::Success);

    CreateMessage::new()
        .content(
            "**Boite a idees du serveur**\n\n\
             Une idee pour ameliorer le serveur ? Propose-la ici.\n\
             Un salon prive sera cree pour en discuter avec le staff, \
             qui te dira ce qu'il en retient.\n\n\
             **Ce que tu peux proposer :**\n\
             > **Evenement** — une animation, un concours, une soiree\n\
             > **Salon / categorie** — un nouveau salon ou une reorganisation\n\
             > **Role** — un nouveau role ou un changement de roles\n\
             > **Bot / fonctionnalite** — une commande ou une automatisation\n\
             > **Reglement** — une regle a ajouter, changer ou clarifier\n\
             > **Autre** — tout le reste\n\n\
             Choisis la categorie, puis decris ton idee dans le formulaire.",
        )
        .components(vec![CreateActionRow::Buttons(vec![button])])
}

/// Clic sur « Proposer une idee » -> menu de categorie (ephemere).
pub async fn handle_panel_click(ctx: &Context, component: &ComponentInteraction) {
    let options: Vec<CreateSelectMenuOption> = IDEA_CATEGORIES
        .iter()
        .map(|(value, label, desc)| CreateSelectMenuOption::new(*label, *value).description(*desc))
        .collect();

    let select =
        CreateSelectMenu::new(CATEGORY_SELECT_ID, CreateSelectMenuKind::String { options })
            .placeholder("Choisis la categorie de ton idee...");

    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("**Quelle est la nature de ton idee ?**")
            .components(vec![CreateActionRow::SelectMenu(select)])
            .ephemeral(true),
    );

    if let Err(e) = component.create_response(&ctx.http, response).await {
        error!(error = %e, "Erreur envoi menu categorie idee");
    }
}

/// Charge la config guild du module (vide si indisponible : on retombe sur les
/// defauts plutot que de bloquer la proposition).
async fn guild_config(ctx: &Context, guild_id: u64) -> HashMap<String, String> {
    let data = ctx.data.read().await;
    match data.get::<ApiClientKey>() {
        Some(base) => base
            .get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME)
            .await
            .unwrap_or_default(),
        None => HashMap::new(),
    }
}

/// Selection de la categorie -> modale de saisie.
pub async fn handle_category_select(ctx: &Context, component: &ComponentInteraction) {
    let category = match &component.data.kind {
        serenity::all::ComponentInteractionDataKind::StringSelect { values } => {
            match values.first() {
                Some(v) => v.clone(),
                None => return,
            }
        }
        _ => return,
    };

    let cfg = match component.guild_id {
        Some(gid) => guild_config(ctx, gid.get()).await,
        None => HashMap::new(),
    };

    // Bornes reglables par serveur. Gardes : min <= max sinon defauts, et max
    // plafonne a 4000 (limite Discord des champs de modale).
    let bound =
        |key: &str, default: u64| BaseApiClient::config_u64(&cfg, key, default).clamp(1, 4000);
    let (t_min, t_max) = {
        let (a, b) = (bound("title_min_len", 5), bound("title_max_len", 100));
        if a <= b {
            (a, b)
        } else {
            (5, 100)
        }
    };
    let (d_min, d_max) = {
        let (a, b) = (bound("desc_min_len", 20), bound("desc_max_len", 2000));
        if a <= b {
            (a, b)
        } else {
            (20, 2000)
        }
    };

    let title_input = CreateInputText::new(InputTextStyle::Short, "Titre de l'idee", FIELD_TITLE)
        .placeholder("Resume ton idee en une phrase...")
        .required(true)
        .min_length(t_min as u16)
        .max_length(t_max as u16);

    let description_input =
        CreateInputText::new(InputTextStyle::Paragraph, "Description", FIELD_DESCRIPTION)
            .placeholder("Explique ton idee : a quoi ca sert, comment ca marcherait, pour qui...")
            .required(true)
            .min_length(d_min as u16)
            .max_length(d_max as u16);

    let modal = CreateModal::new(
        format!("{MODAL_ID_PREFIX}{category}"),
        format!("Nouvelle idee — {}", category_label(&category)),
    )
    .components(vec![
        CreateActionRow::InputText(title_input),
        CreateActionRow::InputText(description_input),
    ]);

    if let Err(e) = component
        .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
        .await
    {
        error!(error = %e, "Erreur ouverture modale idee");
    }
    if let Err(e) = component.delete_response(&ctx.http).await {
        tracing::debug!(error = %e, "Impossible de supprimer le menu de categorie");
    }
}

/// Soumission de la modale -> enregistre l'idee et cree son salon prive.
pub async fn handle_modal_submit(ctx: &Context, modal: &ModalInteraction) {
    let category = match modal.data.custom_id.strip_prefix(MODAL_ID_PREFIX) {
        Some(c) => c.to_string(),
        None => return,
    };
    let guild_id = match modal.guild_id {
        Some(id) => id,
        None => return,
    };
    let author = &modal.user;

    let mut title = String::new();
    let mut description = String::new();
    for row in &modal.data.components {
        for comp in &row.components {
            if let serenity::all::ActionRowComponent::InputText(input) = comp {
                match input.custom_id.as_str() {
                    FIELD_TITLE => title = input.value.clone().unwrap_or_default(),
                    FIELD_DESCRIPTION => description = input.value.clone().unwrap_or_default(),
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
        warn!(error = %e, "Echec defer modale idee");
    }

    let _guard = match OpenGuard::try_acquire(guild_id.get(), author.id.get()) {
        Some(g) => g,
        None => {
            reply(
                ctx,
                modal,
                "Une proposition est deja en cours d'enregistrement. Patiente un instant.",
            )
            .await;
            return;
        }
    };

    // `base` sert uniquement a lire la config guild (toujours en HTTP) ;
    // les operations sur les idees passent par gRPC.
    let (grpc, cfg) = {
        let data = ctx.data.read().await;
        let base = match data.get::<ApiClientKey>() {
            Some(b) => b.clone(),
            None => {
                error!("ApiClientKey introuvable : idee non enregistree");
                reply(ctx, modal, "Service indisponible, reessaie plus tard.").await;
                return;
            }
        };
        let grpc = match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
            Some(g) => g.clone(),
            None => {
                error!("GrpcClientKey introuvable : idee non enregistree");
                reply(ctx, modal, "Service indisponible, reessaie plus tard.").await;
                return;
            }
        };
        let cfg = base
            .get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME)
            .await
            .unwrap_or_default();
        (grpc, cfg)
    };
    let api = ApiClient::new(grpc);

    // Quota d'idees ouvertes. En cas d'echec de lecture on laisse passer :
    // mieux vaut une idee de trop qu'un membre bloque par une panne API.
    let max_open = BaseApiClient::config_u64(&cfg, "max_open_per_user", 3);
    if max_open > 0 {
        match api
            .open_count(&guild_id.to_string(), &author.id.to_string())
            .await
        {
            Ok(count) if count as u64 >= max_open => {
                reply(
                    ctx,
                    modal,
                    &format!(
                        "Tu as deja {count} idee(s) en cours. Limite : {max_open}. \
                         Attends que le staff tranche les precedentes."
                    ),
                )
                .await;
                return;
            }
            Ok(_) => {}
            Err(e) => warn!(error = %e, "Quota idees illisible, ouverture autorisee"),
        }
    }

    // L'idee est enregistree AVANT le salon : si la creation du salon echoue,
    // la proposition n'est pas perdue et reste visible depuis le web.
    let idea = match api
        .create_idea(&CreateIdeaRequest {
            guild_id: guild_id.to_string(),
            title: title.clone(),
            description: description.clone(),
            category: category.clone(),
            author_id: author.id.to_string(),
            author_name: author.name.clone(),
            channel_id: None,
        })
        .await
    {
        Ok(i) => i,
        Err(e) => {
            error!(error = %e, "Echec enregistrement de l'idee");
            reply(ctx, modal, &format!("Echec de l'enregistrement : {e}")).await;
            return;
        }
    };

    let channel = create_idea_channel(ctx, guild_id, author, &cfg, &idea.id, &title).await;
    let channel_id = match channel {
        Some(c) => c,
        None => {
            reply(
                ctx,
                modal,
                "Ton idee est bien enregistree, mais le salon de discussion n'a pas pu etre cree. \
                 Le staff la verra quand meme.",
            )
            .await;
            return;
        }
    };

    if let Err(e) = api
        .set_channel(&idea.id, Some(&channel_id.to_string()))
        .await
    {
        warn!(error = %e, idea = %idea.id, "Salon cree mais non rattache a l'idee");
    }

    // Carte de l'idee + boutons de decision pour le staff.
    let embed = build_idea_embed(
        &idea.id,
        &title,
        &description,
        &category,
        "nouvelle",
        author,
        &cfg,
    );
    let welcome = BaseApiClient::config_or(&cfg, "welcome_message", "");
    let intro = if welcome.trim().is_empty() {
        format!(
            "<@{}> merci pour ta proposition !\nLe staff va en discuter avec toi ici.",
            author.id
        )
    } else {
        welcome.replace("{user}", &format!("<@{}>", author.id))
    };

    if let Err(e) = channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .content(intro)
                .embed(embed)
                .components(vec![build_staff_buttons()]),
        )
        .await
    {
        warn!(error = %e, "Echec envoi de la carte de l'idee");
    }

    info!(
        idea = %idea.id,
        author = %author.name,
        category = %category,
        "Nouvelle idee proposee"
    );
    reply(
        ctx,
        modal,
        &format!("Ton idee est enregistree : <#{channel_id}>"),
    )
    .await;
}

/// Cree le salon prive : auteur + staff + bot, invisible pour le reste.
async fn create_idea_channel(
    ctx: &Context,
    guild_id: serenity::model::id::GuildId,
    author: &serenity::model::user::User,
    cfg: &HashMap<String, String>,
    idea_id: &str,
    title: &str,
) -> Option<ChannelId> {
    // Nom lisible et unique : slug du titre + debut de l'uuid de l'idee.
    let slug: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(20)
        .collect();
    let suffix: String = idea_id.chars().take(4).collect();
    let channel_name = format!("idee-{}-{}", slug.trim_matches('-'), suffix);

    let mut overwrites = vec![
        PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
            kind: PermissionOverwriteType::Role(guild_id.everyone_role()),
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
    if let Some(role_id) = cfg.get("staff_role_id").and_then(|v| v.parse::<u64>().ok()) {
        overwrites.push(PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL
                | Permissions::SEND_MESSAGES
                | Permissions::READ_MESSAGE_HISTORY
                | Permissions::MANAGE_CHANNELS,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Role(serenity::model::id::RoleId::new(role_id)),
        });
    }

    let mut create = CreateChannel::new(&channel_name)
        .kind(ChannelType::Text)
        .topic(format!("Idee — {title} (par {})", author.name))
        .permissions(overwrites);
    if let Some(cat) = cfg
        .get("idea_category_id")
        .and_then(|v| v.parse::<u64>().ok())
    {
        create = create.category(ChannelId::new(cat));
    }

    match guild_id.create_channel(&ctx.http, create).await {
        Ok(c) => Some(c.id),
        Err(e) => {
            error!(error = %e, "Impossible de creer le salon de l'idee");
            None
        }
    }
}

async fn reply(ctx: &Context, modal: &ModalInteraction, content: &str) {
    if let Err(e) = modal
        .edit_response(&ctx.http, EditInteractionResponse::new().content(content))
        .await
    {
        warn!(error = %e, "Echec reponse modale idee");
    }
}
