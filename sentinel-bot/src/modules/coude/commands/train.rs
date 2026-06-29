use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter,
};

use crate::shared::discord_helpers::{reply_api_err, reply_ephemeral};

use crate::modules::coude::catalog::CatalogCacheKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("train")
        .description("Depense un point de statistique pour ameliorer tes stats !")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "stat", "La stat a ameliorer")
                .required(true)
                .add_string_choice("Attaque (+1 ATK)", "attaque")
                .add_string_choice("Defense (+1 DEF)", "defense"),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some((guild_id, config, api)) = crate::modules::coude::command_prelude::coude_prelude(
        ctx,
        command,
        |c| c.channel_profil(),
        true,
    )
    .await
    else {
        return;
    };

    let stat_choice = command
        .data
        .options
        .iter()
        .find(|o| o.name == "stat")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let db_stat = match stat_choice.as_str() {
        "attaque" => "atk",
        "defense" => "def",
        _ => {
            reply_ephemeral(
                ctx,
                command,
                "Choix invalide. Utilise `attaque` ou `defense`.",
            )
            .await;
            return;
        }
    };

    let catalog = {
        let data = ctx.data.read().await;
        data.get::<CatalogCacheKey>().unwrap().clone()
    };

    // Verifier que le joueur existe
    let player = match api
        .get_or_create_player(&guild_id, &command.user.id.to_string(), &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_api_err(ctx, command, e).await;
            return;
        }
    };

    if player.stat_points < 1 {
        reply_ephemeral(
            ctx,
            command,
            &format!(
                "Tu n'as pas de points de statistique a depenser ! (Points : {})\nGagne de l'XP en combattant pour monter de niveau.",
                player.stat_points
            ),
        )
        .await;
        return;
    }

    let updated = match api
        .spend_stat_point(&guild_id, &command.user.id.to_string(), db_stat)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    };

    let class = catalog.get_class(updated.class.as_deref().unwrap_or("bourrin"));
    let effective_atk = class.base_atk + (updated.level - 1) * class.atk_growth + updated.atk;
    let effective_def = class.base_def + (updated.level - 1) * class.def_growth + updated.def;
    let hp = catalog.display_hp(effective_def);

    let stat_label = if db_stat == "atk" { "ATK" } else { "DEF" };
    let stat_emoji = if db_stat == "atk" {
        "\u{2694}\u{fe0f}"
    } else {
        "\u{1f6e1}\u{fe0f}"
    };

    let embed = CreateEmbed::new()
        .title(format!("{} Entrainement : {} +1 !", stat_emoji, stat_label))
        .description(format!(
            "<@{}> depense 1 point de statistique en **{}** !\n\n\
             \u{2764}\u{fe0f} HP: **{}**  |  \u{2694}\u{fe0f} ATK: **{}**  |  \u{1f6e1}\u{fe0f} DEF: **{}**\n\
             \u{1f3af} Points restants : **{}**",
            command.user.id,
            stat_label,
            hp,
            effective_atk,
            effective_def,
            updated.stat_points
        ))
        .color(0x3498DB)
        .footer(CreateEmbedFooter::new(crate::shared::branding::COUDE_TAGLINE_SHORT))
        .timestamp(serenity::model::Timestamp::now());

    crate::modules::coude::channel_check::post_activity(
        ctx,
        command,
        config.channel_activites(),
        embed,
    )
    .await;
}
