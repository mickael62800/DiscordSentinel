//! Consumer de l'event Redis `guild_reset` (factory reset d'un serveur).
//! Apres que l'API a efface les donnees du serveur, le bot annule l'etat
//! Discord : deban de tous les bannis, levee des timeouts, retrait des roles
//! temporaires / quarantaine. Best-effort, en tache detachee (rate-limit gere
//! par serenity).

use serenity::model::id::{GuildId, RoleId};
use serenity::prelude::*;
use tracing::{info, warn};

pub async fn handle_guild_reset_event(ctx: &Context, payload: &str) {
    let event: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return,
    };
    if event.get("event").and_then(|e| e.as_str()) != Some("guild_reset") {
        return;
    }
    let data = match event.get("data") {
        Some(d) => d,
        None => return,
    };

    let gid = match data
        .get("guild_id")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok())
    {
        Some(v) => GuildId::new(v),
        None => return,
    };
    let unban = data.get("unban").and_then(|v| v.as_bool()).unwrap_or(false);
    let unmute = data
        .get("unmute")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let remove_roles = data
        .get("remove_roles")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Verification de la signature HMAC (secret = API_KEY partage bot<->api).
    // En prod (secret non vide) un event guild_reset non signe ou mal signe est
    // REJETE -> impossible de forcer un reset destructif en publiant sur Redis
    // sans le secret. En dev (API_KEY vide) la signature n'est pas exigee.
    let secret = std::env::var("SENTINEL_API_KEY").unwrap_or_default();
    if !secret.is_empty() {
        let guild_id_str = data
            .get("guild_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let expected = sign_guild_reset(&secret, guild_id_str, unban, unmute, remove_roles);
        let got = data.get("sig").and_then(|v| v.as_str()).unwrap_or_default();
        if got.is_empty() || got != expected {
            warn!(guild = %gid, "guild_reset: signature invalide ou absente -> event REJETE");
            return;
        }
    }

    let mut role_ids: Vec<RoleId> = Vec::new();
    if let Some(q) = data
        .get("quarantine_role_id")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok())
    {
        role_ids.push(RoleId::new(q));
    }
    if let Some(arr) = data.get("temp_role_ids").and_then(|v| v.as_array()) {
        for r in arr {
            if let Some(id) = r.as_str().and_then(|s| s.parse::<u64>().ok()) {
                role_ids.push(RoleId::new(id));
            }
        }
    }

    // Travail lourd en tache detachee : ne bloque pas le consumer (ACK rapide).
    let http = ctx.http.clone();
    tokio::spawn(async move {
        info!(guild = %gid, unban, unmute, remove_roles, "guild_reset : annulation de l'etat Discord");

        if unban {
            match gid.bans(&http, None, None).await {
                Ok(bans) => {
                    let n = bans.len();
                    for b in bans {
                        if let Err(e) = gid.unban(&http, b.user.id).await {
                            warn!(error = %e, user = %b.user.id, "guild_reset: echec unban");
                        }
                    }
                    info!(guild = %gid, count = n, "guild_reset: debans traites");
                }
                Err(e) => warn!(error = %e, "guild_reset: echec recuperation des bans"),
            }
        }

        if unmute || remove_roles {
            // Parcourt les membres (best-effort, jusqu'a 1000).
            match gid.members(&http, Some(1000), None).await {
                Ok(members) => {
                    for mut m in members {
                        if unmute && m.communication_disabled_until.is_some() {
                            if let Err(e) = m.enable_communication(&http).await {
                                warn!(error = %e, user = %m.user.id, "guild_reset: echec levee timeout");
                            }
                        }
                        if remove_roles {
                            for role in &role_ids {
                                if m.roles.contains(role) {
                                    if let Err(e) = m.remove_role(&http, *role).await {
                                        warn!(error = %e, user = %m.user.id, role = %role, "guild_reset: echec retrait role");
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => warn!(error = %e, "guild_reset: echec liste des membres"),
            }
        }

        info!(guild = %gid, "guild_reset : annulation Discord terminee");
    });
}

/// Signature HMAC-SHA256 d'un event `guild_reset` (meme format canonique que
/// cote API). Secret vide -> signature vide.
fn sign_guild_reset(
    secret: &str,
    guild_id: &str,
    unban: bool,
    unmute: bool,
    remove_roles: bool,
) -> String {
    if secret.is_empty() {
        return String::new();
    }
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let msg = format!("guild_reset:{guild_id}:{unban}:{unmute}:{remove_roles}");
    let mut mac = <Hmac<Sha256>>::new_from_slice(secret.as_bytes()).expect("cle HMAC");
    mac.update(msg.as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
