//! Commande /protection (Phase 9 Part B).
//!
//! Souscription a un abonnement anti-vol. Les items anti-vol ne sont plus
//! des consommables dans l'inventaire public : ils vivent en DB cote API
//! comme des abonnements temps-base (1/3/5/7 jours) invisibles aux voleurs.
//!
//! TOUTES les reponses sont **ephemeres** : l'interet du systeme est que
//! l'attaquant ne sache pas si sa cible est protegee avant de tenter un
//! vol. Un message visible casserait cet effet de surprise.

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::api_client::{CashboxDepositSource, StealProtectionDuration};
use crate::handler::load_guild_config;
use crate::GameApiKey;

/// Catalogue affiche aux joueurs. Source de verite : domain cote API
/// (`STEAL_PROTECTION_ITEMS`). On duplique la liste ici pour proposer
/// les choices a Discord, mais les prix et les block_chance viennent
/// de l'API quand on les affiche.
struct ProtectionItemChoice {
    key: &'static str,
    label: &'static str,
}

const ITEMS: &[ProtectionItemChoice] = &[
    ProtectionItemChoice {
        key: "chien_garde",
        label: "\u{1f415} Chien de garde (25% block)",
    },
    ProtectionItemChoice {
        key: "alarme_sonore",
        label: "\u{1f514} Alarme sonore (30% block)",
    },
    ProtectionItemChoice {
        key: "piege_a_loup",
        label: "\u{1faa4} Piege a loup (35% block)",
    },
    ProtectionItemChoice {
        key: "camera_surveillance",
        label: "\u{1f4f9} Camera de surveillance (40% block)",
    },
    ProtectionItemChoice {
        key: "leurre_dore",
        label: "\u{1f36f} Leurre dore (45% block)",
    },
    ProtectionItemChoice {
        key: "garde_du_corps",
        label: "\u{1f46e} Garde du corps (50% block)",
    },
    ProtectionItemChoice {
        key: "coffre_fort",
        label: "\u{1f512} Coffre-fort (60% block)",
    },
    ProtectionItemChoice {
        key: "forteresse",
        label: "\u{1f3f0} Forteresse privee (70% block)",
    },
];

pub fn register() -> CreateCommand {
    let mut item_opt = CreateCommandOption::new(
        CommandOptionType::String,
        "item",
        "Item de protection a souscrire",
    )
    .required(true);
    for i in ITEMS {
        item_opt = item_opt.add_string_choice(i.label, i.key);
    }

    CreateCommand::new("protection")
        .description("Souscris un abonnement anti-vol (secret, ephemere)")
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
    // Channel check — mais si le joueur est dans le mauvais salon, on
    // veut quand meme une reponse ephemere pour ne rien laisser filtrer.
    if !crate::channel_check::check_channel(ctx, command, config.channel_profil()).await {
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
    // Valide aussi l'item cote bot pour ne pas taper l'API avec n'importe quoi.
    if !ITEMS.iter().any(|i| i.key == item_key) {
        reply_ephemeral(ctx, command, "Item inconnu.").await;
        return;
    }

    let item_label = ITEMS
        .iter()
        .find(|i| i.key == item_key)
        .map(|i| i.label)
        .unwrap_or(item_key.as_str());

    // Defer ephemeral : 4 appels API avant la reponse (price, get_player,
    // update_coins, buy_protection).
    if !crate::interaction_helper::defer_ephemeral(ctx, command).await {
        return;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => return,
    };

    // 1. Prix demande a l'API (source de verite du catalog)
    let cost = match api.price_steal_protection(&item_key, duration).await {
        Ok(c) => c,
        Err(e) => {
            crate::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    // 2. Charge le joueur pour verifier le solde
    let player = match api
        .get_or_create_player(&guild_id, &user_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            crate::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };
    if player.coins < cost {
        crate::interaction_helper::followup_text(
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

    // 3. Debit du wallet
    if let Err(e) = api.update_player_coins(&guild_id, &user_id, -cost).await {
        crate::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
        return;
    }

    // 4. Souscription. En cas d'echec, on rembourse.
    let (confirmed_cost, expires_at) = match api
        .buy_steal_protection(&guild_id, &user_id, &item_key, duration)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            if let Err(e2) = api.update_player_coins(&guild_id, &user_id, cost).await {
                tracing::warn!(error = %e2, "Echec remboursement protection apres echec souscription");
            }
            crate::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    // 5. Depot dans la caisse communautaire (Phase 9 Part A)
    if let Err(e) = api
        .deposit_cashbox(&guild_id, confirmed_cost, CashboxDepositSource::ProtectionPurchase)
        .await
    {
        tracing::warn!(error = %e, guild_id, "Echec deposit cashbox protection");
    }

    // 6. Reponse ephemere (secret face aux voleurs).
    let expires_date = expires_at.split(&[' ', 'T'][..]).next().unwrap_or(&expires_at);
    let embed = CreateEmbed::new()
        .title("\u{1f6e1}\u{fe0f} Protection activee")
        .description(format!(
            "**{}** souscrit pour **{}**.\n\n\
             \u{1fa99} Cout : **{}** coins\n\
             \u{23f3} Expire le **{}**\n\n\
             _Les voleurs ne verront rien venir._",
            item_label,
            duration.label(),
            confirmed_cost,
            expires_date
        ))
        .color(0x2ECC71)
        .footer(CreateEmbedFooter::new(
            "Coup de Coude | Sentinel — Cumulable avec /assurance",
        ))
        .timestamp(serenity::model::Timestamp::now());

    crate::interaction_helper::followup_embed_ephemeral(ctx, command, embed).await;
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
