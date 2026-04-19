//! Job worker : resout le tournoi hebdo le dimanche a partir de 23h UTC.
//!
//! Pragmatique, version squelette (migration 139) :
//!   - Trouve toutes les guilds ayant wallet_transactions cette semaine.
//!   - Si on est dimanche >= 23h UTC : insere la ligne de resolution avec
//!     top 1 et un prize_amount = 10% du balance coude_cashbox.
//!   - Ne debite pas la caisse ni ne credite le gagnant ici — laisse ca
//!     a un followup (cela demande de passer par le use case
//!     manage_coude_economy qui expose des credits auditables).
//!
//! Unicite : contrainte UNIQUE (guild_id, week_start) empeche le double
//! insert si le worker tick 2x le meme dimanche.

use chrono::{Datelike, Duration, TimeZone, Timelike, Utc};
use sqlx::PgPool;
use tracing::{info, warn};

pub async fn run(pool: &PgPool) -> Result<(), String> {
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
        // Check si deja resolu.
        let exists: Option<(i32,)> = sqlx::query_as(
            r#"
            SELECT 1 FROM coude_weekly_tournaments
            WHERE guild_id = $1 AND week_start = $2
            "#,
        )
        .bind(&guild_id)
        .bind(week_start)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("check exists: {e}"))?;

        if exists.is_some() {
            continue;
        }

        // Top 1 de la semaine.
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
        .bind(&guild_id)
        .bind(week_start)
        .bind(week_end)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("winner query: {e}"))?;

        let (winner_user_id, winner_net) = match winner {
            Some(w) => w,
            None => continue,
        };

        // Username.
        let username: Option<String> = sqlx::query_scalar(
            "SELECT username FROM user_wallets WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(&guild_id)
        .bind(&winner_user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        // Prize = 10% de la caisse (skeleton, pas lu depuis config).
        let cashbox: Option<i64> = sqlx::query_scalar(
            "SELECT balance FROM coude_cashbox WHERE guild_id = $1",
        )
        .bind(&guild_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
        let prize = cashbox.unwrap_or(0) / 10;

        // Insert (ignore si deja present grace a UNIQUE).
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
        .bind(&guild_id)
        .bind(week_start)
        .bind(week_end)
        .bind(&winner_user_id)
        .bind(username.clone())
        .bind(winner_net)
        .bind(prize)
        .execute(pool)
        .await
        .map_err(|e| format!("insert tournament: {e}"))?;

        if insert_res.rows_affected() > 0 {
            info!(
                guild_id = %guild_id,
                winner = %winner_user_id,
                net = winner_net,
                prize,
                "Tournoi hebdo resolu"
            );
        } else {
            warn!(guild_id = %guild_id, "tournoi deja resolu pour cette semaine");
        }
    }

    Ok(())
}
