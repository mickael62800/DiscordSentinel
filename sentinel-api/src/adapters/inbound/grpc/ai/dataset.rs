//! gRPC AiDataset — collecte des messages texte pour l'entrainement IA.
//!
//! Copie du pattern du handler HTTP `ai/dataset.rs::collect_message` : sqlx
//! direct car pas de use case unifie cote API (simple ingestion best-effort).
//! Remplace `POST /api/ai-dataset/collect`, chemin le plus chaud du bot
//! (un appel par message non-bot des guilds ou le module est actif).

use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::adapters::inbound::grpc::errors::sqlx_to_status;

use sentinel_proto::ai_dataset::v1 as proto;
use sentinel_proto::ai_dataset::v1::ai_dataset_service_server::AiDatasetService;
use sentinel_proto::common::v1::Empty;

pub struct AiDatasetGrpc {
    pub pg_pool: sqlx::PgPool,
}

#[tonic::async_trait]
impl AiDatasetService for AiDatasetGrpc {
    async fn collect_message(
        &self,
        request: Request<proto::CollectMessageRequest>,
    ) -> Result<Response<Empty>, Status> {
        let dto = request.into_inner();

        if dto.guild_id.trim().is_empty() || dto.user_id.trim().is_empty() {
            return Err(Status::invalid_argument("guild_id et user_id requis"));
        }
        // Best-effort : un message vide n'est pas une erreur, on ignore.
        if dto.content.trim().is_empty() {
            return Ok(Response::new(Empty {}));
        }

        sqlx::query(
            "INSERT INTO ai_dataset_messages (guild_id, channel_id, channel_name, user_id, content) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&dto.guild_id)
        .bind(&dto.channel_id)
        .bind(dto.channel_name.as_deref())
        .bind(&dto.user_id)
        .bind(&dto.content)
        .execute(&self.pg_pool)
        .await
        .map_err(sqlx_to_status("insert ai_dataset"))?;

        Ok(Response::new(Empty {}))
    }
}
