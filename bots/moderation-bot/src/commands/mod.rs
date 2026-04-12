pub mod appeal;
pub mod ban;
pub mod call;
pub mod compare;
pub mod context;
pub mod evidence;
pub mod expirations;
pub mod export;
pub mod history;
pub mod mass;
pub mod modstats;
pub mod mute;
pub mod notes;
pub mod review;
pub mod template;
pub mod transcript;
pub mod unwarn;
pub mod warn;

use serenity::all::{ChannelId, Context, CreateEmbed, CreateEmbedFooter, CreateMessage, GuildId, UserId};
use serenity::builder::CreateCommand;
use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::heartbeat::ApiClientKey;

/// Envoie un embed de log dans le salon de logs configure pour la guild.
/// Si `log_channel_id` n'est pas configure, ne fait rien.
pub async fn log_to_channel(ctx: &Context, guild_id: &str, embed: CreateEmbed) {
    let log_channel_id = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let config = base.get_guild_config(guild_id).await.unwrap_or_default();
            config
                .get("log_channel_id")
                .and_then(|v| v.parse::<u64>().ok())
        } else {
            None
        }
    };

    let channel = match log_channel_id {
        Some(id) if id > 0 => ChannelId::new(id),
        _ => return,
    };

    if let Err(e) = channel
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await
    {
        tracing::warn!(error = %e, "Echec envoi log dans le salon de logs moderation");
    }
}

/// Verifie si l'utilisateur cible est immunise contre les sanctions de
/// moderation. Lit la config guild `ignored_roles` (IDs separes par des
/// virgules) et compare avec les roles du membre.
///
/// Retourne `Some(role_id)` si le membre possede un role immunise,
/// `None` sinon (y compris en cas d'erreur API/cache — fail-open car un
/// faux negatif ici empeche tout juste d'appliquer une protection, pas
/// de sanctionner quelqu'un qui ne devrait pas l'etre).
pub async fn find_immune_role(
    ctx: &Context,
    guild_id: GuildId,
    target_user_id: UserId,
) -> Option<u64> {
    // 1. Lire ignored_roles depuis la config
    let ignored_roles_raw = {
        let data = ctx.data.read().await;
        let base = data.get::<ApiClientKey>()?;
        let config = base.get_guild_config(&guild_id.to_string()).await.ok()?;
        config.get("ignored_roles").cloned()
    };

    let ignored_roles_str = ignored_roles_raw?;
    if ignored_roles_str.trim().is_empty() {
        return None;
    }

    // 2. Parser les IDs (CSV tolerant)
    let ignored_ids: Vec<u64> = ignored_roles_str
        .split([',', ' ', '\n'])
        .filter_map(|s| s.trim().parse::<u64>().ok())
        .collect();
    if ignored_ids.is_empty() {
        return None;
    }

    // 3. Fetch le membre + intersection avec ses roles
    match guild_id.member(&ctx.http, target_user_id).await {
        Ok(member) => {
            for role in &member.roles {
                let rid = role.get();
                if ignored_ids.contains(&rid) {
                    return Some(rid);
                }
            }
            None
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                guild_id = %guild_id,
                target = %target_user_id,
                "Impossible de fetch le membre pour verifier l'immunite (fail-open)"
            );
            None
        }
    }
}

/// Helper : retourne un message user-friendly pour signaler qu'un user est
/// immunise. Utilise par les commandes avant de sanctionner.
pub fn immunity_message(role_id: u64, action_label: &str) -> String {
    format!(
        "🛡️ Ce membre est **immunise** contre les sanctions (role <@&{}>).\nImpossible d'appliquer : **{}**.",
        role_id, action_label
    )
}

pub fn all() -> Vec<CreateCommand> {
    vec![
        warn::register(),
        mute::register(),
        mute::register_unmute(),
        ban::register(),
        ban::register_unban(),
        history::register(),
        notes::register(),
        call::register(),
        context::register(),
        appeal::register(),
        export::register(),
        expirations::register(),
        compare::register(),
        modstats::register(),
        evidence::register(),
        review::register(),
        template::register(),
        transcript::register(),
        mass::register_massmute(),
        mass::register_massban(),
        unwarn::register(),
    ]
}
