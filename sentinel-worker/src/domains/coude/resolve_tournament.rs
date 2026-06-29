//! Job worker : resout le tournoi hebdo le dimanche a partir de 23h UTC.
//!
//! Le dimanche >= 23h UTC :
//!   - Pour chaque guild ayant de l'activite cette semaine :
//!     - Top 1 par sum(wallet_transactions.amount) sur la semaine
//!     - Prize = balance(coude_cashbox) * tournament_prize_pct/100
//!       (default 10%, configurable par guild via bot_guild_config)
//!     - Debite la caisse, credite le wallet du gagnant, log
//!       wallet_transactions (source=tournament_prize)
//!     - Insere la ligne coude_weekly_tournaments en status=resolved
//!
//! Idempotent : UNIQUE (guild_id, week_start) empeche le double-run.

use chrono::{Datelike, Duration, TimeZone, Timelike, Utc};
use sqlx::PgPool;
use tracing::{info, warn};

const DEFAULT_PRIZE_PCT: i64 = 10;

pub async fn run(pool: &PgPool, redis: &redis::Client) -> Result<(), String> {
    let now = Utc::now();

    // Dimanche = 6 (Mon=0..Sun=6 via num_days_from_monday) et >= 23h UTC.
    if now.weekday().num_days_from_monday() != 6 || now.hour() < 23 {
        return Ok(());
    }

    // Bornes de la semaine qui vient de s'ecouler (ce lundi 00h -> aujourd'hui dimanche 23:59:59).
    let dow = now.weekday().num_days_from_monday() as i64;
    let start_date = now.date_naive() - Duration::days(dow);
    let week_start = Utc
        .from_utc_datetime(&start_date.and_hms_opt(0, 0, 0).unwrap())
        .to_utc();
    let week_end = week_start + Duration::days(7) - Duration::seconds(1);

    // Liste des guilds avec de l'activite cette semaine.
    let guilds: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT DISTINCT guild_id
        FROM wallet_transactions
        WHERE created_at >= $1 AND created_at <= $2
        "#,
    )
    .bind(week_start)
    .bind(week_end)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("list guilds: {e}"))?;

    for (guild_id,) in guilds {
        if let Err(e) = resolve_guild(pool, redis, &guild_id, week_start, week_end).await {
            warn!(guild_id = %guild_id, error = %e, "tournament resolution failed");
        }
    }

    Ok(())
}

async fn resolve_guild(
    pool: &PgPool,
    redis: &redis::Client,
    guild_id: &str,
    week_start: chrono::DateTime<Utc>,
    week_end: chrono::DateTime<Utc>,
) -> Result<(), String> {
    // Check si deja resolu (skip sans log).
    let exists: Option<(i32,)> = sqlx::query_as(
        r#"SELECT 1 FROM coude_weekly_tournaments
           WHERE guild_id = $1 AND week_start = $2"#,
    )
    .bind(guild_id)
    .bind(week_start)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("check exists: {e}"))?;
    if exists.is_some() {
        return Ok(());
    }

    // Tournoi desactive par config ? → on ne resout pas.
    let tournament_enabled: bool = get_config_value(pool, guild_id, "tournament_enabled")
        .await
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
        .unwrap_or(true);
    if !tournament_enabled {
        return Ok(());
    }

    // Top 1 de la semaine (par sum des amounts positifs sur wallet_transactions).
    let winner: Option<(String, i64)> = sqlx::query_as(
        r#"
        SELECT user_id, COALESCE(SUM(amount), 0)::BIGINT
        FROM wallet_transactions
        WHERE guild_id = $1 AND created_at >= $2 AND created_at <= $3
        GROUP BY user_id
        ORDER BY 2 DESC
        LIMIT 1
        "#,
    )
    .bind(guild_id)
    .bind(week_start)
    .bind(week_end)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("winner query: {e}"))?;

    let (winner_user_id, winner_net) = match winner {
        Some(w) => w,
        None => return Ok(()),
    };

    // Username.
    let username: Option<String> = sqlx::query_scalar(
        "SELECT username FROM user_wallets WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(guild_id)
    .bind(&winner_user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    // Prize = pct% de la caisse (configurable).
    let prize_pct: i64 = get_config_value(pool, guild_id, "tournament_prize_pct")
        .await
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(DEFAULT_PRIZE_PCT)
        .clamp(0, 100);

    let cashbox_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM coude_cashbox WHERE guild_id = $1")
            .bind(guild_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .unwrap_or(0);

    let prize = (cashbox_balance * prize_pct) / 100;

    // Transaction atomique : insere le tournoi + debit caisse + credit wallet
    // + log wallet_transactions. Si une etape echoue on rollback et la
    // semaine reste "non resolue" → prochaine tick reessaie.
    let mut tx = pool.begin().await.map_err(|e| format!("begin tx: {e}"))?;

    let insert_res = sqlx::query(
        r#"
        INSERT INTO coude_weekly_tournaments (
            guild_id, week_start, week_end,
            winner_user_id, winner_username, winner_net_gain,
            prize_amount, status, resolved_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'resolved', NOW())
        ON CONFLICT (guild_id, week_start) DO NOTHING
        "#,
    )
    .bind(guild_id)
    .bind(week_start)
    .bind(week_end)
    .bind(&winner_user_id)
    .bind(username.clone())
    .bind(winner_net)
    .bind(prize)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("insert tournament: {e}"))?;

    if insert_res.rows_affected() == 0 {
        // Une autre instance a deja resolu cette semaine.
        tx.rollback().await.ok();
        return Ok(());
    }

    // Debit/credit seulement si prize > 0 et cashbox suffisante.
    if prize > 0 && cashbox_balance >= prize {
        // Debit caisse.
        sqlx::query(
            r#"UPDATE coude_cashbox
               SET balance = balance - $2,
                   total_redistributed = total_redistributed + $2,
                   last_redistribution_at = NOW()
               WHERE guild_id = $1"#,
        )
        .bind(guild_id)
        .bind(prize)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("debit cashbox: {e}"))?;

        // Credit wallet du gagnant (cree si absent).
        let new_balance: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO user_wallets (guild_id, user_id, username, coins, total_earned)
            VALUES ($1, $2, $3, $4, $4)
            ON CONFLICT (guild_id, user_id) DO UPDATE
              SET coins = user_wallets.coins + EXCLUDED.coins,
                  total_earned = user_wallets.total_earned + EXCLUDED.total_earned,
                  updated_at = NOW()
            RETURNING coins
            "#,
        )
        .bind(guild_id)
        .bind(&winner_user_id)
        .bind(username.clone().unwrap_or_default())
        .bind(prize)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("credit wallet: {e}"))?;

        // Log wallet_transactions.
        sqlx::query(
            r#"
            INSERT INTO wallet_transactions (
                guild_id, user_id, amount, balance_after, source, description
            )
            VALUES ($1, $2, $3, $4, 'tournament_prize', $5)
            "#,
        )
        .bind(guild_id)
        .bind(&winner_user_id)
        .bind(prize)
        .bind(new_balance)
        .bind(format!(
            "Prix hebdo tournoi {}",
            week_start.format("%Y-%m-%d")
        ))
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("log wallet_transactions: {e}"))?;
    }

    tx.commit().await.map_err(|e| format!("commit tx: {e}"))?;

    info!(
        guild_id = %guild_id,
        winner = %winner_user_id,
        net = winner_net,
        prize,
        prize_pct,
        "Tournoi hebdo resolu + prix distribue"
    );

    // Top 5 de la semaine pour l'embed Discord (ordonne par net_gain desc).
    let top5_rows: Vec<(String, Option<String>, i64)> = sqlx::query_as(
        r#"
        SELECT wt.user_id,
               uw.username,
               COALESCE(SUM(wt.amount), 0)::BIGINT AS net_gain
        FROM wallet_transactions wt
        LEFT JOIN user_wallets uw
               ON uw.guild_id = wt.guild_id AND uw.user_id = wt.user_id
        WHERE wt.guild_id = $1 AND wt.created_at >= $2 AND wt.created_at <= $3
        GROUP BY wt.user_id, uw.username
        ORDER BY net_gain DESC
        LIMIT 5
        "#,
    )
    .bind(guild_id)
    .bind(week_start)
    .bind(week_end)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let top5_json: Vec<serde_json::Value> = top5_rows
        .into_iter()
        .map(|(user_id, username, net_gain)| {
            serde_json::json!({
                "user_id": user_id,
                "username": username,
                "net_gain": net_gain,
            })
        })
        .collect();

    // Publie l'event Redis pour que sentinel-bot post l'embed (pattern Phase 5B).
    let event_payload = serde_json::json!({
        "event": "tournament_resolved",
        "data": {
            "guild_id": guild_id,
            "winner_user_id": winner_user_id,
            "winner_username": username,
            "winner_net_gain": winner_net,
            "prize_amount": prize,
            "prize_pct": prize_pct,
            "week_start": week_start.to_rfc3339(),
            "week_end": week_end.to_rfc3339(),
            "top5": top5_json,
        }
    });

    match redis.get_multiplexed_async_connection().await {
        Ok(mut conn) => {
            let res =
                crate::common::redis_helpers::xadd_event(&mut conn, &event_payload.to_string())
                    .await;
            if let Err(e) = res {
                warn!(error = %e, guild_id = %guild_id, "XADD tournament_resolved failed");
            } else {
                info!(guild_id = %guild_id, "tournament_resolved event publie");
            }
        }
        Err(e) => warn!(error = %e, "Redis connect failed, XADD tournament_resolved skip"),
    }

    Ok(())
}

/// Lit une valeur de `bot_guild_config` pour `bot_name = 'coude-bot'`.
async fn get_config_value(pool: &PgPool, guild_id: &str, key: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>(
        r#"SELECT config_value FROM bot_guild_config
           WHERE guild_id = $1 AND bot_name = 'coude-bot' AND config_key = $2"#,
    )
    .bind(guild_id)
    .bind(key)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}
