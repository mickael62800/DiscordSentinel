use serenity::model::guild::Role;
use serenity::model::id::{GuildId, RoleId};
use serenity::prelude::*;

use crate::audit_event;
use crate::handler::Handler;

pub async fn handle_create(ctx: &Context, new: &Role) {
    let gid = new.guild_id.to_string();

    Handler::log(ctx, "info", &gid, &format!(
        "Role cree : @{} (couleur: #{:06x}, permissions: {})", new.name, new.colour.0, new.permissions.bits()
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid, "role_create")
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
}

pub async fn handle_delete(
    ctx: &Context,
    guild_id: GuildId,
    removed_role_id: RoleId,
    removed_role: Option<Role>,
) {
    let gid = guild_id.to_string();
    let role_name = removed_role
        .as_ref()
        .map(|r| r.name.clone())
        .unwrap_or_else(|| removed_role_id.to_string());

    Handler::log(ctx, "warn", &gid, &format!(
        "Role supprime : @{}", role_name
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid, "role_delete")
            .with_target(removed_role_id, &role_name),
    )
    .await;
}

pub async fn handle_update(ctx: &Context, old: Option<Role>, new: &Role) {
    let gid = new.guild_id.to_string();

    let mut changes = Vec::new();
    if let Some(ref old_role) = old {
        if old_role.name != new.name {
            changes.push(format!("nom: {} -> {}", old_role.name, new.name));
        }
        if old_role.colour != new.colour {
            changes.push(format!("couleur: #{:06x} -> #{:06x}", old_role.colour.0, new.colour.0));
        }
        if old_role.permissions != new.permissions {
            changes.push("permissions modifiees".to_string());
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

    Handler::log(ctx, "info", &gid, &format!(
        "Role modifie : @{} — {}", new.name, changes.join(", ")
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid, "role_update")
            .with_target(&new.id, &new.name)
            .with_details(serde_json::json!({
                "changes": changes,
                "old_permissions": old.as_ref().map(|r| r.permissions.bits().to_string()),
                "new_permissions": new.permissions.bits().to_string(),
            })),
    )
    .await;
}
