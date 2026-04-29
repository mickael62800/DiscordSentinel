use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, CreateEmbedFooter,
};

use sentinel_shared::discord_helpers::{reply_ephemeral, require_guild_id, reply_api_err};

use crate::modules::coude::GameApiKey;
use crate::modules::coude::load_guild_config;

/// Tiers d'abonnement d'assurance.
///
/// Phase 1 leftovers audit : durees + multipliers migres dans
/// `Config` (`assurance_tier_{day,week,month}_{secs,mult}`). Les
/// labels et la liste des cles restent statiques (UI Discord).
struct InsuranceTier {
    key: &'static str,
    label: &'static str,
    duration_seconds: i64,
    multiplier: i64,
}

fn build_tiers(cfg: &crate::modules::coude::guild_config::Config) -> [InsuranceTier; 3] {
    [
        InsuranceTier {
            key: "day",
            label: "1 jour",
            duration_seconds: cfg.assurance_tier_day_secs(),
            multiplier: cfg.assurance_tier_day_mult(),
        },
        InsuranceTier {
            key: "week",
            label: "1 semaine",
            duration_seconds: cfg.assurance_tier_week_secs(),
            multiplier: cfg.assurance_tier_week_mult(),
        },
        InsuranceTier {
            key: "month",
            label: "1 mois",
            duration_seconds: cfg.assurance_tier_month_secs(),
            multiplier: cfg.assurance_tier_month_mult(),
        },
    ]
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
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };

    let user_id = command.user.id.to_string();

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_profil()).await {
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

    let tiers = build_tiers(&config);
    let tier = match tier_key.as_deref().and_then(|k| tiers.iter().find(|t| t.key == k)) {
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
            reply_api_err(ctx, command, e).await;
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

    // Verifier si deja assure (palier niveau 5 : 2 slots autorises).
    let max_slots: i32 = if player.level >= 5 { 2 } else { 1 };
    if max_slots < 2 {
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
                reply_api_err(ctx, command, e).await;
                return;
            }
        }
    }

    // Deduire le cout
    if let Err(e) = api
        .update_player_coins(&guild_id, &user_id, -total_cost)
        .await
    {
        reply_api_err(ctx, command, e).await;
        return;
    }

    // Phase 2 #3 audit : le RNG `is_scam` est cote API. Le bot envoie le
    // taux de scam (config guild) + duree + level, l'API roule et persiste.
    let resolved = match api
        .buy_insurance_with_roll(
            &guild_id,
            &user_id,
            config.insurance_scam_rate() as u32,
            tier.duration_seconds,
            player.level,
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            // Rembourser
            if let Err(e2) = api
                .update_player_coins(&guild_id, &user_id, total_cost)
                .await
            {
                tracing::warn!(error = %e2, "Echec DB update_player_coins remboursement");
            }
            reply_api_err(ctx, command, e).await;
            return;
        }
    };
    let is_scam = resolved.is_scam;

    // Phase 9 : le cout de l'assurance alimente la caisse communautaire.
    // Best-effort : si le deposit echoue, on garde l'assurance (le joueur
    // a deja paye, pas question de lui faire perdre son abonnement).
    if let Err(e) = api
        .deposit_cashbox(
            &guild_id,
            total_cost,
            crate::modules::coude::api_client::CashboxDepositSource::InsurancePurchase,
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
        .footer(CreateEmbedFooter::new(sentinel_shared::branding::COUDE_TAGLINE_SHORT))
        .timestamp(serenity::model::Timestamp::now());

    crate::modules::coude::channel_check::post_activity(ctx, command, config.channel_activites(), embed).await;
}
