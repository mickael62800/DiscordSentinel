//! Commande `/tout-ou-rien` — backup signature (cf. COUPE_AMELIORATIONS 6.1).
//!
//! Une fois par semaine, le joueur peut miser l integralite de son wallet
//! sur un 50/50. Pile -> wallet x2 (annonce serveur). Face -> il garde
//! 20% (entree au "Memorial des clodos").
//!
//! Animation 10s entre l acceptation et le resultat.
//!
//! Phase 2 #1 audit : la decision RNG + persistance + cooldown sont
//! desormais cote API (`/api/coude/{g}/tout-ou-rien/play`). Le bot ne
//! fait plus que l'animation et l'affichage du verdict.

use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse,
};

use sentinel_shared::discord_helpers::{reply_ephemeral, require_guild_id};

use crate::modules::coude::api_client::PlayToutOuRienResp;
use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

// Duree migree dans `Config::tout_ou_rien_animation_secs` (default 10).

pub fn register() -> CreateCommand {
    CreateCommand::new("tout-ou-rien")
        .description("Mise tout ton wallet sur un 50/50 (1× par semaine, irreversible)")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };

    let config = load_guild_config(ctx, &guild_id).await;
    if !config.casino_enabled() {
        reply_ephemeral(ctx, command, "Le casino est desactive pour ce serveur.").await;
        return;
    }
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_activites()).await {
        return;
    }

    let user_id = command.user.id.to_string();
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    // 1. Appel API atomique : check cooldown + verif solde + RNG +
    //    mutation wallet + cooldown + memorial. Toute la decision est
    //    serveur (auditable, rejouable). Cf. Phase 2 #1 audit.
    let resp: PlayToutOuRienResp =
        match api.play_tout_ou_rien(&guild_id, &user_id, &command.user.name).await {
            Ok(r) => r,
            Err(e) => {
                reply_ephemeral(ctx, command, &e).await;
                return;
            }
        };

    // 2. Annonce de l animation : message public deferred-style.
    //    On a deja le verdict, on attend juste 10s pour le suspense.
    let intro_embed = CreateEmbed::new()
        .title("\u{1f3b2} TOUT-OU-RIEN — la roue cosmique tourne...")
        .description(format!(
            "<@{}> mise **{}** coins (TOUT son wallet) sur un 50/50.\n\n\
             Pile : x2. Face : -80%.\n\
             \u{23f3} Animation 10 secondes...",
            command.user.id, resp.initial_coins
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

    // 3. Suspense.
    tokio::time::sleep(std::time::Duration::from_secs(config.tout_ou_rien_animation_secs())).await;

    // 4. Edit message final.
    let won = resp.outcome == "won";
    let final_embed = if won {
        CreateEmbed::new()
            .title("\u{1f451} TOUT-OU-RIEN — VICTOIRE !")
            .description(format!(
                "<@{}> a TOUT MISE et a GAGNE !\n\n\
                 \u{1f4b0} Mise : **{}** coins\n\
                 \u{1f4c8} Gain : **+{}** coins\n\
                 \u{1f9e7} Solde final : **{}** coins\n\n\
                 Le serveur s incline. \u{1f44f}",
                command.user.id, resp.initial_coins, resp.delta, resp.final_balance
            ))
            .color(0xFFD700) // or
            .footer(CreateEmbedFooter::new("Tout-ou-rien · le destin a souri"))
            .timestamp(serenity::model::Timestamp::now())
    } else {
        CreateEmbed::new()
            .title("\u{1faa6} TOUT-OU-RIEN — RUINE COSMIQUE")
            .description(format!(
                "<@{}> a TOUT MISE et a TOUT PERDU (ou presque).\n\n\
                 \u{1f4b0} Mise : **{}** coins\n\
                 \u{1f4c9} Perte : **{}** coins (80%)\n\
                 \u{1f9fb} Solde final : **{}** coins\n\n\
                 Bienvenue au **Memorial des clodos**. \u{1f47b}",
                command.user.id, resp.initial_coins, -resp.delta, resp.final_balance
            ))
            .color(0xC0392B) // rouge sombre
            .footer(CreateEmbedFooter::new("Tout-ou-rien · le destin t a craches dessus"))
            .timestamp(serenity::model::Timestamp::now())
    };

    if let Err(e) = command
        .edit_response(&ctx.http, EditInteractionResponse::new().embed(final_embed))
        .await
    {
        tracing::warn!(error = %e, "Echec edit_response /tout-ou-rien resultat");
    }
}
