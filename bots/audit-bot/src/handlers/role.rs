use serenity::model::guild::Role;
use serenity::model::id::{GuildId, RoleId};
use serenity::prelude::*;

use crate::audit_event;
use crate::handler::{AnomalyDetectorKey, Handler, WeeklyTrackerKey};
use crate::permission_diff;
use crate::weekly_report::StatField;

pub async fn handle_create(ctx: &Context, new: &Role) {
    let gid = new.guild_id;
    let gid_str = gid.to_string();

    Handler::log(ctx, "info", &gid_str, &format!(
        "Role cree : @{} (couleur: #{:06x}, permissions: {})", new.name, new.colour.0, new.permissions.bits()
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid_str, "role_create")
            .with_target(&new.id, &new.name)
            .with_details(serde_json::json!({
                "colour": format!("#{:06x}", new.colour.0),
                "permissions": new.permissions.bits().to_string(),
                "position": new.position,
                "mentionable": new.mentionable,
                "hoist": new.hoist,
            })),
    )
    .await;

    let data = ctx.data.read().await;
    if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
        tracker.increment(gid, StatField::RoleChange);
    }
}

pub async fn handle_delete(
    ctx: &Context,
    guild_id: GuildId,
    removed_role_id: RoleId,
    removed_role: Option<Role>,
) {
    let gid_str = guild_id.to_string();
    let role_name = removed_role
        .as_ref()
        .map(|r| r.name.clone())
        .unwrap_or_else(|| removed_role_id.to_string());

    Handler::log(ctx, "warn", &gid_str, &format!(
        "Role supprime : @{}", role_name
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid_str, "role_delete")
            .with_target(removed_role_id, &role_name),
    )
    .await;

    let data = ctx.data.read().await;
    if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
        tracker.increment(guild_id, StatField::RoleChange);
    }
}

pub async fn handle_update(ctx: &Context, old: Option<Role>, new: &Role) {
    let gid = new.guild_id;
    let gid_str = gid.to_string();

    let mut changes = Vec::new();
    let mut perm_diff_text = String::new();

    if let Some(ref old_role) = old {
        if old_role.name != new.name {
            changes.push(format!("nom: {} -> {}", old_role.name, new.name));
        }
        if old_role.colour != new.colour {
            changes.push(format!("couleur: #{:06x} -> #{:06x}", old_role.colour.0, new.colour.0));
        }
        if old_role.permissions != new.permissions {
            let diff = permission_diff::diff_permissions(old_role.permissions, new.permissions);
            perm_diff_text = permission_diff::format_diff(&diff);
            changes.push(format!("permissions:\n{}", perm_diff_text));
        }
        if old_role.hoist != new.hoist {
            changes.push(format!("affiche separement: {} -> {}", old_role.hoist, new.hoist));
        }
        if old_role.mentionable != new.mentionable {
            changes.push(format!("mentionnable: {} -> {}", old_role.mentionable, new.mentionable));
        }
    }

    if changes.is_empty() {
        return;
    }

    Handler::log(ctx, "info", &gid_str, &format!(
        "Role modifie : @{} — {}", new.name, changes.join(", ")
    )).await;

    let mut details = serde_json::json!({
        "changes": changes,
        "old_permissions": old.as_ref().map(|r| r.permissions.bits().to_string()),
        "new_permissions": new.permissions.bits().to_string(),
    });

    if !perm_diff_text.is_empty() {
        details["permission_diff"] = serde_json::json!(perm_diff_text);
    }

    Handler::send_event(
        ctx,
        audit_event::simple(gid_str.clone(), "role_update")
            .with_target(&new.id, &new.name)
            .with_details(details),
    )
    .await;

    // Anomaly detection
    let data = ctx.data.read().await;
    if let Some(anomaly) = data.get::<AnomalyDetectorKey>() {
        if let Some(alert) = anomaly.record(gid, "role_change") {
            Handler::log(
                ctx,
                "error",
                &gid_str,
                &format!("ANOMALIE : {} ({} en {}s)", alert.anomaly_type, alert.count, alert.window_secs),
            ).await;

            Handler::send_event(
                ctx,
                audit_event::simple(gid_str, "anomaly_detected")
                    .with_details(serde_json::json!({
                        "anomaly_type": alert.anomaly_type,
                        "count": alert.count,
                        "window_secs": alert.window_secs,
                    })),
            ).await;

            if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
                tracker.increment(gid, StatField::Anomaly);
            }
        }
    }

    if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
        tracker.increment(gid, StatField::RoleChange);
    }
}
