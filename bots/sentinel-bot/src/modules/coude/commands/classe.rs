use serenity::all::{
    ButtonStyle, CommandInteraction, ComponentInteraction, Context, CreateActionRow, CreateButton,
    CreateCommand, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use sentinel_shared::discord_helpers::{
    component_reply_ephemeral as reply_component_ephemeral, reply_ephemeral,
};

use crate::modules::coude::catalog::CatalogCacheKey;
use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

pub const CLASS_SELECT_PREFIX: &str = "classe_select:";

pub fn register() -> CreateCommand {
    CreateCommand::new("classe")
        .description("Choisis ou change ta classe de combat !")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let user_id = command.user.id.to_string();

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_profil()).await {
        return;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => return,
    };
    let catalog = match data.get::<CatalogCacheKey>() {
        Some(c) => c.clone(),
        None => return,
    };

    let player = match api.get_or_create_player(&guild_id, &user_id, &command.user.name).await {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    let current_class = player.class.as_deref().unwrap_or("bourrin");
    let has_chosen = player.class.is_some() && current_class != "bourrin" || player.level > 1;

    // Si le joueur a deja choisi et veut changer, verifier le cooldown (7 jours)
    if has_chosen {
        if let Some(ref changed_at) = player.class_changed_at {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(changed_at) {
                let elapsed = chrono::Utc::now().signed_duration_since(dt.with_timezone(&chrono::Utc));
                if elapsed.num_days() < 7 {
                    let remaining = 7 - elapsed.num_days();
                    reply_ephemeral(
                        ctx, command,
                        &format!("Tu dois attendre encore **{} jour(s)** avant de changer de classe !", remaining),
                    ).await;
                    return;
                }
            }
        }

        // Verifier les coins (cout configurable par guild)
        let cost = config.class_change_cost();
        if player.coins < cost {
            reply_ephemeral(
                ctx, command,
                &format!("Changer de classe coute **{} coins**. Tu n'as que {} coins.", cost, player.coins),
            ).await;
            return;
        }
    }

    // Afficher le menu de selection — tout vient du catalog cache.
    let current_class_info = catalog.get_class(current_class);

    let mut description = format!(
        "Classe actuelle : {} **{}** — {}\n\n",
        current_class_info.emoji, current_class_info.name, current_class_info.passif_description
    );

    if has_chosen {
        description.push_str(&format!("**Changer de classe coute {} coins.**\n\n", config.class_change_cost()));
    } else {
        description.push_str("**Premier choix gratuit !**\n\n");
    }

    description.push_str("Choisis ta classe :");

    let mut embed = CreateEmbed::new()
        .title("\u{2694}\u{fe0f} Choix de Classe")
        .description(&description);
    for c in &catalog.classes {
        embed = embed.field(
            format!("{} {}", c.emoji, capitalize(&c.name)),
            format!("ATK {} | DEF {} | {}", c.base_atk, c.base_def, c.passif_description),
            false,
        );
    }
    let embed = embed
        .color(0x3498DB)
        .footer(CreateEmbedFooter::new(sentinel_shared::branding::COUDE_TAGLINE_SHORT));

    let buttons = vec![
        CreateButton::new(format!("{CLASS_SELECT_PREFIX}bourrin"))
            .label("Bourrin")
            .emoji(serenity::model::channel::ReactionType::Unicode("\u{1f4aa}".to_string()))
            .style(ButtonStyle::Danger),
        CreateButton::new(format!("{CLASS_SELECT_PREFIX}agile"))
            .label("Agile")
            .emoji(serenity::model::channel::ReactionType::Unicode("\u{1f3c3}".to_string()))
            .style(ButtonStyle::Primary),
        CreateButton::new(format!("{CLASS_SELECT_PREFIX}fourbe"))
            .label("Fourbe")
            .emoji(serenity::model::channel::ReactionType::Unicode("\u{1f5e1}\u{fe0f}".to_string()))
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("{CLASS_SELECT_PREFIX}tank"))
            .label("Tank")
            .emoji(serenity::model::channel::ReactionType::Unicode("\u{1f6e1}\u{fe0f}".to_string()))
            .style(ButtonStyle::Success),
    ];

    let row = CreateActionRow::Buttons(buttons);

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(vec![row])
                    .ephemeral(true),
            ),
        )
        .await
        .ok();
}

pub async fn handle_select(ctx: &Context, component: &ComponentInteraction) {
    let class_name = match component.data.custom_id.strip_prefix(CLASS_SELECT_PREFIX) {
        Some(c) => c.to_string(),
        None => return,
    };

    let guild_id_early = match component.guild_id {
        Some(id) => id.to_string(),
        None => return,
    };
    let data_early = ctx.data.read().await;
    let catalog_early = match data_early.get::<CatalogCacheKey>() {
        Some(c) => c.clone(),
        None => return,
    };
    drop(data_early);
    if !catalog_early.classes.iter().any(|c| c.name == class_name) {
        reply_component_ephemeral(ctx, component, "Classe invalide.").await;
        return;
    }
    let _ = guild_id_early; // supprime via shadowing ci-dessous

    // Defer en mode UPDATE_MESSAGE : acquittement du menu avant les 3s. La
    // fin du handler editera le message d'origine (menu de classes) avec le
    // recap de la classe choisie.
    if let Err(e) = component
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await
    {
        tracing::warn!(error = %e, "Echec defer handle_select classe");
    }

    let guild_id = match component.guild_id {
        Some(id) => id.to_string(),
        None => return,
    };

    let user_id = component.user.id.to_string();

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => return,
    };

    // Helper closure-like : envoyer un followup ephemere (on est apres le defer).
    async fn followup_err(ctx: &Context, component: &ComponentInteraction, msg: String) {
        let _ = component
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new()
                    .content(msg)
                    .ephemeral(true),
            )
            .await;
    }

    let player = match api.get_or_create_player(&guild_id, &user_id, &component.user.name).await {
        Ok(p) => p,
        Err(e) => {
            followup_err(ctx, component, format!("Erreur API : {e}")).await;
            return;
        }
    };

    let current_class = player.class.as_deref().unwrap_or("bourrin");
    let has_chosen = player.class.is_some() && current_class != "bourrin" || player.level > 1;

    // Si c'est un changement (pas premier choix), deduire le cout (configurable).
    let config = load_guild_config(ctx, &guild_id).await;
    let class_cost = config.class_change_cost();
    if has_chosen {
        if player.coins < class_cost {
            followup_err(ctx, component, format!("Pas assez de coins ! ({} requis)", class_cost)).await;
            return;
        }
        if let Err(e) = api.update_player_coins(&guild_id, &user_id, -class_cost).await {
            followup_err(ctx, component, format!("Erreur API : {e}")).await;
            return;
        }
        // Phase 9 : le cout de changement de classe alimente la caisse.
        if let Err(e) = api
            .deposit_cashbox(
                &guild_id,
                class_cost,
                crate::modules::coude::api_client::CashboxDepositSource::ClassChangeCost,
            )
            .await
        {
            tracing::warn!(error = %e, guild_id, "Echec deposit cashbox classe");
        }
    }

    // Changer la classe
    if let Err(e) = api.update_player_class(&guild_id, &user_id, &class_name).await {
        followup_err(ctx, component, format!("Erreur API : {e}")).await;
        return;
    }

    let class_info = catalog_early.get_class(&class_name);

    let cost_msg = if has_chosen { format!(" (-{} coins)", class_cost) } else { " (gratuit)".to_string() };

    let embed = CreateEmbed::new()
        .title("\u{2728} Classe choisie !")
        .description(format!(
            "{} Tu es maintenant un **{}** !{}\n\n**{}**\n\n_{}_",
            class_info.emoji, class_info.name, cost_msg, class_info.passif_description, class_info.description
        ))
        .color(0x57F287)
        .footer(CreateEmbedFooter::new(sentinel_shared::branding::COUDE_TAGLINE_SHORT));

    // Apres Acknowledge (DEFERRED_UPDATE_MESSAGE), on edite le message d'origine.
    component
        .edit_response(
            &ctx.http,
            serenity::all::EditInteractionResponse::new()
                .embed(embed)
                .components(vec![]),
        )
        .await
        .ok();
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
