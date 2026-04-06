use rand::Rng;
use sqlx::PgPool;
use tracing::{info, warn, error};
use uuid::Uuid;

/// Combat en phase de paris dont le delai est depasse.
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct BettingCombat {
    pub id: Uuid,
    pub guild_id: String,
    pub channel_id: String,
    pub message_id: Option<String>,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
    pub mise: i64,
    pub special_attack: Option<String>,
    pub defender_special: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct PlayerStats {
    pub user_id: String,
    pub coins: i64,
    pub level: i32,
    pub atk: i32,
    pub def: i32,
}

/// Verifie et resout les combats "betting" dont le delai de paris est ecoule.
/// Le delai est lu depuis la config guild (bet_delay_secs, defaut 300s).
pub async fn run(pool: &PgPool, api_url: &str, bot_token: &str) -> Result<(), String> {
    // Recuperer les combats en phase de paris dont le delai est depasse
    // On utilise un delai par defaut de 300s, la config guild sera lue par combat si besoin
    // Verrouiller atomiquement : passer les combats de "betting" a "resolving"
    // pour eviter qu'un autre worker les traite en parallele
    let combats = sqlx::query_as::<_, BettingCombat>(
        r#"UPDATE coude_combats SET status = 'resolving'
        WHERE id IN (
            SELECT id FROM coude_combats
            WHERE status = 'betting' AND accepted_at < NOW() - INTERVAL '5 minutes'
            FOR UPDATE SKIP LOCKED
        )
        RETURNING id, guild_id, channel_id, message_id,
            attacker_id, attacker_name, defender_id, defender_name,
            mise, special_attack, defender_special
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Erreur requete betting combats: {e}"))?;

    if combats.is_empty() {
        return Ok(());
    }

    info!(count = combats.len(), "Combats betting a resoudre");

    for combat in &combats {
        if let Err(e) = resolve_single(pool, combat, api_url, bot_token).await {
            error!(combat_id = %combat.id, error = %e, "Erreur resolution combat betting");
        }
    }

    Ok(())
}

async fn resolve_single(
    pool: &PgPool,
    combat: &BettingCombat,
    _api_url: &str,
    bot_token: &str,
) -> Result<(), String> {
    // Charger les stats des joueurs
    let attacker = sqlx::query_as::<_, PlayerStats>(
        "SELECT user_id, coins, level, atk, def FROM coude_players WHERE guild_id = $1 AND user_id = $2"
    )
    .bind(&combat.guild_id)
    .bind(&combat.attacker_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("get attacker: {e}"))?
    .ok_or("Attaquant introuvable")?;

    let defender = sqlx::query_as::<_, PlayerStats>(
        "SELECT user_id, coins, level, atk, def FROM coude_players WHERE guild_id = $1 AND user_id = $2"
    )
    .bind(&combat.guild_id)
    .bind(&combat.defender_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("get defender: {e}"))?
    .ok_or("Defenseur introuvable")?;

    // Simuler le combat (rolls simples)
    let (atk_roll, def_roll) = {
        let mut rng = rand::thread_rng();
        (rng.gen_range(1..=20) + attacker.atk, rng.gen_range(1..=20) + defender.def)
    };

    let (winner_id, loser_id, coins_transferred, result_msg) = if atk_roll > def_roll {
        (
            combat.attacker_id.clone(),
            combat.defender_id.clone(),
            combat.mise,
            format!(
                "**{}** remporte le combat contre **{}** !\nRolls : {} vs {}\nGain : +{} coins",
                combat.attacker_name, combat.defender_name, atk_roll, def_roll, combat.mise
            ),
        )
    } else if def_roll > atk_roll {
        (
            combat.defender_id.clone(),
            combat.attacker_id.clone(),
            combat.mise,
            format!(
                "**{}** remporte le combat contre **{}** !\nRolls : {} vs {}\nGain : +{} coins",
                combat.defender_name, combat.attacker_name, def_roll, atk_roll, combat.mise
            ),
        )
    } else {
        // Egalite
        let msg = format!(
            "Egalite entre **{}** et **{}** !\nRolls : {} vs {}",
            combat.attacker_name, combat.defender_name, atk_roll, def_roll
        );
        // Resoudre en egalite
        sqlx::query(
            "UPDATE coude_combats SET status = 'accepted', winner_id = NULL, attacker_roll = $2, defender_roll = $3, result_message = $4, resolved_at = NOW() WHERE id = $1"
        )
        .bind(combat.id)
        .bind(atk_roll)
        .bind(def_roll)
        .bind(&msg)
        .execute(pool)
        .await
        .map_err(|e| format!("resolve draw: {e}"))?;

        post_result_to_discord(bot_token, &combat.channel_id, combat.message_id.as_deref(), &msg).await;
        info!(combat_id = %combat.id, "Combat resolu: egalite");
        return Ok(());
    };

    // Mettre a jour le combat
    sqlx::query(
        r#"UPDATE coude_combats SET status = 'accepted', winner_id = $2,
           attacker_roll = $3, defender_roll = $4, coins_transferred = $5,
           result_message = $6, resolved_at = NOW()
           WHERE id = $1"#
    )
    .bind(combat.id)
    .bind(&winner_id)
    .bind(atk_roll)
    .bind(def_roll)
    .bind(coins_transferred)
    .bind(&result_msg)
    .execute(pool)
    .await
    .map_err(|e| format!("resolve combat: {e}"))?;

    // Transferer les coins
    sqlx::query(
        "UPDATE coude_players SET coins = coins + $3, total_wins = total_wins + 1, total_earned = total_earned + $3, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2"
    )
    .bind(&combat.guild_id).bind(&winner_id).bind(coins_transferred)
    .execute(pool).await.map_err(|e| format!("winner coins: {e}"))?;

    sqlx::query(
        "UPDATE coude_players SET coins = GREATEST(0, coins - $3), total_losses = total_losses + 1, total_lost = total_lost + $3, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2"
    )
    .bind(&combat.guild_id).bind(&loser_id).bind(coins_transferred)
    .execute(pool).await.map_err(|e| format!("loser coins: {e}"))?;

    // Resoudre les paris
    resolve_bets(pool, combat.id, &winner_id).await;

    // XP
    let _ = sqlx::query(
        "UPDATE coude_players SET xp = xp + 15, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2"
    ).bind(&combat.guild_id).bind(&winner_id).execute(pool).await;

    let _ = sqlx::query(
        "UPDATE coude_players SET xp = xp + 5, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2"
    ).bind(&combat.guild_id).bind(&loser_id).execute(pool).await;

    // Lire le salon combats configure (fallback: canal du combat)
    let configured_channel = sqlx::query_scalar::<_, String>(
        "SELECT config_value FROM bot_guild_configs WHERE guild_id = $1 AND bot_name = 'coude' AND config_key = 'channel_combats'"
    )
    .bind(&combat.guild_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .filter(|v| !v.is_empty());

    let target_channel = configured_channel.as_deref().unwrap_or(&combat.channel_id);

    // Poster le resultat sur Discord
    let embed_msg = format!(
        "**Resultat du Coup de Coude !**\n\n{}\n\n<@{}> vs <@{}>\nMise : {} coins",
        result_msg, combat.attacker_id, combat.defender_id, combat.mise
    );
    post_result_to_discord(bot_token, target_channel, combat.message_id.as_deref(), &embed_msg).await;

    // Notification simplifiee dans channel_notifications
    let notif_channel = sqlx::query_scalar::<_, String>(
        "SELECT config_value FROM bot_guild_configs WHERE guild_id = $1 AND bot_name = 'coude' AND config_key = 'channel_notifications'"
    )
    .bind(&combat.guild_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .filter(|v| !v.is_empty());

    if let Some(notif_ch) = notif_channel {
        let notif_msg = format!(
            "Le combat entre **{}** et **{}** est termine !\nResultat dans <#{}>",
            combat.attacker_name, combat.defender_name, target_channel
        );
        post_result_to_discord(bot_token, &notif_ch, None, &notif_msg).await;
    }

    info!(combat_id = %combat.id, winner = %winner_id, "Combat betting resolu par le worker");

    Ok(())
}

async fn resolve_bets(pool: &PgPool, combat_id: Uuid, winner_id: &str) {
    #[derive(sqlx::FromRow)]
    struct Bet { id: Uuid, guild_id: String, bettor_id: String, backed_id: String, amount: i64 }

    let bets = sqlx::query_as::<_, Bet>(
        "SELECT id, guild_id, bettor_id, backed_id, amount FROM coude_bets WHERE combat_id = $1"
    )
    .bind(combat_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    for bet in &bets {
        if bet.backed_id == winner_id {
            let payout = bet.amount * 2;
            let _ = sqlx::query("UPDATE coude_players SET coins = coins + $3, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2")
                .bind(&bet.guild_id).bind(&bet.bettor_id).bind(payout)
                .execute(pool).await;
            let _ = sqlx::query("UPDATE coude_bets SET won = true, payout = $2 WHERE id = $1")
                .bind(bet.id).bind(payout).execute(pool).await;
        } else {
            let _ = sqlx::query("UPDATE coude_bets SET won = false, payout = 0 WHERE id = $1")
                .bind(bet.id).execute(pool).await;
        }
    }
}

async fn post_result_to_discord(bot_token: &str, channel_id: &str, message_id: Option<&str>, content: &str) {
    if bot_token.is_empty() {
        warn!("Pas de token Discord pour poster le resultat");
        return;
    }

    let client = reqwest::Client::new();

    // Editer le message existant si on a le message_id
    if let Some(mid) = message_id {
        let url = format!("https://discord.com/api/v10/channels/{}/messages/{}", channel_id, mid);
        let resp = client.patch(&url)
            .header("Authorization", format!("Bot {}", bot_token))
            .json(&serde_json::json!({
                "embeds": [{
                    "title": "Resultat Resultat du Coup de Coude !",
                    "description": content,
                    "color": 0x57F287
                }],
                "components": []
            }))
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => return,
            Ok(r) => warn!("Discord edit message failed: {}", r.status()),
            Err(e) => warn!("Discord edit request failed: {e}"),
        }
    }

    // Fallback : poster un nouveau message
    let url = format!("https://discord.com/api/v10/channels/{}/messages", channel_id);
    let _ = client.post(&url)
        .header("Authorization", format!("Bot {}", bot_token))
        .json(&serde_json::json!({
            "embeds": [{
                "title": "Resultat Resultat du Coup de Coude !",
                "description": content,
                "color": 0x57F287
            }]
        }))
        .send()
        .await;
}
