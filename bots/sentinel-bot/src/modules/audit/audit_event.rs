use super::api_client::AuditEvent;

pub fn simple(guild_id: String, event_type: &str) -> AuditEvent {
    AuditEvent {
        guild_id,
        event_type: event_type.to_string(),
        actor_id: None,
        actor_name: None,
        target_id: None,
        target_name: None,
        channel_id: None,
        channel_name: None,
        details: serde_json::json!({}),
    }
}

impl AuditEvent {
    pub fn with_target(mut self, id: impl ToString, name: impl ToString) -> Self {
        self.target_id = Some(id.to_string());
        self.target_name = Some(name.to_string());
        self
    }

    pub fn with_actor(mut self, id: impl ToString, name: impl ToString) -> Self {
        self.actor_id = Some(id.to_string());
        self.actor_name = Some(name.to_string());
        self
    }

    pub fn with_channel(mut self, id: impl ToString, name: Option<String>) -> Self {
        self.channel_id = Some(id.to_string());
        self.channel_name = name;
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }
}
