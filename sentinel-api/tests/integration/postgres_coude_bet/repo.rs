//! Tests d'integration postgres pour PgBetRepository.
//! Instantie une vraie chaine ManageWalletService (avec stub taunts) pour
//! tester l'atomicite des mutations wallet + coude_bets.

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::casino::wallet_repository::PgWalletRepository;
use sentinel_api::adapters::outbound::postgres::coude::bet_repository::PgBetRepository;
use sentinel_api::application::casino::manage_wallet_service::ManageWalletService;
use sentinel_api::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase;
use sentinel_api::ports::outbound::coude::bet_repository::BetRepository;
use sentinel_core::domain::entities::coude::bet::calculate_bet_resolution;
use sentinel_core::domain::entities::coude::bet::NewCoudeBet;
use sentinel_core::domain::entities::coude::taunt::TauntEvent;
use sentinel_core::domain::entities::coude::taunt::TauntsConfig;
use sentinel_core::domain::errors::DomainError;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    )
}

/// Stub minimal : pas de taunts, jamais opt-out, config basique.
struct StubTauntsUc;
#[async_trait]
impl ManageCoudeTauntsUseCase for StubTauntsUc {
    async fn on_player_won(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_player_lost(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_player_drew(&self, _: &str, _: &str) -> Result<(), DomainError> {
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
            enabled: false,
            rename_enabled: false,
            messages_enabled: false,
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

fn make_repo(p: PgPool) -> PgBetRepository {
    let wallet_repo = Arc::new(PgWalletRepository::new(p.clone()));
    let wallet_uc = Arc::new(ManageWalletService::new(
        wallet_repo,
        Arc::new(StubTauntsUc),
    ));
    PgBetRepository::new(p, wallet_uc)
}

async fn seed_wallet(p: &PgPool, g: &str, u: &str, coins: i64) {
    sqlx::query(
        "INSERT INTO user_wallets (guild_id, user_id, username, coins) VALUES ($1, $2, 'T', $3) \
                 ON CONFLICT (guild_id, user_id) DO UPDATE SET coins = EXCLUDED.coins",
    )
    .bind(g)
    .bind(u)
    .bind(coins)
    .execute(p)
    .await
    .unwrap();
}

async fn seed_combat(p: &PgPool, g: &str, att: &str, def: &str, status: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO coude_combats (id, guild_id, channel_id, attacker_id, attacker_name, \
          defender_id, defender_name, mise, status, created_at) \
         VALUES ($1, $2, 'ch', $3, 'Att', $4, 'Def', 100, $5, NOW())",
    )
    .bind(id)
    .bind(g)
    .bind(att)
    .bind(def)
    .bind(status)
    .execute(p)
    .await
    .unwrap();
    id
}

// ── list_for_combat ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_for_combat_empty() {
    let repo = make_repo(pool().await);
    assert!(repo
        .list_for_combat(Uuid::new_v4())
        .await
        .unwrap()
        .is_empty());
}

// ── place ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn place_debits_wallet_and_inserts_bet() {
    let p = pool().await;
    let repo = make_repo(p.clone());
    let g = fresh_id();
    let bettor = fresh_id();
    let att = fresh_id();
    let def = fresh_id();
    seed_wallet(&p, &g, &bettor, 1000).await;
    let combat_id = seed_combat(&p, &g, &att, &def, "betting").await;

    let taunts = repo
        .place(NewCoudeBet {
            guild_id: g.clone().into(),
            combat_id,
            bettor_id: bettor.clone(),
            bettor_name: "Bettor".into(),
            backed_id: att.clone(),
            amount: 300,
        })
        .await
        .unwrap();
    // StubTauntsUc -> pas de taunts
    assert!(taunts.is_empty());

    let bets = repo.list_for_combat(combat_id).await.unwrap();
    assert_eq!(bets.len(), 1);
    assert_eq!(bets[0].amount, 300);

    // Wallet debite : 1000 - 300 = 700
    let (coins,): (i64,) =
        sqlx::query_as("SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2")
            .bind(&g)
            .bind(&bettor)
            .fetch_one(&p)
            .await
            .unwrap();
    assert_eq!(coins, 700);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn place_fails_when_insufficient_balance() {
    let p = pool().await;
    let repo = make_repo(p.clone());
    let g = fresh_id();
    let bettor = fresh_id();
    let att = fresh_id();
    let def = fresh_id();
    seed_wallet(&p, &g, &bettor, 50).await;
    let combat_id = seed_combat(&p, &g, &att, &def, "betting").await;

    let err = repo
        .place(NewCoudeBet {
            guild_id: g.clone().into(),
            combat_id,
            bettor_id: bettor.clone(),
            bettor_name: "B".into(),
            backed_id: att,
            amount: 500,
        })
        .await;
    assert!(err.is_err());

    // Pas de pari insere, solde inchange.
    assert!(repo.list_for_combat(combat_id).await.unwrap().is_empty());
    let (coins,): (i64,) =
        sqlx::query_as("SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2")
            .bind(&g)
            .bind(&bettor)
            .fetch_one(&p)
            .await
            .unwrap();
    assert_eq!(coins, 50);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn place_multiple_bets_on_same_combat() {
    let p = pool().await;
    let repo = make_repo(p.clone());
    let g = fresh_id();
    let att = fresh_id();
    let def = fresh_id();
    let combat_id = seed_combat(&p, &g, &att, &def, "betting").await;
    let b1 = fresh_id();
    let b2 = fresh_id();
    seed_wallet(&p, &g, &b1, 500).await;
    seed_wallet(&p, &g, &b2, 500).await;

    repo.place(NewCoudeBet {
        guild_id: g.clone().into(),
        combat_id,
        bettor_id: b1,
        bettor_name: "B1".into(),
        backed_id: att.clone(),
        amount: 100,
    })
    .await
    .unwrap();
    repo.place(NewCoudeBet {
        guild_id: g.clone().into(),
        combat_id,
        bettor_id: b2,
        bettor_name: "B2".into(),
        backed_id: def,
        amount: 200,
    })
    .await
    .unwrap();

    let bets = repo.list_for_combat(combat_id).await.unwrap();
    assert_eq!(bets.len(), 2);
    let total: i64 = bets.iter().map(|b| b.amount).sum();
    assert_eq!(total, 300);
}

// ── refund_unresolved ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refund_unresolved_empty_returns_zero() {
    let repo = make_repo(pool().await);
    let summary = repo.refund_unresolved("g", Uuid::new_v4()).await.unwrap();
    assert_eq!(summary.refunded_count, 0);
    assert_eq!(summary.refunded_total, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refund_unresolved_credits_back_bettors() {
    let p = pool().await;
    let repo = make_repo(p.clone());
    let g = fresh_id();
    let att = fresh_id();
    let def = fresh_id();
    let combat_id = seed_combat(&p, &g, &att, &def, "betting").await;
    let b1 = fresh_id();
    seed_wallet(&p, &g, &b1, 1000).await;

    repo.place(NewCoudeBet {
        guild_id: g.clone().into(),
        combat_id,
        bettor_id: b1.clone(),
        bettor_name: "B".into(),
        backed_id: att,
        amount: 300,
    })
    .await
    .unwrap();
    // 1000 -> 700 apres place

    let summary = repo.refund_unresolved(&g, combat_id).await.unwrap();
    assert_eq!(summary.refunded_count, 1);
    assert_eq!(summary.refunded_total, 300);

    // Wallet retabli : 700 + 300 = 1000.
    let (coins,): (i64,) =
        sqlx::query_as("SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2")
            .bind(&g)
            .bind(&b1)
            .fetch_one(&p)
            .await
            .unwrap();
    assert_eq!(coins, 1000);
}

// ── apply_resolution ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_resolution_winner_gets_payout_draw_refunds() {
    let p = pool().await;
    let repo = make_repo(p.clone());
    let g = fresh_id();
    let att = fresh_id();
    let def = fresh_id();
    let combat_id = seed_combat(&p, &g, &att, &def, "betting").await;

    // 2 bettors : b1 backe att 200, b2 backe def 100
    let b1 = fresh_id();
    let b2 = fresh_id();
    seed_wallet(&p, &g, &b1, 1000).await;
    seed_wallet(&p, &g, &b2, 1000).await;
    seed_wallet(&p, &g, &att, 0).await;
    seed_wallet(&p, &g, &def, 0).await;
    repo.place(NewCoudeBet {
        guild_id: g.clone().into(),
        combat_id,
        bettor_id: b1.clone(),
        bettor_name: "B1".into(),
        backed_id: att.clone(),
        amount: 200,
    })
    .await
    .unwrap();
    repo.place(NewCoudeBet {
        guild_id: g.clone().into(),
        combat_id,
        bettor_id: b2.clone(),
        bettor_name: "B2".into(),
        backed_id: def.clone(),
        amount: 100,
    })
    .await
    .unwrap();

    let bets = repo.list_for_combat(combat_id).await.unwrap();
    let plan = calculate_bet_resolution(&bets, Some(&att), &att, &def);
    repo.apply_resolution(&g, plan).await.unwrap();

    // Apres : b1 (gagnant) doit avoir son payout, b2 (perdant) rien retrocede.
    let (b1_coins,): (i64,) =
        sqlx::query_as("SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2")
            .bind(&g)
            .bind(&b1)
            .fetch_one(&p)
            .await
            .unwrap();
    let (b2_coins,): (i64,) =
        sqlx::query_as("SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2")
            .bind(&g)
            .bind(&b2)
            .fetch_one(&p)
            .await
            .unwrap();
    // b1 : 1000 - 200 + payout. payout > 0 si quelqu'un a backe le perdant.
    assert!(
        b1_coins > 800,
        "b1 devrait recuperer + payout, got {b1_coins}"
    );
    // b2 : 1000 - 100 = 900 (pas de remboursement)
    assert_eq!(b2_coins, 900);

    // Les paris sont marques resolus.
    let bets = repo.list_for_combat(combat_id).await.unwrap();
    for b in &bets {
        assert!(b.won.is_some(), "bet {} should be resolved", b.id);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_resolution_draw_refunds_everyone() {
    let p = pool().await;
    let repo = make_repo(p.clone());
    let g = fresh_id();
    let att = fresh_id();
    let def = fresh_id();
    let combat_id = seed_combat(&p, &g, &att, &def, "betting").await;
    let b1 = fresh_id();
    seed_wallet(&p, &g, &b1, 1000).await;
    seed_wallet(&p, &g, &att, 0).await;
    seed_wallet(&p, &g, &def, 0).await;
    repo.place(NewCoudeBet {
        guild_id: g.clone().into(),
        combat_id,
        bettor_id: b1.clone(),
        bettor_name: "B1".into(),
        backed_id: att.clone(),
        amount: 300,
    })
    .await
    .unwrap();

    let bets = repo.list_for_combat(combat_id).await.unwrap();
    // Egalite : winner_id = None
    let plan = calculate_bet_resolution(&bets, None, &att, &def);
    repo.apply_resolution(&g, plan).await.unwrap();

    // Refund integral : solde b1 = 1000.
    let (coins,): (i64,) =
        sqlx::query_as("SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2")
            .bind(&g)
            .bind(&b1)
            .fetch_one(&p)
            .await
            .unwrap();
    assert_eq!(coins, 1000);
}
