//! Slash command `/coude` — défier un joueur en Coup de Coude.
//!
//! Le fichier `mod.rs` ne contient que la registration et l'orchestration
//! du handler (parsing options → validations → création combat → dispatch UI).
//! Les constructions d'embeds et boutons vivent dans `challenge_ui`.

mod challenge_ui;

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage,
};

use crate::game::progression;
use crate::handler::load_guild_config;
use crate::GameApiKey;

use challenge_ui::{
    build_bloodbath_embed, build_challenge_buttons, build_challenge_embed, build_handicap_warning,
    build_notification_embed, build_surprise_embed,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("coude")
        .description("Defie un joueur en Coup de Coude !")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "cible", "Le joueur a defier")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "mise",
                "Montant de la mise (defaut: 10)",
            )
            .required(false)
            .min_int_value(1),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "special", "Attaque speciale (item)")
                .required(false)
                .add_string_choice("Attaque surprise", "surprise")
                .add_string_choice("Double coup", "double_coup")
                .add_string_choice("Coup traitre", "coup_traitre")
                .add_string_choice("Rage", "rage")
                .add_string_choice("Explosion", "explosion")
                .add_string_choice("Poison", "poison"),
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
    if !crate::channel_check::check_channel(ctx, command, config.channel_combats()).await {
        return;
    }
    if !config.enabled() {
        reply_ephemeral(ctx, command, "Le jeu Coup de Coude est desactive sur ce serveur.").await;
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

    if target_id == command.user.id {
        reply_ephemeral(ctx, command, "Tu ne peux pas te defier toi-meme !").await;
        return;
    }

    let mise = command
        .data
        .options
        .iter()
        .find(|o| o.name == "mise")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(config.default_bet());

    let special = command
        .data
        .options
        .iter()
        .find(|o| o.name == "special")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.clone()),
            _ => None,
        });

    let target = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            reply_ephemeral(ctx, command, "Utilisateur introuvable.").await;
            return;
        }
    };

    if target.bot {
        reply_ephemeral(ctx, command, "Tu ne peux pas defier un bot !").await;
        return;
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    // Creer/recuperer les joueurs
    let attacker = match api
        .get_or_create_player(&guild_id, &command.user.id.to_string(), &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    let defender_player = match api
        .get_or_create_player(&guild_id, &target.id.to_string(), &target.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    // Matchmaking check
    let level_gap = (attacker.level - defender_player.level).abs();
    let (handicap, blocked) =
        progression::matchmaking_handicap(attacker.level, defender_player.level);

    if blocked {
        reply_ephemeral(
            ctx,
            command,
            &format!(
                "Ecart de niveau trop important ! ({} niveaux d'ecart, max 9)\n\
                 Ton niveau : {} | Niveau de <@{}> : {}",
                level_gap, attacker.level, target.id, defender_player.level
            ),
        )
        .await;
        return;
    }

    // Verifier la mise (limites depuis la config)
    if mise < config.min_bet() {
        reply_ephemeral(
            ctx,
            command,
            &format!("La mise minimum est de {} coins.", config.min_bet()),
        )
        .await;
        return;
    }
    if mise > config.max_bet() {
        reply_ephemeral(
            ctx,
            command,
            &format!("La mise maximum est de {} coins.", config.max_bet()),
        )
        .await;
        return;
    }
    if attacker.coins < mise {
        reply_ephemeral(
            ctx,
            command,
            &format!(
                "Tu n'as pas assez de coins ! (tu as {} coins, mise demandee : {})",
                attacker.coins, mise
            ),
        )
        .await;
        return;
    }

    // Verifier pas de combat en cours
    if let Ok(Some(_)) = api
        .get_pending_combat_for_attacker(&guild_id, &command.user.id.to_string())
        .await
    {
        reply_ephemeral(ctx, command, "Tu as deja un defi en attente !").await;
        return;
    }

    // Verifier l'item special
    if let Some(ref item_key) = special {
        let has = match api
            .has_item(&guild_id, &command.user.id.to_string(), item_key)
            .await
        {
            Ok(h) => h,
            Err(e) => {
                reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
                return;
            }
        };
        if !has {
            reply_ephemeral(
                ctx,
                command,
                &format!("Tu n'as pas l'objet **{}** dans ton inventaire !", item_key),
            )
            .await;
            return;
        }
        // Consommer l'item
        if let Err(e) = api
            .use_item(&guild_id, &command.user.id.to_string(), item_key)
            .await
        {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    }

    // Creer le combat (channel_id = salon combats configure)
    let combat_channel = config.channel_combats().unwrap(); // deja verifie par check_channel
    let combat = match api
        .create_combat(
            &guild_id,
            &combat_channel,
            &command.user.id.to_string(),
            &command.user.name,
            &target.id.to_string(),
            &target.name,
            mise,
            special.as_deref(),
        )
        .await
    {
        Ok(c) => c,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur creation combat : {e}")).await;
            return;
        }
    };

    // Si attaque surprise : auto-resolve (gere dans accepter)
    if special.as_deref() == Some("surprise") {
        drop(data);
        super::accepter::resolve_combat_internal(ctx, &combat, command.channel_id).await;

        if let Err(e) = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .embed(build_surprise_embed(command.user.id, target.id)),
                ),
            )
            .await
        {
            tracing::warn!(error = %e, "Echec response Discord");
        }
        return;
    }

    // Bloodbath event : auto-accept
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();
    let events = api.get_active_events(&guild_id).await.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "Echec API get_active_events");
        vec![]
    });
    let bloodbath = events.iter().any(|e| e.event_type == "bloodbath");

    if bloodbath {
        drop(data);
        super::accepter::resolve_combat_internal(ctx, &combat, command.channel_id).await;

        if let Err(e) = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .embed(build_bloodbath_embed(command.user.id, target.id)),
                ),
            )
            .await
        {
            tracing::warn!(error = %e, "Echec response Discord");
        }
        return;
    }

    // Normal flow : envoyer le defi avec boutons
    let special_label = special
        .as_deref()
        .map(|s| format!(" | Special : **{}**", s))
        .unwrap_or_default();

    let handicap_warning = build_handicap_warning(
        command.user.id,
        attacker.level,
        target.id,
        defender_player.level,
        handicap,
    );

    let embed = build_challenge_embed(
        command.user.id,
        target.id,
        mise,
        &special_label,
        &handicap_warning,
    );
    let row = build_challenge_buttons(&combat.id);

    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(vec![row]),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord");
    }

    // Notifier dans le salon notifications (mention du defenseur)
    if let Some(notif_ch) = config.channel_notifications() {
        if let Ok(ch_id) = notif_ch.parse::<u64>() {
            let notif_embed = build_notification_embed(
                target.id,
                &command.user.name,
                mise,
                &combat_channel,
            );

            if let Err(e) = serenity::model::id::ChannelId::new(ch_id)
                .send_message(&ctx.http, CreateMessage::new().embed(notif_embed))
                .await
            {
                tracing::warn!(error = %e, "Echec send_message salon notifications");
            }
        }
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
