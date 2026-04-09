use std::sync::Arc;

use serde::Deserialize;
use sentinel_shared::api_client::BaseApiClient;

// ══════════════════════════════════════════════════════════════════════
// ── Response DTOs (match what the API returns as JSON) ──
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Player {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub coins: i64,
    pub total_wins: i32,
    pub total_losses: i32,
    pub total_draws: i32,
    pub total_earned: i64,
    pub total_lost: i64,
    pub total_stolen: i64,
    pub cowardice_count: i32,
    pub chaos_events: i32,
    pub casino_wins: i32,
    pub casino_losses: i32,
    pub level: i32,
    pub xp: i64,
    pub stat_points: i32,
    pub atk: i32,
    pub def: i32,
    pub class: Option<String>,
    pub title: Option<String>,
    #[serde(default)]
    pub class_changed_at: Option<String>,
    #[serde(default)]
    pub hp_current: Option<i32>,
    #[serde(default)]
    pub hp_max: Option<i32>,
    #[serde(default)]
    pub hp_last_regen: Option<String>,
    #[serde(default)]
    pub repos_last_used: Option<String>,
    #[serde(default)]
    pub season: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Combat {
    pub id: String,
    pub guild_id: String,
    pub channel_id: Option<String>,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
    pub mise: i64,
    pub status: String,
    pub winner_id: Option<String>,
    pub attacker_roll: Option<i32>,
    pub defender_roll: Option<i32>,
    pub chaos_event: Option<String>,
    pub special_attack: Option<String>,
    pub defender_special: Option<String>,
    pub coins_transferred: Option<i64>,
    pub result_message: Option<String>,
    pub message_id: Option<String>,
    pub created_at: String,
    pub accepted_at: Option<String>,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Prime {
    pub id: String,
    pub guild_id: String,
    pub target_id: String,
    pub target_name: String,
    pub placed_by_id: String,
    pub placed_by_name: String,
    pub amount: i64,
    pub claimed: bool,
    pub claimed_by_id: Option<String>,
    pub claimed_by_name: Option<String>,
    pub claimed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct InventoryItem {
    pub guild_id: String,
    pub user_id: String,
    pub item_key: String,
    pub quantity: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ServerEvent {
    pub id: String,
    pub guild_id: String,
    #[serde(default)]
    pub event_type: String,
    pub active: bool,
    pub expires_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct LeaderboardEntry {
    pub user_id: String,
    pub username: String,
    pub value: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Bet {
    pub id: String,
    pub combat_id: String,
    pub bettor_id: String,
    pub bettor_name: String,
    pub backed_id: String,
    pub amount: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct BetResult {
    pub bettor_id: String,
    pub bettor_name: String,
    pub backed_id: String,
    pub amount_bet: i64,
    pub payout: i64,
    pub won: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct FighterBetBonus {
    pub winner_id: String,
    pub winner_bonus: i64,
    pub loser_id: String,
    pub loser_bonus: i64,
    #[serde(default)]
    pub total_pot: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Insurance {
    pub id: String,
    pub is_scam: bool,
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct XpResult {
    pub new_xp: i64,
    pub new_level: i32,
    pub leveled_up: bool,
    pub stat_points_gained: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct CooldownResponse {
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct CowardiceResponse {
    pub cowardice_count: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct CasinoTodayResponse {
    pub count: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct FailliteResponse {
    pub total_lost: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct SuccessResponse {
    pub success: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct HasItemResponse {
    pub has_item: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct ClaimPrimesResponse {
    pub total_claimed: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct ResolveBetsResponse {
    pub results: Vec<BetResult>,
    pub fighter_bonus: Option<FighterBetBonus>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct RefundBetsResponse {
    pub refunded_count: usize,
    pub refunded_total: i64,
}

// ══════════════════════════════════════════════════════════════════════
// ── API Client ──
// ══════════════════════════════════════════════════════════════════════

pub struct ApiClient {
    pub base: Arc<BaseApiClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>) -> Self {
        Self { base }
    }

    // ── Players ──

    /// POST /api/coude/{guild_id}/players/get-or-create
    pub async fn get_or_create_player(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<Player, String> {
        self.base
            .post_json(
                &format!("/api/coude/{guild_id}/players/get-or-create"),
                &serde_json::json!({ "user_id": user_id, "username": username }),
            )
            .await
    }

    /// GET /api/coude/{guild_id}/players/{user_id}
    /// Returns None on 404.
    pub async fn get_player(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Player>, String> {
        let path = format!("/api/coude/{guild_id}/players/{user_id}");
        let resp = self
            .base
            .auth(
                self.base
                    .client()
                    .get(format!("{}{}", self.base.base_url(), path)),
            )
            .send()
            .await
            .map_err(|e| format!("{e}"))?;
        if resp.status().as_u16() == 404 {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(format!("API error {}", resp.status()));
        }
        resp.json::<Option<Player>>()
            .await
            .map_err(|e| format!("{e}"))
    }

    /// PATCH /api/coude/{guild_id}/players/{user_id}/class
    pub async fn update_player_class(
        &self,
        guild_id: &str,
        user_id: &str,
        class: &str,
    ) -> Result<(), String> {
        self.base
            .patch_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/class"),
                &serde_json::json!({ "class": class }),
            )
            .await;
        Ok(())
    }

    /// POST /api/coude/{guild_id}/players/{user_id}/xp
    /// Returns (new_xp, new_level, leveled_up, stat_points_gained).
    pub async fn add_xp(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<(i64, i32, bool, i32), String> {
        let res: XpResult = self
            .base
            .post_json(
                &format!("/api/coude/{guild_id}/players/{user_id}/xp"),
                &serde_json::json!({ "amount": amount }),
            )
            .await?;
        Ok((res.new_xp, res.new_level, res.leveled_up, res.stat_points_gained))
    }

    /// POST /api/coude/{guild_id}/players/{user_id}/spend-stat
    pub async fn spend_stat_point(
        &self,
        guild_id: &str,
        user_id: &str,
        stat: &str,
    ) -> Result<Player, String> {
        self.base
            .post_json(
                &format!("/api/coude/{guild_id}/players/{user_id}/spend-stat"),
                &serde_json::json!({ "stat": stat }),
            )
            .await
    }

    // ── Stats recording ──

    /// POST /api/coude/{guild_id}/players/{user_id}/record-win
    pub async fn record_win(
        &self,
        guild_id: &str,
        user_id: &str,
        earned: i64,
        stolen: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/record-win"),
                &serde_json::json!({ "earned": earned, "stolen": stolen }),
            )
            .await;
        Ok(())
    }

    /// POST /api/coude/{guild_id}/players/{user_id}/record-loss
    pub async fn record_loss(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/record-loss"),
                &serde_json::json!({ "lost": lost }),
            )
            .await;
        Ok(())
    }

    /// POST /api/coude/{guild_id}/players/{user_id}/record-draw
    pub async fn record_draw(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/record-draw"),
                &serde_json::json!({ "lost": lost }),
            )
            .await;
        Ok(())
    }

    /// POST /api/coude/{guild_id}/players/{user_id}/increment-cowardice
    /// Returns the updated cowardice_count.
    pub async fn increment_cowardice(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i32, String> {
        let res: CowardiceResponse = self
            .base
            .post_json(
                &format!("/api/coude/{guild_id}/players/{user_id}/increment-cowardice"),
                &serde_json::json!({}),
            )
            .await?;
        Ok(res.cowardice_count)
    }

    /// POST /api/coude/{guild_id}/players/{user_id}/increment-chaos
    pub async fn increment_chaos_events(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/increment-chaos"),
                &serde_json::json!({}),
            )
            .await;
        Ok(())
    }

    /// POST /api/coude/{guild_id}/players/{user_id}/coins-earned
    pub async fn record_coins_earned(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/coins-earned"),
                &serde_json::json!({ "amount": amount }),
            )
            .await;
        Ok(())
    }

    /// POST /api/coude/{guild_id}/players/{user_id}/coins-lost
    pub async fn record_coins_lost(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/coins-lost"),
                &serde_json::json!({ "amount": amount }),
            )
            .await;
        Ok(())
    }

    // ── Casino ──

    /// POST /api/coude/{guild_id}/players/{user_id}/casino-win
    pub async fn record_casino_win(
        &self,
        guild_id: &str,
        user_id: &str,
        gain: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/casino-win"),
                &serde_json::json!({ "gain": gain }),
            )
            .await;
        Ok(())
    }

    /// POST /api/coude/{guild_id}/players/{user_id}/casino-loss
    pub async fn record_casino_loss(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/casino-loss"),
                &serde_json::json!({ "lost": lost }),
            )
            .await;
        Ok(())
    }

    /// POST /api/coude/{guild_id}/players/{user_id}/casino-faillite
    /// Returns total_lost.
    pub async fn record_casino_faillite(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, String> {
        let res: FailliteResponse = self
            .base
            .post_json(
                &format!("/api/coude/{guild_id}/players/{user_id}/casino-faillite"),
                &serde_json::json!({}),
            )
            .await?;
        Ok(res.total_lost)
    }

    /// GET /api/coude/{guild_id}/players/{user_id}/casino-today
    pub async fn count_casino_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<u64, String> {
        let res: CasinoTodayResponse = self
            .base
            .get_json(&format!(
                "/api/coude/{guild_id}/players/{user_id}/casino-today"
            ))
            .await?;
        Ok(res.count)
    }

    /// Somme des gains casino dans les dernieres 24h via wallet_transactions.
    pub async fn sum_casino_gains_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, String> {
        #[derive(serde::Deserialize)]
        struct Resp {
            total: i64,
        }
        let res: Resp = self
            .base
            .get_json(&format!(
                "/api/coude/{guild_id}/players/{user_id}/casino-gains-today"
            ))
            .await?;
        Ok(res.total)
    }

    /// Nombre de vols effectues dans les dernieres 24h.
    pub async fn count_steal_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<u64, String> {
        #[derive(serde::Deserialize)]
        struct Resp {
            count: u64,
        }
        let res: Resp = self
            .base
            .get_json(&format!(
                "/api/coude/{guild_id}/players/{user_id}/steal-today"
            ))
            .await?;
        Ok(res.count)
    }

    // ── Combat lifecycle ──

    /// POST /api/coude/{guild_id}/combats/create
    pub async fn create_combat(
        &self,
        guild_id: &str,
        channel_id: &str,
        attacker_id: &str,
        attacker_name: &str,
        defender_id: &str,
        defender_name: &str,
        mise: i64,
        special_attack: Option<&str>,
    ) -> Result<Combat, String> {
        self.base
            .post_json(
                &format!("/api/coude/{guild_id}/combats/create"),
                &serde_json::json!({
                    "channel_id": channel_id,
                    "attacker_id": attacker_id,
                    "attacker_name": attacker_name,
                    "defender_id": defender_id,
                    "defender_name": defender_name,
                    "mise": mise,
                    "special_attack": special_attack,
                }),
            )
            .await
    }

    /// GET /api/coude/combats/{combat_id}/detail
    /// Returns None on 404.
    pub async fn get_combat(&self, id: &str) -> Result<Option<Combat>, String> {
        let path = format!("/api/coude/combats/{id}/detail");
        let resp = self
            .base
            .auth(
                self.base
                    .client()
                    .get(format!("{}{}", self.base.base_url(), path)),
            )
            .send()
            .await
            .map_err(|e| format!("{e}"))?;
        if resp.status().as_u16() == 404 {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(format!("API error {}", resp.status()));
        }
        resp.json::<Option<Combat>>()
            .await
            .map_err(|e| format!("{e}"))
    }

    /// GET /api/coude/{guild_id}/combats/pending/attacker/{user_id}
    pub async fn get_pending_combat_for_attacker(
        &self,
        guild_id: &str,
        attacker_id: &str,
    ) -> Result<Option<Combat>, String> {
        let path = format!(
            "/api/coude/{guild_id}/combats/pending/attacker/{attacker_id}"
        );
        let resp = self
            .base
            .auth(
                self.base
                    .client()
                    .get(format!("{}{}", self.base.base_url(), path)),
            )
            .send()
            .await
            .map_err(|e| format!("{e}"))?;
        if resp.status().as_u16() == 404 {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(format!("API error {}", resp.status()));
        }
        resp.json::<Option<Combat>>()
            .await
            .map_err(|e| format!("{e}"))
    }

    /// GET /api/coude/{guild_id}/combats/pending/defender/{user_id}
    pub async fn get_pending_combat_for_defender(
        &self,
        guild_id: &str,
        defender_id: &str,
    ) -> Result<Option<Combat>, String> {
        let path = format!(
            "/api/coude/{guild_id}/combats/pending/defender/{defender_id}"
        );
        let resp = self
            .base
            .auth(
                self.base
                    .client()
                    .get(format!("{}{}", self.base.base_url(), path)),
            )
            .send()
            .await
            .map_err(|e| format!("{e}"))?;
        if resp.status().as_u16() == 404 {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(format!("API error {}", resp.status()));
        }
        resp.json::<Option<Combat>>()
            .await
            .map_err(|e| format!("{e}"))
    }

    /// POST /api/coude/combats/{combat_id}/resolve
    pub async fn resolve_combat(
        &self,
        id: &str,
        status: &str,
        winner_id: Option<&str>,
        attacker_roll: Option<i32>,
        defender_roll: Option<i32>,
        chaos_event: Option<&str>,
        result_message: &str,
        coins_transferred: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/combats/{id}/resolve"),
                &serde_json::json!({
                    "status": status,
                    "winner_id": winner_id,
                    "attacker_roll": attacker_roll,
                    "defender_roll": defender_roll,
                    "chaos_event": chaos_event,
                    "result_message": result_message,
                    "coins_transferred": coins_transferred,
                }),
            )
            .await;
        Ok(())
    }

    /// POST /api/coude/combats/{combat_id}/betting
    /// Returns true if combat was in pending status and got moved to betting.
    pub async fn set_combat_betting(
        &self,
        id: &str,
        message_id: &str,
    ) -> Result<bool, String> {
        let res: SuccessResponse = self
            .base
            .post_json(
                &format!("/api/coude/combats/{id}/betting"),
                &serde_json::json!({ "message_id": message_id }),
            )
            .await?;
        Ok(res.success)
    }

    /// POST /api/coude/combats/{combat_id}/expire
    pub async fn expire_combat(&self, id: &str) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/combats/{id}/expire"),
                &serde_json::json!({}),
            )
            .await;
        Ok(())
    }

    /// POST /api/coude/combats/{combat_id}/defender-special
    pub async fn set_defender_special(
        &self,
        id: &str,
        item_key: &str,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/combats/{id}/defender-special"),
                &serde_json::json!({ "item_key": item_key }),
            )
            .await;
        Ok(())
    }

    /// GET /api/coude/combats/expired
    pub async fn get_expired_combats(&self) -> Result<Vec<Combat>, String> {
        self.base
            .get_json("/api/coude/combats/expired")
            .await
    }

    // ── Bets ──

    /// POST /api/coude/{guild_id}/bets
    pub async fn place_bet(
        &self,
        guild_id: &str,
        combat_id: &str,
        bettor_id: &str,
        bettor_name: &str,
        backed_id: &str,
        amount: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/bets"),
                &serde_json::json!({
                    "combat_id": combat_id,
                    "bettor_id": bettor_id,
                    "bettor_name": bettor_name,
                    "backed_id": backed_id,
                    "amount": amount,
                }),
            )
            .await;
        Ok(())
    }

    /// GET /api/coude/combats/{combat_id}/bets
    pub async fn get_combat_bets(
        &self,
        combat_id: &str,
    ) -> Result<Vec<Bet>, String> {
        self.base
            .get_json(&format!("/api/coude/combats/{combat_id}/bets"))
            .await
    }

    /// GET /api/coude/{guild_id}/combats/betting/{user_id}
    pub async fn get_betting_combat_for_player(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Combat>, String> {
        let path = format!("/api/coude/{guild_id}/combats/betting/{user_id}");
        let resp = self
            .base
            .auth(
                self.base
                    .client()
                    .get(format!("{}{}", self.base.base_url(), path)),
            )
            .send()
            .await
            .map_err(|e| format!("{e}"))?;
        if resp.status().as_u16() == 404 {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(format!("API error {}", resp.status()));
        }
        resp.json::<Option<Combat>>()
            .await
            .map_err(|e| format!("{e}"))
    }

    /// POST /api/coude/combats/{combat_id}/resolve-bets
    /// Returns (bet_results, optional fighter bonus).
    pub async fn resolve_bets(
        &self,
        combat_id: &str,
        winner_id: Option<&str>,
    ) -> Result<(Vec<BetResult>, Option<FighterBetBonus>), String> {
        let res: ResolveBetsResponse = self
            .base
            .post_json(
                &format!("/api/coude/combats/{combat_id}/resolve-bets"),
                &serde_json::json!({ "winner_id": winner_id }),
            )
            .await?;
        Ok((res.results, res.fighter_bonus))
    }

    /// POST /api/coude/combats/{combat_id}/refund-bets
    /// Returns the list of refunded bets.
    pub async fn refund_bets(
        &self,
        combat_id: &str,
    ) -> Result<Vec<Bet>, String> {
        // The API returns { refunded_count, refunded_total } not the bet list.
        // We fetch bets first, then refund.
        let bets = self.get_combat_bets(combat_id).await?;
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/combats/{combat_id}/refund-bets"),
                &serde_json::json!({}),
            )
            .await;
        Ok(bets)
    }

    // ── Cooldowns ──

    /// GET /api/coude/{guild_id}/cooldown/{user_id}/{action}
    /// Returns the expires_at string if a cooldown is active, None otherwise.
    pub async fn check_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
    ) -> Result<Option<String>, String> {
        let res: CooldownResponse = self
            .base
            .get_json(&format!(
                "/api/coude/{guild_id}/cooldown/{user_id}/{action}"
            ))
            .await?;
        Ok(res.expires_at)
    }

    /// POST /api/coude/{guild_id}/cooldown/{user_id}/{action}
    pub async fn set_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
        duration_secs: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/cooldown/{user_id}/{action}"),
                &serde_json::json!({ "duration_secs": duration_secs }),
            )
            .await;
        Ok(())
    }

    // ── Economy ──

    /// POST /api/coude/{guild_id}/transfer
    pub async fn transfer_coins(
        &self,
        guild_id: &str,
        from_id: &str,
        to_id: &str,
        amount: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/transfer"),
                &serde_json::json!({
                    "from_id": from_id,
                    "to_id": to_id,
                    "amount": amount,
                }),
            )
            .await;
        Ok(())
    }

    /// POST /api/coude/{guild_id}/steal
    pub async fn record_steal(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        amount: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/steal"),
                &serde_json::json!({
                    "thief_id": thief_id,
                    "victim_id": victim_id,
                    "amount": amount,
                }),
            )
            .await;
        Ok(())
    }

    // ── Primes ──

    /// POST /api/coude/{guild_id}/primes
    pub async fn create_prime(
        &self,
        guild_id: &str,
        target_id: &str,
        target_name: &str,
        placed_by_id: &str,
        placed_by_name: &str,
        amount: i64,
    ) -> Result<Prime, String> {
        self.base
            .post_json(
                &format!("/api/coude/{guild_id}/primes"),
                &serde_json::json!({
                    "target_id": target_id,
                    "target_name": target_name,
                    "placed_by_id": placed_by_id,
                    "placed_by_name": placed_by_name,
                    "amount": amount,
                }),
            )
            .await
    }

    /// GET /api/coude/{guild_id}/primes/{target_id}/active
    pub async fn get_active_primes(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Vec<Prime>, String> {
        self.base
            .get_json(&format!(
                "/api/coude/{guild_id}/primes/{target_id}/active"
            ))
            .await
    }

    /// POST /api/coude/{guild_id}/primes/claim
    /// Returns total_claimed amount.
    pub async fn claim_primes(
        &self,
        guild_id: &str,
        target_id: &str,
        claimer_id: &str,
        claimer_name: &str,
    ) -> Result<i64, String> {
        let res: ClaimPrimesResponse = self
            .base
            .post_json(
                &format!("/api/coude/{guild_id}/primes/claim"),
                &serde_json::json!({
                    "target_id": target_id,
                    "claimer_id": claimer_id,
                    "claimer_name": claimer_name,
                }),
            )
            .await?;
        Ok(res.total_claimed)
    }

    // ── Insurance ──

    /// POST /api/coude/{guild_id}/insurance/buy
    pub async fn buy_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
        is_scam: bool,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/insurance/buy"),
                &serde_json::json!({ "user_id": user_id, "is_scam": is_scam }),
            )
            .await;
        Ok(())
    }

    /// GET /api/coude/{guild_id}/insurance/{user_id}
    pub async fn get_active_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Insurance>, String> {
        let path = format!("/api/coude/{guild_id}/insurance/{user_id}");
        let resp = self
            .base
            .auth(
                self.base
                    .client()
                    .get(format!("{}{}", self.base.base_url(), path)),
            )
            .send()
            .await
            .map_err(|e| format!("{e}"))?;
        if resp.status().as_u16() == 404 {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(format!("API error {}", resp.status()));
        }
        resp.json::<Option<Insurance>>()
            .await
            .map_err(|e| format!("{e}"))
    }

    /// POST /api/coude/insurance/{insurance_id}/expire
    pub async fn expire_insurance(&self, id: &str) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/insurance/{id}/expire"),
                &serde_json::json!({}),
            )
            .await;
        Ok(())
    }

    // ── Leaderboard ──

    /// GET /api/coude/{guild_id}/leaderboard/{category}?limit={limit}
    async fn leaderboard(
        &self,
        guild_id: &str,
        category: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        self.base
            .get_json(&format!(
                "/api/coude/{guild_id}/leaderboard/{category}?limit={limit}"
            ))
            .await
    }

    pub async fn leaderboard_richest(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        self.leaderboard(guild_id, "richest", limit).await
    }

    pub async fn leaderboard_thieves(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        self.leaderboard(guild_id, "thieves", limit).await
    }

    pub async fn leaderboard_cowards(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        self.leaderboard(guild_id, "cowards", limit).await
    }

    pub async fn leaderboard_chaos(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        self.leaderboard(guild_id, "chaos", limit).await
    }

    pub async fn leaderboard_level(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        self.leaderboard(guild_id, "level", limit).await
    }

    // ── Utility ──

    /// GET /api/coude/guilds
    pub async fn get_all_guild_ids(&self) -> Result<Vec<String>, String> {
        self.base.get_json("/api/coude/guilds").await
    }

    /// GET /api/coude/{guild_id}/players/random?count={count}
    pub async fn get_random_players(
        &self,
        guild_id: &str,
        count: usize,
    ) -> Result<Vec<Player>, String> {
        self.base
            .get_json(&format!(
                "/api/coude/{guild_id}/players/random?count={count}"
            ))
            .await
    }

    /// POST /api/coude/{guild_id}/daily-chaos
    pub async fn log_daily_chaos(
        &self,
        guild_id: &str,
        loser_id: &str,
        loser_name: &str,
        winner_id: &str,
        winner_name: &str,
        amount: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/daily-chaos"),
                &serde_json::json!({
                    "loser_id": loser_id,
                    "loser_name": loser_name,
                    "winner_id": winner_id,
                    "winner_name": winner_name,
                    "amount": amount,
                }),
            )
            .await;
        Ok(())
    }

    /// GET /api/coude/{guild_id}/events
    pub async fn get_active_events(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ServerEvent>, String> {
        self.base
            .get_json(&format!("/api/coude/{guild_id}/events"))
            .await
    }

    // ── Inventory ──

    /// POST /api/coude/{guild_id}/inventory/{user_id}/add
    pub async fn add_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/inventory/{user_id}/add"),
                &serde_json::json!({ "item_key": item_key }),
            )
            .await;
        Ok(())
    }

    /// GET /api/coude/{guild_id}/inventory/{user_id}
    pub async fn get_inventory(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<InventoryItem>, String> {
        self.base
            .get_json(&format!(
                "/api/coude/{guild_id}/inventory/{user_id}"
            ))
            .await
    }

    /// POST /api/coude/{guild_id}/inventory/{user_id}/use
    pub async fn use_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<bool, String> {
        let res: SuccessResponse = self
            .base
            .post_json(
                &format!("/api/coude/{guild_id}/inventory/{user_id}/use"),
                &serde_json::json!({ "item_key": item_key }),
            )
            .await?;
        Ok(res.success)
    }

    /// GET /api/coude/{guild_id}/inventory/{user_id}/has/{item_key}
    pub async fn has_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<bool, String> {
        let res: HasItemResponse = self
            .base
            .get_json(&format!(
                "/api/coude/{guild_id}/inventory/{user_id}/has/{item_key}"
            ))
            .await?;
        Ok(res.has_item)
    }

    // ── Coins (backwards compat helpers) ──

    /// PATCH /api/coude/players/{guild_id}/{user_id}/coins then GET to return updated player.
    pub async fn update_player_coins(
        &self,
        guild_id: &str,
        user_id: &str,
        delta: i64,
    ) -> Result<Player, String> {
        self.base
            .patch_fire_and_forget(
                &format!("/api/coude/players/{guild_id}/{user_id}/coins"),
                &serde_json::json!({ "amount": delta }),
            )
            .await;
        // Fetch the updated player to return it (matching db.rs signature).
        self.get_or_create_player(guild_id, user_id, "").await
    }

    /// Set player coins to an absolute value by computing delta.
    pub async fn set_player_coins(
        &self,
        guild_id: &str,
        user_id: &str,
        coins: i64,
    ) -> Result<(), String> {
        // Get current player to compute delta
        let player = self.get_or_create_player(guild_id, user_id, "").await?;
        let delta = coins - player.coins;
        if delta != 0 {
            self.base
                .patch_fire_and_forget(
                    &format!("/api/coude/players/{guild_id}/{user_id}/coins"),
                    &serde_json::json!({ "amount": delta }),
                )
                .await;
        }
        Ok(())
    }

    // ── HP ──

    pub async fn update_hp(
        &self,
        guild_id: &str,
        user_id: &str,
        hp_current: i32,
        hp_max: i32,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/hp"),
                &serde_json::json!({ "hp_current": hp_current, "hp_max": hp_max }),
            )
            .await;
        Ok(())
    }

    pub async fn repos(&self, guild_id: &str, user_id: &str) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/repos"),
                &serde_json::json!({}),
            )
            .await;
        Ok(())
    }
}
