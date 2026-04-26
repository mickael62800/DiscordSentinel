//! Commande `/tout-ou-rien` — backup signature (cf. COUPE_AMELIORATIONS 6.1).
//!
//! Une fois par semaine, le joueur peut miser l integralite de son wallet
//! sur un 50/50. Pile -> wallet x2 (annonce serveur). Face -> il garde
//! 20% (entree au "Memorial des clodos").
//!
//! Animation 10s entre l acceptation et le resultat.

use rand::Rng;
use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse,
};

use sentinel_shared::discord_helpers::reply_ephemeral;
use sentinel_shared::tout_ou_rien::{
    coin_delta, resolve_outcome, ToutOuRienOutcome, TOUT_OU_RIEN_COOLDOWN_KEY,
    TOUT_OU_RIEN_COOLDOWN_SECS,
};

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

const ANIMATION_DURATION_SECS: u64 = 10;
const MIN_BALANCE_FOR_PLAY: i64 = 100;

pub fn register() -> CreateCommand {
    CreateCommand::new("tout-ou-rien")
        .description("Mise tout ton wallet sur un 50/50 (1× par semaine, irreversible)")
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
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_activites()).await {
        return;
    }

    let user_id = command.user.id.to_string();
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    // 1. Cooldown weekly.
    match api.check_cooldown(&guild_id, &user_id, TOUT_OU_RIEN_COOLDOWN_KEY).await {
        Ok(Some(expires_at_str)) => {
            let expires = chrono::DateTime::parse_from_rfc3339(&expires_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            let remaining = expires
                .signed_duration_since(chrono::Utc::now())
                .num_seconds();
            if remaining > 0 {
                let days = remaining / 86_400;
                let hours = (remaining % 86_400) / 3_600;
                reply_ephemeral(
                    ctx,
                    command,
                    &format!(
                        "Tu as deja tout-ou-rien cette semaine. Reviens dans **{}j {}h**.",
                        days, hours
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

    // 2. Verif solde.
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
    if player.coins < MIN_BALANCE_FOR_PLAY {
        reply_ephemeral(
            ctx,
            command,
            &format!(
                "Il te faut au moins **{}c** pour jouer au tout-ou-rien (tu en as {}).",
                MIN_BALANCE_FOR_PLAY, player.coins
            ),
        )
        .await;
        return;
    }

    // 3. Annonce de l animation : message public deferred-style.
    let intro_embed = CreateEmbed::new()
        .title("\u{1f3b2} TOUT-OU-RIEN — la roue cosmique tourne...")
        .description(format!(
            "<@{}> mise **{}** coins (TOUT son wallet) sur un 50/50.\n\n\
             Pile : x2. Face : -80%.\n\
             \u{23f3} Animation 10 secondes...",
            command.user.id, player.coins
        ))
        .color(0xF1C40F)
        .footer(CreateEmbedFooter::new(sentinel_shared::branding::COUDE_TAGLINE_SHORT))
        .timestamp(serenity::model::Timestamp::now());

    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(intro_embed),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response /tout-ou-rien intro");
        return;
    }

    // 4. Tirage RNG (scope ferme avant await pour Send).
    let outcome = {
        let mut rng = rand::thread_rng();
        let roll: f64 = rng.gen_range(0.0..1.0);
        resolve_outcome(roll)
    };

    // 5. Attente animation.
    tokio::time::sleep(std::time::Duration::from_secs(ANIMATION_DURATION_SECS)).await;

    // 6. Application du delta + edit message final.
    let delta = coin_delta(player.coins, outcome);
    let final_balance = (player.coins + delta).max(0);

    if delta != 0 {
        if let Err(e) = api.update_player_coins(&guild_id, &user_id, delta).await {
            tracing::error!(error = %e, "Echec update_player_coins tout-ou-rien");
            // On continue quand meme pour afficher le resultat (le user
            // verra son solde reel via /profil).
        }
    }

    // 7. Pose le cooldown (apres le tirage, pour que double-clic genere
    //    quand meme une seule animation/payout).
    if let Err(e) = api
        .set_cooldown(&guild_id, &user_id, TOUT_OU_RIEN_COOLDOWN_KEY, TOUT_OU_RIEN_COOLDOWN_SECS)
        .await
    {
        tracing::warn!(error = %e, "Echec set_cooldown tout-ou-rien");
    }

    // 8. Edit message final.
    let final_embed = match outcome {
        ToutOuRienOutcome::Win => CreateEmbed::new()
            .title("\u{1f451} TOUT-OU-RIEN — VICTOIRE !")
            .description(format!(
                "<@{}> a TOUT MISE et a GAGNE !\n\n\
                 \u{1f4b0} Mise : **{}** coins\n\
                 \u{1f4c8} Gain : **+{}** coins\n\
                 \u{1f9e7} Solde final : **{}** coins\n\n\
                 Le serveur s incline. \u{1f44f}",
                command.user.id, player.coins, delta, final_balance
            ))
            .color(0xFFD700) // or
            .footer(CreateEmbedFooter::new("Tout-ou-rien · le destin a souri"))
            .timestamp(serenity::model::Timestamp::now()),
        ToutOuRienOutcome::Lose => CreateEmbed::new()
            .title("\u{1faa6} TOUT-OU-RIEN — RUINE COSMIQUE")
            .description(format!(
                "<@{}> a TOUT MISE et a TOUT PERDU (ou presque).\n\n\
                 \u{1f4b0} Mise : **{}** coins\n\
                 \u{1f4c9} Perte : **{}** coins (80%)\n\
                 \u{1f9fb} Solde final : **{}** coins\n\n\
                 Bienvenue au **Memorial des clodos**. \u{1f47b}",
                command.user.id, player.coins, -delta, final_balance
            ))
            .color(0xC0392B) // rouge sombre
            .footer(CreateEmbedFooter::new("Tout-ou-rien · le destin t a craches dessus"))
            .timestamp(serenity::model::Timestamp::now()),
    };

    if let Err(e) = command
        .edit_response(&ctx.http, EditInteractionResponse::new().embed(final_embed))
        .await
    {
        tracing::warn!(error = %e, "Echec edit_response /tout-ou-rien resultat");
    }
}
