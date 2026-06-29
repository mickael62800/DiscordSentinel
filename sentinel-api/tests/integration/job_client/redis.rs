//! Tests d'integration pour JobClient (enqueue jobs via Redis LPUSH).

use redis::AsyncCommands;
use sentinel_api::adapters::outbound::job_client::JobClient;
use serde_json::json;
use uuid::Uuid;

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6380".into())
}

fn client() -> redis::Client {
    redis::Client::open(redis_url()).unwrap()
}

fn fresh_queue() -> String {
    format!("test-queue-{}", Uuid::new_v4().simple())
}

/// Helper : wait la fin du tokio::spawn puis pop la queue.
async fn wait_and_pop(c: &redis::Client, queue: &str) -> Option<String> {
    // Le enqueue fire-and-forget dans un tokio::spawn → on laisse au runtime
    // le temps d'ecrire via LPUSH. 200ms est largement suffisant en local.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let mut conn = c.get_multiplexed_async_connection().await.unwrap();
    conn.rpop::<_, Option<String>>(queue, None).await.unwrap()
}

#[tokio::test]
async fn enqueue_pushes_job_to_redis_queue() {
    let c = client();
    let queue = fresh_queue();
    let jc = JobClient::new(c.clone(), queue.clone());

    jc.enqueue(
        "analyze_message",
        json!({"content": "hello", "guild_id": "g1"}),
    )
    .await;

    let raw = wait_and_pop(&c, &queue)
        .await
        .expect("queue doit contenir 1 job");
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(parsed["type"], "analyze_message");
    assert_eq!(parsed["payload"]["content"], "hello");
    assert_eq!(parsed["payload"]["guild_id"], "g1");
    assert!(parsed["created_at"].is_string());
}

#[tokio::test]
async fn enqueue_multiple_jobs_all_land_in_queue() {
    // enqueue est fire-and-forget (tokio::spawn) → l'ordre exact d'arrivee
    // sur Redis n'est pas garanti. On verifie juste que les 3 jobs sont
    // tous presents.
    let c = client();
    let queue = fresh_queue();
    let jc = JobClient::new(c.clone(), queue.clone());

    jc.enqueue("job_a", json!({"n": 1})).await;
    jc.enqueue("job_b", json!({"n": 2})).await;
    jc.enqueue("job_c", json!({"n": 3})).await;

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let mut conn = c.get_multiplexed_async_connection().await.unwrap();

    let mut found = std::collections::HashSet::new();
    for _ in 0..3 {
        let raw: Option<String> = conn.rpop(&queue, None).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&raw.unwrap()).unwrap();
        found.insert(parsed["type"].as_str().unwrap().to_string());
    }
    assert_eq!(found.len(), 3);
    assert!(found.contains("job_a"));
    assert!(found.contains("job_b"));
    assert!(found.contains("job_c"));
}

#[tokio::test]
async fn enqueue_accepts_complex_nested_payload() {
    let c = client();
    let queue = fresh_queue();
    let jc = JobClient::new(c.clone(), queue.clone());

    let payload = json!({
        "user_id": "u1",
        "flags": {"spam": true, "links": [1, 2, 3]},
        "meta": null,
    });
    jc.enqueue("complex", payload).await;

    let raw = wait_and_pop(&c, &queue).await.expect("job ecrit");
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(parsed["payload"]["flags"]["spam"], true);
    assert_eq!(parsed["payload"]["flags"]["links"][1], 2);
    assert!(parsed["payload"]["meta"].is_null());
}

#[tokio::test]
async fn new_client_is_clonable() {
    // La structure Clone derive — utilise pour partager le client entre handlers.
    let c = client();
    let jc = JobClient::new(c, "some-queue".into());
    let _jc2 = jc.clone();
}

#[tokio::test]
async fn enqueue_serialized_job_has_required_fields() {
    let c = client();
    let queue = fresh_queue();
    let jc = JobClient::new(c.clone(), queue.clone());
    jc.enqueue("test", json!({})).await;

    let raw = wait_and_pop(&c, &queue).await.expect("job");
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    // Les 3 champs du `Job` struct.
    assert!(parsed.as_object().unwrap().contains_key("type"));
    assert!(parsed.as_object().unwrap().contains_key("payload"));
    assert!(parsed.as_object().unwrap().contains_key("created_at"));
}
