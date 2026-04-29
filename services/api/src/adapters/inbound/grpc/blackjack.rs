//! Implementation gRPC du `BlackjackService`. Wrappe le `BlackjackService`
//! application + le `WalletRepository` (pour `get_wallet`).
//!
//! La logique de lecture de la config guild (min/max bet, starting coins,
//! payout) est dupliquee depuis le handler HTTP `handlers/blackjack/game.rs`
//! pour rester aligne. Les broadcasts WS `blackjack_result` sont aussi
//! emis a la fin de chaque partie pour ne rien casser cote dashboard.

use std::sync::Arc;

use tonic::Request;
use tonic::Response;
use tonic::Status;
use sentinel_proto::blackjack::v1 as proto;
use sentinel_proto::blackjack::v1::blackjack_service_server::BlackjackService as BlackjackGrpcTrait;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::application::casino::blackjack_service::BlackjackService as BlackjackApp;
use crate::domain::entities::casino::blackjack::BlackjackGame;
use crate::domain::entities::casino::blackjack::Card;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::entities::casino::wallet::Wallet;
use crate::ports::outbound::casino::blackjack_table_repository::BlackjackTableRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
pub struct BlackjackGrpc {
    pub svc: Arc<BlackjackApp>,
    pub wallet_repo: Arc<dyn WalletRepository>,
    pub bot_config_repo: Arc<dyn BotConfigRepository>,
    pub table_repo: Arc<dyn BlackjackTableRepository>,
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
    ) -> Result<Response<proto::BlackjackGameResult>, Status> {
        let req = request.into_inner();
        let (min_bet, max_bet, starting_coins, blackjack_payout) = self.config(&req.guild_id).await;
        let result = self
            .svc
            .start_game(
                req.guild_id,
                req.user_id.into(),
                req.username,
                req.bet,
                min_bet,
                max_bet,
                starting_coins,
                blackjack_payout,
            )
            .await
            .map_err(domain_to_status)?;
        // Anti-AFK : empeche le cleanup-worker de fermer la table pendant
        // que le joueur joue. No-op si solo (pas de table).
        let _ = self
            .table_repo
            .touch_activity_by_player(&result.game.guild_id, &result.game.user_id)
            .await;
        if game_is_over(&result.game.status) {
            self.broadcast_result(&result.game, false);
        }
        Ok(Response::new(action_result_to_proto(result)))
    }

    async fn hit(
        &self,
        request: Request<proto::GameIdRequest>,
    ) -> Result<Response<proto::BlackjackGameResult>, Status> {
        let id = parse_uuid(&request.into_inner().game_id)?;
        let result = self.svc.hit(id).await.map_err(domain_to_status)?;
        let _ = self
            .table_repo
            .touch_activity_by_player(&result.game.guild_id, &result.game.user_id)
            .await;
        if game_is_over(&result.game.status) {
            self.broadcast_result(&result.game, false);
        }
        Ok(Response::new(action_result_to_proto(result)))
    }

    async fn stand(
        &self,
        request: Request<proto::GameIdRequest>,
    ) -> Result<Response<proto::BlackjackGameResult>, Status> {
        let id = parse_uuid(&request.into_inner().game_id)?;
        let result = self.svc.stand(id).await.map_err(domain_to_status)?;
        let _ = self
            .table_repo
            .touch_activity_by_player(&result.game.guild_id, &result.game.user_id)
            .await;
        if game_is_over(&result.game.status) {
            self.broadcast_result(&result.game, false);
        }
        Ok(Response::new(action_result_to_proto(result)))
    }

    async fn double_down(
        &self,
        request: Request<proto::GameIdRequest>,
    ) -> Result<Response<proto::BlackjackGameResult>, Status> {
        let id = parse_uuid(&request.into_inner().game_id)?;
        let result = self.svc.double_down(id).await.map_err(domain_to_status)?;
        let _ = self
            .table_repo
            .touch_activity_by_player(&result.game.guild_id, &result.game.user_id)
            .await;
        if game_is_over(&result.game.status) {
            self.broadcast_result(&result.game, true);
        }
        Ok(Response::new(action_result_to_proto(result)))
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

use super::parse_uuid;

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
        user_id: g.user_id.into(),
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

fn taunt_to_proto(t: TauntEvent) -> proto::TauntEvent {
    proto::TauntEvent {
        channel_id: t.channel_id.into(),
        target_user_id: t.target_user_id,
        message: t.message,
        nickname_suffix: t.nickname_suffix,
        streak_kind: t.streak_kind.to_string(),
        streak_value: t.streak_value,
    }
}

fn action_result_to_proto(
    result: crate::application::casino::blackjack_service::BlackjackActionResult,
) -> proto::BlackjackGameResult {
    proto::BlackjackGameResult {
        game: Some(blackjack_game_to_proto(result.game)),
        taunt_events: result.taunt_events.into_iter().map(taunt_to_proto).collect(),
        wallet_balance: result.wallet_balance,
    }
}

fn wallet_to_proto(w: Wallet) -> proto::Wallet {
    proto::Wallet {
        id: w.id.to_string(),
        guild_id: w.guild_id,
        user_id: w.user_id.into(),
        username: w.username,
        coins: w.coins,
        total_earned: w.total_earned,
        total_spent: w.total_spent,
    }
}


#[cfg(test)]
#[path = "tests/blackjack.rs"]
mod tests;
