use std::future::Future;
use std::pin::Pin;

use reqwest::{Client, RequestBuilder, Response};

use crate::domain::entities::{Infraction, LogEntry, ModerationActionRequest, ModerationActionResponse, ModerationRule, SecurityEvent, ServerStats, Ticket, TicketDetail, UpdateRuleParams, UserModerationHistory};
use crate::domain::ports::{AppAdapter, InfractionsRepository, LogsRepository, ModerationRepository, RulesRepository, SecurityRepository, StatsRepository, TicketsRepository};

pub struct ApiAdapter {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ApiAdapter {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url,
            api_key,
        }
    }

    fn auth(&self, req: RequestBuilder) -> RequestBuilder {
        if self.api_key.is_empty() {
            req
        } else {
            req.bearer_auth(&self.api_key)
        }
    }
}

async fn check_response(resp: Response) -> Result<Response, String> {
    let status = resp.status();
    if status.is_success() {
        Ok(resp)
    } else if status.as_u16() == 401 {
        Err("Unauthorized: invalid API key".into())
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(format!("API error {}: {}", status.as_u16(), body))
    }
}

// --- Stats: GET /api/stats ---

impl StatsRepository for ApiAdapter {
    fn get_dashboard_stats(&self) -> Pin<Box<dyn Future<Output = Result<ServerStats, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/stats", self.base_url)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<ServerStats>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
}

// --- Logs: GET /api/logs ---

impl LogsRepository for ApiAdapter {
    fn get_logs(&self) -> Pin<Box<dyn Future<Output = Result<Vec<LogEntry>, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/logs", self.base_url)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<LogEntry>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
}

// --- Infractions: GET /api/infractions ---

impl InfractionsRepository for ApiAdapter {
    fn get_infractions(&self) -> Pin<Box<dyn Future<Output = Result<Vec<Infraction>, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/infractions", self.base_url)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<Infraction>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
}

// --- Rules: GET /api/rules, PATCH /api/rules/{id} ---

impl RulesRepository for ApiAdapter {
    fn get_rules(&self) -> Pin<Box<dyn Future<Output = Result<Vec<ModerationRule>, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/rules", self.base_url)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<ModerationRule>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    fn toggle_rule(&self, id: String, enabled: bool) -> Pin<Box<dyn Future<Output = Result<bool, String>> + Send>> {
        #[derive(serde::Serialize)]
        struct Payload { enabled: bool }

        let req = self.auth(
            self.client.patch(format!("{}/api/rules/{}", self.base_url, id))
        ).json(&Payload { enabled });

        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            check_response(resp).await?;
            Ok(enabled)
        })
    }

    fn update_rule(&self, params: UpdateRuleParams) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        let req = self.auth(
            self.client.post(format!("{}/rules", self.base_url))
        ).json(&params);

        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            check_response(resp).await?;
            Ok(())
        })
    }
}

// --- Tickets: GET /api/tickets, GET /api/tickets/{id}, POST /api/tickets/{id}/messages,
//              PATCH /api/tickets/{id}/close, PATCH /api/tickets/{id}/assign ---

impl TicketsRepository for ApiAdapter {
    fn get_tickets(&self) -> Pin<Box<dyn Future<Output = Result<Vec<Ticket>, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/tickets", self.base_url)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<Ticket>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    fn get_ticket_detail(&self, id: String) -> Pin<Box<dyn Future<Output = Result<TicketDetail, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/tickets/{}", self.base_url, id)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<TicketDetail>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    fn reply_ticket(&self, ticket_id: String, content: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        #[derive(serde::Serialize)]
        struct Payload { content: String }

        let req = self.auth(
            self.client.post(format!("{}/api/tickets/{}/messages", self.base_url, ticket_id))
        ).json(&Payload { content });

        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            check_response(resp).await?;
            Ok(())
        })
    }

    fn close_ticket(&self, id: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        let req = self.auth(self.client.patch(format!("{}/api/tickets/{}/close", self.base_url, id)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            check_response(resp).await?;
            Ok(())
        })
    }

    fn assign_ticket(&self, id: String, assignee: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        #[derive(serde::Serialize)]
        struct Payload { assignee: String }

        let req = self.auth(
            self.client.patch(format!("{}/api/tickets/{}/assign", self.base_url, id))
        ).json(&Payload { assignee });

        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            check_response(resp).await?;
            Ok(())
        })
    }
}

// --- Security: GET /api/security/events ---

impl SecurityRepository for ApiAdapter {
    fn get_events(&self, guild_id: Option<String>) -> Pin<Box<dyn Future<Output = Result<Vec<SecurityEvent>, String>> + Send>> {
        let mut url = format!("{}/api/security/events", self.base_url);
        if let Some(gid) = guild_id {
            url = format!("{}?guild_id={}", url, gid);
        }
        let req = self.auth(self.client.get(url));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<SecurityEvent>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
}

// --- Moderation: POST /api/moderation/actions, GET /api/moderation/history/{guild_id}/{user_id} ---

impl ModerationRepository for ApiAdapter {
    fn log_action(&self, action: ModerationActionRequest) -> Pin<Box<dyn Future<Output = Result<ModerationActionResponse, String>> + Send>> {
        let req = self.auth(
            self.client.post(format!("{}/api/moderation/actions", self.base_url))
        ).json(&action);

        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<ModerationActionResponse>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    fn get_history(&self, guild_id: String, user_id: String) -> Pin<Box<dyn Future<Output = Result<UserModerationHistory, String>> + Send>> {
        let req = self.auth(
            self.client.get(format!("{}/api/moderation/history/{}/{}", self.base_url, guild_id, user_id))
        );

        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<UserModerationHistory>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
}

impl AppAdapter for ApiAdapter {}
