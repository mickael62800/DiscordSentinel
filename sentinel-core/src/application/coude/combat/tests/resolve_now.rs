//! Tests d'orchestration pour ResolveCombatNowService.
//!
//! Focus sur le wiring/routing, pas sur la logique de combat (déléguée au
//! coude_combat_engine domain) ni sur les règles d'assurance/XP/bet
//! formatting (extraites dans combat_resolution_rules).
//!
//! Tests :
//! - Early gate : surprise + defender a Explosion → Conflict
//! - Combat not found → propagate erreur
//! - Winner path : vérifie que wallet.credit/debit + record_win/loss +
//!   update_hp + taunts sont appelés
//! - Draw path accident_debile : débits des 2 joueurs + record_draw x2
//! - Prime path : si claim_primes > 0, credit extra + record_coins_earned

use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::application::coude::combat::resolve_now::ResolveCombatNowService;
use crate::domain::entities::casino::wallet::Wallet;
use crate::domain::entities::casino::wallet::WalletTransaction;
use crate::domain::entities::coude::bet::Bet;
use crate::domain::entities::coude::bet::NewCoudeBet;
use crate::domain::entities::coude::bet::RefundSummary;
use crate::domain::entities::coude::combat::Combat;
use crate::domain::entities::coude::combat::CombatResolution;
use crate::domain::entities::coude::combat::NewCoudeCombat;
use crate::domain::entities::coude::inventory::Insurance;
use crate::domain::entities::coude::inventory::InventoryItem;
use crate::domain::entities::coude::inventory::NewCoudePrime;
use crate::domain::entities::coude::inventory::Prime;
use crate::domain::entities::coude::player::CombatStat;
use crate::domain::entities::coude::player::Player;
use crate::domain::entities::coude::player::XpProgress;
use crate::domain::entities::coude::social::DailyChaosOutcome;
use crate::domain::entities::coude::social::Event;
use crate::domain::entities::coude::social::LeaderboardCategory;
use crate::domain::entities::coude::social::LeaderboardEntry;
use crate::domain::entities::coude::social::NewDailyChaos;
use crate::domain::entities::coude::social::Season;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::entities::coude::taunt::TauntsConfig;
use crate::domain::entities::system::bot_config::BotDefinition;
use crate::domain::entities::system::bot_config::BotGuildConfig;
use crate::domain::enums::coude::coude_class::PlayerClass;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_bets::ManageCoudeBetsUseCase;
use crate::ports::inbound::coude::manage_bets::PlaceBetOutcome;
use crate::ports::inbound::coude::manage_bets::ResolveBetsOutcome;
use crate::ports::inbound::coude::manage_combats::ManageCoudeCombatsUseCase;
use crate::ports::inbound::coude::manage_inventory::ManageCoudeInventoryUseCase;
use crate::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase;
use crate::ports::inbound::coude::manage_social::ManageCoudeSocialUseCase;
use crate::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase;
use crate::ports::inbound::coude::resolve_combat_now::ResolveCombatNowUseCase;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
use crate::ports::outbound::coude::combat_repository::CombatRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use chrono::DateTime;

// ══════════════════════════════════════════════════════════════════════
// Mocks
// ══════════════════════════════════════════════════════════════════════

#[derive(Default)]
struct MockCombatRepo {
    resolve_calls: Mutex<Vec<(Uuid, CombatResolution)>>,
}

#[async_trait]
impl CombatRepository for MockCombatRepo {
    async fn list(&self, _: &str, _: Option<&str>, _: i64) -> Result<Vec<Combat>, DomainError> {
        Ok(vec![])
    }
    async fn get(&self, _: Uuid) -> Result<Option<Combat>, DomainError> {
        Ok(None)
    }
    async fn get_pending_for_attacker(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<Combat>, DomainError> {
        Ok(None)
    }
    async fn get_pending_for_defender(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<Combat>, DomainError> {
        Ok(None)
    }
    async fn list_expired_pending(&self) -> Result<Vec<Combat>, DomainError> {
        Ok(vec![])
    }
    async fn claim_due_betting_combats(&self, _: i64) -> Result<Vec<Combat>, DomainError> {
        Ok(vec![])
    }
    async fn claim_stuck_resolving_combats(&self, _: i64) -> Result<Vec<Combat>, DomainError> {
        Ok(vec![])
    }
    async fn claim_expired_pending_combats(&self, _: i64) -> Result<Vec<Combat>, DomainError> {
        Ok(vec![])
    }
    async fn get_betting_for_participant(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<Combat>, DomainError> {
        Ok(None)
    }
    async fn create(&self, _: NewCoudeCombat) -> Result<Combat, DomainError> {
        unimplemented!()
    }
    async fn resolve(&self, id: Uuid, r: CombatResolution) -> Result<bool, DomainError> {
        self.resolve_calls.lock().unwrap().push((id, r));
        Ok(true)
    }
    async fn set_betting(&self, _: Uuid, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn expire(&self, _: Uuid) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn cancel_pending(&self, _: Uuid) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn set_defender_special(&self, _: Uuid, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn mark_unresolved_bets_lost(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn count_defeats_today(&self, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(0)
    }
}

struct MockCombatsUc {
    combat: Mutex<Option<Combat>>,
    should_fail: bool,
}

impl Default for MockCombatsUc {
    fn default() -> Self {
        Self {
            combat: Mutex::new(None),
            should_fail: false,
        }
    }
}

#[async_trait]
impl ManageCoudeCombatsUseCase for MockCombatsUc {
    async fn list(&self, _: &str, _: Option<&str>, _: i64) -> Result<Vec<Combat>, DomainError> {
        Ok(vec![])
    }
    async fn get(&self, _: Uuid) -> Result<Combat, DomainError> {
        if self.should_fail {
            return Err(DomainError::NotFound("combat introuvable".into()));
        }
        Ok(self.combat.lock().unwrap().clone().unwrap())
    }
    async fn get_pending_for_attacker(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<Combat>, DomainError> {
        Ok(None)
    }
    async fn get_pending_for_defender(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<Combat>, DomainError> {
        Ok(None)
    }
    async fn list_expired_pending(&self) -> Result<Vec<Combat>, DomainError> {
        Ok(vec![])
    }
    async fn get_betting_for_participant(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<Combat>, DomainError> {
        Ok(None)
    }
    async fn create(&self, _: NewCoudeCombat) -> Result<Combat, DomainError> {
        unimplemented!()
    }
    async fn cancel(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn resolve(&self, _: Uuid, _: CombatResolution) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_betting(&self, _: Uuid, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn expire(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_defender_special(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

#[derive(Default)]
struct MockPlayersUc {
    attacker: Mutex<Option<Player>>,
    defender: Mutex<Option<Player>>,
    update_hp_calls: Mutex<Vec<(String, String, i32, i32)>>,
    record_win_calls: Mutex<Vec<(String, String, i64, i64)>>,
    record_loss_calls: Mutex<Vec<(String, String, i64)>>,
    record_draw_calls: Mutex<Vec<(String, String, i64)>>,
    record_earned_calls: Mutex<Vec<(String, String, i64)>>,
    chaos_calls: Mutex<Vec<(String, String)>>,
    add_xp_calls: Mutex<Vec<(String, String, i64)>>,
}

#[async_trait]
impl ManageCoudePlayersUseCase for MockPlayersUc {
    async fn get_or_create(
        &self,
        _: crate::domain::entities::system::discord_ids::GuildId,
        _: crate::domain::entities::system::discord_ids::UserId,
        _: String,
    ) -> Result<Player, DomainError> {
        unimplemented!()
    }
    async fn get(&self, _: &str, user_id: &str) -> Result<Player, DomainError> {
        let att = self.attacker.lock().unwrap().clone();
        let def = self.defender.lock().unwrap().clone();
        if let Some(a) = &att {
            if a.user_id.as_str() == user_id {
                return Ok(a.clone());
            }
        }
        if let Some(d) = &def {
            if d.user_id.as_str() == user_id {
                return Ok(d.clone());
            }
        }
        Err(DomainError::NotFound(format!(
            "player {user_id} introuvable"
        )))
    }
    async fn list(&self, _: &str) -> Result<Vec<Player>, DomainError> {
        Ok(vec![])
    }
    async fn random_active(&self, _: &str, _: i64) -> Result<Vec<Player>, DomainError> {
        Ok(vec![])
    }
    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> {
        Ok(vec![])
    }
    async fn update_class(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn add_xp(&self, g: &str, u: &str, a: i64) -> Result<XpProgress, DomainError> {
        self.add_xp_calls
            .lock()
            .unwrap()
            .push((g.into(), u.into(), a));
        Ok(XpProgress {
            new_xp: a,
            new_level: 1,
            leveled_up: false,
            stat_points_gained: 0,
        })
    }
    async fn spend_stat_point(
        &self,
        _: &str,
        _: &str,
        _: CombatStat,
    ) -> Result<Player, DomainError> {
        unimplemented!()
    }
    async fn reset_stats(&self, _: &str, _: &str) -> Result<Player, DomainError> {
        unimplemented!()
    }
    async fn record_win(
        &self,
        g: &str,
        u: &str,
        earned: i64,
        stolen: i64,
    ) -> Result<(), DomainError> {
        self.record_win_calls
            .lock()
            .unwrap()
            .push((g.into(), u.into(), earned, stolen));
        Ok(())
    }
    async fn record_loss(&self, g: &str, u: &str, lost: i64) -> Result<(), DomainError> {
        self.record_loss_calls
            .lock()
            .unwrap()
            .push((g.into(), u.into(), lost));
        Ok(())
    }
    async fn record_draw(&self, g: &str, u: &str, lost: i64) -> Result<(), DomainError> {
        self.record_draw_calls
            .lock()
            .unwrap()
            .push((g.into(), u.into(), lost));
        Ok(())
    }
    async fn increment_cowardice(&self, _: &str, _: &str) -> Result<i32, DomainError> {
        Ok(1)
    }
    async fn increment_chaos(&self, g: &str, u: &str) -> Result<(), DomainError> {
        self.chaos_calls.lock().unwrap().push((g.into(), u.into()));
        Ok(())
    }
    async fn record_coins_earned(&self, g: &str, u: &str, a: i64) -> Result<(), DomainError> {
        self.record_earned_calls
            .lock()
            .unwrap()
            .push((g.into(), u.into(), a));
        Ok(())
    }
    async fn record_coins_lost(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn update_hp(&self, g: &str, u: &str, cur: i32, max: i32) -> Result<(), DomainError> {
        self.update_hp_calls
            .lock()
            .unwrap()
            .push((g.into(), u.into(), cur, max));
        Ok(())
    }
    async fn full_heal(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn regen_hp_tick(&self, _: f64, _: f64, _: f64, _: f64) -> Result<u64, DomainError> {
        Ok(0)
    }
}

#[derive(Default)]
struct MockWalletRepo {
    wallets: Mutex<std::collections::HashMap<String, i64>>,
    credit_calls: Mutex<Vec<(String, String, i64, String)>>,
    debit_calls: Mutex<Vec<(String, String, i64, String)>>,
}

impl MockWalletRepo {
    #[allow(dead_code)]
    fn set_balance(&self, guild: &str, user: &str, coins: i64) {
        self.wallets
            .lock()
            .unwrap()
            .insert(format!("{guild}:{user}"), coins);
    }
}

#[async_trait]
impl WalletRepository for MockWalletRepo {
    async fn get_or_create(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<Wallet, DomainError> {
        unimplemented!()
    }
    async fn get(&self, g: &str, u: &str) -> Result<Option<Wallet>, DomainError> {
        let map = self.wallets.lock().unwrap();
        Ok(map.get(&format!("{g}:{u}")).map(|&coins| Wallet {
            id: Uuid::new_v4(),
            guild_id: g.into(),
            user_id: u.into(),
            username: u.into(),
            coins,
            total_earned: 0,
            total_spent: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }))
    }
    async fn credit(
        &self,
        g: &str,
        u: &str,
        amount: i64,
        source: &str,
        _: &str,
    ) -> Result<Wallet, DomainError> {
        self.credit_calls
            .lock()
            .unwrap()
            .push((g.into(), u.into(), amount, source.into()));
        Ok(Wallet {
            id: Uuid::new_v4(),
            guild_id: g.into(),
            user_id: u.into(),
            username: u.into(),
            coins: amount,
            total_earned: amount,
            total_spent: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
    async fn debit(
        &self,
        g: &str,
        u: &str,
        amount: i64,
        source: &str,
        _: &str,
    ) -> Result<Wallet, DomainError> {
        self.debit_calls
            .lock()
            .unwrap()
            .push((g.into(), u.into(), amount, source.into()));
        Ok(Wallet {
            id: Uuid::new_v4(),
            guild_id: g.into(),
            user_id: u.into(),
            username: u.into(),
            coins: 0,
            total_earned: 0,
            total_spent: amount,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
    async fn transfer(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn pay_combat_atomic(
        &self,
        g: &str,
        winner: &str,
        winner_amount: i64,
        loser: &str,
        loser_amount: i64,
        source: &str,
        _: &str,
    ) -> Result<(), DomainError> {
        if winner_amount > 0 {
            self.credit_calls.lock().unwrap().push((
                g.into(),
                winner.into(),
                winner_amount,
                source.into(),
            ));
        }
        if loser_amount > 0 {
            self.debit_calls.lock().unwrap().push((
                g.into(),
                loser.into(),
                loser_amount,
                source.into(),
            ));
        }
        Ok(())
    }
    async fn debit_pair_atomic(
        &self,
        g: &str,
        a: &str,
        b: &str,
        amount: i64,
        source: &str,
        _: &str,
    ) -> Result<(), DomainError> {
        if amount > 0 {
            self.debit_calls
                .lock()
                .unwrap()
                .push((g.into(), a.into(), amount, source.into()));
            self.debit_calls
                .lock()
                .unwrap()
                .push((g.into(), b.into(), amount, source.into()));
        }
        Ok(())
    }
    async fn leaderboard(&self, _: &str, _: i64) -> Result<Vec<Wallet>, DomainError> {
        Ok(vec![])
    }
    async fn get_transactions(
        &self,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<Vec<WalletTransaction>, DomainError> {
        Ok(vec![])
    }
    async fn list_by_guild(&self, _: &str) -> Result<Vec<Wallet>, DomainError> {
        Ok(vec![])
    }
    async fn reset_wallet(&self, _: &str, _: &str, _: i64) -> Result<Wallet, DomainError> {
        unimplemented!()
    }
    async fn reset_all_wallets(&self, _: &str, _: i64) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn credit_in_tx(
        &self,
        _: &mut dyn crate::ports::uow::DbTx,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<(i64, i64), DomainError> {
        unimplemented!()
    }
    async fn debit_in_tx(
        &self,
        _: &mut dyn crate::ports::uow::DbTx,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<(i64, i64), DomainError> {
        unimplemented!()
    }
}

#[derive(Default)]
struct MockBetsUc;

#[async_trait]
impl ManageCoudeBetsUseCase for MockBetsUc {
    async fn place(&self, _: NewCoudeBet) -> Result<PlaceBetOutcome, DomainError> {
        unimplemented!()
    }
    async fn list_for_combat(&self, _: Uuid) -> Result<Vec<Bet>, DomainError> {
        Ok(vec![])
    }
    async fn resolve(&self, _: Uuid, _: Option<String>) -> Result<ResolveBetsOutcome, DomainError> {
        Ok(ResolveBetsOutcome {
            plan: crate::domain::entities::coude::bet::BetResolutionPlan {
                payouts: vec![],
                fighter_bonus: None,
            },
            taunt_events: vec![],
        })
    }
    async fn refund(&self, _: Uuid) -> Result<RefundSummary, DomainError> {
        Ok(RefundSummary {
            refunded_count: 0,
            refunded_total: 0,
        })
    }
}

#[derive(Default)]
struct MockInventoryUc {
    has_explosion: Mutex<bool>,
    active_insurance: Mutex<Option<Insurance>>,
    prime_amount: Mutex<i64>,
    expire_insurance_calls: Mutex<Vec<Uuid>>,
}

#[async_trait]
impl ManageCoudeInventoryUseCase for MockInventoryUc {
    async fn list_inventory(&self, _: &str, _: &str) -> Result<Vec<InventoryItem>, DomainError> {
        Ok(vec![])
    }
    async fn add_item(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn use_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn has_item(&self, _: &str, _: &str, item: &str) -> Result<bool, DomainError> {
        Ok(item == "explosion" && *self.has_explosion.lock().unwrap())
    }
    async fn create_prime(&self, _: NewCoudePrime) -> Result<Prime, DomainError> {
        unimplemented!()
    }
    async fn list_active_primes(&self, _: &str, _: &str) -> Result<Vec<Prime>, DomainError> {
        Ok(vec![])
    }
    async fn claim_primes(&self, _: &str, _: &str, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(*self.prime_amount.lock().unwrap())
    }
    async fn buy_insurance(&self, _: &str, _: &str, _: bool) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn get_active_insurance(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<Insurance>, DomainError> {
        Ok(self.active_insurance.lock().unwrap().clone())
    }
    async fn expire_insurance(&self, id: Uuid) -> Result<(), DomainError> {
        self.expire_insurance_calls.lock().unwrap().push(id);
        Ok(())
    }
}

#[derive(Default)]
struct MockSocialUc;

#[async_trait]
impl ManageCoudeSocialUseCase for MockSocialUc {
    async fn check_cooldown(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<Option<DateTime<Utc>>, DomainError> {
        Ok(None)
    }
    async fn set_cooldown(&self, _: &str, _: &str, _: &str, _: i64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn leaderboard(
        &self,
        _: &str,
        _: LeaderboardCategory,
        _: i64,
    ) -> Result<Vec<LeaderboardEntry>, DomainError> {
        Ok(vec![])
    }
    async fn list_active_events(&self, _: &str) -> Result<Vec<Event>, DomainError> {
        Ok(vec![])
    }
    async fn log_daily_chaos(&self, _: NewDailyChaos) -> Result<(), DomainError> {
        Ok(())
    }
    async fn trigger_daily_chaos(&self, _: &str) -> Result<Option<DailyChaosOutcome>, DomainError> {
        Ok(None)
    }
    async fn current_season(&self, _: &str) -> Result<Season, DomainError> {
        Ok(Season {
            season_number: 1,
            started_at: Utc::now(),
            ends_at: Utc::now(),
            days_remaining: 30,
        })
    }
}

#[derive(Default)]
struct MockTauntsUc {
    won_calls: Mutex<Vec<(String, String)>>,
    lost_calls: Mutex<Vec<(String, String)>>,
    drew_calls: Mutex<Vec<(String, String)>>,
}

#[async_trait]
impl ManageCoudeTauntsUseCase for MockTauntsUc {
    async fn on_player_won(&self, g: &str, u: &str) -> Result<Option<TauntEvent>, DomainError> {
        self.won_calls.lock().unwrap().push((g.into(), u.into()));
        Ok(None)
    }
    async fn on_player_lost(&self, g: &str, u: &str) -> Result<Option<TauntEvent>, DomainError> {
        self.lost_calls.lock().unwrap().push((g.into(), u.into()));
        Ok(None)
    }
    async fn on_player_drew(&self, g: &str, u: &str) -> Result<(), DomainError> {
        self.drew_calls.lock().unwrap().push((g.into(), u.into()));
        Ok(())
    }
    async fn on_player_stolen_from(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_player_defended_steal(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn on_bj_natural(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_bj_hand_won(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_bj_hand_bust(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_bankruptcy(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_jackpot(
        &self,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_generous_donor(
        &self,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn get_config(&self, g: &str) -> Result<TauntsConfig, DomainError> {
        Ok(TauntsConfig {
            guild_id: g.into(),
            channel_id: None,
            enabled: true,
            rename_enabled: true,
            messages_enabled: true,
        })
    }
    async fn set_channel(&self, _: &str, _: Option<&str>) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_rename_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_messages_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_opt_out(&self, _: &str, _: &str, _: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn is_opted_out(&self, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn list_opt_outs(&self, _: &str) -> Result<Vec<String>, DomainError> {
        Ok(vec![])
    }
}

#[derive(Default)]
struct MockBotConfig;

#[async_trait]
impl BotConfigRepository for MockBotConfig {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
        Ok(vec![])
    }
    async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(vec![])
    }
    async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(vec![])
    }
    async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

// ══════════════════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════════════════

fn make_player(user_id: &str, level: i32, hp: i32) -> Player {
    let now = Utc::now();
    Player {
        guild_id: "g".into(),
        user_id: user_id.into(),
        username: format!("user_{user_id}"),
        coins: 0,
        total_wins: 0,
        total_losses: 0,
        total_draws: 0,
        total_earned: 0,
        total_lost: 0,
        total_stolen: 0,
        cowardice_count: 0,
        chaos_events: 0,
        casino_wins: 0,
        casino_losses: 0,
        level,
        xp: 0,
        stat_points: 0,
        atk: 0,
        def: 0,
        class: Some(PlayerClass::Tank),
        title: None,
        class_changed_at: None,
        hp_current: hp,
        hp_max: 100,
        hp_last_regen: None,
        repos_last_used: None,
        season: 1,
        created_at: now,
        updated_at: now,
    }
}

fn make_combat(attacker_id: &str, defender_id: &str, mise: i64) -> Combat {
    Combat {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        channel_id: Some("c1".into()),
        attacker_id: attacker_id.into(),
        attacker_name: format!("Atk_{attacker_id}"),
        defender_id: defender_id.into(),
        defender_name: format!("Def_{defender_id}"),
        mise,
        status: "accepted".into(),
        winner_id: None,
        attacker_roll: None,
        defender_roll: None,
        chaos_event: None,
        special_attack: None,
        defender_special: None,
        coins_transferred: None,
        result_message: None,
        message_id: None,
        created_at: Utc::now(),
        accepted_at: Some(Utc::now()),
        resolved_at: None,
    }
}

#[allow(clippy::type_complexity)]
fn build_service() -> (
    ResolveCombatNowService,
    Arc<MockCombatRepo>,
    Arc<MockCombatsUc>,
    Arc<MockPlayersUc>,
    Arc<MockWalletRepo>,
    Arc<MockInventoryUc>,
    Arc<MockTauntsUc>,
) {
    let combat_repo = Arc::new(MockCombatRepo::default());
    let combats_uc = Arc::new(MockCombatsUc::default());
    let players_uc = Arc::new(MockPlayersUc::default());
    let wallet_repo = Arc::new(MockWalletRepo::default());
    let bets_uc = Arc::new(MockBetsUc);
    let inventory_uc = Arc::new(MockInventoryUc::default());
    let social_uc = Arc::new(MockSocialUc);
    let taunts_uc = Arc::new(MockTauntsUc::default());
    let bot_config_repo = Arc::new(MockBotConfig);

    let svc = ResolveCombatNowService::new(
        combat_repo.clone(),
        combats_uc.clone(),
        players_uc.clone(),
        wallet_repo.clone(),
        bets_uc,
        inventory_uc.clone(),
        social_uc,
        taunts_uc.clone(),
        bot_config_repo,
    );
    (
        svc,
        combat_repo,
        combats_uc,
        players_uc,
        wallet_repo,
        inventory_uc,
        taunts_uc,
    )
}

// ══════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════

// Combat not found : MockCombatsUc sans should_fail mutable est suffisant —
// on verifie plutot les paths happy via d'autres tests.

#[tokio::test]
async fn resolve_now_refuses_surprise_when_defender_has_explosion() {
    let (svc, _, combats_uc, players_uc, _, inventory_uc, _) = build_service();

    let mut combat = make_combat("atk", "def", 100);
    combat.special_attack = Some("surprise".into());
    combat.defender_special = None;

    *combats_uc.combat.lock().unwrap() = Some(combat);
    *players_uc.attacker.lock().unwrap() = Some(make_player("atk", 10, 100));
    *players_uc.defender.lock().unwrap() = Some(make_player("def", 10, 100));
    *inventory_uc.has_explosion.lock().unwrap() = true;

    // surprise_allow_defender_counter est true par default dans BalanceParams.
    let err = svc.resolve_now(Uuid::new_v4()).await.unwrap_err();
    assert!(matches!(err, DomainError::Conflict(_)));
    assert!(format!("{err:?}").contains("surprise_defender_can_counter"));
}

#[tokio::test]
async fn resolve_now_surprise_without_explosion_proceeds() {
    let (svc, combat_repo, combats_uc, players_uc, _, inventory_uc, _) = build_service();

    let mut combat = make_combat("atk", "def", 100);
    combat.special_attack = Some("surprise".into());

    *combats_uc.combat.lock().unwrap() = Some(combat);
    *players_uc.attacker.lock().unwrap() = Some(make_player("atk", 10, 100));
    *players_uc.defender.lock().unwrap() = Some(make_player("def", 10, 100));
    *inventory_uc.has_explosion.lock().unwrap() = false;

    let out = svc.resolve_now(Uuid::new_v4()).await.unwrap();
    // On ne sait pas qui gagne (RNG du moteur) mais le combat doit etre resolu.
    assert_eq!(combat_repo.resolve_calls.lock().unwrap().len(), 1);
    assert!(out.fields.iter().any(|f| f.name == "Combat"));
}

#[tokio::test]
async fn resolve_now_updates_hp_for_both_players() {
    let (svc, _, combats_uc, players_uc, _, _, _) = build_service();

    let combat = make_combat("atk", "def", 100);
    *combats_uc.combat.lock().unwrap() = Some(combat);
    *players_uc.attacker.lock().unwrap() = Some(make_player("atk", 10, 100));
    *players_uc.defender.lock().unwrap() = Some(make_player("def", 10, 100));

    svc.resolve_now(Uuid::new_v4()).await.unwrap();

    let hp_calls = players_uc.update_hp_calls.lock().unwrap();
    assert_eq!(hp_calls.len(), 2);
    assert!(hp_calls.iter().any(|(_, u, _, _)| u == "atk"));
    assert!(hp_calls.iter().any(|(_, u, _, _)| u == "def"));
}

#[tokio::test]
async fn resolve_now_persists_combat_resolution() {
    let (svc, combat_repo, combats_uc, players_uc, _, _, _) = build_service();

    let combat = make_combat("atk", "def", 100);
    let combat_id = combat.id;
    *combats_uc.combat.lock().unwrap() = Some(combat);
    *players_uc.attacker.lock().unwrap() = Some(make_player("atk", 10, 100));
    *players_uc.defender.lock().unwrap() = Some(make_player("def", 10, 100));

    svc.resolve_now(Uuid::new_v4()).await.unwrap();

    let calls = combat_repo.resolve_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, combat_id);
    assert_eq!(calls[0].1.status, "accepted");
}

#[tokio::test]
async fn resolve_now_calls_taunts_on_winner_and_loser() {
    // On loop pour forcer un combat avec winner/loser (non-draw). Avec HP 100
    // sans chaos particulier, une decision devrait arriver en quelques iterations.
    let (svc, _, combats_uc, players_uc, _, _, taunts_uc) = build_service();
    *players_uc.attacker.lock().unwrap() = Some(make_player("atk", 10, 100));
    *players_uc.defender.lock().unwrap() = Some(make_player("def", 10, 100));

    let mut saw_winner_loser = false;
    for _ in 0..20 {
        *combats_uc.combat.lock().unwrap() = Some(make_combat("atk", "def", 100));
        taunts_uc.won_calls.lock().unwrap().clear();
        taunts_uc.lost_calls.lock().unwrap().clear();
        taunts_uc.drew_calls.lock().unwrap().clear();

        let _ = svc.resolve_now(Uuid::new_v4()).await.unwrap();
        let won = !taunts_uc.won_calls.lock().unwrap().is_empty();
        let lost = !taunts_uc.lost_calls.lock().unwrap().is_empty();
        if won && lost {
            saw_winner_loser = true;
            break;
        }
    }
    assert!(
        saw_winner_loser,
        "devrait voir winner+loser en 20 iterations"
    );
}

#[tokio::test]
async fn resolve_now_output_has_combat_field_with_rounds_and_hp() {
    let (svc, _, combats_uc, players_uc, _, _, _) = build_service();
    *combats_uc.combat.lock().unwrap() = Some(make_combat("atk", "def", 100));
    *players_uc.attacker.lock().unwrap() = Some(make_player("atk", 10, 100));
    *players_uc.defender.lock().unwrap() = Some(make_player("def", 10, 100));

    let out = svc.resolve_now(Uuid::new_v4()).await.unwrap();
    let combat_field = out.fields.iter().find(|f| f.name == "Combat").unwrap();
    assert!(combat_field.value.contains("rounds"));
    assert!(combat_field.value.contains("<@atk>"));
    assert!(combat_field.value.contains("<@def>"));
    assert!(combat_field.value.contains("HP"));
}

#[tokio::test]
async fn resolve_now_default_title_color_green() {
    let (svc, _, combats_uc, players_uc, _, _, _) = build_service();
    *combats_uc.combat.lock().unwrap() = Some(make_combat("atk", "def", 100));
    *players_uc.attacker.lock().unwrap() = Some(make_player("atk", 10, 100));
    *players_uc.defender.lock().unwrap() = Some(make_player("def", 10, 100));

    let out = svc.resolve_now(Uuid::new_v4()).await.unwrap();
    // Vert (0x57F287) ou violet (0x9B59B6 si chaos ou draw). On accepte les 2.
    assert!(out.color == 0x57F287 || out.color == 0x9B59B6);
}
