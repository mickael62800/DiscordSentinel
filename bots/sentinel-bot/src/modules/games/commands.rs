use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage, EditMessage,
};
use serenity::builder::CreateEmbed;
use tracing::{info, warn};

use sentinel_shared::discord_helpers::reply_ephemeral as reply;
use sentinel_shared::embeds::{info_embed, success_embed};
use sentinel_shared::heartbeat::ApiClientKey;

use super::api_client::{Game, GameApiClient};
use super::emoji::parse_reaction_type;

pub fn all() -> Vec<CreateCommand> {
    vec![register_public(), register_admin()]
}

fn register_public() -> CreateCommand {
    CreateCommand::new("game")
        .description("Consulter et s'inscrire aux jeux")
        .default_member_permissions(serenity::all::Permissions::empty())
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "list", "Lister les jeux disponibles"),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "join", "S'inscrire a un jeu")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "name", "Nom du jeu")
                        .required(true),
                ),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "leave", "Se desinscrire d'un jeu")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "name", "Nom du jeu")
                        .required(true),
                ),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "my-games", "Voir mes jeux"),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "players", "Voir les joueurs d'un jeu")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "name", "Nom du jeu")
                        .required(true),
                ),
        )
}

fn register_admin() -> CreateCommand {
    CreateCommand::new("game-admin")
        .description("Gerer les jeux (admin)")
        .default_member_permissions(serenity::all::Permissions::MANAGE_GUILD)
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "create", "Creer un jeu")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "name", "Nom du jeu")
                        .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "emoji", "Emoji (unicode ou <:name:id>)")
                        .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "category", "Categorie (ex: RPG)")
                        .required(false),
                ),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "delete", "Supprimer un jeu")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "name", "Nom du jeu")
                        .required(true),
                ),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "panel", "Deployer le panneau d'une categorie")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "category", "Categorie (vide = jeux sans categorie)")
                        .required(false),
                ),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "refresh", "Rafraichir le panneau d'une categorie")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "category", "Categorie (vide = jeux sans categorie)")
                        .required(false),
                ),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(g) => g.to_string(),
        None => {
            reply(ctx, command, "Cette commande ne fonctionne que dans un serveur.").await;
            return;
        }
    };

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(b) => b.clone(),
        None => return,
    };
    drop(data);

    let api = GameApiClient::new(base);

    let sub = &command.data.options;
    let sub_name = sub.first().map(|o| o.name.as_str()).unwrap_or("");
    let top_name = command.data.name.as_str();

    match (top_name, sub_name) {
        ("game-admin", "create") => handle_create(ctx, command, &api, &guild_id).await,
        ("game-admin", "delete") => handle_delete(ctx, command, &api, &guild_id).await,
        ("game-admin", "panel") => handle_panel(ctx, command, &api, &guild_id).await,
        ("game-admin", "refresh") => handle_refresh(ctx, command, &api, &guild_id).await,
        ("game", "list") => handle_list(ctx, command, &api, &guild_id).await,
        ("game", "join") => handle_join(ctx, command, &api, &guild_id).await,
        ("game", "leave") => handle_leave(ctx, command, &api, &guild_id).await,
        ("game", "my-games") => handle_my_games(ctx, command, &api, &guild_id).await,
        ("game", "players") => handle_players(ctx, command, &api, &guild_id).await,
        _ => reply(ctx, command, "Sous-commande inconnue.").await,
    }
}

// ── Sub-commands ──

async fn handle_create(ctx: &Context, cmd: &CommandInteraction, api: &GameApiClient, guild_id: &str) {
    if !has_manage_guild(ctx, cmd).await {
        reply(ctx, cmd, "Tu as besoin de la permission **Gerer le serveur** pour creer un jeu.").await;
        return;
    }

    let name = get_string_option(cmd, "name").unwrap_or_default();
    let emoji = get_string_option(cmd, "emoji").unwrap_or_default();
    let emoji_trim = emoji.trim();
    let category = get_string_option(cmd, "category");

    if parse_reaction_type(emoji_trim).is_none() {
        reply(ctx, cmd, "Emoji invalide. Utilise un emoji unicode (ex. 🎮) ou un emoji serveur (ex. `<:name:123456>`).").await;
        return;
    }

    match api
        .create_game(
            guild_id,
            &name,
            &cmd.user.id.to_string(),
            Some(emoji_trim),
            category.as_deref(),
        )
        .await
    {
        Ok(game) => {
            let desc = format!(
                "**{}** {} est maintenant disponible.\nCategorie : {}\nLes joueurs peuvent s'inscrire avec `/game join {}` ou via le panneau.",
                game.game_name,
                game.emoji.clone().unwrap_or_default(),
                game.category.clone().unwrap_or_else(|| "(aucune)".into()),
                game.game_name,
            );
            let embed = success_embed("Jeu cree !").description(desc);
            reply_embed(ctx, cmd, embed).await;
            info!(game = %game.game_name, guild = %guild_id, "Jeu cree");
        }
        Err(e) => reply(ctx, cmd, &format!("Erreur : {e}")).await,
    }
}

async fn handle_delete(ctx: &Context, cmd: &CommandInteraction, api: &GameApiClient, guild_id: &str) {
    if !has_manage_guild(ctx, cmd).await {
        reply(ctx, cmd, "Tu as besoin de la permission **Gerer le serveur** pour supprimer un jeu.").await;
        return;
    }

    let name = get_string_option(cmd, "name").unwrap_or_default();
    let game = match api.get_game_by_name(guild_id, &name).await {
        Ok(Some(g)) => g,
        Ok(None) => { reply(ctx, cmd, &format!("Jeu **{}** introuvable.", name)).await; return; }
        Err(e) => { reply(ctx, cmd, &format!("Erreur : {e}")).await; return; }
    };

    match api.delete_game(guild_id, &game.id).await {
        Ok(()) => {
            reply(ctx, cmd, &format!("Jeu **{}** supprime.", game.game_name)).await;
            info!(game = %game.game_name, guild = %guild_id, "Jeu supprime");
        }
        Err(e) => reply(ctx, cmd, &format!("Erreur : {e}")).await,
    }
}

async fn handle_list(ctx: &Context, cmd: &CommandInteraction, api: &GameApiClient, guild_id: &str) {
    match api.list_games(guild_id).await {
        Ok(games) => {
            if games.is_empty() {
                reply(ctx, cmd, "Aucun jeu configure. Un admin peut en creer avec `/game-admin create`.").await;
            } else {
                let list: String = games.iter()
                    .map(|g| format!("- {} **{}**", g.emoji.clone().unwrap_or_default(), g.game_name))
                    .collect::<Vec<_>>()
                    .join("\n");
                let embed = info_embed("Jeux disponibles")
                    .description(format!("{}\n\n*Inscris-toi avec `/game join <nom>`*", list));
                reply_embed(ctx, cmd, embed).await;
            }
        }
        Err(e) => reply(ctx, cmd, &format!("Erreur : {e}")).await,
    }
}

async fn handle_join(ctx: &Context, cmd: &CommandInteraction, api: &GameApiClient, guild_id: &str) {
    let name = get_string_option(cmd, "name").unwrap_or_default();
    let game = match api.get_game_by_name(guild_id, &name).await {
        Ok(Some(g)) => g,
        Ok(None) => { reply(ctx, cmd, &format!("Jeu **{}** introuvable. Utilise `/game list` pour voir les jeux.", name)).await; return; }
        Err(e) => { reply(ctx, cmd, &format!("Erreur : {e}")).await; return; }
    };

    match api.subscribe(guild_id, &game.id, &cmd.user.id.to_string()).await {
        Ok(()) => reply(ctx, cmd, &format!("Tu es inscrit a **{}** ! Tu seras ping quand quelqu'un ecrira `#{}`.", game.game_name, game.game_name)).await,
        Err(e) => reply(ctx, cmd, &format!("Erreur : {e}")).await,
    }
}

async fn handle_leave(ctx: &Context, cmd: &CommandInteraction, api: &GameApiClient, guild_id: &str) {
    let name = get_string_option(cmd, "name").unwrap_or_default();
    let game = match api.get_game_by_name(guild_id, &name).await {
        Ok(Some(g)) => g,
        Ok(None) => { reply(ctx, cmd, &format!("Jeu **{}** introuvable.", name)).await; return; }
        Err(e) => { reply(ctx, cmd, &format!("Erreur : {e}")).await; return; }
    };

    match api.unsubscribe(guild_id, &game.id, &cmd.user.id.to_string()).await {
        Ok(()) => reply(ctx, cmd, &format!("Tu es desinscrit de **{}**.", game.game_name)).await,
        Err(e) => reply(ctx, cmd, &format!("Erreur : {e}")).await,
    }
}

async fn handle_my_games(ctx: &Context, cmd: &CommandInteraction, api: &GameApiClient, guild_id: &str) {
    match api.get_user_games(guild_id, &cmd.user.id.to_string()).await {
        Ok(games) => {
            if games.is_empty() {
                reply(ctx, cmd, "Tu n'es inscrit a aucun jeu. Utilise `/game join <nom>`.").await;
            } else {
                let list: String = games.iter()
                    .map(|g| format!("- {} **{}**", g.emoji.clone().unwrap_or_default(), g.game_name))
                    .collect::<Vec<_>>()
                    .join("\n");
                reply(ctx, cmd, &format!("Tes jeux :\n{}", list)).await;
            }
        }
        Err(e) => reply(ctx, cmd, &format!("Erreur : {e}")).await,
    }
}

async fn handle_players(ctx: &Context, cmd: &CommandInteraction, api: &GameApiClient, guild_id: &str) {
    let name = get_string_option(cmd, "name").unwrap_or_default();
    let game = match api.get_game_by_name(guild_id, &name).await {
        Ok(Some(g)) => g,
        Ok(None) => { reply(ctx, cmd, &format!("Jeu **{}** introuvable.", name)).await; return; }
        Err(e) => { reply(ctx, cmd, &format!("Erreur : {e}")).await; return; }
    };

    match api.get_subscribers(guild_id, &game.id).await {
        Ok(subs) => {
            if subs.is_empty() {
                reply(ctx, cmd, &format!("Aucun joueur inscrit a **{}**.", game.game_name)).await;
            } else {
                let list: String = subs.iter()
                    .map(|s| format!("<@{}>", s.user_id))
                    .collect::<Vec<_>>()
                    .join(", ");
                reply(ctx, cmd, &format!("Joueurs inscrits a **{}** ({}) : {}", game.game_name, subs.len(), list)).await;
            }
        }
        Err(e) => reply(ctx, cmd, &format!("Erreur : {e}")).await,
    }
}

// ── Panels ──

async fn handle_panel(ctx: &Context, cmd: &CommandInteraction, api: &GameApiClient, guild_id: &str) {
    if !has_manage_guild(ctx, cmd).await {
        reply(ctx, cmd, "Permission **Gerer le serveur** requise.").await;
        return;
    }

    let category = get_string_option(cmd, "category");
    let games = match api.list_games_by_category(guild_id, category.as_deref()).await {
        Ok(g) => g,
        Err(e) => { reply(ctx, cmd, &format!("Erreur : {e}")).await; return; }
    };

    let games_with_emoji: Vec<&Game> = games.iter().filter(|g| g.emoji.is_some()).collect();
    if games_with_emoji.is_empty() {
        reply(ctx, cmd, "Aucun jeu avec emoji dans cette categorie. Ajoute-en avec `/game-admin create`.").await;
        return;
    }

    let embed = build_panel_embed(category.as_deref(), &games_with_emoji);

    let msg = match cmd
        .channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await
    {
        Ok(m) => m,
        Err(e) => { reply(ctx, cmd, &format!("Erreur envoi message : {e}")).await; return; }
    };

    // Ajoute les reactions
    for g in &games_with_emoji {
        if let Some(em) = &g.emoji {
            if let Some(rt) = parse_reaction_type(em) {
                if let Err(e) = msg.react(&ctx.http, rt).await {
                    warn!(error = %e, game = %g.game_name, "Echec ajout reaction panel");
                }
            }
        }
    }

    // Sauve le panel
    if let Err(e) = api
        .save_panel(
            guild_id,
            &msg.channel_id.to_string(),
            &msg.id.to_string(),
            category.as_deref(),
        )
        .await
    {
        reply(ctx, cmd, &format!("Panel envoye mais erreur de sauvegarde : {e}")).await;
        return;
    }

    reply(ctx, cmd, &format!("Panneau deploye ({} jeux).", games_with_emoji.len())).await;
}

async fn handle_refresh(ctx: &Context, cmd: &CommandInteraction, api: &GameApiClient, guild_id: &str) {
    if !has_manage_guild(ctx, cmd).await {
        reply(ctx, cmd, "Permission **Gerer le serveur** requise.").await;
        return;
    }

    let category = get_string_option(cmd, "category");

    // Trouve le panel existant via list_panels
    let panels = match api.list_panels(guild_id).await {
        Ok(p) => p,
        Err(e) => { reply(ctx, cmd, &format!("Erreur : {e}")).await; return; }
    };
    let cat_norm = category.as_deref().map(str::to_lowercase);
    let panel = panels.into_iter().find(|p| {
        p.category.as_deref().map(str::to_lowercase) == cat_norm
    });
    let panel = match panel {
        Some(p) => p,
        None => { reply(ctx, cmd, "Aucun panneau existant pour cette categorie. Utilise `/game-admin panel` d'abord.").await; return; }
    };

    let games = match api.list_games_by_category(guild_id, category.as_deref()).await {
        Ok(g) => g,
        Err(e) => { reply(ctx, cmd, &format!("Erreur : {e}")).await; return; }
    };
    let games_with_emoji: Vec<&Game> = games.iter().filter(|g| g.emoji.is_some()).collect();
    let embed = build_panel_embed(category.as_deref(), &games_with_emoji);

    // Edit le message
    let channel_id: serenity::model::id::ChannelId = match panel.channel_id.parse::<u64>() {
        Ok(id) => serenity::model::id::ChannelId::new(id),
        Err(_) => { reply(ctx, cmd, "channel_id invalide en DB.").await; return; }
    };
    let message_id: serenity::model::id::MessageId = match panel.message_id.parse::<u64>() {
        Ok(id) => serenity::model::id::MessageId::new(id),
        Err(_) => { reply(ctx, cmd, "message_id invalide en DB.").await; return; }
    };

    let mut msg = match channel_id.message(&ctx.http, message_id).await {
        Ok(m) => m,
        Err(e) => { reply(ctx, cmd, &format!("Message panneau introuvable : {e}")).await; return; }
    };

    if let Err(e) = msg.edit(&ctx.http, EditMessage::new().embed(embed)).await {
        reply(ctx, cmd, &format!("Erreur edition : {e}")).await;
        return;
    }

    // Reactions : on retire toutes les reactions du bot et on remet les bonnes.
    // Simple approche : delete_all_reactions (necessite MANAGE_MESSAGES).
    let _ = msg.delete_reactions(&ctx.http).await;
    for g in &games_with_emoji {
        if let Some(em) = &g.emoji {
            if let Some(rt) = parse_reaction_type(em) {
                if let Err(e) = msg.react(&ctx.http, rt).await {
                    warn!(error = %e, game = %g.game_name, "Echec ajout reaction refresh");
                }
            }
        }
    }

    reply(ctx, cmd, &format!("Panneau rafraichi ({} jeux).", games_with_emoji.len())).await;
}

fn build_panel_embed(category: Option<&str>, games: &[&Game]) -> CreateEmbed {
    let title = match category {
        Some(c) => format!("- [ {} ] -", c),
        None => "- [ Jeux ] -".to_string(),
    };
    let desc = if games.is_empty() {
        "*Aucun jeu.*".to_string()
    } else {
        games
            .iter()
            .map(|g| format!("{} @{}", g.emoji.clone().unwrap_or_default(), g.game_name))
            .collect::<Vec<_>>()
            .join("\n")
    };
    info_embed(&title).description(desc)
}

// ── Helpers ──

fn get_string_option(cmd: &CommandInteraction, name: &str) -> Option<String> {
    let sub = cmd.data.options.first()?;
    if let CommandDataOptionValue::SubCommand(opts) = &sub.value {
        opts.iter().find(|o| o.name == name).and_then(|o| {
            if let CommandDataOptionValue::String(s) = &o.value { Some(s.clone()) } else { None }
        })
    } else {
        None
    }
}

async fn has_manage_guild(ctx: &Context, cmd: &CommandInteraction) -> bool {
    if let Some(guild_id) = cmd.guild_id {
        if let Ok(member) = guild_id.member(&ctx.http, cmd.user.id).await {
            #[allow(deprecated)]
            if let Ok(perms) = member.permissions(&ctx.cache) {
                return perms.manage_guild();
            }
        }
    }
    false
}

async fn reply_embed(ctx: &Context, cmd: &CommandInteraction, embed: CreateEmbed) {
    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().embed(embed).ephemeral(true),
    );
    if let Err(e) = cmd.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur reponse embed commande game");
    }
}

