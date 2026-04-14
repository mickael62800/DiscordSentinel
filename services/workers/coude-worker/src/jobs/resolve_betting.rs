use sqlx::PgPool;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::combat_engine::{self, PlayerLite, ServerEventLite};
use crate::jobs::wallet_log::{credit_and_log, debit_and_log};

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

/// Rangee coude_players necessaire au moteur de combat.
#[derive(Debug, sqlx::FromRow)]
struct PlayerRow {
    pub user_id: String,
    pub class: Option<String>,
    pub level: i32,
    pub atk: i32,
    pub def: i32,
    pub cowardice_count: i32,
    pub hp_current: Option<i32>,
}

impl From<PlayerRow> for PlayerLite {
    fn from(row: PlayerRow) -> Self {
        Self {
            user_id: row.user_id,
            class: row.class,
            level: row.level,
            atk: row.atk,
            def: row.def,
            cowardice_count: row.cowardice_count,
            hp_current: row.hp_current,
        }
    }
}

/// Verifie et resout les combats "betting" dont le delai de paris est ecoule.
/// Le combat passe par le moteur multi-rounds complet (HP, classes, chaos,
/// items) au lieu du vieux random simple.
pub async fn run(pool: &PgPool, _api_url: &str, bot_token: &str) -> Result<(), String> {
    // Verrouiller atomiquement : passer les combats de "betting" a "resolving"
    // pour eviter qu'un autre worker les traite en parallele.
    //
    // Le delai de paris est configurable par guild via la cle `bet_delay_secs`
    // dans bot_guild_config (defaut 300 = 5 min). Cette cle vit sous
    // `bot_name = 'coude-worker'` (migration 120). Avant, elle etait
    // referencee sous `bot_name = 'coude'` qui n'existait nulle part, donc
    // la valeur editee depuis l'UI desktop n'etait JAMAIS prise en compte
    // et le worker tournait toujours avec le defaut en dur.
    //
    // 1. Recuperer les combats en phase betting dont le delai est ecoule.
    let mut combats = sqlx::query_as::<_, BettingCombat>(
        r#"UPDATE coude_combats SET status = 'resolving'
        WHERE id IN (
            SELECT c.id FROM coude_combats c
            LEFT JOIN bot_guild_config cfg
                ON cfg.guild_id = c.guild_id
                AND cfg.bot_name = 'coude-worker'
                AND cfg.config_key = 'bet_delay_secs'
            WHERE c.status = 'betting'
              AND c.accepted_at < NOW() - (COALESCE(
                    CASE WHEN cfg.config_value ~ '^\d+$' THEN cfg.config_value::int ELSE NULL END,
                    300
                  ) * INTERVAL '1 second')
            FOR UPDATE OF c SKIP LOCKED
        )
        RETURNING id, guild_id, channel_id, message_id,
            attacker_id, attacker_name, defender_id, defender_name,
            mise, special_attack, defender_special
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Erreur requete betting combats: {e}"))?;

    // 2. Recuperer les combats bloques en 'resolving' depuis > 2 min (crash
    //    d'un tick precedent). On les ATOMIQUEMENT touche (accepted_at =
    //    NOW()) pour eviter que deux workers concurrents re-traitent le
    //    meme combat stuck et doublent XP/coins/chaos. Avant, un simple
    //    SELECT sans lock permettait le double-traitement. Avec NOW(), le
    //    filtre `< NOW() - 2 min` ne matche plus, donc un tick ulterieur
    //    ne le reprendra pas tant que ce worker n'a pas termine.
    let stuck = sqlx::query_as::<_, BettingCombat>(
        r#"UPDATE coude_combats SET accepted_at = NOW()
        WHERE id IN (
            SELECT id FROM coude_combats
            WHERE status = 'resolving'
              AND accepted_at < NOW() - INTERVAL '2 minutes'
            FOR UPDATE SKIP LOCKED
        )
        RETURNING id, guild_id, channel_id, message_id,
            attacker_id, attacker_name, defender_id, defender_name,
            mise, special_attack, defender_special
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Erreur requete stuck combats: {e}"))?;

    if !stuck.is_empty() {
        info!(count = stuck.len(), "Combats bloques en 'resolving' recuperes (retry)");
        combats.extend(stuck);
    }

    if combats.is_empty() {
        // Log pour debug : combien de combats en betting existent (sans filtre delai)
        let total_betting: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM coude_combats WHERE status IN ('betting', 'resolving')",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);
        if total_betting > 0 {
            info!(
                betting_count = total_betting,
                "Aucun combat a resoudre ce tick (combats en attente de fin du delai de paris)"
            );
        }
        return Ok(());
    }

    info!(count = combats.len(), "Combats betting a resoudre (multi-round)");

    for combat in &combats {
        if let Err(e) = resolve_single(pool, combat, bot_token).await {
            error!(combat_id = %combat.id, error = %e, "Erreur resolution combat betting");
        }
    }

    Ok(())
}

async fn resolve_single(
    pool: &PgPool,
    combat: &BettingCombat,
    bot_token: &str,
) -> Result<(), String> {
    // ── Charger les joueurs complets ──
    let attacker_row = sqlx::query_as::<_, PlayerRow>(
        "SELECT user_id, class::text, level, atk, def, cowardice_count, hp_current
         FROM coude_players WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&combat.guild_id)
    .bind(&combat.attacker_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("get attacker: {e}"))?
    .ok_or("Attaquant introuvable")?;

    let defender_row = sqlx::query_as::<_, PlayerRow>(
        "SELECT user_id, class::text, level, atk, def, cowardice_count, hp_current
         FROM coude_players WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&combat.guild_id)
    .bind(&combat.defender_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("get defender: {e}"))?
    .ok_or("Defenseur introuvable")?;

    let attacker: PlayerLite = attacker_row.into();
    let defender: PlayerLite = defender_row.into();

    // ── Charger les events actifs (happy_hour, bloodbath...) ──
    let events = match sqlx::query_as::<_, (String,)>(
        "SELECT event_type FROM coude_events
         WHERE guild_id = $1 AND active = true AND expires_at > NOW()",
    )
    .bind(&combat.guild_id)
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows
            .into_iter()
            .map(|(event_type,)| ServerEventLite { event_type })
            .collect(),
        Err(e) => {
            warn!(error = %e, guild_id = %combat.guild_id, "Echec fetch events actifs");
            Vec::new()
        }
    };

    // HP courants (defaut = HP max calcule par le moteur)
    let atk_hp_max = combat_engine::combat::calculate_hp_max(&attacker);
    let def_hp_max = combat_engine::combat::calculate_hp_max(&defender);
    let atk_hp_current = attacker.hp_current.unwrap_or(atk_hp_max).min(atk_hp_max);
    let def_hp_current = defender.hp_current.unwrap_or(def_hp_max).min(def_hp_max);

    // ── Appel du moteur de combat multi-rounds ──
    let result = combat_engine::resolve_combat(
        &attacker,
        &defender,
        atk_hp_current,
        def_hp_current,
        combat.mise,
        combat.special_attack.as_deref(),
        combat.defender_special.as_deref(),
        &events,
    );

    // Extract du premier round pour compat DB (ancien schema avec 1 roll)
    let first_atk_roll = result.rounds.first().map(|r| r.attacker_roll).unwrap_or(0);
    let first_def_roll = result.rounds.first().map(|r| r.defender_roll).unwrap_or(0);
    let chaos_event_key = result
        .rounds
        .iter()
        .find_map(|r| r.chaos_event)
        .map(|ce| ce.key().to_string());

    // ── Draw path ──
    if result.winner_id.is_none() {
        sqlx::query(
            r#"UPDATE coude_combats
               SET status = 'accepted', winner_id = NULL,
                   attacker_roll = $2, defender_roll = $3,
                   chaos_event = $4, result_message = $5, resolved_at = NOW()
               WHERE id = $1"#,
        )
        .bind(combat.id)
        .bind(first_atk_roll)
        .bind(first_def_roll)
        .bind(chaos_event_key.as_deref())
        .bind(&result.message)
        .execute(pool)
        .await
        .map_err(|e| format!("resolve draw: {e}"))?;

        // Update HP des 2 joueurs (important : HP ont pu baisser pendant les rounds)
        update_player_hp(pool, &combat.guild_id, &combat.attacker_id, result.attacker_hp_final.max(0), result.attacker_hp_max).await;
        update_player_hp(pool, &combat.guild_id, &combat.defender_id, result.defender_hp_final.max(0), result.defender_hp_max).await;

        post_result_to_discord(
            bot_token,
            &combat.channel_id,
            combat.message_id.as_deref(),
            &result.message,
        )
        .await;

        info!(combat_id = %combat.id, rounds = result.total_rounds, "Combat resolu: egalite");
        return Ok(());
    }

    // ── Winner path ──
    let winner_id = result.winner_id.clone().unwrap();
    let loser_id = result.loser_id.clone().unwrap();
    let coins_transferred = result.coins_won;
    let coins_lost = result.coins_lost_by_loser;

    sqlx::query(
        r#"UPDATE coude_combats
           SET status = 'accepted', winner_id = $2,
               attacker_roll = $3, defender_roll = $4,
               coins_transferred = $5, chaos_event = $6,
               result_message = $7, resolved_at = NOW()
           WHERE id = $1"#,
    )
    .bind(combat.id)
    .bind(&winner_id)
    .bind(first_atk_roll)
    .bind(first_def_roll)
    .bind(coins_transferred)
    .bind(chaos_event_key.as_deref())
    .bind(&result.message)
    .execute(pool)
    .await
    .map_err(|e| format!("resolve combat: {e}"))?;

    // ── Stats dans coude_players (sans toucher coins) ──
    let _ = sqlx::query(
        "UPDATE coude_players SET total_wins = total_wins + 1,
         total_earned = total_earned + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&combat.guild_id).bind(&winner_id).bind(coins_transferred)
    .execute(pool).await;

    let _ = sqlx::query(
        "UPDATE coude_players SET total_losses = total_losses + 1,
         total_lost = total_lost + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&combat.guild_id).bind(&loser_id).bind(coins_lost)
    .execute(pool).await;

    // ── Transferer les coins dans user_wallets (wallet partage) ──
    let combat_desc = format!("Combat {} vs {}", winner_id, loser_id);
    credit_and_log(pool, &combat.guild_id, &winner_id, coins_transferred, true, "coude_combat_win", &combat_desc).await;
    debit_and_log(pool, &combat.guild_id, &loser_id, coins_lost, "coude_combat_loss", &combat_desc).await;

    // ── Update HP apres le combat ──
    update_player_hp(pool, &combat.guild_id, &combat.attacker_id, result.attacker_hp_final.max(0), result.attacker_hp_max).await;
    update_player_hp(pool, &combat.guild_id, &combat.defender_id, result.defender_hp_final.max(0), result.defender_hp_max).await;

    // ── Vol bonus chaos "Vol" ──
    if result.vol_coins > 0 {
        // Stats coude_players
        let _ = sqlx::query(
            "UPDATE coude_players SET total_stolen = total_stolen + $3,
             updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(&combat.guild_id).bind(&winner_id).bind(result.vol_coins)
        .execute(pool).await;
        // Coins wallet
        credit_and_log(pool, &combat.guild_id, &winner_id, result.vol_coins, true, "coude_combat_vol_bonus", "Bonus chaos vol").await;
    }

    // ── Chaos events count ──
    if result.chaos_events_count > 0 {
        let _ = sqlx::query(
            "UPDATE coude_players SET chaos_events = chaos_events + 1, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(&combat.guild_id)
        .bind(&combat.attacker_id)
        .execute(pool)
        .await;
        let _ = sqlx::query(
            "UPDATE coude_players SET chaos_events = chaos_events + 1, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(&combat.guild_id)
        .bind(&combat.defender_id)
        .execute(pool)
        .await;
    }

    // ── XP ──
    let xp_winner: i64 = if result.is_giant_killer { 30 } else { 15 };
    let _ = sqlx::query(
        "UPDATE coude_players SET xp = xp + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&combat.guild_id)
    .bind(&winner_id)
    .bind(xp_winner)
    .execute(pool)
    .await;
    let _ = sqlx::query(
        "UPDATE coude_players SET xp = xp + 5, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&combat.guild_id)
    .bind(&loser_id)
    .execute(pool)
    .await;

    // ── Resoudre les paris (parieurs gagnants/perdants) ──
    let bet_results = resolve_bets(pool, combat.id, &winner_id).await;

    // Bonus combat : gagnant recoit 10% des paris perdants
    if bet_results.total_lost_by_bettors > 0 {
        let combat_bonus = bet_results.total_lost_by_bettors / 10;
        if combat_bonus > 0 {
            // Stats coude_players
            let _ = sqlx::query(
                "UPDATE coude_players SET total_earned = total_earned + $3,
                 updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(&combat.guild_id).bind(&winner_id).bind(combat_bonus)
            .execute(pool).await;
            // Coins wallet
            credit_and_log(pool, &combat.guild_id, &winner_id, combat_bonus, true, "coude_combat_bet_bonus", "Bonus 10% paris perdants").await;
        }
    }

    // ── Poster le resultat sur Discord ──
    let configured_channel = sqlx::query_scalar::<_, String>(
        "SELECT config_value FROM bot_guild_config
         WHERE guild_id = $1 AND bot_name = 'coude' AND config_key = 'channel_combats'",
    )
    .bind(&combat.guild_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .filter(|v| !v.is_empty());

    let target_channel = configured_channel.as_deref().unwrap_or(&combat.channel_id);

    post_result_to_discord(
        bot_token,
        target_channel,
        combat.message_id.as_deref(),
        &result.message,
    )
    .await;

    info!(
        combat_id = %combat.id,
        winner = %winner_id,
        rounds = result.total_rounds,
        chaos = result.chaos_events_count,
        "Combat betting resolu par le moteur multi-rounds"
    );

    Ok(())
}

async fn update_player_hp(pool: &PgPool, guild_id: &str, user_id: &str, hp: i32, hp_max: i32) {
    if let Err(e) = sqlx::query(
        "UPDATE coude_players SET hp_current = $3, hp_max = $4, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(hp)
    .bind(hp_max)
    .execute(pool)
    .await
    {
        warn!(error = %e, user_id, hp, hp_max, "Echec update HP");
    }
}

struct BetResult {
    total_lost_by_bettors: i64,
}

async fn resolve_bets(pool: &PgPool, combat_id: Uuid, winner_id: &str) -> BetResult {
    #[derive(sqlx::FromRow)]
    struct Bet {
        id: Uuid,
        guild_id: String,
        bettor_id: String,
        backed_id: String,
        amount: i64,
    }

    let bets = match sqlx::query_as::<_, Bet>(
        "SELECT id, guild_id, bettor_id, backed_id, amount FROM coude_bets WHERE combat_id = $1",
    )
    .bind(combat_id)
    .fetch_all(pool)
    .await
    {
        Ok(b) => b,
        Err(e) => {
            warn!(error = %e, combat_id = %combat_id, "Echec chargement paris");
            return BetResult { total_lost_by_bettors: 0 };
        }
    };

    let mut total_lost_by_bettors = 0i64;

    for bet in &bets {
        if bet.backed_id == winner_id {
            let payout = bet.amount * 2;
            let desc = format!("Pari gagne combat {}", combat_id);
            credit_and_log(pool, &bet.guild_id, &bet.bettor_id, payout, false, "coude_bet_win_worker", &desc).await;
            let _ = sqlx::query("UPDATE coude_bets SET won = true, payout = $2 WHERE id = $1")
                .bind(bet.id)
                .bind(payout)
                .execute(pool)
                .await;
        } else {
            total_lost_by_bettors += bet.amount;
            let _ = sqlx::query("UPDATE coude_bets SET won = false, payout = 0 WHERE id = $1")
                .bind(bet.id)
                .execute(pool)
                .await;
        }
    }

    BetResult { total_lost_by_bettors }
}

async fn post_result_to_discord(
    bot_token: &str,
    channel_id: &str,
    message_id: Option<&str>,
    content: &str,
) {
    if bot_token.is_empty() {
        warn!("Pas de token Discord pour poster le resultat");
        return;
    }

    let client = reqwest::Client::new();

    // Editer le message existant si on a le message_id
    if let Some(mid) = message_id {
        let url = format!("https://discord.com/api/v10/channels/{}/messages/{}", channel_id, mid);
        let resp = client
            .patch(&url)
            .header("Authorization", format!("Bot {}", bot_token))
            .json(&serde_json::json!({
                "embeds": [{
                    "title": "⚔️ Résultat du Coup de Coude !",
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
    if let Err(e) = client
        .post(&url)
        .header("Authorization", format!("Bot {}", bot_token))
        .json(&serde_json::json!({
            "embeds": [{
                "title": "⚔️ Résultat du Coup de Coude !",
                "description": content,
                "color": 0x57F287
            }]
        }))
        .send()
        .await
    {
        warn!(error = %e, channel_id, "Echec post resultat combat Discord (fallback)");
    }
}
