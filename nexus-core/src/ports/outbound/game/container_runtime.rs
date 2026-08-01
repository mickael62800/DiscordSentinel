//! Container Runtime — port abstrait pilotant un Docker daemon (ou autre).
//!
//! Volontairement minimaliste : on n'expose que ce qui est strictement
//! necessaire au game portal. Pas de `exec arbitraire`, pas de `mount` host
//! configurable depuis le caller, pas de `--privileged`. Toutes les
//! contraintes de securite sont **dans la signature** : si une option
//! dangereuse n'est pas exposee ici, l'application ne peut pas la passer.

use async_trait::async_trait;
use std::collections::HashMap;

use crate::domain::errors::DomainError;

/// Specification d'un container a creer. Genere par le use case a partir
/// du template + config + ports alloues. Le runtime fait juste l'execution.
#[derive(Debug, Clone)]
pub struct ContainerSpec {
    /// Image Docker (whitelist verifiee par le caller).
    pub image: String,
    /// Nom du container Docker (unique).
    pub name: String,
    /// Variables d'environnement complete'es (defaults + overrides).
    pub env: HashMap<String, String>,
    /// Port mappings host_port -> container_port.
    pub port_mappings: Vec<PortMapping>,
    /// Volume nomme a monter sur un point de montage interne.
    /// Format : (volume_name, container_path).
    pub volumes: Vec<VolumeMount>,
    /// Memoire max (bytes). Hard-limit Docker.
    pub memory_bytes: u64,
    /// Plafond CPU en nombre de coeurs (2.0 = deux coeurs pleins). None =
    /// plafond par defaut de l'adapter.
    ///
    /// C'est une PROTECTION, pas un accelerateur : un serveur ne va pas plus
    /// vite parce qu'on lui donne des coeurs, mais un serveur emballe ne peut
    /// plus asphyxier les autres ni la base de donnees.
    pub cpu_limit: Option<f64>,
    /// Network name (cree au boot si absent).
    pub network: String,
    /// User non-root applique (--user UID:GID). None = laisse le default de l'image.
    pub user: Option<String>,
    /// Restart policy : "no" (par defaut), "on-failure:N".
    pub restart_policy: RestartPolicy,
    /// Labels Docker pour traçabilite (sentinel.* — lecture par le reconciler).
    pub labels: HashMap<String, String>,
    /// Override de la commande Docker (Cmd). None = laisse l'image decider.
    /// Cas d'usage : Terraria/ryshe ou il faut passer -autocreate, -world,
    /// etc. via flags CLI car l'image ne lit pas tout depuis l'env.
    pub command: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: PortProtocol,
    /// Adresse host sur laquelle binder le port. "0.0.0.0" pour un port
    /// jeu (exposé au reseau), "127.0.0.1" pour un port d'admin (RCON) qui
    /// ne doit etre joignable que depuis l'app locale.
    pub host_ip: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortProtocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone)]
pub struct VolumeMount {
    pub volume_name: String,
    pub container_path: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy {
    None,
    OnFailure(u32),
}

/// Etat observe d'un container (par inspect).
#[derive(Debug, Clone)]
pub struct ContainerStatus {
    pub container_id: String,
    pub state: ContainerState,
    pub exit_code: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerState {
    Created,
    Running,
    Restarting,
    Paused,
    Exited,
    Dead,
}

/// Stats temps-reel d'un container.
#[derive(Debug, Clone, Default)]
pub struct ContainerStats {
    pub cpu_percent: f64,
    pub memory_used_bytes: u64,
    pub memory_limit_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    /// Ce runtime peut-il reellement piloter des conteneurs ?
    ///
    /// `false` pour l'implementation de repli, utilisee quand le socket
    /// Docker est indisponible. Elle laisse le listing et la configuration
    /// fonctionner mais echoue sur toute operation de cycle de vie.
    ///
    /// Permet de REFUSER une creation d'emblee au lieu de laisser fabriquer
    /// un serveur qui ne demarrera jamais. Par defaut `true` : un runtime qui
    /// ne se declare pas est suppose fonctionnel.
    fn is_operational(&self) -> bool {
        true
    }

    /// Cree le network dedie (idempotent).
    async fn ensure_network(&self, name: &str) -> Result<(), DomainError>;

    /// Cree le volume nomme (idempotent).
    async fn ensure_volume(&self, name: &str) -> Result<(), DomainError>;

    /// Pull l'image si absente. Bloquant.
    async fn pull_image_if_missing(&self, image: &str) -> Result<(), DomainError>;

    /// Cree le container. Retourne son id Docker. Ne le demarre PAS.
    async fn create_container(&self, spec: &ContainerSpec) -> Result<String, DomainError>;

    /// Demarre un container existant.
    async fn start_container(&self, container_id: &str) -> Result<(), DomainError>;

    /// Pose un fichier (utf-8) sur le filesystem du container, a un chemin
    /// absolu. A appeler entre `create_container` et `start_container` —
    /// les volumes nommes sont deja montes a ce stade. Les sous-repertoires
    /// inexistants sont crees. Permet de seed des fichiers de config que
    /// l'image ne genere pas elle-meme (ex : ryshe/terraria + config.json).
    async fn upload_file_to_container(
        &self,
        container_id: &str,
        path: &str,
        content: &str,
    ) -> Result<(), DomainError>;

    /// Arrete proprement (SIGTERM puis SIGKILL apres `timeout_secs`).
    async fn stop_container(
        &self,
        container_id: &str,
        timeout_secs: u32,
    ) -> Result<(), DomainError>;

    /// Restart (= stop + start, geres en interne).
    async fn restart_container(
        &self,
        container_id: &str,
        timeout_secs: u32,
    ) -> Result<(), DomainError>;

    /// Supprime le container (force).
    async fn remove_container(&self, container_id: &str) -> Result<(), DomainError>;

    /// Supprime un volume nomme (ne casse rien si en cours d'utilisation,
    /// retourne une erreur dans ce cas).
    async fn remove_volume(&self, name: &str) -> Result<(), DomainError>;

    /// Supprime une image Docker. Retourne true si supprimee, false si
    /// l'image n'existait pas / etait encore utilisee. force=true tente
    /// la suppression meme si des containers stoppes l'utilisent.
    async fn remove_image(&self, image: &str, force: bool) -> Result<bool, DomainError>;

    /// Inspect : retourne le status courant.
    async fn inspect(&self, container_id: &str) -> Result<Option<ContainerStatus>, DomainError>;

    /// Stats (snapshot one-shot, pas un stream).
    async fn stats(&self, container_id: &str) -> Result<ContainerStats, DomainError>;

    /// Logs (last N lines), pas de follow.
    async fn logs(&self, container_id: &str, lines: u32) -> Result<Vec<String>, DomainError>;

    /// Liste tous les containers managed (filtre par label `sentinel.managed=true`).
    /// Pour le reconciler.
    async fn list_managed_containers(&self) -> Result<Vec<ManagedContainer>, DomainError>;
}

/// Container detecte par le reconciler.
#[derive(Debug, Clone)]
pub struct ManagedContainer {
    pub container_id: String,
    pub name: String,
    pub state: ContainerState,
    pub labels: HashMap<String, String>,
}
