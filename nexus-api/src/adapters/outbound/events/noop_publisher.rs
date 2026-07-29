//! Publieur inerte : utilise quand REDIS_URL n'est pas configuree.
//!
//! Les evenements sont seulement traces. Consequence fonctionnelle : le bot ne
//! creera pas les salons de session (il ne recoit rien).

use async_trait::async_trait;
use nexus_core::ports::outbound::events::EventPublisher;

pub struct NoopEventPublisher;

#[async_trait]
impl EventPublisher for NoopEventPublisher {
    async fn publish(&self, event: &str, data: serde_json::Value) {
        tracing::debug!(event, %data, "event non publie (publieur noop)");
    }
}
