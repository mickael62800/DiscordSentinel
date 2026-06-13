//! Salon prive du compagnon : choix d'espece, carte, actions de soin.

use serenity::all::{
    ButtonStyle, ChannelId, ChannelType, ComponentInteraction, Context, CreateActionRow,
    CreateButton, CreateChannel, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, EditInteractionResponse, PermissionOverwrite,
    PermissionOverwriteType, Permissions, RoleId,
};
use tracing::{error, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

use super::MODULE_BOT_NAME;

pub const PICK_PREFIX: &str = "tama_pick:";
pub const ACT_PREFIX: &str = "tama_act:";
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

#[derive(serde::Deserialize)]
struct PetEventDto {
    detail: String,
}

#[derive(serde::Deserialize)]
struct PetDto {
    name: String,
    species: String,
    level: i32,
    xp_in_level: i64,
    xp_for_level: i64,
    hunger: i32,
    happiness: i32,
    energy: i32,
    status: String,
    str: i32,
    vit: i32,
    agi: i32,
    elo: i32,
    wins: i32,
    losses: i32,
    #[serde(default)]
    events: Vec<PetEventDto>,
}

async fn get_api(ctx: &Context) -> Option<std::sync::Arc<BaseApiClient>> {
    let data = ctx.data.read().await;
    data.get::<ApiClientKey>().map(std::sync::Arc::clone)
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

    // Categorie configurable.
    let cfg = api.get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME).await.unwrap_or_default();
    let category_id = cfg.get("tama_category_id").and_then(|v| v.parse::<u64>().ok());

    let everyone = RoleId::new(guild_id.get());
    let name = format!("tama-{}", component.user.name.chars().take(15).collect::<String>().to_lowercase());
    let mut builder = CreateChannel::new(&name)
        .kind(ChannelType::Text)
        .topic(format!("[tama:{}]", user_id))
        .permissions(vec![
            PermissionOverwrite { allow: Permissions::empty(), deny: Permissions::VIEW_CHANNEL, kind: PermissionOverwriteType::Role(everyone) },
            PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
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
            let _ = component.edit_response(&ctx.http, EditInteractionResponse::new().content("Erreur lors de la creation du salon.")).await;
            return;
        }
    };

    // Pet existant ?
    let pet = fetch_pet(&api, &guild_id.to_string(), &user_id.to_string()).await;
    let msg = match pet {
        Some(p) if p.status != "dead" => card_message(&p),
        _ => species_choice_message(),
    };
    let _ = channel.id.send_message(&ctx.http, msg).await;

    let _ = component
        .edit_response(&ctx.http, EditInteractionResponse::new().content(format!("Ton salon : <#{}>", channel.id)))
        .await;
}

// ── Choix d'espece (naissance) ──

pub async fn handle_pick(ctx: &Context, component: &ComponentInteraction) {
    let species = component.data.custom_id.strip_prefix(PICK_PREFIX).unwrap_or("").to_string();
    let guild_id = component.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let api = match get_api(ctx).await { Some(a) => a, None => return };

    let body = serde_json::json!({
        "guild_id": guild_id,
        "owner_id": component.user.id.to_string(),
        "name": component.user.name,
        "species": species,
    });
    match api.post_json::<_, PetDto>("/api/tamagotchi/pets", &body).await {
        Ok(pet) => {
            let _ = component
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::UpdateMessage(
                        update_from_card(&pet),
                    ),
                )
                .await;
        }
        Err(e) => {
            warn!(error = %e, "Echec creation pet");
            reply_ephemeral(ctx, component, "Impossible de creer le compagnon (en as-tu deja un ?).").await;
        }
    }
}

// ── Actions de soin ──

pub async fn handle_action(ctx: &Context, component: &ComponentInteraction) {
    let action = component.data.custom_id.strip_prefix(ACT_PREFIX).unwrap_or("").to_string();
    let guild_id = component.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let user_id = component.user.id.to_string();
    let api = match get_api(ctx).await { Some(a) => a, None => return };

    let pet = match fetch_pet(&api, &guild_id, &user_id).await {
        Some(p) => p,
        None => { reply_ephemeral(ctx, component, "Tu n'as pas de compagnon ici.").await; return; }
    };
    // On a besoin de l'id du pet -> on le relit (fetch_pet ne l'expose pas).
    let pet_id = match fetch_pet_id(&api, &guild_id, &user_id).await {
        Some(id) => id,
        None => return,
    };

    let cfg = api.get_guild_config_for(&guild_id, MODULE_BOT_NAME).await.unwrap_or_default();
    let xp = BaseApiClient::config_u64(&cfg, "xp_per_action", 5) as i64;
    let body = match action.as_str() {
        "feed" => serde_json::json!({
            "action": "feed",
            "coin_cost": BaseApiClient::config_u64(&cfg, "feed_cost", 20) as i64,
            "hunger_delta": BaseApiClient::config_u64(&cfg, "feed_hunger_gain", 40) as i32,
            "xp_gain": xp,
            "cooldown_secs": BaseApiClient::config_u64(&cfg, "feed_cooldown_secs", 1800) as i64,
        }),
        "play" => serde_json::json!({
            "action": "play",
            "happiness_delta": BaseApiClient::config_u64(&cfg, "play_happiness_gain", 30) as i32,
            "energy_delta": -(BaseApiClient::config_u64(&cfg, "play_energy_cost", 10) as i32),
            "xp_gain": xp,
            "cooldown_secs": BaseApiClient::config_u64(&cfg, "play_cooldown_secs", 1800) as i64,
        }),
        "sleep" => serde_json::json!({
            "action": "sleep",
            "energy_delta": BaseApiClient::config_u64(&cfg, "sleep_energy_gain", 60) as i32,
            "cooldown_secs": BaseApiClient::config_u64(&cfg, "sleep_cooldown_secs", 1020) as i64,
        }),
        "cuddle" => serde_json::json!({
            "action": "cuddle",
            "happiness_delta": BaseApiClient::config_u64(&cfg, "cuddle_happiness_gain", 15) as i32,
            "xp_gain": xp,
            "cooldown_secs": BaseApiClient::config_u64(&cfg, "cuddle_cooldown_secs", 3600) as i64,
        }),
        _ => { let _ = pet; return; }
    };

    match api.post_json::<_, PetDto>(&format!("/api/tamagotchi/pets/{pet_id}/care"), &body).await {
        Ok(updated) => {
            let _ = component
                .create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(update_from_card(&updated)))
                .await;
        }
        Err(e) => {
            warn!(error = %e, action, "Echec action soin");
            reply_ephemeral(ctx, component, "Action impossible (cooldown, coins insuffisants, ou compagnon mort).").await;
        }
    }
}

pub async fn handle_history(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = component.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let api = match get_api(ctx).await { Some(a) => a, None => return };
    let pet = fetch_pet(&api, &guild_id, &component.user.id.to_string()).await;
    let text = match pet {
        Some(p) if !p.events.is_empty() => {
            p.events.iter().map(|e| format!("• {}", e.detail)).collect::<Vec<_>>().join("\n")
        }
        _ => "Aucune action recente.".to_string(),
    };
    reply_ephemeral(ctx, component, &text).await;
}

pub async fn handle_close(ctx: &Context, component: &ComponentInteraction) {
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
    SPECIES.iter().find(|(k, _)| *k == key).map(|(_, d)| *d).unwrap_or("Compagnon")
}

fn card_embed(p: &PetDto) -> CreateEmbed {
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
        .field("Combat", format!("FORCE {} · VITALITE {} · AGILITE {}", p.str, p.vit, p.agi), false)
        .field("ELO", format!("{} ({}V/{}D)", p.elo, p.wins, p.losses), true)
        .footer(CreateEmbedFooter::new("Tamagotchi"));
    if let Some(last) = p.events.first() {
        e = e.field("Derniere action", &last.detail, false);
    }
    e
}

fn care_buttons() -> Vec<CreateActionRow> {
    vec![
        CreateActionRow::Buttons(vec![
            CreateButton::new(format!("{ACT_PREFIX}feed")).label("Nourrir").emoji('🍗').style(ButtonStyle::Primary),
            CreateButton::new(format!("{ACT_PREFIX}play")).label("Jouer").emoji('🎲').style(ButtonStyle::Primary),
            CreateButton::new(format!("{ACT_PREFIX}sleep")).label("Dormir").emoji('💤').style(ButtonStyle::Secondary),
            CreateButton::new(format!("{ACT_PREFIX}cuddle")).label("Caliner").emoji('🤗').style(ButtonStyle::Secondary),
        ]),
        CreateActionRow::Buttons(vec![
            CreateButton::new(HIST_ID).label("Historique").style(ButtonStyle::Secondary),
            CreateButton::new(CLOSE_ID).label("Fermer salon").style(ButtonStyle::Danger),
        ]),
    ]
}

fn card_message(p: &PetDto) -> CreateMessage {
    CreateMessage::new().embed(card_embed(p)).components(care_buttons())
}

fn update_from_card(p: &PetDto) -> CreateInteractionResponseMessage {
    CreateInteractionResponseMessage::new().embed(card_embed(p)).components(care_buttons())
}

fn species_choice_message() -> CreateMessage {
    let embed = CreateEmbed::new()
        .title("🥚 Un œuf ! Choisis ton compagnon")
        .description("Chaque espece a ses affinites de combat. Choisis bien !")
        .color(0xf1c40f);
    let buttons: Vec<CreateButton> = SPECIES
        .iter()
        .map(|(k, d)| CreateButton::new(format!("{PICK_PREFIX}{k}")).label(*d).style(ButtonStyle::Success))
        .collect();
    // 6 boutons -> 2 rangees de 3.
    let (r1, r2) = buttons.split_at(3);
    CreateMessage::new()
        .embed(embed)
        .components(vec![CreateActionRow::Buttons(r1.to_vec()), CreateActionRow::Buttons(r2.to_vec())])
}

async fn reply_ephemeral(ctx: &Context, component: &ComponentInteraction, text: &str) {
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(text).ephemeral(true),
            ),
        )
        .await;
}

async fn fetch_pet(api: &BaseApiClient, guild_id: &str, owner_id: &str) -> Option<PetDto> {
    api.get_json::<PetDto>(&format!("/api/tamagotchi/{guild_id}/{owner_id}")).await.ok()
}

#[derive(serde::Deserialize)]
struct PetIdDto { id: String }

async fn fetch_pet_id(api: &BaseApiClient, guild_id: &str, owner_id: &str) -> Option<String> {
    api.get_json::<PetIdDto>(&format!("/api/tamagotchi/{guild_id}/{owner_id}")).await.ok().map(|p| p.id)
}
