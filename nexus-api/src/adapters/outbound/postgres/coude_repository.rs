use super::pg_err;
use async_trait::async_trait;
use nexus_core::{
    domain::{entities::coude::PlayerClass, errors::DomainError},
    ports::outbound::coude_repository::{
        CoudeCombat, CoudeCombatResult, CoudeCombatSnapshot, CoudeProfile, CoudeRepository,
    },
};
use sqlx::PgPool;
pub struct PgCoudeRepository {
    pool: PgPool,
}

#[derive(sqlx::FromRow)]
struct ProfileRow {
    guild_id: String, user_id: String, username: String, class: String, level: i32, xp: i64,
    atk: i32, def: i32, hp_current: i32, hp_max: i32, coins: i64, stat_points: i32,
    title: String, total_wins: i32, total_losses: i32, total_draws: i32, total_stolen: i64,
    cowardice_count: i32, chaos_events: i32,
}
impl PgCoudeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
#[async_trait]
impl CoudeRepository for PgCoudeRepository {
    async fn find_profile(
        &self,
        guild: &str,
        user: &str,
    ) -> Result<Option<CoudeProfile>, DomainError> {
        let row: Option<ProfileRow> = sqlx::query_as("SELECT p.guild_id,p.user_id,p.username,p.class,p.level,p.xp,p.atk,p.def,p.hp_current,p.hp_max,COALESCE(w.coins, 0) AS coins,p.stat_points,p.title,p.total_wins,p.total_losses,p.total_draws,p.total_stolen,p.cowardice_count,p.chaos_events FROM nexus_coude_players p LEFT JOIN nexus_wallets w ON w.guild_id=p.guild_id AND w.user_id=p.user_id WHERE p.guild_id=$1 AND p.user_id=$2").bind(guild).bind(user).fetch_optional(&self.pool).await.map_err(pg_err)?;
        row.map(|row| {
                PlayerClass::parse(&row.class)
                    .map(|class| CoudeProfile {
                        guild_id: row.guild_id,
                        user_id: row.user_id,
                        username: row.username,
                        class,
                        level: row.level,
                        xp: row.xp,
                        atk: row.atk,
                        def: row.def,
                        hp_current: row.hp_current,
                        hp_max: row.hp_max,
                        coins: row.coins,
                        stat_points: row.stat_points,
                        title: row.title,
                        total_wins: row.total_wins,
                        total_losses: row.total_losses,
                        total_draws: row.total_draws,
                        total_stolen: row.total_stolen,
                        cowardice_count: row.cowardice_count,
                        chaos_events: row.chaos_events,
                    })
                    .ok_or_else(|| DomainError::Internal("classe Coude invalide".into()))
            })
        .transpose()
    }
    async fn list_combat_history(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<CoudeCombatResult>, DomainError> {
        // Uniquement les combats RESOLUS : un defi en attente n'a ni
        // vainqueur ni recit, l'afficher dans un historique n'apprendrait
        // rien. Le joueur peut etre d'un cote comme de l'autre.
        let rows: Vec<CombatRow> = sqlx::query_as(
            "SELECT id, attacker_id, attacker_name, defender_id, defender_name,                     mise, winner_id, attacker_roll, defender_roll, chaos_event,                     special_attack, result_message, coins_transferred, resolved_at              FROM nexus_coude_combats              WHERE guild_id = $1 AND (attacker_id = $2 OR defender_id = $2)                AND status = 'resolved'              ORDER BY resolved_at DESC NULLS LAST              LIMIT $3",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|r| CoudeCombatResult {
                id: r.id,
                attacker_id: r.attacker_id,
                attacker_name: r.attacker_name,
                defender_id: r.defender_id,
                defender_name: r.defender_name,
                mise: r.mise,
                winner_id: r.winner_id,
                attacker_roll: r.attacker_roll,
                defender_roll: r.defender_roll,
                chaos_event: r.chaos_event,
                special_attack: r.special_attack,
                result_message: r.result_message,
                coins_transferred: r.coins_transferred,
                resolved_at: r.resolved_at,
            })
            .collect())
    }

    async fn list_profiles(&self, guild: &str, limit: i64) -> Result<Vec<CoudeProfile>, DomainError> {
        let rows: Vec<ProfileRow> = sqlx::query_as(
            "SELECT p.guild_id,p.user_id,p.username,p.class,p.level,p.xp,p.atk,p.def,p.hp_current,p.hp_max,COALESCE(w.coins, 0) AS coins,p.stat_points,p.title,p.total_wins,p.total_losses,p.total_draws,p.total_stolen,p.cowardice_count,p.chaos_events FROM nexus_coude_players p LEFT JOIN nexus_wallets w ON w.guild_id=p.guild_id AND w.user_id=p.user_id WHERE p.guild_id=$1 ORDER BY p.level DESC, p.xp DESC LIMIT $2",
        )
        .bind(guild)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        // Une classe illisible en base ne doit pas faire echouer tout le
        // classement : on ignore la ligne fautive plutot que d'avorter.
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                PlayerClass::parse(&row.class).map(|class| CoudeProfile {
                    guild_id: row.guild_id,
                    user_id: row.user_id,
                    username: row.username,
                    class,
                    level: row.level,
                    xp: row.xp,
                    atk: row.atk,
                    def: row.def,
                    hp_current: row.hp_current,
                    hp_max: row.hp_max,
                    coins: row.coins,
                    stat_points: row.stat_points,
                    title: row.title,
                    total_wins: row.total_wins,
                    total_losses: row.total_losses,
                    total_draws: row.total_draws,
                    total_stolen: row.total_stolen,
                    cowardice_count: row.cowardice_count,
                    chaos_events: row.chaos_events,
                })
            })
            .collect())
    }
    async fn create_profile(&self, p: &CoudeProfile) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        sqlx::query("INSERT INTO nexus_coude_players (guild_id,user_id,username,class,level,xp,atk,def,hp_current,hp_max) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT (guild_id,user_id) DO NOTHING").bind(&p.guild_id).bind(&p.user_id).bind(&p.username).bind(p.class.as_str()).bind(p.level).bind(p.xp).bind(p.atk).bind(p.def).bind(p.hp_current).bind(p.hp_max).execute(&mut *tx).await.map_err(pg_err)?;
        // Coude n'a pas de monnaie propre : un nouveau joueur obtient le
        // wallet Nexus normal (et sa configuration starting_coins), une fois.
        sqlx::query("INSERT INTO nexus_wallets (guild_id,user_id,username,coins) SELECT $1,$2,$3,COALESCE((SELECT starting_coins FROM nexus_guild_config WHERE guild_id=$1),100) ON CONFLICT (guild_id,user_id) DO UPDATE SET username=CASE WHEN EXCLUDED.username <> '' THEN EXCLUDED.username ELSE nexus_wallets.username END, updated_at=NOW()").bind(&p.guild_id).bind(&p.user_id).bind(&p.username).execute(&mut *tx).await.map_err(pg_err)?;
        tx.commit().await.map_err(pg_err)?;
        Ok(())
    }
    async fn update_class(&self, guild_id: &str, user_id: &str, class: PlayerClass, atk: i32, def: i32, hp_max: i32) -> Result<(), DomainError> {
        let result = sqlx::query("UPDATE nexus_coude_players SET class=$3, atk=$4, def=$5, hp_current=$6, hp_max=$6, class_changed_at=NOW(), updated_at=NOW() WHERE guild_id=$1 AND user_id=$2")
            .bind(guild_id).bind(user_id).bind(class.as_str()).bind(atk).bind(def).bind(hp_max).execute(&self.pool).await.map_err(pg_err)?;
        if result.rows_affected() != 1 { return Err(DomainError::NotFound(format!("profil Coude {user_id}"))); }
        Ok(())
    }
    async fn spend_stat_point(&self, guild_id: &str, user_id: &str, stat: &str) -> Result<CoudeProfile, DomainError> {
        let column = match stat { "atk" => "atk", "def" => "def", _ => return Err(DomainError::Validation("stat invalide".into())) };
        let sql = format!("UPDATE nexus_coude_players SET {column}={column}+1, stat_points=stat_points-1, hp_max=CASE WHEN $3='def' THEN hp_max+10 ELSE hp_max END, hp_current=CASE WHEN $3='def' THEN LEAST(hp_current+10,hp_max+10) ELSE hp_current END, updated_at=NOW() WHERE guild_id=$1 AND user_id=$2 AND stat_points > 0");
        let result = sqlx::query(&sql).bind(guild_id).bind(user_id).bind(stat).execute(&self.pool).await.map_err(pg_err)?;
        if result.rows_affected() != 1 { return Err(DomainError::Validation("aucun point de statistique disponible".into())); }
        self.find_profile(guild_id, user_id).await?.ok_or_else(|| DomainError::NotFound(format!("profil Coude {user_id}")))
    }
    async fn set_progress(&self, guild_id: &str, user_id: &str, xp: i64, level: i32, stat_points: i32, title: &str) -> Result<(), DomainError> { sqlx::query("UPDATE nexus_coude_players SET xp=$3,level=$4,stat_points=$5,title=$6,updated_at=NOW() WHERE guild_id=$1 AND user_id=$2").bind(guild_id).bind(user_id).bind(xp).bind(level).bind(stat_points).bind(title).execute(&self.pool).await.map_err(pg_err)?; Ok(()) }
    async fn create_combat(
        &self,
        guild_id: &str,
        channel_id: &str,
        attacker: &CoudeProfile,
        defender: &CoudeProfile,
        mise: i64,
    ) -> Result<CoudeCombat, DomainError> {
        if mise <= 0 {
            return Err(DomainError::Validation("mise invalide".into()));
        }
        let row: (uuid::Uuid, String, String, String, i64, String) = sqlx::query_as("INSERT INTO nexus_coude_combats (guild_id,channel_id,attacker_id,attacker_name,defender_id,defender_name,mise,expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7,NOW()+INTERVAL '24 hours') RETURNING id,guild_id,attacker_id,defender_id,mise,status")
            .bind(guild_id).bind(channel_id).bind(&attacker.user_id).bind(&attacker.username).bind(&defender.user_id).bind(&defender.username).bind(mise).fetch_one(&self.pool).await.map_err(pg_err)?;
        Ok(CoudeCombat {
            id: row.0,
            guild_id: row.1,
            attacker_id: row.2,
            defender_id: row.3,
            mise: row.4,
            status: row.5,
        })
    }
    async fn accept_combat(&self, id: uuid::Uuid, defender_id: &str) -> Result<bool, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let combat: Option<(String, String, String, i64)> = sqlx::query_as(
            "SELECT guild_id, attacker_id, defender_id, mise FROM nexus_coude_combats WHERE id=$1 AND defender_id=$2 AND status='pending' AND (expires_at IS NULL OR expires_at > NOW()) FOR UPDATE",
        ).bind(id).bind(defender_id).fetch_optional(&mut *tx).await.map_err(pg_err)?;
        let Some((guild_id, attacker_id, defender_id, mise)) = combat else { return Ok(false); };
        // Lock both balances before accepting: neither participant can enter a
        // duel they are unable to settle.
        let balances: Vec<(String, i64)> = sqlx::query_as(
            "SELECT user_id, coins FROM nexus_wallets WHERE guild_id=$1 AND user_id IN ($2, $3) ORDER BY user_id FOR UPDATE",
        ).bind(&guild_id).bind(&attacker_id).bind(&defender_id).fetch_all(&mut *tx).await.map_err(pg_err)?;
        if balances.len() != 2 || balances.iter().any(|(_, coins)| *coins < mise) {
            return Err(DomainError::Validation("coins insuffisants pour accepter ce defi".into()));
        }
        let result = sqlx::query("UPDATE nexus_coude_combats SET status='accepted' WHERE id=$1 AND status='pending'")
            .bind(id).execute(&mut *tx).await.map_err(pg_err)?;
        tx.commit().await.map_err(pg_err)?;
        Ok(result.rows_affected() == 1)
    }
    async fn refuse_combat(&self, id: uuid::Uuid, defender_id: &str) -> Result<bool, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let result: Option<(String,)> = sqlx::query_as("UPDATE nexus_coude_combats SET status='refused', resolved_at=NOW() WHERE id=$1 AND defender_id=$2 AND status='pending' RETURNING guild_id")
            .bind(id).bind(defender_id).fetch_optional(&mut *tx).await.map_err(pg_err)?;
        let Some((guild_id,)) = result else { return Ok(false); };
        sqlx::query("UPDATE nexus_coude_players SET cowardice_count=cowardice_count+1,updated_at=NOW() WHERE guild_id=$1 AND user_id=$2")
            .bind(guild_id).bind(defender_id).execute(&mut *tx).await.map_err(pg_err)?;
        tx.commit().await.map_err(pg_err)?;
        Ok(true)
    }
    async fn resolution_snapshot(&self, id: uuid::Uuid) -> Result<Option<CoudeCombatSnapshot>, DomainError> {
        let row: Option<(uuid::Uuid, String, String, String, i64, String)> = sqlx::query_as("SELECT id,guild_id,attacker_id,defender_id,mise,status FROM nexus_coude_combats WHERE id=$1 AND status='accepted'")
            .bind(id).fetch_optional(&self.pool).await.map_err(pg_err)?;
        let Some((id, guild_id, attacker_id, defender_id, mise, status)) = row else { return Ok(None); };
        let Some(attacker) = self.find_profile(&guild_id, &attacker_id).await? else { return Err(DomainError::NotFound(format!("profil Coude {attacker_id}"))); };
        let Some(defender) = self.find_profile(&guild_id, &defender_id).await? else { return Err(DomainError::NotFound(format!("profil Coude {defender_id}"))); };
        Ok(Some(CoudeCombatSnapshot { combat: CoudeCombat { id, guild_id, attacker_id, defender_id, mise, status }, attacker, defender }))
    }
    async fn resolve_combat(
        &self,
        id: uuid::Uuid,
        winner_id: Option<&str>,
        attacker_roll: i32,
        defender_roll: i32,
        transferred: i64,
        attacker_hp: i32,
        defender_hp: i32,
    ) -> Result<bool, DomainError> {
        if !(1..=6).contains(&attacker_roll) || !(1..=6).contains(&defender_roll) || transferred < 0 {
            return Err(DomainError::Validation("resultat de duel invalide".into()));
        }
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let combat: Option<(String, String, String, i64)> = sqlx::query_as(
            "SELECT guild_id, attacker_id, defender_id, mise FROM nexus_coude_combats WHERE id=$1 AND status='accepted' FOR UPDATE",
        ).bind(id).fetch_optional(&mut *tx).await.map_err(pg_err)?;
        let Some((guild_id, attacker_id, defender_id, mise)) = combat else { return Ok(false); };
        let valid_winner = winner_id.is_none() || winner_id == Some(attacker_id.as_str()) || winner_id == Some(defender_id.as_str());
        if !valid_winner || transferred != if winner_id.is_some() { mise } else { 0 } {
            return Err(DomainError::Validation("resultat de duel incoherent".into()));
        }
        if let Some(winner) = winner_id {
            let loser = if winner == attacker_id { defender_id.as_str() } else { attacker_id.as_str() };
            let debit = sqlx::query("UPDATE nexus_wallets SET coins=coins-$1, total_spent=total_spent+$1, updated_at=NOW() WHERE guild_id=$2 AND user_id=$3 AND coins >= $1")
                .bind(transferred).bind(&guild_id).bind(loser).execute(&mut *tx).await.map_err(pg_err)?;
            if debit.rows_affected() != 1 { return Err(DomainError::Validation("coins insuffisants pour regler ce duel".into())); }
            sqlx::query("UPDATE nexus_wallets SET coins=coins+$1, total_earned=total_earned+$1, updated_at=NOW() WHERE guild_id=$2 AND user_id=$3")
                .bind(transferred).bind(&guild_id).bind(winner).execute(&mut *tx).await.map_err(pg_err)?;
            let bounties: Vec<(i64,)> = sqlx::query_as("UPDATE nexus_coude_primes SET claimed=TRUE,claimed_by_id=$3,claimed_at=NOW() WHERE guild_id=$1 AND target_id=$2 AND claimed=FALSE RETURNING amount")
                .bind(&guild_id).bind(loser).bind(winner).fetch_all(&mut *tx).await.map_err(pg_err)?;
            let bounty: i64 = bounties.into_iter().map(|(amount,)| amount).sum();
            if bounty > 0 {
                sqlx::query("UPDATE nexus_wallets SET coins=coins+$1,total_earned=total_earned+$1,updated_at=NOW() WHERE guild_id=$2 AND user_id=$3")
                    .bind(bounty).bind(&guild_id).bind(winner).execute(&mut *tx).await.map_err(pg_err)?;
            }
            sqlx::query("UPDATE nexus_coude_players SET total_wins=total_wins+1 WHERE guild_id=$1 AND user_id=$2")
                .bind(&guild_id).bind(winner).execute(&mut *tx).await.map_err(pg_err)?;
            sqlx::query("UPDATE nexus_coude_players SET total_losses=total_losses+1 WHERE guild_id=$1 AND user_id=$2")
                .bind(&guild_id).bind(loser).execute(&mut *tx).await.map_err(pg_err)?;
            for (user_id, amount, source) in [(loser, -transferred, "coude_loss"), (winner, transferred, "coude_win")] {
                let (balance_after,): (i64,) = sqlx::query_as("SELECT coins FROM nexus_wallets WHERE guild_id=$1 AND user_id=$2").bind(&guild_id).bind(user_id).fetch_one(&mut *tx).await.map_err(pg_err)?;
                sqlx::query("INSERT INTO nexus_wallet_transactions (guild_id,user_id,amount,balance_after,source,description) VALUES ($1,$2,$3,$4,$5,'Coup de Coude')").bind(&guild_id).bind(user_id).bind(amount).bind(balance_after).bind(source).execute(&mut *tx).await.map_err(pg_err)?;
            }
            let winners: Vec<(String, i64)> = sqlx::query_as("UPDATE nexus_coude_bets SET won=TRUE,payout=amount*2 WHERE combat_id=$1 AND backed_id=$2 RETURNING bettor_id,payout")
                .bind(id).bind(winner).fetch_all(&mut *tx).await.map_err(pg_err)?;
            sqlx::query("UPDATE nexus_coude_bets SET won=FALSE,payout=0 WHERE combat_id=$1 AND backed_id<>$2")
                .bind(id).bind(winner).execute(&mut *tx).await.map_err(pg_err)?;
            for (bettor, payout) in winners { sqlx::query("UPDATE nexus_wallets SET coins=coins+$3,total_earned=total_earned+$3 WHERE guild_id=$1 AND user_id=$2").bind(&guild_id).bind(bettor).bind(payout).execute(&mut *tx).await.map_err(pg_err)?; }
        } else {
            sqlx::query("UPDATE nexus_coude_players SET total_draws=total_draws+1 WHERE guild_id=$1 AND user_id IN ($2, $3)")
                .bind(&guild_id).bind(&attacker_id).bind(&defender_id).execute(&mut *tx).await.map_err(pg_err)?;
            let refunds: Vec<(String, i64)> = sqlx::query_as("UPDATE nexus_coude_bets SET won=FALSE,payout=amount WHERE combat_id=$1 RETURNING bettor_id,payout").bind(id).fetch_all(&mut *tx).await.map_err(pg_err)?;
            for (bettor, payout) in refunds { sqlx::query("UPDATE nexus_wallets SET coins=coins+$3,total_earned=total_earned+$3 WHERE guild_id=$1 AND user_id=$2").bind(&guild_id).bind(bettor).bind(payout).execute(&mut *tx).await.map_err(pg_err)?; }
        }
        sqlx::query("UPDATE nexus_coude_players SET hp_current=$3,updated_at=NOW() WHERE guild_id=$1 AND user_id=$2")
            .bind(&guild_id).bind(&attacker_id).bind(attacker_hp.max(0)).execute(&mut *tx).await.map_err(pg_err)?;
        sqlx::query("UPDATE nexus_coude_players SET hp_current=$3,updated_at=NOW() WHERE guild_id=$1 AND user_id=$2")
            .bind(&guild_id).bind(&defender_id).bind(defender_hp.max(0)).execute(&mut *tx).await.map_err(pg_err)?;
        let result = sqlx::query("UPDATE nexus_coude_combats SET status='resolved', winner_id=$2, attacker_roll=$3, defender_roll=$4, coins_transferred=$5, resolved_at=NOW() WHERE id=$1 AND status='accepted'")
            .bind(id).bind(winner_id).bind(attacker_roll).bind(defender_roll).bind(transferred).execute(&mut *tx).await.map_err(pg_err)?;
        tx.commit().await.map_err(pg_err)?;
        Ok(result.rows_affected() == 1)
    }
}

#[derive(sqlx::FromRow)]
struct CombatRow {
    id: uuid::Uuid,
    attacker_id: String,
    attacker_name: String,
    defender_id: String,
    defender_name: String,
    mise: i64,
    winner_id: Option<String>,
    attacker_roll: Option<i32>,
    defender_roll: Option<i32>,
    chaos_event: Option<String>,
    special_attack: Option<String>,
    result_message: Option<String>,
    coins_transferred: i64,
    resolved_at: Option<chrono::DateTime<chrono::Utc>>,
}
