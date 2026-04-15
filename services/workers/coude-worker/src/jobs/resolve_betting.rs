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

    // ── Draw path (inclut l'explosion) ──
    if result.winner_id.is_none() {
        // Explosion : les deux joueurs perdent coins_lost_by_loser chacun.
        // Pour l'egalite normale, coins_lost_by_loser est 0. Le draw path
        // couvrait les 2 cas mais sans debiter les coins pour l'explosion.
        let explosion_loss = result.coins_lost_by_loser;

        sqlx::query(
            r#"UPDATE coude_combats
               SET status = 'accepted', winner_id = NULL,
                   attacker_roll = $2, defender_roll = $3,
                   coins_transferred = $4, chaos_event = $5,
                   result_message = $6, resolved_at = NOW()
               WHERE id = $1"#,
        )
        .bind(combat.id)
        .bind(first_atk_roll)
        .bind(first_def_roll)
        .bind(explosion_loss)
        .bind(chaos_event_key.as_deref())
        .bind(&result.message)
        .execute(pool)
        .await
        .map_err(|e| format!("resolve draw: {e}"))?;

        // Update HP des 2 joueurs (important : HP ont pu baisser pendant les rounds)
        update_player_hp(pool, &combat.guild_id, &combat.attacker_id, result.attacker_hp_final.max(0), result.attacker_hp_max).await;
        update_player_hp(pool, &combat.guild_id, &combat.defender_id, result.defender_hp_final.max(0), result.defender_hp_max).await;

        // Debiter les 2 joueurs pour l'explosion (50% mise chacun).
        if explosion_loss > 0 {
            let desc = format!("Explosion combat {}", combat.id);
            debit_and_log(pool, &combat.guild_id, &combat.attacker_id, explosion_loss, "coude_combat_explosion", &desc).await;
            debit_and_log(pool, &combat.guild_id, &combat.defender_id, explosion_loss, "coude_combat_explosion", &desc).await;

            if let Err(e) = sqlx::query(
                "UPDATE coude_players SET total_lost = total_lost + $3, updated_at = NOW()
                 WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(&combat.guild_id).bind(&combat.attacker_id).bind(explosion_loss)
            .execute(pool).await {
                warn!(error = %e, "Echec update total_lost attacker explosion");
            }
            if let Err(e) = sqlx::query(
                "UPDATE coude_players SET total_lost = total_lost + $3, updated_at = NOW()
                 WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(&combat.guild_id).bind(&combat.defender_id).bind(explosion_loss)
            .execute(pool).await {
                warn!(error = %e, "Echec update total_lost defender explosion");
            }
        }

        // Rembourser les paris : egalite/explosion = personne ne gagne, rembourse tout.
        refund_all_bets(pool, combat.id).await;

        post_result_to_discord(
            bot_token,
            &combat.channel_id,
            combat.message_id.as_deref(),
            &result.message,
        )
        .await;

        info!(combat_id = %combat.id, rounds = result.total_rounds, explosion_loss, "Combat resolu: egalite/explosion");
        return Ok(());
    }

    // ── Winner path ──
    let winner_id = result.winner_id.clone().unwrap();
    let loser_id = result.loser_id.clone().unwrap();

    // Cap le transfert sur le solde reel du perdant pour ne pas creer de
    // coins ex-nihilo (le debit utilise GREATEST(0, coins-X), donc sans cap
    // ici le gagnant recevrait plus que ce que le perdant possede).
    let loser_balance: i64 = sqlx::query_scalar(
        "SELECT COALESCE(coins, 0) FROM user_wallets WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&combat.guild_id)
    .bind(&loser_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(0);

    let coins_transferred = result.coins_won.min(loser_balance);
    let coins_lost = result.coins_lost_by_loser.min(loser_balance);

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
    if let Err(e) = sqlx::query(
        "UPDATE coude_players SET total_wins = total_wins + 1,
         total_earned = total_earned + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&combat.guild_id).bind(&winner_id).bind(coins_transferred)
    .execute(pool).await {
        warn!(error = %e, winner_id, "Echec update total_wins/total_earned");
    }

    if let Err(e) = sqlx::query(
        "UPDATE coude_players SET total_losses = total_losses + 1,
         total_lost = total_lost + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&combat.guild_id).bind(&loser_id).bind(coins_lost)
    .execute(pool).await {
        warn!(error = %e, loser_id, "Echec update total_losses/total_lost");
    }

    // ── Transferer les coins dans user_wallets (wallet partage) ──
    let combat_desc = format!("Combat {} vs {}", winner_id, loser_id);
    credit_and_log(pool, &combat.guild_id, &winner_id, coins_transferred, true, "coude_combat_win", &combat_desc).await;
    debit_and_log(pool, &combat.guild_id, &loser_id, coins_lost, "coude_combat_loss", &combat_desc).await;

    // ── Update HP apres le combat ──
    update_player_hp(pool, &combat.guild_id, &combat.attacker_id, result.attacker_hp_final.max(0), result.attacker_hp_max).await;
    update_player_hp(pool, &combat.guild_id, &combat.defender_id, result.defender_hp_final.max(0), result.defender_hp_max).await;

    // ── Vol bonus chaos "Vol" ──
    // Cap sur le solde restant du perdant : apres le debit principal, il peut
    // rester 0 coins. On ne cree pas de coins ex-nihilo.
    if result.vol_coins > 0 {
        let loser_balance_after: i64 = sqlx::query_scalar(
            "SELECT COALESCE(coins, 0) FROM user_wallets WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(&combat.guild_id)
        .bind(&loser_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(0);

        let vol_coins_capped = result.vol_coins.min(loser_balance_after);
        if vol_coins_capped > 0 {
            // Debit du perdant AVANT credit du gagnant pour eviter la duplication.
            debit_and_log(pool, &combat.guild_id, &loser_id, vol_coins_capped, "coude_combat_vol_victim", "Victime vol chaos").await;
            credit_and_log(pool, &combat.guild_id, &winner_id, vol_coins_capped, true, "coude_combat_vol_bonus", "Bonus chaos vol").await;

            if let Err(e) = sqlx::query(
                "UPDATE coude_players SET total_stolen = total_stolen + $3,
                 updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(&combat.guild_id).bind(&winner_id).bind(vol_coins_capped)
            .execute(pool).await {
                warn!(error = %e, winner_id, "Echec update total_stolen");
            }
        }
    }

    // ── Chaos events count ──
    if result.chaos_events_count > 0 {
        if let Err(e) = sqlx::query(
            "UPDATE coude_players SET chaos_events = chaos_events + 1, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(&combat.guild_id)
        .bind(&combat.attacker_id)
        .execute(pool)
        .await {
            warn!(error = %e, "Echec update chaos_events attacker");
        }
        if let Err(e) = sqlx::query(
            "UPDATE coude_players SET chaos_events = chaos_events + 1, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(&combat.guild_id)
        .bind(&combat.defender_id)
        .execute(pool)
        .await {
            warn!(error = %e, "Echec update chaos_events defender");
        }
    }

    // ── XP ──
    let xp_winner: i64 = if result.is_giant_killer { 30 } else { 15 };
    if let Err(e) = sqlx::query(
        "UPDATE coude_players SET xp = xp + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&combat.guild_id)
    .bind(&winner_id)
    .bind(xp_winner)
    .execute(pool)
    .await {
        warn!(error = %e, winner_id, "Echec update xp winner");
    }
    if let Err(e) = sqlx::query(
        "UPDATE coude_players SET xp = xp + 5, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&combat.guild_id)
    .bind(&loser_id)
    .execute(pool)
    .await {
        warn!(error = %e, loser_id, "Echec update xp loser");
    }

    // ── Resoudre les paris (parieurs gagnants/perdants) ──
    let bet_results = resolve_bets(pool, combat.id, &winner_id).await;

    // Bonus combat : gagnant recoit 10% des paris perdants
    if bet_results.total_lost_by_bettors > 0 {
        let combat_bonus = bet_results.total_lost_by_bettors / 10;
        if combat_bonus > 0 {
            // Stats coude_players
            if let Err(e) = sqlx::query(
                "UPDATE coude_players SET total_earned = total_earned + $3,
                 updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(&combat.guild_id).bind(&winner_id).bind(combat_bonus)
            .execute(pool).await {
                warn!(error = %e, winner_id, "Echec update total_earned bet_bonus");
            }
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
        guild_id = %combat.guild_id,
        winner_id = %winner_id,
        loser_id = %loser_id,
        attacker_id = %combat.attacker_id,
        defender_id = %combat.defender_id,
        mise = combat.mise,
        coins_transferred = coins_transferred,
        coins_lost = coins_lost,
        vol_coins = result.vol_coins,
        stolen_bonus = result.stolen_bonus,
        rounds = result.total_rounds,
        chaos_events = result.chaos_events_count,
        is_giant_killer = result.is_giant_killer,
        attacker_hp_final = result.attacker_hp_final,
        defender_hp_final = result.defender_hp_final,
        "Combat betting resolu"
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
            let payout = bet.amount.saturating_mul(2);
            let desc = format!("Pari gagne combat {}", combat_id);
            credit_and_log(pool, &bet.guild_id, &bet.bettor_id, payout, false, "coude_bet_win_worker", &desc).await;
            if let Err(e) = sqlx::query("UPDATE coude_bets SET won = true, payout = $2 WHERE id = $1")
                .bind(bet.id)
                .bind(payout)
                .execute(pool)
                .await
            {
                warn!(error = %e, bet_id = %bet.id, "Echec mark bet won");
            }
        } else {
            total_lost_by_bettors = total_lost_by_bettors.saturating_add(bet.amount);
            if let Err(e) = sqlx::query("UPDATE coude_bets SET won = false, payout = 0 WHERE id = $1")
                .bind(bet.id)
                .execute(pool)
                .await
            {
                warn!(error = %e, bet_id = %bet.id, "Echec mark bet lost");
            }
        }
    }

    BetResult { total_lost_by_bettors }
}

/// Rembourse TOUS les paris d'un combat (egalite, explosion, refused, expired).
/// Chaque parieur recupere son amount dans user_wallets et le bet est marque
/// won = false, payout = 0 pour eviter un refund double.
async fn refund_all_bets(pool: &PgPool, combat_id: Uuid) {
    #[derive(sqlx::FromRow)]
    struct RefundBet {
        id: Uuid,
        guild_id: String,
        bettor_id: String,
        amount: i64,
    }

    let bets = match sqlx::query_as::<_, RefundBet>(
        "SELECT id, guild_id, bettor_id, amount FROM coude_bets
         WHERE combat_id = $1 AND payout IS NULL",
    )
    .bind(combat_id)
    .fetch_all(pool)
    .await
    {
        Ok(b) => b,
        Err(e) => {
            warn!(error = %e, combat_id = %combat_id, "Echec chargement paris pour refund");
            return;
        }
    };

    if bets.is_empty() {
        return;
    }

    let desc = format!("Refund pari combat {} (egalite/explosion)", combat_id);
    for bet in &bets {
        credit_and_log(pool, &bet.guild_id, &bet.bettor_id, bet.amount, false, "coude_bet_refund", &desc).await;
        if let Err(e) = sqlx::query(
            "UPDATE coude_bets SET won = false, payout = $2 WHERE id = $1",
        )
        .bind(bet.id)
        .bind(bet.amount)
        .execute(pool)
        .await {
            warn!(error = %e, bet_id = %bet.id, "Echec mark bet refunded");
        }
    }
    info!(combat_id = %combat_id, refunded = bets.len(), "Paris rembourses");
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
