use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::game::server::{GameServer, GameServerStatus};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait GameServerRepository: Send + Sync {
    /// Insere une ligne en statut `created`. Retourne l'entite avec id genere.
    async fn create(&self, server: NewGameServer) -> Result<GameServer, DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<GameServer>, DomainError>;
    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<GameServer>, DomainError>;
    async fn list_running(&self) -> Result<Vec<GameServer>, DomainError>;
    async fn list_active(&self) -> Result<Vec<GameServer>, DomainError>;

    /// Maj champs critiques (status, container_id, ports, volume) — atomique.
    async fn update_runtime(
        &self,
        id: Uuid,
        update: GameServerRuntimeUpdate,
    ) -> Result<(), DomainError>;

    async fn update_status(
        &self,
        id: Uuid,
        status: GameServerStatus,
        last_error: Option<&str>,
    ) -> Result<(), DomainError>;

    /// Transition de statut ATOMIQUE conditionnelle. Passe le serveur de
    /// l'un des etats `from` vers `to` en une seule requete
    /// (`UPDATE ... WHERE id = $1 AND status = ANY(from)`). Retourne `true`
    /// si la ligne a bien ete mise a jour (claim reussi), `false` si le
    /// statut courant n'etait dans aucun des `from` (quelqu'un d'autre a
    /// deja pris la transition / etat incompatible). Sert de verrou contre
    /// les start/stop concurrents.
    async fn try_transition_status(
        &self,
        id: Uuid,
        from: &[GameServerStatus],
        to: GameServerStatus,
    ) -> Result<bool, DomainError>;

    async fn update_player_activity(&self, id: Uuid, player_count: i32) -> Result<(), DomainError>;

    /// Enregistre une tentative de redemarrage auto : incremente
    /// `restart_attempts` et pose `last_restart_at = NOW()`. Sert au backoff.
    async fn record_restart_attempt(&self, id: Uuid) -> Result<(), DomainError>;

    /// Remet `restart_attempts` a 0 (serveur recupere). No-op si deja a 0.
    async fn reset_restart_attempts(&self, id: Uuid) -> Result<(), DomainError>;

    /// Soft-delete (status = deleted, deleted_at = NOW()).
    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Compte les serveurs actifs (non-deleted) d'une guild + leur memoire totale.
    /// Pour le calcul de quota.
    async fn count_active_for_guild(&self, guild_id: &str) -> Result<(i32, i32), DomainError>;

    /// Pour un template donne, retourne (nb_servers_actifs, derniere_activite).
    /// derniere_activite = MAX(updated_at) sur tous les serveurs (incluant
    /// soft-deleted) qui ont utilise ce template. Utilise par le job
    /// image-cleanup pour decider si l'image Docker peut etre supprimee.
    async fn template_usage(&self, template_id: uuid::Uuid) -> Result<TemplateUsage, DomainError>;

    /// Enregistre les salons Discord (texte + vocal) crees pour la session.
    async fn set_session_channels(
        &self,
        id: Uuid,
        text_channel_id: Option<&str>,
        voice_channel_id: Option<&str>,
    ) -> Result<(), DomainError>;

    /// Marque l'IP comme revelee (le job de revelation l'a publiee).
    async fn mark_ip_revealed(&self, id: Uuid) -> Result<(), DomainError>;
}

#[derive(Debug, Clone)]
pub struct TemplateUsage {
    pub active_count: i32,
    pub last_activity_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Donnees pour creer un nouveau serveur.
#[derive(Debug, Clone)]
pub struct NewGameServer {
    pub guild_id: String,
    pub template_id: Uuid,
    pub name: String,
    pub allocated_memory_mb: i32,
    pub owner_user_id: String,
    pub idle_shutdown_days: Option<i32>,
}

/// Maj des champs runtime (apres allocation Docker).
#[derive(Debug, Clone, Default)]
pub struct GameServerRuntimeUpdate {
    pub status: Option<GameServerStatus>,
    pub container_id: Option<String>,
    pub container_name: Option<String>,
    pub host_port: Option<u16>,
    pub rcon_port: Option<u16>,
    pub rcon_password: Option<String>,
    pub volume_name: Option<String>,
    pub started_at_now: bool,
    pub stopped_at_now: bool,
    pub last_error: Option<String>,
    pub clear_last_error: bool,
}
