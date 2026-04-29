use serenity::all::{
    ButtonStyle, CommandDataOptionValue, CommandInteraction, CommandOptionType,
    ComponentInteraction, Context, CreateActionRow, CreateButton, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, EditMessage,
};
use std::time::Duration;

use sentinel_shared::discord_helpers::{reply_ephemeral, require_guild_id};

use crate::modules::coude::api_client::ApiClient;
use crate::modules::coude::catalog::{CatalogCache, CatalogCacheKey};
use crate::modules::coude::GameApiKey;
use crate::modules::coude::load_guild_config;

pub const STEAL_DEFEND_PREFIX: &str = "steal_defend:";

/// Malus applique au roll du defenseur quand il est AFK (n'a pas clique
/// sur le bouton "Se defendre !"). Represente le fait qu'il ne reagit
/// pas a l'alerte.
const AFK_DEFENDER_MALUS: i32 = 8;

/// Verifie si la cible a une protection anti-vol active (Phase 9 Part B).
///
/// Depuis le refactor en abonnements temps-base, ce check est delegue a
/// l'API : elle liste les protections actives, roll les dés, et retourne
/// celle qui a bloque (ou None). Le bot ne fait qu'afficher le resultat.
/// Les items NE SONT PLUS consommes — c'est un abonnement.
async fn try_trigger_protection(
    api: &ApiClient,
    guild_id: &str,
    target_id: &str,
) -> Option<(String, String, u32, u32)> {
    match api.try_trigger_steal_protection(guild_id, target_id).await {
        Ok(Some(trigger)) => Some((
            trigger.item_key,
            trigger.item_name,
            trigger.rolled_value,
            trigger.block_chance_percent,
        )),
        Ok(None) => None,
        Err(e) => {
            tracing::warn!(error = %e, "Echec try_trigger_steal_protection API");
            None
        }
    }
}

// Templates STEAL_SUCCESS_AFK / STEAL_SUCCESS_FIGHT / STEAL_FAIL migres
// dans `coude_flavor_templates` (Phase 3 #9 audit). Le bot consomme via
// `api.random_flavor`. Pas de fallback local.

fn format_msg(template: &str, voleur: &str, victime: &str, montant: i64) -> String {
    template
        .replace("{voleur}", voleur)
        .replace("{victime}", victime)
        .replace("{montant}", &montant.to_string())
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
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };

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
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_activites()).await {
        return;
    }
    if !config.steal_enabled() {
        reply_ephemeral(ctx, command, "Le vol est desactive sur ce serveur.").await;
        return;
    }

    // Defer ephemeral : /voler enchaine 5 appels API (count_steal, cooldown,
    // get player x2, set_cooldown) avant de repondre. Sans defer, Discord
    // coupait l'interaction a 3s.
    if !crate::modules::coude::interaction_helper::defer_ephemeral(ctx, command).await {
        return;
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    // Verifier la limite quotidienne de vols
    let max_daily = config.steal_max_daily();
    if max_daily > 0 {
        let today_count = api.count_steal_today(&guild_id, &thief_id).await.unwrap_or(0);
        if today_count >= max_daily {
            crate::modules::coude::interaction_helper::followup_text(
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
                crate::modules::coude::interaction_helper::followup_text(
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
            crate::modules::coude::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
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
            crate::modules::coude::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    let target_user = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            crate::modules::coude::interaction_helper::followup_text(ctx, command, "Utilisateur introuvable.").await;
            return;
        }
    };

    if target_user.bot {
        crate::modules::coude::interaction_helper::followup_text(ctx, command, "Tu ne peux pas voler un bot !").await;
        return;
    }

    let target_player = match api
        .get_or_create_player(&guild_id, &target_id_str, &target_user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            crate::modules::coude::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    let min_target_coins = config.voler_min_target_coins();
    if target_player.coins < min_target_coins {
        crate::modules::coude::interaction_helper::followup_text(
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
        crate::modules::coude::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
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

    // Confirme au voleur via followup ephemeral (on a defer plus haut).
    crate::modules::coude::interaction_helper::followup_text(
        ctx,
        command,
        "\u{1f575}\u{fe0f} Tentative de vol lancee... Reste discret !",
    )
    .await;

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
    let failure_penalty_pct = config.steal_failure_penalty_pct();

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

        let (result_embed, taunt_events) = resolve_steal_attempt(
            api,
            &catalog_timeout,
            &guild_id_clone,
            &thief_id_clone,
            &target_id_clone,
            &thief_player,
            &target_player,
            true, // AFK
            failure_penalty_pct,
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

        // Phase 9 Part D : dispatch les taunt events (IO pur).
        if !taunt_events.is_empty() {
            if let Ok(guild_id_u64) = guild_id_clone.parse::<u64>() {
                let gid = serenity::all::GuildId::new(guild_id_u64);
                crate::modules::coude::taunts_dispatch::dispatch_all(&ctx_clone, gid, &taunt_events).await;
            }
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
    thief_player: &crate::modules::coude::api_client::Player,
    target_player: &crate::modules::coude::api_client::Player,
    afk: bool,
    failure_penalty_pct: u64,
) -> (CreateEmbed, Vec<crate::modules::coude::api_client::TauntEvent>) {
    // Phase 2 #4 audit : tirage RNG (d20 thief/victim + % wallet) cote API.
    // Pas de fallback local : si l'API est down on retourne une erreur.
    let api_roll = match api.roll_steal(guild_id, afk).await {
        Ok(r) => r,
        Err(_) => {
            let embed = CreateEmbed::new()
                .title("\u{26a0}\u{fe0f} API indisponible")
                .description("Veuillez reessayer plus tard.")
                .color(0x95A5A6);
            return (embed, Vec::new());
        }
    };
    let (thief_roll, target_roll): (i32, i32) = (api_roll.thief_d20, api_roll.victim_d20);
    let class_bonus = if thief_player.class.as_deref() == Some("fourbe") {
        4
    } else {
        0
    };
    // Phase 9 Part C : somme des items de boost voleur actifs.
    let boost_bonus = api
        .get_steal_boost_total(guild_id, thief_id)
        .await
        .unwrap_or(0);
    let thief_bonus = class_bonus + boost_bonus;

    let mut target_bonus = target_player.def / 10;
    if afk {
        target_bonus -= AFK_DEFENDER_MALUS;
    }
    let thief_total = thief_roll + thief_bonus;
    let target_total = target_roll + target_bonus;

    // Detail du roll : on explicite le boost_bonus separement seulement
    // s'il est non nul, pour ne pas leaker quand le voleur n'a rien
    // depense (afficher toujours "+boost: 0" donnerait l'info aux
    // curieux).
    let thief_detail = if boost_bonus > 0 {
        format!(
            "d20: {} + class: {} + boost: {}",
            thief_roll, class_bonus, boost_bonus
        )
    } else {
        format!("d20: {} + bonus: {}", thief_roll, class_bonus)
    };
    let roll_detail = format!(
        "\n\n\u{1f3b2} Voleur: {} ({}) vs Victime: {} (d20: {} + DEF bonus: {}{})",
        thief_total,
        thief_detail,
        target_total,
        target_roll,
        target_bonus + if afk { AFK_DEFENDER_MALUS } else { 0 },
        if afk {
            format!(" - AFK: {}", AFK_DEFENDER_MALUS)
        } else {
            String::new()
        },
    );

    let mut taunt_events: Vec<crate::modules::coude::api_client::TauntEvent> = Vec::new();

    if thief_total > target_total {
        // Le voleur a gagne le roll — mais une protection active peut
        // encore bloquer le vol (Phase 9 Part B : abonnements temps-base,
        // plus de consommation d'item).
        if let Some((_key, name, rolled, chance)) = try_trigger_protection(api, guild_id, target_id).await {
            // Phase 9 Part D : blocage reussi → reset le victim streak.
            if let Err(e) = api.track_steal_defended(guild_id, target_id).await {
                tracing::warn!(error = %e, "Echec track_steal_defended");
            }

            // Ligne explicite : le voleur AVAIT gagne le roll des des, mais
            // la protection a bloque grace au tirage % (rolled <= chance).
            let protection_detail = format!(
                "\n\u{1f3b2} Le voleur avait gagne le combat ({} > {}), mais la protection a fait un jet de **{}/100** (seuil **{}%**) → \u{2705} bloque !",
                thief_total, target_total, rolled, chance
            );

            let block_msg = format!(
                "\u{1f6e1}\u{fe0f} <@{}> etait protege par **{}** qui a bloque la tentative de vol de <@{}> !",
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
            let embed = CreateEmbed::new()
                .title("\u{1f6e1}\u{fe0f} Vol bloque !")
                .description(format!("{}{}{}{}", block_msg, roll_detail, protection_detail, xp_line))
                .color(0x3498DB)
                .footer(CreateEmbedFooter::new(sentinel_shared::branding::COUDE_TAGLINE_SHORT))
                .timestamp(serenity::model::Timestamp::now());
            return (embed, taunt_events);
        }

        // Pas de protection : le vol reussit. % vole tire cote API plus haut.
        let steal_pct: f64 = (api_roll.steal_pct_bp as f64) / 10_000.0;
        let stolen = ((target_player.coins as f64 * steal_pct) as i64).max(1);

        // Migration wallet unifie : record_steal delegue a
        // ManageWalletUseCase::transfer cote API (faillite victime +
        // jackpot voleur auto-detectes). Retourne le montant
        // effectivement vole (clamp serveur) + TauntEvents a
        // dispatcher.
        let stolen = match api
            .record_steal(guild_id, thief_id, target_id, stolen)
            .await
        {
            Ok((actual_stolen, wallet_taunts)) => {
                taunt_events.extend(wallet_taunts);
                actual_stolen
            }
            Err(e) => {
                tracing::warn!(error = %e, "Echec API record_steal");
                stolen
            }
        };

        // Phase 9 Part D : incremente le victim streak + collecte taunt event.
        // Reste separe du record_steal car depend du nombre de vols
        // subis et non du montant.
        match api.track_steal_victim(guild_id, target_id).await {
            Ok(Some(ev)) => taunt_events.push(ev),
            Ok(None) => {}
            Err(e) => tracing::warn!(error = %e, "Echec track_steal_victim"),
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

        // Phase 3 #9 audit : tirage du template via l'API (catalogue
        // editable runtime). Pas de fallback local.
        let key = if afk { "steal_success_afk" } else { "steal_success_fight" };
        let template_str: String = match api.random_flavor(key, "fr").await {
            Ok(Some(s)) => s,
            Ok(None) | Err(_) => {
                let embed = CreateEmbed::new()
                    .title("\u{26a0}\u{fe0f} API indisponible")
                    .description("Veuillez reessayer plus tard.")
                    .color(0x95A5A6);
                return (embed, taunt_events);
            }
        };
        let msg_text = format_msg(
            &template_str,
            &format!("<@{}>", thief_id),
            &format!("<@{}>", target_id),
            stolen,
        );

        let embed = CreateEmbed::new()
            .title("\u{1f4b0} Vol reussi !")
            .description(format!("{}{}{}", msg_text, roll_detail, xp_line))
            .color(0x57F287)
            .footer(CreateEmbedFooter::new(sentinel_shared::branding::COUDE_TAGLINE_SHORT))
            .timestamp(serenity::model::Timestamp::now());
        (embed, taunt_events)
    } else {
        // Vol echoue : le voleur perd `steal_failure_penalty_pct`% de
        // ses coins, victime +3 XP.
        let lost =
            ((thief_player.coins as f64 * (failure_penalty_pct as f64 / 100.0)) as i64).max(1);

        // Migration wallet unifie : delegue la penalite a
        // record_steal_fail_penalty (wallet_uc.debit + faillite
        // auto-detectee cote voleur). Le montant reellement perdu peut
        // etre clamp au solde du voleur cote serveur ; le message
        // affiche utilise la valeur du serveur pour coherence.
        let lost = match api.record_steal_fail_penalty(guild_id, thief_id, lost).await {
            Ok((actual_lost, wallet_taunts)) => {
                taunt_events.extend(wallet_taunts);
                actual_lost.max(1)
            }
            Err(e) => {
                tracing::warn!(error = %e, "Echec API record_steal_fail_penalty");
                lost
            }
        };

        // Phase 9 Part D : vol rate = victime a "resiste", reset son streak.
        if let Err(e) = api.track_steal_defended(guild_id, target_id).await {
            tracing::warn!(error = %e, "Echec track_steal_defended (fail path)");
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

        let template_str: String = match api.random_flavor("steal_fail", "fr").await {
            Ok(Some(s)) => s,
            Ok(None) | Err(_) => {
                let embed = CreateEmbed::new()
                    .title("\u{26a0}\u{fe0f} API indisponible")
                    .description("Veuillez reessayer plus tard.")
                    .color(0x95A5A6);
                return (embed, taunt_events);
            }
        };
        let msg_text = format_msg(
            &template_str,
            &format!("<@{}>", thief_id),
            &format!("<@{}>", target_id),
            lost,
        );

        let embed = CreateEmbed::new()
            .title("\u{1f6a8} Vol rate !")
            .description(format!("{}{}{}", msg_text, roll_detail, xp_line))
            .color(0xED4245)
            .footer(CreateEmbedFooter::new(sentinel_shared::branding::COUDE_TAGLINE_SHORT))
            .timestamp(serenity::model::Timestamp::now());
        (embed, taunt_events)
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

    // Defer en mode UPDATE_MESSAGE : on acquitte le bouton avant les 3s sans
    // afficher de loader au user ; on editera le message original a la fin.
    if let Err(e) = component
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await
    {
        tracing::warn!(error = %e, "Echec defer handle_defend");
    }

    let config = load_guild_config(ctx, guild_id).await;
    let failure_penalty_pct = config.steal_failure_penalty_pct();

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
            let _ = component
                .create_followup(
                    &ctx.http,
                    serenity::all::CreateInteractionResponseFollowup::new()
                        .content(format!("Erreur API : {e}"))
                        .ephemeral(true),
                )
                .await;
            return;
        }
    };

    let target_player = match api
        .get_or_create_player(guild_id, target_id, &component.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            let _ = component
                .create_followup(
                    &ctx.http,
                    serenity::all::CreateInteractionResponseFollowup::new()
                        .content(format!("Erreur API : {e}"))
                        .ephemeral(true),
                )
                .await;
            return;
        }
    };

    let (embed, taunt_events) = resolve_steal_attempt(
        api,
        &catalog_defend,
        guild_id,
        thief_id,
        target_id,
        &thief_player,
        &target_player,
        false, // defense active
        failure_penalty_pct,
    )
    .await;

    // Apres Acknowledge (DEFERRED_UPDATE_MESSAGE), edit_response edite le
    // message d'origine (le challenge de vol) pour afficher le resultat et
    // retirer les boutons.
    let edit_result = component
        .edit_response(
            &ctx.http,
            serenity::all::EditInteractionResponse::new()
                .embed(embed.clone())
                .components(vec![]),
        )
        .await;

    if let Err(e) = edit_result {
        tracing::warn!(error = %e, thief_id, target_id, guild_id,
            "Echec edit_response Discord (defend vol) — fallback en followup");
        // Fallback : poste le resultat en followup public dans le salon
        // pour que le user voie quand meme le verdict de son roll, meme si
        // le message original n'est plus editable (supprime, expire, etc.).
        if let Err(e2) = component
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new()
                    .embed(embed),
            )
            .await
        {
            tracing::error!(error = %e2, thief_id, target_id, guild_id,
                "Echec followup defend vol — l utilisateur ne verra rien");
        }
    }

    // Drop le data guard avant le dispatch async (il lock TypeMap).
    drop(data);

    // Phase 9 Part D : dispatch IO pur.
    if !taunt_events.is_empty() {
        if let Ok(guild_id_u64) = guild_id.parse::<u64>() {
            let gid = serenity::all::GuildId::new(guild_id_u64);
            crate::modules::coude::taunts_dispatch::dispatch_all(ctx, gid, &taunt_events).await;
        }
    }
}

