//! Commande `/potion` — utiliser une potion de l'inventaire hors combat
//! pour restaurer des HP. Les potions sont achetees via `/shop`.
//!
//! Contrairement a `/repos` (full heal, cooldown 12h), `/potion` est
//! utilisable autant de fois que tu as de potions en inventaire, mais
//! chaque potion ne rend qu'un montant fixe de HP.

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseFollowup,
    CreateInteractionResponseMessage,
};

use crate::modules::coude::catalog::CatalogCacheKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("potion")
        .description("Utilise une potion de soin pour recuperer des HP (hors combat)")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "type",
                "Le type de potion a utiliser",
            )
            .required(true)
            .add_string_choice("Potion de Soin (+30 HP)", "potion_soin")
            .add_string_choice("Potion Majeure (+80 HP)", "potion_majeure"),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some((guild_id, _config, api)) = crate::modules::coude::command_prelude::coude_prelude(
        ctx,
        command,
        |c| c.channel_profil(),
        false,
    )
    .await
    else {
        return;
    };

    // Defer immediat : on enchaine 3 appels API (get_player, has_item,
    // use_item, update_hp). Sans defer, Discord coupe l'interaction
    // apres 3s et affiche un "L'interaction a echoue" — ce que le
    // joueur voyait comme une "erreur". Le defer nous donne 15 min.
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(false),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec defer potion");
        return;
    }

    // Recuperer le choix de potion
    let potion_key = command
        .data
        .options
        .iter()
        .find(|o| o.name == "type")
        .and_then(|o| match &o.value {
            serenity::all::CommandDataOptionValue::String(s) => Some(s.clone()),
            _ => None,
        });

    let potion_key = match potion_key {
        Some(k) => k,
        None => {
            followup_info(ctx, command, "Type de potion manquant.").await;
            return;
        }
    };

    let user_id = command.user.id.to_string();

    let catalog = {
        let data = ctx.data.read().await;
        data.get::<CatalogCacheKey>().unwrap().clone()
    };

    // Le bot ne calcule plus le bareme/heal/clamp ni l'anti-gaspillage : tout
    // est resolu server-side de facon ATOMIQUE (consommation item + heal).
    let (actually_healed, new_hp, hp_max) = match api
        .use_potion(&guild_id, &user_id, &potion_key)
        .await
    {
        Ok(crate::modules::coude::api_client::UsePotionOutcome::Healed {
            actually_healed,
            new_hp,
            hp_max,
        }) => (actually_healed, new_hp, hp_max),
        Ok(crate::modules::coude::api_client::UsePotionOutcome::NotAPotion) => {
            followup_info(ctx, command, "Cet objet n'est pas une potion utilisable.").await;
            return;
        }
        Ok(crate::modules::coude::api_client::UsePotionOutcome::AlreadyFull) => {
            followup_info(
                ctx,
                command,
                "\u{2764}\u{fe0f} Tu es deja a pleine sante ! Inutile de gaspiller une potion.",
            )
            .await;
            return;
        }
        Ok(crate::modules::coude::api_client::UsePotionOutcome::Wasteful {
            hp_missing,
            heal_amount,
        }) => {
            let item_name = catalog
                .get_item(&potion_key)
                .map(|i| i.name.clone())
                .unwrap_or_else(|| "cette potion".into());
            followup_info(
                ctx,
                command,
                &format!(
                    "\u{26a0}\u{fe0f} Gaspillage ! Il ne te manque que **{}** HP, la **{}** en heal {}. Utilise une Potion de Soin (+30) ou attends davantage avant de l'utiliser.",
                    hp_missing, item_name, heal_amount
                ),
            )
            .await;
            return;
        }
        Ok(crate::modules::coude::api_client::UsePotionOutcome::NoItem) => {
            let name = catalog
                .get_item(&potion_key)
                .map(|i| i.name.clone())
                .unwrap_or_else(|| "cette potion".into());
            followup_info(
                ctx,
                command,
                &format!("Tu n'as pas de **{}** dans ton inventaire !", name),
            )
            .await;
            return;
        }
        Err(e) => {
            followup_info(ctx, command, &e).await;
            return;
        }
    };

    let item = catalog.get_item(&potion_key);
    let (emoji, name) = item
        .map(|i| (i.emoji.clone(), i.name.clone()))
        .unwrap_or_else(|| ("\u{1f9ea}".into(), "Potion".into()));

    let embed = CreateEmbed::new()
        .title(format!("{} Potion utilisee !", emoji))
        .description(format!(
            "<@{}> utilise une **{}** et recupere **+{} HP** !\n\n\
             \u{2764}\u{fe0f} **{}/{}** HP",
            command.user.id, name, actually_healed, new_hp, hp_max
        ))
        .color(0x57F287)
        .footer(CreateEmbedFooter::new(
            crate::shared::branding::COUDE_TAGLINE_SHORT,
        ))
        .timestamp(serenity::model::Timestamp::now());

    if let Err(e) = command
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new().embed(embed),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec followup Discord potion");
    }
}

/// Followup ephemeral apres defer (utilise quand le defer a deja ete fait).
async fn followup_info(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new()
                .content(content)
                .ephemeral(true),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec followup Discord potion");
    }
}

// `reply_ephemeral_pre_defer` supprime : `require_guild_id` (shared)
// utilise `reply_ephemeral` standard, suffisant ici (l'unique caller etait
// le guard guild_id qui s'execute avant tout defer).
