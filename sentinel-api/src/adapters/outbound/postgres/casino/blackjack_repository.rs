use crate::adapters::outbound::postgres::pg_ctx;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::ports::outbound::casino::blackjack_repository::BlackjackRepository;
use sentinel_core::domain::entities::casino::blackjack::BlackjackGame;
use sentinel_core::domain::errors::DomainError;

pub struct PgBlackjackRepository {
    pool: PgPool,
}

impl PgBlackjackRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct BlackjackRow {
    id: Uuid,
    guild_id: String,
    user_id: String,
    username: String,
    bet: i64,
    player_hand: serde_json::Value,
    dealer_hand: serde_json::Value,
    deck: serde_json::Value,
    status: String,
    player_score: i32,
    dealer_score: i32,
    doubled: bool,
    payout: i64,
    created_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
}

impl From<BlackjackRow> for BlackjackGame {
    fn from(r: BlackjackRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            user_id: r.user_id.into(),
            username: r.username,
            bet: r.bet,
            player_hand: serde_json::from_value(r.player_hand).unwrap_or_default(),
            dealer_hand: serde_json::from_value(r.dealer_hand).unwrap_or_default(),
            deck: serde_json::from_value(r.deck).unwrap_or_default(),
            status: r.status,
            player_score: r.player_score,
            dealer_score: r.dealer_score,
            doubled: r.doubled,
            payout: r.payout,
            created_at: r.created_at,
            finished_at: r.finished_at,
        }
    }
}

#[async_trait]
impl BlackjackRepository for PgBlackjackRepository {
    async fn create(&self, game: &BlackjackGame) -> Result<(), DomainError> {
        let player_hand = serde_json::to_value(&game.player_hand)
            .map_err(|e| DomainError::Internal(format!("sérialisation player_hand : {e}")))?;
        let dealer_hand = serde_json::to_value(&game.dealer_hand)
            .map_err(|e| DomainError::Internal(format!("sérialisation dealer_hand : {e}")))?;
        let deck = serde_json::to_value(&game.deck)
            .map_err(|e| DomainError::Internal(format!("sérialisation deck : {e}")))?;

        sqlx::query(
            "INSERT INTO blackjack_games (id, guild_id, user_id, username, bet, player_hand, dealer_hand, deck, status, player_score, dealer_score, doubled, payout, created_at, finished_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)"
        )
        .bind(game.id)
        .bind(game.guild_id.as_str())
        .bind(game.user_id.as_str())
        .bind(&game.username)
        .bind(game.bet)
        .bind(player_hand)
        .bind(dealer_hand)
        .bind(deck)
        .bind(&game.status)
        .bind(game.player_score)
        .bind(game.dealer_score)
        .bind(game.doubled)
        .bind(game.payout)
        .bind(game.created_at)
        .bind(game.finished_at)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("blackjack create "))?;

        Ok(())
    }

    async fn get_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<BlackjackGame>, DomainError> {
        // Une partie est reellement "active" uniquement si :
        //  - status = 'playing' ET finished_at IS NULL (pas un blackjack naturel)
        //  - created_at dans les 30 dernieres minutes (au-dela, on considere
        //    la partie abandonnee suite a crash bot/timeout Discord — ne doit
        //    pas bloquer eternellement le user)
        let row = sqlx::query_as::<_, BlackjackRow>(
            "SELECT id, guild_id, user_id, username, bet, player_hand, dealer_hand, deck, status, player_score, dealer_score, doubled, payout, created_at, finished_at
             FROM blackjack_games
             WHERE guild_id = $1 AND user_id = $2 \
               AND status = 'playing' AND finished_at IS NULL \
               AND created_at > NOW() - INTERVAL '30 minutes'
             ORDER BY created_at DESC
             LIMIT 1"
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("blackjack get_active "))?;

        Ok(row.map(BlackjackGame::from))
    }

    async fn update(&self, game: &BlackjackGame) -> Result<(), DomainError> {
        let player_hand = serde_json::to_value(&game.player_hand)
            .map_err(|e| DomainError::Internal(format!("sérialisation player_hand : {e}")))?;
        let dealer_hand = serde_json::to_value(&game.dealer_hand)
            .map_err(|e| DomainError::Internal(format!("sérialisation dealer_hand : {e}")))?;
        let deck = serde_json::to_value(&game.deck)
            .map_err(|e| DomainError::Internal(format!("sérialisation deck : {e}")))?;

        // Guard : ne mettre a jour que si la partie est encore en cours
        // Empeche les race conditions (deux hit simultanes, hit+stand, etc.)
        let result = sqlx::query(
            "UPDATE blackjack_games SET
                player_hand = $1, dealer_hand = $2, deck = $3,
                status = $4, player_score = $5, dealer_score = $6,
                doubled = $7, payout = $8, bet = $9, finished_at = $10
             WHERE id = $11 AND status = 'playing'",
        )
        .bind(player_hand)
        .bind(dealer_hand)
        .bind(deck)
        .bind(&game.status)
        .bind(game.player_score)
        .bind(game.dealer_score)
        .bind(game.doubled)
        .bind(game.payout)
        .bind(game.bet)
        .bind(game.finished_at)
        .bind(game.id)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("blackjack update "))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::Conflict(
                "Partie deja terminee ou action concurrente".into(),
            ));
        }

        Ok(())
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Option<BlackjackGame>, DomainError> {
        let row = sqlx::query_as::<_, BlackjackRow>(
            "SELECT id, guild_id, user_id, username, bet, player_hand, dealer_hand, deck, status, player_score, dealer_score, doubled, payout, created_at, finished_at
             FROM blackjack_games
             WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("blackjack get_by_id "))?;

        Ok(row.map(BlackjackGame::from))
    }

    async fn list_by_guild(
        &self,
        guild_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<BlackjackGame>, DomainError> {
        let rows = if let Some(s) = status {
            sqlx::query_as::<_, BlackjackRow>(
                "SELECT id, guild_id, user_id, username, bet, player_hand, dealer_hand, deck, status, player_score, dealer_score, doubled, payout, created_at, finished_at
                 FROM blackjack_games
                 WHERE guild_id = $1 AND status = $2
                 ORDER BY created_at DESC
                 LIMIT 200"
            )
            .bind(guild_id)
            .bind(s)
            .fetch_all(&self.pool).await
        } else {
            sqlx::query_as::<_, BlackjackRow>(
                "SELECT id, guild_id, user_id, username, bet, player_hand, dealer_hand, deck, status, player_score, dealer_score, doubled, payout, created_at, finished_at
                 FROM blackjack_games
                 WHERE guild_id = $1
                 ORDER BY created_at DESC
                 LIMIT 200"
            )
            .bind(guild_id)
            .fetch_all(&self.pool).await
        }
        .map_err(pg_ctx("blackjack list_by_guild "))?;

        Ok(rows.into_iter().map(BlackjackGame::from).collect())
    }

    async fn cancel_game(&self, id: Uuid) -> Result<(), DomainError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(pg_ctx("cancel_game begin "))?;

        // Recupere la partie pour obtenir la mise et valider le status.
        let row = sqlx::query_as::<_, BlackjackRow>(
            "SELECT id, guild_id, user_id, username, bet, player_hand, dealer_hand, deck, status, player_score, dealer_score, doubled, payout, created_at, finished_at
             FROM blackjack_games WHERE id = $1 FOR UPDATE"
        )
        .bind(id)
        .fetch_optional(&mut *tx).await
        .map_err(pg_ctx("cancel_game select "))?
        .ok_or_else(|| DomainError::NotFound(format!("Partie blackjack {id} introuvable")))?;

        // Seules les parties en cours sont annulables.
        // Les parties solo ont status "playing", les parties multi-tables
        // peuvent etre "waiting" quand le dealer est en train de distribuer.
        if !matches!(row.status.as_str(), "playing" | "waiting") {
            return Err(DomainError::Conflict(format!(
                "Partie deja terminee (status = {})",
                row.status
            )));
        }

        // Marque la partie comme annulee.
        sqlx::query(
            "UPDATE blackjack_games
             SET status = 'cancelled', finished_at = NOW()
             WHERE id = $1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(pg_ctx("cancel_game update "))?;

        // Rembourse la mise sur le wallet du joueur.
        let refund = row.bet + if row.doubled { row.bet } else { 0 };
        sqlx::query(
            "UPDATE user_wallets
             SET coins = coins + $1, total_spent = GREATEST(0, total_spent - $1), updated_at = NOW()
             WHERE guild_id = $2 AND user_id = $3",
        )
        .bind(refund)
        .bind(row.guild_id.as_str())
        .bind(row.user_id.as_str())
        .execute(&mut *tx)
        .await
        .map_err(pg_ctx("cancel_game refund "))?;

        sqlx::query(
            "INSERT INTO wallet_transactions (id, guild_id, user_id, amount, balance_after, source, description, created_at)
             SELECT $1, $2, $3, $4,
                    (SELECT coins FROM user_wallets WHERE guild_id = $2 AND user_id = $3),
                    'blackjack_cancel', 'Annulation partie blackjack admin', NOW()"
        )
        .bind(Uuid::new_v4())
        .bind(row.guild_id.as_str())
        .bind(row.user_id.as_str())
        .bind(refund)
        .execute(&mut *tx).await
        .map_err(pg_ctx("cancel_game audit "))?;

        tx.commit().await.map_err(pg_ctx("cancel_game commit "))?;

        tracing::info!(
            game_id = %id, guild_id = %row.guild_id, user_id = %row.user_id,
            refund, "Blackjack game cancelled"
        );
        Ok(())
    }
}
