//! Commande `/ban-sursis` — « ban en sursis » : au lieu d'un ban Discord direct,
//! on retire tous les roles du membre, on lui donne le role Sursis (ne voit que
//! le reglement + son salon d'appel), et il a N jours pour contester. Passe ce
//! delai, un worker le bannit definitivement.
//!
//! Boutons modo dans le salon d'appel : Gracier (restaure) / Bannir maintenant.

use serenity::all::{
    ButtonStyle, CommandInteraction, CommandOptionType, ComponentInteraction, Context,
    CreateButton, CreateCommand, CreateCommandOption, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage, EditMember,
};
use serenity::model::id::{RoleId, UserId};
use tracing::{info, warn};

use crate::shared::discord_helpers::{option_str, option_user, reply_ephemeral, require_guild_id};
use crate::shared::grpc_client::GrpcClientKey;
use crate::shared::heartbeat::ApiClientKey;

use super::super::api_client::{ApiClient, CreateSursisParams, SursisData};

pub const SURSIS_PARDON_PREFIX: &str = "mod_sursis_pardon_";
pub const SURSIS_BAN_PREFIX: &str = "mod_sursis_ban_";

pub fn register() -> CreateCommand {
    CreateCommand::new("ban-sursis")
        .description("Ban avec appel : le membre passe en Sursis et peut contester")
        .default_member_permissions(serenity::all::Permissions::BAN_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "membre", "Membre a sanctionner")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "reason", "Raison").required(false),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    // Garde de permission (le default_member_permissions du register est
    // reecrivable par un admin de guilde -> on re-verifie cote code, comme /ban).
    if !super::has_mod_permission(command, serenity::all::Permissions::BAN_MEMBERS) {
        reply_ephemeral(
            ctx,
            command,
            "❌ Permission BAN_MEMBERS requise pour /ban-sursis.",
        )
        .await;
        warn!(user = %command.user.name, "Tentative /ban-sursis sans permission");
        return;
    }
    let Some(guild_id) = require_guild_id(ctx, command).await else {
        return;
    };
    let Some(target) = option_user(&command.data.options, "membre") else {
        return;
    };
    let reason = option_str(&command.data.options, "reason").unwrap_or("Aucune raison spécifiée");
    let target_name = command
        .data
        .resolved
        .users
        .get(&target)
        .map(|u| u.name.clone())
        .unwrap_or_default();

    if let Some(gid) = command.guild_id {
        if let Err(msg) = super::check_hierarchy(ctx, command, gid, target) {
            reply_ephemeral(ctx, command, &format!("❌ {msg}")).await;
            return;
        }
    }

    match apply_sursis(
        ctx,
        &guild_id,
        target,
        &target_name,
        &command.user.id.to_string(),
        &command.user.name,
        reason,
    )
    .await
    {
        Some(applied) => {
            reply_ephemeral(
                ctx,
                command,
                &format!(
                    "⏳ <@{target}> est placé en sursis.{}",
                    applied
                        .channel
                        .map(|c| format!(" Salon d'appel : <#{c}>"))
                        .unwrap_or_default()
                ),
            )
            .await;
        }
        None => {
            reply_ephemeral(
                ctx,
                command,
                "Le rôle Sursis n'est pas configuré. Dashboard → Modération → « Rôle Sursis ».",
            )
            .await;
        }
    }
}

/// Resultat d'une mise en sursis appliquee.
pub struct SursisApplied {
    pub channel: Option<serenity::model::id::ChannelId>,
}

/// Met un membre en sursis (partage entre /ban-sursis et Automod). Renvoie
/// `None` si le role Sursis n'est pas configure (l'appelant peut alors retomber
/// sur un ban dur).
pub async fn apply_sursis(
    ctx: &Context,
    guild_id: &str,
    target: UserId,
    target_name: &str,
    moderator_id: &str,
    moderator_name: &str,
    reason: &str,
) -> Option<SursisApplied> {
    let gid_u64: u64 = guild_id.parse().ok()?;
    let guild = serenity::model::id::GuildId::new(gid_u64);

    // Config : role Sursis (requis).
    let (sursis_role, base, grpc) = {
        let data = ctx.data.read().await;
        let base = data.get::<ApiClientKey>().cloned()?;
        let grpc = data.get::<GrpcClientKey>().cloned()?;
        let cfg = base
            .get_guild_config_for(guild_id, crate::modules::moderation::MODULE_BOT_NAME)
            .await
            .unwrap_or_default();
        let role = cfg
            .get("sursis_role_id")
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|&n| n > 0);
        (role, base, grpc)
    };
    let mod_api = ApiClient::new(base, grpc);
    // Role Sursis non configure -> l'appelant peut retomber sur un ban dur.
    let sursis_role = RoleId::new(sursis_role?);

    // Membre + ses roles actuels (a sauvegarder).
    let mut member = match guild.member(&ctx.http, target).await {
        Ok(m) => m,
        Err(_) => return None,
    };
    let saved_roles: Vec<String> = member.roles.iter().map(|r| r.to_string()).collect();

    // Applique le sursis : on remplace tous les roles par le seul role Sursis.
    // (edit remplace les roles non-manages ; les roles manages sont ignores.)
    if let Err(e) = member
        .edit(&ctx.http, EditMember::new().roles(vec![sursis_role]))
        .await
    {
        warn!(error = %e, "Echec application du role Sursis (edit roles)");
        // Fallback : au moins ajouter le role Sursis.
        let _ = guild.member(&ctx.http, target).await.map(|m| async move {
            let _ = m.add_role(&ctx.http, sursis_role).await;
        });
    }

    // Salon d'appel (sous appeal_category_id), sans boutons pour l'instant.
    // Meme cadre « mode d'emploi » que l'appel, avec une note de sursis en tete.
    let context = format!(
        "⏳ **Ban en sursis** — sans appel accepté, le bannissement sera **automatique** à l'échéance.\n**Raison :** {reason}"
    );
    let intro =
        super::appeal::guidelines_embed(ctx, guild_id, target.get(), None, Some(&context)).await;
    let channel = crate::modules::moderation::create_appeal_channel(
        ctx,
        guild_id,
        target.get(),
        target_name,
        intro,
        vec![],
    )
    .await;

    // Enregistre le sursis (delai depuis la config cote API).
    let created = mod_api
        .create_sursis(CreateSursisParams {
            guild_id,
            user_id: &target.to_string(),
            username: target_name,
            moderator_id,
            moderator_name,
            reason,
            saved_roles,
            channel_id: channel.map(|c| c.to_string()),
        })
        .await
        .ok();
    let sursis_id = created.as_ref().map(|s| s.id.clone()).unwrap_or_default();
    let expires_at = created
        .as_ref()
        .map(|s| s.expires_at.clone())
        .unwrap_or_default();

    // Poste les boutons modo dans le salon (maintenant qu'on a l'id).
    if let (Some(channel), false) = (channel, sursis_id.is_empty()) {
        let row = serenity::all::CreateActionRow::Buttons(vec![
            CreateButton::new(format!("{SURSIS_PARDON_PREFIX}{sursis_id}"))
                .label("Gracier (accepter l'appel)")
                .emoji('♻')
                .style(ButtonStyle::Success),
            CreateButton::new(format!("{SURSIS_BAN_PREFIX}{sursis_id}"))
                .label("Bannir maintenant")
                .emoji('🔨')
                .style(ButtonStyle::Danger),
        ]);
        let panel = crate::shared::embeds::info_embed("Décision de modération")
            .description(
                "**Gracier** : rend ses rôles et lève le sursis.\n**Bannir maintenant** : ban immédiat.\nSinon, ban automatique à l'échéance.",
            )
            .footer(CreateEmbedFooter::new(
                if expires_at.is_empty() { "".to_string() } else { format!("Échéance : {expires_at}") },
            ));
        let _ = channel
            .send_message(
                &ctx.http,
                serenity::builder::CreateMessage::new()
                    .embed(panel)
                    .components(vec![row]),
            )
            .await;
    }

    // DM au membre.
    if let Ok(dm) = target.create_dm_channel(&ctx.http).await {
        let mut txt = format!(
            "Tu as été placé en **sursis** sur **{}**.\n**Raison :** {reason}\n\nTu peux contester dans le salon dédié avant le bannissement définitif.",
            guild
                .to_partial_guild(&ctx.http)
                .await
                .map(|g| g.name)
                .unwrap_or_else(|_| guild_id.to_string())
        );
        if let Some(c) = channel {
            txt.push_str(&format!("\n➡️ Ton salon d'appel : <#{c}>"));
        }
        let _ = dm
            .send_message(
                &ctx.http,
                serenity::builder::CreateMessage::new().content(txt),
            )
            .await;
    }

    info!(target = %target, by = moderator_name, "Ban en sursis applique");
    Some(SursisApplied { channel })
}

// ── Boutons ──

async fn get_sursis(mod_api: &ApiClient, id: &str) -> Option<SursisData> {
    mod_api.get_sursis(id).await.ok()
}

/// Bouton « Gracier » : restaure les roles + leve le sursis.
pub async fn handle_pardon(ctx: &Context, component: &ComponentInteraction) {
    if !super::appeal::ensure_moderator(ctx, component).await {
        return;
    }
    let Some(id) = component
        .data
        .custom_id
        .strip_prefix(SURSIS_PARDON_PREFIX)
        .map(str::to_string)
    else {
        return;
    };
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await;

    let mod_api = {
        let data = ctx.data.read().await;
        match (
            data.get::<ApiClientKey>().cloned(),
            data.get::<GrpcClientKey>().cloned(),
        ) {
            (Some(base), Some(grpc)) => ApiClient::new(base, grpc),
            _ => return,
        }
    };
    let Some(sursis) = get_sursis(&mod_api, &id).await else {
        followup(ctx, component, "Sursis introuvable.").await;
        return;
    };
    let Some(guild_id) = component.guild_id else {
        return;
    };
    let Ok(uid) = sursis.user_id.parse::<u64>() else {
        return;
    };

    // Claim d'abord : ne restaure les roles / n'agit que si CE clic a bien leve
    // le sursis (garde d'etat). Sinon (deja banni/gracie par une action
    // concurrente ou un double-clic), on s'abstient.
    let claimed = mod_api.resolve_sursis(&id, "gracie").await.unwrap_or(false);
    if !claimed {
        followup(ctx, component, "⚠️ Ce sursis a déjà été traité.").await;
        return;
    }
    // Restaure les roles sauvegardes.
    if let Ok(mut member) = guild_id.member(&ctx.http, UserId::new(uid)).await {
        let roles: Vec<RoleId> = sursis
            .saved_roles
            .iter()
            .filter_map(|r| r.parse::<u64>().ok().map(RoleId::new))
            .collect();
        let _ = member.edit(&ctx.http, EditMember::new().roles(roles)).await;
    }
    if let Ok(dm) = UserId::new(uid).create_dm_channel(&ctx.http).await {
        let _ = dm
            .send_message(
                &ctx.http,
                serenity::builder::CreateMessage::new()
                    .content("✅ Ton appel a été accepté : tes accès sont rétablis."),
            )
            .await;
    }
    followup(
        ctx,
        component,
        "♻️ Membre gracié : rôles restaurés, sursis levé. Salon supprimé…",
    )
    .await;
    let _ = component.channel_id.delete(&ctx.http).await;
    info!(sursis = %id, "Sursis gracie");
}

/// Bouton « Bannir maintenant » : ban immediat.
pub async fn handle_ban_now(ctx: &Context, component: &ComponentInteraction) {
    if !super::appeal::ensure_moderator(ctx, component).await {
        return;
    }
    let Some(id) = component
        .data
        .custom_id
        .strip_prefix(SURSIS_BAN_PREFIX)
        .map(str::to_string)
    else {
        return;
    };
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await;

    let mod_api = {
        let data = ctx.data.read().await;
        match (
            data.get::<ApiClientKey>().cloned(),
            data.get::<GrpcClientKey>().cloned(),
        ) {
            (Some(base), Some(grpc)) => ApiClient::new(base, grpc),
            _ => return,
        }
    };
    let Some(sursis) = get_sursis(&mod_api, &id).await else {
        followup(ctx, component, "Sursis introuvable.").await;
        return;
    };
    let Some(guild_id) = component.guild_id else {
        return;
    };
    let Ok(uid) = sursis.user_id.parse::<u64>() else {
        return;
    };

    // Claim d'abord : ne bannit que si CE clic a bien resolu le sursis (garde
    // d'etat -> pas de re-ban sur double-clic ni si deja gracie).
    let claimed = mod_api.resolve_sursis(&id, "banni").await.unwrap_or(false);
    if !claimed {
        followup(ctx, component, "⚠️ Ce sursis a déjà été traité.").await;
        return;
    }
    if let Err(e) = guild_id
        .ban_with_reason(&ctx.http, UserId::new(uid), 0, "Ban en sursis confirmé")
        .await
    {
        warn!(error = %e, "Echec ban depuis sursis");
    }
    followup(ctx, component, "🔨 Membre banni. Salon supprimé…").await;
    let _ = component.channel_id.delete(&ctx.http).await;
    info!(sursis = %id, "Sursis -> ban immediat");
}

async fn followup(ctx: &Context, component: &ComponentInteraction, msg: &str) {
    let _ = component
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new()
                .content(msg)
                .ephemeral(true),
        )
        .await;
}
