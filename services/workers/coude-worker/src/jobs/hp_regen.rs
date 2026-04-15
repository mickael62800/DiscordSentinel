//! Regen passive des HP des joueurs Coup de Coude.
//!
//! Taux degressifs par paliers de % HP courant :
//! - [0, 25 %)   : 100 HP/h
//! - [25, 50 %)  : 50  HP/h
//! - [50, 75 %)  : 30  HP/h
//! - [75, 100 %) : 10  HP/h
//!
//! Le calcul se base sur `hp_last_regen` : on multiplie le taux du palier
//! par le nombre d'heures ecoulees depuis la derniere mise a jour, on floor
//! a l'entier et on n'update que si le gain est >= 1. Ca garantit :
//! - aucun "tick d'overflow" si le worker est redemarre rapidement ;
//! - aucune perte de secondes si l'intervalle > taux (les fractions
//!   s'accumulent via `hp_last_regen` qui reste figee tant que le gain < 1).
//!
//! Le palier est recalcule a partir du HP AVANT ce tick — simpliste mais
//! suffisant a la granularite d'un tick de quelques minutes.

use sqlx::PgPool;
use tracing::{debug, warn};

// Taux par defaut (HP/h). Configurables via env.
const DEFAULT_RATE_0_25: f64 = 100.0;
const DEFAULT_RATE_25_50: f64 = 50.0;
const DEFAULT_RATE_50_75: f64 = 30.0;
const DEFAULT_RATE_75_100: f64 = 10.0;

pub async fn run(pool: &PgPool) -> Result<(), String> {
    let rate_0_25 = env_rate("HP_REGEN_RATE_0_25", DEFAULT_RATE_0_25);
    let rate_25_50 = env_rate("HP_REGEN_RATE_25_50", DEFAULT_RATE_25_50);
    let rate_50_75 = env_rate("HP_REGEN_RATE_50_75", DEFAULT_RATE_50_75);
    let rate_75_100 = env_rate("HP_REGEN_RATE_75_100", DEFAULT_RATE_75_100);

    // Un seul UPDATE global base sur une CTE qui precalcule le gain par
    // joueur. On met a jour hp_last_regen seulement pour les lignes qui
    // ont effectivement gagne au moins 1 HP, sinon la fraction serait
    // perdue a chaque tick (le joueur en haut palier avec 10 HP/h et un
    // tick de 5 min gagnerait 0.83 HP -> flooror a 0 -> jamais de regen).
    //
    // Exclusion : on skip les joueurs avec un combat en cours (pending /
    // betting / resolving) sinon le regen peut ecraser un hp_current frais
    // pose par une resolution de combat concurrente. Le joueur recupere
    // naturellement son HP au prochain tick apres la fin du combat.
    let result = sqlx::query(
        r#"
        WITH regen AS (
            SELECT
                guild_id,
                user_id,
                FLOOR(
                    (CASE
                        WHEN hp_current * 4 < hp_max THEN $1::float8
                        WHEN hp_current * 2 < hp_max THEN $2::float8
                        WHEN hp_current * 4 < hp_max * 3 THEN $3::float8
                        ELSE $4::float8
                    END) * EXTRACT(EPOCH FROM (NOW() - hp_last_regen)) / 3600.0
                )::int AS amount
            FROM coude_players p
            WHERE hp_current < hp_max
              AND hp_last_regen IS NOT NULL
              AND NOT EXISTS (
                  SELECT 1 FROM coude_combats c
                  WHERE c.guild_id = p.guild_id
                    AND (c.attacker_id = p.user_id OR c.defender_id = p.user_id)
                    AND c.status IN ('pending', 'betting', 'resolving')
              )
        )
        UPDATE coude_players p
        SET hp_current = LEAST(p.hp_max, p.hp_current + r.amount),
            hp_last_regen = NOW(),
            updated_at = NOW()
        FROM regen r
        WHERE p.guild_id = r.guild_id
          AND p.user_id = r.user_id
          AND r.amount > 0
        "#,
    )
    .bind(rate_0_25)
    .bind(rate_25_50)
    .bind(rate_50_75)
    .bind(rate_75_100)
    .execute(pool)
    .await
    .map_err(|e| format!("hp_regen update: {e}"))?;

    let affected = result.rows_affected();
    if affected > 0 {
        debug!(affected, "hp_regen: {} joueurs regen", affected);
    }
    Ok(())
}

fn env_rate(key: &str, default: f64) -> f64 {
    match std::env::var(key) {
        Ok(v) => v.parse::<f64>().unwrap_or_else(|_| {
            warn!(key, value = %v, "hp_regen: env var invalide, fallback default");
            default
        }),
        Err(_) => default,
    }
}
