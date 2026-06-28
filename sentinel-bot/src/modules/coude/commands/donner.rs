use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter,
};

use crate::shared::discord_helpers::{reply_ephemeral, require_guild_id, reply_api_err};

use crate::modules::coude::catalog::CatalogCacheKey;
use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

// Constantes migrees dans guild_config.rs :
// gift_min_coins, gift_min_coins_after, gift_tax_rate, gift_cooldown_secs

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
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_activites()).await {
        return;
    }

    let target_id =
        crate::shared::discord_helpers::option_user(&command.data.options, "cible").unwrap();

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

    let quantite_opt: Option<i64> = command
        .data
        .options
        .iter()
        .find(|o| o.name == "quantite")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(v) => Some(*v),
            _ => None,
        });
    // Defaut 1 pour les items ; pour les coins on exige explicitement le champ.
    let quantite = quantite_opt.unwrap_or(1);

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
            reply_api_err(ctx, command, e).await;
            return;
        }
    };

    if let Err(e) = api
        .get_or_create_player(&guild_id, &target_id_str, &target_user.name)
        .await
    {
        reply_api_err(ctx, command, e).await;
        return;
    }

    if don_type == "coins" {
        // ── Don de coins (avec taxe 10%, cooldown 1h) ──
        // Le montant DOIT venir du champ `quantite`. S'il est absent, on
        // l'explique clairement au lieu du trompeur "don minimum 10".
        let amount = match quantite_opt {
            Some(v) => v,
            None => {
                reply_ephemeral(
                    ctx,
                    command,
                    "Indique le montant du don dans le champ **quantite** (ex : quantite: 50).",
                )
                .await;
                return;
            }
        };

        if amount < config.gift_min_coins() {
            reply_ephemeral(
                ctx,
                command,
                &format!("Le don minimum est de {} coins.", config.gift_min_coins()),
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

        // Note : la validation "garder au moins X coins apres le don" est
        // desormais faite cote API (gift_coins), source de verite. On garde
        // les pre-checks ci-dessus uniquement comme retour UI rapide.

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
                reply_api_err(ctx, command, e).await;
                return;
            }
        }

        // Don taxe : taxe + validation du solde minimum + mutations wallet
        // (transfert de la part recue + debit de la taxe) sont calcules et
        // appliques **cote API** (gift_coins). Le bot ne calcule plus la regle.
        // L'API retourne (received, tax, taunts).
        let (received, tax, transfer_taunts) = match api
            .gift_coins(
                &guild_id,
                &donor_id,
                &target_id_str,
                amount,
                config.gift_tax_rate(),
                config.gift_min_coins_after(),
            )
            .await
        {
            Ok(r) => r,
            Err(e) => {
                reply_api_err(ctx, command, e).await;
                return;
            }
        };

        // Depot de la taxe dans la caisse communautaire (bookkeeping aval,
        // best-effort : la taxe est deja prelevee cote API).
        if tax > 0 {
            if let Err(e) = api
                .deposit_cashbox(
                    &guild_id,
                    tax,
                    crate::modules::coude::api_client::CashboxDepositSource::DonationTax,
                )
                .await
            {
                tracing::warn!(error = %e, guild_id, "Echec deposit cashbox donner");
            }
        }

        // Set cooldown
        if let Err(e) = api
            .set_cooldown(&guild_id, &donor_id, "donner_coins", config.gift_cooldown_secs())
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
            .footer(CreateEmbedFooter::new(crate::shared::branding::COUDE_TAGLINE_SHORT))
            .timestamp(serenity::model::Timestamp::now());

        crate::modules::coude::channel_check::post_activity(ctx, command, config.channel_activites(), embed).await;

        // Migration wallet unifie : dispatch en un seul passage les
        // TauntEvents retournes par l'API (faillite cote emetteur si le
        // don vide son wallet, jackpot cote recepteur, don genereux si
        // le montant depasse le seuil). Plus besoin d'un second appel
        // `track_generous_donor` (backend le declenche desormais dans
        // la meme transaction logique que le transfer).
        if !transfer_taunts.is_empty() {
            let guild_id_val = command.guild_id.unwrap();
            crate::modules::coude::taunts_dispatch::dispatch_all(
                ctx,
                guild_id_val,
                &transfer_taunts,
            )
            .await;
        }
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
                Err(e) => { error_message = Some(e); break; }
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
                error_message = Some(e);
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
                error_message = Some(e);
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
            .footer(CreateEmbedFooter::new(crate::shared::branding::COUDE_TAGLINE_SHORT))
            .timestamp(serenity::model::Timestamp::now());

        crate::modules::coude::channel_check::post_activity(ctx, command, config.channel_activites(), embed).await;
    }
}

