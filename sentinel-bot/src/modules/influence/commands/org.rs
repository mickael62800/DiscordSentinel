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
                "dissoudre",
                "Dissout ton organisation (fondateur) et supprime ses salons",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "nom", "Ton organisation")
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
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "tresorerie",
                "Consulte la trésorerie d'une organisation",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "nom", "Organisation")
                    .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "don",
                "Reverse des coins à la trésorerie de ton organisation",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "nom", "Organisation")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::Integer, "montant", "Montant à reverser")
                    .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "retrait",
                "Retire des coins de la trésorerie (dirigeants)",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "nom", "Organisation")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::Integer, "montant", "Montant à retirer")
                    .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "paye",
                "Paie un membre depuis la trésorerie (dirigeants)",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "nom", "Organisation")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::User, "membre", "Membre à payer")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::Integer, "montant", "Montant à verser")
                    .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "dividende",
                "Verse un montant à CHAQUE membre depuis la trésorerie (dirigeants)",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "nom", "Organisation")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "montant",
                    "Montant versé à chaque membre",
                )
                .required(true),
            ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "classement",
            "Palmarès des organisations par trésor de guerre",
        ))
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
                    // Salon prive auto-cree : les membres de l'orga y discutent
                    // a l'abri des regards. Best-effort (ne bloque pas la fondation).
                    let mut channel_line = String::new();
                    if let Some(chan) = create_org_channel(ctx, &guild_id, &org.name, &user_id).await {
                        let _ =
                            api_client::link_channel(&api, &guild_id, &org.name, &chan.to_string())
                                .await;
                        channel_line = format!("\n\n📢 Salon privé : <#{chan}>");
                    }
                    // Vocal privé de l'org (meme categorie que le texte) : les
                    // membres peuvent communiquer en vocal. Best-effort.
                    if let Some(vchan) =
                        create_org_voice_channel(ctx, &guild_id, &org.name, &user_id).await
                    {
                        channel_line.push_str(&format!("\n🔊 Vocal privé : <#{vchan}>"));
                    }
                    let base_desc = if org.motto.is_empty() {
                        "Tu en es le **Fondateur**.".to_string()
                    } else {
                        format!("*« {} »*\n\nTu en es le **Fondateur**.", org.motto)
                    };
                    let embed = CreateEmbed::new()
                        .title(format!("{} Organisation fondée : {}", org.emoji, org.name))
                        .color(0x8E44AD)
                        .field("Type", org.kind_label.clone(), true)
                        .field("Trésorerie", format!("{} 💰", org.treasury), true)
                        .description(format!("{base_desc}{channel_line}"));
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
                // Acces au salon prive (texte) + au vocal prive de l'organisation.
                if let Some(chan) = &org.discord_channel_id {
                    grant_channel_access(ctx, chan, &user_id).await;
                }
                grant_org_voice_access(ctx, &guild_id, &org.name, &user_id).await;
                reply_ephemeral(
                    ctx,
                    command,
                    &format!("{} Tu as rejoint **{}** comme Recrue.", org.emoji, org.name),
                )
                .await
            }
            Err(e) => reply_ephemeral(ctx, command, &format!("Impossible de rejoindre : {e}")).await,
        },
        "dissoudre" => match api_client::dissolve_org(&api, &guild_id, &name, &user_id).await {
            Ok(org) => {
                cleanup_org_discord(ctx, &guild_id, &org).await;
                reply_ephemeral(
                    ctx,
                    command,
                    &format!(
                        "🗑️ L'organisation **{}** a été dissoute (salons et rôle supprimés).",
                        org.name
                    ),
                )
                .await;
                crate::modules::influence::press::publish_news(
                    ctx,
                    &guild_id,
                    &format!("Dissolution : {}", org.name),
                    &format!(
                        "L'organisation **{}** a été dissoute par <@{}>.",
                        org.name, user_id
                    ),
                )
                .await;
            }
            Err(e) => reply_ephemeral(ctx, command, &format!("Impossible de dissoudre : {e}")).await,
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
        "dividende" => {
            let montant = option_i64(&opts, "montant").unwrap_or(0);
            match api_client::treasury_dividend(&api, &guild_id, &name, &user_id, &username, montant)
                .await
            {
                Ok(r) => {
                    reply_ephemeral(
                        ctx,
                        command,
                        &format!(
                            "💸 Dividende de **{}** versé à **{}** membre(s) (total **{}**). Trésorerie restante : **{}** 💰",
                            r.per_member, r.paid_count, r.total, r.treasury_left
                        ),
                    )
                    .await;
                }
                Err(e) => reply_ephemeral(ctx, command, &format!("Impossible : {e}")).await,
            }
        }
        "classement" => match api_client::org_ranking(&api, &guild_id).await {
            Ok(orgs) => {
                let lines = if orgs.is_empty() {
                    "*Aucune organisation.*".to_string()
                } else {
                    orgs.iter()
                        .enumerate()
                        .map(|(i, o)| {
                            let medal = match i {
                                0 => "🥇",
                                1 => "🥈",
                                2 => "🥉",
                                _ => "▫️",
                            };
                            format!(
                                "{medal} **{}** — {} 💰 · ⚖️ {} influence ({} membres)",
                                o.name, o.treasury, o.collective_influence, o.member_count
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                let embed = CreateEmbed::new()
                    .title("🏆 Classement des organisations")
                    .color(0x8E44AD)
                    .description(lines);
                reply_ephemeral_embed(ctx, command, embed).await;
            }
            Err(e) => reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await,
        },
        "tresorerie" => match api_client::get_treasury(&api, &guild_id, &name).await {
            Ok(v) => reply_ephemeral_embed(ctx, command, treasury_embed(&v)).await,
            Err(e) => reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await,
        },
        "don" => {
            let montant = option_i64(&opts, "montant").unwrap_or(0);
            match api_client::treasury_deposit(&api, &guild_id, &name, &user_id, &username, montant)
                .await
            {
                Ok(v) => {
                    let embed = treasury_embed(&v).title(format!(
                        "💰 +{montant} versés à {}",
                        v.org_name
                    ));
                    reply_ephemeral_embed(ctx, command, embed).await;
                }
                Err(e) => reply_ephemeral(ctx, command, &format!("Impossible : {e}")).await,
            }
        }
        "retrait" => {
            let montant = option_i64(&opts, "montant").unwrap_or(0);
            match api_client::treasury_withdraw(&api, &guild_id, &name, &user_id, &username, montant)
                .await
            {
                Ok(v) => {
                    let embed = treasury_embed(&v)
                        .title(format!("💸 -{montant} retirés de {}", v.org_name));
                    reply_ephemeral_embed(ctx, command, embed).await;
                }
                Err(e) => reply_ephemeral(ctx, command, &format!("Impossible : {e}")).await,
            }
        }
        "paye" => {
            let montant = option_i64(&opts, "montant").unwrap_or(0);
            // Beneficiaire : option User -> id + pseudo (via resolved).
            let benef = opts.iter().find(|o| o.name == "membre").and_then(|o| match &o.value {
                CommandDataOptionValue::User(uid) => {
                    let name = command
                        .data
                        .resolved
                        .users
                        .get(uid)
                        .map(|u| u.name.clone())
                        .unwrap_or_default();
                    Some((uid.to_string(), name))
                }
                _ => None,
            });
            let Some((benef_id, benef_name)) = benef else {
                reply_ephemeral(ctx, command, "Membre à payer invalide.").await;
                return;
            };
            match api_client::treasury_pay(
                &api, &guild_id, &name, &user_id, &username, &benef_id, &benef_name, montant,
            )
            .await
            {
                Ok(v) => {
                    let embed = treasury_embed(&v)
                        .title(format!("💵 {montant} versés à {benef_name}"));
                    reply_ephemeral_embed(ctx, command, embed).await;
                }
                Err(e) => reply_ephemeral(ctx, command, &format!("Impossible : {e}")).await,
            }
        }
        _ => {}
    }
}

/// Extrait une option entiere.
fn option_i64(opts: &[serenity::all::CommandDataOption], key: &str) -> Option<i64> {
    opts.iter().find(|o| o.name == key).and_then(|o| match &o.value {
        CommandDataOptionValue::Integer(i) => Some(*i),
        _ => None,
    })
}

/// Embed de trésorerie (solde + derniers mouvements).
fn treasury_embed(v: &api_client::TreasuryView) -> CreateEmbed {
    let movements = if v.movements.is_empty() {
        "*Aucun mouvement.*".to_string()
    } else {
        v.movements
            .iter()
            .map(|m| {
                format!(
                    "• {} **{}** par {} — solde {}",
                    m.kind_label, m.amount, m.actor_username, m.treasury_after
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    CreateEmbed::new()
        .title(format!("🏦 Trésorerie de {}", v.org_name))
        .color(0x8E44AD)
        .field("Solde", format!("**{}** 💰", v.balance), false)
        .field("Derniers mouvements", movements, false)
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
        .field(
            "⚖️ Influence collective",
            o.collective_influence.to_string(),
            true,
        )
        .field(
            "🎖️ Réputation collective",
            o.collective_reputation.to_string(),
            true,
        );

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

/// Nom par defaut de la categorie d'accueil des salons d'organisations, utilisee
/// si `influence_org_category_id` n'est pas configure.
const ORG_CATEGORY_NAME: &str = "🏢 Organisations";

/// Cree le salon texte PRIVE d'une organisation, sous une categorie, visible des
/// seuls membres (et du bot). Best-effort : renvoie l'id du salon cree ou `None`.
async fn create_org_channel(
    ctx: &Context,
    guild_id: &str,
    org_name: &str,
    founder_user_id: &str,
) -> Option<serenity::model::id::ChannelId> {
    use serenity::all::{
        ChannelType, CreateChannel, PermissionOverwrite, PermissionOverwriteType, Permissions,
    };
    use serenity::model::id::{ChannelId, GuildId, RoleId, UserId};

    let gid = GuildId::new(guild_id.parse::<u64>().ok()?);

    // Categorie : config `influence_org_category_id`, sinon on cree/trouve
    // « 🏢 Organisations ».
    let cfg =
        crate::shared::discord_helpers::guild_config_or_default(ctx, guild_id, "influence-bot")
            .await;
    let category_id: Option<ChannelId> = match cfg
        .get("influence_org_category_id")
        .filter(|s| !s.is_empty())
    {
        Some(s) => s.parse::<u64>().ok().map(ChannelId::new),
        None => find_or_create_category(ctx, gid).await,
    };

    // @everyone ne voit pas le salon ; le fondateur (et le bot) le voient/parlent.
    let visible = Permissions::VIEW_CHANNEL
        | Permissions::SEND_MESSAGES
        | Permissions::READ_MESSAGE_HISTORY;
    let mut overwrites = vec![PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::VIEW_CHANNEL,
        kind: PermissionOverwriteType::Role(RoleId::new(gid.get())),
    }];
    if let Ok(fid) = founder_user_id.parse::<u64>() {
        overwrites.push(PermissionOverwrite {
            allow: visible,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(UserId::new(fid)),
        });
    }
    if let Ok(me) = ctx.http.get_current_user().await {
        overwrites.push(PermissionOverwrite {
            allow: visible,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(me.id),
        });
    }

    let mut builder = CreateChannel::new(org_name)
        .kind(ChannelType::Text)
        .topic(format!("Salon privé de l'organisation {org_name}"))
        .permissions(overwrites);
    if let Some(cat) = category_id {
        builder = builder.category(cat);
    }
    gid.create_channel(&ctx.http, builder).await.ok().map(|c| c.id)
}

/// Cree le VOCAL prive d'une organisation, sous la MEME categorie que son salon
/// texte, joignable des seuls membres (et du bot). Best-effort.
async fn create_org_voice_channel(
    ctx: &Context,
    guild_id: &str,
    org_name: &str,
    founder_user_id: &str,
) -> Option<serenity::model::id::ChannelId> {
    use serenity::all::{
        ChannelType, CreateChannel, PermissionOverwrite, PermissionOverwriteType, Permissions,
    };
    use serenity::model::id::{ChannelId, GuildId, RoleId, UserId};

    let gid = GuildId::new(guild_id.parse::<u64>().ok()?);

    // Meme categorie que le salon texte de l'org.
    let cfg =
        crate::shared::discord_helpers::guild_config_or_default(ctx, guild_id, "influence-bot")
            .await;
    let category_id: Option<ChannelId> = match cfg
        .get("influence_org_category_id")
        .filter(|s| !s.is_empty())
    {
        Some(s) => s.parse::<u64>().ok().map(ChannelId::new),
        None => find_or_create_category(ctx, gid).await,
    };

    // @everyone ne voit/rejoint pas ; fondateur + bot peuvent voir/parler.
    let allow = Permissions::VIEW_CHANNEL | Permissions::CONNECT | Permissions::SPEAK;
    let mut overwrites = vec![PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::VIEW_CHANNEL | Permissions::CONNECT,
        kind: PermissionOverwriteType::Role(RoleId::new(gid.get())),
    }];
    if let Ok(fid) = founder_user_id.parse::<u64>() {
        overwrites.push(PermissionOverwrite {
            allow,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(UserId::new(fid)),
        });
    }
    if let Ok(me) = ctx.http.get_current_user().await {
        overwrites.push(PermissionOverwrite {
            allow,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(me.id),
        });
    }

    let mut builder = CreateChannel::new(format!("🔊 {org_name}"))
        .kind(ChannelType::Voice)
        .permissions(overwrites);
    if let Some(cat) = category_id {
        builder = builder.category(cat);
    }
    gid.create_channel(&ctx.http, builder).await.ok().map(|c| c.id)
}

/// Trouve la categorie « Organisations » ou la cree si absente.
async fn find_or_create_category(
    ctx: &Context,
    gid: serenity::model::id::GuildId,
) -> Option<serenity::model::id::ChannelId> {
    use serenity::all::{ChannelType, CreateChannel};
    if let Ok(channels) = gid.channels(&ctx.http).await {
        if let Some(c) = channels
            .values()
            .find(|c| c.kind == ChannelType::Category && c.name == ORG_CATEGORY_NAME)
        {
            return Some(c.id);
        }
    }
    gid.create_channel(
        &ctx.http,
        CreateChannel::new(ORG_CATEGORY_NAME).kind(ChannelType::Category),
    )
    .await
    .ok()
    .map(|c| c.id)
}

/// Donne a un membre l'acces au salon prive de son organisation (best-effort).
async fn grant_channel_access(ctx: &Context, channel_id: &str, user_id: &str) {
    use serenity::all::{PermissionOverwrite, PermissionOverwriteType, Permissions};
    use serenity::model::id::{ChannelId, UserId};
    let (Ok(cid), Ok(uid)) = (channel_id.parse::<u64>(), user_id.parse::<u64>()) else {
        return;
    };
    let overwrite = PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL
            | Permissions::SEND_MESSAGES
            | Permissions::READ_MESSAGE_HISTORY,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(UserId::new(uid)),
    };
    let _ = ChannelId::new(cid).create_permission(&ctx.http, overwrite).await;
}

/// Supprime les ressources Discord d'une org dissoute : salon texte (id stocke),
/// salon vocal (retrouve par nom « 🔊 {org} ») et role Discord si present.
/// Best-effort (chaque suppression est independante).
async fn cleanup_org_discord(ctx: &Context, guild_id: &str, org: &api_client::Organization) {
    use serenity::all::ChannelType;
    use serenity::model::id::{ChannelId, GuildId, RoleId};

    // Salon texte (id stocke).
    if let Some(chan) = &org.discord_channel_id {
        if let Ok(cid) = chan.parse::<u64>() {
            let _ = ChannelId::new(cid).delete(&ctx.http).await;
        }
    }

    let Ok(gid) = guild_id.parse::<u64>() else {
        return;
    };
    let gid = GuildId::new(gid);

    // Salon vocal (retrouve par nom).
    let vname = format!("🔊 {}", org.name);
    if let Ok(channels) = gid.channels(&ctx.http).await {
        if let Some(vc) = channels
            .values()
            .find(|c| c.kind == ChannelType::Voice && c.name == vname)
        {
            let _ = vc.id.delete(&ctx.http).await;
        }
    }

    // Role Discord de l'org (si cree).
    if let Some(role) = &org.discord_role_id {
        if let Ok(rid) = role.parse::<u64>() {
            let _ = gid.delete_role(&ctx.http, RoleId::new(rid)).await;
        }
    }
}

/// Donne l'acces au VOCAL prive de l'org (retrouve par nom « 🔊 {org} ») a un
/// membre qui vient de rejoindre. Best-effort.
async fn grant_org_voice_access(ctx: &Context, guild_id: &str, org_name: &str, user_id: &str) {
    use serenity::all::{ChannelType, PermissionOverwrite, PermissionOverwriteType, Permissions};
    use serenity::model::id::{GuildId, UserId};
    let (Ok(gid), Ok(uid)) = (guild_id.parse::<u64>(), user_id.parse::<u64>()) else {
        return;
    };
    let gid = GuildId::new(gid);
    let name = format!("🔊 {org_name}");
    let Ok(channels) = gid.channels(&ctx.http).await else {
        return;
    };
    let Some(vc) = channels
        .values()
        .find(|c| c.kind == ChannelType::Voice && c.name == name)
    else {
        return;
    };
    let overwrite = PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL | Permissions::CONNECT | Permissions::SPEAK,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(UserId::new(uid)),
    };
    let _ = vc.id.create_permission(&ctx.http, overwrite).await;
}
