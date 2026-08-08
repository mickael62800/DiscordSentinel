//! # nexus-proto — Définitions protobuf/gRPC de la plateforme jeux Nexus

pub mod game {
    pub mod v1 {
        tonic::include_proto!("nexus.game.v1");
    }
}
