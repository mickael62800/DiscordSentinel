use rand::Rng;
use serenity::all::{
    ButtonStyle, CommandDataOptionValue, CommandInteraction, CommandOptionType,
    ComponentInteraction, Context, CreateActionRow, CreateButton, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, EditMessage,
};
use std::time::Duration;

use crate::api_client::ApiClient;
use crate::catalog::{CatalogCache, CatalogCacheKey};
use crate::GameApiKey;
use crate::handler::load_guild_config;

pub const STEAL_DEFEND_PREFIX: &str = "steal_defend:";

/// Malus applique au roll du defenseur quand il est AFK (n'a pas clique
/// sur le bouton "Se defendre !"). Represente le fait qu'il ne reagit
/// pas a l'alerte.
const AFK_DEFENDER_MALUS: i32 = 8;

/// Verifie les items anti-vol de la cible et en declenche un si possible.
///
/// Retourne `Some((item_key, item_name))` si un item a bloque le vol
/// (auquel cas il a ete consomme via `use_item`). Sinon `None`.
///
/// Note : on evite de garder un `ThreadRng` (pas `Send`) au-dela d'un
/// `await` en rollant dans un bloc scope, puis en l'oubliant avant d'appeler
/// l'API.
async fn try_trigger_protection(
    api: &ApiClient,
    catalog: &CatalogCache,
    guild_id: &str,
    target_id: &str,
) -> Option<(String, String)> {
    for item in &catalog.anti_theft_items {
        let key = item.key.clone();
        let pct = item.block_chance_percent;
        let has = api.has_item(guild_id, target_id, &key).await.unwrap_or(false);
        if !has {
            continue;
        }
        // Roll dans un bloc scope pour que le ThreadRng soit drop avant l'await.
        let triggered = {
            let mut rng = rand::thread_rng();
            let roll: u32 = rng.gen_range(1..=100);
            roll <= pct
        };
        if triggered {
            // Consomme l'item et retourne son info pour affichage.
            let _ = api.use_item(guild_id, target_id, &key).await;
            let name = catalog
                .get_item(&key)
                .map(|i| i.name.clone())
                .unwrap_or_else(|| key.clone());
            return Some((key, name));
        }
    }
    None
}

const STEAL_SUCCESS_AFK: &[&str] = &[
    "\u{1f4b0} {voleur} a fait les poches de {victime} pendant sa sieste ! (-{montant} coins)",
    "\u{1f575}\u{fe0f} {voleur} s'est glisse dans l'ombre et a chipe {montant} coins a {victime} !",
    "\u{1f3ad} {voleur} a distrait {victime} avec un tour de magie et lui a pique {montant} coins !",
    "\u{1f431} {voleur} a vole {montant} coins a {victime} avec l'agilite d'un chat !",
    "\u{1f4a4} {victime} dormait sur son tresor... {voleur} en a profite pour prendre {montant} coins !",
];

const STEAL_SUCCESS_FIGHT: &[&str] = &[
    "\u{1f4aa} {victime} s'est debattu, mais {voleur} est plus malin ! {montant} coins voles !",
    "\u{1f93c} Apres une lutte acharnee, {voleur} repart avec {montant} coins de {victime} !",
    "\u{1f3c3} {voleur} a arrache le sac de {victime} et s'est enfui en courant ! {montant} coins !",
];

const STEAL_FAIL: &[&str] = &[
    "\u{1f6a8} {victime} a attrape {voleur} la main dans le sac ! {voleur} perd {montant} coins !",
    "\u{1f44a} {victime} a mis une gifle a {voleur} en pleine tentative ! -{montant} coins !",
    "\u{1f34c} {voleur} a glisse sur une peau de banane en essayant de voler {victime} ! -{montant} coins !",
    "\u{1f415} Le chien de {victime} a mordu {voleur} ! Vol rate et {montant} coins en frais medicaux !",
    "\u{1fab4} {victime} avait pose un piege ! {voleur} se retrouve suspendu par les pieds ! -{montant} coins !",
    "\u{1f921} {voleur} a essaye de pickpocket {victime} mais a sorti son propre portefeuille ! -{montant} coins !",
];

fn format_msg(template: &str, voleur: &str, victime: &str, montant: i64) -> String {
    template
        .replace("{voleur}", voleur)
        .replace("{victime}", victime)
        .replace("{montant}", &montant.to_string())
}

fn pick_random<'a>(messages: &[&'a str]) -> &'a str {
    let mut rng = rand::thread_rng();
    let idx = rng.gen_range(0..messages.len());
    messages[idx]
}

pub fn register() -> CreateCommand {
    CreateCommand::new("voler")
        .description("Tente de pickpocket un joueur !")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "cible", "Le joueur a voler")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let target_id = command
        .data
        .options
        .iter()
        .find(|o| o.name == "cible")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        })
        .unwrap();

    let thief_id = command.user.id.to_string();
    let target_id_str = target_id.to_string();

    if thief_id == target_id_str {
        reply_ephemeral(ctx, command, "Tu ne peux pas te voler toi-meme !").await;
        return;
    }

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::channel_check::check_channel(ctx, command, config.channel_activites()).await {
        return;
    }
    if !config.steal_enabled() {
        reply_ephemeral(ctx, command, "Le vol est desactive sur ce serveur.").await;
        return;
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    // Verifier la limite quotidienne de vols
    let max_daily = config.steal_max_daily();
    if max_daily > 0 {
        let today_count = api.count_steal_today(&guild_id, &thief_id).await.unwrap_or(0);
        if today_count >= max_daily {
            reply_ephemeral(
                ctx,
                command,
                &format!("Tu as atteint la limite de {} vols par jour !", max_daily),
            )
            .await;
            return;
        }
    }

    // Verifier le cooldown (30 min)
    match api.check_cooldown(&guild_id, &thief_id, "voler").await {
        Ok(Some(expires_at_str)) => {
            let expires = chrono::DateTime::parse_from_rfc3339(&expires_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            let remaining = expires
                .signed_duration_since(chrono::Utc::now())
                .num_seconds();
            if remaining > 0 {
                let mins = remaining / 60;
                let secs = remaining % 60;
                reply_ephemeral(
                    ctx,
                    command,
                    &format!(
                        "Tu dois attendre encore {}m{}s avant de pouvoir voler quelqu'un !",
                        mins, secs
                    ),
                )
                .await;
                return;
            }
        }
        Ok(None) => {}
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    }

    // Creer/recuperer les joueurs
    let _thief_player = match api
        .get_or_create_player(&guild_id, &thief_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    let target_user = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            reply_ephemeral(ctx, command, "Utilisateur introuvable.").await;
            return;
        }
    };

    if target_user.bot {
        reply_ephemeral(ctx, command, "Tu ne peux pas voler un bot !").await;
        return;
    }

    let target_player = match api
        .get_or_create_player(&guild_id, &target_id_str, &target_user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    if target_player.coins < 10 {
        reply_ephemeral(
            ctx,
            command,
            &format!(
                "<@{}> n'a que {} coins... Meme les voleurs ont des principes !",
                target_id, target_player.coins
            ),
        )
        .await;
        return;
    }

    // Poser le cooldown (30 min = 1800s)
    if let Err(e) = api
        .set_cooldown(&guild_id, &thief_id, "voler", config.steal_cooldown_secs())
        .await
    {
        reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
        return;
    }

    // Envoyer l'alerte publique (voleur anonyme) avec bouton de defense
    let custom_id = format!(
        "steal_defend:{}:{}:{}",
        thief_id, target_id_str, guild_id
    );

    let embed = CreateEmbed::new()
        .title("\u{26a0}\u{fe0f} Tentative de vol !")
        .description(format!(
            "\u{26a0}\u{fe0f} Quelqu'un tente de voler <@{}> !\n\n\
             <@{}>, tu as **60 secondes** pour te defendre !",
            target_id, target_id
        ))
        .color(0xFFA500)
        .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel — 60s pour reagir"))
        .timestamp(serenity::model::Timestamp::now());

    let defend_btn = CreateButton::new(&custom_id)
        .label("Se defendre !")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{1f6e1}\u{fe0f}".to_string(),
        ))
        .style(ButtonStyle::Primary);

    let row = CreateActionRow::Buttons(vec![defend_btn]);

    // Repondre en ephemere au voleur pour confirmer
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("\u{1f575}\u{fe0f} Tentative de vol lancee... Reste discret !")
                    .ephemeral(true),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord");
        return;
    }

    // Poster le message public avec le bouton dans le salon d'activites
    let activity_channel = config.channel_activites();
    let channel_id = match activity_channel.and_then(|id| id.parse::<u64>().ok()) {
        Some(ch_id) => serenity::model::id::ChannelId::new(ch_id),
        None => command.channel_id,
    };

    let alert_msg = match channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(embed).components(vec![row]),
        )
        .await
    {
        Ok(msg) => msg,
        Err(e) => {
            tracing::warn!(error = %e, "Echec send_message alerte vol");
            return;
        }
    };

    // Spawn timeout task: after 60s, if no defend, auto-succeed
    let ctx_clone = ctx.clone();
    let msg_id = alert_msg.id;
    let msg_channel_id = channel_id;
    let thief_id_clone = thief_id.clone();
    let target_id_clone = target_id_str.clone();
    let guild_id_clone = guild_id.clone();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(60)).await;

        // Try to edit the message — if button was already clicked, the custom_id
        // won't match anymore (component removed), so we check if components still exist
        let msg = match msg_channel_id.message(&ctx_clone.http, msg_id).await {
            Ok(m) => m,
            Err(_) => return,
        };

        // If components were removed, the defend button was clicked — do nothing
        if msg.components.is_empty() {
            return;
        }

        // Timeout : la victime est AFK. On simule un combat de vol comme
        // si elle se defendait, mais avec un malus sur son roll. Les items
        // anti-vol peuvent toujours declencher.
        let data = ctx_clone.data.read().await;
        let api = data.get::<GameApiKey>().unwrap();
        let catalog_timeout = data.get::<CatalogCacheKey>().unwrap().clone();

        let thief_player = match api
            .get_or_create_player(&guild_id_clone, &thief_id_clone, "")
            .await
        {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(error = %e, "Echec API get_or_create_player thief (timeout vol)");
                return;
            }
        };

        let target_player = match api
            .get_or_create_player(&guild_id_clone, &target_id_clone, "")
            .await
        {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(error = %e, "Echec API get_or_create_player target (timeout vol)");
                return;
            }
        };

        let result_embed = resolve_steal_attempt(
            api,
            &catalog_timeout,
            &guild_id_clone,
            &thief_id_clone,
            &target_id_clone,
            &thief_player,
            &target_player,
            true, // AFK
        )
        .await;

        // Edit the original alert message to show result (remove button)
        if let Err(e) = msg_channel_id
            .edit_message(
                &ctx_clone.http,
                msg_id,
                EditMessage::new()
                    .embed(result_embed)
                    .components(vec![]),
            )
            .await
        {
            tracing::warn!(error = %e, "Echec edit_message (timeout vol)");
        }
    });
}

/// Resout une tentative de vol. Centralise la logique pour les deux
/// chemins (clic "Se defendre" ou timeout AFK).
///
/// - `afk = true` → malus de `AFK_DEFENDER_MALUS` sur le roll du
///   defenseur, plage de coins voles 10-15% (moins que le defendu actif).
/// - `afk = false` → roll normal, plage 15-25%.
///
/// Dans les deux cas, les items anti-vol de la cible peuvent bloquer
/// le vol apres le roll (un item tire sa chance et se consume au blocage).
async fn resolve_steal_attempt(
    api: &ApiClient,
    catalog: &CatalogCache,
    guild_id: &str,
    thief_id: &str,
    target_id: &str,
    thief_player: &crate::api_client::Player,
    target_player: &crate::api_client::Player,
    afk: bool,
) -> CreateEmbed {
    use rand::Rng;

    // Roll d20 + bonus
    let (thief_roll, target_roll): (i32, i32) = {
        let mut rng = rand::thread_rng();
        (rng.gen_range(1..=20), rng.gen_range(1..=20))
    };
    let thief_bonus = if thief_player.class.as_deref() == Some("fourbe") {
        4
    } else {
        0
    };
    let mut target_bonus = target_player.def / 10;
    if afk {
        target_bonus -= AFK_DEFENDER_MALUS;
    }
    let thief_total = thief_roll + thief_bonus;
    let target_total = target_roll + target_bonus;

    let roll_detail = format!(
        "\n\n\u{1f3b2} Voleur: {} (d20: {} + bonus: {}) vs Victime: {} (d20: {} + DEF bonus: {}{})",
        thief_total,
        thief_roll,
        thief_bonus,
        target_total,
        target_roll,
        target_bonus + if afk { AFK_DEFENDER_MALUS } else { 0 },
        if afk {
            format!(" - AFK: {}", AFK_DEFENDER_MALUS)
        } else {
            String::new()
        },
    );

    if thief_total > target_total {
        // Le voleur a gagne le roll — mais un item anti-vol peut encore
        // bloquer le vol (et se consommer).
        if let Some((_key, name)) = try_trigger_protection(api, catalog, guild_id, target_id).await {
            let block_msg = format!(
                "\u{1f6e1}\u{fe0f} <@{}> avait un **{}** qui a bloque la tentative de vol de <@{}> !\n\
                 L'item est consomme.",
                target_id, name, thief_id
            );
            // La victime gagne +3 XP comme pour une defense reussie.
            let mut xp_line = String::new();
            if let Ok((_new_xp, new_level, leveled_up, stat_points)) =
                api.add_xp(guild_id, target_id, 3).await
            {
                xp_line.push_str(&format!("\n\u{2b06}\u{fe0f} +3 XP pour <@{}>", target_id));
                if leveled_up {
                    let title = catalog.title_for_level(new_level).to_string();
                    xp_line.push_str(&format!(
                        "\n\u{1f31f} **LEVEL UP !** Niveau **{}** \u{300c}{}\u{300d} ! (+{} points de stats)",
                        new_level, title, stat_points
                    ));
                }
            }
            return CreateEmbed::new()
                .title("\u{1f6e1}\u{fe0f} Vol bloque !")
                .description(format!("{}{}{}", block_msg, roll_detail, xp_line))
                .color(0x3498DB)
                .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
                .timestamp(serenity::model::Timestamp::now());
        }

        // Pas de protection : le vol reussit.
        // AFK : 10-15% | defendu : 15-25%.
        let steal_pct: f64 = {
            let mut rng = rand::thread_rng();
            if afk {
                rng.gen_range(10.0..=15.0) / 100.0
            } else {
                rng.gen_range(15.0..=25.0) / 100.0
            }
        };
        let stolen = ((target_player.coins as f64 * steal_pct) as i64).max(1);

        if let Err(e) = api
            .record_steal(guild_id, thief_id, target_id, stolen)
            .await
        {
            tracing::warn!(error = %e, "Echec API record_steal");
        }

        let mut xp_line = String::new();
        if let Ok((_new_xp, new_level, leveled_up, stat_points)) =
            api.add_xp(guild_id, thief_id, 5).await
        {
            xp_line.push_str("\n\u{2b06}\u{fe0f} +5 XP pour le voleur");
            if leveled_up {
                let title = catalog.title_for_level(new_level).to_string();
                xp_line.push_str(&format!(
                    "\n\u{1f31f} **LEVEL UP !** Niveau **{}** \u{300c}{}\u{300d} ! (+{} points de stats)",
                    new_level, title, stat_points
                ));
            }
        }

        let template = if afk { STEAL_SUCCESS_AFK } else { STEAL_SUCCESS_FIGHT };
        let msg_text = format_msg(
            pick_random(template),
            &format!("<@{}>", thief_id),
            &format!("<@{}>", target_id),
            stolen,
        );

        CreateEmbed::new()
            .title("\u{1f4b0} Vol reussi !")
            .description(format!("{}{}{}", msg_text, roll_detail, xp_line))
            .color(0x57F287)
            .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
            .timestamp(serenity::model::Timestamp::now())
    } else {
        // Vol echoue : le voleur perd 15% de ses coins, victime +3 XP.
        let lost = ((thief_player.coins as f64 * 0.15) as i64).max(1);

        if let Err(e) = api.record_coins_lost(guild_id, thief_id, lost).await {
            tracing::warn!(error = %e, "Echec API record_coins_lost vol");
        }

        let mut xp_line = String::new();
        if let Ok((_new_xp, new_level, leveled_up, stat_points)) =
            api.add_xp(guild_id, target_id, 3).await
        {
            xp_line.push_str(&format!("\n\u{2b06}\u{fe0f} +3 XP pour <@{}>", target_id));
            if leveled_up {
                let title = catalog.title_for_level(new_level).to_string();
                xp_line.push_str(&format!(
                    "\n\u{1f31f} **LEVEL UP !** Niveau **{}** \u{300c}{}\u{300d} ! (+{} points de stats)",
                    new_level, title, stat_points
                ));
            }
        }

        let msg_text = format_msg(
            pick_random(STEAL_FAIL),
            &format!("<@{}>", thief_id),
            &format!("<@{}>", target_id),
            lost,
        );

        CreateEmbed::new()
            .title("\u{1f6a8} Vol rate !")
            .description(format!("{}{}{}", msg_text, roll_detail, xp_line))
            .color(0xED4245)
            .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
            .timestamp(serenity::model::Timestamp::now())
    }
}

/// Handle the defend button click from a steal attempt.
pub async fn handle_defend(ctx: &Context, component: &ComponentInteraction) {
    let parts: Vec<&str> = component.data.custom_id.split(':').collect();
    // Format: steal_defend:{thief_id}:{target_id}:{guild_id}
    if parts.len() != 4 {
        return;
    }

    let thief_id = parts[1];
    let target_id = parts[2];
    let guild_id = parts[3];

    // Only the target can click the defend button
    if component.user.id.to_string() != target_id {
        if let Err(e) = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Seule la victime peut se defendre !")
                        .ephemeral(true),
                ),
            )
            .await
        {
            tracing::warn!(error = %e, "Echec response Discord");
        }
        return;
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();
    let catalog_defend = data.get::<CatalogCacheKey>().unwrap().clone();

    // Fetch both players
    let thief_player = match api
        .get_or_create_player(guild_id, thief_id, "")
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_component_ephemeral(ctx, component, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    let target_player = match api
        .get_or_create_player(guild_id, target_id, &component.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_component_ephemeral(ctx, component, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    let embed = resolve_steal_attempt(
        api,
        &catalog_defend,
        guild_id,
        thief_id,
        target_id,
        &thief_player,
        &target_player,
        false, // defense active
    )
    .await;

    // Acknowledge the interaction by updating the original message
    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(vec![]),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord (defend vol)");
    }
}

async fn reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord");
    }
}

async fn reply_component_ephemeral(
    ctx: &Context,
    component: &ComponentInteraction,
    content: &str,
) {
    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord");
    }
}
