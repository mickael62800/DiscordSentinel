//! Background tasks du module security : captcha timeout / slowmode revert / lockdown revert.

use serenity::all::Context;

use super::{CaptchaPendingKey, LockdownKey, QuarantineKey, SecurityConfigKey, SlowmodeKey};

/// Spawn les background tasks security : captcha timeout / slowmode revert / lockdown revert.
pub fn spawn_background(ctx: Context) {
    // 1. Captcha timeout + quarantine kick (30s loop)
    let ctx_q = ctx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;

            let data = ctx_q.data.read().await;
            let Some(quarantine) = data.get::<QuarantineKey>() else { continue };
            let captcha_timeout = data
                .get::<SecurityConfigKey>()
                .map(|c| c.captcha_timeout_secs)
                .unwrap_or(300);

            if let Some(cp) = data.get::<CaptchaPendingKey>() {
                cp.cleanup_expired();
            }

            let expired = quarantine.expired_users(captcha_timeout);
            for (guild_id, user_id) in expired {
                if let Err(e) = guild_id.kick(&ctx_q.http, user_id).await {
                    tracing::warn!(
                        error = %e,
                        guild_id = %guild_id,
                        user_id = %user_id,
                        "Impossible de kick l'utilisateur (captcha timeout)"
                    );
                } else {
                    tracing::info!(
                        guild_id = %guild_id,
                        user_id = %user_id,
                        "Utilisateur kick (captcha timeout)"
                    );
                }
                quarantine.remove_tracking(guild_id, user_id);
                if let Some(cp) = data.get::<CaptchaPendingKey>() {
                    cp.remove(guild_id, user_id);
                }
            }
        }
    });

    // 2. Slowmode revert (15s loop)
    let ctx_s = ctx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;

            let data = ctx_s.data.read().await;
            let Some(slowmode) = data.get::<SlowmodeKey>() else { continue };
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
            let Some(lockdown) = data.get::<LockdownKey>() else { continue };
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
