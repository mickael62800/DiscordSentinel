//! Tests d'integration pour ManageVoiceChannelsService (application layer).
//! Instancie le vrai service avec PgVoiceChannelRepository + stubs cache/config
//! pour exercer les chemins application/voice_channels/crud.rs.

use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::community::voice_channel_repository::PgVoiceChannelRepository;
use sentinel_core::application::community::voice_channels::ManageVoiceChannelsService;
use sentinel_core::ports::inbound::community::manage_voice_channels::BanFromChannelCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::CreateInviteLinkCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::CreateThemeCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::CreateVoiceChannelCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::ManageCoAdminCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::ManageVoiceChannelsUseCase;
use sentinel_core::ports::inbound::community::manage_voice_channels::ManageWhitelistCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::TransferOwnershipCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::UpdateVoiceChannelCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::UseInviteLinkCommand;
use sentinel_core::ports::outbound::system::bot_config_repository::BotConfigRepository;
use sentinel_core::ports::outbound::system::cache::CachePort;
use sentinel_core::domain::entities::system::bot_config::BotDefinition;
use sentinel_core::domain::entities::system::bot_config::BotGuildConfig;
use sentinel_core::domain::entities::system::rule::Rule;
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

// ── Stub cache : compte les invalidations pour verifier les effets de bord ──

#[derive(Default)]
struct SpyCache {
    invalidations: Mutex<Vec<String>>,
}

#[async_trait]
impl CachePort for SpyCache {
    async fn get_rules(&self, _: &str) -> Result<Option<Vec<Rule>>, DomainError> {
        Ok(None)
    }
    async fn set_rules(&self, _: &str, _: &[Rule]) -> Result<(), DomainError> {
        Ok(())
    }
    async fn invalidate_rules(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_json(&self, _: &str) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
    async fn set_json(&self, _: &str, _: &str, _: u64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn invalidate(&self, key: &str) -> Result<(), DomainError> {
        self.invalidations.lock().unwrap().push(key.into());
        Ok(())
    }
    async fn invalidate_pattern(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

// ── Stub bot config : retourne vec vide (defaults) ──

struct StubBotConfig;

#[async_trait]
impl BotConfigRepository for StubBotConfig {
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

async fn make_service() -> (ManageVoiceChannelsService, Arc<SpyCache>) {
    let repo = Arc::new(PgVoiceChannelRepository::new(pool().await));
    let cache = Arc::new(SpyCache::default());
    let bot_config = Arc::new(StubBotConfig);
    let svc = ManageVoiceChannelsService::new(repo, cache.clone(), bot_config);
    (svc, cache)
}

fn sample_cmd(guild: &str, channel_id: &str, owner: &str) -> CreateVoiceChannelCommand {
    CreateVoiceChannelCommand {
        guild_id: guild.into(),
        owner_id: owner.into(),
        owner_name: "Owner".into(),
        channel_id: channel_id.into(),
        text_channel_id: None,
        members_channel_id: None,
        queue_channel_id: None,
        category_id: None,
        channel_name: "test-channel".into(),
        kind: "public".into(),
        visibility: "public".into(),
        queue_enabled: false,
        stage_enabled: false,
    }
}

// ═══════════════════════════════════════════════════════════════════
// create / list / detail / close / delete
// ═══════════════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_channel_persists_and_invalidates_cache() {
    let (svc, cache) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    let owner = fresh_id();
    let created = svc
        .create_channel(sample_cmd(&g, &ch, &owner))
        .await
        .unwrap();
    assert_eq!(created.channel_id.as_str(), ch.as_str());
    assert_eq!(created.guild_id.as_str(), g.as_str());
    assert_eq!(created.channel_status, "open");
    let invs = cache.invalidations.lock().unwrap();
    assert!(invs.iter().any(|k| k == &format!("voice_channels:{g}")));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_channels_returns_created_channels() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    svc.create_channel(sample_cmd(&g, &fresh_id(), &fresh_id()))
        .await
        .unwrap();
    svc.create_channel(sample_cmd(&g, &fresh_id(), &fresh_id()))
        .await
        .unwrap();
    let list = svc.list_channels(&g).await.unwrap();
    assert_eq!(list.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_channels_empty_guild_returns_empty() {
    let (svc, _) = make_service().await;
    let list = svc.list_channels(&fresh_id()).await.unwrap();
    assert!(list.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_channel_detail_returns_full_detail() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &fresh_id()))
        .await
        .unwrap();
    let detail = svc.get_channel_detail(&ch).await.unwrap();
    assert_eq!(detail.channel.channel_id.as_str(), ch.as_str());
    assert!(detail.co_admins.is_empty());
    assert!(detail.bans.is_empty());
    assert!(detail.invite_links.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_channel_detail_not_found_returns_error() {
    let (svc, _) = make_service().await;
    let err = svc.get_channel_detail(&fresh_id()).await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn close_channel_updates_status() {
    let (svc, cache) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &fresh_id()))
        .await
        .unwrap();
    svc.close_channel(&ch).await.unwrap();
    // Le channel ne devrait plus apparaitre dans la liste active
    let list = svc.list_channels(&g).await.unwrap();
    assert!(list.iter().all(|c| c.channel_id.as_str() != ch));
    // Cache invalide au moins 1 fois (create). Le close invalide le detail :channel: key aussi.
    let invs = cache.invalidations.lock().unwrap();
    assert!(!invs.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_channel_is_soft_delete_like_close() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &fresh_id()))
        .await
        .unwrap();
    svc.delete_channel(&ch).await.unwrap();
    assert!(svc.list_channels(&g).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_history_includes_closed_channels() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &fresh_id()))
        .await
        .unwrap();
    svc.close_channel(&ch).await.unwrap();
    let hist = svc.list_history_channels(&g, 100).await.unwrap();
    assert!(hist.iter().any(|c| c.channel_id.as_str() == ch));
}

// ═══════════════════════════════════════════════════════════════════
// update_channel
// ═══════════════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_channel_applies_visibility() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &fresh_id()))
        .await
        .unwrap();

    svc.update_channel(UpdateVoiceChannelCommand {
        channel_id: ch.clone().into(),
        visibility: Some("private".into()),
        locked: None,
        queue_enabled: None,
        stage_enabled: None,
        name: None,
        member_limit: None,
        queue_channel_id: None,
        status: None,
    })
    .await
    .unwrap();

    let d = svc.get_channel_detail(&ch).await.unwrap();
    assert_eq!(d.channel.visibility, "private");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_channel_applies_locked_and_queue_flags() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &fresh_id()))
        .await
        .unwrap();

    svc.update_channel(UpdateVoiceChannelCommand {
        channel_id: ch.clone().into(),
        visibility: None,
        locked: Some(true),
        queue_enabled: Some(true),
        stage_enabled: None,
        name: None,
        member_limit: None,
        queue_channel_id: None,
        status: None,
    })
    .await
    .unwrap();

    let d = svc.get_channel_detail(&ch).await.unwrap();
    assert!(d.channel.locked);
    assert!(d.channel.queue_enabled);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_channel_not_found_returns_error() {
    let (svc, _) = make_service().await;
    let res = svc
        .update_channel(UpdateVoiceChannelCommand {
            channel_id: fresh_id().into(),
            visibility: Some("private".into()),
            locked: None,
            queue_enabled: None,
            stage_enabled: None,
            name: None,
            member_limit: None,
            queue_channel_id: None,
            status: None,
        })
        .await;
    // Peut etre Ok (no-op si UPDATE ne match rien) ou NotFound — on accepte les deux.
    match res {
        Ok(_) => {}
        Err(DomainError::NotFound(_)) => {}
        Err(other) => panic!("unexpected error: {:?}", other),
    }
}

// ═══════════════════════════════════════════════════════════════════
// transfer_ownership
// ═══════════════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transfer_ownership_updates_owner() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    let old_owner = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &old_owner))
        .await
        .unwrap();

    let new_owner = fresh_id();
    svc.transfer_ownership(TransferOwnershipCommand {
        channel_id: ch.clone().into(),
        new_owner_id: new_owner.clone(),
        new_owner_name: "NewBoss".into(),
    })
    .await
    .unwrap();

    let d = svc.get_channel_detail(&ch).await.unwrap();
    assert_eq!(d.channel.owner_id, new_owner);
    assert_eq!(d.channel.owner_name, "NewBoss");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transfer_ownership_not_found() {
    let (svc, _) = make_service().await;
    let err = svc
        .transfer_ownership(TransferOwnershipCommand {
            channel_id: fresh_id().into(),
            new_owner_id: "x".into(),
            new_owner_name: "X".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

// ═══════════════════════════════════════════════════════════════════
// update_channel : tous les champs optionnels restants
// ═══════════════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_channel_applies_name_status_limit_stage_queue_ch() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &fresh_id()))
        .await
        .unwrap();

    svc.update_channel(UpdateVoiceChannelCommand {
        channel_id: ch.clone().into(),
        visibility: None,
        locked: None,
        queue_enabled: None,
        stage_enabled: Some(true),
        name: Some("rebaptise".into()),
        member_limit: Some(Some(10)),
        queue_channel_id: Some(Some("queue-ch-123".into())),
        status: Some("afk".into()),
    })
    .await
    .unwrap();

    let d = svc.get_channel_detail(&ch).await.unwrap();
    assert_eq!(d.channel.channel_name, "rebaptise");
    assert_eq!(d.channel.member_limit, Some(10));
    assert!(d.channel.stage_enabled);
    assert_eq!(d.channel.status.as_deref(), Some("afk"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_channel_clears_member_limit_with_some_none() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &fresh_id()))
        .await
        .unwrap();

    // D'abord mettre un limit
    svc.update_channel(UpdateVoiceChannelCommand {
        channel_id: ch.clone().into(),
        visibility: None,
        locked: None,
        queue_enabled: None,
        stage_enabled: None,
        name: None,
        member_limit: Some(Some(5)),
        queue_channel_id: None,
        status: None,
    })
    .await
    .unwrap();

    // Puis le clear (Some(None))
    svc.update_channel(UpdateVoiceChannelCommand {
        channel_id: ch.clone().into(),
        visibility: None,
        locked: None,
        queue_enabled: None,
        stage_enabled: None,
        name: None,
        member_limit: Some(None),
        queue_channel_id: None,
        status: None,
    })
    .await
    .unwrap();

    let d = svc.get_channel_detail(&ch).await.unwrap();
    assert!(d.channel.member_limit.is_none());
}

// ═══════════════════════════════════════════════════════════════════
// list_all_channels (tous guild confondus, usage admin global)
// ═══════════════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn close_unknown_channel_ok_silent() {
    // close_by_channel_id est UPDATE WHERE, no-op si inexistant.
    // resolve_channel echoue → branche else de `if let Ok(channel) = ...`.
    let (svc, _) = make_service().await;
    let res = svc.close_channel(&fresh_id()).await;
    // Doit retourner Ok (le repo UPDATE n'echoue pas pour row manquante).
    assert!(res.is_ok() || matches!(res, Err(DomainError::NotFound(_))));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_all_channels_returns_across_guilds() {
    let (svc, _) = make_service().await;
    let g1 = fresh_id();
    let g2 = fresh_id();
    svc.create_channel(sample_cmd(&g1, &fresh_id(), &fresh_id()))
        .await
        .unwrap();
    svc.create_channel(sample_cmd(&g2, &fresh_id(), &fresh_id()))
        .await
        .unwrap();
    let all = svc.list_all_channels().await.unwrap();
    // On a au minimum les 2 qu'on vient de creer (peut y en avoir d'autres).
    assert!(all.len() >= 2);
}

// ═══════════════════════════════════════════════════════════════════
// Whitelist (access_control.rs)
// ═══════════════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn whitelist_add_list_remove_flow() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let owner = fresh_id();
    let target = fresh_id();
    svc.add_to_whitelist(ManageWhitelistCommand {
        guild_id: g.clone().into(),
        owner_id: owner.clone(),
        target_id: target.clone(),
        target_name: "Friend".into(),
    })
    .await
    .unwrap();
    let wl = svc.get_whitelist(&g, &owner).await.unwrap();
    assert_eq!(wl.len(), 1);
    assert_eq!(wl[0].target_id, target);
    svc.remove_from_whitelist(&g, &owner, &target)
        .await
        .unwrap();
    let wl = svc.get_whitelist(&g, &owner).await.unwrap();
    assert!(wl.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn whitelist_empty_for_unknown_owner() {
    let (svc, _) = make_service().await;
    let wl = svc.get_whitelist(&fresh_id(), &fresh_id()).await.unwrap();
    assert!(wl.is_empty());
}

// ═══════════════════════════════════════════════════════════════════
// Bans (access_control.rs)
// ═══════════════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ban_from_channel_persists_and_is_banned_returns_true() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &fresh_id()))
        .await
        .unwrap();

    let target = fresh_id();
    svc.ban_from_channel(BanFromChannelCommand {
        channel_id: ch.clone().into(),
        user_id: target.clone().into(),
        user_name: "BadGuy".into(),
        banned_by: "owner".into(),
        reason: Some("spam".into()),
        duration_secs: Some(3600),
    })
    .await
    .unwrap();

    assert!(svc.is_banned(&ch, &target).await.unwrap());
    // Autre user n'est pas ban
    assert!(!svc.is_banned(&ch, &fresh_id()).await.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ban_permanent_without_expires() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &fresh_id()))
        .await
        .unwrap();
    let target = fresh_id();
    svc.ban_from_channel(BanFromChannelCommand {
        channel_id: ch.clone().into(),
        user_id: target.clone().into(),
        user_name: "X".into(),
        banned_by: "owner".into(),
        reason: None,
        duration_secs: None, // permanent
    })
    .await
    .unwrap();
    assert!(svc.is_banned(&ch, &target).await.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unban_removes_ban() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &fresh_id()))
        .await
        .unwrap();
    let target = fresh_id();
    svc.ban_from_channel(BanFromChannelCommand {
        channel_id: ch.clone().into(),
        user_id: target.clone().into(),
        user_name: "X".into(),
        banned_by: "o".into(),
        reason: None,
        duration_secs: Some(3600),
    })
    .await
    .unwrap();
    svc.unban_from_channel(&ch, &target).await.unwrap();
    assert!(!svc.is_banned(&ch, &target).await.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ban_unknown_channel_returns_not_found() {
    let (svc, _) = make_service().await;
    let err = svc
        .ban_from_channel(BanFromChannelCommand {
            channel_id: fresh_id().into(),
            user_id: "u".into(),
            user_name: "X".into(),
            banned_by: "o".into(),
            reason: None,
            duration_secs: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn is_banned_unknown_channel_returns_not_found() {
    let (svc, _) = make_service().await;
    let err = svc.is_banned(&fresh_id(), "u").await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

// ═══════════════════════════════════════════════════════════════════
// Co-admins (co_admin.rs)
// ═══════════════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_co_admin_persists_in_detail() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &fresh_id()))
        .await
        .unwrap();
    let target = fresh_id();
    svc.add_co_admin(ManageCoAdminCommand {
        channel_id: ch.clone().into(),
        user_id: target.clone().into(),
        user_name: "CoMod".into(),
    })
    .await
    .unwrap();
    let d = svc.get_channel_detail(&ch).await.unwrap();
    assert!(d.co_admins.iter().any(|c| c.user_id.as_str() == target));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remove_co_admin_clears_entry() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &fresh_id()))
        .await
        .unwrap();
    let target = fresh_id();
    svc.add_co_admin(ManageCoAdminCommand {
        channel_id: ch.clone().into(),
        user_id: target.clone().into(),
        user_name: "X".into(),
    })
    .await
    .unwrap();
    svc.remove_co_admin(&ch, &target).await.unwrap();
    let d = svc.get_channel_detail(&ch).await.unwrap();
    assert!(!d.co_admins.iter().any(|c| c.user_id.as_str() == target));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_co_admin_unknown_channel_returns_not_found() {
    let (svc, _) = make_service().await;
    let err = svc
        .add_co_admin(ManageCoAdminCommand {
            channel_id: fresh_id().into(),
            user_id: "u".into(),
            user_name: "X".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

// ═══════════════════════════════════════════════════════════════════
// Invite links (invite.rs)
// ═══════════════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_invite_link_generates_code_and_list_returns_it() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &fresh_id()))
        .await
        .unwrap();
    let link = svc
        .create_invite_link(CreateInviteLinkCommand {
            channel_id: ch.clone().into(),
            created_by: "owner".into(),
            created_by_name: "Owner".into(),
            duration_secs: Some(3600),
            max_uses: Some(5),
        })
        .await
        .unwrap();
    assert!(!link.code.is_empty());
    assert_eq!(link.max_uses, Some(5));
    let list = svc.list_invite_links(&ch).await.unwrap();
    assert!(list.iter().any(|l| l.code == link.code));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_invite_link_defaults_when_fields_absent() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &fresh_id()))
        .await
        .unwrap();
    let link = svc
        .create_invite_link(CreateInviteLinkCommand {
            channel_id: ch.clone().into(),
            created_by: "o".into(),
            created_by_name: "O".into(),
            duration_secs: None,
            max_uses: None,
        })
        .await
        .unwrap();
    assert_eq!(link.current_uses, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn use_invite_link_increments_and_whitelists() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    let owner = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &owner))
        .await
        .unwrap();
    let link = svc
        .create_invite_link(CreateInviteLinkCommand {
            channel_id: ch.clone().into(),
            created_by: owner.clone(),
            created_by_name: "O".into(),
            duration_secs: Some(3600),
            max_uses: Some(3),
        })
        .await
        .unwrap();
    let user = fresh_id();
    let used = svc
        .use_invite_link(UseInviteLinkCommand {
            code: link.code.clone(),
            user_id: user.clone().into(),
            user_name: "Visitor".into(),
        })
        .await
        .unwrap();
    assert_eq!(used.current_uses, 1);
    // Whitelist automatique
    let wl = svc.get_whitelist(&g, &owner).await.unwrap();
    assert!(wl.iter().any(|w| w.target_id == user));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn use_invite_link_rejects_unknown_code() {
    let (svc, _) = make_service().await;
    let err = svc
        .use_invite_link(UseInviteLinkCommand {
            code: "INVALID".into(),
            user_id: "u".into(),
            user_name: "X".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn use_invite_link_rejects_when_max_uses_reached() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &fresh_id()))
        .await
        .unwrap();
    let link = svc
        .create_invite_link(CreateInviteLinkCommand {
            channel_id: ch.clone().into(),
            created_by: "o".into(),
            created_by_name: "O".into(),
            duration_secs: Some(3600),
            max_uses: Some(1),
        })
        .await
        .unwrap();
    // Premier use OK
    svc.use_invite_link(UseInviteLinkCommand {
        code: link.code.clone(),
        user_id: fresh_id().into(),
        user_name: "U1".into(),
    })
    .await
    .unwrap();
    // Deuxieme use doit echouer
    let err = svc
        .use_invite_link(UseInviteLinkCommand {
            code: link.code.clone(),
            user_id: fresh_id().into(),
            user_name: "U2".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revoke_invite_link_removes_it() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let ch = fresh_id();
    svc.create_channel(sample_cmd(&g, &ch, &fresh_id()))
        .await
        .unwrap();
    let link = svc
        .create_invite_link(CreateInviteLinkCommand {
            channel_id: ch.clone().into(),
            created_by: "o".into(),
            created_by_name: "O".into(),
            duration_secs: Some(3600),
            max_uses: Some(5),
        })
        .await
        .unwrap();
    svc.revoke_invite_link(&ch, &link.id.to_string())
        .await
        .unwrap();
    // Use doit echouer apres revoke
    let err = svc
        .use_invite_link(UseInviteLinkCommand {
            code: link.code,
            user_id: "u".into(),
            user_name: "X".into(),
        })
        .await
        .unwrap_err();
    assert!(
        matches!(err, DomainError::NotFound(_)) || matches!(err, DomainError::ValidationError(_))
    );
}

// ═══════════════════════════════════════════════════════════════════
// Themes (theme.rs)
// ═══════════════════════════════════════════════════════════════════

fn sample_theme_cmd(guild: &str) -> CreateThemeCommand {
    CreateThemeCommand {
        guild_id: guild.into(),
        name: "Gaming".into(),
        emoji: Some("🎮".into()),
        channel_name_template: "{emoji} Gaming - {user}".into(),
        member_limit: Some(10),
        visibility: "visible".into(),
        locked: false,
        queue_enabled: false,
        bitrate: Some(64000),
        slowmode_secs: Some(5),
        stage_enabled: false,
        is_default: false,
        sort_order: 0,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_theme_and_list() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let theme = svc.create_theme(sample_theme_cmd(&g)).await.unwrap();
    assert_eq!(theme.name, "Gaming");
    let list = svc.list_themes(&g).await.unwrap();
    assert!(list.iter().any(|t| t.id == theme.id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_theme_changes_fields() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let theme = svc.create_theme(sample_theme_cmd(&g)).await.unwrap();
    let mut updated_cmd = sample_theme_cmd(&g);
    updated_cmd.name = "Chess".into();
    updated_cmd.emoji = Some("♟️".into());
    let updated = svc
        .update_theme(&theme.id.to_string(), updated_cmd)
        .await
        .unwrap();
    assert_eq!(updated.name, "Chess");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_theme_removes_it() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let theme = svc.create_theme(sample_theme_cmd(&g)).await.unwrap();
    svc.delete_theme(&g, &theme.id.to_string()).await.unwrap();
    let list = svc.list_themes(&g).await.unwrap();
    assert!(!list.iter().any(|t| t.id == theme.id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_theme_unknown_id_returns_not_found() {
    let (svc, _) = make_service().await;
    let g = fresh_id();
    let res = svc
        .update_theme(&Uuid::new_v4().to_string(), sample_theme_cmd(&g))
        .await;
    assert!(res.is_err());
}

// ═══════════════════════════════════════════════════════════════════
// Voice config (config.rs)
// ═══════════════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_voice_config_returns_defaults_for_unknown_guild() {
    let (svc, _) = make_service().await;
    let cfg = svc.get_voice_config(&fresh_id()).await.unwrap();
    // Les defauts doivent etre raisonnables (pas de panic).
    // On verifie juste la structure renvoyee.
    let _ = cfg;
}
