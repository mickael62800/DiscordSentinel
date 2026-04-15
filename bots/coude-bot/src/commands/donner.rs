use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::catalog::CatalogCacheKey;
use crate::handler::load_guild_config;
use crate::GameApiKey;

/// Minimum coins pour un don.
const MIN_COINS_GIFT: i64 = 10;
/// Le donneur doit garder au moins 50 coins apres le don.
const MIN_COINS_AFTER_GIFT: i64 = 50;
/// Taxe sur les dons de coins (10%).
const COIN_TAX_RATE: f64 = 0.10;
/// Cooldown pour les dons de coins (1 heure = 3600s).
const COIN_GIFT_COOLDOWN_SECS: i64 = 3600;

pub fn register() -> CreateCommand {
    CreateCommand::new("donner")
        .description("Donne des coins ou des items a un autre joueur")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "cible", "Le joueur a qui donner")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "type", "Ce que tu veux donner")
                .required(true)
                .add_string_choice("Coins", "coins")
                .add_string_choice("Potion de Soin", "potion_soin")
                .add_string_choice("Potion Majeure", "potion_majeure")
                .add_string_choice("Antidote", "antidote")
                .add_string_choice("Rage", "rage")
                .add_string_choice("Double Coup", "double_coup")
                .add_string_choice("Coup Traitre", "coup_traitre")
                .add_string_choice("Poison", "poison")
                .add_string_choice("Bouclier", "bouclier")
                .add_string_choice("Explosion", "explosion")
                .add_string_choice("Mindgame", "mindgame")
                .add_string_choice("Attaque Surprise", "surprise"),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "quantite",
                "Quantite (defaut 1 pour items, obligatoire pour coins)",
            )
            .required(false)
            .min_int_value(1),
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
    if !crate::channel_check::check_channel(ctx, command, config.channel_activites()).await {
        return;
    }

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

    let don_type = command
        .data
        .options
        .iter()
        .find(|o| o.name == "type")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let quantite = command
        .data
        .options
        .iter()
        .find(|o| o.name == "quantite")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(1);

    let donor_id = command.user.id.to_string();
    let target_id_str = target_id.to_string();

    if donor_id == target_id_str {
        reply_ephemeral(ctx, command, "Tu ne peux pas te donner a toi-meme !").await;
        return;
    }

    let target_user = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            reply_ephemeral(ctx, command, "Utilisateur introuvable.").await;
            return;
        }
    };

    if target_user.bot {
        reply_ephemeral(ctx, command, "Tu ne peux pas donner a un bot !").await;
        return;
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();
    let catalog = data.get::<CatalogCacheKey>().unwrap().clone();

    // Ensure both players exist
    let donor = match api
        .get_or_create_player(&guild_id, &donor_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    if let Err(e) = api
        .get_or_create_player(&guild_id, &target_id_str, &target_user.name)
        .await
    {
        reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
        return;
    }

    if don_type == "coins" {
        // ── Don de coins (avec taxe 10%, cooldown 1h) ──
        let amount = quantite;

        if amount < MIN_COINS_GIFT {
            reply_ephemeral(
                ctx,
                command,
                &format!("Le don minimum est de {} coins.", MIN_COINS_GIFT),
            )
            .await;
            return;
        }

        if donor.coins < amount {
            reply_ephemeral(
                ctx,
                command,
                &format!(
                    "Pas assez de coins ! Tu as {} coins.",
                    donor.coins
                ),
            )
            .await;
            return;
        }

        if donor.coins - amount < MIN_COINS_AFTER_GIFT {
            reply_ephemeral(
                ctx,
                command,
                &format!(
                    "Tu dois garder au moins {} coins apres le don. Tu as {} coins.",
                    MIN_COINS_AFTER_GIFT, donor.coins
                ),
            )
            .await;
            return;
        }

        // Check cooldown (1 hour)
        match api
            .check_cooldown(&guild_id, &donor_id, "donner_coins")
            .await
        {
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
                            "Tu dois attendre encore {}m{}s avant de pouvoir donner des coins !",
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

        // Apply 10% tax (gold sink)
        let tax = ((amount as f64) * COIN_TAX_RATE).ceil() as i64;
        let received = amount - tax;

        // Transfert atomique donor -> target de `received` coins (partie
        // qui arrive effectivement au destinataire). C'est une seule
        // transaction SQL cote API : impossible d'avoir un debit sans
        // credit correspondant.
        if let Err(e) = api
            .transfer_coins(&guild_id, &donor_id, &target_id_str, received)
            .await
        {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }

        // Debit separe de la taxe (gold sink : les coins sont juste
        // retires du donor, personne ne les recoit). Best-effort : si ce
        // debit echoue, le donor n'a pas paye la taxe mais le destinataire
        // a bien recu son montant. Pas de coins perdus, juste un manque
        // a gagner pour le sink.
        if tax > 0 {
            if let Err(e) = api.update_player_coins(&guild_id, &donor_id, -tax).await {
                tracing::warn!(error = %e, donor = %donor_id, tax, "Echec debit taxe donner (donor non taxe)");
            }
        }

        // Set cooldown
        if let Err(e) = api
            .set_cooldown(&guild_id, &donor_id, "donner_coins", COIN_GIFT_COOLDOWN_SECS)
            .await
        {
            tracing::warn!(error = %e, "Echec set_cooldown donner_coins");
        }

        let embed = CreateEmbed::new()
            .title("\u{1f381} Don de coins !")
            .description(format!(
                "<@{}> a donne **{} coins** a <@{}> !\n\n\
                 \u{1f4b0} Montant envoye : {} coins\n\
                 \u{1f3e6} Taxe (10%) : {} coins\n\
                 \u{2705} Montant recu : {} coins",
                command.user.id, amount, target_id, amount, tax, received
            ))
            .color(0x57F287)
            .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
            .timestamp(serenity::model::Timestamp::now());

        crate::channel_check::post_activity(ctx, command, config.channel_activites(), embed).await;
    } else {
        // ── Don d'item (pas de taxe, pas de cooldown) ──
        let item_key = &don_type;
        let qty = quantite as i32;

        // Verify item exists in shop
        let item = match catalog.get_item(item_key) {
            Some(i) => i,
            None => {
                reply_ephemeral(ctx, command, "Objet inconnu.").await;
                return;
            }
        };

        // Transfer items one by one, en accumulant les transferts reussis
        // pour pouvoir TOUT rollback si un intermediaire echoue. Avant, un
        // echec au i-eme item laissait les (i-1) premiers deja transferes
        // chez le destinataire (partial transfer).
        let mut transferred = 0i32;
        let mut error_message: Option<String> = None;

        for i in 0..qty {
            let has = match api.has_item(&guild_id, &donor_id, item_key).await {
                Ok(h) => h,
                Err(e) => { error_message = Some(format!("Erreur API : {e}")); break; }
            };
            if !has {
                error_message = Some(if i == 0 {
                    format!("Tu n'as pas de **{}** dans ton inventaire !", item.name)
                } else {
                    format!("Tu n'as que {} **{}** ! (don partiel impossible)", i, item.name)
                });
                break;
            }

            // Remove from donor
            if let Err(e) = api.use_item(&guild_id, &donor_id, item_key).await {
                error_message = Some(format!("Erreur API : {e}"));
                break;
            }

            // Give to receiver
            if let Err(e) = api.add_item(&guild_id, &target_id_str, item_key).await {
                // L'item est deja consomme cote donor mais pas chez la
                // target : rollback la consommation pour ne pas perdre
                // l'item. Ce rollback est en dehors du compteur
                // `transferred` car ce i-eme item n'a jamais atteint la
                // cible.
                let _ = api.add_item(&guild_id, &donor_id, item_key).await;
                error_message = Some(format!("Erreur API : {e}"));
                break;
            }

            transferred += 1;
        }

        if let Some(msg) = error_message {
            // Rollback complet : redonner au donor tous les items transferes
            // et les retirer au destinataire, pour revenir a l'etat initial.
            for _ in 0..transferred {
                let _ = api.use_item(&guild_id, &target_id_str, item_key).await;
                let _ = api.add_item(&guild_id, &donor_id, item_key).await;
            }
            reply_ephemeral(ctx, command, &msg).await;
            return;
        }

        let qty_label = if qty > 1 {
            format!("{}x ", qty)
        } else {
            String::new()
        };

        let embed = CreateEmbed::new()
            .title("\u{1f381} Don d'objet !")
            .description(format!(
                "<@{}> a donne **{}{}** {} a <@{}> !",
                command.user.id, qty_label, item.name, item.emoji, target_id
            ))
            .color(0x3498DB)
            .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
            .timestamp(serenity::model::Timestamp::now());

        crate::channel_check::post_activity(ctx, command, config.channel_activites(), embed).await;
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
