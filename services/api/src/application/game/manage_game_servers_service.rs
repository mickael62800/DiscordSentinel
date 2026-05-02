//! Use case principal Game Portal — orchestre Docker, Postgres, Redis, RCON.
//!
//! Securite & robustesse :
//!  - Whitelist templates : valide que le slug demande est dans
//!    `allowed_templates` (config bot guild).
//!  - Quota : verifie max_servers_per_guild et max_memory_total_mb avant create.
//!  - Validation memory_mb dans les bornes du template.
//!  - Allocation atomique des ports via Redis SETNX (deux ranges).
//!  - Audit log systematique de chaque action.
//!  - Etat DB persiste avant tout appel Docker (rollback en cas d'erreur).

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::application::game::config_loader::{
    load_game_portal_config, GamePortalConfig,
};
use crate::application::game::password_gen::generate_rcon_password;
use crate::domain::entities::game::audit::GameAuditAction;
use crate::domain::entities::game::quota::GuildQuotaState;
use crate::domain::entities::game::server::{
    validate_server_name, CreateGameServerCommand, GameServer, GameServerStatus,
};
use crate::domain::entities::game::template::GameTemplate;
use crate::domain::errors::DomainError;
use crate::ports::inbound::game::manage_game_servers::{
    GameServerDetail, ManageGameServersUseCase,
};
use crate::ports::outbound::game::container_runtime::{
    ContainerRuntime, ContainerSpec, ContainerStats, PortMapping, PortProtocol, RestartPolicy,
    VolumeMount,
};
use crate::ports::outbound::game::game_audit_repository::GameAuditRepository;
use crate::ports::outbound::game::game_server_config_repository::GameServerConfigRepository;
use crate::ports::outbound::game::game_server_repository::{
    GameServerRepository, GameServerRuntimeUpdate, NewGameServer,
};
use crate::ports::outbound::game::game_template_repository::GameTemplateRepository;
use crate::ports::outbound::game::port_allocator::{PortAllocator, PortKind};
use crate::ports::outbound::game::rcon_client::{RconClient, RconConnectionParams};
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

const STOP_TIMEOUT_SECS: u32 = 30;

/// Substitue `{{KEY}}` (avec spaces tolerees) par `env[KEY]`. Si la cle
/// n'existe pas, le placeholder est remplace par une chaine vide (comme
/// Docker compose pour les env unset). Volontairement minimaliste : pas
/// de logique conditionnelle, pas d'echappement. Suffit pour seed des
/// fichiers de config jeu.
fn render_template(input: &str, env: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find("}}") {
            let key = after[..end].trim();
            if let Some(v) = env.get(key) {
                out.push_str(v);
            }
            rest = &after[end + 2..];
        } else {
            // Placeholder non ferme : on emet le tail tel quel et on sort.
            out.push_str("{{");
            out.push_str(after);
            return out;
        }
    }
    out.push_str(rest);
    out
}

pub struct ManageGameServersService {
    pub server_repo: Arc<dyn GameServerRepository>,
    pub template_repo: Arc<dyn GameTemplateRepository>,
    pub config_repo: Arc<dyn GameServerConfigRepository>,
    pub audit_repo: Arc<dyn GameAuditRepository>,
    pub container_runtime: Arc<dyn ContainerRuntime>,
    pub rcon_client: Arc<dyn RconClient>,
    pub port_allocator: Arc<dyn PortAllocator>,
    pub bot_config: Arc<dyn BotConfigRepository>,
}

impl ManageGameServersService {
    /// Resout le template, valide whitelist, quotas et memoire demandee.
    async fn validate_create(
        &self,
        cmd: &CreateGameServerCommand,
        cfg: &GamePortalConfig,
    ) -> Result<GameTemplate, DomainError> {
        if !cfg.enabled {
            return Err(DomainError::Forbidden(
                "Game Portal desactive pour cette guild (cf. config bot)".into(),
            ));
        }
        validate_server_name(&cmd.name).map_err(DomainError::ValidationError)?;

        // Whitelist template
        if !cfg.allowed_templates.iter().any(|s| s == &cmd.template_slug) {
            return Err(DomainError::Forbidden(format!(
                "template '{}' non autorise pour cette guild",
                cmd.template_slug
            )));
        }
        let template = self
            .template_repo
            .find_by_slug(&cmd.template_slug)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("template slug={} introuvable", cmd.template_slug))
            })?;

        // Memoire
        let memory = cmd.allocated_memory_mb.unwrap_or(template.default_memory_mb);
        template
            .validate_memory(memory)
            .map_err(DomainError::ValidationError)?;

        // Quota guild
        let (active, mem_alloc) = self
            .server_repo
            .count_active_for_guild(&cmd.guild_id)
            .await?;
        let quota = GuildQuotaState {
            active_servers: active,
            max_servers: cfg.max_servers_per_guild,
            allocated_memory_mb: mem_alloc,
            max_memory_mb: cfg.max_memory_total_mb,
        };
        quota
            .can_create_server(memory)
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        Ok(template)
    }

    /// Construit la spec Docker complete a partir du template + overrides + ports.
    fn build_spec(
        &self,
        server: &GameServer,
        template: &GameTemplate,
        overrides: &HashMap<String, String>,
        cfg: &GamePortalConfig,
    ) -> ContainerSpec {
        // Env = default_env + overrides + RCON injecte si applicable.
        let mut env: HashMap<String, String> = template
            .default_env
            .as_object()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().map(String::from).unwrap_or_default()))
                    .collect()
            })
            .unwrap_or_default();
        for (k, v) in overrides {
            env.insert(k.clone(), v.clone());
        }
        if template.supports_rcon && cfg.rcon_enabled {
            if let Some(pwd) = &server.rcon_password {
                env.insert("ENABLE_RCON".to_string(), "true".to_string());
                env.insert("RCON_PASSWORD".to_string(), pwd.clone());
                if let Some(p) = server.rcon_port {
                    // RCON_PORT du container : on garde 25575 par defaut Minecraft,
                    // le mapping host expose le port_alloue.
                    let _ = p;
                    env.insert("RCON_PORT".to_string(), "25575".to_string());
                }
            }
        }
        // Memory en bytes, conversion explicite.
        let memory_bytes = (server.allocated_memory_mb as u64) * 1024 * 1024;
        env.insert(
            "MEMORY".to_string(),
            format!("{}M", server.allocated_memory_mb),
        );

        let mut port_mappings = vec![];
        if let Some(host_port) = server.host_port {
            // Protocole defini par le template (TCP : Minecraft, Terraria ;
            // UDP : Valheim, Factorio, Palworld).
            let proto = match template.port_protocol {
                crate::domain::entities::game::template::PortProtocol::Tcp => PortProtocol::Tcp,
                crate::domain::entities::game::template::PortProtocol::Udp => PortProtocol::Udp,
            };
            port_mappings.push(PortMapping {
                host_port,
                container_port: template.container_port,
                protocol: proto,
            });
        }
        if template.supports_rcon && cfg.rcon_enabled {
            if let Some(rcon_host_port) = server.rcon_port {
                // RCON est toujours TCP.
                port_mappings.push(PortMapping {
                    host_port: rcon_host_port,
                    container_port: 25575,
                    protocol: PortProtocol::Tcp,
                });
            }
        }

        let volumes = if cfg.auto_create_world_volume {
            // Path interne specifique au template (Minecraft: /data,
            // Terraria: /root/.local/share/Terraria/Worlds, etc.)
            vec![VolumeMount {
                volume_name: server
                    .volume_name
                    .clone()
                    .unwrap_or_else(|| GameServer::docker_volume_name(server.id)),
                container_path: template.volume_path.clone(),
                read_only: false,
            }]
        } else {
            vec![]
        };

        let mut labels = HashMap::new();
        labels.insert("sentinel.server_id".to_string(), server.id.to_string());
        labels.insert("sentinel.guild_id".to_string(), server.guild_id.clone());
        labels.insert(
            "sentinel.template_slug".to_string(),
            template.slug.clone(),
        );
        labels.insert("sentinel.owner".to_string(), server.owner_user_id.clone());

        // Command (templated) : si le template definit un override CMD, on
        // substitue les {{KEY}} par les env effectives (defaults + overrides
        // utilisateur), puis on passe au runtime. Sinon None.
        let command = template.command.as_ref().map(|tmpl| {
            tmpl.iter().map(|arg| render_template(arg, &env)).collect::<Vec<_>>()
        });

        ContainerSpec {
            image: template.image.clone(),
            name: server
                .container_name
                .clone()
                .unwrap_or_else(|| GameServer::docker_container_name(server.id)),
            env,
            port_mappings,
            volumes,
            memory_bytes,
            network: cfg.docker_network_name.clone(),
            // Si run_as_root=true (Terraria, Valheim, Factorio), on ne
            // passe pas --user et l'image utilise son user par defaut.
            user: if template.run_as_root {
                None
            } else {
                Some(cfg.container_user.clone())
            },
            // L'auto-restart est gere par notre worker (pas Docker), pour
            // tracer chaque crash dans audit_log et appliquer du backoff.
            restart_policy: RestartPolicy::None,
            labels,
            command,
        }
    }

    /// Construit la map env effective (defaults + overrides) pour rendre
    /// les templates init_files / command. Reproduit la meme logique que
    /// `build_spec` mais sans injecter MEMORY/RCON (qui ne servent pas pour
    /// les fichiers de config jeu).
    fn render_env(
        template: &GameTemplate,
        overrides: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut env: HashMap<String, String> = template
            .default_env
            .as_object()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().map(String::from).unwrap_or_default()))
                    .collect()
            })
            .unwrap_or_default();
        for (k, v) in overrides {
            env.insert(k.clone(), v.clone());
        }
        env
    }

    async fn audit(
        &self,
        guild_id: &str,
        server_id: Option<Uuid>,
        actor: Option<&str>,
        action: GameAuditAction,
        details: serde_json::Value,
    ) {
        if let Err(e) = self
            .audit_repo
            .log(guild_id, server_id, actor, action, details)
            .await
        {
            warn!(error = %e, "game_audit log failed");
        }
    }
}

#[async_trait]
impl ManageGameServersUseCase for ManageGameServersService {
    async fn create(&self, cmd: CreateGameServerCommand) -> Result<GameServer, DomainError> {
        let cfg = load_game_portal_config(&self.bot_config, &cmd.guild_id).await?;
        let template = self.validate_create(&cmd, &cfg).await?;

        let memory = cmd.allocated_memory_mb.unwrap_or(template.default_memory_mb);

        // 1. Creation DB en statut `created` (pas encore de container).
        let new = NewGameServer {
            guild_id: cmd.guild_id.clone(),
            template_id: template.id,
            name: cmd.name.clone(),
            allocated_memory_mb: memory,
            owner_user_id: cmd.owner_user_id.clone(),
            idle_shutdown_days: None,
        };
        let server = self.server_repo.create(new).await?;
        let server_id = server.id;
        info!(server_id = %server_id, guild_id = %cmd.guild_id, "game_server cree (DB)");

        // 2. Configs initiales (overrides eventuels).
        if !cmd.initial_config.is_empty() {
            self.config_repo
                .replace_all(server_id, cmd.initial_config.clone(), Some(&cmd.owner_user_id))
                .await?;
        }

        // 3. Audit
        self.audit(
            &cmd.guild_id,
            Some(server_id),
            Some(&cmd.owner_user_id),
            GameAuditAction::Create,
            serde_json::json!({
                "template": template.slug,
                "memory_mb": memory,
                "name": cmd.name,
            }),
        )
        .await;

        Ok(server)
    }

    async fn list_for_guild(&self, guild_id: &str) -> Result<Vec<GameServer>, DomainError> {
        self.server_repo.list_by_guild(guild_id).await
    }

    async fn get(&self, id: Uuid) -> Result<GameServerDetail, DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;
        let config = self.config_repo.get_all(id).await?;
        Ok(GameServerDetail { server, config })
    }

    async fn delete(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;

        // Stop si actif (best-effort).
        if let Some(cid) = &server.container_id {
            if server.status.is_active() {
                if let Err(e) = self
                    .container_runtime
                    .stop_container(cid, STOP_TIMEOUT_SECS)
                    .await
                {
                    warn!(error = %e, "stop avant delete a echoue");
                }
            }
            if let Err(e) = self.container_runtime.remove_container(cid).await {
                warn!(error = %e, "remove container a echoue");
            }
        }

        // Volume
        if let Some(vol) = &server.volume_name {
            if let Err(e) = self.container_runtime.remove_volume(vol).await {
                warn!(error = %e, volume = %vol, "remove volume a echoue (peut-etre encore utilise)");
            }
        }

        // Liberation des ports
        if let Some(p) = server.host_port {
            let _ = self.port_allocator.release(PortKind::Game, p).await;
        }
        if let Some(p) = server.rcon_port {
            let _ = self.port_allocator.release(PortKind::Rcon, p).await;
        }

        self.server_repo.soft_delete(id).await?;
        self.audit(
            &server.guild_id,
            Some(id),
            Some(actor_user_id),
            GameAuditAction::Delete,
            serde_json::json!({ "name": server.name }),
        )
        .await;
        Ok(())
    }

    async fn start(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;
        if !server.status.can_start() {
            return Err(DomainError::Conflict(format!(
                "transition start invalide depuis status {:?}",
                server.status
            )));
        }
        let cfg = load_game_portal_config(&self.bot_config, &server.guild_id).await?;
        let template = self
            .template_repo
            .find_by_id(server.template_id)
            .await?
            .ok_or_else(|| {
                DomainError::Internal("template du serveur introuvable".into())
            })?;

        // Marque starting tout de suite (etat transient).
        self.server_repo
            .update_status(id, GameServerStatus::Starting, None)
            .await?;

        // Si pas de container_id encore -> create complete (alloue ports +
        // volume + container).
        let mut server = server;
        if server.container_id.is_none() {
            // Allocation ports.
            let game_port = self
                .port_allocator
                .allocate(
                    PortKind::Game,
                    cfg.port_range_start,
                    cfg.port_range_end,
                    &server.id.to_string(),
                )
                .await?;
            let rcon_port = if template.supports_rcon && cfg.rcon_enabled {
                Some(
                    self.port_allocator
                        .allocate(
                            PortKind::Rcon,
                            cfg.rcon_port_range_start,
                            cfg.rcon_port_range_end,
                            &server.id.to_string(),
                        )
                        .await?,
                )
            } else {
                None
            };
            let rcon_password = if rcon_port.is_some() {
                Some(generate_rcon_password())
            } else {
                None
            };
            let volume_name = if cfg.auto_create_world_volume {
                Some(GameServer::docker_volume_name(server.id))
            } else {
                None
            };
            let container_name = GameServer::docker_container_name(server.id);

            // Maj DB avec les ressources allouees AVANT toute call Docker
            // (en cas de crash, le reconciler peut nettoyer).
            self.server_repo
                .update_runtime(
                    id,
                    GameServerRuntimeUpdate {
                        host_port: Some(game_port),
                        rcon_port,
                        rcon_password: rcon_password.clone(),
                        volume_name: volume_name.clone(),
                        container_name: Some(container_name.clone()),
                        ..Default::default()
                    },
                )
                .await?;
            server.host_port = Some(game_port);
            server.rcon_port = rcon_port;
            server.rcon_password = rcon_password;
            server.volume_name = volume_name.clone();
            server.container_name = Some(container_name);

            // Pre-requis Docker
            self.container_runtime
                .ensure_network(&cfg.docker_network_name)
                .await?;
            if let Some(vol) = &server.volume_name {
                self.container_runtime.ensure_volume(vol).await?;
            }
            self.container_runtime
                .pull_image_if_missing(&template.image)
                .await?;

            // Build spec + create
            let overrides = self.config_repo.get_all(id).await?;
            let spec = self.build_spec(&server, &template, &overrides, &cfg);
            let cid = match self.container_runtime.create_container(&spec).await {
                Ok(cid) => cid,
                Err(e) => {
                    self.server_repo
                        .update_status(
                            id,
                            GameServerStatus::Error,
                            Some(&format!("create_container: {e}")),
                        )
                        .await?;
                    return Err(e);
                }
            };
            self.server_repo
                .update_runtime(
                    id,
                    GameServerRuntimeUpdate {
                        container_id: Some(cid.clone()),
                        ..Default::default()
                    },
                )
                .await?;
            server.container_id = Some(cid);
        }

        // Start
        let cid = server.container_id.as_ref().ok_or_else(|| {
            DomainError::Internal("container_id absent apres create".into())
        })?;

        // Init files : pour les jeux dont l'image ne genere pas elle-meme
        // ses fichiers de config (ex Terraria/ryshe + /tshock/config.json).
        // On rend les {{KEY}} a partir des env effectives (defaults +
        // overrides) et on upload chaque fichier dans le container *avant*
        // start. Reupload systematique = la modif des champs UI prend
        // effet au prochain start sans recreer le container.
        if !template.init_files.is_empty() {
            let overrides = self.config_repo.get_all(id).await.unwrap_or_default();
            let render_env = Self::render_env(&template, &overrides);
            for f in &template.init_files {
                let path = render_template(&f.path, &render_env);
                let content = render_template(&f.content, &render_env);
                if let Err(e) = self
                    .container_runtime
                    .upload_file_to_container(cid, &path, &content)
                    .await
                {
                    error!(error = %e, path = %path, "init_file upload echoue");
                    self.server_repo
                        .update_status(
                            id,
                            GameServerStatus::Error,
                            Some(&format!("init_file {path}: {e}")),
                        )
                        .await?;
                    return Err(e);
                }
            }
        }

        if let Err(e) = self.container_runtime.start_container(cid).await {
            self.server_repo
                .update_status(
                    id,
                    GameServerStatus::Error,
                    Some(&format!("start: {e}")),
                )
                .await?;
            return Err(e);
        }

        self.server_repo
            .update_runtime(
                id,
                GameServerRuntimeUpdate {
                    status: Some(GameServerStatus::Running),
                    started_at_now: true,
                    clear_last_error: true,
                    ..Default::default()
                },
            )
            .await?;

        self.audit(
            &server.guild_id,
            Some(id),
            Some(actor_user_id),
            GameAuditAction::Start,
            serde_json::json!({ "host_port": server.host_port }),
        )
        .await;
        Ok(())
    }

    async fn stop(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;
        if !server.status.can_stop() {
            return Err(DomainError::Conflict(format!(
                "transition stop invalide depuis status {:?}",
                server.status
            )));
        }
        self.server_repo
            .update_status(id, GameServerStatus::Stopping, None)
            .await?;

        if let Some(cid) = &server.container_id {
            if let Err(e) = self
                .container_runtime
                .stop_container(cid, STOP_TIMEOUT_SECS)
                .await
            {
                error!(error = %e, "stop_container failed");
                self.server_repo
                    .update_status(
                        id,
                        GameServerStatus::Error,
                        Some(&format!("stop: {e}")),
                    )
                    .await?;
                return Err(e);
            }
        }

        self.server_repo
            .update_runtime(
                id,
                GameServerRuntimeUpdate {
                    status: Some(GameServerStatus::Stopped),
                    stopped_at_now: true,
                    clear_last_error: true,
                    ..Default::default()
                },
            )
            .await?;

        self.audit(
            &server.guild_id,
            Some(id),
            Some(actor_user_id),
            GameAuditAction::Stop,
            serde_json::json!({}),
        )
        .await;
        Ok(())
    }

    async fn restart(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError> {
        // restart = stop + start. On peut deleguer a Docker direct si container existe,
        // mais on prefere passer par notre logique (audit + transitions).
        self.stop(id, actor_user_id).await?;
        self.start(id, actor_user_id).await?;
        self.audit(
            "?",
            Some(id),
            Some(actor_user_id),
            GameAuditAction::Restart,
            serde_json::json!({}),
        )
        .await;
        Ok(())
    }

    async fn get_logs(&self, id: Uuid, lines: u32) -> Result<Vec<String>, DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;
        let cid = server.container_id.ok_or_else(|| {
            DomainError::Conflict("container_id non defini (jamais demarre)".into())
        })?;
        self.container_runtime.logs(&cid, lines.min(1000)).await
    }

    async fn get_stats(&self, id: Uuid) -> Result<ContainerStats, DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;
        let cid = server.container_id.ok_or_else(|| {
            DomainError::Conflict("container_id non defini".into())
        })?;
        self.container_runtime.stats(&cid).await
    }

    async fn update_config(
        &self,
        id: Uuid,
        entries: HashMap<String, String>,
        actor_user_id: &str,
    ) -> Result<(), DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;
        // Validation des keys (SCREAMING_SNAKE_CASE) + values dans les bornes
        // declarees par le template.
        for k in entries.keys() {
            crate::domain::entities::game::config::validate_config_key(k)
                .map_err(DomainError::ValidationError)?;
        }
        self.config_repo
            .replace_all(id, entries.clone(), Some(actor_user_id))
            .await?;
        self.audit(
            &server.guild_id,
            Some(id),
            Some(actor_user_id),
            GameAuditAction::ConfigUpdate,
            serde_json::json!({ "keys_updated": entries.keys().collect::<Vec<_>>() }),
        )
        .await;
        Ok(())
    }

    async fn execute_rcon(
        &self,
        id: Uuid,
        command: &str,
        actor_user_id: &str,
    ) -> Result<String, DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;
        if server.status != GameServerStatus::Running {
            return Err(DomainError::Conflict(
                "le serveur doit etre running pour RCON".into(),
            ));
        }
        let cfg = load_game_portal_config(&self.bot_config, &server.guild_id).await?;
        if !cfg.rcon_enabled {
            return Err(DomainError::Forbidden("RCON desactive".into()));
        }
        let port = server
            .rcon_port
            .ok_or_else(|| DomainError::Conflict("rcon_port non alloue".into()))?;
        let pwd = server
            .rcon_password
            .ok_or_else(|| DomainError::Conflict("rcon_password non defini".into()))?;
        let params = RconConnectionParams {
            host: "127.0.0.1".to_string(),
            port,
            password: pwd,
            timeout_secs: 5,
        };
        let resp = self.rcon_client.execute(&params, command).await?;
        self.audit(
            &server.guild_id,
            Some(id),
            Some(actor_user_id),
            GameAuditAction::CommandRcon,
            serde_json::json!({ "cmd": command }),
        )
        .await;
        Ok(resp.raw)
    }
}
