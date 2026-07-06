//! Event bus Redis Streams — Phase 5B de la roadmap.
//!
//! Remplace l'ancien pub/sub `listen_redis` par une stream unique `sentinel:events`
//! avec consumer groups durables. Chaque event est persiste jusqu'a ACK, et les
//! consumers rattrapent les events manques apres un redemarrage.
//!
//! # Architecture
//!
//! - **Producers** : appellent `publish(conn, event, data)` qui fait un `XADD`
//!   avec `MAXLEN ~ 10000` (borne memoire, ~ = O(1) amorti).
//! - **Consumers durables** (moderation-bot, ticket-bot) : `listen_stream_group`
//!   avec un consumer group unique. `XREADGROUP` + `XACK` garantissent l'at-least-once.
//! - **Consumers live tail** (gateway) : `XREAD $` sans group, fire-and-forget
//!   pour le relay WebSocket (voir `sentinel-gateway/src/redis_subscriber.rs`).
//!
//! # Format des entries
//!
//! Chaque entry a un seul champ `payload` contenant le JSON `{"event": ..., "data": ...}`.
//! Ce format volontairement identique a l'ancien pub/sub pour minimiser les changements
//! cote handlers — ils recoivent toujours une `String` JSON.
//!
//! # Auto-claim pending
//!
//! Au demarrage et periodiquement, chaque consumer fait un `XAUTOCLAIM` des entries
//! pending depuis plus de 60s pour recuperer les events d'un consumer precedent qui
//! aurait crash avant d'ACK.

use std::time::Duration;

use futures_util::Future;
use redis::streams::{StreamId, StreamReadOptions, StreamReadReply};
use redis::AsyncCommands;
use tracing::{debug, error, info, warn};

/// Nom de la stream partagee par tous les producers.
pub const STREAM_KEY: &str = "sentinel:events";

/// Borne de taille approximative de la stream (~ = XADD O(1) amorti).
pub const STREAM_MAXLEN: usize = 10_000;

/// Nom du champ qui contient le JSON de l'event.
pub const PAYLOAD_FIELD: &str = "payload";

/// Delai d'attente avant qu'un event pending soit considere abandonne.
const AUTOCLAIM_MIN_IDLE_MS: u64 = 60_000;

/// Timeout du BLOCK cote XREADGROUP (equilibre latence vs CPU).
const BLOCK_MS: u64 = 5_000;

/// Nombre max d'entries lues par appel XREADGROUP.
const BATCH_COUNT: usize = 32;

/// Delai de reconnexion apres une erreur Redis.
const RECONNECT_DELAY_SECS: u64 = 5;

/// Intervalle entre deux passes d'auto-claim des pending.
const AUTOCLAIM_INTERVAL_SECS: u64 = 30;

/// Publie un event sur la stream.
///
/// Le payload serialise est `{"event": <event>, "data": <data>}` (identique a
/// l'ancien format pub/sub). Retourne l'ID de l'entry cree par Redis.
pub async fn publish(
    conn: &mut redis::aio::MultiplexedConnection,
    event: &str,
    data: serde_json::Value,
) -> redis::RedisResult<String> {
    let payload = serde_json::json!({ "event": event, "data": data }).to_string();
    let id: String = redis::cmd("XADD")
        .arg(STREAM_KEY)
        .arg("MAXLEN")
        .arg("~")
        .arg(STREAM_MAXLEN)
        .arg("*")
        .arg(PAYLOAD_FIELD)
        .arg(payload)
        .query_async(conn)
        .await?;
    Ok(id)
}

/// Cree le consumer group s'il n'existe pas deja. Idempotent.
async fn ensure_group(
    conn: &mut redis::aio::MultiplexedConnection,
    group: &str,
) -> redis::RedisResult<()> {
    // XGROUP CREATE <key> <group> $ MKSTREAM — $ = ne consomme que les nouveaux events
    let res: redis::RedisResult<String> = redis::cmd("XGROUP")
        .arg("CREATE")
        .arg(STREAM_KEY)
        .arg(group)
        .arg("$")
        .arg("MKSTREAM")
        .query_async(conn)
        .await;
    match res {
        Ok(_) => {
            info!(group = %group, stream = %STREAM_KEY, "Consumer group cree");
            Ok(())
        }
        Err(e) if e.code() == Some("BUSYGROUP") => Ok(()),
        Err(e) => Err(e),
    }
}

/// Lance un consumer durable avec reconnexion automatique.
///
/// `group` est le nom du consumer group (typiquement le nom du bot : `moderation-bot`).
/// `consumer` est l'identifiant de l'instance (typiquement hostname + pid).
/// `handler` est appele avec la String JSON `{"event", "data"}` pour chaque entry.
///
/// La fonction ne retourne jamais (boucle infinie avec reconnexion).
pub async fn listen_stream_group<F, Fut>(group: String, consumer: String, handler: F)
where
    F: Fn(String) -> Fut + Send + Sync + Clone + 'static,
    Fut: Future<Output = ()> + Send,
{
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    loop {
        match run_consumer(&redis_url, &group, &consumer, handler.clone()).await {
            Ok(()) => warn!(group = %group, "Consumer loop exited cleanly, reconnecting..."),
            Err(e) => error!(group = %group, error = %e, "Consumer error, reconnecting..."),
        }
        tokio::time::sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
    }
}

async fn run_consumer<F, Fut>(
    redis_url: &str,
    group: &str,
    consumer: &str,
    handler: F,
) -> redis::RedisResult<()>
where
    F: Fn(String) -> Fut + Send + Sync + Clone + 'static,
    Fut: Future<Output = ()> + Send,
{
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_multiplexed_async_connection().await?;

    ensure_group(&mut conn, group).await?;
    info!(group = %group, consumer = %consumer, "Stream consumer demarre");

    // Dedup des IDs deja traites (idempotence sur re-livraison / reclaim).
    let mut seen = ProcessedIds::new(4096);

    // Premier passage : claim des pending laisses par un consumer precedent.
    if let Err(e) = autoclaim_pending(&mut conn, group, consumer, &handler, &mut seen).await {
        warn!(error = %e, "autoclaim initial failed");
    }

    let opts = StreamReadOptions::default()
        .group(group, consumer)
        .block(BLOCK_MS as usize)
        .count(BATCH_COUNT);

    let mut last_autoclaim = std::time::Instant::now();

    loop {
        // XREADGROUP STREAMS sentinel:events >  (> = seulement les nouveaux)
        let reply: Option<StreamReadReply> =
            conn.xread_options(&[STREAM_KEY], &[">"], &opts).await?;

        if let Some(reply) = reply {
            for key in reply.keys {
                for entry in key.ids {
                    process_entry(&mut conn, group, &entry, &handler, &mut seen).await;
                }
            }
        }

        if last_autoclaim.elapsed() >= Duration::from_secs(AUTOCLAIM_INTERVAL_SECS) {
            if let Err(e) =
                autoclaim_pending(&mut conn, group, consumer, &handler, &mut seen).await
            {
                warn!(error = %e, "autoclaim periodique failed");
            }
            last_autoclaim = std::time::Instant::now();
        }
    }
}

/// Deduplication bornee des IDs de message Redis deja traites. Un message peut
/// etre re-livre (reclaim via XAUTOCLAIM) s'il a ete traite mais pas acke (crash
/// reseau sur le XACK) -> sans garde, l'action du handler serait rejouee. On
/// memorise les derniers IDs traites (FIFO borne) pour ne pas rejouer.
struct ProcessedIds {
    set: std::collections::HashSet<String>,
    order: std::collections::VecDeque<String>,
    cap: usize,
}

impl ProcessedIds {
    fn new(cap: usize) -> Self {
        Self {
            set: std::collections::HashSet::new(),
            order: std::collections::VecDeque::new(),
            cap,
        }
    }
    fn contains(&self, id: &str) -> bool {
        self.set.contains(id)
    }
    fn record(&mut self, id: String) {
        if self.set.insert(id.clone()) {
            self.order.push_back(id);
            if self.order.len() > self.cap {
                if let Some(old) = self.order.pop_front() {
                    self.set.remove(&old);
                }
            }
        }
    }
}

async fn process_entry<F, Fut>(
    conn: &mut redis::aio::MultiplexedConnection,
    group: &str,
    entry: &StreamId,
    handler: &F,
    seen: &mut ProcessedIds,
) where
    F: Fn(String) -> Fut + Send + Sync,
    Fut: Future<Output = ()> + Send,
{
    // Idempotence : un ID deja traite (re-livre car non acke) n'est PAS rejoue,
    // on se contente de (re)acker pour vider la pending list.
    if seen.contains(entry.id.as_str()) {
        debug!(entry_id = %entry.id, "Entry deja traitee -> skip (idempotence)");
        let _ = conn
            .xack::<_, _, _, i64>(STREAM_KEY, group, &[entry.id.as_str()])
            .await;
        return;
    }
    let payload = extract_payload(&entry.map);
    match payload {
        Some(s) => {
            handler(s).await;
            seen.record(entry.id.to_string());
            if let Err(e) = conn
                .xack::<_, _, _, i64>(STREAM_KEY, group, &[entry.id.as_str()])
                .await
            {
                warn!(error = %e, entry_id = %entry.id, "XACK failed");
            } else {
                debug!(entry_id = %entry.id, "Entry ACKed");
            }
        }
        None => {
            // Entry malformee — ACK quand meme pour ne pas bloquer la pending list
            warn!(entry_id = %entry.id, "Entry sans champ payload, ACK skip");
            let _ = conn
                .xack::<_, _, _, i64>(STREAM_KEY, group, &[entry.id.as_str()])
                .await;
        }
    }
}

fn extract_payload(map: &std::collections::HashMap<String, redis::Value>) -> Option<String> {
    let value = map.get(PAYLOAD_FIELD)?;
    match value {
        redis::Value::BulkString(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        redis::Value::SimpleString(s) => Some(s.clone()),
        _ => None,
    }
}

/// Rattrape les entries pending depuis plus de `AUTOCLAIM_MIN_IDLE_MS`.
///
/// Utilise `XAUTOCLAIM` (Redis 6.2+) qui est plus simple que XPENDING+XCLAIM.
async fn autoclaim_pending<F, Fut>(
    conn: &mut redis::aio::MultiplexedConnection,
    group: &str,
    consumer: &str,
    handler: &F,
    seen: &mut ProcessedIds,
) -> redis::RedisResult<()>
where
    F: Fn(String) -> Fut + Send + Sync,
    Fut: Future<Output = ()> + Send,
{
    // XAUTOCLAIM <key> <group> <consumer> <min-idle-ms> <start> [COUNT n]
    // Reply: [next_cursor, [claimed entries], [deleted ids]]
    let reply: redis::Value = redis::cmd("XAUTOCLAIM")
        .arg(STREAM_KEY)
        .arg(group)
        .arg(consumer)
        .arg(AUTOCLAIM_MIN_IDLE_MS)
        .arg("0-0")
        .arg("COUNT")
        .arg(BATCH_COUNT)
        .query_async(conn)
        .await?;

    let claimed = parse_autoclaim_entries(&reply);
    if claimed.is_empty() {
        return Ok(());
    }

    info!(group = %group, count = claimed.len(), "Auto-claim pending entries");
    for entry in claimed {
        process_entry(conn, group, &entry, handler, seen).await;
    }
    Ok(())
}

/// Parse le reply XAUTOCLAIM en liste de StreamId.
///
/// Structure: Array([next_cursor, Array([entries...]), Array([deleted_ids...])])
fn parse_autoclaim_entries(value: &redis::Value) -> Vec<StreamId> {
    let redis::Value::Array(ref outer) = *value else {
        return Vec::new();
    };
    if outer.len() < 2 {
        return Vec::new();
    }
    let redis::Value::Array(ref entries) = outer[1] else {
        return Vec::new();
    };

    let mut result = Vec::new();
    for entry in entries {
        let redis::Value::Array(ref pair) = *entry else {
            continue;
        };
        if pair.len() < 2 {
            continue;
        }
        let id = match &pair[0] {
            redis::Value::BulkString(b) => String::from_utf8_lossy(b).into_owned(),
            redis::Value::SimpleString(s) => s.clone(),
            _ => continue,
        };
        let redis::Value::Array(ref fields) = pair[1] else {
            continue;
        };
        let mut map = std::collections::HashMap::new();
        let mut i = 0;
        while i + 1 < fields.len() {
            let key = match &fields[i] {
                redis::Value::BulkString(b) => String::from_utf8_lossy(b).into_owned(),
                redis::Value::SimpleString(s) => s.clone(),
                _ => {
                    i += 2;
                    continue;
                }
            };
            map.insert(key, fields[i + 1].clone());
            i += 2;
        }
        result.push(StreamId { id, map });
    }
    result
}

/// Construit un nom de consumer unique pour cette instance.
///
/// Format : `{hostname}-{pid}` — suffisamment unique pour distinguer plusieurs
/// replicas du meme bot (multi-instance) tout en restant stable a travers les
/// redemarrages si le hostname est stable (k8s StatefulSet, docker-compose).
pub fn default_consumer_name() -> String {
    let host = std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string());
    let pid = std::process::id();
    format!("{host}-{pid}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_consumer_name_has_pid() {
        let name = default_consumer_name();
        assert!(name.contains('-'));
        let parts: Vec<&str> = name.rsplitn(2, '-').collect();
        assert!(parts[0].parse::<u32>().is_ok(), "pid should be numeric");
    }

    #[test]
    fn parse_autoclaim_empty_reply() {
        let empty = redis::Value::Array(vec![]);
        assert!(parse_autoclaim_entries(&empty).is_empty());

        let cursor_only = redis::Value::Array(vec![
            redis::Value::BulkString(b"0-0".to_vec()),
            redis::Value::Array(vec![]),
        ]);
        assert!(parse_autoclaim_entries(&cursor_only).is_empty());
    }

    #[test]
    fn parse_autoclaim_single_entry() {
        let reply = redis::Value::Array(vec![
            redis::Value::BulkString(b"0-0".to_vec()),
            redis::Value::Array(vec![redis::Value::Array(vec![
                redis::Value::BulkString(b"1700000000000-0".to_vec()),
                redis::Value::Array(vec![
                    redis::Value::BulkString(b"payload".to_vec()),
                    redis::Value::BulkString(b"{\"event\":\"test\"}".to_vec()),
                ]),
            ])]),
            redis::Value::Array(vec![]),
        ]);
        let entries = parse_autoclaim_entries(&reply);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "1700000000000-0");
        let payload = extract_payload(&entries[0].map).unwrap();
        assert_eq!(payload, "{\"event\":\"test\"}");
    }

    // ── Tests extract_payload edge cases ─────────────────

    fn build_map(
        entries: Vec<(&str, redis::Value)>,
    ) -> std::collections::HashMap<String, redis::Value> {
        entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    #[test]
    fn extract_payload_missing_field_returns_none() {
        // Map vide → champ payload absent
        let map = std::collections::HashMap::<String, redis::Value>::new();
        assert_eq!(extract_payload(&map), None);

        // Map avec un autre champ, mais pas "payload"
        let map = build_map(vec![(
            "other_field",
            redis::Value::BulkString(b"value".to_vec()),
        )]);
        assert_eq!(extract_payload(&map), None);
    }

    #[test]
    fn extract_payload_bulk_string_utf8() {
        let map = build_map(vec![(
            "payload",
            redis::Value::BulkString(b"{\"event\":\"test\"}".to_vec()),
        )]);
        assert_eq!(
            extract_payload(&map),
            Some("{\"event\":\"test\"}".to_string())
        );
    }

    #[test]
    fn extract_payload_bulk_string_with_unicode() {
        // Accents et emojis dans le payload
        let payload = "{\"event\":\"sanction\",\"reason\":\"caractères spéciaux \u{1f6a8}\"}";
        let map = build_map(vec![(
            "payload",
            redis::Value::BulkString(payload.as_bytes().to_vec()),
        )]);
        assert_eq!(extract_payload(&map), Some(payload.to_string()));
    }

    #[test]
    fn extract_payload_bulk_string_invalid_utf8_lossy() {
        // Bytes invalides UTF-8 → from_utf8_lossy remplace par replacement char U+FFFD
        let map = build_map(vec![(
            "payload",
            redis::Value::BulkString(vec![0xFF, 0xFE, b'x']),
        )]);
        let result = extract_payload(&map);
        assert!(result.is_some());
        // Le "x" final doit toujours être la, les bytes invalides sont remplaces
        assert!(result.unwrap().contains('x'));
    }

    #[test]
    fn extract_payload_simple_string() {
        let map = build_map(vec![(
            "payload",
            redis::Value::SimpleString("inline-string".to_string()),
        )]);
        assert_eq!(extract_payload(&map), Some("inline-string".to_string()));
    }

    #[test]
    fn extract_payload_unsupported_variants_return_none() {
        // Integer, Nil, Array, etc. ne sont pas des payloads valides
        let map = build_map(vec![("payload", redis::Value::Int(42))]);
        assert_eq!(extract_payload(&map), None);

        let map = build_map(vec![("payload", redis::Value::Nil)]);
        assert_eq!(extract_payload(&map), None);

        let map = build_map(vec![(
            "payload",
            redis::Value::Array(vec![redis::Value::Int(1)]),
        )]);
        assert_eq!(extract_payload(&map), None);
    }
}
