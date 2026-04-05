use serenity::all::{
    ButtonStyle, CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
    CreateActionRow, CreateButton, CreateCommand, CreateCommandOption, CreateEmbed,
    CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage,
};

use crate::game::progression;
use crate::handler::{GameDbKey, load_guild_config};

pub fn register() -> CreateCommand {
    CreateCommand::new("coude")
        .description("Defie un joueur en Coup de Coude !")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "cible", "Le joueur a defier")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "mise", "Montant de la mise (defaut: 10)")
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
                .add_string_choice("Inversion", "inversion"),
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
    let db = data.get::<GameDbKey>().unwrap();

    // Creer/recuperer les joueurs
    let attacker = match db
        .get_or_create_player(&guild_id, &command.user.id.to_string(), &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
            return;
        }
    };

    let defender_player = match db
        .get_or_create_player(&guild_id, &target.id.to_string(), &target.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
            return;
        }
    };

    // Matchmaking check
    let level_gap = (attacker.level - defender_player.level).abs();
    let (handicap, blocked) = progression::matchmaking_handicap(attacker.level, defender_player.level);

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
        reply_ephemeral(ctx, command, &format!("La mise minimum est de {} coins.", config.min_bet())).await;
        return;
    }
    if mise > config.max_bet() {
        reply_ephemeral(ctx, command, &format!("La mise maximum est de {} coins.", config.max_bet())).await;
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
    if let Ok(Some(_)) = db
        .get_pending_combat_for_attacker(&guild_id, &command.user.id.to_string())
        .await
    {
        reply_ephemeral(ctx, command, "Tu as deja un defi en attente !").await;
        return;
    }

    // Verifier l'item special
    if let Some(ref item_key) = special {
        let has = match db
            .has_item(&guild_id, &command.user.id.to_string(), item_key)
            .await
        {
            Ok(h) => h,
            Err(e) => {
                reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
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
        if let Err(e) = db
            .use_item(&guild_id, &command.user.id.to_string(), item_key)
            .await
        {
            reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
            return;
        }
    }

    // Creer le combat
    let combat = match db
        .create_combat(
            &guild_id,
            &command.channel_id.to_string(),
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
        // On simule un accept automatique
        drop(data);
        super::accepter::resolve_combat_internal(ctx, &combat, command.channel_id).await;

        command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(
                        CreateEmbed::new()
                            .title("\u{1f4a8} ATTAQUE SURPRISE !")
                            .description(format!(
                                "<@{}> lance une attaque surprise sur <@{}> !\nImpossible de refuser...",
                                command.user.id, target.id
                            ))
                            .color(0xFF4500)
                            .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
                            .timestamp(serenity::model::Timestamp::now()),
                    ),
                ),
            )
            .await
            .ok();
        return;
    }

    // Bloodbath event : auto-accept
    let data = ctx.data.read().await;
    let db = data.get::<GameDbKey>().unwrap();
    let events = db.get_active_events(&guild_id).await.unwrap_or_default();
    let bloodbath = events.iter().any(|e| e.event_type == "bloodbath");

    if bloodbath {
        drop(data);
        super::accepter::resolve_combat_internal(ctx, &combat, command.channel_id).await;

        command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(
                        CreateEmbed::new()
                            .title("\u{1fa78} BLOODBATH EN COURS !")
                            .description(format!(
                                "Pas le choix ! <@{}> est force d'accepter le defi de <@{}> !",
                                target.id, command.user.id
                            ))
                            .color(0xED4245)
                            .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
                            .timestamp(serenity::model::Timestamp::now()),
                    ),
                ),
            )
            .await
            .ok();
        return;
    }

    // Normal flow : envoyer le defi avec boutons
    let special_label = special
        .as_deref()
        .map(|s| format!(" | Special : **{}**", s))
        .unwrap_or_default();

    let handicap_warning = if level_gap >= 3 {
        let handicap_pct = ((1.0 - handicap) * 100.0) as i32;
        let stronger_name = if attacker.level > defender_player.level {
            format!("<@{}>", command.user.id)
        } else {
            format!("<@{}>", target.id)
        };
        format!(
            "\n\u{2696}\u{fe0f} **Handicap matchmaking** : {} a -{}% ATK (ecart {} niveaux). Si l'underdog gagne : mise doublee + XP x2 !",
            stronger_name, handicap_pct, level_gap
        )
    } else {
        String::new()
    };

    let embed = CreateEmbed::new()
        .title("\u{1f44a} Coup de Coude !")
        .description(format!(
            "<@{}> defie <@{}> pour **{} coins** !{}{}\n\n<@{}>, tu acceptes ?",
            command.user.id, target.id, mise, special_label, handicap_warning, target.id
        ))
        .color(0xFFA500)
        .field("Attaquant", format!("<@{}>", command.user.id), true)
        .field("Defenseur", format!("<@{}>", target.id), true)
        .field("Mise", format!("{} coins", mise), true)
        .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel — Expire dans 24h"))
        .timestamp(serenity::model::Timestamp::now());

    let accept_btn = CreateButton::new(format!("coude_accept:{}", combat.id))
        .label("Accepter")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{270a}".to_string(),
        ))
        .style(ButtonStyle::Success);

    let item_btn = CreateButton::new(format!("coude_defend:{}", combat.id))
        .label("Objet")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{1f6e1}\u{fe0f}".to_string(),
        ))
        .style(ButtonStyle::Primary);

    let refuse_btn = CreateButton::new(format!("coude_refuse:{}", combat.id))
        .label("Refuser")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{1f414}".to_string(),
        ))
        .style(ButtonStyle::Danger);

    let cancel_btn = CreateButton::new(format!("coude_cancel:{}", combat.id))
        .label("Annuler")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{274c}".to_string(),
        ))
        .style(ButtonStyle::Secondary);

    let row = CreateActionRow::Buttons(vec![accept_btn, item_btn, refuse_btn, cancel_btn]);

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(vec![row]),
            ),
        )
        .await
        .ok();
}

async fn reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
        .ok();
}
