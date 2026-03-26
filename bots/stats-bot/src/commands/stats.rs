use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use tracing::error;

use crate::handler::ApiClientKey;

/// Enregistre la commande /stats avec ses sous-commandes.
pub fn register() -> CreateCommand {
    CreateCommand::new("stats")
        .description("Consulter les statistiques du serveur ou d'un utilisateur")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "user",
                "Statistiques d'un utilisateur",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "target",
                    "Utilisateur cible (par défaut : vous)",
                ),
            ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "server",
            "Statistiques globales du serveur",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "top",
                "Classement des membres les plus actifs",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "limit",
                    "Nombre de membres à afficher (défaut : 10)",
                )
                .min_int_value(1)
                .max_int_value(25),
            ),
        )
}

/// Dispatch la slash command.
pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let sub = &command.data.options[0];
    let result = match sub.name.as_str() {
        "user" => handle_user(ctx, command).await,
        "server" => handle_server(ctx, command).await,
        "top" => handle_top(ctx, command).await,
        _ => reply_text(ctx, command, "Sous-commande inconnue.").await,
    };

    if let Err(e) = result {
        error!(error = %e, "Erreur commande stats");
    }
}

/// /stats user [target]
async fn handle_user(
    ctx: &Context,
    command: &CommandInteraction,
) -> Result<(), serenity::Error> {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return reply_text(ctx, command, "Cette commande ne fonctionne que dans un serveur.").await,
    };

    let sub_options = match &command.data.options[0].value {
        CommandDataOptionValue::SubCommand(opts) => opts,
        _ => return reply_text(ctx, command, "Erreur interne.").await,
    };

    let target_id = sub_options
        .iter()
        .find(|o| o.name == "target")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        })
        .unwrap_or(command.user.id);

    let target_user = target_id.to_user(&ctx.http).await.unwrap_or(command.user.clone());

    let data = ctx.data.read().await;
    let api = match data.get::<ApiClientKey>() {
        Some(api) => api,
        None => return reply_text(ctx, command, "Erreur interne : API non configurée.").await,
    };

    // Stats depuis l'API
    let stats = api
        .get_user_stats(&guild_id.to_string(), &target_id.to_string())
        .await
        .unwrap_or(None);

    let (message_count, voice_seconds) = match &stats {
        Some(s) => (s.message_count, s.voice_seconds),
        None => (0, 0),
    };

    // Infractions depuis l'API
    let infractions = api
        .get_infractions(&guild_id.to_string())
        .await
        .unwrap_or_default();

    let user_id_str = target_id.to_string();
    let user_infractions: Vec<_> = infractions
        .iter()
        .filter(|i| i.user_id == user_id_str)
        .collect();

    let warn_count = user_infractions.iter().filter(|i| i.action == "warn").count();
    let delete_count = user_infractions.iter().filter(|i| i.action == "delete").count();
    let mute_count = user_infractions.iter().filter(|i| i.action == "mute").count();
    let ban_count = user_infractions.iter().filter(|i| i.action == "ban").count();

    let hours = voice_seconds / 3600;
    let minutes = (voice_seconds % 3600) / 60;

    let embed = CreateEmbed::new()
        .title(format!("Statistiques de {}", target_user.name))
        .thumbnail(target_user.face())
        .color(0x5865F2)
        .field("Messages envoyés", format!("{message_count}"), true)
        .field("Temps en vocal", format!("{}h {:02}min", hours, minutes), true)
        .field("\u{200b}", "\u{200b}", true)
        .field("Avertissements", format!("{warn_count}"), true)
        .field("Messages supprimés", format!("{delete_count}"), true)
        .field("Mutes", format!("{mute_count}"), true)
        .field("Bans", format!("{ban_count}"), true)
        .field("Total infractions", format!("{}", user_infractions.len()), true)
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Données persistées via l'API Sentinel",
        ));

    reply_embed(ctx, command, embed).await
}

/// /stats server
async fn handle_server(
    ctx: &Context,
    command: &CommandInteraction,
) -> Result<(), serenity::Error> {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return reply_text(ctx, command, "Cette commande ne fonctionne que dans un serveur.").await,
    };

    let data = ctx.data.read().await;
    let api = match data.get::<ApiClientKey>() {
        Some(api) => api,
        None => return reply_text(ctx, command, "Erreur interne : API non configurée.").await,
    };

    let overview = match api.get_guild_overview(&guild_id.to_string()).await {
        Ok(o) => o,
        Err(e) => {
            error!(error = %e, "Erreur récupération overview");
            return reply_text(ctx, command, "Impossible de récupérer les statistiques du serveur.").await;
        }
    };

    let hours = overview.total_voice_seconds / 3600;
    let minutes = (overview.total_voice_seconds % 3600) / 60;

    let guild_name = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map(|g| g.name)
        .unwrap_or_else(|_| "Serveur".to_string());

    let embed = CreateEmbed::new()
        .title(format!("Statistiques de {guild_name}"))
        .color(0x57F287)
        .field("Messages totaux", format!("{}", overview.total_messages), true)
        .field("Temps vocal total", format!("{}h {:02}min", hours, minutes), true)
        .field("Membres actifs", format!("{}", overview.active_members), true)
        .field("Avertissements", format!("{}", overview.total_warns), true)
        .field("Mutes", format!("{}", overview.total_mutes), true)
        .field("Bans", format!("{}", overview.total_bans), true)
        .field("Total infractions", format!("{}", overview.total_infractions), true)
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Données persistées via l'API Sentinel",
        ));

    reply_embed(ctx, command, embed).await
}

/// /stats top [limit]
async fn handle_top(
    ctx: &Context,
    command: &CommandInteraction,
) -> Result<(), serenity::Error> {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return reply_text(ctx, command, "Cette commande ne fonctionne que dans un serveur.").await,
    };

    let sub_options = match &command.data.options[0].value {
        CommandDataOptionValue::SubCommand(opts) => opts,
        _ => return reply_text(ctx, command, "Erreur interne.").await,
    };

    let limit = sub_options
        .iter()
        .find(|o| o.name == "limit")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(n) => Some(*n as u32),
            _ => None,
        })
        .unwrap_or(10);

    let data = ctx.data.read().await;
    let api = match data.get::<ApiClientKey>() {
        Some(api) => api,
        None => return reply_text(ctx, command, "Erreur interne : API non configurée.").await,
    };

    let leaderboard = match api.get_leaderboard(&guild_id.to_string(), limit).await {
        Ok(l) => l,
        Err(e) => {
            error!(error = %e, "Erreur récupération leaderboard");
            return reply_text(ctx, command, "Impossible de récupérer le classement.").await;
        }
    };

    if leaderboard.is_empty() {
        return reply_text(ctx, command, "Aucune statistique disponible pour le moment.").await;
    }

    let mut description = String::new();
    for (i, member) in leaderboard.iter().enumerate() {
        let hours = member.voice_seconds / 3600;
        let minutes = (member.voice_seconds % 3600) / 60;
        let medal = match i {
            0 => "\u{1f947}",
            1 => "\u{1f948}",
            2 => "\u{1f949}",
            _ => "\u{25ab}\u{fe0f}",
        };
        description.push_str(&format!(
            "{medal} **#{}.** {} — {} messages | {}h {:02}min en vocal\n",
            i + 1,
            member.username,
            member.message_count,
            hours,
            minutes,
        ));
    }

    let embed = CreateEmbed::new()
        .title("Classement des membres les plus actifs")
        .description(description)
        .color(0xFEE75C)
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Données persistées via l'API Sentinel",
        ));

    reply_embed(ctx, command, embed).await
}

async fn reply_text(
    ctx: &Context,
    command: &CommandInteraction,
    content: &str,
) -> Result<(), serenity::Error> {
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
}

async fn reply_embed(
    ctx: &Context,
    command: &CommandInteraction,
    embed: CreateEmbed,
) -> Result<(), serenity::Error> {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(embed),
            ),
        )
        .await
}
