use chrono::Datelike;
use sqlx::PgPool;
use tracing::{info, warn};

/// Phase 2 A.4 — Partition manager pour les tables event-heavy partitionnees.
///
/// Pour chaque table de la liste, ce job verifie qu'une partition existe
/// pour les 2 prochains mois (M+1 et M+2), et la cree si manquante.
/// Idempotent : tourner ce job N fois par jour est safe.
///
/// Ne supprime pas les anciennes partitions (rétention/archivage = futur).
pub async fn run(pool: &PgPool) -> Result<(), String> {
    /// (table_name, partition_key_column)
    const PARTITIONED_TABLES: &[(&str, &str)] = &[
        ("infractions", "created_at"),
        ("audit_logs", "created_at"),
        ("user_activity_log", "created_at"),
        ("logs", "timestamp"),
    ];

    let now = chrono::Utc::now();
    let mut created = 0u32;

    for (table, _key) in PARTITIONED_TABLES {
        // On garantit M+1 et M+2 (le mois courant a deja ete cree par la migration
        // ou un run precedent).
        for offset in 1..=2 {
            let target_month = add_months(now, offset);
            let next_month = add_months(now, offset + 1);

            let suffix = format!("{}_{:02}", target_month.format("%Y"), target_month.month());
            let part_name = format!("{table}_{suffix}");
            let from_str = target_month.format("%Y-%m-01").to_string();
            let to_str = next_month.format("%Y-%m-01").to_string();

            // Verifier si la partition existe deja
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM pg_class WHERE relname = $1)",
            )
            .bind(&part_name)
            .fetch_one(pool)
            .await
            .map_err(|e| format!("check exists {part_name}: {e}"))?;

            if exists {
                continue;
            }

            let sql = format!(
                "CREATE TABLE {part_name} PARTITION OF {table} \
                 FOR VALUES FROM ('{from_str}') TO ('{to_str}')"
            );

            match sqlx::query(&sql).execute(pool).await {
                Ok(_) => {
                    info!(partition = %part_name, "Partition mensuelle creee");
                    created += 1;
                }
                Err(e) => warn!(partition = %part_name, error = %e, "Echec creation partition"),
            }
        }
    }

    if created > 0 {
        info!(created, "Partition manager : nouvelles partitions creees");
    }

    Ok(())
}

/// Ajoute `n` mois a une date UTC, en gerant correctement les overflow d'annee.
fn add_months(date: chrono::DateTime<chrono::Utc>, n: u32) -> chrono::DateTime<chrono::Utc> {
    use chrono::{Datelike, TimeZone};
    let total_months = date.year() * 12 + date.month0() as i32 + n as i32;
    let new_year = total_months / 12;
    let new_month = (total_months % 12) as u32 + 1;
    chrono::Utc
        .with_ymd_and_hms(new_year, new_month, 1, 0, 0, 0)
        .single()
        .unwrap_or(date)
}
