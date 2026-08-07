//! Adaptateur gRPC unaire consomme par `atrium-bot`.

use std::sync::Arc;

use atrium_core::{
    domain::{ConversationScope, WelcomeRequest},
    ports::inbound::GenerateWelcomeReplyUseCase,
};
use atrium_proto::welcome::v1::{
    self as proto,
    welcome_service_server::{WelcomeService, WelcomeServiceServer},
};
use tonic::{Request, Response, Status};

use crate::{merge_context, rag::RagService, welcome_use_case, AppConfig};

use std::pin::Pin;
use tokio_stream::Stream;

pub async fn serve(config: AppConfig, rag: Arc<RagService>) {
    let addr = config.grpc_addr;
    let welcome_service = WelcomeGrpc {
        welcome: welcome_use_case(&config),
        rag: Some(rag.clone()),
    };
    let rag_service = RagGrpc { rag: Some(rag) };

    tracing::info!(%addr, "Atrium gRPC démarré (Welcome & RAG)");
    tonic::transport::Server::builder()
        .add_service(WelcomeServiceServer::new(welcome_service))
        .add_service(proto::rag_service_server::RagServiceServer::new(rag_service))
        .serve(addr)
        .await
        .expect("serveur gRPC Atrium");
}

pub struct WelcomeGrpc {
    pub welcome: Arc<dyn GenerateWelcomeReplyUseCase>,
    pub rag: Option<Arc<RagService>>,
}

impl WelcomeGrpc {
    pub fn new(welcome: Arc<dyn GenerateWelcomeReplyUseCase>) -> Self {
        Self { welcome, rag: None }
    }
}

pub struct RagGrpc {
    pub rag: Option<Arc<RagService>>,
}

#[tonic::async_trait]
impl WelcomeService for WelcomeGrpc {
    type StreamReplyStream = Pin<Box<dyn Stream<Item = Result<proto::ReplyChunk, Status>> + Send + 'static>>;

    async fn generate_reply(
        &self,
        request: Request<proto::GenerateReplyRequest>,
    ) -> Result<Response<proto::GenerateReplyResponse>, Status> {
        let input = request.into_inner();
        let scope = match proto::ConversationScope::try_from(input.scope)
            .unwrap_or(proto::ConversationScope::General)
        {
            proto::ConversationScope::General => ConversationScope::General,
            proto::ConversationScope::Direct => ConversationScope::Direct,
        };
        let retrieved = match &self.rag {
            Some(rag) => rag
                .context_for(&input.guild_id, &input.member_message)
                .await
                .map_err(|error| {
                    tracing::warn!(%error, "Recherche RAG gRPC indisponible");
                    Status::unavailable("recherche de connaissances indisponible")
                })?,
            None => String::new(),
        };
        let reply = self
            .welcome
            .reply(WelcomeRequest {
                guild_id: input.guild_id,
                member_id: input.member_id,
                member_display_name: input.member_display_name,
                channel_id: input.channel_id,
                scope,
                member_message: input.member_message,
                server_context: merge_context(&input.server_context, &retrieved),
            })
            .await
            .map_err(|error| Status::invalid_argument(error.to_string()))?;
        Ok(Response::new(proto::GenerateReplyResponse {
            reply: reply.content,
            generated_by_ai: reply.generated_by_ai,
        }))
    }

    async fn stream_reply(
        &self,
        request: Request<proto::GenerateReplyRequest>,
    ) -> Result<Response<Self::StreamReplyStream>, Status> {
        let input = request.into_inner();
        let scope = match proto::ConversationScope::try_from(input.scope)
            .unwrap_or(proto::ConversationScope::General)
        {
            proto::ConversationScope::General => ConversationScope::General,
            proto::ConversationScope::Direct => ConversationScope::Direct,
        };
        let retrieved = match &self.rag {
            Some(rag) => rag
                .context_for(&input.guild_id, &input.member_message)
                .await
                .map_err(|error| {
                    tracing::warn!(%error, "Recherche RAG gRPC indisponible");
                    Status::unavailable("recherche de connaissances indisponible")
                })?,
            None => String::new(),
        };
        let reply = self
            .welcome
            .reply(WelcomeRequest {
                guild_id: input.guild_id,
                member_id: input.member_id,
                member_display_name: input.member_display_name,
                channel_id: input.channel_id,
                scope,
                member_message: input.member_message,
                server_context: merge_context(&input.server_context, &retrieved),
            })
            .await
            .map_err(|error| Status::invalid_argument(error.to_string()))?;

        // Simuler ou découper la réponse en tokens pour le streaming gRPC
        let content = reply.content;
        let output_stream = async_stream::try_stream! {
            let words: Vec<&str> = content.split_whitespace().collect();
            for (idx, word) in words.iter().enumerate() {
                let space = if idx > 0 { " " } else { "" };
                let delta = format!("{space}{word}");
                let is_final = idx == words.len() - 1;
                yield proto::ReplyChunk {
                    delta,
                    is_final,
                };
            }
        };

        Ok(Response::new(Box::pin(output_stream)))
    }
}

#[tonic::async_trait]
impl proto::rag_service_server::RagService for RagGrpc {
    async fn search_knowledge(
        &self,
        request: Request<proto::SearchKnowledgeRequest>,
    ) -> Result<Response<proto::SearchKnowledgeResponse>, Status> {
        let req = request.into_inner();
        let rag = self.rag.as_ref().ok_or_else(|| Status::unavailable("RAG non configuré"))?;
        let chunks = rag.search_chunks(&req.query, req.limit).await.map_err(|e| Status::internal(e))?;

        let proto_chunks = chunks
            .into_iter()
            .map(|(source, content, similarity)| proto::SearchKnowledgeChunk {
                source,
                content,
                similarity,
            })
            .collect();

        Ok(Response::new(proto::SearchKnowledgeResponse {
            chunks: proto_chunks,
        }))
    }
}
