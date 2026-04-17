//! Commande /boost-voleur (Phase 9 Part C).
//!
//! Symmetrique de /protection : achete un abonnement qui ajoute un bonus
//! plat au roll de vol de l'attaquant pendant N jours. Ephemere pour
//! que la cible ne sache pas sur quoi l'attaquant se repose.

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::modules::coude::api_client::{CashboxDepositSource, StealProtectionDuration};
use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

struct BoostItemChoice {
    key: &'static str,
    label: &'static str,
}

/// Source de verite : domain cote API (`STEAL_BOOST_ITEMS`). On duplique
/// pour afficher les choices a Discord ; les prix viennent de l'API.
const ITEMS: &[BoostItemChoice] = &[
    BoostItemChoice {
        key: "crochet",
        label: "\u{1f527} Crochet (+5 au roll)",
    },
    BoostItemChoice {
        key: "passe_partout",
        label: "\u{1f5dd}\u{fe0f} Passe-partout (+10 au roll)",
    },
    BoostItemChoice {
        key: "deguisement",
        label: "\u{1f977} Deguisement (+15 au roll)",
    },
    BoostItemChoice {
        key: "fumigene",
        label: "\u{1f4a8} Fumigene (+20 au roll)",
    },
    BoostItemChoice {
        key: "marteau",
        label: "\u{1fa9a} Marteau (+25 au roll)",
    },
];

pub fn register() -> CreateCommand {
    let mut item_opt =
        CreateCommandOption::new(CommandOptionType::String, "item", "Item de boost a souscrire")
            .required(true);
    for i in ITEMS {
        item_opt = item_opt.add_string_choice(i.label, i.key);
    }

    CreateCommand::new("boost-voleur")
        .description("Souscris un abonnement boost voleur (secret, ephemere)")
        .add_option(item_opt)
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "duree", "Duree de l'abonnement")
                .required(true)
                .add_string_choice("1 jour", "1d")
                .add_string_choice("3 jours (-10%)", "3d")
                .add_string_choice("5 jours (-15%)", "5d")
                .add_string_choice("7 jours (-20%)", "7d"),
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

    let config = load_guild_config(ctx, &guild_id).await;
    if !config.enabled() {
        reply_ephemeral(ctx, command, "Le jeu Coup de Coude est desactive sur ce serveur.").await;
        return;
    }
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_profil()).await {
        return;
    }

    let user_id = command.user.id.to_string();

    let item_key = command
        .data
        .options
        .iter()
        .find(|o| o.name == "item")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.clone()),
            _ => None,
        });
    let duration_key = command
        .data
        .options
        .iter()
        .find(|o| o.name == "duree")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.clone()),
            _ => None,
        });

    let Some(item_key) = item_key else {
        reply_ephemeral(ctx, command, "Item manquant.").await;
        return;
    };
    let Some(duration) = duration_key.as_deref().and_then(StealProtectionDuration::from_key) else {
        reply_ephemeral(ctx, command, "Duree invalide.").await;
        return;
    };
    if !ITEMS.iter().any(|i| i.key == item_key) {
        reply_ephemeral(ctx, command, "Item inconnu.").await;
        return;
    }

    let item_label = ITEMS
        .iter()
        .find(|i| i.key == item_key)
        .map(|i| i.label)
        .unwrap_or(item_key.as_str());

    // Defer ephemeral : 4 appels API avant la reponse.
    if !crate::modules::coude::interaction_helper::defer_ephemeral(ctx, command).await {
        return;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => return,
    };

    let cost = match api.price_steal_boost(&item_key, duration).await {
        Ok(c) => c,
        Err(e) => {
            crate::modules::coude::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    let player = match api
        .get_or_create_player(&guild_id, &user_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            crate::modules::coude::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };
    if player.coins < cost {
        crate::modules::coude::interaction_helper::followup_text(
            ctx,
            command,
            &format!(
                "Pas assez de coins ! L'abonnement {} ({}) coute **{}** coins, tu en as {}.",
                item_label,
                duration.label(),
                cost,
                player.coins
            ),
        )
        .await;
        return;
    }

    if let Err(e) = api.update_player_coins(&guild_id, &user_id, -cost).await {
        crate::modules::coude::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
        return;
    }

    let (confirmed_cost, expires_at) = match api
        .buy_steal_boost(&guild_id, &user_id, &item_key, duration)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            if let Err(e2) = api.update_player_coins(&guild_id, &user_id, cost).await {
                tracing::warn!(error = %e2, "Echec remboursement boost apres echec souscription");
            }
            crate::modules::coude::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    if let Err(e) = api
        .deposit_cashbox(&guild_id, confirmed_cost, CashboxDepositSource::BoostPurchase)
        .await
    {
        tracing::warn!(error = %e, guild_id, "Echec deposit cashbox boost");
    }

    let expires_date = expires_at.split(&[' ', 'T'][..]).next().unwrap_or(&expires_at);
    let embed = CreateEmbed::new()
        .title("\u{1f5e1}\u{fe0f} Boost voleur active")
        .description(format!(
            "**{}** souscrit pour **{}**.\n\n\
             \u{1fa99} Cout : **{}** coins\n\
             \u{23f3} Expire le **{}**\n\n\
             _Ta cible ne saura rien. Les boosts actifs s'additionnent._",
            item_label,
            duration.label(),
            confirmed_cost,
            expires_date
        ))
        .color(0x8E44AD)
        .footer(CreateEmbedFooter::new(
            "Coup de Coude | Sentinel — Cumulatif avec les autres boosts",
        ))
        .timestamp(serenity::model::Timestamp::now());

    crate::modules::coude::interaction_helper::followup_embed_ephemeral(ctx, command, embed).await;
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
