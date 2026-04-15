//! Commande `/potion` — utiliser une potion de l'inventaire hors combat
//! pour restaurer des HP. Les potions sont achetees via `/shop`.
//!
//! Contrairement a `/repos` (full heal, cooldown 12h), `/potion` est
//! utilisable autant de fois que tu as de potions en inventaire, mais
//! chaque potion ne rend qu'un montant fixe de HP.

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
};

use crate::catalog::CatalogCacheKey;
use crate::handler::load_guild_config;
use crate::GameApiKey;

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
    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            reply_ephemeral_pre_defer(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::channel_check::check_channel(ctx, command, config.channel_profil()).await {
        return;
    }

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

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();
    let catalog = data.get::<CatalogCacheKey>().unwrap().clone();

    if !catalog.is_potion(&potion_key) {
        followup_info(ctx, command, "Cet objet n'est pas une potion utilisable.").await;
        return;
    }

    let heal_amount = catalog.potion_heal_amount(&potion_key);
    if heal_amount <= 0 {
        followup_info(ctx, command, "Potion invalide.").await;
        return;
    }

    // Charger le player pour connaitre HP actuels
    let player = match api
        .get_or_create_player(&guild_id, &user_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            followup_info(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    let hp_max = player.hp_max.unwrap_or(100);
    let hp_current = player.hp_current.unwrap_or(hp_max);

    if hp_current >= hp_max {
        followup_info(
            ctx,
            command,
            "\u{2764}\u{fe0f} Tu es deja a pleine sante ! Inutile de gaspiller une potion.",
        )
        .await;
        return;
    }

    // Protection anti-gaspillage : si la potion heal beaucoup plus que
    // le manque de HP (ex. potion_majeure +80 alors qu'il manque 10 HP),
    // on refuse l'usage et on propose la plus petite.
    let hp_missing = hp_max - hp_current;
    if heal_amount > hp_missing * 3 && heal_amount > 40 {
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

    // Verifier que le joueur a bien la potion
    match api.has_item(&guild_id, &user_id, &potion_key).await {
        Ok(true) => {}
        Ok(false) => {
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
            followup_info(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    }

    // Consommer l'item
    if let Err(e) = api.use_item(&guild_id, &user_id, &potion_key).await {
        followup_info(ctx, command, &format!("Erreur API (use_item) : {e}")).await;
        return;
    }

    // Calculer le soin effectif (clamp au HP max)
    let new_hp = (hp_current + heal_amount).min(hp_max);
    let actually_healed = new_hp - hp_current;

    // Mettre a jour les HP
    if let Err(e) = api.update_hp(&guild_id, &user_id, new_hp, hp_max).await {
        followup_info(ctx, command, &format!("Erreur API (update_hp) : {e}")).await;
        return;
    }

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
        .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
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

/// Reply ephemeral AVANT le defer (utilise pour les cas "commande invalide"
/// ou le channel check qui faillit avant d'arriver au defer).
async fn reply_ephemeral_pre_defer(ctx: &Context, command: &CommandInteraction, content: &str) {
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
        tracing::warn!(error = %e, "Echec response Discord potion (pre-defer)");
    }
}

