use async_trait::async_trait;

use crate::domain::entities::community::level::UserLevel;
use crate::domain::entities::community::level::XpSource;
use crate::domain::entities::system::discord_ids::GuildId;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::errors::DomainError;

pub struct AddXpCommand {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub amount: i64,
    pub source: XpSource,
}

pub struct AddXpResult {
    pub user_level: UserLevel,
    /// `true` si le niveau de la source (texte ou vocal) a augmente.
    pub leveled_up: bool,
    /// Ancien niveau de la source declenchante (texte ou vocal).
    pub old_level: i32,
    /// Ancien niveau global (= level_from_xp(xp_text + xp_voice) avant l'ajout).
    /// Sert au bot pour declencher le renommage `[NN]Pseudo` uniquement
    /// quand le niveau total change reellement.
    pub old_level_global: i32,
    pub source: XpSource,
}

/// Set la valeur exacte de l'XP texte et/ou voix d'un utilisateur.
/// `None` = ne pas modifier ce champ. Les niveaux sont recalcules
/// automatiquement depuis les nouvelles valeurs d'XP.
pub struct SetUserXpCommand {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub xp_text: Option<i64>,
    pub xp_voice: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetTarget {
    All,
    Text,
    Voice,
}

#[async_trait]
pub trait ManageLevelsUseCase: Send + Sync {
    async fn add_xp(&self, cmd: AddXpCommand) -> Result<AddXpResult, DomainError>;
    async fn get_user_level(&self, guild_id: &str, user_id: &str)
        -> Result<UserLevel, DomainError>;
    async fn get_leaderboard(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<UserLevel>, DomainError>;
    async fn get_leaderboard_by_source(
        &self,
        guild_id: &str,
        source: XpSource,
        limit: i64,
    ) -> Result<Vec<UserLevel>, DomainError>;
    /// Set valeur exacte XP texte/voix (admin override). Recalcule les niveaux.
    async fn set_user_xp(&self, cmd: SetUserXpCommand) -> Result<UserLevel, DomainError>;
    /// Reset XP a 0 sur la cible (text / voice / all). Recalcule les niveaux.
    async fn reset_user_xp(
        &self,
        guild_id: &str,
        user_id: &str,
        target: ResetTarget,
    ) -> Result<UserLevel, DomainError>;
}
