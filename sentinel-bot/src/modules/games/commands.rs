use std::collections::HashMap;

use serenity::all::{
    ButtonStyle, Colour, CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
    CreateActionRow, CreateButton, CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, EditMessage, EditRole, GuildId, RoleId,
};
use serenity::builder::CreateEmbed;
use tracing::{info, warn};

use crate::shared::discord_helpers::reply_ephemeral as reply;
use crate::shared::embeds::{info_embed, success_embed};
use crate::shared::heartbeat::ApiClientKey;

use super::api_client::{Game, GameApiClient};
use super::emoji::parse_reaction_type;
use super::MODULE_BOT_NAME;

/// Prefix du custom_id des select menus de panel de jeux (LEGACY : anciens
/// panels deployes avant la bascule boutons ; le handler reste pour compat).
/// Format : `game_panel_select_{panel_id}_{chunk_index}`.
pub const PANEL_SELECT_PREFIX: &str = "game_panel_select_";

/// Prefix du custom_id des boutons-icones de panel de jeux (nouveau systeme).
/// Format : `game_panel_btn|{panel_id}|{game_id}`. Cliquer toggle le role du
/// jeu (abonnement aux notifs) et met a jour le compteur d'abonnes du bouton.
pub const PANEL_BUTTON_PREFIX: &str = "game_panel_btn|";

/// Max jeux affiches dans un panel a boutons (5 boutons x 5 rangees).
pub const MAX_BUTTONS_PER_PANEL: usize = 25;

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

    let guild_id_obj = match cmd.guild_id {
        Some(g) => g,
        None => { reply(ctx, cmd, "Commande disponible uniquement dans un serveur.").await; return; }
    };

    // Lit la couleur de role configuree pour game-bot (hex sans #).
    let role_color = load_role_color(&api.base, guild_id).await;

    // 1) Cree le role Discord.
    let role = match guild_id_obj
        .create_role(
            &ctx.http,
            EditRole::new()
                .name(&name)
                .colour(Colour::new(role_color))
                .mentionable(true)
                .hoist(false),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, game = %name, "Erreur create_role Discord");
            reply(ctx, cmd, &format!(
                "Erreur creation du role Discord : {e}. Verifie que le bot a la permission **Gerer les roles**."
            )).await;
            return;
        }
    };
    let role_id_str = role.id.get().to_string();

    // 2) Insere en DB avec le role_id. Si ca echoue, rollback du role Discord.
    match api
        .create_game(
            guild_id,
            &name,
            &cmd.user.id.to_string(),
            Some(&role_id_str),
            emoji_clean.as_deref(),
            category.as_deref(),
        )
        .await
    {
        Ok(game) => {
            let desc = format!(
                "**{}** {} est maintenant disponible.\nCategorie : {}\nRole : <@&{}>\nLes joueurs peuvent s'inscrire avec `/game join {}` ou via le panneau.",
                game.game_name,
                game.emoji.clone().unwrap_or_default(),
                game.category.clone().unwrap_or_else(|| "(aucune)".into()),
                role_id_str,
                game.game_name,
            );
            let embed = success_embed("Jeu cree !").description(desc);
            reply_embed(ctx, cmd, embed).await;
            info!(game = %game.game_name, role = %role_id_str, guild = %guild_id, "Jeu cree (avec role)");
        }
        Err(e) => {
            // Rollback : le jeu n'a pas ete cree, on supprime le role pour
            // eviter de laisser un role orphelin.
            if let Err(del_err) = guild_id_obj.delete_role(&ctx.http, role.id).await {
                warn!(error = %del_err, role = %role_id_str, "Rollback delete_role a echoue");
            }
            reply(ctx, cmd, &format!("Erreur : {e}")).await;
        }
    }
}

async fn load_role_color(base: &crate::shared::api_client::BaseApiClient, guild_id: &str) -> u32 {
    let raw = base
        .get_guild_config_for(guild_id, MODULE_BOT_NAME)
        .await
        .unwrap_or_default();
    let hex = crate::shared::api_client::BaseApiClient::config_or(&raw, "role_color", "3498db");
    let trimmed = hex.trim().trim_start_matches('#');
    u32::from_str_radix(trimmed, 16).unwrap_or(0x3498db)
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
            // Supprime le role Discord associe (best-effort : si l'admin
            // l'a deja supprime a la main, on ignore).
            if let (Some(role_id_str), Some(guild_id_obj)) = (game.role_id.as_deref(), cmd.guild_id) {
                if let Ok(rid) = role_id_str.parse::<u64>() {
                    if let Err(e) = guild_id_obj.delete_role(&ctx.http, RoleId::new(rid)).await {
                        warn!(error = %e, role = %role_id_str, game = %game.game_name, "Erreur delete_role (le role a peut-etre deja ete supprime manuellement)");
                    }
                }
            }
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

    let (guild_id_obj, role_id) = match (cmd.guild_id, game.role_id.as_deref().and_then(|s| s.parse::<u64>().ok())) {
        (Some(g), Some(rid)) => (g, RoleId::new(rid)),
        (Some(_), None) => {
            reply(ctx, cmd, &format!(
                "Le jeu **{}** n'a pas de role Discord associe (jeu legacy). Demande a un admin de le recreer.",
                game.game_name
            )).await;
            return;
        }
        _ => { reply(ctx, cmd, "Commande disponible uniquement dans un serveur.").await; return; }
    };

    let member = match guild_id_obj.member(&ctx.http, cmd.user.id).await {
        Ok(m) => m,
        Err(e) => { reply(ctx, cmd, &format!("Impossible de lire ton profil : {e}")).await; return; }
    };
    match member.add_role(&ctx.http, role_id).await {
        Ok(()) => reply(ctx, cmd, &format!(
            "Tu es inscrit a **{}** ! Utilise <@&{}> pour pinger les joueurs.",
            game.game_name, role_id.get()
        )).await,
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

    let (guild_id_obj, role_id) = match (cmd.guild_id, game.role_id.as_deref().and_then(|s| s.parse::<u64>().ok())) {
        (Some(g), Some(rid)) => (g, RoleId::new(rid)),
        (Some(_), None) => {
            reply(ctx, cmd, &format!("Le jeu **{}** n'a pas de role Discord associe.", game.game_name)).await;
            return;
        }
        _ => { reply(ctx, cmd, "Commande disponible uniquement dans un serveur.").await; return; }
    };

    let member = match guild_id_obj.member(&ctx.http, cmd.user.id).await {
        Ok(m) => m,
        Err(e) => { reply(ctx, cmd, &format!("Impossible de lire ton profil : {e}")).await; return; }
    };
    match member.remove_role(&ctx.http, role_id).await {
        Ok(()) => reply(ctx, cmd, &format!("Tu es desinscrit de **{}**.", game.game_name)).await,
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

    let games_slice: Vec<&Game> = games.iter().take(MAX_BUTTONS_PER_PANEL).collect();
    if games.len() > MAX_BUTTONS_PER_PANEL {
        warn!(total = games.len(), shown = MAX_BUTTONS_PER_PANEL, "Panel tronque : trop de jeux pour un seul message (max 25 boutons)");
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

    // 3) Edite le message pour attacher les boutons-icones en utilisant panel.id.
    let gid = cmd.guild_id.unwrap_or_default();
    let components = build_panel_button_components(ctx, gid, &panel.id, &games_slice);
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
    let games_slice: Vec<&Game> = games.iter().take(MAX_BUTTONS_PER_PANEL).collect();

    let embed = build_panel_embed(category.as_deref(), &games_slice);
    let gid = cmd.guild_id.unwrap_or_default();
    let components = build_panel_button_components(ctx, gid, &panel.id, &games_slice);

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

pub(crate) fn build_panel_embed(category: Option<&str>, games: &[&Game]) -> CreateEmbed {
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
        s.push_str("\n\n*Clique sur l'icone d'un jeu ci-dessous pour t'abonner / te desabonner a ses notifications. Le nombre = abonnes.*");
        s
    };
    info_embed(&title).description(desc)
}

/// Construit les rangees de BOUTONS-ICONES d'un panel (nouveau systeme).
/// Un bouton par jeu : emoji du jeu + compteur d'abonnes (membres ayant le
/// role). Max 25 jeux (5x5). Cliquer toggle le role. Partage entre la pose du
/// panel, le refresh et le handler de clic (pour re-render apres toggle).
pub(crate) fn build_panel_button_components(
    ctx: &Context,
    guild_id: GuildId,
    panel_id: &str,
    games: &[&Game],
) -> Vec<CreateActionRow> {
    if games.is_empty() {
        return Vec::new();
    }

    // Compte les abonnes de chaque role en UN seul passage du cache membres.
    let role_ids: Vec<RoleId> = games
        .iter()
        .filter_map(|g| g.role_id.as_deref().and_then(|s| s.parse::<u64>().ok()).map(RoleId::new))
        .collect();
    let counts = role_member_counts(ctx, guild_id, &role_ids);

    let shown: Vec<&&Game> = games.iter().take(MAX_BUTTONS_PER_PANEL).collect();
    shown
        .chunks(5)
        .map(|chunk| {
            let buttons: Vec<CreateButton> = chunk
                .iter()
                .map(|g| {
                    let role_id = g
                        .role_id
                        .as_deref()
                        .and_then(|s| s.parse::<u64>().ok())
                        .map(RoleId::new);
                    let count = role_id.and_then(|r| counts.get(&r)).copied().unwrap_or(0);
                    let cid = format!("{}{}|{}", PANEL_BUTTON_PREFIX, panel_id, g.id);
                    let mut btn = CreateButton::new(cid).style(ButtonStyle::Secondary);
                    match g.emoji.as_deref().and_then(parse_reaction_type) {
                        Some(rt) => btn = btn.emoji(rt).label(count.to_string()),
                        None => {
                            // Pas d'emoji : on retombe sur nom tronque + compteur.
                            let mut name = g.game_name.clone();
                            truncate_chars(&mut name, 70);
                            btn = btn.label(format!("{} {}", name, count));
                        }
                    }
                    btn
                })
                .collect();
            CreateActionRow::Buttons(buttons)
        })
        .collect()
}

/// Compte, depuis le cache, le nombre de membres possedant chacun des roles.
/// Un seul passage sur les membres du serveur (O(membres x roles/membre)).
fn role_member_counts(
    ctx: &Context,
    guild_id: GuildId,
    role_ids: &[RoleId],
) -> HashMap<RoleId, usize> {
    let mut counts: HashMap<RoleId, usize> = role_ids.iter().map(|r| (*r, 0usize)).collect();
    if counts.is_empty() {
        return counts;
    }
    if let Some(guild) = ctx.cache.guild(guild_id) {
        for member in guild.members.values() {
            for r in &member.roles {
                if let Some(c) = counts.get_mut(r) {
                    *c += 1;
                }
            }
        }
    }
    counts
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
