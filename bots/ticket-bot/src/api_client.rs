//! Client API du ticket-bot.
//!
//! Phase 7A — Migration gRPC :
//! - Tout le domaine `tickets` (list, get, create, reply, close, status, assign,
//!   channels) passe par gRPC via `SentinelGrpcClient::tickets()`.
//! - `update_ticket_priority` et `update_ticket_sla` restent sur HTTP : ils
//!   utilisent un endpoint flexible (`PATCH /api/tickets/{id}/status` avec
//!   `{priority}`) ou un handler ad hoc (`/sla`) qui n'a pas de RPC dedie en v1.
//!
//! Surface publique inchangee : `handler.rs` et `commands/*` continuent
//! d'appeler les memes methodes.
//!
//! ## Comportement si l'API tombe
//!
//! Tous les appels gRPC passent par `SentinelGrpcClient::guarded()` :
//! - Apres 5 echecs consecutifs (`Unavailable`/`DeadlineExceeded`/`Internal`),
//!   le circuit breaker s'ouvre 10s et les appels suivants renvoient
//!   immediatement `Err("API indisponible...")`.
//! - `list_tickets` : la boucle de check_escalations (toutes les 5 min) saute
//!   simplement le tour — pas d'escalade pendant la panne, repart au tour
//!   suivant quand l'API revient.
//! - `create_ticket` / `reply_ticket` : la commande slash repond a
//!   l'utilisateur « API indisponible, reessayez ». Pas de message perdu
//!   silencieusement.
//! - `close_ticket` / `update_status` / `assign_ticket` : idem.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::grpc_client::{GrpcCallError, SentinelGrpcClient};

use sentinel_proto::tickets::v1 as proto;

// ── DTOs publics (surface inchangee) ──

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Ticket {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub author_id: String,
    pub author_name: String,
    pub assigned_to: Option<String>,
    pub server: String,
    pub category: String,
    pub ticket_type: Option<String>,
    pub channel_id: Option<String>,
    pub voice_channel_id: Option<String>,
    pub invited_user_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub messages_count: u32,
    #[serde(default)]
    pub first_response_at: Option<String>,
    #[serde(default)]
    pub resolved_at: Option<String>,
    #[serde(default)]
    pub satisfaction_rating: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TicketMessage {
    pub id: String,
    pub ticket_id: String,
    pub author_name: String,
    pub author_role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TicketDetail {
    pub ticket: Ticket,
    pub messages: Vec<TicketMessage>,
}

#[derive(Debug, Serialize)]
pub struct CreateTicketRequest {
    pub title: String,
    pub priority: String,
    pub author_id: String,
    pub author_name: String,
    pub server: String,
    pub category: String,
    pub ticket_type: String,
    pub channel_id: Option<String>,
}

// ── Client ──

pub struct ApiClient {
    // Phase 7A.opt F.6 : `base` n'est plus utilise car SLA et priority sont
    // maintenant en gRPC. Conserve pour compat TypeMap (le heartbeat des
    // ticket-bot passe encore par BaseApiClient dans main.rs).
    #[allow(dead_code)]
    pub base: Arc<BaseApiClient>,
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>, grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { base, grpc }
    }

    /// Helper : construit un `ApiClient` depuis le TypeMap Serenity. Renvoie
    /// `None` si le `BaseApiClient` ou le `SentinelGrpcClient` ne sont pas
    /// presents (ne devrait pas arriver en pratique : main.rs les insere
    /// tous deux au demarrage).
    #[allow(dead_code)]
    pub fn from_data(data: &serenity::prelude::TypeMap) -> Option<Self> {
        let base = data
            .get::<sentinel_shared::heartbeat::ApiClientKey>()?
            .clone();
        let grpc = data
            .get::<sentinel_shared::grpc_client::GrpcClientKey>()?
            .clone();
        Some(Self::new(base, grpc))
    }

    pub async fn list_tickets(&self) -> Result<Vec<Ticket>, String> {
        let req = proto::ListTicketsRequest {
            status: None,
            priority: None,
            search: None,
            author_id: None,
            limit: 200,
            offset: 0,
        };
        let mut client = self.grpc.tickets();
        let list = self
            .grpc
            .guarded(|| async move { client.list_tickets(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(list.tickets.into_iter().map(proto_ticket_to_dto).collect())
    }

    pub async fn create_ticket(&self, request: &CreateTicketRequest) -> Result<Ticket, String> {
        let req = proto::CreateTicketRequest {
            title: request.title.clone(),
            priority: request.priority.clone(),
            author_id: request.author_id.clone(),
            author_name: request.author_name.clone(),
            server: request.server.clone(),
            category: request.category.clone(),
            ticket_type: request.ticket_type.clone(),
            channel_id: request.channel_id.clone(),
        };
        let mut client = self.grpc.tickets();
        let t = self
            .grpc
            .guarded(|| async move { client.create_ticket(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(proto_ticket_to_dto(t))
    }

    #[allow(dead_code)]
    pub async fn get_ticket(&self, id: &str) -> Result<TicketDetail, String> {
        let req = proto::GetTicketDetailRequest {
            id: id.to_string(),
        };
        let mut client = self.grpc.tickets();
        let detail = self
            .grpc
            .guarded(|| async move {
                client.get_ticket_detail(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(proto_ticket_detail_to_dto(detail))
    }

    pub async fn reply_ticket(
        &self,
        ticket_id: &str,
        content: &str,
        author_name: &str,
        author_role: &str,
    ) -> Result<(), String> {
        let req = proto::ReplyTicketRequest {
            ticket_id: ticket_id.to_string(),
            content: content.to_string(),
            author_name: author_name.to_string(),
            author_role: author_role.to_string(),
        };
        let mut client = self.grpc.tickets();
        self.grpc
            .guarded(|| async move { client.reply_ticket(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    #[allow(dead_code)]
    pub async fn update_status(&self, id: &str, status: &str) -> Result<(), String> {
        let req = proto::UpdateStatusRequest {
            id: id.to_string(),
            status: status.to_string(),
        };
        let mut client = self.grpc.tickets();
        self.grpc
            .guarded(|| async move { client.update_status(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn close_ticket(&self, id: &str) -> Result<(), String> {
        let req = proto::CloseTicketRequest {
            id: id.to_string(),
        };
        let mut client = self.grpc.tickets();
        self.grpc
            .guarded(|| async move { client.close_ticket(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    #[allow(dead_code)]
    pub async fn assign_ticket(&self, id: &str, assignee: &str) -> Result<(), String> {
        let req = proto::AssignTicketRequest {
            ticket_id: id.to_string(),
            assignee: assignee.to_string(),
        };
        let mut client = self.grpc.tickets();
        self.grpc
            .guarded(|| async move { client.assign_ticket(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn update_ticket_channel(
        &self,
        id: &str,
        voice_channel_id: Option<String>,
        invited_user_id: Option<String>,
    ) -> Result<(), String> {
        let req = proto::UpdateTicketChannelRequest {
            ticket_id: id.to_string(),
            voice_channel_id,
            invited_user_id,
        };
        let mut client = self.grpc.tickets();
        self.grpc
            .guarded(|| async move {
                client.update_ticket_channel(req).await.map(|_| ())
            })
            .await
            .map_err(grpc_err_to_string)
    }

    // ── Phase 7A.opt F.6 — priority et SLA migres en gRPC ──

    /// gRPC `TicketsService.UpdatePriority`.
    pub async fn update_ticket_priority(&self, id: &str, priority: &str) -> Result<(), String> {
        let req = proto::UpdatePriorityRequest {
            id: id.to_string(),
            priority: priority.to_string(),
        };
        let mut client = self.grpc.tickets();
        self.grpc
            .guarded(|| async move { client.update_priority(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    /// gRPC `TicketsService.UpdateSla`.
    pub async fn update_ticket_sla(
        &self,
        id: &str,
        first_response_at: Option<&str>,
        resolved_at: Option<&str>,
        satisfaction_rating: Option<u8>,
    ) {
        let req = proto::UpdateSlaRequest {
            id: id.to_string(),
            first_response_at: first_response_at.map(|s| s.to_string()),
            resolved_at: resolved_at.map(|s| s.to_string()),
            satisfaction_rating: satisfaction_rating.map(|r| r as i32),
        };
        let mut client = self.grpc.tickets();
        let _ = self
            .grpc
            .guarded(|| async move { client.update_sla(req).await.map(|_| ()) })
            .await;
        // fire-and-forget : on ignore l'erreur (historique du handler HTTP).
    }
}

// ── Helpers proto -> DTO ──

fn proto_ticket_to_dto(t: proto::Ticket) -> Ticket {
    Ticket {
        id: t.id,
        title: t.title,
        status: t.status,
        priority: t.priority,
        author_id: t.author_id,
        author_name: t.author_name,
        assigned_to: t.assigned_to,
        server: t.server,
        category: t.category,
        ticket_type: Some(t.ticket_type),
        channel_id: t.channel_id,
        voice_channel_id: t.voice_channel_id,
        invited_user_id: t.invited_user_id,
        created_at: t.created_at,
        updated_at: t.updated_at,
        messages_count: t.messages_count,
        first_response_at: None,
        resolved_at: None,
        satisfaction_rating: None,
    }
}

fn proto_ticket_message_to_dto(m: proto::TicketMessage) -> TicketMessage {
    TicketMessage {
        id: m.id,
        ticket_id: m.ticket_id,
        author_name: m.author_name,
        author_role: m.author_role,
        content: m.content,
        created_at: m.created_at,
    }
}

fn proto_ticket_detail_to_dto(d: proto::TicketDetail) -> TicketDetail {
    TicketDetail {
        ticket: d
            .ticket
            .map(proto_ticket_to_dto)
            .unwrap_or_else(empty_ticket),
        messages: d.messages.into_iter().map(proto_ticket_message_to_dto).collect(),
    }
}

fn empty_ticket() -> Ticket {
    Ticket {
        id: String::new(),
        title: String::new(),
        status: String::new(),
        priority: String::new(),
        author_id: String::new(),
        author_name: String::new(),
        assigned_to: None,
        server: String::new(),
        category: String::new(),
        ticket_type: None,
        channel_id: None,
        voice_channel_id: None,
        invited_user_id: None,
        created_at: String::new(),
        updated_at: String::new(),
        messages_count: 0,
        first_response_at: None,
        resolved_at: None,
        satisfaction_rating: None,
    }
}

fn grpc_err_to_string(e: GrpcCallError) -> String {
    match e {
        GrpcCallError::Unavailable => "API indisponible (circuit breaker ouvert)".to_string(),
        GrpcCallError::Status(s) => format!("gRPC {:?}: {}", s.code(), s.message()),
        GrpcCallError::Transport(t) => format!("transport gRPC: {t}"),
    }
}
