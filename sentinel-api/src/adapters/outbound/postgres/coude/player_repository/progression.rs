//! Methodes de progression : class, XP, stat points, reset stats.
//!
//! Sous-module du repository Postgres coude_players. Les fonctions
//! prennent `&PgPlayerRepository` en argument ; le trait impl
//! dans `mod.rs` delegate ici.

use crate::adapters::outbound::postgres::casino::wallet_tx_log::log_wallet_tx;
use sentinel_core::domain::entities::coude::player::title_for_level as coude_title_for_level;
use sentinel_core::domain::entities::coude::player::xp_for_level as coude_xp_for_level;
use sentinel_core::domain::entities::coude::player::CombatStat;
use sentinel_core::domain::entities::coude::player::Player;
use sentinel_core::domain::entities::coude::player::XpProgress;
use sentinel_core::domain::entities::coude::player::COUDE_MAX_LEVEL;
use sentinel_core::domain::errors::DomainError;

use super::super::super::pg_err;
use super::PgPlayerRepository;
use super::PlayerRow;
use super::PLAYER_COLUMNS;
pub(super) async fn update_class(
    repo: &PgPlayerRepository,
    guild_id: &str,
    user_id: &str,
    class: &str,
) -> Result<bool, DomainError> {
    // Phase 2 A.3 — la colonne est maintenant un enum Postgres `coude_class`,
    // on cast explicitement le bind string vers l'enum.
    let result = sqlx::query(
        "UPDATE coude_players SET class = $3::coude_class, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(class)
    .execute(&repo.pool)
    .await
    .map_err(pg_err)?;
    Ok(result.rows_affected() > 0)
}

pub(super) async fn add_xp(
    repo: &PgPlayerRepository,
    guild_id: &str,
    user_id: &str,
    amount: i64,
) -> Result<Option<XpProgress>, DomainError> {
    let mut tx = repo.pool.begin().await.map_err(pg_err)?;

    let row: Option<(i64, i32, i32)> = sqlx::query_as(
        "SELECT xp, level, stat_points
         FROM coude_players
         WHERE guild_id = $1 AND user_id = $2
         FOR UPDATE",
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(pg_err)?;

    let Some((mut current_xp, mut current_level, mut current_stat_points)) = row else {
        return Ok(None);
    };

    let old_level = current_level;
    current_xp += amount;

    // Application déterministe du barème de niveaux du domaine.
    while current_level < COUDE_MAX_LEVEL && current_xp >= coude_xp_for_level(current_level + 1) {
        current_level += 1;
        current_stat_points += 3;
    }

    let leveled_up = current_level > old_level;
    let stat_points_gained = (current_level - old_level) * 3;
    let new_title = coude_title_for_level(current_level);

    sqlx::query(
        "UPDATE coude_players
         SET xp = $3, level = $4, stat_points = $5, title = $6, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(current_xp)
    .bind(current_level)
    .bind(current_stat_points)
    .bind(new_title)
    .execute(&mut *tx)
    .await
    .map_err(pg_err)?;

    tx.commit().await.map_err(pg_err)?;

    Ok(Some(XpProgress {
        new_xp: current_xp,
        new_level: current_level,
        leveled_up,
        stat_points_gained,
    }))
}

pub(super) async fn spend_stat_point(
    repo: &PgPlayerRepository,
    guild_id: &str,
    user_id: &str,
    stat: CombatStat,
) -> Result<Option<Player>, DomainError> {
    // `stat.column()` retourne uniquement "atk" ou "def" — sûr à interpoler.
    // L'UPDATE a besoin de l'alias `cp` parce que PLAYER_COLUMNS reference
    // `cp.guild_id` / `cp.user_id` dans la sous-requete wallet — sans
    // cet alias sur la table cible, Postgres jette "missing FROM-clause
    // entry for table cp" et toute la commande /train tombe en 500.
    // Investir en DEF gonfle aussi hp_max (+2 HP) et hp_current (le joueur
    // profite immediatement du bonus). Invariant : `hp_max` DB doit toujours
    // correspondre a `100 + effective_def * 2` pour que /repos restaure au
    // bon max et que le moteur de combat ne cape pas les HP a tort.
    let col = stat.column();
    let hp_delta = if col == "def" {
        ", hp_max = hp_max + 2, hp_current = hp_current + 2"
    } else {
        ""
    };
    let sql = format!(
        r#"UPDATE coude_players AS cp
           SET {col} = {col} + 1, stat_points = stat_points - 1{hp_delta}, updated_at = NOW()
           WHERE cp.guild_id = $1 AND cp.user_id = $2 AND cp.stat_points >= 1
           RETURNING {cols}"#,
        col = col,
        hp_delta = hp_delta,
        cols = PLAYER_COLUMNS
    );
    let row: Option<PlayerRow> = sqlx::query_as(&sql)
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&repo.pool)
        .await
        .map_err(pg_err)?;
    Ok(row.map(Into::into))
}

pub(super) async fn reset_stats(
    repo: &PgPlayerRepository,
    guild_id: &str,
    user_id: &str,
    cost: i64,
) -> Result<Option<Player>, DomainError> {
    let mut tx = repo.pool.begin().await.map_err(pg_err)?;

    // Verifier que le wallet a assez de coins pour payer le reset (lock).
    let wallet_coins: Option<i64> = sqlx::query_scalar(
        "SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(pg_err)?;

    let balance = wallet_coins.unwrap_or(0);
    if balance < cost {
        tx.commit().await.map_err(pg_err)?;
        return Ok(None);
    }

    // Reset les stats dans coude_players. Retrancher `def * 2` de hp_max
    // (chaque point DEF avait ajoute +2 HP via spend_stat_point) et clamper
    // hp_current au nouveau plafond — sinon le joueur conserverait une
    // barre HP > hp_max, ce que /repos et le moteur de combat n'acceptent pas.
    sqlx::query(
        r#"UPDATE coude_players
           SET stat_points = stat_points + atk + def,
               hp_max = hp_max - (def * 2),
               hp_current = LEAST(hp_current, hp_max - (def * 2)),
               atk = 0, def = 0, updated_at = NOW()
           WHERE guild_id = $1 AND user_id = $2 AND (atk > 0 OR def > 0)"#,
    )
    .bind(guild_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(pg_err)?;

    // Debiter le cout du reset sur le wallet partage.
    let balance_after: i64 = sqlx::query_scalar(
        "UPDATE user_wallets SET coins = coins - $3, total_spent = total_spent + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2
         RETURNING coins",
    )
    .bind(guild_id).bind(user_id).bind(cost)
    .fetch_one(&mut *tx).await.map_err(pg_err)?;

    log_wallet_tx(
        &mut tx,
        guild_id,
        user_id,
        -cost,
        balance_after,
        "coude_reset_stats",
        "Reset des stats",
    )
    .await?;

    // Re-fetch le joueur avec les coins a jour.
    let sql = format!(
        "SELECT {cols} FROM coude_players cp WHERE cp.guild_id = $1 AND cp.user_id = $2",
        cols = PLAYER_COLUMNS
    );
    let row: Option<PlayerRow> = sqlx::query_as(&sql)
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_err)?;

    tx.commit().await.map_err(pg_err)?;
    Ok(row.map(Into::into))
}
