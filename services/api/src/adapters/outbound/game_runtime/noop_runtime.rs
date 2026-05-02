//! Fallback : ContainerRuntime qui retourne Internal a chaque appel.
//! Utilise quand le socket Docker n'est pas dispo (dev local sans Docker).
//! Les endpoints de listing / detail continuent de fonctionner ; seules
//! les operations qui exigent Docker echouent proprement.

use async_trait::async_trait;

use crate::domain::errors::DomainError;
use crate::ports::outbound::game::container_runtime::{
    ContainerRuntime, ContainerSpec, ContainerStats, ContainerStatus, ManagedContainer,
};

pub struct NoopContainerRuntime;

fn err() -> DomainError {
    DomainError::Internal("Docker socket indisponible (Game Portal desactive)".into())
}

#[async_trait]
impl ContainerRuntime for NoopContainerRuntime {
    async fn ensure_network(&self, _: &str) -> Result<(), DomainError> { Err(err()) }
    async fn ensure_volume(&self, _: &str) -> Result<(), DomainError> { Err(err()) }
    async fn pull_image_if_missing(&self, _: &str) -> Result<(), DomainError> { Err(err()) }
    async fn create_container(&self, _: &ContainerSpec) -> Result<String, DomainError> { Err(err()) }
    async fn start_container(&self, _: &str) -> Result<(), DomainError> { Err(err()) }
    async fn upload_file_to_container(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Err(err()) }
    async fn stop_container(&self, _: &str, _: u32) -> Result<(), DomainError> { Err(err()) }
    async fn restart_container(&self, _: &str, _: u32) -> Result<(), DomainError> { Err(err()) }
    async fn remove_container(&self, _: &str) -> Result<(), DomainError> { Err(err()) }
    async fn remove_volume(&self, _: &str) -> Result<(), DomainError> { Err(err()) }
    async fn remove_image(&self, _: &str, _: bool) -> Result<bool, DomainError> { Ok(false) }
    async fn inspect(&self, _: &str) -> Result<Option<ContainerStatus>, DomainError> { Ok(None) }
    async fn stats(&self, _: &str) -> Result<ContainerStats, DomainError> { Err(err()) }
    async fn logs(&self, _: &str, _: u32) -> Result<Vec<String>, DomainError> { Err(err()) }
    async fn list_managed_containers(&self) -> Result<Vec<ManagedContainer>, DomainError> { Ok(vec![]) }
}
