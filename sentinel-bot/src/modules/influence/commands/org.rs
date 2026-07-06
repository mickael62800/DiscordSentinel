//! Commande `/org` — organisations du jeu Influence (05.md).
//!
//! Sous-commandes : `create`, `info`, `join`, `membres`.

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter,
};

use crate::modules::influence::api_client;
use crate::shared::discord_helpers::{option_str, reply_ephemeral, reply_ephemeral_embed, require_guild_id};
use crate::shared::heartbeat::ApiClientKey;

pub fn register() -> CreateCommand {
    // Choix du type d'organisation (aligne sur OrganizationKind).
    let kind_opt = CreateCommandOption::new(
        CommandOptionType::String,
        "type",
        "Type d'organisation",
    )
    .required(true)
    .add_string_choice("Entreprise", "entreprise")
    .add_string_choice("Parti politique", "parti")
    .add_string_choice("Média", "media")
    .add_string_choice("Syndicat", "syndicat")
    .add_string_choice("Organisation secrète", "secrete");

    CreateCommand::new("org")
        .description("Gère les organisations du jeu Influence")
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "create", "Fonde une organisation")
                .add_sub_option(kind_opt)
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "nom", "Nom de l'organisation")
                        .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "devise", "Devise / slogan")
                        .required(false),
                ),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "info", "Infos sur une organisation")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "nom", "Nom de l'organisation")
                        .required(true),
                ),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "join", "Rejoins une organisation")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "nom", "Nom de l'organisation")
                        .required(true),
                ),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "membres", "Liste les membres")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "nom", "Nom de l'organisation")
                        .required(true),
                ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "role",
                "Crée un rôle Discord au nom de ton organisation",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "nom", "Ton organisation")
                    .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "relation",
                "Declare une relation envers une autre organisation",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "nom", "Ton organisation")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "cible", "Organisation visee")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "type", "Nature de la relation")
                    .required(true)
                    .add_string_choice("Alliance", "alliance")
                    .add_string_choice("Rivalité", "rivalite")
                    .add_string_choice("Boycott", "boycott"),
            ),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else {
        return;
    };

    // Sous-commande + ses options.
    let Some(sub) = command.data.options.first() else {
        return;
    };
    let sub_name = sub.name.clone();
    let opts = match &sub.value {
        CommandDataOptionValue::SubCommand(o) => o.clone(),
        _ => return,
    };

    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };

    let user_id = command.user.id.to_string();
    let username = command.user.name.clone();
    let name = option_str(&opts, "nom").unwrap_or("").to_string();

    match sub_name.as_str() {
        "create" => {
            let kind = option_str(&opts, "type").unwrap_or("entreprise");
            let motto = option_str(&opts, "devise").unwrap_or("");
            match api_client::create_org(&api, &guild_id, &user_id, &username, kind, &name, motto).await {
                Ok(org) => {
                    let embed = CreateEmbed::new()
                        .title(format!("{} Organisation fondée : {}", org.emoji, org.name))
                        .color(0x8E44AD)
                        .field("Type", org.kind_label.clone(), true)
                        .field("Trésorerie", format!("{} 💰", org.treasury), true)
                        .description(if org.motto.is_empty() {
                            "Tu en es le **Fondateur**.".to_string()
                        } else {
                            format!("*« {} »*\n\nTu en es le **Fondateur**.", org.motto)
                        });
                    reply_ephemeral_embed(ctx, command, embed).await;
                    // Une du journal : une nouvelle organisation voit le jour.
                    crate::modules::influence::press::publish_news(
                        ctx,
                        &guild_id,
                        &format!("{} Nouvelle organisation : {}", org.emoji, org.name),
                        &format!(
                            "**{}** ({}) vient d'être fondée par <@{}>.",
                            org.name, org.kind_label, user_id
                        ),
                    )
                    .await;
                }
                Err(e) => reply_ephemeral(ctx, command, &format!("Impossible de fonder : {e}")).await,
            }
        }
        "info" => match api_client::org_info(&api, &guild_id, &name).await {
            Ok(o) => reply_ephemeral_embed(ctx, command, info_embed(&o)).await,
            Err(e) => reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await,
        },
        "join" => match api_client::join_org(&api, &guild_id, &name, &user_id, &username).await {
            Ok(org) => {
                // Si l'orga a un role Discord, on l'attribue au nouveau membre.
                if let (Some(role), Some(gid)) = (&org.discord_role_id, command.guild_id) {
                    if let Ok(rid) = role.parse::<u64>() {
                        if let Ok(m) = gid.member(&ctx.http, command.user.id).await {
                            let _ = m
                                .add_role(&ctx.http, serenity::model::id::RoleId::new(rid))
                                .await;
                        }
                    }
                }
                reply_ephemeral(
                    ctx,
                    command,
                    &format!("{} Tu as rejoint **{}** comme Recrue.", org.emoji, org.name),
                )
                .await
            }
            Err(e) => reply_ephemeral(ctx, command, &format!("Impossible de rejoindre : {e}")).await,
        },
        "role" => handle_role(ctx, command, &api, &guild_id, &user_id, &username, &name).await,
        "membres" => match api_client::org_members(&api, &guild_id, &name).await {
            Ok(members) => {
                let lines = if members.is_empty() {
                    "*Aucun membre.*".to_string()
                } else {
                    members
                        .iter()
                        .map(|m| format!("• **{}** — {}", m.username, m.role_label))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                let embed = CreateEmbed::new()
                    .title(format!("👥 Membres de {name}"))
                    .color(0x8E44AD)
                    .description(lines);
                reply_ephemeral_embed(ctx, command, embed).await;
            }
            Err(e) => reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await,
        },
        "relation" => {
            let cible = option_str(&opts, "cible").unwrap_or("");
            let rtype = option_str(&opts, "type").unwrap_or("");
            match api_client::set_relation(&api, &guild_id, &user_id, &username, &name, cible, rtype)
                .await
            {
                Ok(_) => {
                    reply_ephemeral(
                        ctx,
                        command,
                        &format!("🔗 Relation déclarée : **{name}** → **{cible}**."),
                    )
                    .await
                }
                Err(e) => reply_ephemeral(ctx, command, &format!("Impossible : {e}")).await,
            }
        }
        _ => {}
    }
}

fn info_embed(o: &api_client::OrgInfo) -> CreateEmbed {
    let embed = CreateEmbed::new()
        .title(format!("{} {}", o.emoji, o.name))
        .color(0x8E44AD)
        .description(if o.motto.is_empty() {
            String::new()
        } else {
            format!("*« {} »*", o.motto)
        })
        .field("Type", o.kind_label.clone(), true)
        .field("Membres", o.member_count.to_string(), true)
        .field("Trésorerie", format!("{} 💰", o.treasury), true)
        .field("Réputation", o.reputation.to_string(), true)
        .field("Influence", o.influence.to_string(), true);

    let embed = if o.relations.is_empty() {
        embed
    } else {
        let rels = o
            .relations
            .iter()
            .map(|r| format!("{} {} — {}", r.emoji, r.relation, r.other))
            .collect::<Vec<_>>()
            .join("\n");
        embed.field("Relations", rels, false)
    };

    embed.footer(CreateEmbedFooter::new(
        "Le patrimoine appartient à l'organisation, pas au dirigeant.",
    ))
}

/// Crée le rôle Discord de l'organisation (fondateur payant / modo gratuit).
async fn handle_role(
    ctx: &Context,
    command: &CommandInteraction,
    api: &std::sync::Arc<crate::shared::api_client::BaseApiClient>,
    guild_id: &str,
    user_id: &str,
    username: &str,
    org_name: &str,
) {
    use serenity::all::{EditRole, Permissions};
    let Some(gid) = command.guild_id else { return };

    // Moderateur ? (permissions de gestion des roles / admin).
    let is_moderator = command
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| p.contains(Permissions::MANAGE_ROLES) || p.contains(Permissions::ADMINISTRATOR))
        .unwrap_or(false);

    // Autorisation + cout cote API.
    let prep = match api_client::prepare_role(api, guild_id, user_id, username, is_moderator, org_name).await {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Impossible : {e}")).await;
            return;
        }
    };

    // Cree le role Discord au nom de l'orga.
    let role = match gid
        .create_role(&ctx.http, EditRole::new().name(&prep.org_name).mentionable(true))
        .await
    {
        Ok(r) => r,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Échec création du rôle : {e}")).await;
            return;
        }
    };

    // Attribue le role au fondateur.
    if let Ok(founder) = prep.founder_user_id.parse::<u64>() {
        if let Ok(m) = gid.member(&ctx.http, serenity::model::id::UserId::new(founder)).await {
            let _ = m.add_role(&ctx.http, role.id).await;
        }
    }

    // Lie le role a l'orga en base ET debite le fondateur (paiement effectif
    // ici, une fois le role reellement cree -> pas de double debit si la
    // creation echouait avant).
    let _ = api_client::link_role(
        api,
        guild_id,
        &prep.org_name,
        &role.id.to_string(),
        user_id,
        is_moderator,
    )
    .await;

    reply_ephemeral(
        ctx,
        command,
        &format!(
            "✅ Rôle <@&{}> créé pour **{}**. Les membres qui rejoignent l'obtiendront automatiquement.",
            role.id, prep.org_name
        ),
    )
    .await;
}
