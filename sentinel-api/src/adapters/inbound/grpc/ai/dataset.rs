//! gRPC AiDataset — collecte des messages texte pour l'entrainement IA.
//!
//! Copie du pattern du handler HTTP `ai/dataset.rs::collect_message` : sqlx
//! direct car pas de use case unifie cote API (simple ingestion best-effort).
//! Remplace `POST /api/ai-dataset/collect`, chemin le plus chaud du bot
//! (un message par message non-bot des guilds ou le module est actif).
//!
//! Client-streaming : le bot maintient une stream longue duree, le serveur
//! insere chaque message au fil de l'eau. Un insert qui echoue est logge
//! mais n'interrompt PAS la stream (best-effort).

use tonic::Request;
use tonic::Response;
use tonic::Status;
use tonic::Streaming;
use tracing::warn;

use sentinel_proto::ai_dataset::v1 as proto;
use sentinel_proto::ai_dataset::v1::ai_dataset_service_server::AiDatasetService;
use sentinel_proto::common::v1::Empty;

pub struct AiDatasetGrpc {
    pub pg_pool: sqlx::PgPool,
}

#[tonic::async_trait]
impl AiDatasetService for AiDatasetGrpc {
    async fn collect_messages(
        &self,
        request: Request<Streaming<proto::CollectMessageRequest>>,
    ) -> Result<Response<Empty>, Status> {
        let mut stream = request.into_inner();

        while let Some(dto) = stream.message().await? {
            // Best-effort : on ignore silencieusement les messages invalides
            // ou vides plutot que de rompre la stream.
            if dto.guild_id.trim().is_empty()
                || dto.user_id.trim().is_empty()
                || dto.content.trim().is_empty()
            {
                continue;
            }

            if let Err(e) = sqlx::query(
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
            {
                // Un insert rate ne doit pas tuer la stream : on logge et continue.
                warn!(error = %e, "Echec insert ai_dataset (stream), message ignore");
            }
        }

        Ok(Response::new(Empty {}))
    }
}
