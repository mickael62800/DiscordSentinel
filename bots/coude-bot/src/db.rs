use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

// ── Modeles ──

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Player {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub class: String,
    pub coins: i64,
    pub total_wins: i32,
    pub total_losses: i32,
    pub total_draws: i32,
    pub total_earned: i64,
    pub total_lost: i64,
    pub total_stolen: i64,
    pub cowardice_count: i32,
    pub casino_wins: i32,
    pub casino_losses: i32,
    pub chaos_events: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub level: i32,
    pub xp: i64,
    pub atk: i32,
    pub def: i32,
    pub stat_points: i32,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Combat {
    pub id: Uuid,
    pub guild_id: String,
    pub channel_id: String,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
    pub mise: i64,
    pub status: String,
    pub winner_id: Option<String>,
    pub attacker_roll: Option<i32>,
    pub defender_roll: Option<i32>,
    pub chaos_event: Option<String>,
    pub special_attack: Option<String>,
    pub result_message: Option<String>,
    pub coins_transferred: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub defender_special: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Prime {
    pub id: Uuid,
    pub guild_id: String,
    pub target_id: String,
    pub target_name: String,
    pub placed_by_id: String,
    pub placed_by_name: String,
    pub amount: i64,
    pub claimed: bool,
    pub claimed_by_id: Option<String>,
    pub claimed_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub claimed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct InventoryItem {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub item_key: String,
    pub quantity: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServerEvent {
    pub id: Uuid,
    pub guild_id: String,
    pub event_type: String,
    pub active: bool,
    pub started_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LeaderboardEntry {
    pub user_id: String,
    pub username: String,
    pub value: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Bet {
    pub id: Uuid,
    pub combat_id: Uuid,
    pub bettor_id: String,
    pub bettor_name: String,
    pub backed_id: String,
    pub amount: i64,
}

pub struct BetResult {
    pub bettor_id: String,
    pub bettor_name: String,
    pub amount: i64,
    pub won: bool,
    pub payout: i64,
}

/// Gains des combattants issus du pot de paris.
pub struct FighterBetBonus {
    /// 10% du pot total pour le gagnant
    pub winner_bonus: i64,
    /// 5% du pot total pour le perdant
    pub loser_bonus: i64,
    /// Pot total des paris
    pub total_pot: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Insurance {
    pub id: Uuid,
    pub is_scam: bool,
    pub expires_at: DateTime<Utc>,
}

// ── Database ──

pub struct GameDb {
    pool: PgPool,
}

impl GameDb {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ── Players ──

    pub async fn get_or_create_player(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<Player, sqlx::Error> {
        let player = sqlx::query_as::<_, Player>(
            r#"
            INSERT INTO coude_players (guild_id, user_id, username)
            VALUES ($1, $2, $3)
            ON CONFLICT (guild_id, user_id) DO UPDATE SET username = EXCLUDED.username, updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(username)
        .fetch_one(&self.pool)
        .await?;
        Ok(player)
    }

    pub async fn get_player(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Player>, sqlx::Error> {
        sqlx::query_as::<_, Player>(
            "SELECT * FROM coude_players WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn update_player_coins(
        &self,
        guild_id: &str,
        user_id: &str,
        delta: i64,
    ) -> Result<Player, sqlx::Error> {
        sqlx::query_as::<_, Player>(
            r#"
            UPDATE coude_players
            SET coins = GREATEST(0, coins + $3), updated_at = NOW()
            WHERE guild_id = $1 AND user_id = $2
            RETURNING *
            "#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(delta)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn set_player_coins(
        &self,
        guild_id: &str,
        user_id: &str,
        coins: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE coude_players SET coins = $3, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(coins)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_player_class(
        &self,
        guild_id: &str,
        user_id: &str,
        class: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE coude_players SET class = $3, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(class)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn record_win(
        &self,
        guild_id: &str,
        user_id: &str,
        earned: i64,
        stolen: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE coude_players
            SET total_wins = total_wins + 1,
                coins = coins + $3,
                total_earned = total_earned + $3,
                total_stolen = total_stolen + $4,
                updated_at = NOW()
            WHERE guild_id = $1 AND user_id = $2
            "#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(earned)
        .bind(stolen)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn record_loss(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE coude_players
            SET total_losses = total_losses + 1,
                coins = GREATEST(0, coins - $3),
                total_lost = total_lost + $3,
                updated_at = NOW()
            WHERE guild_id = $1 AND user_id = $2
            "#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(lost)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn record_draw(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE coude_players
            SET total_draws = total_draws + 1,
                coins = GREATEST(0, coins - $3),
                total_lost = total_lost + $3,
                updated_at = NOW()
            WHERE guild_id = $1 AND user_id = $2
            "#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(lost)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn increment_cowardice(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i32, sqlx::Error> {
        let row: (i32,) = sqlx::query_as(
            r#"
            UPDATE coude_players
            SET cowardice_count = cowardice_count + 1, updated_at = NOW()
            WHERE guild_id = $1 AND user_id = $2
            RETURNING cowardice_count
            "#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    pub async fn increment_chaos_events(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE coude_players SET chaos_events = chaos_events + 1, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn record_casino_win(
        &self,
        guild_id: &str,
        user_id: &str,
        gain: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE coude_players
            SET casino_wins = casino_wins + 1,
                coins = coins + $3,
                total_earned = total_earned + $3,
                updated_at = NOW()
            WHERE guild_id = $1 AND user_id = $2
            "#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(gain)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn record_casino_loss(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE coude_players
            SET casino_losses = casino_losses + 1,
                coins = GREATEST(0, coins - $3),
                total_lost = total_lost + $3,
                updated_at = NOW()
            WHERE guild_id = $1 AND user_id = $2
            "#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(lost)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn record_casino_faillite(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            r#"
            UPDATE coude_players
            SET casino_losses = casino_losses + 1,
                total_lost = total_lost + coins,
                coins = 0,
                updated_at = NOW()
            WHERE guild_id = $1 AND user_id = $2
            RETURNING total_lost
            "#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    // ── Combats ──

    pub async fn create_combat(
        &self,
        guild_id: &str,
        channel_id: &str,
        attacker_id: &str,
        attacker_name: &str,
        defender_id: &str,
        defender_name: &str,
        mise: i64,
        special_attack: Option<&str>,
    ) -> Result<Combat, sqlx::Error> {
        sqlx::query_as::<_, Combat>(
            r#"
            INSERT INTO coude_combats (guild_id, channel_id, attacker_id, attacker_name, defender_id, defender_name, mise, special_attack)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(guild_id)
        .bind(channel_id)
        .bind(attacker_id)
        .bind(attacker_name)
        .bind(defender_id)
        .bind(defender_name)
        .bind(mise)
        .bind(special_attack)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get_combat(&self, id: Uuid) -> Result<Option<Combat>, sqlx::Error> {
        sqlx::query_as::<_, Combat>("SELECT * FROM coude_combats WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn get_pending_combat_for_defender(
        &self,
        guild_id: &str,
        defender_id: &str,
    ) -> Result<Option<Combat>, sqlx::Error> {
        sqlx::query_as::<_, Combat>(
            "SELECT * FROM coude_combats WHERE guild_id = $1 AND defender_id = $2 AND status = 'pending' ORDER BY created_at DESC LIMIT 1",
        )
        .bind(guild_id)
        .bind(defender_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_pending_combat_for_attacker(
        &self,
        guild_id: &str,
        attacker_id: &str,
    ) -> Result<Option<Combat>, sqlx::Error> {
        sqlx::query_as::<_, Combat>(
            "SELECT * FROM coude_combats WHERE guild_id = $1 AND attacker_id = $2 AND status = 'pending' ORDER BY created_at DESC LIMIT 1",
        )
        .bind(guild_id)
        .bind(attacker_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn resolve_combat(
        &self,
        id: Uuid,
        status: &str,
        winner_id: Option<&str>,
        attacker_roll: Option<i32>,
        defender_roll: Option<i32>,
        chaos_event: Option<&str>,
        result_message: &str,
        coins_transferred: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE coude_combats
            SET status = $2, winner_id = $3, attacker_roll = $4, defender_roll = $5,
                chaos_event = $6, result_message = $7, coins_transferred = $8, resolved_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(winner_id)
        .bind(attacker_roll)
        .bind(defender_roll)
        .bind(chaos_event)
        .bind(result_message)
        .bind(coins_transferred)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn expire_combat(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE coude_combats SET status = 'expired', resolved_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Enregistre l'objet defensif choisi par le defenseur.
    pub async fn set_defender_special(&self, id: Uuid, item_key: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE coude_combats SET defender_special = $1 WHERE id = $2")
            .bind(item_key)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Primes ──

    pub async fn create_prime(
        &self,
        guild_id: &str,
        target_id: &str,
        target_name: &str,
        placed_by_id: &str,
        placed_by_name: &str,
        amount: i64,
    ) -> Result<Prime, sqlx::Error> {
        sqlx::query_as::<_, Prime>(
            r#"
            INSERT INTO coude_primes (guild_id, target_id, target_name, placed_by_id, placed_by_name, amount)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(guild_id)
        .bind(target_id)
        .bind(target_name)
        .bind(placed_by_id)
        .bind(placed_by_name)
        .bind(amount)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get_active_primes(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Vec<Prime>, sqlx::Error> {
        sqlx::query_as::<_, Prime>(
            "SELECT * FROM coude_primes WHERE guild_id = $1 AND target_id = $2 AND claimed = FALSE",
        )
        .bind(guild_id)
        .bind(target_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn claim_primes(
        &self,
        guild_id: &str,
        target_id: &str,
        claimed_by_id: &str,
        claimed_by_name: &str,
    ) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            r#"
            WITH claimed AS (
                UPDATE coude_primes
                SET claimed = TRUE, claimed_by_id = $3, claimed_by_name = $4, claimed_at = NOW()
                WHERE guild_id = $1 AND target_id = $2 AND claimed = FALSE
                RETURNING amount
            )
            SELECT COALESCE(SUM(amount), 0) FROM claimed
            "#,
        )
        .bind(guild_id)
        .bind(target_id)
        .bind(claimed_by_id)
        .bind(claimed_by_name)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    // ── Inventory ──

    pub async fn add_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO coude_inventory (guild_id, user_id, item_key, quantity)
            VALUES ($1, $2, $3, 1)
            ON CONFLICT (guild_id, user_id, item_key) DO UPDATE SET quantity = coude_inventory.quantity + 1
            "#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(item_key)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_inventory(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<InventoryItem>, sqlx::Error> {
        sqlx::query_as::<_, InventoryItem>(
            "SELECT * FROM coude_inventory WHERE guild_id = $1 AND user_id = $2 AND quantity > 0",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn use_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE coude_inventory
            SET quantity = quantity - 1
            WHERE guild_id = $1 AND user_id = $2 AND item_key = $3 AND quantity > 0
            "#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(item_key)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn has_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<bool, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM coude_inventory WHERE guild_id = $1 AND user_id = $2 AND item_key = $3 AND quantity > 0",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(item_key)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0 > 0)
    }

    // ── Leaderboard ──

    pub async fn leaderboard_richest(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
        sqlx::query_as::<_, LeaderboardEntry>(
            "SELECT user_id, username, coins AS value FROM coude_players WHERE guild_id = $1 ORDER BY coins DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn leaderboard_thieves(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
        sqlx::query_as::<_, LeaderboardEntry>(
            "SELECT user_id, username, total_stolen AS value FROM coude_players WHERE guild_id = $1 ORDER BY total_stolen DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn leaderboard_cowards(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
        sqlx::query_as::<_, LeaderboardEntry>(
            "SELECT user_id, username, cowardice_count::BIGINT AS value FROM coude_players WHERE guild_id = $1 ORDER BY cowardice_count DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn leaderboard_chaos(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
        sqlx::query_as::<_, LeaderboardEntry>(
            "SELECT user_id, username, chaos_events::BIGINT AS value FROM coude_players WHERE guild_id = $1 ORDER BY chaos_events DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    // ── Events ──

    pub async fn get_active_events(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ServerEvent>, sqlx::Error> {
        sqlx::query_as::<_, ServerEvent>(
            "SELECT * FROM coude_events WHERE guild_id = $1 AND active = TRUE AND expires_at > NOW()",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
    }

    // ── Bets (Paris) ──

    pub async fn get_pending_combat_for_player(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Combat>, sqlx::Error> {
        sqlx::query_as::<_, Combat>(
            "SELECT * FROM coude_combats WHERE guild_id = $1 AND (attacker_id = $2 OR defender_id = $2) AND status = 'pending' ORDER BY created_at DESC LIMIT 1",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn place_bet(
        &self,
        guild_id: &str,
        combat_id: Uuid,
        bettor_id: &str,
        bettor_name: &str,
        backed_id: &str,
        amount: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO coude_bets (guild_id, combat_id, bettor_id, bettor_name, backed_id, amount)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(guild_id)
        .bind(combat_id)
        .bind(bettor_id)
        .bind(bettor_name)
        .bind(backed_id)
        .bind(amount)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_combat_bets(&self, combat_id: Uuid) -> Result<Vec<Bet>, sqlx::Error> {
        sqlx::query_as::<_, Bet>(
            "SELECT id, combat_id, bettor_id, bettor_name, backed_id, amount FROM coude_bets WHERE combat_id = $1",
        )
        .bind(combat_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Resout les paris d'un combat. Retourne (resultats_parieurs, bonus_combattants).
    /// - Parieurs gagnants : mise + part du pot perdant (apres commission)
    /// - Combattant gagnant : +10% du pot total
    /// - Combattant perdant : +5% du pot total
    pub async fn resolve_bets(
        &self,
        combat_id: Uuid,
        winner_id: Option<&str>,
    ) -> Result<(Vec<BetResult>, Option<FighterBetBonus>), sqlx::Error> {
        let bets = self.get_combat_bets(combat_id).await?;
        if bets.is_empty() {
            return Ok((vec![], None));
        }

        let total_pot: i64 = bets.iter().map(|b| b.amount).sum();

        // Commission pour les combattants : 15% du pot (10% gagnant + 5% perdant)
        let fighter_cut = (total_pot as f64 * 0.15) as i64;
        let distributable_pot = total_pot - fighter_cut; // 85% restant pour les parieurs

        let mut results = Vec::new();
        let mut fighter_bonus: Option<FighterBetBonus> = None;

        // Recuperer le guild_id
        let guild_id_row: Option<(String,)> = sqlx::query_as(
            "SELECT guild_id FROM coude_bets WHERE combat_id = $1 LIMIT 1",
        )
        .bind(combat_id)
        .fetch_optional(&self.pool)
        .await?;
        let guild_id = guild_id_row.map(|(g,)| g).unwrap_or_default();

        match winner_id {
            Some(winner) => {
                let winner_bonus = (total_pot as f64 * 0.10) as i64;
                let loser_bonus = (total_pot as f64 * 0.05) as i64;

                fighter_bonus = Some(FighterBetBonus {
                    winner_bonus,
                    loser_bonus,
                    total_pot,
                });

                // Pools parieurs
                let winner_pool: i64 = bets
                    .iter()
                    .filter(|b| b.backed_id == winner)
                    .map(|b| b.amount)
                    .sum();
                let loser_pool: i64 = bets
                    .iter()
                    .filter(|b| b.backed_id != winner)
                    .map(|b| b.amount)
                    .sum();

                // Part du pot redistribuable pour les parieurs gagnants
                let parieur_winner_pot = if winner_pool > 0 {
                    // Parieurs gagnants recup leur mise + part proportionnelle du pot perdant (- commission)
                    let loser_pot_after_cut = (loser_pool as f64 * 0.85) as i64;
                    loser_pot_after_cut
                } else {
                    0
                };

                for bet in &bets {
                    if bet.backed_id == winner {
                        let share = if winner_pool > 0 {
                            (bet.amount as f64 / winner_pool as f64 * parieur_winner_pot as f64) as i64
                        } else {
                            0
                        };
                        let payout = bet.amount + share;
                        results.push(BetResult {
                            bettor_id: bet.bettor_id.clone(),
                            bettor_name: bet.bettor_name.clone(),
                            amount: bet.amount,
                            won: true,
                            payout,
                        });
                    } else {
                        results.push(BetResult {
                            bettor_id: bet.bettor_id.clone(),
                            bettor_name: bet.bettor_name.clone(),
                            amount: bet.amount,
                            won: false,
                            payout: 0,
                        });
                    }
                }
            }
            None => {
                // Egalite/accident : tout le monde perd, pas de bonus combattants
                for bet in &bets {
                    results.push(BetResult {
                        bettor_id: bet.bettor_id.clone(),
                        bettor_name: bet.bettor_name.clone(),
                        amount: bet.amount,
                        won: false,
                        payout: 0,
                    });
                }
            }
        }

        // Appliquer les paiements parieurs
        for result in &results {
            if result.won && result.payout > 0 {
                if !guild_id.is_empty() {
                    let _ = self.update_player_coins(&guild_id, &result.bettor_id, result.payout).await;
                }
            }
        }

        // Appliquer les bonus combattants
        if let Some(ref bonus) = fighter_bonus {
            if !guild_id.is_empty() {
                if let Some(winner) = winner_id {
                    let _ = self.update_player_coins(&guild_id, winner, bonus.winner_bonus).await;
                }
                // Trouver le perdant
                let combat: Option<Combat> = sqlx::query_as(
                    "SELECT * FROM coude_combats WHERE id = $1",
                )
                .bind(combat_id)
                .fetch_optional(&self.pool)
                .await?;

                if let Some(combat) = combat {
                    let loser_id = if Some(combat.attacker_id.as_str()) == winner_id {
                        &combat.defender_id
                    } else {
                        &combat.attacker_id
                    };
                    let _ = self.update_player_coins(&guild_id, loser_id, bonus.loser_bonus).await;
                }
            }
        }

        Ok((results, fighter_bonus))
    }

    // ── Cooldowns ──

    pub async fn check_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
    ) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
        let row: Option<(DateTime<Utc>,)> = sqlx::query_as(
            "SELECT expires_at FROM coude_cooldowns WHERE guild_id = $1 AND user_id = $2 AND action = $3 AND expires_at > NOW()",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(action)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(expires_at,)| expires_at))
    }

    pub async fn set_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
        duration_secs: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO coude_cooldowns (guild_id, user_id, action, expires_at)
            VALUES ($1, $2, $3, NOW() + make_interval(secs => $4::double precision))
            ON CONFLICT (guild_id, user_id, action) DO UPDATE SET expires_at = NOW() + make_interval(secs => $4::double precision)
            "#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(action)
        .bind(duration_secs as f64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn transfer_coins(
        &self,
        guild_id: &str,
        from_id: &str,
        to_id: &str,
        amount: i64,
    ) -> Result<(), sqlx::Error> {
        self.update_player_coins(guild_id, from_id, -amount).await?;
        self.update_player_coins(guild_id, to_id, amount).await?;
        Ok(())
    }

    // ── Insurance (Assurance) ──

    pub async fn buy_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
        is_scam: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO coude_insurances (guild_id, user_id, is_scam, expires_at)
            VALUES ($1, $2, $3, NOW() + INTERVAL '1 hour')
            "#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(is_scam)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_active_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Insurance>, sqlx::Error> {
        sqlx::query_as::<_, Insurance>(
            "SELECT id, is_scam, expires_at FROM coude_insurances WHERE guild_id = $1 AND user_id = $2 AND active = TRUE AND expires_at > NOW() ORDER BY expires_at DESC LIMIT 1",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn expire_insurance(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE coude_insurances SET active = FALSE, expires_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Expired combats helpers ──

    pub async fn get_expired_combats(&self) -> Result<Vec<Combat>, sqlx::Error> {
        sqlx::query_as::<_, Combat>(
            "SELECT * FROM coude_combats WHERE status = 'pending' AND created_at < NOW() - INTERVAL '24 hours'",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn refund_bets(&self, combat_id: Uuid) -> Result<Vec<Bet>, sqlx::Error> {
        let bets = self.get_combat_bets(combat_id).await?;

        for bet in &bets {
            // Recuperer le guild_id depuis le combat
            let guild_id_row: Option<(String,)> = sqlx::query_as(
                "SELECT guild_id FROM coude_bets WHERE id = $1",
            )
            .bind(bet.id)
            .fetch_optional(&self.pool)
            .await?;

            if let Some((guild_id,)) = guild_id_row {
                // Rembourser le parieur
                self.update_player_coins(&guild_id, &bet.bettor_id, bet.amount)
                    .await?;
            }
        }

        // Marquer les paris comme rembourses
        sqlx::query(
            "UPDATE coude_bets SET won = false, payout = amount WHERE combat_id = $1",
        )
        .bind(combat_id)
        .execute(&self.pool)
        .await?;

        Ok(bets)
    }

    pub async fn get_all_guild_ids(&self) -> Result<Vec<String>, sqlx::Error> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT guild_id FROM coude_players",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    // ── Daily Chaos ──

    pub async fn get_random_players(
        &self,
        guild_id: &str,
        count: usize,
    ) -> Result<Vec<Player>, sqlx::Error> {
        sqlx::query_as::<_, Player>(
            "SELECT * FROM coude_players WHERE guild_id = $1 AND coins > 50 ORDER BY RANDOM() LIMIT $2",
        )
        .bind(guild_id)
        .bind(count as i64)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn log_daily_chaos(
        &self,
        guild_id: &str,
        loser_id: &str,
        loser_name: &str,
        winner_id: &str,
        winner_name: &str,
        amount: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO coude_daily_chaos (guild_id, loser_id, loser_name, winner_id, winner_name, amount)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(guild_id)
        .bind(loser_id)
        .bind(loser_name)
        .bind(winner_id)
        .bind(winner_name)
        .bind(amount)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── XP / Leveling ──

    /// Ajoute de l'XP a un joueur. Retourne (new_xp, new_level, leveled_up, stat_points_gained).
    pub async fn add_xp(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<(i64, i32, bool, i32), sqlx::Error> {
        use crate::game::progression;

        // Recuperer le joueur actuel
        let player: Player = sqlx::query_as(
            "SELECT * FROM coude_players WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let mut new_xp = player.xp + amount;
        let mut new_level = player.level;
        let mut total_stat_points = 0i32;

        // Check multi-level-up
        while new_level < progression::MAX_LEVEL {
            let needed = progression::xp_for_level(new_level);
            if new_xp >= needed {
                new_xp -= needed;
                new_level += 1;
                total_stat_points += 3;
            } else {
                break;
            }
        }

        // Cap XP at max level
        if new_level >= progression::MAX_LEVEL {
            new_level = progression::MAX_LEVEL;
        }

        let leveled_up = new_level > player.level;
        let new_title = progression::title_for_level(new_level);

        sqlx::query(
            r#"
            UPDATE coude_players
            SET xp = $3, level = $4, stat_points = stat_points + $5, title = $6, updated_at = NOW()
            WHERE guild_id = $1 AND user_id = $2
            "#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(new_xp)
        .bind(new_level)
        .bind(total_stat_points)
        .bind(new_title)
        .execute(&self.pool)
        .await?;

        Ok((new_xp, new_level, leveled_up, total_stat_points))
    }

    /// Depense un point de stat. stat doit etre "atk" ou "def".
    pub async fn spend_stat_point(
        &self,
        guild_id: &str,
        user_id: &str,
        stat: &str,
    ) -> Result<Player, sqlx::Error> {
        let col = match stat {
            "atk" => "atk",
            "def" => "def",
            _ => return Err(sqlx::Error::Protocol("Stat invalide".into())),
        };

        // Verifier que le joueur a des points
        let player: Player = sqlx::query_as(
            "SELECT * FROM coude_players WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if player.stat_points < 1 {
            return Err(sqlx::Error::Protocol("Pas assez de points de statistique".into()));
        }

        let query = format!(
            "UPDATE coude_players SET {} = {} + 1, stat_points = stat_points - 1, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2 RETURNING *",
            col, col
        );

        let updated: Player = sqlx::query_as(&query)
            .bind(guild_id)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(updated)
    }

    pub async fn leaderboard_level(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
        sqlx::query_as::<_, LeaderboardEntry>(
            "SELECT user_id, username, level::BIGINT AS value FROM coude_players WHERE guild_id = $1 ORDER BY level DESC, xp DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }
}
