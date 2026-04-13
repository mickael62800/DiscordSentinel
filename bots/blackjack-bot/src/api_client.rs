//! Client API du blackjack-bot.
//!
//! Phase 7A — Migration gRPC :
//! - Solo : start_game, hit, stand, double_down, get_active -> gRPC
//! - Wallet : get_wallet -> gRPC
//! - Tables multijoueur (create/join/get/list/close) -> HTTP retenu
//!   (use cases dedies pas encore exposes en proto v1).
//!
//! Surface publique inchangee : `handler.rs` et `commands/*` continuent
//! d'appeler les memes methodes.
//!
//! ## Comportement si l'API tombe
//!
//! Tous les appels gRPC passent par le circuit breaker (5 echecs ->
//! ouverture 10s). Pendant l'ouverture :
//! - `start_game` retourne `Err("API indisponible")` -> commande slash
//!   repond a l'utilisateur, aucune mise n'est debitee silencieusement.
//! - `hit/stand/double_down` retournent l'erreur ; le joueur peut retenter
//!   son action (la partie reste dans l'etat precedent cote BDD).
//! - `get_active` permet de detecter les parties orphelines au demarrage.
//! - `get_wallet` echoue, l'embed wallet affiche un message d'erreur.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::grpc_client::{GrpcCallError, SentinelGrpcClient};

use sentinel_proto::blackjack::v1 as proto;

// ── Response DTOs (surface inchangee) ──

#[derive(Debug, Clone, Deserialize)]
pub struct CardDto {
    pub rank: String,
    pub suit: String,
    pub filename: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlackjackGameDto {
    pub id: String,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub bet: i64,
    pub player_hand: Vec<CardDto>,
    pub dealer_hand: Vec<CardDto>,
    pub status: String,
    pub player_score: i32,
    pub dealer_score: i32,
    pub doubled: bool,
    pub payout: i64,
    pub created_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WalletDto {
    pub id: String,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub coins: i64,
    pub total_earned: i64,
    pub total_spent: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TableDto {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub owner_id: String,
    pub owner_name: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TablePlayerDto {
    pub user_id: String,
    pub user_name: String,
}

// ── API Client ──

#[derive(Clone)]
pub struct ApiClient {
    pub base: Arc<BaseApiClient>,
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>, grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { base, grpc }
    }

    // ── Blackjack solo (gRPC) ──

    pub async fn start_game(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        bet: i64,
    ) -> Result<BlackjackGameDto, String> {
        let req = proto::StartGameRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            bet,
        };
        let mut client = self.grpc.blackjack();
        let game = self
            .grpc
            .guarded(|| async move { client.start_game(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(proto_game_to_dto(game))
    }

    pub async fn hit(&self, game_id: &str) -> Result<BlackjackGameDto, String> {
        self.game_action(game_id, BlackjackAction::Hit).await
    }

    pub async fn stand(&self, game_id: &str) -> Result<BlackjackGameDto, String> {
        self.game_action(game_id, BlackjackAction::Stand).await
    }

    pub async fn double_down(&self, game_id: &str) -> Result<BlackjackGameDto, String> {
        self.game_action(game_id, BlackjackAction::Double).await
    }

    async fn game_action(
        &self,
        game_id: &str,
        action: BlackjackAction,
    ) -> Result<BlackjackGameDto, String> {
        let req = proto::GameIdRequest {
            game_id: game_id.to_string(),
        };
        let mut client = self.grpc.blackjack();
        let game = self
            .grpc
            .guarded(|| async move {
                let result = match action {
                    BlackjackAction::Hit => client.hit(req).await,
                    BlackjackAction::Stand => client.stand(req).await,
                    BlackjackAction::Double => client.double_down(req).await,
                };
                result.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(proto_game_to_dto(game))
    }

    pub async fn get_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<BlackjackGameDto>, String> {
        let req = proto::GetActiveRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.blackjack();
        let resp = self
            .grpc
            .guarded(|| async move { client.get_active(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(resp.game.map(proto_game_to_dto))
    }

    // ── Wallet (gRPC) ──

    pub async fn get_wallet(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<WalletDto, String> {
        let req = proto::GetWalletRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.blackjack();
        let w = self
            .grpc
            .guarded(|| async move { client.get_wallet(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(WalletDto {
            id: w.id,
            guild_id: w.guild_id,
            user_id: w.user_id,
            username: w.username,
            coins: w.coins,
            total_earned: w.total_earned,
            total_spent: w.total_spent,
        })
    }

    // ── Tables (HTTP retenu — pas de RPC v1) ──

    pub async fn create_table(
        &self,
        guild_id: &str,
        channel_id: &str,
        owner_id: &str,
        owner_name: &str,
    ) -> Result<TableDto, String> {
        self.base
            .post_json(
                "/api/blackjack/tables",
                &serde_json::json!({
                    "guild_id": guild_id,
                    "channel_id": channel_id,
                    "owner_id": owner_id,
                    "owner_name": owner_name,
                }),
            )
            .await
    }

    pub async fn join_table(
        &self,
        table_id: &str,
        user_id: &str,
        user_name: &str,
    ) -> Result<(), String> {
        // Avant : post_fire_and_forget + Ok(()) -> le bot confirmait "inscrit"
        // quel que soit le status HTTP. Desormais on verifie explicitement.
        let url = format!(
            "{}/api/blackjack/tables/{}/join",
            self.base.base_url(),
            table_id
        );
        let req = self
            .base
            .client()
            .post(url)
            .json(&serde_json::json!({ "user_id": user_id, "user_name": user_name }));
        let resp = self.base.auth(req).send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("API error: {}", resp.status()));
        }
        Ok(())
    }

    pub async fn get_table_by_channel(
        &self,
        channel_id: &str,
    ) -> Result<Option<TableDto>, String> {
        self.base
            .get_json(&format!("/api/blackjack/tables/by-channel/{channel_id}"))
            .await
    }

    pub async fn list_table_players(
        &self,
        table_id: &str,
    ) -> Result<Vec<TablePlayerDto>, String> {
        self.base
            .get_json(&format!("/api/blackjack/tables/{table_id}/players"))
            .await
    }

    pub async fn close_table(&self, table_id: &str) -> Result<(), String> {
        let req = self.base.client().delete(format!(
            "{}/api/blackjack/tables/{}",
            self.base.base_url(),
            table_id
        ));
        let resp = self.base.auth(req).send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("API error: {}", resp.status()));
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum BlackjackAction {
    Hit,
    Stand,
    Double,
}

// ── Helpers ──

fn proto_game_to_dto(g: proto::BlackjackGame) -> BlackjackGameDto {
    BlackjackGameDto {
        id: g.id,
        guild_id: g.guild_id,
        user_id: g.user_id,
        username: g.username,
        bet: g.bet,
        player_hand: g
            .player_hand
            .into_iter()
            .map(|c| CardDto {
                rank: c.rank,
                suit: c.suit,
                filename: c.filename,
            })
            .collect(),
        dealer_hand: g
            .dealer_hand
            .into_iter()
            .map(|c| CardDto {
                rank: c.rank,
                suit: c.suit,
                filename: c.filename,
            })
            .collect(),
        status: g.status,
        player_score: g.player_score,
        dealer_score: g.dealer_score,
        doubled: g.doubled,
        payout: g.payout,
        created_at: g.created_at,
        finished_at: g.finished_at,
    }
}

fn grpc_err_to_string(e: GrpcCallError) -> String {
    match e {
        GrpcCallError::Unavailable => {
            "API indisponible, reessaie dans quelques instants.".to_string()
        }
        GrpcCallError::Status(s) => {
            // On nettoie les prefixes techniques cote domaine API pour que
            // le message affiche au joueur soit directement lisible.
            let raw = s.message();
            let clean = raw
                .trim_start_matches("Données invalides : ")
                .trim_start_matches("Donnees invalides : ")
                .trim_start_matches("Conflit : ")
                .trim_start_matches("Introuvable : ");
            match s.code() {
                tonic::Code::InvalidArgument | tonic::Code::AlreadyExists | tonic::Code::NotFound => {
                    clean.to_string()
                }
                tonic::Code::Unauthenticated | tonic::Code::PermissionDenied => {
                    format!("Accès refusé : {clean}")
                }
                tonic::Code::Unavailable | tonic::Code::DeadlineExceeded => {
                    "API temporairement indisponible, reessaie.".to_string()
                }
                _ => format!("Erreur API : {clean}"),
            }
        }
        GrpcCallError::Transport(_) => "Erreur de connexion à l'API, reessaie.".to_string(),
    }
}

// Garde-fou pour eviter un warning unused sur le derive Serialize.
#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct StartGamePayload {
    guild_id: String,
    user_id: String,
    username: String,
    bet: i64,
}
