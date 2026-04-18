use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateActionRow,
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, CreateSelectMenu, CreateSelectMenuKind,
    CreateSelectMenuOption, EditMessage,
};
use serenity::builder::CreateEmbed;
use tracing::{info, warn};

use sentinel_shared::discord_helpers::reply_ephemeral as reply;
use sentinel_shared::embeds::{info_embed, success_embed};
use sentinel_shared::heartbeat::ApiClientKey;

use super::api_client::{Game, GameApiClient};
use super::emoji::parse_reaction_type;

/// Prefix du custom_id des select menus de panel de jeux.
/// Format : `game_panel_select_{panel_id}_{chunk_index}`.
pub const PANEL_SELECT_PREFIX: &str = "game_panel_select_";

/// Max options par select menu (limite Discord).
const MAX_OPTIONS_PER_SELECT: usize = 25;
/// Max select menus par message (limite Discord : 5 action rows).
const MAX_SELECTS_PER_MESSAGE: usize = 5;

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
                    CreateCommandOption::new(CommandOptionType::String, "emoji", "Emoji optionnel (unicode ou <:name:id>)")
                        .required(false),
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
    let emoji_raw = get_string_option(cmd, "emoji");
    let category = get_string_option(cmd, "category");

    // Emoji optionnel : on valide seulement s'il est fourni.
    let emoji_clean: Option<String> = match emoji_raw.as_deref().map(str::trim) {
        Some(e) if !e.is_empty() => {
            if parse_reaction_type(e).is_none() {
                reply(ctx, cmd, "Emoji invalide. Utilise un emoji unicode (ex. 🎮) ou un emoji serveur (ex. `<:name:123456>`).").await;
                return;
            }
            Some(e.to_string())
        }
        _ => None,
    };

    match api
        .create_game(
            guild_id,
            &name,
            &cmd.user.id.to_string(),
            emoji_clean.as_deref(),
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

    if games.is_empty() {
        reply(ctx, cmd, "Aucun jeu dans cette categorie. Ajoute-en avec `/game-admin create`.").await;
        return;
    }

    let max_games = MAX_OPTIONS_PER_SELECT * MAX_SELECTS_PER_MESSAGE;
    let games_slice: Vec<&Game> = games.iter().take(max_games).collect();
    if games.len() > max_games {
        warn!(total = games.len(), shown = max_games, "Panel tronque : trop de jeux pour un seul message");
    }

    let embed = build_panel_embed(category.as_deref(), &games_slice);

    // 1) Envoie un message initial avec l'embed seulement (pas encore de components).
    let msg = match cmd
        .channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await
    {
        Ok(m) => m,
        Err(e) => { reply(ctx, cmd, &format!("Erreur envoi message : {e}")).await; return; }
    };

    // 2) Sauve le panel en DB pour obtenir son UUID.
    let panel = match api
        .save_panel(
            guild_id,
            &msg.channel_id.to_string(),
            &msg.id.to_string(),
            category.as_deref(),
        )
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply(ctx, cmd, &format!("Panel envoye mais erreur de sauvegarde : {e}")).await;
            return;
        }
    };

    // 3) Edite le message pour attacher les components (select menus) en utilisant panel.id.
    let components = build_panel_components(&panel.id, &games_slice);
    let mut msg_mut = msg;
    if let Err(e) = msg_mut
        .edit(&ctx.http, EditMessage::new().components(components))
        .await
    {
        warn!(error = %e, "Erreur attachement components au panel");
    }

    reply(ctx, cmd, &format!("Panneau deploye ({} jeux).", games_slice.len())).await;
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
    let max_games = MAX_OPTIONS_PER_SELECT * MAX_SELECTS_PER_MESSAGE;
    let games_slice: Vec<&Game> = games.iter().take(max_games).collect();

    let embed = build_panel_embed(category.as_deref(), &games_slice);
    let components = build_panel_components(&panel.id, &games_slice);

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

    // Retire les eventuelles vieilles reactions (legacy panels pre-select-menu).
    let _ = msg.delete_reactions(&ctx.http).await;

    if let Err(e) = msg
        .edit(
            &ctx.http,
            EditMessage::new().embed(embed).components(components),
        )
        .await
    {
        reply(ctx, cmd, &format!("Erreur edition : {e}")).await;
        return;
    }

    reply(ctx, cmd, &format!("Panneau rafraichi ({} jeux).", games_slice.len())).await;
}

fn build_panel_embed(category: Option<&str>, games: &[&Game]) -> CreateEmbed {
    let title = match category {
        Some(c) => format!("- [ {} ] -", c),
        None => "- [ Jeux ] -".to_string(),
    };
    let desc = if games.is_empty() {
        "*Aucun jeu.*".to_string()
    } else {
        let mut lines = Vec::with_capacity(games.len());
        for (idx, g) in games.iter().enumerate() {
            let emoji = g.emoji.clone().unwrap_or_default();
            let prefix = if emoji.is_empty() {
                String::new()
            } else {
                format!("{} ", emoji)
            };
            lines.push(format!("{}. {}**{}**", idx + 1, prefix, g.game_name));
        }
        let mut s = lines.join("\n");
        s.push_str("\n\n*Utilise le menu ci-dessous pour selectionner les jeux que tu veux suivre.*");
        s
    };
    info_embed(&title).description(desc)
}

/// Construit les action rows de select menus pour un panel donne.
/// Si la liste depasse 25 jeux, on split en plusieurs select menus (max 5).
fn build_panel_components(panel_id: &str, games: &[&Game]) -> Vec<CreateActionRow> {
    if games.is_empty() {
        return Vec::new();
    }

    let total_chunks = games.chunks(MAX_OPTIONS_PER_SELECT).count();
    games
        .chunks(MAX_OPTIONS_PER_SELECT)
        .enumerate()
        .take(MAX_SELECTS_PER_MESSAGE)
        .map(|(chunk_idx, chunk)| {
            let options: Vec<CreateSelectMenuOption> = chunk
                .iter()
                .map(|g| build_select_option(g))
                .collect();

            let custom_id = format!("{}{}_{}", PANEL_SELECT_PREFIX, panel_id, chunk_idx);
            let placeholder = if total_chunks > 1 {
                format!(
                    "Choisis les jeux que tu veux suivre ({}/{})",
                    chunk_idx + 1,
                    total_chunks.min(MAX_SELECTS_PER_MESSAGE),
                )
            } else {
                "Choisis les jeux que tu veux suivre".to_string()
            };

            let max_values = options.len().min(MAX_OPTIONS_PER_SELECT) as u8;
            let select = CreateSelectMenu::new(
                custom_id,
                CreateSelectMenuKind::String { options },
            )
            .placeholder(placeholder)
            .min_values(0)
            .max_values(max_values);

            CreateActionRow::SelectMenu(select)
        })
        .collect()
}

fn build_select_option(g: &Game) -> CreateSelectMenuOption {
    // label : max 100 chars
    let mut label = g.game_name.clone();
    truncate_chars(&mut label, 100);

    let mut option = CreateSelectMenuOption::new(label, g.id.clone());

    if let Some(cat) = &g.category {
        if !cat.is_empty() {
            let mut desc = format!("Categorie : {}", cat);
            truncate_chars(&mut desc, 100);
            option = option.description(desc);
        }
    }

    if let Some(em) = &g.emoji {
        if let Some(rt) = parse_reaction_type(em) {
            option = option.emoji(rt);
        }
    }

    option
}

fn truncate_chars(s: &mut String, max: usize) {
    if s.chars().count() > max {
        let truncated: String = s.chars().take(max).collect();
        *s = truncated;
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
