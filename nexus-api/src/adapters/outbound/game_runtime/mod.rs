//! Adapters Game Portal — implementations infra des ports outbound game.
//!
//! - `docker_runtime` : impl ContainerRuntime via bollard (Docker socket).
//! - `rcon_minecraft` : impl RconClient via crate `rcon`.
//! - `redis_port_allocator` : impl PortAllocator via Redis SETNX.

pub mod docker_runtime;
pub mod noop_runtime;
pub mod rcon_minecraft;
pub mod rcon_pooled;
pub mod redis_port_allocator;
