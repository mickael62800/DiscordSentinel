use std::sync::Arc;

use crate::domain::entities::Infraction;
use crate::domain::ports::InfractionsRepository;

pub struct InfractionsService {
    repo: Arc<dyn InfractionsRepository>,
}

impl InfractionsService {
    pub fn new(repo: Arc<dyn InfractionsRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_infractions(&self, guild_id: Option<String>) -> Result<Vec<Infraction>, String> {
        self.repo.get_infractions(guild_id).await
    }
}
