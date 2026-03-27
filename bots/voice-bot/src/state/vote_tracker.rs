use std::collections::HashSet;

use dashmap::DashMap;
use serenity::model::id::{ChannelId, UserId};

#[derive(Clone, Debug)]
pub struct ActiveVote {
    pub target: UserId,
    pub voice_channel_id: ChannelId,
    pub votes_yes: HashSet<UserId>,
    pub votes_no: HashSet<UserId>,
    pub total_members: usize,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn uid(id: u64) -> UserId { UserId::new(id) }
    fn cid(id: u64) -> ChannelId { ChannelId::new(id) }

    #[test]
    fn test_majority_3_members() {
        let mut v = ActiveVote {
            target: uid(99),
            voice_channel_id: cid(1),
            votes_yes: HashSet::new(),
            votes_no: HashSet::new(),
            total_members: 3,
        };
        // Besoin de 2 votes pour majorite (3/2 + 1 = 2)
        assert!(!v.majority_reached());
        v.votes_yes.insert(uid(1));
        assert!(!v.majority_reached());
        v.votes_yes.insert(uid(2));
        assert!(v.majority_reached());
    }

    #[test]
    fn test_rejected_by_majority_no() {
        let mut v = ActiveVote {
            target: uid(99),
            voice_channel_id: cid(1),
            votes_yes: HashSet::new(),
            votes_no: HashSet::new(),
            total_members: 3,
        };
        v.votes_no.insert(uid(1));
        assert!(!v.rejected());
        v.votes_no.insert(uid(2));
        assert!(v.rejected());
    }

    #[test]
    fn test_rejected_when_all_voted() {
        let mut v = ActiveVote {
            target: uid(99),
            voice_channel_id: cid(1),
            votes_yes: HashSet::new(),
            votes_no: HashSet::new(),
            total_members: 2,
        };
        v.votes_yes.insert(uid(1));
        v.votes_no.insert(uid(2));
        assert!(v.all_voted());
        // 1 oui, 1 non sur 2 → pas de majorite oui, mais all_voted → rejected
        assert!(v.rejected());
    }

    #[test]
    fn test_start_vote_prevents_duplicate() {
        let tracker = VoteTracker::new();
        assert!(tracker.start_vote(cid(10), cid(20), uid(99), uid(1), 5));
        assert!(!tracker.start_vote(cid(10), cid(20), uid(99), uid(2), 5));
    }

    #[test]
    fn test_cast_vote_prevents_duplicate_vote() {
        let tracker = VoteTracker::new();
        tracker.start_vote(cid(10), cid(20), uid(99), uid(1), 5);

        // uid(1) a deja vote oui (initiateur)
        let v = tracker.cast_vote(cid(10), uid(1), true).unwrap();
        assert_eq!(v.votes_yes.len(), 1); // pas de double

        // uid(2) vote oui
        let v = tracker.cast_vote(cid(10), uid(2), true).unwrap();
        assert_eq!(v.votes_yes.len(), 2);
    }

    #[test]
    fn test_target_cannot_vote() {
        let tracker = VoteTracker::new();
        tracker.start_vote(cid(10), cid(20), uid(99), uid(1), 5);

        let v = tracker.cast_vote(cid(10), uid(99), false).unwrap();
        assert_eq!(v.votes_no.len(), 0); // target ignoree
    }

    #[test]
    fn test_end_vote() {
        let tracker = VoteTracker::new();
        tracker.start_vote(cid(10), cid(20), uid(99), uid(1), 5);
        assert!(tracker.has_active_vote(cid(10)));

        let ended = tracker.end_vote(cid(10));
        assert!(ended.is_some());
        assert!(!tracker.has_active_vote(cid(10)));
    }

    #[test]
    fn test_status_text() {
        let mut v = ActiveVote {
            target: uid(99),
            voice_channel_id: cid(1),
            votes_yes: HashSet::new(),
            votes_no: HashSet::new(),
            total_members: 5,
        };
        v.votes_yes.insert(uid(1));
        v.votes_yes.insert(uid(2));
        v.votes_no.insert(uid(3));
        let text = v.status_text();
        assert!(text.contains("2/3"));
        assert!(text.contains("1"));
    }
}
