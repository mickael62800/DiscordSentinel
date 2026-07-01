use crate::application::coude::steal::roll::RollStealService;
use crate::domain::entities::coude::steal::roll::STEAL_D20_MAX;
use crate::domain::entities::coude::steal::roll::STEAL_D20_MIN;
use crate::domain::entities::coude::steal::STEAL_PCT_ACTIVE_MAX_BP;
use crate::domain::entities::coude::steal::STEAL_PCT_ACTIVE_MIN_BP;
use crate::domain::entities::coude::steal::STEAL_PCT_AFK_MAX_BP;
use crate::domain::entities::coude::steal::STEAL_PCT_AFK_MIN_BP;
use crate::ports::inbound::coude::roll_steal::RollStealCommand;
use crate::ports::inbound::coude::roll_steal::RollStealUseCase;
#[tokio::test]
async fn rolls_within_d20_bounds() {
    let svc = RollStealService::new();
    for _ in 0..50 {
        let r = svc.roll(RollStealCommand { guild_id: "g".into(), afk: true }).await.unwrap();
        assert!(r.thief_d20 >= STEAL_D20_MIN && r.thief_d20 <= STEAL_D20_MAX);
        assert!(r.victim_d20 >= STEAL_D20_MIN && r.victim_d20 <= STEAL_D20_MAX);
    }
}

#[tokio::test]
async fn afk_pct_in_afk_range() {
    let svc = RollStealService::new();
    for _ in 0..50 {
        let r = svc.roll(RollStealCommand { guild_id: "g".into(), afk: true }).await.unwrap();
        assert!(
            r.steal_pct_bp >= STEAL_PCT_AFK_MIN_BP && r.steal_pct_bp <= STEAL_PCT_AFK_MAX_BP,
            "{}",
            r.steal_pct_bp
        );
    }
}

#[tokio::test]
async fn active_pct_in_active_range() {
    let svc = RollStealService::new();
    for _ in 0..50 {
        let r = svc
            .roll(RollStealCommand {
                guild_id: "g".into(),
                afk: false,
            })
            .await
            .unwrap();
        assert!(
            r.steal_pct_bp >= STEAL_PCT_ACTIVE_MIN_BP
                && r.steal_pct_bp <= STEAL_PCT_ACTIVE_MAX_BP,
            "{}",
            r.steal_pct_bp
        );
    }
}
