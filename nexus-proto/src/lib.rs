//! # nexus-proto — definitions protobuf/gRPC de la plateforme jeux Nexus
//!
//! Stub volontaire : aucun fichier `.proto` n'est encore defini, donc pas de
//! `build.rs` tonic-build pour l'instant. Quand les premiers protos Nexus
//! arriveront, reprendre exactement le modele de `sentinel-proto`
//! (`build.rs` avec fallback `protoc-bin-vendored`, un sous-module Rust par
//! package proto via `tonic::include_proto!`).
