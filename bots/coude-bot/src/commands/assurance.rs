use rand::Rng;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage,
};

use crate::GameApiKey;
use crate::handler::load_guild_config;

/// Tiers d'abonnement d'assurance.
///
/// Le prix est calcule comme `base_cost * multiplier`. Les multiplicateurs
/// offrent une petite remise sur les durees longues pour inciter aux
/// abonnements plus longs :
///   - 1 jour   : 1x  (prix de base, pas de remise)
///   - 1 semaine: 6x  (remise d'un jour par rapport a 7x quotidien)
///   - 1 mois   : 22x (remise de 8 jours par rapport a 30x quotidien)
struct InsuranceTier {
    key: &'static str,
    label: &'static str,
    duration_seconds: i64,
    multiplier: i64,
}

const TIERS: &[InsuranceTier] = &[
    InsuranceTier {
        key: "day",
        label: "1 jour",
        duration_seconds: 86_400,
        multiplier: 1,
    },
    InsuranceTier {
        key: "week",
        label: "1 semaine",
        duration_seconds: 604_800,
        multiplier: 6,
    },
    InsuranceTier {
        key: "month",
        label: "1 mois",
        duration_seconds: 2_592_000,
        multiplier: 22,
    },
];

fn find_tier(key: &str) -> Option<&'static InsuranceTier> {
    TIERS.iter().find(|t| t.key == key)
}

pub fn register() -> CreateCommand {
    CreateCommand::new("assurance")
        .description("Souscris une assurance temporaire contre les pertes de combat")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "duree",
                "Duree de l'abonnement d'assurance",
            )
            .required(true)
            .add_string_choice("1 jour (1x prix de base)", "day")
            .add_string_choice("1 semaine (6x prix de base)", "week")
            .add_string_choice("1 mois (22x prix de base)", "month"),
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

    let user_id = command.user.id.to_string();

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::channel_check::check_channel(ctx, command, config.channel_profil()).await {
        return;
    }
    let base_cost = config.insurance_cost();

    // Recuperer la duree choisie
    let tier_key = command
        .data
        .options
        .iter()
        .find(|o| o.name == "duree")
        .and_then(|o| match &o.value {
            serenity::all::CommandDataOptionValue::String(s) => Some(s.clone()),
            _ => None,
        });

    let tier = match tier_key.as_deref().and_then(find_tier) {
        Some(t) => t,
        None => {
            reply_ephemeral(ctx, command, "Duree d'abonnement invalide.").await;
            return;
        }
    };

    let total_cost = base_cost.saturating_mul(tier.multiplier);

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let player = match api
        .get_or_create_player(&guild_id, &user_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    if player.coins < total_cost {
        reply_ephemeral(
            ctx,
            command,
            &format!(
                "Pas assez de coins ! L'assurance **{}** coute {} coins, tu en as {}.",
                tier.label, total_cost, player.coins
            ),
        )
        .await;
        return;
    }

    // Verifier si deja assure
    match api.get_active_insurance(&guild_id, &user_id).await {
        Ok(Some(_)) => {
            reply_ephemeral(
                ctx,
                command,
                "Tu as deja une assurance active ! Attends sa fin avant de souscrire une nouvelle.",
            )
            .await;
            return;
        }
        Ok(None) => {}
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    }

    // Deduire le cout
    if let Err(e) = api
        .update_player_coins(&guild_id, &user_id, -total_cost)
        .await
    {
        reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
        return;
    }

    // Chance d'arnaque depuis la config
    let is_scam = {
        let mut rng = rand::thread_rng();
        rng.gen_range(1..=100) <= config.insurance_scam_rate()
    };

    if let Err(e) = api
        .buy_insurance(&guild_id, &user_id, is_scam, tier.duration_seconds)
        .await
    {
        // Rembourser
        if let Err(e2) = api
            .update_player_coins(&guild_id, &user_id, total_cost)
            .await
        {
            tracing::warn!(error = %e2, "Echec DB update_player_coins remboursement");
        }
        reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
        return;
    }

    // Phase 9 : le cout de l'assurance alimente la caisse communautaire.
    // Best-effort : si le deposit echoue, on garde l'assurance (le joueur
    // a deja paye, pas question de lui faire perdre son abonnement).
    if let Err(e) = api
        .deposit_cashbox(
            &guild_id,
            total_cost,
            crate::api_client::CashboxDepositSource::InsurancePurchase,
        )
        .await
    {
        tracing::warn!(error = %e, guild_id, "Echec deposit cashbox assurance");
    }

    let description = if is_scam {
        format!(
            "\u{1f6e1}\u{fe0f} <@{}> a souscrit une **Assurance Coup de Coude** pour **{}** !\n\n\
             Les pertes de combat seront reduites de 50%.\n\n\
             \u{1f6e1}\u{fe0f} Assurance activee... (mais est-elle fiable ? \u{1f60f})",
            command.user.id, tier.label
        )
    } else {
        format!(
            "\u{1f6e1}\u{fe0f} <@{}> a souscrit une **Assurance Coup de Coude** pour **{}** !\n\n\
             Les pertes de combat seront reduites de 50%.",
            command.user.id, tier.label
        )
    };

    let embed = CreateEmbed::new()
        .title("\u{1f6e1}\u{fe0f} Assurance activee !")
        .description(description)
        .color(0x3498DB)
        .field("Cout", format!("{} coins", total_cost), true)
        .field("Duree", tier.label, true)
        .field("Protection", "50% des pertes de combat", true)
        .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
        .timestamp(serenity::model::Timestamp::now());

    crate::channel_check::post_activity(ctx, command, config.channel_activites(), embed).await;
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
