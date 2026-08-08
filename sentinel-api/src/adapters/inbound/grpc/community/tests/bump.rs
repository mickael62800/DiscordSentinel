use super::*;
use async_trait::async_trait;
use chrono::TimeZone;
use sentinel_core::domain::errors::DomainError;
use std::sync::Mutex;

#[derive(Default)]
struct MockUc {
    recorded: Mutex<Vec<RecordBumpCommand>>,
    reward: Option<BumpReward>,
    due: Vec<DueReminder>,
    states: Vec<BumpState>,
    marked: Mutex<Vec<(String, Option<String>)>>,
}

#[async_trait]
impl ManageBumpUseCase for MockUc {
    async fn record_bump(&self, cmd: RecordBumpCommand) -> Result<BumpReward, DomainError> {
        self.recorded.lock().unwrap().push(cmd);
        Ok(self.reward.clone().unwrap_or(BumpReward {
            rewarded: true,
            reward: 10,
            weekly_count: 3,
            new_balance: Some(110),
            vip_role_id: Some("vip1".into()),
            vip_just_unlocked: true,
        }))
    }
    async fn due_reminders(&self) -> Result<Vec<DueReminder>, DomainError> {
        Ok(self.due.clone())
    }
    async fn mark_reminder_sent(
        &self,
        guild_id: &str,
        provider: Option<String>,
    ) -> Result<(), DomainError> {
        self.marked
            .lock()
            .unwrap()
            .push((guild_id.into(), provider));
        Ok(())
    }
    async fn guild_status(&self, _guild_id: &str) -> Result<Vec<BumpState>, DomainError> {
        Ok(self.states.clone())
    }
}

fn grpc(uc: Arc<MockUc>) -> BumpGrpc {
    BumpGrpc { uc }
}

#[tokio::test]
async fn record_bump_forwards_and_maps() {
    let uc = Arc::new(MockUc::default());
    let resp = grpc(uc.clone())
        .record_bump(Request::new(proto::RecordBumpRequest {
            guild_id: "g1".into(),
            user_id: "u1".into(),
            username: "Alice".into(),
            channel_id: "c1".into(),
            provider: "disboard".into(),
        }))
        .await
        .unwrap()
        .into_inner();
    let rec = uc.recorded.lock().unwrap();
    assert_eq!(rec[0].guild_id, "g1");
    assert_eq!(rec[0].provider, "disboard");
    assert!(resp.rewarded);
    assert_eq!(resp.reward, 10);
    assert_eq!(resp.vip_role_id.as_deref(), Some("vip1"));
    assert!(resp.vip_just_unlocked);
}

#[tokio::test]
async fn due_reminders_maps_list() {
    let uc = Arc::new(MockUc {
        due: vec![DueReminder {
            guild_id: "g1".into(),
            channel_id: "c1".into(),
            provider: "discordl".into(),
        }],
        ..Default::default()
    });
    let list = grpc(uc)
        .due_reminders(Request::new(proto::DueRemindersRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(list.reminders.len(), 1);
    assert_eq!(list.reminders[0].provider, "discordl");
}

#[tokio::test]
async fn mark_reminder_sent_forwards_provider() {
    let uc = Arc::new(MockUc::default());
    grpc(uc.clone())
        .mark_reminder_sent(Request::new(proto::MarkReminderSentRequest {
            guild_id: "g1".into(),
            provider: Some("disboard".into()),
        }))
        .await
        .unwrap();
    assert_eq!(
        uc.marked.lock().unwrap().as_slice(),
        &[("g1".into(), Some("disboard".into()))]
    );
}

#[tokio::test]
async fn guild_status_computes_ready_at() {
    let uc = Arc::new(MockUc {
        states: vec![BumpState {
            provider: "disboard".into(),
            channel_id: "c1".into(),
            last_bump_at: chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            cooldown_minutes: 120,
        }],
        ..Default::default()
    });
    let list = grpc(uc)
        .guild_status(Request::new(proto::GuildStatusRequest {
            guild_id: "g1".into(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(list.statuses.len(), 1);
    // last_bump_at + 120 min = 02:00:00.
    assert_eq!(list.statuses[0].ready_at, "2026-01-01T02:00:00+00:00");
}
