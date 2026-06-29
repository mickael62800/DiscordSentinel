//! Tests d'integration postgres pour PgVoiceChannelRepository.
//! Repo massif (792L) — couvre channels + co-admins + whitelist + bans + invites + themes.

use chrono::Duration;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::community::voice_channel_repository::PgVoiceChannelRepository;
use sentinel_api::ports::outbound::community::voice_channel_repository::VoiceChannelRepository;
use sentinel_core::domain::entities::community::voice_channel::VoiceChannel;
use sentinel_core::domain::entities::community::voice_channel::VoiceChannelBan;
use sentinel_core::domain::entities::community::voice_channel::VoiceChannelCoAdmin;
use sentinel_core::domain::entities::community::voice_channel::VoiceChannelInviteLink;
use sentinel_core::domain::entities::community::voice_channel::VoiceChannelTheme;
use sentinel_core::domain::entities::community::voice_channel::VoiceChannelWhitelistEntry;
use sentinel_core::domain::enums::community::voice_channel_kind::VoiceChannelKind;

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

fn sample_channel(guild: &str, channel_id: &str, owner: &str) -> VoiceChannel {
    VoiceChannel {
        id: Uuid::new_v4(),
        guild_id: guild.into(),
        owner_id: owner.into(),
        owner_name: "Owner".into(),
        channel_id: channel_id.into(),
        text_channel_id: None,
        members_channel_id: None,
        queue_channel_id: None,
        category_id: None,
        channel_name: "🎮-test".into(),
        kind: VoiceChannelKind::Public,
        visibility: "public".into(),
        queue_enabled: false,
        locked: false,
        stage_enabled: false,
        member_limit: None,
        status: None,
        channel_status: "open".into(),
        closed_at: None,
        created_at: Utc::now(),
    }
}

// ── Channels ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_and_find_by_channel_id() {
    let repo = PgVoiceChannelRepository::new(pool().await);
    let g = fresh_id();
    let ch = fresh_id();
    let vc = sample_channel(&g, &ch, &fresh_id());
    repo.save(&vc).await.unwrap();
    let got = repo.find_by_channel_id(&ch).await.unwrap().unwrap();
    assert_eq!(got.channel_id, ch);
    assert_eq!(got.channel_status, "open");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_id_absent_returns_none() {
    let repo = PgVoiceChannelRepository::new(pool().await);
    assert!(repo.find_by_id(Uuid::new_v4()).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_all_by_guild_scope() {
    let repo = PgVoiceChannelRepository::new(pool().await);
    let g = fresh_id();
    repo.save(&sample_channel(&g, &fresh_id(), &fresh_id()))
        .await
        .unwrap();
    repo.save(&sample_channel(&g, &fresh_id(), &fresh_id()))
        .await
        .unwrap();
    // Autre guild
    repo.save(&sample_channel(&fresh_id(), &fresh_id(), &fresh_id()))
        .await
        .unwrap();
    let got = repo.find_all_by_guild(&g).await.unwrap();
    assert_eq!(got.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn close_marks_channel_closed() {
    let repo = PgVoiceChannelRepository::new(pool().await);
    let vc = sample_channel(&fresh_id(), &fresh_id(), &fresh_id());
    repo.save(&vc).await.unwrap();
    repo.close(vc.id).await.unwrap();
    let got = repo.find_by_id(vc.id).await.unwrap().unwrap();
    assert_eq!(got.channel_status, "closed");
    assert!(got.closed_at.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn close_by_channel_id_works() {
    // find_by_channel_id filtre `channel_status = 'open'` donc apres
    // close, on verifie via find_by_id (pas de filtre status).
    let repo = PgVoiceChannelRepository::new(pool().await);
    let ch = fresh_id();
    let vc = sample_channel(&fresh_id(), &ch, &fresh_id());
    repo.save(&vc).await.unwrap();
    repo.close_by_channel_id(&ch).await.unwrap();
    assert!(repo.find_by_channel_id(&ch).await.unwrap().is_none()); // filtree
    let got = repo.find_by_id(vc.id).await.unwrap().unwrap();
    assert_eq!(got.channel_status, "closed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_closed_by_guild_scope_and_limit() {
    let repo = PgVoiceChannelRepository::new(pool().await);
    let g = fresh_id();
    for _ in 0..3 {
        let vc = sample_channel(&g, &fresh_id(), &fresh_id());
        repo.save(&vc).await.unwrap();
        repo.close(vc.id).await.unwrap();
    }
    let got = repo.find_closed_by_guild(&g, 10).await.unwrap();
    assert_eq!(got.len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_is_soft_delete_aliased_to_close() {
    // Par design : delete() = close() (soft-delete pour preserver l'historique).
    let repo = PgVoiceChannelRepository::new(pool().await);
    let vc = sample_channel(&fresh_id(), &fresh_id(), &fresh_id());
    repo.save(&vc).await.unwrap();
    repo.delete(vc.id).await.unwrap();
    let got = repo.find_by_id(vc.id).await.unwrap().unwrap();
    assert_eq!(got.channel_status, "closed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_multiple_fields() {
    let repo = PgVoiceChannelRepository::new(pool().await);
    let vc = sample_channel(&fresh_id(), &fresh_id(), &fresh_id());
    repo.save(&vc).await.unwrap();
    repo.update_visibility(vc.id, "private").await.unwrap();
    repo.update_locked(vc.id, true).await.unwrap();
    repo.update_queue_enabled(vc.id, true).await.unwrap();
    repo.update_name(vc.id, "new-name").await.unwrap();
    repo.update_status(vc.id, Some("brb")).await.unwrap();
    repo.update_member_limit(vc.id, Some(10)).await.unwrap();
    repo.update_owner(vc.id, "new-owner", "NewName")
        .await
        .unwrap();
    repo.update_queue_channel(vc.id, Some("q1")).await.unwrap();
    repo.update_stage(vc.id, true).await.unwrap();

    let got = repo.find_by_id(vc.id).await.unwrap().unwrap();
    assert_eq!(got.visibility, "private");
    assert!(got.locked);
    assert!(got.queue_enabled);
    assert_eq!(got.channel_name, "new-name");
    assert_eq!(got.status.as_deref(), Some("brb"));
    assert_eq!(got.member_limit, Some(10));
    assert_eq!(got.owner_id, "new-owner");
    assert_eq!(got.queue_channel_id.as_deref(), Some("q1"));
    assert!(got.stage_enabled);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_status_none_clears() {
    let repo = PgVoiceChannelRepository::new(pool().await);
    let vc = sample_channel(&fresh_id(), &fresh_id(), &fresh_id());
    repo.save(&vc).await.unwrap();
    repo.update_status(vc.id, Some("afk")).await.unwrap();
    repo.update_status(vc.id, None).await.unwrap();
    let got = repo.find_by_id(vc.id).await.unwrap().unwrap();
    assert!(got.status.is_none());
}

// ── Co-admins ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_find_and_remove_co_admin() {
    let repo = PgVoiceChannelRepository::new(pool().await);
    let vc = sample_channel(&fresh_id(), &fresh_id(), &fresh_id());
    repo.save(&vc).await.unwrap();
    let co = VoiceChannelCoAdmin {
        id: Uuid::new_v4(),
        voice_channel_id: vc.id,
        user_id: "user1".into(),
        user_name: "Alice".into(),
        granted_at: Utc::now(),
    };
    repo.add_co_admin(&co).await.unwrap();
    assert_eq!(repo.find_co_admins(vc.id).await.unwrap().len(), 1);
    repo.remove_co_admin(vc.id, "user1").await.unwrap();
    assert!(repo.find_co_admins(vc.id).await.unwrap().is_empty());
}

// ── Whitelist ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn whitelist_add_find_remove() {
    let repo = PgVoiceChannelRepository::new(pool().await);
    let g = fresh_id();
    let owner = fresh_id();
    let entry = VoiceChannelWhitelistEntry {
        id: Uuid::new_v4(),
        guild_id: g.clone().into(),
        owner_id: owner.clone(),
        target_id: "t1".into(),
        target_name: "Target".into(),
        created_at: Utc::now(),
    };
    repo.add_to_whitelist(&entry).await.unwrap();
    assert_eq!(repo.find_whitelist(&g, &owner).await.unwrap().len(), 1);
    repo.remove_from_whitelist(&g, &owner, "t1").await.unwrap();
    assert!(repo.find_whitelist(&g, &owner).await.unwrap().is_empty());
}

// ── Bans ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ban_save_find_and_remove() {
    let repo = PgVoiceChannelRepository::new(pool().await);
    let vc = sample_channel(&fresh_id(), &fresh_id(), &fresh_id());
    repo.save(&vc).await.unwrap();
    let ban = VoiceChannelBan {
        id: Uuid::new_v4(),
        voice_channel_id: vc.id,
        user_id: "trouble".into(),
        user_name: "Trouble".into(),
        banned_by: "mod".into(),
        reason: Some("spam".into()),
        expires_at: Some(Utc::now() + Duration::hours(1)),
        created_at: Utc::now(),
    };
    repo.save_ban(&ban).await.unwrap();
    let found = repo
        .find_active_ban(vc.id, "trouble")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.user_id, "trouble");
    let all = repo.find_bans(vc.id).await.unwrap();
    assert_eq!(all.len(), 1);

    repo.remove_ban(vc.id, "trouble").await.unwrap();
    assert!(repo
        .find_active_ban(vc.id, "trouble")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_active_ban_excludes_expired() {
    let repo = PgVoiceChannelRepository::new(pool().await);
    let vc = sample_channel(&fresh_id(), &fresh_id(), &fresh_id());
    repo.save(&vc).await.unwrap();
    let expired = VoiceChannelBan {
        id: Uuid::new_v4(),
        voice_channel_id: vc.id,
        user_id: "u1".into(),
        user_name: "U".into(),
        banned_by: "mod".into(),
        reason: None,
        expires_at: Some(Utc::now() - Duration::hours(1)),
        created_at: Utc::now() - Duration::hours(2),
    };
    repo.save_ban(&expired).await.unwrap();
    assert!(repo.find_active_ban(vc.id, "u1").await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cleanup_expired_bans_does_not_panic() {
    let repo = PgVoiceChannelRepository::new(pool().await);
    let _ = repo.cleanup_expired_bans().await.unwrap();
}

// ── Invite Links ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invite_save_find_increment_revoke() {
    let repo = PgVoiceChannelRepository::new(pool().await);
    let vc = sample_channel(&fresh_id(), &fresh_id(), &fresh_id());
    repo.save(&vc).await.unwrap();
    let code = format!("code-{}", Uuid::new_v4().simple());
    let link = VoiceChannelInviteLink {
        id: Uuid::new_v4(),
        voice_channel_id: vc.id,
        guild_id: vc.guild_id.clone(),
        channel_id: vc.channel_id.clone(),
        created_by: "u1".into(),
        created_by_name: "Alice".into(),
        code: code.clone(),
        max_uses: Some(5),
        current_uses: 0,
        expires_at: Utc::now() + Duration::hours(24),
        revoked: false,
        created_at: Utc::now(),
    };
    repo.save_invite_link(&link).await.unwrap();

    let by_code = repo.find_invite_by_code(&code).await.unwrap().unwrap();
    assert_eq!(by_code.code, code);

    let lst = repo.find_invite_links(vc.id).await.unwrap();
    assert_eq!(lst.len(), 1);

    assert!(repo.increment_invite_uses(link.id).await.unwrap());
    repo.revoke_invite_link(link.id).await.unwrap();
    let after = repo.find_invite_by_code(&code).await.unwrap().unwrap();
    assert!(after.revoked);
}

// ── Themes ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn theme_save_find_update_delete() {
    let repo = PgVoiceChannelRepository::new(pool().await);
    let g = fresh_id();
    let theme = VoiceChannelTheme {
        id: Uuid::new_v4(),
        guild_id: g.clone().into(),
        name: "Gaming".into(),
        emoji: Some("game".into()),
        channel_name_template: "🎮-{user}".into(),
        member_limit: Some(5),
        visibility: "public".into(),
        locked: false,
        queue_enabled: false,
        bitrate: Some(64000),
        slowmode_secs: None,
        stage_enabled: false,
        is_default: false,
        sort_order: 0,
        created_at: Utc::now(),
    };
    repo.save_theme(&theme).await.unwrap();
    let found = repo.find_theme(theme.id).await.unwrap().unwrap();
    assert_eq!(found.name, "Gaming");

    let by_guild = repo.find_themes(&g).await.unwrap();
    assert_eq!(by_guild.len(), 1);

    let mut upd = theme.clone();
    upd.name = "GamingX".into();
    repo.update_theme(&upd).await.unwrap();
    assert_eq!(
        repo.find_theme(theme.id).await.unwrap().unwrap().name,
        "GamingX"
    );

    repo.delete_theme(theme.id).await.unwrap();
    assert!(repo.find_theme(theme.id).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clear_default_themes_unsets_is_default() {
    let repo = PgVoiceChannelRepository::new(pool().await);
    let g = fresh_id();
    let mut t = VoiceChannelTheme {
        id: Uuid::new_v4(),
        guild_id: g.clone().into(),
        name: "Default".into(),
        emoji: None,
        channel_name_template: "{user}".into(),
        member_limit: None,
        visibility: "public".into(),
        locked: false,
        queue_enabled: false,
        bitrate: None,
        slowmode_secs: None,
        stage_enabled: false,
        is_default: true,
        sort_order: 0,
        created_at: Utc::now(),
    };
    repo.save_theme(&t).await.unwrap();
    repo.clear_default_themes(&g).await.unwrap();
    t = repo.find_theme(t.id).await.unwrap().unwrap();
    assert!(!t.is_default);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_all_returns_some_rows() {
    let repo = PgVoiceChannelRepository::new(pool().await);
    // Juste verifier que la methode tourne sans erreur.
    let _ = repo.find_all().await.unwrap();
}
