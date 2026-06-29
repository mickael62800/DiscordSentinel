//! Tests d'integration REELS pour le systeme de tickets (avec PostgreSQL).

use sqlx::PgPool;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url).await.unwrap()
}

fn ugid() -> String {
    format!(
        "{}",
        uuid::Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    )
}

async fn create_ticket(
    pool: &PgPool,
    gid: &str,
    author: &str,
    title: &str,
    category: &str,
) -> uuid::Uuid {
    sqlx::query_as::<_, (uuid::Uuid,)>(
        r#"INSERT INTO tickets (id, title, author_id, author_name, server, category, ticket_type)
           VALUES (gen_random_uuid(), $1, $2, 'User', $3, $4, $4) RETURNING id"#,
    )
    .bind(title)
    .bind(author)
    .bind(gid)
    .bind(category)
    .fetch_one(pool)
    .await
    .unwrap()
    .0
}

#[tokio::test]
async fn ticket_create_defaults() {
    let p = pool().await;
    let id = create_ticket(&p, &ugid(), "444", "Mon probleme", "support").await;
    let row =
        sqlx::query_as::<_, (String, String)>("SELECT status, priority FROM tickets WHERE id = $1")
            .bind(id)
            .fetch_one(&p)
            .await
            .unwrap();
    assert_eq!(row.0, "open");
    assert_eq!(row.1, "medium");
}

#[tokio::test]
async fn ticket_close_sets_status() {
    let p = pool().await;
    let gid = ugid();
    let id = create_ticket(&p, &gid, "444", "A fermer", "support").await;
    sqlx::query("UPDATE tickets SET status = 'closed', resolved_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&p)
        .await
        .unwrap();
    let status = sqlx::query_as::<_, (String,)>("SELECT status FROM tickets WHERE id = $1")
        .bind(id)
        .fetch_one(&p)
        .await
        .unwrap()
        .0;
    assert_eq!(status, "closed");
}

#[tokio::test]
async fn ticket_messages_cascade_delete() {
    let p = pool().await;
    let id = create_ticket(&p, &ugid(), "444", "Ticket msg", "support").await;
    for _ in 0..3 {
        sqlx::query("INSERT INTO ticket_messages (id, ticket_id, author_name, content) VALUES (gen_random_uuid(), $1, 'Staff', 'Reponse')")
            .bind(id).execute(&p).await.unwrap();
    }
    let msgs =
        sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM ticket_messages WHERE ticket_id = $1")
            .bind(id)
            .fetch_one(&p)
            .await
            .unwrap()
            .0;
    assert_eq!(msgs, 3);

    // Supprimer le ticket → messages supprimes en cascade
    sqlx::query("DELETE FROM tickets WHERE id = $1")
        .bind(id)
        .execute(&p)
        .await
        .unwrap();
    let after =
        sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM ticket_messages WHERE ticket_id = $1")
            .bind(id)
            .fetch_one(&p)
            .await
            .unwrap()
            .0;
    assert_eq!(after, 0);
}

#[tokio::test]
async fn ticket_filter_by_status() {
    let p = pool().await;
    let gid = ugid();
    create_ticket(&p, &gid, "444", "Open1", "support").await;
    create_ticket(&p, &gid, "444", "Open2", "support").await;
    let id3 = create_ticket(&p, &gid, "444", "Closed", "support").await;
    sqlx::query("UPDATE tickets SET status = 'closed' WHERE id = $1")
        .bind(id3)
        .execute(&p)
        .await
        .unwrap();

    let open = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM tickets WHERE server = $1 AND status = 'open'",
    )
    .bind(&gid)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;
    assert_eq!(open, 2);
}

#[tokio::test]
async fn ticket_assign_to_staff() {
    let p = pool().await;
    let id = create_ticket(&p, &ugid(), "444", "Assign", "support").await;
    sqlx::query("UPDATE tickets SET assigned_to = '333', status = 'in_progress' WHERE id = $1")
        .bind(id)
        .execute(&p)
        .await
        .unwrap();
    let row = sqlx::query_as::<_, (Option<String>, String)>(
        "SELECT assigned_to, status FROM tickets WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(row.0.unwrap(), "333");
    assert_eq!(row.1, "in_progress");
}

#[tokio::test]
async fn ticket_satisfaction_rating() {
    let p = pool().await;
    let id = create_ticket(&p, &ugid(), "444", "Rate", "support").await;
    sqlx::query("UPDATE tickets SET satisfaction_rating = 5, status = 'closed' WHERE id = $1")
        .bind(id)
        .execute(&p)
        .await
        .unwrap();
    let rating = sqlx::query_as::<_, (Option<i32>,)>(
        "SELECT satisfaction_rating FROM tickets WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;
    assert_eq!(rating, Some(5));
}

#[tokio::test]
async fn ticket_first_response_tracked() {
    let p = pool().await;
    let id = create_ticket(&p, &ugid(), "444", "SLA", "support").await;
    sqlx::query("UPDATE tickets SET first_response_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&p)
        .await
        .unwrap();
    let has_response = sqlx::query_as::<_, (bool,)>(
        "SELECT first_response_at IS NOT NULL FROM tickets WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;
    assert!(has_response);
}
