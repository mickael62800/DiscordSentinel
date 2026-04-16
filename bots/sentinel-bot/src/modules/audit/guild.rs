use serenity::model::guild::Guild;
use serenity::prelude::*;

use super::audit_event;
use super::handler_impl::Handler;

pub async fn handle_update(
    ctx: &Context,
    old: Option<Guild>,
    new_incomplete: &serenity::model::guild::PartialGuild,
) {
    let gid = new_incomplete.id.to_string();

    let mut changes = Vec::new();
    if let Some(ref old_guild) = old {
        if old_guild.name != new_incomplete.name {
            changes.push(format!("nom: {} -> {}", old_guild.name, new_incomplete.name));
        }
        if old_guild.icon != new_incomplete.icon {
            changes.push("icone modifiee".to_string());
        }
        if old_guild.verification_level != new_incomplete.verification_level {
            changes.push(format!("niveau verification: {:?} -> {:?}", old_guild.verification_level, new_incomplete.verification_level));
        }
        if old_guild.default_message_notifications != new_incomplete.default_message_notifications {
            changes.push("notifications par defaut modifiees".to_string());
        }
    }

    if changes.is_empty() {
        return;
    }

    Handler::log(ctx, "warn", &gid, &format!(
        "Serveur modifie : {}", changes.join(", ")
    )).await;

    let mut evt = audit_event::simple(gid, "guild_update")
        .with_details(serde_json::json!({
            "changes": changes,
        }));
    evt.target_name = Some(new_incomplete.name.clone());

    Handler::send_event(ctx, evt).await;
}
