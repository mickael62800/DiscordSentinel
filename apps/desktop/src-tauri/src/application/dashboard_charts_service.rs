use std::sync::Arc;

use crate::domain::entities::{DailyActivity, TopUser};
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

    pub async fn get_top_users(
        &self,
        guild_id: String,
        limit: Option<u32>,
    ) -> Result<Vec<TopUser>, String> {
        self.repo.get_top_users(guild_id, limit).await
    }
}
