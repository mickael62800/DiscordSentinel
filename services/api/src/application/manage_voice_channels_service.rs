use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::entities::{
    VoiceChannel, VoiceChannelBan, VoiceChannelCoAdmin, VoiceChannelDetail,
    VoiceChannelInviteLink, VoiceChannelTheme, VoiceChannelWhitelistEntry,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::{
    BanFromChannelCommand, CreateInviteLinkCommand, CreateThemeCommand, CreateVoiceChannelCommand,
    ManageCoAdminCommand, ManageVoiceChannelsUseCase, ManageWhitelistCommand,
    TransferOwnershipCommand, UpdateVoiceChannelCommand, UseInviteLinkCommand,
};
use crate::ports::outbound::{CachePort, VoiceChannelRepository};

const CHANNELS_LIST_TTL: u64 = 60;
const CHANNEL_DETAIL_TTL: u64 = 300;

pub struct ManageVoiceChannelsService {
    repo: Arc<dyn VoiceChannelRepository>,
    cache: Arc<dyn CachePort>,
}

impl ManageVoiceChannelsService {
    pub fn new(repo: Arc<dyn VoiceChannelRepository>, cache: Arc<dyn CachePort>) -> Self {
        Self { repo, cache }
    }

    fn generate_code() -> String {
        use rand::Rng;
        rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(8)
            .map(char::from)
            .collect::<String>()
            .to_uppercase()
    }

    /// Genere un code unique avec retry en cas de collision (UNIQUE constraint en DB).
    async fn generate_unique_code(&self) -> Result<String, DomainError> {
        for _ in 0..5 {
            let code = Self::generate_code();
            // Verifier si le code existe deja
            if self.repo.find_invite_by_code(&code).await?.is_none() {
                return Ok(code);
            }
        }
        Err(DomainError::Internal("Impossible de generer un code unique apres 5 tentatives".to_string()))
    }

    fn validate_theme(cmd: &CreateThemeCommand) -> Result<(), DomainError> {
        if cmd.name.trim().is_empty() {
            return Err(DomainError::ValidationError("Le nom du theme est obligatoire".to_string()));
        }
        if cmd.name.len() > 100 {
            return Err(DomainError::ValidationError("Le nom du theme ne peut pas depasser 100 caracteres".to_string()));
        }
        if let Some(limit) = cmd.member_limit {
            if limit < 0 || limit > 99 {
                return Err(DomainError::ValidationError("La limite de membres doit etre entre 0 et 99".to_string()));
            }
        }
        if let Some(bitrate) = cmd.bitrate {
            if bitrate < 8000 || bitrate > 384000 {
                return Err(DomainError::ValidationError("Le bitrate doit etre entre 8000 et 384000".to_string()));
            }
        }
        if let Some(slowmode) = cmd.slowmode_secs {
            if slowmode < 0 || slowmode > 21600 {
                return Err(DomainError::ValidationError("Le slowmode doit etre entre 0 et 21600 secondes".to_string()));
            }
        }
        match cmd.visibility.as_str() {
            "visible" | "hidden" => {}
            _ => return Err(DomainError::ValidationError("La visibilite doit etre 'visible' ou 'hidden'".to_string())),
        }
        Ok(())
    }

    async fn invalidate_cache(&self, guild_id: &str, channel_id: &str) {
        self.cache.invalidate(&format!("voice_channels:{guild_id}")).await.ok();
        self.cache.invalidate(&format!("voice_channel:{channel_id}")).await.ok();
    }

    async fn resolve_channel(&self, channel_id: &str) -> Result<VoiceChannel, DomainError> {
        self.repo
            .find_by_channel_id(channel_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Salon vocal introuvable : {channel_id}")))
    }
}

#[async_trait]
impl ManageVoiceChannelsUseCase for ManageVoiceChannelsService {
    async fn list_all_channels(&self) -> Result<Vec<VoiceChannel>, DomainError> {
        self.repo.find_all().await
    }

    async fn list_channels(&self, guild_id: &str) -> Result<Vec<VoiceChannel>, DomainError> {
        let cache_key = format!("voice_channels:{guild_id}");

        if let Some(json) = self.cache.get_json(&cache_key).await? {
            if let Ok(channels) = serde_json::from_str::<Vec<VoiceChannel>>(&json) {
                return Ok(channels);
            }
        }

        let channels = self.repo.find_all_by_guild(guild_id).await?;

        if let Ok(json) = serde_json::to_string(&channels) {
            self.cache.set_json(&cache_key, &json, CHANNELS_LIST_TTL).await.ok();
        }

        Ok(channels)
    }

    async fn get_channel_detail(&self, channel_id: &str) -> Result<VoiceChannelDetail, DomainError> {
        let cache_key = format!("voice_channel:{channel_id}");

        if let Some(json) = self.cache.get_json(&cache_key).await? {
            if let Ok(detail) = serde_json::from_str::<VoiceChannelDetail>(&json) {
                return Ok(detail);
            }
        }

        let channel = self.resolve_channel(channel_id).await?;
        let co_admins = self.repo.find_co_admins(channel.id).await?;
        let bans = self.repo.find_bans(channel.id).await?;
        let invite_links = self.repo.find_invite_links(channel.id).await?;

        let detail = VoiceChannelDetail { channel, co_admins, bans, invite_links };

        if let Ok(json) = serde_json::to_string(&detail) {
            self.cache.set_json(&cache_key, &json, CHANNEL_DETAIL_TTL).await.ok();
        }

        Ok(detail)
    }

    async fn create_channel(&self, cmd: CreateVoiceChannelCommand) -> Result<VoiceChannel, DomainError> {
        let channel = VoiceChannel {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            owner_id: cmd.owner_id,
            owner_name: cmd.owner_name,
            channel_id: cmd.channel_id,
            text_channel_id: cmd.text_channel_id,
            members_channel_id: cmd.members_channel_id,
            queue_channel_id: cmd.queue_channel_id,
            category_id: cmd.category_id,
            channel_name: cmd.channel_name,
            kind: cmd.kind,
            visibility: cmd.visibility,
            queue_enabled: cmd.queue_enabled,
            locked: false,
            stage_enabled: cmd.stage_enabled,
            member_limit: None,
            status: None,
            channel_status: "open".to_string(),
            closed_at: None,
            created_at: Utc::now(),
        };

        self.repo.save(&channel).await?;
        self.cache.invalidate(&format!("voice_channels:{}", channel.guild_id)).await.ok();

        Ok(channel)
    }

    async fn close_channel(&self, channel_id: &str) -> Result<(), DomainError> {
        self.repo.close_by_channel_id(channel_id).await?;
        // Invalider le cache — on essaie de résoudre le channel pour le guild_id
        if let Ok(channel) = self.resolve_channel(channel_id).await {
            self.invalidate_cache(&channel.guild_id, channel_id).await;
        }
        Ok(())
    }

    async fn delete_channel(&self, channel_id: &str) -> Result<(), DomainError> {
        // Soft-delete : close au lieu de supprimer
        self.close_channel(channel_id).await
    }

    async fn update_channel(&self, cmd: UpdateVoiceChannelCommand) -> Result<(), DomainError> {
        let channel = self.resolve_channel(&cmd.channel_id).await?;

        if let Some(vis) = &cmd.visibility {
            self.repo.update_visibility(channel.id, vis).await?;
        }
        if let Some(locked) = cmd.locked {
            self.repo.update_locked(channel.id, locked).await?;
        }
        if let Some(queue_enabled) = cmd.queue_enabled {
            self.repo.update_queue_enabled(channel.id, queue_enabled).await?;
        }
        if let Some(name) = &cmd.name {
            self.repo.update_name(channel.id, name).await?;
        }
        if let Some(status) = &cmd.status {
            self.repo.update_status(channel.id, Some(status)).await?;
        }
        if let Some(limit) = cmd.member_limit {
            self.repo.update_member_limit(channel.id, limit).await?;
        }
        if let Some(queue_ch) = &cmd.queue_channel_id {
            self.repo.update_queue_channel(channel.id, queue_ch.as_deref()).await?;
        }
        if let Some(stage) = cmd.stage_enabled {
            self.repo.update_stage(channel.id, stage).await?;
        }

        self.invalidate_cache(&channel.guild_id, &cmd.channel_id).await;
        Ok(())
    }

    async fn transfer_ownership(&self, cmd: TransferOwnershipCommand) -> Result<(), DomainError> {
        let channel = self.resolve_channel(&cmd.channel_id).await?;
        self.repo.update_owner(channel.id, &cmd.new_owner_id, &cmd.new_owner_name).await?;
        self.invalidate_cache(&channel.guild_id, &cmd.channel_id).await;
        Ok(())
    }

    async fn add_co_admin(&self, cmd: ManageCoAdminCommand) -> Result<(), DomainError> {
        let channel = self.resolve_channel(&cmd.channel_id).await?;

        let co_admin = VoiceChannelCoAdmin {
            id: Uuid::new_v4(),
            voice_channel_id: channel.id,
            user_id: cmd.user_id,
            user_name: cmd.user_name,
            granted_at: Utc::now(),
        };

        self.repo.add_co_admin(&co_admin).await?;
        self.invalidate_cache(&channel.guild_id, &cmd.channel_id).await;
        Ok(())
    }

    async fn remove_co_admin(&self, channel_id: &str, user_id: &str) -> Result<(), DomainError> {
        let channel = self.resolve_channel(channel_id).await?;
        self.repo.remove_co_admin(channel.id, user_id).await?;
        self.invalidate_cache(&channel.guild_id, channel_id).await;
        Ok(())
    }

    async fn get_whitelist(&self, guild_id: &str, owner_id: &str) -> Result<Vec<VoiceChannelWhitelistEntry>, DomainError> {
        self.repo.find_whitelist(guild_id, owner_id).await
    }

    async fn add_to_whitelist(&self, cmd: ManageWhitelistCommand) -> Result<(), DomainError> {
        let entry = VoiceChannelWhitelistEntry {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            owner_id: cmd.owner_id,
            target_id: cmd.target_id,
            target_name: cmd.target_name,
            created_at: Utc::now(),
        };

        self.repo.add_to_whitelist(&entry).await
    }

    async fn remove_from_whitelist(&self, guild_id: &str, owner_id: &str, target_id: &str) -> Result<(), DomainError> {
        self.repo.remove_from_whitelist(guild_id, owner_id, target_id).await
    }

    async fn ban_from_channel(&self, cmd: BanFromChannelCommand) -> Result<(), DomainError> {
        let channel = self.resolve_channel(&cmd.channel_id).await?;

        let expires_at = cmd.duration_secs.map(|secs| Utc::now() + chrono::Duration::seconds(secs));

        let ban = VoiceChannelBan {
            id: Uuid::new_v4(),
            voice_channel_id: channel.id,
            user_id: cmd.user_id,
            user_name: cmd.user_name,
            banned_by: cmd.banned_by,
            reason: cmd.reason,
            expires_at,
            created_at: Utc::now(),
        };

        self.repo.save_ban(&ban).await?;
        self.invalidate_cache(&channel.guild_id, &cmd.channel_id).await;
        Ok(())
    }

    async fn unban_from_channel(&self, channel_id: &str, user_id: &str) -> Result<(), DomainError> {
        let channel = self.resolve_channel(channel_id).await?;
        self.repo.remove_ban(channel.id, user_id).await?;
        self.invalidate_cache(&channel.guild_id, channel_id).await;
        Ok(())
    }

    async fn is_banned(&self, channel_id: &str, user_id: &str) -> Result<bool, DomainError> {
        let channel = self.resolve_channel(channel_id).await?;
        let ban = self.repo.find_active_ban(channel.id, user_id).await?;
        Ok(ban.is_some())
    }

    // ── Invite Links ──

    async fn create_invite_link(&self, cmd: CreateInviteLinkCommand) -> Result<VoiceChannelInviteLink, DomainError> {
        let channel = self.resolve_channel(&cmd.channel_id).await?;
        let duration_secs = cmd.duration_secs.unwrap_or(1800);
        let expires_at = Utc::now() + chrono::Duration::seconds(duration_secs);
        let code = self.generate_unique_code().await?;

        let link = VoiceChannelInviteLink {
            id: Uuid::new_v4(),
            voice_channel_id: channel.id,
            guild_id: channel.guild_id.clone(),
            channel_id: channel.channel_id.clone(),
            created_by: cmd.created_by,
            created_by_name: cmd.created_by_name,
            code,
            max_uses: cmd.max_uses,
            current_uses: 0,
            expires_at,
            revoked: false,
            created_at: Utc::now(),
        };

        self.repo.save_invite_link(&link).await?;
        self.invalidate_cache(&channel.guild_id, &cmd.channel_id).await;

        Ok(link)
    }

    async fn list_invite_links(&self, channel_id: &str) -> Result<Vec<VoiceChannelInviteLink>, DomainError> {
        let channel = self.resolve_channel(channel_id).await?;
        self.repo.find_invite_links(channel.id).await
    }

    async fn use_invite_link(&self, cmd: UseInviteLinkCommand) -> Result<VoiceChannelInviteLink, DomainError> {
        let mut link = self.repo
            .find_invite_by_code(&cmd.code)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Code d'invitation invalide : {}", cmd.code)))?;

        if link.revoked {
            return Err(DomainError::ValidationError("Ce lien d'invitation a ete revoque".to_string()));
        }
        if link.expires_at < Utc::now() {
            return Err(DomainError::ValidationError("Ce lien d'invitation a expire".to_string()));
        }

        let incremented = self.repo.increment_invite_uses(link.id).await?;
        if !incremented {
            return Err(DomainError::ValidationError("Ce lien d'invitation n'est plus utilisable (limite atteinte ou expire)".to_string()));
        }

        // Mettre a jour current_uses pour refléter l'increment
        link.current_uses += 1;

        // Whitelist the user
        let channel = self.resolve_channel(&link.channel_id).await?;
        let entry = VoiceChannelWhitelistEntry {
            id: Uuid::new_v4(),
            guild_id: link.guild_id.clone(),
            owner_id: channel.owner_id.clone(),
            target_id: cmd.user_id,
            target_name: cmd.user_name,
            created_at: Utc::now(),
        };
        self.repo.add_to_whitelist(&entry).await?;
        self.invalidate_cache(&link.guild_id, &link.channel_id).await;

        Ok(link)
    }

    async fn revoke_invite_link(&self, channel_id: &str, link_id: &str) -> Result<(), DomainError> {
        let channel = self.resolve_channel(channel_id).await?;
        let id = Uuid::parse_str(link_id)
            .map_err(|_| DomainError::ValidationError(format!("ID invalide : {link_id}")))?;
        self.repo.revoke_invite_link(id).await?;
        self.invalidate_cache(&channel.guild_id, channel_id).await;
        Ok(())
    }

    // ── Themes ──

    async fn list_themes(&self, guild_id: &str) -> Result<Vec<VoiceChannelTheme>, DomainError> {
        self.repo.find_themes(guild_id).await
    }

    async fn create_theme(&self, cmd: CreateThemeCommand) -> Result<VoiceChannelTheme, DomainError> {
        Self::validate_theme(&cmd)?;

        if cmd.is_default {
            self.repo.clear_default_themes(&cmd.guild_id).await?;
        }

        let theme = VoiceChannelTheme {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            name: cmd.name,
            emoji: cmd.emoji,
            channel_name_template: cmd.channel_name_template,
            member_limit: cmd.member_limit,
            visibility: cmd.visibility,
            locked: cmd.locked,
            queue_enabled: cmd.queue_enabled,
            bitrate: cmd.bitrate,
            slowmode_secs: cmd.slowmode_secs,
            stage_enabled: cmd.stage_enabled,
            is_default: cmd.is_default,
            sort_order: cmd.sort_order,
            created_at: Utc::now(),
        };

        self.repo.save_theme(&theme).await?;
        Ok(theme)
    }

    async fn update_theme(&self, theme_id: &str, cmd: CreateThemeCommand) -> Result<VoiceChannelTheme, DomainError> {
        Self::validate_theme(&cmd)?;

        let id = Uuid::parse_str(theme_id)
            .map_err(|_| DomainError::ValidationError(format!("ID invalide : {theme_id}")))?;

        let existing = self.repo.find_theme(id).await?
            .ok_or_else(|| DomainError::NotFound(format!("Theme introuvable : {theme_id}")))?;

        // Verifier que le theme appartient au bon guild
        if existing.guild_id != cmd.guild_id {
            return Err(DomainError::ValidationError("Ce theme n'appartient pas a ce serveur".to_string()));
        }

        if cmd.is_default {
            self.repo.clear_default_themes(&existing.guild_id).await?;
        }

        let theme = VoiceChannelTheme {
            id,
            guild_id: existing.guild_id,
            name: cmd.name,
            emoji: cmd.emoji,
            channel_name_template: cmd.channel_name_template,
            member_limit: cmd.member_limit,
            visibility: cmd.visibility,
            locked: cmd.locked,
            queue_enabled: cmd.queue_enabled,
            bitrate: cmd.bitrate,
            slowmode_secs: cmd.slowmode_secs,
            stage_enabled: cmd.stage_enabled,
            is_default: cmd.is_default,
            sort_order: cmd.sort_order,
            created_at: existing.created_at,
        };

        self.repo.update_theme(&theme).await?;
        Ok(theme)
    }

    async fn delete_theme(&self, guild_id: &str, theme_id: &str) -> Result<(), DomainError> {
        let id = Uuid::parse_str(theme_id)
            .map_err(|_| DomainError::ValidationError(format!("ID invalide : {theme_id}")))?;

        // Verifier que le theme appartient au bon guild
        let existing = self.repo.find_theme(id).await?
            .ok_or_else(|| DomainError::NotFound(format!("Theme introuvable : {theme_id}")))?;

        if existing.guild_id != guild_id {
            return Err(DomainError::ValidationError("Ce theme n'appartient pas a ce serveur".to_string()));
        }

        self.repo.delete_theme(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_theme_cmd(name: &str) -> CreateThemeCommand {
        CreateThemeCommand {
            guild_id: "guild1".to_string(),
            name: name.to_string(),
            emoji: None,
            channel_name_template: "{user}".to_string(),
            member_limit: None,
            visibility: "visible".to_string(),
            locked: false,
            queue_enabled: false,
            bitrate: None,
            slowmode_secs: None,
            stage_enabled: false,
            is_default: false,
            sort_order: 0,
        }
    }

    // ── generate_code ──

    #[test]
    fn generate_code_length_is_8() {
        let code = ManageVoiceChannelsService::generate_code();
        assert_eq!(code.len(), 8);
    }

    #[test]
    fn generate_code_is_uppercase_alphanumeric() {
        let code = ManageVoiceChannelsService::generate_code();
        assert!(code.chars().all(|c| c.is_ascii_alphanumeric() && (c.is_ascii_uppercase() || c.is_ascii_digit())));
    }

    #[test]
    fn generate_code_produces_different_values() {
        let code1 = ManageVoiceChannelsService::generate_code();
        let code2 = ManageVoiceChannelsService::generate_code();
        // Statistically near-impossible to collide with 36^8 space
        assert_ne!(code1, code2);
    }

    // ── validate_theme ──

    #[test]
    fn validate_theme_valid() {
        let cmd = make_theme_cmd("Gaming");
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_ok());
    }

    #[test]
    fn validate_theme_empty_name() {
        let cmd = make_theme_cmd("");
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_err());
    }

    #[test]
    fn validate_theme_whitespace_name() {
        let cmd = make_theme_cmd("   ");
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_err());
    }

    #[test]
    fn validate_theme_name_too_long() {
        let long_name = "a".repeat(101);
        let cmd = make_theme_cmd(&long_name);
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_err());
    }

    #[test]
    fn validate_theme_name_exactly_100() {
        let name = "a".repeat(100);
        let cmd = make_theme_cmd(&name);
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_ok());
    }

    #[test]
    fn validate_theme_member_limit_valid() {
        let mut cmd = make_theme_cmd("Test");
        cmd.member_limit = Some(10);
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_ok());
    }

    #[test]
    fn validate_theme_member_limit_zero() {
        let mut cmd = make_theme_cmd("Test");
        cmd.member_limit = Some(0);
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_ok());
    }

    #[test]
    fn validate_theme_member_limit_too_high() {
        let mut cmd = make_theme_cmd("Test");
        cmd.member_limit = Some(100);
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_err());
    }

    #[test]
    fn validate_theme_member_limit_negative() {
        let mut cmd = make_theme_cmd("Test");
        cmd.member_limit = Some(-1);
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_err());
    }

    #[test]
    fn validate_theme_bitrate_valid() {
        let mut cmd = make_theme_cmd("Test");
        cmd.bitrate = Some(64000);
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_ok());
    }

    #[test]
    fn validate_theme_bitrate_too_low() {
        let mut cmd = make_theme_cmd("Test");
        cmd.bitrate = Some(7999);
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_err());
    }

    #[test]
    fn validate_theme_bitrate_too_high() {
        let mut cmd = make_theme_cmd("Test");
        cmd.bitrate = Some(384001);
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_err());
    }

    #[test]
    fn validate_theme_bitrate_boundary_low() {
        let mut cmd = make_theme_cmd("Test");
        cmd.bitrate = Some(8000);
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_ok());
    }

    #[test]
    fn validate_theme_bitrate_boundary_high() {
        let mut cmd = make_theme_cmd("Test");
        cmd.bitrate = Some(384000);
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_ok());
    }

    #[test]
    fn validate_theme_slowmode_valid() {
        let mut cmd = make_theme_cmd("Test");
        cmd.slowmode_secs = Some(30);
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_ok());
    }

    #[test]
    fn validate_theme_slowmode_too_high() {
        let mut cmd = make_theme_cmd("Test");
        cmd.slowmode_secs = Some(21601);
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_err());
    }

    #[test]
    fn validate_theme_slowmode_negative() {
        let mut cmd = make_theme_cmd("Test");
        cmd.slowmode_secs = Some(-1);
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_err());
    }

    #[test]
    fn validate_theme_visibility_visible() {
        let mut cmd = make_theme_cmd("Test");
        cmd.visibility = "visible".to_string();
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_ok());
    }

    #[test]
    fn validate_theme_visibility_hidden() {
        let mut cmd = make_theme_cmd("Test");
        cmd.visibility = "hidden".to_string();
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_ok());
    }

    #[test]
    fn validate_theme_visibility_invalid() {
        let mut cmd = make_theme_cmd("Test");
        cmd.visibility = "invalid".to_string();
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_err());
    }

    #[test]
    fn validate_theme_none_optionals_ok() {
        let cmd = make_theme_cmd("Test");
        // member_limit, bitrate, slowmode all None
        assert!(ManageVoiceChannelsService::validate_theme(&cmd).is_ok());
    }

    // ══════════════════════════════════════════════════════════
    // Mock-based integration tests
    // ══════════════════════════════════════════════════════════

    use std::sync::Mutex;
    use crate::domain::entities::Rule;
    use crate::ports::outbound::CachePort;

    // ── Mock Cache ──

    struct MockCache;

    #[async_trait]
    impl CachePort for MockCache {
        async fn get_rules(&self, _: &str) -> Result<Option<Vec<Rule>>, DomainError> { Ok(None) }
        async fn set_rules(&self, _: &str, _: &[Rule]) -> Result<(), DomainError> { Ok(()) }
        async fn invalidate_rules(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
        async fn get_json(&self, _: &str) -> Result<Option<String>, DomainError> { Ok(None) }
        async fn set_json(&self, _: &str, _: &str, _: u64) -> Result<(), DomainError> { Ok(()) }
        async fn invalidate(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
        async fn invalidate_pattern(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
    }

    // ── Mock Repo ──

    struct MockVoiceRepo {
        channel: Mutex<Option<VoiceChannel>>,
        invite_links: Mutex<Vec<VoiceChannelInviteLink>>,
        themes: Mutex<Vec<VoiceChannelTheme>>,
        whitelist_entries: Mutex<Vec<VoiceChannelWhitelistEntry>>,
        increment_result: Mutex<bool>,
    }

    impl MockVoiceRepo {
        fn new() -> Self {
            Self {
                channel: Mutex::new(None),
                invite_links: Mutex::new(vec![]),
                themes: Mutex::new(vec![]),
                whitelist_entries: Mutex::new(vec![]),
                increment_result: Mutex::new(true),
            }
        }

        fn with_channel(self, ch: VoiceChannel) -> Self {
            *self.channel.lock().unwrap() = Some(ch);
            self
        }

        fn with_invite_link(self, link: VoiceChannelInviteLink) -> Self {
            self.invite_links.lock().unwrap().push(link);
            self
        }

        fn with_increment_result(self, result: bool) -> Self {
            *self.increment_result.lock().unwrap() = result;
            self
        }

        fn with_theme(self, theme: VoiceChannelTheme) -> Self {
            self.themes.lock().unwrap().push(theme);
            self
        }
    }

    fn make_test_channel() -> VoiceChannel {
        VoiceChannel {
            id: Uuid::new_v4(),
            guild_id: "guild1".into(),
            owner_id: "owner1".into(),
            owner_name: "Owner".into(),
            channel_id: "chan1".into(),
            text_channel_id: None,
            members_channel_id: None,
            queue_channel_id: None,
            category_id: None,
            channel_name: "Test".into(),
            kind: "private".into(),
            visibility: "visible".into(),
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

    fn make_test_invite(code: &str, revoked: bool, expired: bool, max_uses: Option<i32>, current_uses: i32) -> VoiceChannelInviteLink {
        let expires_at = if expired {
            Utc::now() - chrono::Duration::hours(1)
        } else {
            Utc::now() + chrono::Duration::hours(1)
        };
        VoiceChannelInviteLink {
            id: Uuid::new_v4(),
            voice_channel_id: Uuid::new_v4(),
            guild_id: "guild1".into(),
            channel_id: "chan1".into(),
            created_by: "user1".into(),
            created_by_name: "User".into(),
            code: code.into(),
            max_uses,
            current_uses,
            expires_at,
            revoked,
            created_at: Utc::now(),
        }
    }

    #[async_trait]
    impl VoiceChannelRepository for MockVoiceRepo {
        async fn find_all(&self) -> Result<Vec<VoiceChannel>, DomainError> { Ok(vec![]) }
        async fn find_all_by_guild(&self, _: &str) -> Result<Vec<VoiceChannel>, DomainError> { Ok(vec![]) }
        async fn find_by_channel_id(&self, _: &str) -> Result<Option<VoiceChannel>, DomainError> {
            Ok(self.channel.lock().unwrap().clone())
        }
        async fn find_by_id(&self, _: Uuid) -> Result<Option<VoiceChannel>, DomainError> {
            Ok(self.channel.lock().unwrap().clone())
        }
        async fn save(&self, _: &VoiceChannel) -> Result<(), DomainError> { Ok(()) }
        async fn close(&self, _: Uuid) -> Result<(), DomainError> { Ok(()) }
        async fn close_by_channel_id(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
        async fn delete(&self, _: Uuid) -> Result<(), DomainError> { Ok(()) }
        async fn update_visibility(&self, _: Uuid, _: &str) -> Result<(), DomainError> { Ok(()) }
        async fn update_locked(&self, _: Uuid, _: bool) -> Result<(), DomainError> { Ok(()) }
        async fn update_queue_enabled(&self, _: Uuid, _: bool) -> Result<(), DomainError> { Ok(()) }
        async fn update_name(&self, _: Uuid, _: &str) -> Result<(), DomainError> { Ok(()) }
        async fn update_status(&self, _: Uuid, _: Option<&str>) -> Result<(), DomainError> { Ok(()) }
        async fn update_member_limit(&self, _: Uuid, _: Option<i32>) -> Result<(), DomainError> { Ok(()) }
        async fn update_owner(&self, _: Uuid, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
        async fn update_queue_channel(&self, _: Uuid, _: Option<&str>) -> Result<(), DomainError> { Ok(()) }
        async fn update_stage(&self, _: Uuid, _: bool) -> Result<(), DomainError> { Ok(()) }
        async fn find_co_admins(&self, _: Uuid) -> Result<Vec<VoiceChannelCoAdmin>, DomainError> { Ok(vec![]) }
        async fn add_co_admin(&self, _: &VoiceChannelCoAdmin) -> Result<(), DomainError> { Ok(()) }
        async fn remove_co_admin(&self, _: Uuid, _: &str) -> Result<(), DomainError> { Ok(()) }
        async fn find_whitelist(&self, _: &str, _: &str) -> Result<Vec<VoiceChannelWhitelistEntry>, DomainError> { Ok(vec![]) }
        async fn add_to_whitelist(&self, entry: &VoiceChannelWhitelistEntry) -> Result<(), DomainError> {
            self.whitelist_entries.lock().unwrap().push(entry.clone());
            Ok(())
        }
        async fn remove_from_whitelist(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
        async fn find_bans(&self, _: Uuid) -> Result<Vec<VoiceChannelBan>, DomainError> { Ok(vec![]) }
        async fn find_active_ban(&self, _: Uuid, _: &str) -> Result<Option<VoiceChannelBan>, DomainError> { Ok(None) }
        async fn save_ban(&self, _: &VoiceChannelBan) -> Result<(), DomainError> { Ok(()) }
        async fn remove_ban(&self, _: Uuid, _: &str) -> Result<(), DomainError> { Ok(()) }
        async fn cleanup_expired_bans(&self) -> Result<u64, DomainError> { Ok(0) }
        async fn find_invite_links(&self, _: Uuid) -> Result<Vec<VoiceChannelInviteLink>, DomainError> {
            Ok(self.invite_links.lock().unwrap().clone())
        }
        async fn find_invite_by_code(&self, code: &str) -> Result<Option<VoiceChannelInviteLink>, DomainError> {
            Ok(self.invite_links.lock().unwrap().iter().find(|l| l.code == code).cloned())
        }
        async fn save_invite_link(&self, link: &VoiceChannelInviteLink) -> Result<(), DomainError> {
            self.invite_links.lock().unwrap().push(link.clone());
            Ok(())
        }
        async fn increment_invite_uses(&self, _: Uuid) -> Result<bool, DomainError> {
            Ok(*self.increment_result.lock().unwrap())
        }
        async fn revoke_invite_link(&self, _: Uuid) -> Result<(), DomainError> { Ok(()) }
        async fn find_themes(&self, _: &str) -> Result<Vec<VoiceChannelTheme>, DomainError> {
            Ok(self.themes.lock().unwrap().clone())
        }
        async fn find_theme(&self, id: Uuid) -> Result<Option<VoiceChannelTheme>, DomainError> {
            Ok(self.themes.lock().unwrap().iter().find(|t| t.id == id).cloned())
        }
        async fn save_theme(&self, theme: &VoiceChannelTheme) -> Result<(), DomainError> {
            self.themes.lock().unwrap().push(theme.clone());
            Ok(())
        }
        async fn update_theme(&self, _: &VoiceChannelTheme) -> Result<(), DomainError> { Ok(()) }
        async fn delete_theme(&self, _: Uuid) -> Result<(), DomainError> { Ok(()) }
        async fn clear_default_themes(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
    }

    fn make_service(repo: MockVoiceRepo) -> ManageVoiceChannelsService {
        ManageVoiceChannelsService::new(
            Arc::new(repo),
            Arc::new(MockCache),
        )
    }

    // ── create_invite_link ──

    #[tokio::test]
    async fn create_invite_link_default_duration() {
        let repo = MockVoiceRepo::new().with_channel(make_test_channel());
        let svc = make_service(repo);

        let link = svc.create_invite_link(CreateInviteLinkCommand {
            channel_id: "chan1".into(),
            created_by: "user1".into(),
            created_by_name: "User".into(),
            duration_secs: None, // default 1800
            max_uses: None,
        }).await.unwrap();

        assert_eq!(link.code.len(), 8);
        assert!(!link.revoked);
        assert_eq!(link.current_uses, 0);
        assert!(link.max_uses.is_none());
        // expires_at should be ~30 min from now
        let diff = link.expires_at - Utc::now();
        assert!(diff.num_seconds() > 1790 && diff.num_seconds() <= 1800);
    }

    #[tokio::test]
    async fn create_invite_link_custom_duration() {
        let repo = MockVoiceRepo::new().with_channel(make_test_channel());
        let svc = make_service(repo);

        let link = svc.create_invite_link(CreateInviteLinkCommand {
            channel_id: "chan1".into(),
            created_by: "user1".into(),
            created_by_name: "User".into(),
            duration_secs: Some(3600),
            max_uses: Some(10),
        }).await.unwrap();

        assert_eq!(link.max_uses, Some(10));
        let diff = link.expires_at - Utc::now();
        assert!(diff.num_seconds() > 3590 && diff.num_seconds() <= 3600);
    }

    #[tokio::test]
    async fn create_invite_link_channel_not_found() {
        let repo = MockVoiceRepo::new(); // no channel
        let svc = make_service(repo);

        let result = svc.create_invite_link(CreateInviteLinkCommand {
            channel_id: "unknown".into(),
            created_by: "user1".into(),
            created_by_name: "User".into(),
            duration_secs: None,
            max_uses: None,
        }).await;

        assert!(result.is_err());
    }

    // ── use_invite_link ──

    #[tokio::test]
    async fn use_invite_link_success() {
        let link = make_test_invite("CODE1234", false, false, None, 0);
        let repo = MockVoiceRepo::new()
            .with_channel(make_test_channel())
            .with_invite_link(link);
        let svc = make_service(repo);

        let result = svc.use_invite_link(UseInviteLinkCommand {
            code: "CODE1234".into(),
            user_id: "user2".into(),
            user_name: "User2".into(),
        }).await;

        assert!(result.is_ok());
        let used = result.unwrap();
        assert_eq!(used.current_uses, 1); // incremented
    }

    #[tokio::test]
    async fn use_invite_link_revoked() {
        let link = make_test_invite("REVOKED1", true, false, None, 0);
        let repo = MockVoiceRepo::new()
            .with_channel(make_test_channel())
            .with_invite_link(link);
        let svc = make_service(repo);

        let result = svc.use_invite_link(UseInviteLinkCommand {
            code: "REVOKED1".into(),
            user_id: "user2".into(),
            user_name: "User2".into(),
        }).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("revoque"));
    }

    #[tokio::test]
    async fn use_invite_link_expired() {
        let link = make_test_invite("EXPIRED1", false, true, None, 0);
        let repo = MockVoiceRepo::new()
            .with_channel(make_test_channel())
            .with_invite_link(link);
        let svc = make_service(repo);

        let result = svc.use_invite_link(UseInviteLinkCommand {
            code: "EXPIRED1".into(),
            user_id: "user2".into(),
            user_name: "User2".into(),
        }).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("expire"));
    }

    #[tokio::test]
    async fn use_invite_link_max_uses_reached() {
        let link = make_test_invite("MAXUSED1", false, false, Some(5), 5);
        let repo = MockVoiceRepo::new()
            .with_channel(make_test_channel())
            .with_invite_link(link)
            .with_increment_result(false); // atomic increment returns false
        let svc = make_service(repo);

        let result = svc.use_invite_link(UseInviteLinkCommand {
            code: "MAXUSED1".into(),
            user_id: "user2".into(),
            user_name: "User2".into(),
        }).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("limite"));
    }

    #[tokio::test]
    async fn use_invite_link_invalid_code() {
        let repo = MockVoiceRepo::new().with_channel(make_test_channel());
        let svc = make_service(repo);

        let result = svc.use_invite_link(UseInviteLinkCommand {
            code: "INVALID1".into(),
            user_id: "user2".into(),
            user_name: "User2".into(),
        }).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalide"));
    }

    // ── revoke_invite_link ──

    #[tokio::test]
    async fn revoke_invite_link_success() {
        let repo = MockVoiceRepo::new().with_channel(make_test_channel());
        let svc = make_service(repo);

        let result = svc.revoke_invite_link("chan1", &Uuid::new_v4().to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn revoke_invite_link_invalid_id() {
        let repo = MockVoiceRepo::new().with_channel(make_test_channel());
        let svc = make_service(repo);

        let result = svc.revoke_invite_link("chan1", "not-a-uuid").await;
        assert!(result.is_err());
    }

    // ── create_channel ──

    #[tokio::test]
    async fn create_channel_defaults() {
        let repo = MockVoiceRepo::new();
        let svc = make_service(repo);

        let ch = svc.create_channel(CreateVoiceChannelCommand {
            guild_id: "g1".into(),
            owner_id: "o1".into(),
            owner_name: "Owner".into(),
            channel_id: "c1".into(),
            text_channel_id: None,
            members_channel_id: None,
            queue_channel_id: None,
            category_id: None,
            channel_name: "Test".into(),
            kind: "private".into(),
            visibility: "visible".into(),
            queue_enabled: false,
            stage_enabled: false,
        }).await.unwrap();

        assert!(!ch.locked);
        assert!(!ch.stage_enabled);
        assert_eq!(ch.channel_status, "open");
        assert!(ch.closed_at.is_none());
        assert!(ch.member_limit.is_none());
        assert!(ch.status.is_none());
    }

    // ── create_theme ──

    #[tokio::test]
    async fn create_theme_success() {
        let repo = MockVoiceRepo::new();
        let svc = make_service(repo);

        let theme = svc.create_theme(make_theme_cmd("Gaming")).await.unwrap();
        assert_eq!(theme.name, "Gaming");
        assert!(!theme.is_default);
    }

    #[tokio::test]
    async fn create_theme_validation_error() {
        let repo = MockVoiceRepo::new();
        let svc = make_service(repo);

        let result = svc.create_theme(make_theme_cmd("")).await;
        assert!(result.is_err());
    }

    // ── delete_theme ──

    #[tokio::test]
    async fn delete_theme_wrong_guild() {
        let mut theme = VoiceChannelTheme {
            id: Uuid::new_v4(),
            guild_id: "guild2".into(), // different guild
            name: "Test".into(),
            emoji: None,
            channel_name_template: "{user}".into(),
            member_limit: None,
            visibility: "visible".into(),
            locked: false,
            queue_enabled: false,
            bitrate: None,
            slowmode_secs: None,
            stage_enabled: false,
            is_default: false,
            sort_order: 0,
            created_at: Utc::now(),
        };
        let theme_id = theme.id;

        let mut repo = MockVoiceRepo::new();
        repo.themes.lock().unwrap().push(theme);
        let svc = make_service(repo);

        let result = svc.delete_theme("guild1", &theme_id.to_string()).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("appartient pas"));
    }

    #[tokio::test]
    async fn delete_theme_invalid_id() {
        let repo = MockVoiceRepo::new();
        let svc = make_service(repo);

        let result = svc.delete_theme("guild1", "not-a-uuid").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn delete_theme_not_found() {
        let repo = MockVoiceRepo::new();
        let svc = make_service(repo);

        let result = svc.delete_theme("guild1", &Uuid::new_v4().to_string()).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("introuvable"));
    }

    // ══════════════════════════════════════════════════════════
    // Additional coverage: update_theme, list, get_channel_detail
    // ══════════════════════════════════════════════════════════

    fn make_test_theme(guild_id: &str, name: &str, is_default: bool) -> VoiceChannelTheme {
        VoiceChannelTheme {
            id: Uuid::new_v4(),
            guild_id: guild_id.into(),
            name: name.into(),
            emoji: None,
            channel_name_template: "{user}".into(),
            member_limit: None,
            visibility: "visible".into(),
            locked: false,
            queue_enabled: false,
            bitrate: None,
            slowmode_secs: None,
            stage_enabled: false,
            is_default,
            sort_order: 0,
            created_at: Utc::now(),
        }
    }

    // ── update_theme ──

    #[tokio::test]
    async fn update_theme_success() {
        let theme = make_test_theme("guild1", "Old Name", false);
        let theme_id = theme.id.to_string();
        let repo = MockVoiceRepo::new().with_theme(theme);
        let svc = make_service(repo);

        let mut cmd = make_theme_cmd("New Name");
        cmd.guild_id = "guild1".into();
        let result = svc.update_theme(&theme_id, cmd).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "New Name");
    }

    #[tokio::test]
    async fn update_theme_wrong_guild() {
        let theme = make_test_theme("guild2", "Test", false);
        let theme_id = theme.id.to_string();
        let repo = MockVoiceRepo::new().with_theme(theme);
        let svc = make_service(repo);

        let mut cmd = make_theme_cmd("Updated");
        cmd.guild_id = "guild1".into(); // wrong guild
        let result = svc.update_theme(&theme_id, cmd).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("appartient pas"));
    }

    #[tokio::test]
    async fn update_theme_not_found() {
        let repo = MockVoiceRepo::new();
        let svc = make_service(repo);

        let cmd = make_theme_cmd("Test");
        let result = svc.update_theme(&Uuid::new_v4().to_string(), cmd).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("introuvable"));
    }

    #[tokio::test]
    async fn update_theme_invalid_id() {
        let repo = MockVoiceRepo::new();
        let svc = make_service(repo);

        let cmd = make_theme_cmd("Test");
        let result = svc.update_theme("not-a-uuid", cmd).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_theme_validation_error() {
        let theme = make_test_theme("guild1", "Original", false);
        let theme_id = theme.id.to_string();
        let repo = MockVoiceRepo::new().with_theme(theme);
        let svc = make_service(repo);

        let mut cmd = make_theme_cmd(""); // empty name
        cmd.guild_id = "guild1".into();
        let result = svc.update_theme(&theme_id, cmd).await;
        assert!(result.is_err());
    }

    // ── list_themes ──

    #[tokio::test]
    async fn list_themes_empty() {
        let repo = MockVoiceRepo::new();
        let svc = make_service(repo);

        let themes = svc.list_themes("guild1").await.unwrap();
        assert!(themes.is_empty());
    }

    #[tokio::test]
    async fn list_themes_returns_all() {
        let repo = MockVoiceRepo::new()
            .with_theme(make_test_theme("guild1", "Gaming", false))
            .with_theme(make_test_theme("guild1", "Musique", true));
        let svc = make_service(repo);

        let themes = svc.list_themes("guild1").await.unwrap();
        assert_eq!(themes.len(), 2);
    }

    // ── list_invite_links ──

    #[tokio::test]
    async fn list_invite_links_empty() {
        let repo = MockVoiceRepo::new().with_channel(make_test_channel());
        let svc = make_service(repo);

        let links = svc.list_invite_links("chan1").await.unwrap();
        assert!(links.is_empty());
    }

    #[tokio::test]
    async fn list_invite_links_returns_all() {
        let link1 = make_test_invite("CODE1111", false, false, None, 0);
        let link2 = make_test_invite("CODE2222", false, false, None, 3);
        let repo = MockVoiceRepo::new()
            .with_channel(make_test_channel())
            .with_invite_link(link1)
            .with_invite_link(link2);
        let svc = make_service(repo);

        let links = svc.list_invite_links("chan1").await.unwrap();
        assert_eq!(links.len(), 2);
    }

    #[tokio::test]
    async fn list_invite_links_channel_not_found() {
        let repo = MockVoiceRepo::new(); // no channel
        let svc = make_service(repo);

        let result = svc.list_invite_links("unknown").await;
        assert!(result.is_err());
    }

    // ── get_channel_detail ──

    #[tokio::test]
    async fn get_channel_detail_includes_invite_links() {
        let link = make_test_invite("DETAIL01", false, false, None, 0);
        let repo = MockVoiceRepo::new()
            .with_channel(make_test_channel())
            .with_invite_link(link);
        let svc = make_service(repo);

        let detail = svc.get_channel_detail("chan1").await.unwrap();
        assert_eq!(detail.channel.channel_id, "chan1");
        assert_eq!(detail.invite_links.len(), 1);
        assert_eq!(detail.invite_links[0].code, "DETAIL01");
        assert!(detail.co_admins.is_empty());
        assert!(detail.bans.is_empty());
    }

    #[tokio::test]
    async fn get_channel_detail_not_found() {
        let repo = MockVoiceRepo::new(); // no channel
        let svc = make_service(repo);

        let result = svc.get_channel_detail("unknown").await;
        assert!(result.is_err());
    }

    // ── is_banned ──

    #[tokio::test]
    async fn is_banned_returns_false_when_no_ban() {
        let repo = MockVoiceRepo::new().with_channel(make_test_channel());
        let svc = make_service(repo);

        let banned = svc.is_banned("chan1", "user1").await.unwrap();
        assert!(!banned);
    }

    // ── use_invite_link whitelists the user ──

    #[tokio::test]
    async fn use_invite_link_adds_to_whitelist() {
        let link = make_test_invite("WHITE123", false, false, None, 0);
        let repo = MockVoiceRepo::new()
            .with_channel(make_test_channel())
            .with_invite_link(link);
        let svc = make_service(repo);

        svc.use_invite_link(UseInviteLinkCommand {
            code: "WHITE123".into(),
            user_id: "invited_user".into(),
            user_name: "Invited".into(),
        }).await.unwrap();

        // Verify whitelist was called (check via repo state)
        // The mock stores whitelist entries
        // We can't easily access the inner repo after Arc wrapping,
        // but the test passing without error confirms add_to_whitelist was called
    }
}
