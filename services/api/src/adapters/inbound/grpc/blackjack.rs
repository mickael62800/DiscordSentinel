//! Implementation gRPC du `BlackjackService`. Wrappe le `BlackjackService`
//! application + le `WalletRepository` (pour `get_wallet`).
//!
//! La logique de lecture de la config guild (min/max bet, starting coins,
//! payout) est dupliquee depuis le handler HTTP `handlers/blackjack/game.rs`
//! pour rester aligne. Les broadcasts WS `blackjack_result` sont aussi
//! emis a la fin de chaque partie pour ne rien casser cote dashboard.

use std::sync::Arc;
use std::str::FromStr;

use tonic::{Request, Response, Status};
use uuid::Uuid;

use sentinel_proto::blackjack::v1 as proto;
use sentinel_proto::blackjack::v1::blackjack_service_server::BlackjackService as BlackjackGrpcTrait;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::application::BlackjackService as BlackjackApp;
use crate::domain::entities::{BlackjackGame, Card, Wallet};
use crate::ports::outbound::{BotConfigRepository, WalletRepository};

pub struct BlackjackGrpc {
    pub svc: Arc<BlackjackApp>,
    pub wallet_repo: Arc<dyn WalletRepository>,
    pub bot_config_repo: Arc<dyn BotConfigRepository>,
    pub broadcaster: Arc<EventBroadcaster>,
}

impl BlackjackGrpc {
    fn broadcast_result(&self, game: &BlackjackGame, doubled: bool) {
        let mut payload = serde_json::json!({
            "guild_id": game.guild_id,
            "user_id": game.user_id,
            "username": game.username,
            "status": game.status,
            "payout": game.payout,
            "bet": game.bet,
        });
        if doubled {
            payload["doubled"] = serde_json::Value::Bool(true);
        }
        self.broadcaster.broadcast("blackjack_result", payload);
    }

    async fn config(&self, guild_id: &str) -> (i64, i64, i64, f64) {
        let cfg = self
            .bot_config_repo
            .get_config(guild_id, "blackjack-bot")
            .await
            .unwrap_or_default();
        let min_bet = cfg
            .iter()
            .find(|c| c.config_key == "min_bet")
            .and_then(|c| c.config_value.parse().ok())
            .unwrap_or(10);
        let max_bet = cfg
            .iter()
            .find(|c| c.config_key == "max_bet")
            .and_then(|c| c.config_value.parse().ok())
            .unwrap_or(1000);
        let starting_coins = cfg
            .iter()
            .find(|c| c.config_key == "starting_coins")
            .and_then(|c| c.config_value.parse().ok())
            .unwrap_or(200);
        let blackjack_payout: f64 = cfg
            .iter()
            .find(|c| c.config_key == "blackjack_payout")
            .and_then(|c| c.config_value.parse().ok())
            .unwrap_or(1.5);
        (min_bet, max_bet, starting_coins, blackjack_payout)
    }
}

#[tonic::async_trait]
impl BlackjackGrpcTrait for BlackjackGrpc {
    async fn start_game(
        &self,
        request: Request<proto::StartGameRequest>,
    ) -> Result<Response<proto::BlackjackGame>, Status> {
        let req = request.into_inner();
        let (min_bet, max_bet, starting_coins, blackjack_payout) = self.config(&req.guild_id).await;
        let game = self
            .svc
            .start_game(
                req.guild_id,
                req.user_id,
                req.username,
                req.bet,
                min_bet,
                max_bet,
                starting_coins,
                blackjack_payout,
            )
            .await
            .map_err(domain_to_status)?;
        if game_is_over(&game.status) {
            self.broadcast_result(&game, false);
        }
        Ok(Response::new(blackjack_game_to_proto(game)))
    }

    async fn hit(
        &self,
        request: Request<proto::GameIdRequest>,
    ) -> Result<Response<proto::BlackjackGame>, Status> {
        let id = parse_uuid(&request.into_inner().game_id)?;
        let game = self.svc.hit(id).await.map_err(domain_to_status)?;
        if game_is_over(&game.status) {
            self.broadcast_result(&game, false);
        }
        Ok(Response::new(blackjack_game_to_proto(game)))
    }

    async fn stand(
        &self,
        request: Request<proto::GameIdRequest>,
    ) -> Result<Response<proto::BlackjackGame>, Status> {
        let id = parse_uuid(&request.into_inner().game_id)?;
        let game = self.svc.stand(id).await.map_err(domain_to_status)?;
        if game_is_over(&game.status) {
            self.broadcast_result(&game, false);
        }
        Ok(Response::new(blackjack_game_to_proto(game)))
    }

    async fn double_down(
        &self,
        request: Request<proto::GameIdRequest>,
    ) -> Result<Response<proto::BlackjackGame>, Status> {
        let id = parse_uuid(&request.into_inner().game_id)?;
        let game = self.svc.double_down(id).await.map_err(domain_to_status)?;
        if game_is_over(&game.status) {
            self.broadcast_result(&game, true);
        }
        Ok(Response::new(blackjack_game_to_proto(game)))
    }

    async fn get_active(
        &self,
        request: Request<proto::GetActiveRequest>,
    ) -> Result<Response<proto::GetActiveResponse>, Status> {
        let req = request.into_inner();
        let game = self
            .svc
            .get_active(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::GetActiveResponse {
            game: game.map(blackjack_game_to_proto),
        }))
    }

    async fn get_wallet(
        &self,
        request: Request<proto::GetWalletRequest>,
    ) -> Result<Response<proto::Wallet>, Status> {
        let req = request.into_inner();
        let wallet = self
            .wallet_repo
            .get(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?
            .ok_or_else(|| Status::not_found("Wallet introuvable"))?;
        Ok(Response::new(wallet_to_proto(wallet)))
    }
}

fn game_is_over(status: &str) -> bool {
    matches!(
        status,
        "player_blackjack" | "player_bust" | "dealer_bust" | "player_win" | "dealer_win" | "push"
    )
}

fn parse_uuid(s: &str) -> Result<Uuid, Status> {
    Uuid::from_str(s).map_err(|_| Status::invalid_argument("game_id n'est pas un UUID valide"))
}

fn card_to_proto(c: &Card) -> proto::Card {
    proto::Card {
        rank: c.rank.clone(),
        suit: c.suit.clone(),
        filename: c.filename(),
    }
}

fn blackjack_game_to_proto(g: BlackjackGame) -> proto::BlackjackGame {
    proto::BlackjackGame {
        id: g.id.to_string(),
        guild_id: g.guild_id,
        user_id: g.user_id,
        username: g.username,
        bet: g.bet,
        player_hand: g.player_hand.iter().map(card_to_proto).collect(),
        dealer_hand: g.dealer_hand.iter().map(card_to_proto).collect(),
        status: g.status,
        player_score: g.player_score,
        dealer_score: g.dealer_score,
        doubled: g.doubled,
        payout: g.payout,
        created_at: g.created_at.to_rfc3339(),
        finished_at: g.finished_at.map(|d| d.to_rfc3339()),
    }
}

fn wallet_to_proto(w: Wallet) -> proto::Wallet {
    proto::Wallet {
        id: w.id.to_string(),
        guild_id: w.guild_id,
        user_id: w.user_id,
        username: w.username,
        coins: w.coins,
        total_earned: w.total_earned,
        total_spent: w.total_spent,
    }
}
