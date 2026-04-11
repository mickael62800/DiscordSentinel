use std::sync::Arc;

use crate::domain::entities::{Member, MemberSummary};
use crate::domain::ports::MembersRepository;

pub struct MembersService {
    repo: Arc<dyn MembersRepository>,
}

impl MembersService {
    pub fn new(repo: Arc<dyn MembersRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_members(&self, guild_id: String) -> Result<Vec<Member>, String> {
        self.repo.get_members(guild_id).await
    }

    pub async fn get_member_summary(&self, guild_id: String, user_id: String) -> Result<MemberSummary, String> {
        self.repo.get_member_summary(guild_id, user_id).await
    }
}
