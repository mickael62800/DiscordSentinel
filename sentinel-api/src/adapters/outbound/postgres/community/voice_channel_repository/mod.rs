use sqlx::PgPool;

pub struct PgVoiceChannelRepository {
    pool: PgPool,
}

impl PgVoiceChannelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

mod channels;
mod co_admins;
mod whitelist;
mod presets;
mod bans;
mod invites;
mod themes;
