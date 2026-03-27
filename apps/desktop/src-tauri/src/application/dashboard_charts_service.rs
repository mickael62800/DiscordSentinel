use std::sync::Arc;

use crate::domain::entities::DailyActivity;
use crate::domain::ports::DashboardChartsRepository;

pub struct DashboardChartsService {
    repo: Arc<dyn DashboardChartsRepository>,
}

impl DashboardChartsService {
    pub fn new(repo: Arc<dyn DashboardChartsRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_activity_trend(
        &self,
        guild_id: Option<String>,
        days: Option<i32>,
    ) -> Result<Vec<DailyActivity>, String> {
        self.repo.get_activity_trend(guild_id, days).await
    }
}
