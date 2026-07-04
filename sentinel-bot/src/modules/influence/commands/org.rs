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
                        .field("Type", org.kind_label, true)
                        .field("Trésorerie", format!("{} 💰", org.treasury), true)
                        .description(if org.motto.is_empty() {
                            "Tu en es le **Fondateur**.".to_string()
                        } else {
                            format!("*« {} »*\n\nTu en es le **Fondateur**.", org.motto)
                        });
                    reply_ephemeral_embed(ctx, command, embed).await;
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
                reply_ephemeral(
                    ctx,
                    command,
                    &format!("{} Tu as rejoint **{}** comme Recrue.", org.emoji, org.name),
                )
                .await
            }
            Err(e) => reply_ephemeral(ctx, command, &format!("Impossible de rejoindre : {e}")).await,
        },
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
