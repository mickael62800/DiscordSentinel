use std::collections::HashSet;
use std::time::Instant;

use dashmap::DashMap;
use serenity::model::id::{ChannelId, UserId};

#[allow(dead_code)]
const VOTE_TIMEOUT_SECS: u64 = 60;

#[derive(Clone, Debug)]
pub struct ActiveVote {
    pub target: UserId,
    pub voice_channel_id: ChannelId,
    pub votes_yes: HashSet<UserId>,
    pub votes_no: HashSet<UserId>,
    pub total_members: usize,
    #[allow(dead_code)]
    pub started_at: Instant,
}

impl ActiveVote {
    pub fn majority_reached(&self) -> bool {
        let needed = (self.total_members / 2) + 1;
        self.votes_yes.len() >= needed
    }

    pub fn rejected(&self) -> bool {
        let needed_no = (self.total_members / 2) + 1;
        self.votes_no.len() >= needed_no || self.all_voted()
    }

    pub fn all_voted(&self) -> bool {
        (self.votes_yes.len() + self.votes_no.len()) >= self.total_members
    }

    #[allow(dead_code)]
    pub fn is_expired(&self) -> bool {
        self.started_at.elapsed().as_secs() >= VOTE_TIMEOUT_SECS
    }

    pub fn status_text(&self) -> String {
        let needed = (self.total_members / 2) + 1;
        format!(
            "Pour : **{}/{}** | Contre : **{}**",
            self.votes_yes.len(),
            needed,
            self.votes_no.len()
        )
    }
}

pub struct VoteTracker {
    map: DashMap<ChannelId, ActiveVote>,
}

impl VoteTracker {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
        }
    }

    pub fn start_vote(
        &self,
        members_channel_id: ChannelId,
        voice_channel_id: ChannelId,
        target: UserId,
        initiator: UserId,
        total_members: usize,
    ) -> bool {
        if self.map.contains_key(&members_channel_id) {
            return false;
        }

        let mut votes_yes = HashSet::new();
        votes_yes.insert(initiator);

        self.map.insert(
            members_channel_id,
            ActiveVote {
                target,
                voice_channel_id,
                votes_yes,
                votes_no: HashSet::new(),
                total_members,
                started_at: Instant::now(),
            },
        );

        true
    }

    pub fn cast_vote(
        &self,
        members_channel_id: ChannelId,
        voter: UserId,
        vote_yes: bool,
    ) -> Option<ActiveVote> {
        let mut entry = self.map.get_mut(&members_channel_id)?;
        let vote = entry.value_mut();

        if vote.votes_yes.contains(&voter) || vote.votes_no.contains(&voter) {
            return Some(vote.clone());
        }

        if voter == vote.target {
            return Some(vote.clone());
        }

        if vote_yes {
            vote.votes_yes.insert(voter);
        } else {
            vote.votes_no.insert(voter);
        }

        Some(vote.clone())
    }

    pub fn end_vote(&self, members_channel_id: ChannelId) -> Option<ActiveVote> {
        self.map.remove(&members_channel_id).map(|(_, v)| v)
    }

    pub fn has_active_vote(&self, members_channel_id: ChannelId) -> bool {
        self.map.contains_key(&members_channel_id)
    }
}
