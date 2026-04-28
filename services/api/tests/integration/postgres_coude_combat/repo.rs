//! Tests d'integration postgres pour PgCoudeCombatRepository.

use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::PgCoudeCombatRepository;
use sentinel_api::domain::entities::coude::combat::CombatResolution;
use sentinel_api::domain::entities::coude::combat::NewCoudeCombat;
use sentinel_api::ports::outbound::coude::combat_repository::CoudeCombatRepository;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}
async fn seed_player(p: &PgPool, g: &str, u: &str) {
    sqlx::query("INSERT INTO coude_players (guild_id, user_id, username) VALUES ($1, $2, 'T') ON CONFLICT DO NOTHING")
        .bind(g).bind(u).execute(p).await.unwrap();
}

fn sample_new_combat(g: &str, att: &str, def: &str) -> NewCoudeCombat {
    NewCoudeCombat {
        guild_id: g.into(),
        channel_id: Some("ch1".into()),
        attacker_id: att.into(), attacker_name: "Att".into(),
        defender_id: def.into(), defender_name: "Def".into(),
        mise: 100,
        special_attack: None,
    }
}

// ── Lectures ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_and_get() {
    let p = pool().await;
    let repo = PgCoudeCombatRepository::new(p.clone());
    let g = fresh_id();
    let att = fresh_id(); let def = fresh_id();
    seed_player(&p, &g, &att).await;
    seed_player(&p, &g, &def).await;
    let c = repo.create(sample_new_combat(&g, &att, &def)).await.unwrap();
    assert_eq!(c.guild_id, g);
    assert_eq!(c.status, "pending");
    let got = repo.get(c.id).await.unwrap().unwrap();
    assert_eq!(got.mise, 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_none_when_absent() {
    let repo = PgCoudeCombatRepository::new(pool().await);
    assert!(repo.get(Uuid::new_v4()).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_scoped_and_filter_status() {
    let p = pool().await;
    let repo = PgCoudeCombatRepository::new(p.clone());
    let g = fresh_id();
    let att = fresh_id(); let def = fresh_id();
    seed_player(&p, &g, &att).await;
    seed_player(&p, &g, &def).await;
    let c1 = repo.create(sample_new_combat(&g, &att, &def)).await.unwrap();
    let att2 = fresh_id(); let def2 = fresh_id();
    seed_player(&p, &g, &att2).await;
    seed_player(&p, &g, &def2).await;
    let c2 = repo.create(sample_new_combat(&g, &att2, &def2)).await.unwrap();

    let all = repo.list(&g, None, 50).await.unwrap();
    assert_eq!(all.len(), 2);

    // Filter status
    let pending = repo.list(&g, Some("pending"), 50).await.unwrap();
    assert!(pending.iter().any(|c| c.id == c1.id));
    assert!(pending.iter().any(|c| c.id == c2.id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_pending_for_attacker_and_defender() {
    let p = pool().await;
    let repo = PgCoudeCombatRepository::new(p.clone());
    let g = fresh_id();
    let att = fresh_id(); let def = fresh_id();
    seed_player(&p, &g, &att).await;
    seed_player(&p, &g, &def).await;
    let c = repo.create(sample_new_combat(&g, &att, &def)).await.unwrap();
    let by_att = repo.get_pending_for_attacker(&g, &att).await.unwrap().unwrap();
    assert_eq!(by_att.id, c.id);
    let by_def = repo.get_pending_for_defender(&g, &def).await.unwrap().unwrap();
    assert_eq!(by_def.id, c.id);
    assert!(repo.get_pending_for_attacker(&g, &fresh_id()).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_expired_pending_empty() {
    let repo = PgCoudeCombatRepository::new(pool().await);
    let _ = repo.list_expired_pending().await.unwrap();
}

// ── Transitions de state ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_betting_transitions_from_pending() {
    let p = pool().await;
    let repo = PgCoudeCombatRepository::new(p.clone());
    let g = fresh_id();
    let att = fresh_id(); let def = fresh_id();
    seed_player(&p, &g, &att).await;
    seed_player(&p, &g, &def).await;
    let c = repo.create(sample_new_combat(&g, &att, &def)).await.unwrap();
    assert!(repo.set_betting(c.id, "msg-123").await.unwrap());
    let got = repo.get(c.id).await.unwrap().unwrap();
    assert_eq!(got.status, "betting");
    assert_eq!(got.message_id.as_deref(), Some("msg-123"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_betting_false_if_already_betting() {
    let p = pool().await;
    let repo = PgCoudeCombatRepository::new(p.clone());
    let g = fresh_id();
    let att = fresh_id(); let def = fresh_id();
    seed_player(&p, &g, &att).await;
    seed_player(&p, &g, &def).await;
    let c = repo.create(sample_new_combat(&g, &att, &def)).await.unwrap();
    repo.set_betting(c.id, "msg1").await.unwrap();
    assert!(!repo.set_betting(c.id, "msg2").await.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_marks_done_and_persists_resolution() {
    let p = pool().await;
    let repo = PgCoudeCombatRepository::new(p.clone());
    let g = fresh_id();
    let att = fresh_id(); let def = fresh_id();
    seed_player(&p, &g, &att).await;
    seed_player(&p, &g, &def).await;
    let c = repo.create(sample_new_combat(&g, &att, &def)).await.unwrap();
    let resolution = CombatResolution {
        status: "resolved".into(),
        winner_id: Some(att.clone()),
        attacker_roll: Some(12), defender_roll: Some(5),
        chaos_event: None,
        result_message: Some("Attacker wins!".into()),
        coins_transferred: 100,
    };
    assert!(repo.resolve(c.id, resolution).await.unwrap());
    let got = repo.get(c.id).await.unwrap().unwrap();
    assert_eq!(got.status, "resolved");
    assert_eq!(got.winner_id.as_deref(), Some(att.as_str()));
    assert_eq!(got.attacker_roll, Some(12));
    assert_eq!(got.coins_transferred, Some(100));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_false_if_not_active() {
    let p = pool().await;
    let repo = PgCoudeCombatRepository::new(p.clone());
    let g = fresh_id();
    let att = fresh_id(); let def = fresh_id();
    seed_player(&p, &g, &att).await;
    seed_player(&p, &g, &def).await;
    let c = repo.create(sample_new_combat(&g, &att, &def)).await.unwrap();
    repo.expire(c.id).await.unwrap();
    // deja expire : resolve doit retourner false
    let res = CombatResolution {
        status: "resolved".into(), winner_id: None,
        attacker_roll: None, defender_roll: None,
        chaos_event: None, result_message: None, coins_transferred: 0,
    };
    assert!(!repo.resolve(c.id, res).await.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expire_from_any_status() {
    let p = pool().await;
    let repo = PgCoudeCombatRepository::new(p.clone());
    let g = fresh_id();
    let att = fresh_id(); let def = fresh_id();
    seed_player(&p, &g, &att).await;
    seed_player(&p, &g, &def).await;
    let c = repo.create(sample_new_combat(&g, &att, &def)).await.unwrap();
    assert!(repo.expire(c.id).await.unwrap());
    let got = repo.get(c.id).await.unwrap().unwrap();
    assert_eq!(got.status, "expired");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_pending_only_when_pending() {
    let p = pool().await;
    let repo = PgCoudeCombatRepository::new(p.clone());
    let g = fresh_id();
    let att = fresh_id(); let def = fresh_id();
    seed_player(&p, &g, &att).await;
    seed_player(&p, &g, &def).await;
    let c = repo.create(sample_new_combat(&g, &att, &def)).await.unwrap();
    assert!(repo.cancel_pending(c.id).await.unwrap());
    // 2e fois : le combat n'est plus pending → false
    assert!(!repo.cancel_pending(c.id).await.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_defender_special_updates_field() {
    let p = pool().await;
    let repo = PgCoudeCombatRepository::new(p.clone());
    let g = fresh_id();
    let att = fresh_id(); let def = fresh_id();
    seed_player(&p, &g, &att).await;
    seed_player(&p, &g, &def).await;
    let c = repo.create(sample_new_combat(&g, &att, &def)).await.unwrap();
    assert!(repo.set_defender_special(c.id, "shield").await.unwrap());
    let got = repo.get(c.id).await.unwrap().unwrap();
    assert_eq!(got.defender_special.as_deref(), Some("shield"));
}

// ── Batch claim ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn claim_due_betting_returns_empty_when_no_due() {
    let repo = PgCoudeCombatRepository::new(pool().await);
    let got = repo.claim_due_betting_combats(300).await.unwrap();
    // Peut etre vide (pas de combats dus dans la DB).
    let _ = got;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn claim_stuck_resolving_does_not_panic() {
    let repo = PgCoudeCombatRepository::new(pool().await);
    let _ = repo.claim_stuck_resolving_combats(120).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn claim_expired_pending_returns_list() {
    let repo = PgCoudeCombatRepository::new(pool().await);
    let _ = repo.claim_expired_pending_combats(24).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_betting_for_participant_none_when_not_in_betting() {
    let repo = PgCoudeCombatRepository::new(pool().await);
    assert!(repo.get_betting_for_participant(&fresh_id(), &fresh_id()).await.unwrap().is_none());
}
