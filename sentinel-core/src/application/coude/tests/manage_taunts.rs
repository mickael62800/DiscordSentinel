use async_trait::async_trait;
use std::sync::Arc;
use std::sync::Mutex;

use crate::domain::entities::coude::player::CombatStat;
use crate::domain::entities::coude::player::Player;
use crate::domain::entities::coude::player::XpProgress;
use crate::domain::entities::coude::taunt::TauntsConfig;
use crate::domain::entities::system::bot_config::BotGuildConfig;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase;
use crate::ports::outbound::coude::player_repository::PlayerRepository;
use crate::ports::outbound::coude::taunts_repository::TauntsRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
// ══════════════════════════════════════════════════════════
// Mocks
// ══════════════════════════════════════════════════════════

struct MockTauntsRepo {
    config: Mutex<TauntsConfig>,
    opted_out: Mutex<bool>,
    opt_out_calls: Mutex<Vec<(String, bool)>>,
    set_channel_calls: Mutex<Vec<Option<String>>>,
    set_enabled_calls: Mutex<Vec<bool>>,
    set_rename_calls: Mutex<Vec<bool>>,
    set_messages_calls: Mutex<Vec<bool>>,
    opt_outs_list: Mutex<Vec<String>>,
}

impl MockTauntsRepo {
    fn default_config() -> TauntsConfig {
        TauntsConfig {
            guild_id: "g".into(),
            channel_id: Some("chan-1".into()),
            enabled: true,
            rename_enabled: true,
            messages_enabled: true,
        }
    }
    fn new() -> Self {
        Self {
            config: Mutex::new(Self::default_config()),
            opted_out: Mutex::new(false),
            opt_out_calls: Mutex::new(vec![]),
            set_channel_calls: Mutex::new(vec![]),
            set_enabled_calls: Mutex::new(vec![]),
            set_rename_calls: Mutex::new(vec![]),
            set_messages_calls: Mutex::new(vec![]),
            opt_outs_list: Mutex::new(vec![]),
        }
    }
    fn with_config(self, f: impl FnOnce(&mut TauntsConfig)) -> Self {
        f(&mut *self.config.lock().unwrap());
        self
    }
}

#[async_trait]
impl TauntsRepository for MockTauntsRepo {
    async fn get_or_init_config(&self, _: &str) -> Result<TauntsConfig, DomainError> {
        Ok(self.config.lock().unwrap().clone())
    }
    async fn set_channel(&self, _: &str, c: Option<&str>) -> Result<(), DomainError> {
        self.set_channel_calls
            .lock()
            .unwrap()
            .push(c.map(String::from));
        Ok(())
    }
    async fn set_enabled(&self, _: &str, e: bool) -> Result<(), DomainError> {
        self.set_enabled_calls.lock().unwrap().push(e);
        Ok(())
    }
    async fn set_rename_enabled(&self, _: &str, e: bool) -> Result<(), DomainError> {
        self.set_rename_calls.lock().unwrap().push(e);
        Ok(())
    }
    async fn set_messages_enabled(&self, _: &str, e: bool) -> Result<(), DomainError> {
        self.set_messages_calls.lock().unwrap().push(e);
        Ok(())
    }
    async fn is_opted_out(&self, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(*self.opted_out.lock().unwrap())
    }
    async fn list_opt_outs(&self, _: &str) -> Result<Vec<String>, DomainError> {
        Ok(self.opt_outs_list.lock().unwrap().clone())
    }
    async fn set_opt_out(&self, _: &str, u: &str, o: bool) -> Result<(), DomainError> {
        self.opt_out_calls.lock().unwrap().push((u.into(), o));
        Ok(())
    }
}

#[derive(Default)]
struct MockPlayerRepo {
    win_streak: Mutex<Option<i32>>,
    loss_streak: Mutex<Option<i32>>,
    steal_streak: Mutex<Option<i32>>,
    bj_win_streak: Mutex<Option<i32>>,
    bj_bust_streak: Mutex<Option<i32>>,
    reset_combat_calls: Mutex<u32>,
    reset_steal_calls: Mutex<u32>,
}

#[async_trait]
impl PlayerRepository for MockPlayerRepo {
    async fn get_or_create(&self, _: &str, _: &str, _: &str) -> Result<Player, DomainError> {
        unimplemented!()
    }
    async fn get(&self, _: &str, _: &str) -> Result<Option<Player>, DomainError> {
        unimplemented!()
    }
    async fn list(&self, _: &str, _: i64) -> Result<Vec<Player>, DomainError> {
        unimplemented!()
    }
    async fn random_active(&self, _: &str, _: i64, _: i64) -> Result<Vec<Player>, DomainError> {
        unimplemented!()
    }
    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> {
        unimplemented!()
    }
    async fn update_class(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn add_xp(&self, _: &str, _: &str, _: i64) -> Result<Option<XpProgress>, DomainError> {
        unimplemented!()
    }
    async fn spend_stat_point(
        &self,
        _: &str,
        _: &str,
        _: CombatStat,
    ) -> Result<Option<Player>, DomainError> {
        unimplemented!()
    }
    async fn reset_stats(&self, _: &str, _: &str, _: i64) -> Result<Option<Player>, DomainError> {
        unimplemented!()
    }
    async fn record_coins_earned(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn record_coins_lost(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn record_win(&self, _: &str, _: &str, _: i64, _: i64) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn record_loss(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn record_draw(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn touch_win_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> {
        Ok(*self.win_streak.lock().unwrap())
    }
    async fn touch_loss_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> {
        Ok(*self.loss_streak.lock().unwrap())
    }
    async fn reset_combat_streaks(&self, _: &str, _: &str) -> Result<(), DomainError> {
        *self.reset_combat_calls.lock().unwrap() += 1;
        Ok(())
    }
    async fn touch_steal_victim_streak(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<i32>, DomainError> {
        Ok(*self.steal_streak.lock().unwrap())
    }
    async fn reset_steal_victim_streak(&self, _: &str, _: &str) -> Result<(), DomainError> {
        *self.reset_steal_calls.lock().unwrap() += 1;
        Ok(())
    }
    async fn touch_bj_win_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> {
        Ok(*self.bj_win_streak.lock().unwrap())
    }
    async fn touch_bj_bust_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> {
        Ok(*self.bj_bust_streak.lock().unwrap())
    }
    async fn reset_bj_bust_streak(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn increment_cowardice(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> {
        unimplemented!()
    }
    async fn increment_chaos(&self, _: &str, _: &str) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn update_hp(&self, _: &str, _: &str, _: i32, _: i32) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn full_heal(&self, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn regen_hp_tick(&self, _: f64, _: f64, _: f64, _: f64) -> Result<u64, DomainError> {
        unimplemented!()
    }
}

#[derive(Default)]
struct MockBotConfigRepo {
    entries: Mutex<Vec<BotGuildConfig>>,
}
impl MockBotConfigRepo {
    fn with(pairs: &[(&str, &str)]) -> Self {
        let m = Self::default();
        for (k, v) in pairs {
            m.entries.lock().unwrap().push(BotGuildConfig {
                id: uuid::Uuid::new_v4(),
                guild_id: "g".into(),
                bot_name: "coude-bot".into(),
                config_key: k.to_string(),
                config_value: v.to_string(),
                updated_at: chrono::Utc::now(),
            });
        }
        m
    }
}
#[async_trait]
impl BotConfigRepository for MockBotConfigRepo {
    async fn get_definitions(
        &self,
    ) -> Result<Vec<crate::domain::entities::system::bot_config::BotDefinition>, DomainError> {
        Ok(vec![])
    }
    async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(self.entries.lock().unwrap().clone())
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

fn make_service(
    taunts: Arc<MockTauntsRepo>,
    players: Arc<MockPlayerRepo>,
    bot: Arc<MockBotConfigRepo>,
) -> crate::application::coude::manage_taunts_service::ManageCoudeTauntsService {
    crate::application::coude::manage_taunts_service::ManageCoudeTauntsService::new(
        taunts, players, bot,
    )
}

// ══════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn on_player_won_no_threshold_returns_none() {
    let p = Arc::new(MockPlayerRepo::default());
    *p.win_streak.lock().unwrap() = Some(1); // sous threshold (first is 3)
    let svc = make_service(
        Arc::new(MockTauntsRepo::new()),
        p,
        Arc::new(MockBotConfigRepo::default()),
    );
    let ev = svc.on_player_won("g", "u").await.unwrap();
    assert!(ev.is_none());
}

#[tokio::test]
async fn on_player_won_threshold_crossed_emits_event() {
    let p = Arc::new(MockPlayerRepo::default());
    *p.win_streak.lock().unwrap() = Some(3);
    let svc = make_service(
        Arc::new(MockTauntsRepo::new()),
        p,
        Arc::new(MockBotConfigRepo::default()),
    );
    let ev = svc.on_player_won("g", "u").await.unwrap();
    assert!(ev.is_some());
    assert_eq!(ev.unwrap().streak_value, 3);
}

#[tokio::test]
async fn on_player_won_player_none_returns_none() {
    let p = Arc::new(MockPlayerRepo::default()); // win_streak = None
    let svc = make_service(
        Arc::new(MockTauntsRepo::new()),
        p,
        Arc::new(MockBotConfigRepo::default()),
    );
    assert!(svc.on_player_won("g", "u").await.unwrap().is_none());
}

#[tokio::test]
async fn gate_disabled_returns_none() {
    let t = Arc::new(MockTauntsRepo::new().with_config(|c| c.enabled = false));
    let p = Arc::new(MockPlayerRepo::default());
    *p.win_streak.lock().unwrap() = Some(3);
    let svc = make_service(t, p, Arc::new(MockBotConfigRepo::default()));
    assert!(svc.on_player_won("g", "u").await.unwrap().is_none());
}

#[tokio::test]
async fn gate_no_channel_returns_none() {
    let t = Arc::new(MockTauntsRepo::new().with_config(|c| c.channel_id = None));
    let p = Arc::new(MockPlayerRepo::default());
    *p.win_streak.lock().unwrap() = Some(3);
    let svc = make_service(t, p, Arc::new(MockBotConfigRepo::default()));
    assert!(svc.on_player_won("g", "u").await.unwrap().is_none());
}

#[tokio::test]
async fn on_player_lost_and_drew() {
    let p = Arc::new(MockPlayerRepo::default());
    *p.loss_streak.lock().unwrap() = Some(3);
    let svc = make_service(
        Arc::new(MockTauntsRepo::new()),
        p.clone(),
        Arc::new(MockBotConfigRepo::default()),
    );
    assert!(svc.on_player_lost("g", "u").await.unwrap().is_some());
    svc.on_player_drew("g", "u").await.unwrap();
    assert_eq!(*p.reset_combat_calls.lock().unwrap(), 1);
}

#[tokio::test]
async fn on_player_stolen_from_and_defended() {
    let p = Arc::new(MockPlayerRepo::default());
    *p.steal_streak.lock().unwrap() = Some(3);
    let svc = make_service(
        Arc::new(MockTauntsRepo::new()),
        p.clone(),
        Arc::new(MockBotConfigRepo::default()),
    );
    assert!(svc.on_player_stolen_from("g", "u").await.unwrap().is_some());
    svc.on_player_defended_steal("g", "u").await.unwrap();
    assert_eq!(*p.reset_steal_calls.lock().unwrap(), 1);
}

#[tokio::test]
async fn on_bj_natural_one_shot() {
    let p = Arc::new(MockPlayerRepo::default());
    *p.bj_win_streak.lock().unwrap() = Some(1);
    let svc = make_service(
        Arc::new(MockTauntsRepo::new()),
        p,
        Arc::new(MockBotConfigRepo::default()),
    );
    let ev = svc.on_bj_natural("g", "u").await.unwrap();
    assert!(ev.is_some());
}

#[tokio::test]
async fn on_bj_hand_won_threshold() {
    let p = Arc::new(MockPlayerRepo::default());
    *p.bj_win_streak.lock().unwrap() = Some(3);
    let svc = make_service(
        Arc::new(MockTauntsRepo::new()),
        p,
        Arc::new(MockBotConfigRepo::default()),
    );
    assert!(svc.on_bj_hand_won("g", "u").await.unwrap().is_some());
}

#[tokio::test]
async fn on_bj_hand_bust_threshold() {
    let p = Arc::new(MockPlayerRepo::default());
    *p.bj_bust_streak.lock().unwrap() = Some(3);
    let svc = make_service(
        Arc::new(MockTauntsRepo::new()),
        p,
        Arc::new(MockBotConfigRepo::default()),
    );
    assert!(svc.on_bj_hand_bust("g", "u").await.unwrap().is_some());
}

#[tokio::test]
async fn bankruptcy_disabled_by_config_returns_none() {
    let bot = Arc::new(MockBotConfigRepo::with(&[(
        "bankruptcy_taunt_enabled",
        "false",
    )]));
    let svc = make_service(
        Arc::new(MockTauntsRepo::new()),
        Arc::new(MockPlayerRepo::default()),
        bot,
    );
    assert!(svc.on_bankruptcy("g", "u").await.unwrap().is_none());
}

#[tokio::test]
async fn bankruptcy_enabled_emits_event() {
    let svc = make_service(
        Arc::new(MockTauntsRepo::new()),
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockBotConfigRepo::default()),
    );
    assert!(svc.on_bankruptcy("g", "u").await.unwrap().is_some());
}

#[tokio::test]
async fn jackpot_below_threshold_returns_none() {
    let svc = make_service(
        Arc::new(MockTauntsRepo::new()),
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockBotConfigRepo::default()),
    );
    // default = 10_000
    assert!(svc.on_jackpot("g", "u", 5_000).await.unwrap().is_none());
}

#[tokio::test]
async fn jackpot_at_threshold_emits() {
    let svc = make_service(
        Arc::new(MockTauntsRepo::new()),
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockBotConfigRepo::default()),
    );
    assert!(svc.on_jackpot("g", "u", 10_000).await.unwrap().is_some());
}

#[tokio::test]
async fn jackpot_custom_threshold_from_config() {
    let bot = Arc::new(MockBotConfigRepo::with(&[("jackpot_threshold", "500")]));
    let svc = make_service(
        Arc::new(MockTauntsRepo::new()),
        Arc::new(MockPlayerRepo::default()),
        bot,
    );
    assert!(svc.on_jackpot("g", "u", 600).await.unwrap().is_some());
    assert!(svc.on_jackpot("g", "u", 400).await.unwrap().is_none());
}

#[tokio::test]
async fn donor_below_threshold_returns_none() {
    let svc = make_service(
        Arc::new(MockTauntsRepo::new()),
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockBotConfigRepo::default()),
    );
    // default = 1_000
    assert!(svc
        .on_generous_donor("g", "u", 500)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn donor_at_threshold_emits() {
    let svc = make_service(
        Arc::new(MockTauntsRepo::new()),
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockBotConfigRepo::default()),
    );
    assert!(svc
        .on_generous_donor("g", "u", 1_000)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn config_setters_forward_to_repo() {
    let t = Arc::new(MockTauntsRepo::new());
    let svc = make_service(
        t.clone(),
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockBotConfigRepo::default()),
    );

    svc.set_channel("g", Some("c2")).await.unwrap();
    svc.set_enabled("g", false).await.unwrap();
    svc.set_rename_enabled("g", false).await.unwrap();
    svc.set_messages_enabled("g", false).await.unwrap();
    svc.set_opt_out("g", "u1", true).await.unwrap();

    assert_eq!(
        t.set_channel_calls.lock().unwrap()[0].as_deref(),
        Some("c2")
    );
    assert_eq!(t.set_enabled_calls.lock().unwrap()[0], false);
    assert_eq!(t.set_rename_calls.lock().unwrap()[0], false);
    assert_eq!(t.set_messages_calls.lock().unwrap()[0], false);
    assert_eq!(t.opt_out_calls.lock().unwrap()[0], ("u1".into(), true));
}

#[tokio::test]
async fn get_config_and_opt_outs_forward() {
    let t = Arc::new(MockTauntsRepo::new());
    *t.opted_out.lock().unwrap() = true;
    *t.opt_outs_list.lock().unwrap() = vec!["u1".into(), "u2".into()];
    let svc = make_service(
        t,
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockBotConfigRepo::default()),
    );
    assert!(svc.get_config("g").await.is_ok());
    assert_eq!(svc.is_opted_out("g", "u").await.unwrap(), true);
    assert_eq!(svc.list_opt_outs("g").await.unwrap().len(), 2);
}
