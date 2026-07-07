//! Salon prive du compagnon : choix d'espece, carte, actions de soin.

use serenity::all::{
    ButtonStyle, ChannelId, ChannelType, ComponentInteraction, Context, CreateActionRow,
    CreateAttachment, CreateButton, CreateChannel, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, CreateSelectMenu,
    CreateSelectMenuKind, EditInteractionResponse, PermissionOverwrite, PermissionOverwriteType,
    Permissions, RoleId,
};

use super::card_render::{render_card_png, CardData};
use tracing::{error, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

use super::api_client::{CareArgs, CombatArgs, PetData as PetDto, TamaApi, TrainArgs, VisitArgs};
use super::MODULE_BOT_NAME;

/// Verrou in-flight par joueur : empeche un double-clic (ou retry Discord) de
/// lancer deux actions concurrentes (nourrir/jouer/entrainer/acheter) -> deux
/// debits/credits. Complementaire de la garde de cooldown atomique cote serveur.
fn action_locks() -> &'static dashmap::DashMap<u64, ()> {
    static LOCKS: std::sync::OnceLock<dashmap::DashMap<u64, ()>> = std::sync::OnceLock::new();
    LOCKS.get_or_init(dashmap::DashMap::new)
}

/// Garde RAII du verrou d'action d'un joueur (relachee au Drop, tout chemin).
struct ActionGuard(u64);

impl ActionGuard {
    /// `None` si une action est deja en cours pour ce joueur.
    fn try_acquire(user_id: u64) -> Option<Self> {
        use dashmap::mapref::entry::Entry;
        match action_locks().entry(user_id) {
            Entry::Occupied(_) => None,
            Entry::Vacant(v) => {
                v.insert(());
                Some(ActionGuard(user_id))
            }
        }
    }
}

impl Drop for ActionGuard {
    fn drop(&mut self) {
        action_locks().remove(&self.0);
    }
}

pub const PICK_PREFIX: &str = "tama_pick:";
pub const ACT_PREFIX: &str = "tama_act:";
pub const TRAIN_PREFIX: &str = "tama_train:";
pub const BUY_PREFIX: &str = "tama_buy:";
pub const SHOP_OPEN_ID: &str = "tama_shop";
pub const VISIT_OPEN_ID: &str = "tama_visit";
pub const VISIT_SELECT_ID: &str = "tama_visit_sel";
pub const COMBAT_OPEN_ID: &str = "tama_combat";
pub const COMBAT_SELECT_ID: &str = "tama_combat_sel";
pub const CLOSE_ID: &str = "tama_close";
pub const HIST_ID: &str = "tama_hist";

const SPECIES: [(&str, &str); 6] = [
    ("sanglier", "🐗 Sanglier"),
    ("renard", "🦊 Renard"),
    ("tortue", "🐢 Tortue"),
    ("loup", "🐺 Loup"),
    ("lapin", "🐰 Lapin"),
    ("ours", "🐻 Ours"),
];

async fn get_api(ctx: &Context) -> Option<std::sync::Arc<BaseApiClient>> {
    let data = ctx.data.read().await;
    data.get::<ApiClientKey>().map(std::sync::Arc::clone)
}

/// Client gRPC tamagotchi (compagnons). La config guild + le wallet restent
/// sur `BaseApiClient` (HTTP), domaines distincts.
async fn get_tama(ctx: &Context) -> Option<TamaApi> {
    let data = ctx.data.read().await;
    TamaApi::from_data(&data)
}

/// Lit l'ID du proprietaire depuis le topic du salon (`[tama:<id>]`).
async fn channel_owner_id(ctx: &Context, channel_id: ChannelId) -> Option<u64> {
    let topic = match channel_id.to_channel(&ctx.http).await {
        Ok(serenity::model::channel::Channel::Guild(gc)) => gc.topic,
        _ => None,
    }?;
    let start = topic.find("[tama:")? + "[tama:".len();
    let end = topic[start..].find(']')? + start;
    topic[start..end].parse::<u64>().ok()
}

/// Verifie que l'auteur du clic est bien le proprietaire du salon. Repond un
/// message ephemere et retourne false sinon. SECURITE : empeche un autre
/// membre d'agir sur le compagnon (choix d'espece, soins) dans ce salon.
async fn ensure_owner(ctx: &Context, component: &ComponentInteraction) -> bool {
    match channel_owner_id(ctx, component.channel_id).await {
        Some(owner) if owner == component.user.id.get() => true,
        Some(_) => {
            reply_ephemeral(
                ctx,
                component,
                "Ce n'est pas ton compagnon — ouvre le tien via le panneau.",
            )
            .await;
            false
        }
        // Pas de topic exploitable : on n'autorise pas (fail-closed).
        None => {
            reply_ephemeral(ctx, component, "Salon invalide.").await;
            false
        }
    }
}

// ── Ouverture du salon prive ──

pub async fn handle_open(ctx: &Context, component: &ComponentInteraction) {
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await;

    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };
    let user_id = component.user.id;

    let api = match get_api(ctx).await {
        Some(a) => a,
        None => return,
    };
    let tama = match get_tama(ctx).await {
        Some(t) => t,
        None => return,
    };

    // Categorie configurable.
    let cfg = api
        .get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME)
        .await
        .unwrap_or_default();
    let category_id = cfg
        .get("tama_category_id")
        .and_then(|v| v.parse::<u64>().ok());

    let everyone = RoleId::new(guild_id.get());
    let name = format!(
        "tama-{}",
        component
            .user
            .name
            .chars()
            .take(15)
            .collect::<String>()
            .to_lowercase()
    );
    let mut builder = CreateChannel::new(&name)
        .kind(ChannelType::Text)
        .topic(format!("[tama:{}]", user_id))
        .permissions(vec![
            PermissionOverwrite {
                allow: Permissions::empty(),
                deny: Permissions::VIEW_CHANNEL,
                kind: PermissionOverwriteType::Role(everyone),
            },
            PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL
                    | Permissions::SEND_MESSAGES
                    | Permissions::READ_MESSAGE_HISTORY,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(user_id),
            },
        ]);
    if let Some(cat) = category_id {
        builder = builder.category(ChannelId::new(cat));
    }

    let channel = match guild_id.create_channel(&ctx.http, builder).await {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Echec creation salon tamagotchi");
            let _ = component
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content("Erreur lors de la creation du salon."),
                )
                .await;
            return;
        }
    };

    // Pet existant ?
    let pet = fetch_pet(&tama, &guild_id.to_string(), &user_id.to_string()).await;
    let has_living_pet = matches!(&pet, Some(p) if p.status != "dead");
    let msg = match pet {
        Some(p) if p.status != "dead" => {
            card_message(&api, &guild_id.to_string(), &user_id.to_string(), &p).await
        }
        _ => species_choice_message(),
    };
    let sent = channel.id.send_message(&ctx.http, msg).await;

    // Memorise la position de la carte pour le rafraichissement automatique.
    // (Pour un nouveau joueur, le pet n'existe pas encore -> persiste dans
    // handle_pick apres la naissance.)
    if has_living_pet {
        if let Ok(message) = &sent {
            persist_card_location(
                &tama,
                &guild_id.to_string(),
                &user_id.to_string(),
                channel.id.get(),
                message.id.get(),
            )
            .await;
        }
    }

    let _ = component
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content(format!("Ton salon : <#{}>", channel.id)),
        )
        .await;
}

// ── Choix d'espece (naissance) ──

pub async fn handle_pick(ctx: &Context, component: &ComponentInteraction) {
    if !ensure_owner(ctx, component).await {
        return;
    }
    let _guard = match ActionGuard::try_acquire(component.user.id.get()) {
        Some(g) => g,
        None => {
            reply_ephemeral(ctx, component, "Une action est deja en cours, patiente.").await;
            return;
        }
    };
    let species = component
        .data
        .custom_id
        .strip_prefix(PICK_PREFIX)
        .unwrap_or("")
        .to_string();
    let guild_id = component
        .guild_id
        .map(|g| g.to_string())
        .unwrap_or_default();
    let api = match get_api(ctx).await {
        Some(a) => a,
        None => return,
    };
    let tama = match get_tama(ctx).await {
        Some(t) => t,
        None => return,
    };

    match tama
        .create_pet(
            &guild_id,
            &component.user.id.to_string(),
            &component.user.name,
            &species,
        )
        .await
    {
        Ok(pet) => {
            let resp =
                update_from_card(&api, &guild_id, &component.user.id.to_string(), &pet).await;
            let _ = component
                .create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(resp))
                .await;
            // La carte = le message de choix d'espece qu'on vient d'editer
            // (meme message_id). Memorise sa position pour le refresh auto.
            persist_card_location(
                &tama,
                &guild_id,
                &component.user.id.to_string(),
                component.channel_id.get(),
                component.message.id.get(),
            )
            .await;
        }
        Err(e) => {
            warn!(error = %e, "Echec creation pet");
            reply_ephemeral(
                ctx,
                component,
                "Impossible de creer le compagnon (en as-tu deja un ?).",
            )
            .await;
        }
    }
}

// ── Actions de soin ──

pub async fn handle_action(ctx: &Context, component: &ComponentInteraction) {
    if !ensure_owner(ctx, component).await {
        return;
    }
    let _guard = match ActionGuard::try_acquire(component.user.id.get()) {
        Some(g) => g,
        None => {
            reply_ephemeral(ctx, component, "Une action est deja en cours, patiente.").await;
            return;
        }
    };
    let action = component
        .data
        .custom_id
        .strip_prefix(ACT_PREFIX)
        .unwrap_or("")
        .to_string();
    let guild_id = component
        .guild_id
        .map(|g| g.to_string())
        .unwrap_or_default();
    let user_id = component.user.id.to_string();
    let api = match get_api(ctx).await {
        Some(a) => a,
        None => return,
    };
    let tama = match get_tama(ctx).await {
        Some(t) => t,
        None => return,
    };

    let pet = match fetch_pet(&tama, &guild_id, &user_id).await {
        Some(p) => p,
        None => {
            reply_ephemeral(ctx, component, "Tu n'as pas de compagnon ici.").await;
            return;
        }
    };
    let pet_id = pet.id.clone();

    // Balance (couts/deltas/cooldown) calculee server-side : on n'envoie que
    // l'action. On valide juste que l'action est connue cote bot.
    let args = match action.as_str() {
        "feed" | "play" | "sleep" | "cuddle" => CareArgs {
            action: action.clone(),
        },
        _ => return,
    };

    match tama.care(&pet_id, args).await {
        Ok(updated) => {
            // Evolution : si l'action a fait franchir un palier de stade, on
            // l'annonce publiquement dans le salon.
            if super::card_render::stage_from_level(pet.level)
                != super::card_render::stage_from_level(updated.level)
            {
                let _ = component
                    .channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().content(format!(
                            "🎉 **{}** a évolué en **{}** !",
                            updated.name,
                            super::card_render::stage_label(updated.level),
                        )),
                    )
                    .await;
            }
            let resp = update_from_card(&api, &guild_id, &user_id, &updated).await;
            let _ = component
                .create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(resp))
                .await;
        }
        Err(e) => {
            warn!(error = %e, action, "Echec action soin");
            reply_ephemeral(ctx, component, &format!("\u{26a0}\u{fe0f} {e}")).await;
        }
    }
}

pub async fn handle_train(ctx: &Context, component: &ComponentInteraction) {
    if !ensure_owner(ctx, component).await {
        return;
    }
    let _guard = match ActionGuard::try_acquire(component.user.id.get()) {
        Some(g) => g,
        None => {
            reply_ephemeral(ctx, component, "Une action est deja en cours, patiente.").await;
            return;
        }
    };
    let stat = component
        .data
        .custom_id
        .strip_prefix(TRAIN_PREFIX)
        .unwrap_or("")
        .to_string();
    let guild_id = component
        .guild_id
        .map(|g| g.to_string())
        .unwrap_or_default();
    let user_id = component.user.id.to_string();
    let api = match get_api(ctx).await {
        Some(a) => a,
        None => return,
    };
    let tama = match get_tama(ctx).await {
        Some(t) => t,
        None => return,
    };
    let pet_id = match fetch_pet(&tama, &guild_id, &user_id).await {
        Some(p) => p.id,
        None => return,
    };
    // Balance (cout energie/coins, gain, cooldown) calculee server-side.
    let args = TrainArgs { stat };
    match tama.train(&pet_id, args).await {
        Ok(p) => {
            let resp = update_from_card(&api, &guild_id, &user_id, &p).await;
            let _ = component
                .create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(resp))
                .await;
        }
        Err(e) => {
            warn!(error = %e, "Echec entrainement");
            reply_ephemeral(ctx, component, &format!("\u{26a0}\u{fe0f} {e}")).await;
        }
    }
}

pub async fn handle_shop_open(ctx: &Context, component: &ComponentInteraction) {
    if !ensure_owner(ctx, component).await {
        return;
    }
    let guild_id = component
        .guild_id
        .map(|g| g.to_string())
        .unwrap_or_default();
    let api = match get_api(ctx).await {
        Some(a) => a,
        None => return,
    };
    let cfg = api
        .get_guild_config_for(&guild_id, MODULE_BOT_NAME)
        .await
        .unwrap_or_default();
    let price = |k: &str, d: u64| BaseApiClient::config_u64(&cfg, k, d);
    let buttons = vec![
        CreateButton::new(format!("{BUY_PREFIX}croquettes"))
            .label(format!(
                "Croquettes ({}c)",
                price("shop_croquettes_price", 15)
            ))
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("{BUY_PREFIX}repas"))
            .label(format!("Repas ({}c)", price("shop_repas_price", 40)))
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("{BUY_PREFIX}boisson"))
            .label(format!("Boisson ({}c)", price("shop_boisson_price", 25)))
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("{BUY_PREFIX}jouet"))
            .label(format!("Jouet ({}c)", price("shop_jouet_price", 20)))
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("{BUY_PREFIX}potion"))
            .label(format!(
                "Potion soin ({}c)",
                price("shop_potion_price", 100)
            ))
            .style(ButtonStyle::Success),
    ];
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("🛒 **Boutique** — achete un objet :")
                    .components(vec![CreateActionRow::Buttons(buttons)])
                    .ephemeral(true),
            ),
        )
        .await;
}

pub async fn handle_buy(ctx: &Context, component: &ComponentInteraction) {
    if !ensure_owner(ctx, component).await {
        return;
    }
    let _guard = match ActionGuard::try_acquire(component.user.id.get()) {
        Some(g) => g,
        None => {
            reply_ephemeral(ctx, component, "Une action est deja en cours, patiente.").await;
            return;
        }
    };
    let item = component
        .data
        .custom_id
        .strip_prefix(BUY_PREFIX)
        .unwrap_or("")
        .to_string();
    let guild_id = component
        .guild_id
        .map(|g| g.to_string())
        .unwrap_or_default();
    let user_id = component.user.id.to_string();
    let tama = match get_tama(ctx).await {
        Some(t) => t,
        None => return,
    };
    let pet_id = match fetch_pet(&tama, &guild_id, &user_id).await {
        Some(p) => p.id,
        None => return,
    };
    // Prix ET effets calcules server-side : on n'envoie que l'action
    // (`buy_<item>`). Le label ne sert qu'au message de confirmation.
    let label = match item.as_str() {
        "croquettes" => "Croquettes",
        "repas" => "Repas premium",
        "boisson" => "Boisson energisante",
        "jouet" => "Jouet",
        "potion" => "Potion de soin",
        _ => return,
    };
    let args = CareArgs {
        action: format!("buy_{item}"),
    };
    match tama.care(&pet_id, args).await {
        Ok(_) => reply_ephemeral(ctx, component, &format!("✅ {label} achete et utilise !")).await,
        Err(e) => {
            warn!(error = %e, item, "Echec achat");
            reply_ephemeral(ctx, component, &format!("\u{26a0}\u{fe0f} {e}")).await;
        }
    }
}

pub async fn handle_visit_open(ctx: &Context, component: &ComponentInteraction) {
    if !ensure_owner(ctx, component).await {
        return;
    }
    let menu = CreateSelectMenu::new(
        VISIT_SELECT_ID,
        CreateSelectMenuKind::User {
            default_users: None,
        },
    )
    .placeholder("Choisis un joueur a visiter");
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("👋 **Visite** — son compagnon gagnera un peu d'XP et de coins :")
                    .components(vec![CreateActionRow::SelectMenu(menu)])
                    .ephemeral(true),
            ),
        )
        .await;
}

pub async fn handle_visit_select(ctx: &Context, component: &ComponentInteraction) {
    if !ensure_owner(ctx, component).await {
        return;
    }
    let target = match &component.data.kind {
        serenity::model::application::ComponentInteractionDataKind::UserSelect { values } => {
            values.first().copied()
        }
        _ => None,
    };
    let target = match target {
        Some(t) => t,
        None => return,
    };
    let guild_id = component
        .guild_id
        .map(|g| g.to_string())
        .unwrap_or_default();
    let tama = match get_tama(ctx).await {
        Some(t) => t,
        None => return,
    };

    // Recompenses/cooldown/limite calcules server-side.
    let args = VisitArgs {
        guild_id: guild_id.clone(),
        visitor_id: component.user.id.to_string(),
        visitor_name: component.user.name.clone(),
        target_id: target.to_string(),
    };

    match tama.visit(args).await {
        Ok(r) => {
            reply_ephemeral(
                ctx,
                component,
                &format!(
                    "👋 Tu as rendu visite a **{}** ! Son compagnon gagne +{} XP et +{} coins.",
                    r.target_name, r.xp_reward, r.coins_reward
                ),
            )
            .await;
        }
        Err(e) => {
            warn!(error = %e, "Echec visite");
            reply_ephemeral(ctx, component, &format!("\u{26a0}\u{fe0f} {e}")).await;
        }
    }
}

pub async fn handle_combat_open(ctx: &Context, component: &ComponentInteraction) {
    if !ensure_owner(ctx, component).await {
        return;
    }
    let menu = CreateSelectMenu::new(
        COMBAT_SELECT_ID,
        CreateSelectMenuKind::User {
            default_users: None,
        },
    )
    .placeholder("Choisis un adversaire");
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("⚔️ **Combat** — choisis l'adversaire :")
                    .components(vec![CreateActionRow::SelectMenu(menu)])
                    .ephemeral(true),
            ),
        )
        .await;
}

pub async fn handle_combat_select(ctx: &Context, component: &ComponentInteraction) {
    if !ensure_owner(ctx, component).await {
        return;
    }
    let target = match &component.data.kind {
        serenity::model::application::ComponentInteractionDataKind::UserSelect { values } => {
            values.first().copied()
        }
        _ => None,
    };
    let target = match target {
        Some(t) => t,
        None => return,
    };
    let guild_id = component
        .guild_id
        .map(|g| g.to_string())
        .unwrap_or_default();
    let tama = match get_tama(ctx).await {
        Some(t) => t,
        None => return,
    };
    // Niveau de l'attaquant AVANT le combat (pour detecter une evolution apres).
    let user_id = component.user.id.to_string();
    let before_level = fetch_pet(&tama, &guild_id, &user_id).await.map(|p| p.level);

    // Cout/cooldown/poids/ELO/XP/alea calcules server-side.
    let args = CombatArgs {
        guild_id: guild_id.clone(),
        attacker_id: component.user.id.to_string(),
        attacker_name: component.user.name.clone(),
        target_id: target.to_string(),
    };

    match tama.combat(args).await {
        Ok(r) => {
            let issue = if r.attacker_won {
                "🏆 **Victoire !**"
            } else {
                "💀 **Défaite...**"
            };
            let sign = if r.attacker_elo_delta >= 0 { "+" } else { "" };
            reply_ephemeral(
                ctx,
                component,
                &format!(
                    "{issue} contre **{}**\nPuissance : {} vs {}\nELO : {} ({sign}{})",
                    r.defender_name,
                    r.attacker_power,
                    r.defender_power,
                    r.attacker_new_elo,
                    r.attacker_elo_delta
                ),
            )
            .await;

            // Le combat (gagne OU perdu) donne de l'XP -> possible evolution.
            if let (Some(old), Some(p2)) =
                (before_level, fetch_pet(&tama, &guild_id, &user_id).await)
            {
                if super::card_render::stage_from_level(old)
                    != super::card_render::stage_from_level(p2.level)
                {
                    let _ = component
                        .channel_id
                        .send_message(
                            &ctx.http,
                            CreateMessage::new().content(format!(
                                "🎉 **{}** a évolué en **{}** !",
                                p2.name,
                                super::card_render::stage_label(p2.level),
                            )),
                        )
                        .await;
                }
            }
        }
        Err(e) => {
            warn!(error = %e, "Echec combat");
            reply_ephemeral(ctx, component, &format!("\u{26a0}\u{fe0f} {e}")).await;
        }
    }
}

pub async fn handle_history(ctx: &Context, component: &ComponentInteraction) {
    if !ensure_owner(ctx, component).await {
        return;
    }
    let guild_id = component
        .guild_id
        .map(|g| g.to_string())
        .unwrap_or_default();
    let tama = match get_tama(ctx).await {
        Some(t) => t,
        None => return,
    };
    let pet = fetch_pet(&tama, &guild_id, &component.user.id.to_string()).await;
    let text = match pet {
        Some(p) if !p.events.is_empty() => p
            .events
            .iter()
            .map(|e| format!("• {e}"))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => "Aucune action recente.".to_string(),
    };
    reply_ephemeral(ctx, component, &text).await;
}

pub async fn handle_close(ctx: &Context, component: &ComponentInteraction) {
    if !ensure_owner(ctx, component).await {
        return;
    }
    // Supprime le salon prive.
    if let Err(e) = component.channel_id.delete(&ctx.http).await {
        warn!(error = %e, "Echec suppression salon tamagotchi");
        reply_ephemeral(ctx, component, "Impossible de fermer le salon.").await;
    }
}

// ── Rendu ──

fn bar(value: i32) -> String {
    let v = value.clamp(0, 100);
    let filled = (v / 10) as usize;
    let empty = 10 - filled;
    format!("{}{} {}", "█".repeat(filled), "░".repeat(empty), v)
}

fn species_display(key: &str) -> &'static str {
    SPECIES
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, d)| *d)
        .unwrap_or("Compagnon")
}

pub(super) fn card_embed(p: &PetDto) -> CreateEmbed {
    let status = match p.status.as_str() {
        "sick" => "🤒 Malade",
        "dead" => "🪦 Mort",
        _ => "🟢 En forme",
    };
    let mut e = CreateEmbed::new()
        .title(format!("{}  ·  Niv. {}", p.name, p.level))
        .description(format!("{} · {}", species_display(&p.species), status))
        .color(0x9b59b6)
        .field("XP", format!("{}/{}", p.xp_in_level, p.xp_for_level), false)
        .field("🍗 Faim", bar(p.hunger), true)
        .field("😊 Bonheur", bar(p.happiness), true)
        .field("⚡ Energie", bar(p.energy), true)
        .field(
            "Combat",
            format!("FORCE {} · VITALITE {} · AGILITE {}", p.str_, p.vit, p.agi),
            false,
        )
        .field(
            "ELO",
            format!("{} ({}V/{}D)", p.elo, p.wins, p.losses),
            true,
        )
        .footer(CreateEmbedFooter::new("Tamagotchi"));
    if let Some(last) = p.events.first() {
        e = e.field("Derniere action", last, false);
    }
    e
}

pub(super) fn care_buttons() -> Vec<CreateActionRow> {
    vec![
        CreateActionRow::Buttons(vec![
            CreateButton::new(format!("{ACT_PREFIX}feed"))
                .label("Nourrir")
                .emoji('🍗')
                .style(ButtonStyle::Primary),
            CreateButton::new(format!("{ACT_PREFIX}play"))
                .label("Jouer")
                .emoji('🎲')
                .style(ButtonStyle::Primary),
            CreateButton::new(format!("{ACT_PREFIX}sleep"))
                .label("Dormir")
                .emoji('💤')
                .style(ButtonStyle::Secondary),
            CreateButton::new(format!("{ACT_PREFIX}cuddle"))
                .label("Caliner")
                .emoji('🤗')
                .style(ButtonStyle::Secondary),
        ]),
        CreateActionRow::Buttons(vec![
            CreateButton::new(format!("{TRAIN_PREFIX}str"))
                .label("Force")
                .emoji('💪')
                .style(ButtonStyle::Secondary),
            CreateButton::new(format!("{TRAIN_PREFIX}vit"))
                .label("Vitalite")
                .emoji('🛡')
                .style(ButtonStyle::Secondary),
            CreateButton::new(format!("{TRAIN_PREFIX}agi"))
                .label("Agilite")
                .emoji('🏃')
                .style(ButtonStyle::Secondary),
            CreateButton::new(SHOP_OPEN_ID)
                .label("Boutique")
                .emoji('🛒')
                .style(ButtonStyle::Primary),
        ]),
        CreateActionRow::Buttons(vec![
            CreateButton::new(VISIT_OPEN_ID)
                .label("Visiter")
                .emoji('👋')
                .style(ButtonStyle::Primary),
            CreateButton::new(COMBAT_OPEN_ID)
                .label("Combat")
                .emoji('⚔')
                .style(ButtonStyle::Danger),
            CreateButton::new(HIST_ID)
                .label("Historique")
                .style(ButtonStyle::Secondary),
            CreateButton::new(CLOSE_ID)
                .label("Fermer salon")
                .style(ButtonStyle::Danger),
        ]),
    ]
}

/// Couleur d'accent (hex sans #) par espece, pour le placeholder avatar.
fn species_color(key: &str) -> &'static str {
    match key {
        "sanglier" => "8a5a3c",
        "renard" => "e07b39",
        "tortue" => "3c8a5a",
        "loup" => "5a6a8a",
        "lapin" => "c9a0c9",
        "ours" => "6b4a2f",
        _ => "5865f2",
    }
}

fn age_days(born_at_rfc3339: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(born_at_rfc3339)
        .map(|d| {
            (chrono::Utc::now() - d.with_timezone(&chrono::Utc))
                .num_days()
                .max(0)
        })
        .unwrap_or(0)
}

async fn fetch_coins(api: &BaseApiClient, guild_id: &str, owner_id: &str) -> i64 {
    #[derive(serde::Deserialize)]
    struct W {
        coins: i64,
    }
    api.get_json::<W>(&format!("/api/wallet/{guild_id}/{owner_id}"))
        .await
        .map(|w| w.coins)
        .unwrap_or(0)
}

/// Construit les donnees + rend le PNG de la carte (None si rendu echoue).
pub(super) async fn render_card(
    api: &BaseApiClient,
    guild_id: &str,
    owner_id: &str,
    p: &PetDto,
) -> Option<Vec<u8>> {
    // Seuils visuels du sprite (fatigue / mecontentement), reglables par
    // serveur (`tamagotchi-bot`). Clamp [0, 100] a la lecture.
    let sprite_cfg = api
        .get_guild_config_for(guild_id, MODULE_BOT_NAME)
        .await
        .unwrap_or_default();
    let sprite_tired_energy_threshold =
        BaseApiClient::config_u64(&sprite_cfg, "sprite_tired_energy_threshold", 25).min(100) as i32;
    let sprite_unhappy_stat_threshold =
        BaseApiClient::config_u64(&sprite_cfg, "sprite_unhappy_stat_threshold", 25).min(100) as i32;

    let data = CardData {
        name: p.name.clone(),
        species_label: species_display(&p.species).to_string(),
        specialization: None,
        age_days: age_days(&p.born_at),
        level: p.level,
        xp_in_level: p.xp_in_level,
        xp_for_level: p.xp_for_level,
        hunger: p.hunger,
        happiness: p.happiness,
        energy: p.energy,
        str_: p.str_,
        vit: p.vit,
        agi: p.agi,
        elo: p.elo,
        wins: p.wins,
        losses: p.losses,
        coins: fetch_coins(api, guild_id, owner_id).await,
        status: p.status.clone(),
        species_color: species_color(&p.species).to_string(),
        species_slug: p.species.clone(),
        sprite_tired_energy_threshold,
        sprite_unhappy_stat_threshold,
    };
    render_card_png(&data)
}

async fn card_message(
    api: &BaseApiClient,
    guild_id: &str,
    owner_id: &str,
    p: &PetDto,
) -> CreateMessage {
    match render_card(api, guild_id, owner_id, p).await {
        Some(png) => CreateMessage::new()
            .embed(
                CreateEmbed::new()
                    .image("attachment://card.png")
                    .color(0x232838),
            )
            .add_file(CreateAttachment::bytes(png, "card.png"))
            .components(care_buttons()),
        None => CreateMessage::new()
            .embed(card_embed(p))
            .components(care_buttons()),
    }
}

async fn update_from_card(
    api: &BaseApiClient,
    guild_id: &str,
    owner_id: &str,
    p: &PetDto,
) -> CreateInteractionResponseMessage {
    match render_card(api, guild_id, owner_id, p).await {
        Some(png) => CreateInteractionResponseMessage::new()
            .embed(
                CreateEmbed::new()
                    .image("attachment://card.png")
                    .color(0x232838),
            )
            .add_file(CreateAttachment::bytes(png, "card.png"))
            .components(care_buttons()),
        None => CreateInteractionResponseMessage::new()
            .embed(card_embed(p))
            .components(care_buttons()),
    }
}

fn species_choice_message() -> CreateMessage {
    let embed = CreateEmbed::new()
        .title("🥚 Un œuf ! Choisis ton compagnon")
        .description("Chaque espece a ses affinites de combat. Choisis bien !")
        .color(0xf1c40f);
    let buttons: Vec<CreateButton> = SPECIES
        .iter()
        .map(|(k, d)| {
            CreateButton::new(format!("{PICK_PREFIX}{k}"))
                .label(*d)
                .style(ButtonStyle::Success)
        })
        .collect();
    // 6 boutons -> 2 rangees de 3.
    let (r1, r2) = buttons.split_at(3);
    CreateMessage::new().embed(embed).components(vec![
        CreateActionRow::Buttons(r1.to_vec()),
        CreateActionRow::Buttons(r2.to_vec()),
    ])
}

async fn reply_ephemeral(ctx: &Context, component: &ComponentInteraction, text: &str) {
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(text)
                    .ephemeral(true),
            ),
        )
        .await;
}

/// Enregistre cote API (gRPC) la position de la carte du joueur, pour le
/// rafraichissement automatique.
async fn persist_card_location(
    tama: &TamaApi,
    guild_id: &str,
    owner_id: &str,
    channel_id: u64,
    message_id: u64,
) {
    tama.set_card_location(guild_id, owner_id, channel_id, message_id)
        .await;
}

async fn fetch_pet(tama: &TamaApi, guild_id: &str, owner_id: &str) -> Option<PetDto> {
    tama.get_pet(guild_id, owner_id).await
}
