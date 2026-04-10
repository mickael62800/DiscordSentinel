//! Phase 6A — Cleanup des tables blackjack inactives.
//!
//! Avant Phase 6A, blackjack-bot avait une boucle interne `sleep(60s)` qui
//! lisait son `ChannelManager` in-memory (DashMap) pour fermer les tables
//! sans activite depuis 30 min. Ce pattern ne scale pas horizontalement
//! (chaque replica tracking son propre sous-ensemble de tables).
//!
//! Le worker exploite la colonne `blackjack_tables.last_activity` (deja
//! mise a jour par l'API a chaque action) :
//!   1. Query les tables `status='open' AND last_activity < NOW() - 30 min`
//!   2. UPDATE `status='closed'` en batch (source de verite DB)
//!   3. Publie un event `blackjack_table_afk` sur la stream `sentinel:events`
//!      (Phase 5B) — le blackjack-bot consume et fait le DELETE du channel
//!      Discord local + retire de son ChannelManager
//!
//! Idempotence : l'UPDATE `WHERE status = 'open'` empeche le double-traitement
//! si plusieurs workers tournent. Le bot peut aussi recevoir l'event
//! plusieurs fois (at-least-once streams) — le DELETE channel est idempotent
//! (404 si deja supprime).

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::DEFAULT_AFK_TIMEOUT_SECS;

const STREAM_KEY: &str = "sentinel:events";
const STREAM_MAXLEN: usize = 10_000;
const PAYLOAD_FIELD: &str = "payload";

#[derive(Debug, sqlx::FromRow)]
struct AfkTable {
    id: Uuid,
    guild_id: String,
    channel_id: String,
    owner_id: String,
    last_activity: DateTime<Utc>,
}

pub async fn run(pool: &PgPool, redis: &redis::Client) -> Result<(), String> {
    // 1. Query les tables AFK — clam-based : on UPDATE directement via
    //    `RETURNING` pour garantir l'atomicite (une table ne sera marquee
    //    closed qu'une seule fois, meme avec plusieurs replicas workers).
    let afk: Vec<AfkTable> = sqlx::query_as::<_, AfkTable>(
        "UPDATE blackjack_tables SET status = 'closed' \
         WHERE id IN ( \
             SELECT id FROM blackjack_tables \
             WHERE status = 'open' \
               AND last_activity < NOW() - make_interval(secs => $1) \
             ORDER BY last_activity ASC \
             FOR UPDATE SKIP LOCKED \
             LIMIT 50 \
         ) \
         RETURNING id, guild_id, channel_id, owner_id, last_activity",
    )
    .bind(DEFAULT_AFK_TIMEOUT_SECS)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query afk tables: {e}"))?;

    if afk.is_empty() {
        debug!("Aucune table blackjack AFK");
        return Ok(());
    }

    // 2. Publie les events vers le bot
    let mut conn = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connect: {e}"))?;

    let now = Utc::now();
    let mut published = 0u32;

    for table in &afk {
        let idle_minutes = (now - table.last_activity).num_minutes().max(0);
        let payload = serde_json::json!({
            "event": "blackjack_table_afk",
            "data": {
                "table_id": table.id.to_string(),
                "guild_id": table.guild_id,
                "channel_id": table.channel_id,
                "owner_id": table.owner_id,
                "idle_minutes": idle_minutes,
            }
        });

        let serialized = match serde_json::to_string(&payload) {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "serialize blackjack_table_afk");
                continue;
            }
        };

        let res: redis::RedisResult<String> = redis::cmd("XADD")
            .arg(STREAM_KEY)
            .arg("MAXLEN")
            .arg("~")
            .arg(STREAM_MAXLEN)
            .arg("*")
            .arg(PAYLOAD_FIELD)
            .arg(&serialized)
            .query_async(&mut conn)
            .await;

        match res {
            Ok(_) => {
                published += 1;
                info!(
                    table_id = %table.id,
                    channel_id = %table.channel_id,
                    idle_minutes,
                    "Table blackjack AFK marquee closed"
                );
            }
            Err(e) => warn!(table_id = %table.id, error = %e, "XADD blackjack_table_afk failed"),
        }
    }

    if published > 0 {
        info!(
            published,
            total = afk.len(),
            "Cleanup blackjack : tables closed + events emis"
        );
    }

    Ok(())
}
