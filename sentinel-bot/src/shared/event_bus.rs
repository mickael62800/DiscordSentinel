//! Bus d'evenements de Sentinel : stream `sentinel:events`.
//!
//! La mecanique (consumer groups, ACK, auto-claim, deduplication) vit dans
//! `platform-common::event_bus` — elle etait auparavant dupliquee a l'identique
//! dans `nexus-bot`, a la constante de stream pres.
//!
//! Ce module ne garde que la configuration propre a Sentinel et re-expose
//! l'API historique, pour ne pas toucher les dizaines de sites d'appel.

use futures_util::Future;
use platform_common::EventBus;

pub use platform_common::default_consumer_name;

/// Nom de la stream partagee par tous les producers Sentinel.
pub const STREAM_KEY: &str = "sentinel:events";

/// Borne de taille approximative de la stream (`~` = XADD O(1) amorti).
pub const STREAM_MAXLEN: usize = 10_000;

/// Le bus de cette plateforme.
const BUS: EventBus = EventBus::with_maxlen(STREAM_KEY, STREAM_MAXLEN);

/// Publie un event sur `sentinel:events`.
///
/// Le payload serialise est `{"event": <event>, "data": <data>}`. Retourne
/// l'ID de l'entry creee par Redis.
pub async fn publish(
    conn: &mut redis::aio::MultiplexedConnection,
    event: &str,
    data: serde_json::Value,
) -> redis::RedisResult<String> {
    BUS.publish(conn, event, data).await
}

/// Lance un consumer durable, avec reconnexion automatique.
///
/// `group` est le nom du consumer group (typiquement le nom du module),
/// `consumer` l'identifiant de l'instance. Ne retourne jamais.
pub async fn listen_stream_group<F, Fut>(group: String, consumer: String, handler: F)
where
    F: Fn(String) -> Fut + Send + Sync + Clone + 'static,
    Fut: Future<Output = ()> + Send,
{
    BUS.listen_stream_group(group, consumer, handler).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La clef de stream est un contrat inter-services : le worker et l'API
    /// publient dessus, la gateway la lit en live-tail. La changer par
    /// inadvertance couperait tout le bus en silence.
    #[test]
    fn stream_key_est_celle_de_sentinel() {
        assert_eq!(STREAM_KEY, "sentinel:events");
        assert_eq!(BUS.stream_key(), STREAM_KEY);
    }
}
