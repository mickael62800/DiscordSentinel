use crate::domain::entities::coude::PlayerClass;
use crate::domain::errors::DomainError;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct CoudeProfile {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub class: PlayerClass,
    pub level: i32,
    pub xp: i64,
    pub atk: i32,
    pub def: i32,
    pub hp_current: i32,
    pub hp_max: i32,
    pub coins: i64,
    pub stat_points: i32,
    pub title: String,
    pub total_wins: i32,
    pub total_losses: i32,
    pub total_draws: i32,
    pub total_stolen: i64,
    pub cowardice_count: i32,
    pub chaos_events: i32,
}
#[derive(Debug, Clone)]
pub struct CoudeCombat {
    pub id: uuid::Uuid,
    pub guild_id: String,
    pub attacker_id: String,
    pub defender_id: String,
    pub mise: i64,
    pub status: String,
}
#[derive(Debug, Clone)]
pub struct CoudeCombatSnapshot {
    pub combat: CoudeCombat,
    pub attacker: CoudeProfile,
    pub defender: CoudeProfile,
}

#[async_trait]
pub trait CoudeRepository: Send + Sync {
    async fn find_profile(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoudeProfile>, DomainError>;
    async fn create_profile(&self, profile: &CoudeProfile) -> Result<(), DomainError>;
    async fn update_class(&self, guild_id: &str, user_id: &str, class: PlayerClass, atk: i32, def: i32, hp_max: i32) -> Result<(), DomainError>;
    async fn spend_stat_point(&self, guild_id: &str, user_id: &str, stat: &str) -> Result<CoudeProfile, DomainError>;
    async fn set_progress(&self, guild_id: &str, user_id: &str, xp: i64, level: i32, stat_points: i32, title: &str) -> Result<(), DomainError>;
    async fn create_combat(
        &self,
        guild_id: &str,
        channel_id: &str,
        attacker: &CoudeProfile,
        defender: &CoudeProfile,
        mise: i64,
    ) -> Result<CoudeCombat, DomainError>;
    async fn accept_combat(&self, id: uuid::Uuid, defender_id: &str) -> Result<bool, DomainError>;
    async fn refuse_combat(&self, id: uuid::Uuid, defender_id: &str) -> Result<bool, DomainError>;
    async fn resolution_snapshot(&self, id: uuid::Uuid) -> Result<Option<CoudeCombatSnapshot>, DomainError>;
    async fn resolve_combat(
        &self,
        id: uuid::Uuid,
        winner_id: Option<&str>,
        attacker_roll: i32,
        defender_roll: i32,
        transferred: i64,
        attacker_hp: i32,
        defender_hp: i32,
    ) -> Result<bool, DomainError>;
}
