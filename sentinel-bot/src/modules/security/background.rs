//! Background tasks du module security : slowmode revert / lockdown
//! revert. Phase 5F — la quarantine kick a ete deplacee dans
//! sentinel-worker (`security::kick_expired_quarantine`) ; le bot la
//! consume via `quarantine_expired_consumer.rs`.

use serenity::all::Context;

use super::{LockdownKey, SecurityConfigKey, SlowmodeKey};

/// Spawn les background tasks security restantes : slowmode + lockdown
/// revert (encore en RAM tant que les `PermissionOverwrite` originaux
/// ne sont pas persistes en DB).
pub fn spawn_background(ctx: Context) {
    // 2. Slowmode revert (15s loop)
    let ctx_s = ctx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;

            let data = ctx_s.data.read().await;
            let Some(slowmode) = data.get::<SlowmodeKey>() else {
                continue;
            };
            let duration = data
                .get::<SecurityConfigKey>()
                .map(|c| c.slowmode_duration_secs)
                .unwrap_or(300);

            let expired = slowmode.expired_guilds(duration);
            for guild_id in expired {
                slowmode.deactivate_with_http(&ctx_s.http, guild_id).await;
            }
        }
    });

    // 3. Lockdown revert (15s loop)
    let ctx_l = ctx;
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;

            let data = ctx_l.data.read().await;
            let Some(lockdown) = data.get::<LockdownKey>() else {
                continue;
            };
            let duration = data
                .get::<SecurityConfigKey>()
                .map(|c| c.lockdown_duration_secs)
                .unwrap_or(600);

            let expired = lockdown.expired_guilds(duration);
            for guild_id in expired {
                lockdown.deactivate_with_http(&ctx_l.http, guild_id).await;
            }
        }
    });
}
