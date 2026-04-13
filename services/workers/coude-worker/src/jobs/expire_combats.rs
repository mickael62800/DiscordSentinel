use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

/// Modele leger pour les combats expires.
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct ExpiredCombat {
    pub id: Uuid,
    pub guild_id: String,
    pub channel_id: String,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
    pub mise: i64,
}

/// Expire les combats en attente dont la duree configuree est depassee.
/// - Le defenseur perd 20% de la mise
/// - Le compteur de lachete du defenseur est incremente
/// - La mise de l'attaquant est remboursee (pas de penalite)
/// - Les paris sur ce combat sont rembourses
pub async fn run(pool: &PgPool) -> Result<(), String> {
    // Lire la duree d'expiration par guild (defaut 24h = 86400s)
    // On utilise le min de toutes les guilds pour la requete globale,
    // puis on verifie individuellement par guild
    let combats = sqlx::query_as::<_, ExpiredCombat>(
        r#"
        SELECT c.id, c.guild_id, c.channel_id, c.attacker_id, c.attacker_name, c.defender_id, c.defender_name, c.mise
        FROM coude_combats c
        LEFT JOIN bot_guild_config cfg ON cfg.guild_id = c.guild_id AND cfg.bot_name = 'coude' AND cfg.config_key = 'combat_expire_secs'
        WHERE c.status = 'pending'
          AND c.created_at < NOW() - MAKE_INTERVAL(secs := COALESCE(
                CASE WHEN cfg.config_value ~ '^\d+$' THEN cfg.config_value::int ELSE NULL END,
                86400
              ))
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Erreur requete combats expires: {e}"))?;

    if combats.is_empty() {
        return Ok(());
    }

    info!(count = combats.len(), "Combats expires trouves");

    let mut errors = Vec::new();

    for combat in &combats {
        if let Err(e) = expire_single_combat(pool, combat).await {
            warn!(
                combat_id = %combat.id,
                error = %e,
                "Erreur expiration combat"
            );
            errors.push(format!("combat {}: {}", combat.id, e));
        }
    }

    info!(
        total = combats.len(),
        errors = errors.len(),
        "Expiration combats terminee"
    );

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Erreurs partielles: {}", errors.join("; ")))
    }
}

async fn expire_single_combat(pool: &PgPool, combat: &ExpiredCombat) -> Result<(), String> {
    // 1. Marquer le combat comme expire
    sqlx::query("UPDATE coude_combats SET status = 'expired', resolved_at = NOW() WHERE id = $1")
        .bind(combat.id)
        .execute(pool)
        .await
        .map_err(|e| format!("expire combat: {e}"))?;

    // 2. Penalite pour le defenseur : 20% de la mise
    let penalty = (combat.mise as f64 * 0.20).max(1.0) as i64;

    // Phase 8 : coins sur user_wallets (wallet partage), stats sur coude_players.
    sqlx::query(
        "UPDATE user_wallets SET coins = GREATEST(0, coins - $3), total_spent = total_spent + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&combat.guild_id)
    .bind(&combat.defender_id)
    .bind(penalty)
    .execute(pool)
    .await
    .map_err(|e| format!("penalite defenseur wallet: {e}"))?;

    sqlx::query(
        "UPDATE coude_players SET total_lost = total_lost + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&combat.guild_id)
    .bind(&combat.defender_id)
    .bind(penalty)
    .execute(pool)
    .await
    .map_err(|e| format!("penalite defenseur stats: {e}"))?;

    // 3. Incrementer la lachete du defenseur
    sqlx::query(
        "UPDATE coude_players SET cowardice_count = cowardice_count + 1, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&combat.guild_id)
    .bind(&combat.defender_id)
    .execute(pool)
    .await
    .map_err(|e| format!("increment lachete: {e}"))?;

    // 4. Rembourser les paris sur ce combat
    refund_combat_bets(pool, combat.id).await?;

    info!(
        combat_id = %combat.id,
        defender = %combat.defender_name,
        penalty = penalty,
        "Combat expire: {} n'a pas repondu en 24h -> -{} coins + lachete",
        combat.defender_name,
        penalty
    );

    Ok(())
}

async fn refund_combat_bets(pool: &PgPool, combat_id: Uuid) -> Result<(), String> {
    // Recuperer les paris pour ce combat
    let bets: Vec<(Uuid, String, String, i64)> = sqlx::query_as(
        "SELECT id, guild_id, bettor_id, amount FROM coude_bets WHERE combat_id = $1",
    )
    .bind(combat_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("get bets: {e}"))?;

    if bets.is_empty() {
        return Ok(());
    }

    // Rembourser chaque parieur sur le wallet partage (Phase 8).
    for (bet_id, guild_id, bettor_id, amount) in &bets {
        sqlx::query(
            "UPDATE user_wallets SET coins = coins + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(bettor_id)
        .bind(amount)
        .execute(pool)
        .await
        .map_err(|e| format!("refund bet {}: {}", bet_id, e))?;
    }

    // Marquer les paris comme rembourses
    sqlx::query("UPDATE coude_bets SET won = false, payout = amount WHERE combat_id = $1")
        .bind(combat_id)
        .execute(pool)
        .await
        .map_err(|e| format!("update bets: {e}"))?;

    info!(
        combat_id = %combat_id,
        refunded = bets.len(),
        "Paris rembourses pour combat expire"
    );

    Ok(())
}
