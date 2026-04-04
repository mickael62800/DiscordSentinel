use serenity::model::guild::Member;
use serenity::model::id::GuildId;
use serenity::model::user::User;
use serenity::prelude::*;

use sentinel_shared::heartbeat::ApiClientKey;

use crate::audit_event;
use crate::handler::{AnomalyDetectorKey, Handler, WeeklyTrackerKey};
use crate::weekly_report::StatField;

pub async fn handle_addition(ctx: &Context, new_member: &Member) {
    let gid = new_member.guild_id;
    let gid_str = gid.to_string();

    Handler::log(ctx, "info", &gid_str, &format!(
        "Nouveau membre : {} ({}) — compte cree le {}",
        new_member.user.name, new_member.user.id, new_member.user.created_at()
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid_str.clone(), "member_join")
            .with_target(&new_member.user.id, &new_member.user.name)
            .with_details(serde_json::json!({
                "account_created_at": new_member.user.created_at().to_string(),
            })),
    )
    .await;

    // Surveillance
    Handler::track_activity(
        ctx, &gid_str, &new_member.user.id.to_string(), "member_join",
        None, None, None,
        serde_json::json!({"account_created_at": new_member.user.created_at().to_string()}),
    ).await;

    let data = ctx.data.read().await;
    if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
        tracker.increment(gid, StatField::MemberJoin);
    }
}

pub async fn handle_removal(ctx: &Context, guild_id: GuildId, user: &User) {
    let gid_str = guild_id.to_string();

    Handler::log(ctx, "warn", &gid_str, &format!(
        "Membre parti : {} ({})", user.name, user.id
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid_str.clone(), "member_leave")
            .with_target(&user.id, &user.name),
    )
    .await;

    // Surveillance
    Handler::track_activity(
        ctx, &gid_str, &user.id.to_string(), "member_leave",
        None, None, None, serde_json::json!({}),
    ).await;

    // Anomaly detection (kick pattern)
    let data = ctx.data.read().await;
    if let Some(anomaly) = data.get::<AnomalyDetectorKey>() {
        if let Some(alert) = anomaly.record(guild_id, "kick") {
            Handler::log(
                ctx,
                "error",
                &gid_str,
                &format!("ANOMALIE : {} ({} en {}s)", alert.anomaly_type, alert.count, alert.window_secs),
            ).await;

            Handler::send_event(
                ctx,
                audit_event::simple(gid_str.clone(), "anomaly_detected")
                    .with_details(serde_json::json!({
                        "anomaly_type": alert.anomaly_type,
                        "count": alert.count,
                        "window_secs": alert.window_secs,
                    })),
            ).await;

            if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
                tracker.increment(guild_id, StatField::Anomaly);
            }
        }
    }

    if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
        tracker.increment(guild_id, StatField::MemberLeave);
    }
}

pub async fn handle_ban_addition(ctx: &Context, guild_id: GuildId, banned_user: &User) {
    let gid_str = guild_id.to_string();

    Handler::log(ctx, "error", &gid_str, &format!(
        "Membre banni : {} ({})", banned_user.name, banned_user.id
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid_str.clone(), "member_ban")
            .with_target(&banned_user.id, &banned_user.name),
    )
    .await;

    // Anomaly detection
    let data = ctx.data.read().await;
    if let Some(anomaly) = data.get::<AnomalyDetectorKey>() {
        if let Some(alert) = anomaly.record(guild_id, "ban") {
            Handler::log(
                ctx,
                "error",
                &gid_str,
                &format!("ANOMALIE : {} ({} en {}s)", alert.anomaly_type, alert.count, alert.window_secs),
            ).await;

            Handler::send_event(
                ctx,
                audit_event::simple(gid_str.clone(), "anomaly_detected")
                    .with_details(serde_json::json!({
                        "anomaly_type": alert.anomaly_type,
                        "count": alert.count,
                        "window_secs": alert.window_secs,
                    })),
            ).await;

            if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
                tracker.increment(guild_id, StatField::Anomaly);
            }
        }
    }

    if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
        tracker.increment(guild_id, StatField::Ban);
    }
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
    let gid = new_member.guild_id;
    let gid_str = gid.to_string();
    let user_name = &new_member.user.name;
    let user_id = new_member.user.id.to_string();

    // Changement de pseudo (nickname)
    let old_nick = old.as_ref().and_then(|m| m.nick.clone());
    let new_nick = new_member.nick.clone();
    if old_nick != new_nick {
        let old_label = old_nick.as_deref().unwrap_or("(aucun)");
        let new_label = new_nick.as_deref().unwrap_or("(aucun)");
        Handler::log(ctx, "info", &gid_str, &format!(
            "{} a change de pseudo : {} -> {}", user_name, old_label, new_label
        )).await;

        Handler::send_event(
            ctx,
            audit_event::simple(gid_str.clone(), "member_nickname_update")
                .with_target(&user_id, user_name)
                .with_details(serde_json::json!({
                    "old_nickname": old_label,
                    "new_nickname": new_label,
                })),
        )
        .await;

        // Surveillance : pseudo change
        Handler::track_activity(
            ctx, &gid_str, &user_id, "nickname_changed",
            None, None,
            Some(&format!("{} -> {}", old_label, new_label)),
            serde_json::json!({"old": old_label, "new": new_label}),
        ).await;

        // Envoyer l'historique pseudos au backend
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let req = base
                .client()
                .post(format!("{}/api/name-history", base.base_url()))
                .json(&serde_json::json!({
                    "guild_id": gid_str,
                    "user_id": user_id,
                    "old_name": old_label,
                    "new_name": new_label,
                }));
            base.auth(req).send().await.ok();
        }
    }

    // Changement d'avatar serveur
    let old_avatar = old.as_ref().and_then(|m| m.avatar.map(|a| a.to_string()));
    let new_avatar = new_member.avatar.map(|a| a.to_string());
    if old_avatar != new_avatar {
        // Construire les URLs d'avatar
        let old_avatar_url = old.as_ref().and_then(|m| {
            m.avatar.map(|hash| {
                let ext = if hash.is_animated() { "gif" } else { "png" };
                format!("https://cdn.discordapp.com/guilds/{}/users/{}/avatars/{}.{}?size=128", gid, m.user.id, hash, ext)
            })
        });
        let new_avatar_url = new_member.avatar.map(|hash| {
            let ext = if hash.is_animated() { "gif" } else { "png" };
            format!("https://cdn.discordapp.com/guilds/{}/users/{}/avatars/{}.{}?size=128", gid, new_member.user.id, hash, ext)
        });
        // Fallback sur l'avatar global si pas d'avatar serveur
        let new_url = new_avatar_url.unwrap_or_else(|| {
            new_member.user.avatar.map(|hash| {
                let ext = if hash.is_animated() { "gif" } else { "png" };
                format!("https://cdn.discordapp.com/avatars/{}/{}.{}?size=128", new_member.user.id, hash, ext)
            }).unwrap_or_default()
        });

        Handler::log(ctx, "info", &gid_str, &format!(
            "{} a change son avatar serveur", user_name
        )).await;

        Handler::send_event(
            ctx,
            audit_event::simple(gid_str.clone(), "member_avatar_update")
                .with_target(&user_id, user_name)
                .with_details(serde_json::json!({
                    "old_avatar_url": old_avatar_url,
                    "new_avatar_url": new_url,
                })),
        )
        .await;

        // Surveillance : avatar change
        Handler::track_activity(
            ctx, &gid_str, &user_id, "avatar_changed",
            None, None, None,
            serde_json::json!({"new_avatar_url": new_url}),
        ).await;
    }

    // Changement de roles
    let old_roles: Vec<String> = old
        .as_ref()
        .map(|m| m.roles.iter().map(|r| r.to_string()).collect())
        .unwrap_or_default();
    let new_roles: Vec<String> = new_member.roles.iter().map(|r| r.to_string()).collect();

    if old_roles != new_roles {
        Handler::log(ctx, "info", &gid_str, &format!(
            "{} — roles modifies", user_name
        )).await;

        Handler::send_event(
            ctx,
            audit_event::simple(gid_str.clone(), "member_roles_update")
                .with_target(&user_id, user_name)
                .with_details(serde_json::json!({
                    "old_roles": old_roles,
                    "new_roles": new_roles,
                })),
        )
        .await;

        // Surveillance : roles changes
        Handler::track_activity(
            ctx, &gid_str, &user_id, "roles_changed",
            None, None, None,
            serde_json::json!({"old_roles": old_roles, "new_roles": new_roles}),
        ).await;

        // Weekly stats
        let data = ctx.data.read().await;
        if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
            tracker.increment(gid, StatField::RoleChange);
        }
    }

    // Timeout (mute) detecte
    let old_timeout = old.as_ref().and_then(|m| m.communication_disabled_until);
    let new_timeout = new_member.communication_disabled_until;
    if old_timeout.is_none() && new_timeout.is_some() {
        Handler::log(ctx, "warn", &gid_str, &format!(
            "{} a ete mute (timeout jusqu'a {})", user_name, new_timeout.unwrap()
        )).await;

        Handler::send_event(
            ctx,
            audit_event::simple(gid_str.clone(), "member_timeout")
                .with_target(&user_id, user_name)
                .with_details(serde_json::json!({
                    "timeout_until": new_timeout.unwrap().to_string(),
                })),
        )
        .await;
    } else if old_timeout.is_some() && new_timeout.is_none() {
        Handler::log(ctx, "info", &gid_str, &format!(
            "{} n'est plus mute (timeout leve)", user_name
        )).await;

        Handler::send_event(
            ctx,
            audit_event::simple(gid_str, "member_timeout_removed")
                .with_target(&user_id, user_name),
        )
        .await;
    }
}
