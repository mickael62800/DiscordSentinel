use serenity::model::guild::Member;
use serenity::model::id::GuildId;
use serenity::model::user::User;
use serenity::prelude::*;

use crate::audit_event;
use crate::handler::Handler;

pub async fn handle_addition(ctx: &Context, new_member: &Member) {
    let gid = new_member.guild_id.to_string();

    Handler::log(ctx, "info", &gid, &format!(
        "Nouveau membre : {} ({}) — compte cree le {}",
        new_member.user.name, new_member.user.id, new_member.user.created_at()
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid, "member_join")
            .with_target(&new_member.user.id, &new_member.user.name)
            .with_details(serde_json::json!({
                "account_created_at": new_member.user.created_at().to_string(),
            })),
    )
    .await;
}

pub async fn handle_removal(ctx: &Context, guild_id: GuildId, user: &User) {
    let gid = guild_id.to_string();

    Handler::log(ctx, "warn", &gid, &format!(
        "Membre parti : {} ({})", user.name, user.id
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid, "member_leave")
            .with_target(&user.id, &user.name),
    )
    .await;
}

pub async fn handle_ban_addition(ctx: &Context, guild_id: GuildId, banned_user: &User) {
    let gid = guild_id.to_string();

    Handler::log(ctx, "error", &gid, &format!(
        "Membre banni : {} ({})", banned_user.name, banned_user.id
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid, "member_ban")
            .with_target(&banned_user.id, &banned_user.name),
    )
    .await;
}

pub async fn handle_ban_removal(ctx: &Context, guild_id: GuildId, unbanned_user: &User) {
    let gid = guild_id.to_string();

    Handler::log(ctx, "info", &gid, &format!(
        "Membre debanni : {} ({})", unbanned_user.name, unbanned_user.id
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid, "member_unban")
            .with_target(&unbanned_user.id, &unbanned_user.name),
    )
    .await;
}

pub async fn handle_update(
    ctx: &Context,
    old: Option<Member>,
    new_member: &Member,
) {
    let gid = new_member.guild_id.to_string();
    let user_name = &new_member.user.name;
    let user_id = new_member.user.id.to_string();

    // Changement de pseudo (nickname)
    let old_nick = old.as_ref().and_then(|m| m.nick.clone());
    let new_nick = new_member.nick.clone();
    if old_nick != new_nick {
        let old_label = old_nick.as_deref().unwrap_or("(aucun)");
        let new_label = new_nick.as_deref().unwrap_or("(aucun)");
        Handler::log(ctx, "info", &gid, &format!(
            "{} a change de pseudo : {} -> {}", user_name, old_label, new_label
        )).await;

        Handler::send_event(
            ctx,
            audit_event::simple(gid.clone(), "member_nickname_update")
                .with_target(&user_id, user_name)
                .with_details(serde_json::json!({
                    "old_nickname": old_label,
                    "new_nickname": new_label,
                })),
        )
        .await;
    }

    // Changement d'avatar serveur
    let old_avatar = old.as_ref().and_then(|m| m.avatar.map(|a| a.to_string()));
    let new_avatar = new_member.avatar.map(|a| a.to_string());
    if old_avatar != new_avatar {
        Handler::log(ctx, "info", &gid, &format!(
            "{} a change son avatar serveur", user_name
        )).await;

        Handler::send_event(
            ctx,
            audit_event::simple(gid.clone(), "member_avatar_update")
                .with_target(&user_id, user_name),
        )
        .await;
    }

    // Changement de roles
    let old_roles: Vec<String> = old
        .as_ref()
        .map(|m| m.roles.iter().map(|r| r.to_string()).collect())
        .unwrap_or_default();
    let new_roles: Vec<String> = new_member.roles.iter().map(|r| r.to_string()).collect();

    if old_roles != new_roles {
        Handler::log(ctx, "info", &gid, &format!(
            "{} — roles modifies", user_name
        )).await;

        Handler::send_event(
            ctx,
            audit_event::simple(gid.clone(), "member_roles_update")
                .with_target(&user_id, user_name)
                .with_details(serde_json::json!({
                    "old_roles": old_roles,
                    "new_roles": new_roles,
                })),
        )
        .await;
    }

    // Timeout (mute) detecte
    let old_timeout = old.as_ref().and_then(|m| m.communication_disabled_until);
    let new_timeout = new_member.communication_disabled_until;
    if old_timeout.is_none() && new_timeout.is_some() {
        Handler::log(ctx, "warn", &gid, &format!(
            "{} a ete mute (timeout jusqu'a {})", user_name, new_timeout.unwrap()
        )).await;

        Handler::send_event(
            ctx,
            audit_event::simple(gid.clone(), "member_timeout")
                .with_target(&user_id, user_name)
                .with_details(serde_json::json!({
                    "timeout_until": new_timeout.unwrap().to_string(),
                })),
        )
        .await;
    } else if old_timeout.is_some() && new_timeout.is_none() {
        Handler::log(ctx, "info", &gid, &format!(
            "{} n'est plus mute (timeout leve)", user_name
        )).await;

        Handler::send_event(
            ctx,
            audit_event::simple(gid, "member_timeout_removed")
                .with_target(&user_id, user_name),
        )
        .await;
    }
}
