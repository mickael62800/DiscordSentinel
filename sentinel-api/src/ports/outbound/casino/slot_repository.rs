use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sentinel_core::ports::uow::DbTx;
use sentinel_core::domain::entities::casino::slot::SlotJackpotPool;
use sentinel_core::domain::entities::casino::slot::SlotSpin;
use sentinel_core::domain::entities::casino::slot::SlotTopWinner;
use sentinel_core::domain::errors::DomainError;

#[async_trait]
pub trait SlotRepository: Send + Sync {
    /// Recupere l etat du pool jackpot pour une guild. None si pas encore initialise.
    async fn get_jackpot_pool(&self, guild_id: &str) -> Result<Option<SlotJackpotPool>, DomainError>;

    /// Insere une row pool si absente, avec `starting` comme valeur initiale.
    /// Idempotent : ne touche pas une row existante.
    async fn init_jackpot_pool_if_absent(&self, guild_id: &str, starting: i64) -> Result<(), DomainError>;

    /// Ajoute `amount` au pool dans une tx en cours. Retourne le nouveau total.
    /// Si la row n existe pas, elle est creee a `starting + amount`.
    async fn add_to_jackpot_pool_in_tx(
        &self,
        tx: &mut dyn DbTx,
        guild_id: &str,
        amount: i64,
        starting: i64,
    ) -> Result<i64, DomainError>;

    /// Reset le pool a `reset_to` apres qu un winner ait remporte le jackpot.
    /// Met a jour les champs last_won_*.
    async fn claim_jackpot_pool_in_tx(
        &self,
        tx: &mut dyn DbTx,
        guild_id: &str,
        winner_id: &str,
        won_amount: i64,
        reset_to: i64,
    ) -> Result<(), DomainError>;

    /// Persiste un spin dans la tx en cours (cf. flow atomique :
    /// debit wallet + spin log + jackpot ops dans la meme tx).
    async fn log_spin_in_tx(
        &self,
        tx: &mut dyn DbTx,
        spin: &SlotSpin,
    ) -> Result<(), DomainError>;

    /// Timestamp du dernier spin du joueur dans cette guild. Utilise pour
    /// l enforcement du cooldown.
    async fn last_spin_at(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<DateTime<Utc>>, DomainError>;

    /// True si le user a deja claim son daily bonus aujourd hui (date Postgres
    /// CURRENT_DATE).
    async fn has_claimed_daily_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<bool, DomainError>;

    /// Marque le daily bonus comme claim (insere dans slot_daily_claims).
    /// ON CONFLICT DO NOTHING : idempotent.
    async fn mark_daily_claimed_in_tx(
        &self,
        tx: &mut dyn DbTx,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError>;

    /// Liste les N derniers spins d une guild (tous joueurs confondus).
    async fn recent_spins(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<SlotSpin>, DomainError>;

    /// Top winners sur les `days` derniers jours, classe par total_payout.
    async fn top_winners(
        &self,
        guild_id: &str,
        days: i64,
        limit: i64,
    ) -> Result<Vec<SlotTopWinner>, DomainError>;
}
