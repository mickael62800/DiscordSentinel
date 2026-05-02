//! Implementation Redis du port allocator.
//!
//! Strategie : pour chaque port du range, une cle Redis `game:port:{kind}:{port}`
//! contenant l'owner_key (server_id). Allocation = SETNX avec TTL long
//! (24h, refresh si besoin). Liberation = DEL.

use async_trait::async_trait;
use redis::AsyncCommands;
use redis::Client;

use crate::domain::errors::DomainError;
use crate::ports::outbound::game::port_allocator::{PortAllocator, PortKind};

const KEY_TTL_SECS: u64 = 60 * 60 * 24 * 7; // 7j (refresh sur usage)

pub struct RedisPortAllocator {
    client: Client,
}

impl RedisPortAllocator {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn key(kind: PortKind, port: u16) -> String {
        let prefix = match kind {
            PortKind::Game => "game",
            PortKind::Rcon => "rcon",
        };
        format!("game:port:{prefix}:{port}")
    }
}

#[async_trait]
impl PortAllocator for RedisPortAllocator {
    async fn allocate(
        &self,
        kind: PortKind,
        range_start: u16,
        range_end: u16,
        owner_key: &str,
    ) -> Result<u16, DomainError> {
        if range_start > range_end {
            return Err(DomainError::ValidationError(
                "range_start > range_end".into(),
            ));
        }
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| DomainError::Internal(format!("redis conn: {e}")))?;

        for port in range_start..=range_end {
            let key = Self::key(kind, port);
            // SET NX EX (atomic). Retourne true si on a gagne le slot.
            let won: bool = redis::cmd("SET")
                .arg(&key)
                .arg(owner_key)
                .arg("NX")
                .arg("EX")
                .arg(KEY_TTL_SECS)
                .query_async(&mut conn)
                .await
                .map_err(|e| DomainError::Internal(format!("redis SET NX: {e}")))?;
            if won {
                return Ok(port);
            }
        }
        Err(DomainError::ValidationError(
            "aucun port libre dans le range configure".into(),
        ))
    }

    async fn release(&self, kind: PortKind, port: u16) -> Result<(), DomainError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| DomainError::Internal(format!("redis conn: {e}")))?;
        let _: i64 = conn
            .del(Self::key(kind, port))
            .await
            .map_err(|e| DomainError::Internal(format!("redis DEL: {e}")))?;
        Ok(())
    }

    async fn is_available(&self, kind: PortKind, port: u16) -> Result<bool, DomainError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| DomainError::Internal(format!("redis conn: {e}")))?;
        let exists: bool = conn
            .exists(Self::key(kind, port))
            .await
            .map_err(|e| DomainError::Internal(format!("redis EXISTS: {e}")))?;
        Ok(!exists)
    }
}
