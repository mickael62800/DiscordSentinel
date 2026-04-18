use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::builder::CreateEmbed;
use tracing::{info, warn};

use sentinel_shared::discord_helpers::reply_ephemeral as reply;
use sentinel_shared::embeds::{success_embed, info_embed};
use sentinel_shared::heartbeat::ApiClientKey;

use super::api_client::GameApiClient;

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
                ),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "delete", "Supprimer un jeu")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "name", "Nom du jeu")
                        .required(true),
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
    // Check admin
    if !has_manage_guild(ctx, cmd).await {
        reply(ctx, cmd, "Tu as besoin de la permission **Gerer le serveur** pour creer un jeu.").await;
        return;
    }

    let name = get_string_option(cmd, "name").unwrap_or_default();
    match api.create_game(guild_id, &name, &cmd.user.id.to_string()).await {
        Ok(game) => {
            let embed = success_embed("Jeu cree !")
                .description(format!("**{}** est maintenant disponible.\nLes joueurs peuvent s'inscrire avec `/game join {}`\nMentionnez-le avec `#{}`", game.game_name, game.game_name, game.game_name));
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
                reply(ctx, cmd, "Aucun jeu configure. Un admin peut en creer avec `/game create`.").await;
            } else {
                let list: String = games.iter()
                    .map(|g| format!("- **{}**", g.game_name))
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
                    .map(|g| format!("- **{}**", g.game_name))
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
