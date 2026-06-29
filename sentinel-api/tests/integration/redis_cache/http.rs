//! Tests d'integration pour RedisCache avec une vraie connexion Redis.

use chrono::Utc;
use sentinel_api::adapters::outbound::redis_cache::RedisCache;
use sentinel_api::ports::outbound::system::cache::CachePort;
use sentinel_core::domain::entities::system::rule::Rule;
use sentinel_core::domain::enums::moderation::flag_type::FlagType;
use uuid::Uuid;

async fn cache() -> RedisCache {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6380".into());
    let client = redis::Client::open(url).unwrap();
    RedisCache::new(client).await.unwrap()
}
fn fresh_guild() -> String {
    format!("cache-{}", Uuid::new_v4().simple())
}

fn sample_rule(g: &str, flag: FlagType) -> Rule {
    let now = Utc::now();
    Rule {
        id: Uuid::new_v4(),
        guild_id: g.into(),
        flag_type: flag,
        weight: 1.5,
        threshold_warn: 0.3,
        threshold_delete: 0.5,
        threshold_mute: 0.7,
        threshold_ban: 0.9,
        enabled: true,
        created_at: now,
        updated_at: now,
    }
}

// ── Rules cache ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rules_miss_when_empty() {
    let cache = cache().await;
    assert!(cache.get_rules(&fresh_guild()).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rules_set_and_get_roundtrip() {
    let cache = cache().await;
    let g = fresh_guild();
    let rules = vec![
        sample_rule(&g, FlagType::Spam),
        sample_rule(&g, FlagType::Insult),
    ];
    cache.set_rules(&g, &rules).await.unwrap();
    let got = cache.get_rules(&g).await.unwrap().unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].weight, 1.5);
    assert!(got.iter().any(|r| r.flag_type == FlagType::Spam));
    assert!(got.iter().any(|r| r.flag_type == FlagType::Insult));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rules_invalidate_removes() {
    let cache = cache().await;
    let g = fresh_guild();
    cache
        .set_rules(&g, &[sample_rule(&g, FlagType::Spam)])
        .await
        .unwrap();
    cache.invalidate_rules(&g).await.unwrap();
    assert!(cache.get_rules(&g).await.unwrap().is_none());
}

// ── Generic JSON cache ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_get_none_when_absent() {
    let cache = cache().await;
    assert!(cache
        .get_json(&format!("nokey-{}", Uuid::new_v4()))
        .await
        .unwrap()
        .is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_set_get_with_ttl() {
    let cache = cache().await;
    let key = format!("test:k-{}", Uuid::new_v4().simple());
    cache.set_json(&key, "{\"a\":1}", 60).await.unwrap();
    let got = cache.get_json(&key).await.unwrap().unwrap();
    assert_eq!(got, "{\"a\":1}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_invalidate() {
    let cache = cache().await;
    let key = format!("test:del-{}", Uuid::new_v4().simple());
    cache.set_json(&key, "val", 60).await.unwrap();
    cache.invalidate(&key).await.unwrap();
    assert!(cache.get_json(&key).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalidate_pattern_wipes_multiple_keys() {
    let cache = cache().await;
    let prefix = format!("testpat-{}-", Uuid::new_v4().simple());
    cache
        .set_json(&format!("{prefix}1"), "a", 60)
        .await
        .unwrap();
    cache
        .set_json(&format!("{prefix}2"), "b", 60)
        .await
        .unwrap();
    cache
        .set_json(&format!("{prefix}3"), "c", 60)
        .await
        .unwrap();
    cache
        .invalidate_pattern(&format!("{prefix}*"))
        .await
        .unwrap();
    assert!(cache
        .get_json(&format!("{prefix}1"))
        .await
        .unwrap()
        .is_none());
    assert!(cache
        .get_json(&format!("{prefix}2"))
        .await
        .unwrap()
        .is_none());
    assert!(cache
        .get_json(&format!("{prefix}3"))
        .await
        .unwrap()
        .is_none());
}

// ── Stats ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stats_track_hits_and_misses() {
    let cache = cache().await;
    let g = fresh_guild();
    // 2 misses (get_rules + get_json)
    let _ = cache.get_rules(&g).await.unwrap();
    let _ = cache
        .get_json(&format!("missingkey-{}", Uuid::new_v4()))
        .await
        .unwrap();
    // 1 hit
    cache
        .set_rules(&g, &[sample_rule(&g, FlagType::Spam)])
        .await
        .unwrap();
    let _ = cache.get_rules(&g).await.unwrap();

    let s = cache.stats();
    assert!(s.hits >= 1);
    assert!(s.misses >= 2);
    assert!(s.hit_rate_percent >= 0.0 && s.hit_rate_percent <= 100.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stats_initial_zero() {
    let cache = cache().await;
    let s = cache.stats();
    // Note : peut etre > 0 si d'autres tests ont touche la meme instance.
    // On verifie juste que la structure est coherente.
    assert_eq!(s.total, s.hits + s.misses);
    if s.total > 0 {
        let expected = (s.hits as f64 / s.total as f64) * 100.0;
        let rounded = (expected * 10.0).round() / 10.0;
        assert!((s.hit_rate_percent - rounded).abs() < 0.2);
    } else {
        assert_eq!(s.hit_rate_percent, 0.0);
    }
}
