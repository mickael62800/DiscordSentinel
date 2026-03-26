use std::sync::Arc;

use crate::domain::entities::ServerStats;
use crate::domain::ports::StatsRepository;

pub struct DashboardService {
    repo: Arc<dyn StatsRepository>,
}

impl DashboardService {
    pub fn new(repo: Arc<dyn StatsRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_stats(&self) -> Result<ServerStats, String> {
        self.repo.get_dashboard_stats().await
    }
}
