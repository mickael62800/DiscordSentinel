use super::*;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::domain::entities::coude::combat::CombatResolution;
use crate::domain::entities::coude::combat::Combat;
use crate::domain::entities::coude::combat::NewCoudeCombat;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_combats::ManageCoudeCombatsUseCase;
use crate::ports::inbound::coude::resolve_betting_batch::ResolveBettingBatchUseCase;
use crate::ports::inbound::coude::resolve_betting_batch::ResolvedBettingCombatOutput;
use crate::ports::inbound::coude::expire_combats_batch::ExpireCombatsBatchUseCase;
use crate::ports::inbound::coude::expire_combats_batch::ExpiredCombatOutput;
use crate::ports::inbound::coude::resolve_combat_now::ResolveCombatNowOutput;
use crate::ports::inbound::coude::resolve_combat_now::ResolveCombatNowUseCase;
use crate::ports::inbound::coude::resolve_combat_now::ResolvedCombatEmbedField;
fn sample_combat() -> Combat {
    Combat {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        channel_id: Some("c".into()),
        attacker_id: "a".into(), attacker_name: "Atk".into(),
        defender_id: "d".into(), defender_name: "Def".into(),
        mise: 100,
        status: "pending".into(),
        winner_id: None,
        attacker_roll: None, defender_roll: None,
        chaos_event: None, special_attack: None, defender_special: None,
        coins_transferred: None, result_message: None, message_id: None,
        created_at: Utc::now(), accepted_at: None, resolved_at: None,
    }
}

#[derive(Default)]
struct MockCombatsUc {
    list_return: Mutex<Vec<Combat>>,
    list_calls: Mutex<Vec<(String, Option<String>, i64)>>,
    get_return: Mutex<Option<Combat>>,
    pending_attacker: Mutex<Option<Combat>>,
    pending_defender: Mutex<Option<Combat>>,
    expired_pending: Mutex<Vec<Combat>>,
    betting_participant: Mutex<Option<Combat>>,
    create_calls: Mutex<Vec<NewCoudeCombat>>,
    cancel_calls: Mutex<Vec<Uuid>>,
    resolve_calls: Mutex<Vec<(Uuid, CombatResolution)>>,
    set_betting_calls: Mutex<Vec<(Uuid, String)>>,
    set_betting_return: Mutex<bool>,
    expire_calls: Mutex<Vec<Uuid>>,
    set_defender_calls: Mutex<Vec<(Uuid, String)>>,
}

#[async_trait]
impl ManageCoudeCombatsUseCase for MockCombatsUc {
    async fn list(&self, g: &str, s: Option<&str>, l: i64) -> Result<Vec<Combat>, DomainError> {
        self.list_calls.lock().unwrap().push((g.into(), s.map(String::from), l));
        Ok(self.list_return.lock().unwrap().clone())
    }
    async fn get(&self, _: Uuid) -> Result<Combat, DomainError> {
        self.get_return.lock().unwrap().clone()
            .ok_or_else(|| DomainError::NotFound("combat".into()))
    }
    async fn get_pending_for_attacker(&self, _: &str, _: &str) -> Result<Option<Combat>, DomainError> {
        Ok(self.pending_attacker.lock().unwrap().clone())
    }
    async fn get_pending_for_defender(&self, _: &str, _: &str) -> Result<Option<Combat>, DomainError> {
        Ok(self.pending_defender.lock().unwrap().clone())
    }
    async fn list_expired_pending(&self) -> Result<Vec<Combat>, DomainError> {
        Ok(self.expired_pending.lock().unwrap().clone())
    }
    async fn get_betting_for_participant(&self, _: &str, _: &str) -> Result<Option<Combat>, DomainError> {
        Ok(self.betting_participant.lock().unwrap().clone())
    }
    async fn create(&self, new: NewCoudeCombat) -> Result<Combat, DomainError> {
        self.create_calls.lock().unwrap().push(new);
        Ok(sample_combat())
    }
    async fn cancel(&self, id: Uuid) -> Result<(), DomainError> {
        self.cancel_calls.lock().unwrap().push(id);
        Ok(())
    }
    async fn resolve(&self, id: Uuid, r: CombatResolution) -> Result<(), DomainError> {
        self.resolve_calls.lock().unwrap().push((id, r));
        Ok(())
    }
    async fn set_betting(&self, id: Uuid, msg: &str) -> Result<bool, DomainError> {
        self.set_betting_calls.lock().unwrap().push((id, msg.into()));
        Ok(*self.set_betting_return.lock().unwrap())
    }
    async fn expire(&self, id: Uuid) -> Result<(), DomainError> {
        self.expire_calls.lock().unwrap().push(id);
        Ok(())
    }
    async fn set_defender_special(&self, id: Uuid, item: &str) -> Result<(), DomainError> {
        self.set_defender_calls.lock().unwrap().push((id, item.into()));
        Ok(())
    }
}

#[derive(Default)]
struct MockResolveBatch {
    outputs: Mutex<Vec<ResolvedBettingCombatOutput>>,
}

#[async_trait]
impl ResolveBettingBatchUseCase for MockResolveBatch {
    async fn resolve_batch(&self) -> Result<Vec<ResolvedBettingCombatOutput>, DomainError> {
        Ok(self.outputs.lock().unwrap().clone())
    }
}

#[derive(Default)]
struct MockExpireBatch {
    outputs: Mutex<Vec<ExpiredCombatOutput>>,
}

#[async_trait]
impl ExpireCombatsBatchUseCase for MockExpireBatch {
    async fn expire_batch(&self) -> Result<Vec<ExpiredCombatOutput>, DomainError> {
        Ok(self.outputs.lock().unwrap().clone())
    }
}

#[derive(Default)]
struct MockResolveNow {
    calls: Mutex<Vec<Uuid>>,
    output: Mutex<Option<ResolveCombatNowOutput>>,
}

#[async_trait]
impl ResolveCombatNowUseCase for MockResolveNow {
    async fn resolve_now(&self, id: Uuid) -> Result<ResolveCombatNowOutput, DomainError> {
        self.calls.lock().unwrap().push(id);
        Ok(self.output.lock().unwrap().clone().unwrap_or(ResolveCombatNowOutput {
            combat_id: id.to_string(),
            title: "Result".into(),
            description: "desc".into(),
            color: 0x57F287,
            fields: vec![ResolvedCombatEmbedField {
                name: "Combat".into(),
                value: "5 rounds".into(),
                inline: false,
            }],
            taunt_events: vec![],
            vendetta_humiliation: None,
        }))
    }
}

fn grpc(uc: Arc<MockCombatsUc>) -> CombatsGrpc {
    CombatsGrpc {
        uc,
        resolve_batch_uc: Arc::new(MockResolveBatch::default()),
        expire_batch_uc: Arc::new(MockExpireBatch::default()),
        resolve_now_uc: Arc::new(MockResolveNow::default()),
    }
}

fn grpc_full(
    uc: Arc<MockCombatsUc>,
    rb: Arc<MockResolveBatch>,
    eb: Arc<MockExpireBatch>,
    rn: Arc<MockResolveNow>,
) -> CombatsGrpc {
    CombatsGrpc { uc, resolve_batch_uc: rb, expire_batch_uc: eb, resolve_now_uc: rn }
}

// ── list ──

#[tokio::test]
async fn list_default_limit_when_zero() {
    let uc = Arc::new(MockCombatsUc::default());
    let g = grpc(uc.clone());
    let _ = g.list(Request::new(proto::ListCombatsRequest {
        guild_id: "g".into(),
        status: None,
        limit: 0,
    })).await.unwrap();
    assert_eq!(uc.list_calls.lock().unwrap()[0].2, 50);
}

#[tokio::test]
async fn list_caps_limit_at_500() {
    let uc = Arc::new(MockCombatsUc::default());
    let g = grpc(uc.clone());
    let _ = g.list(Request::new(proto::ListCombatsRequest {
        guild_id: "g".into(),
        status: Some("pending".into()),
        limit: 9999,
    })).await.unwrap();
    let calls = uc.list_calls.lock().unwrap();
    assert_eq!(calls[0].1.as_deref(), Some("pending"));
    assert_eq!(calls[0].2, 500);
}

#[tokio::test]
async fn list_returns_combat_list() {
    let uc = Arc::new(MockCombatsUc::default());
    uc.list_return.lock().unwrap().push(sample_combat());
    let g = grpc(uc);
    let resp = g.list(Request::new(proto::ListCombatsRequest {
        guild_id: "g".into(), status: None, limit: 10,
    })).await.unwrap();
    assert_eq!(resp.into_inner().combats.len(), 1);
}

// ── get ──

#[tokio::test]
async fn get_rejects_invalid_uuid() {
    let g = grpc(Arc::new(MockCombatsUc::default()));
    let err = g.get(Request::new(proto::GetCombatRequest {
        id: "not-a-uuid".into(),
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn get_returns_combat() {
    let uc = Arc::new(MockCombatsUc::default());
    *uc.get_return.lock().unwrap() = Some(sample_combat());
    let g = grpc(uc);
    let resp = g.get(Request::new(proto::GetCombatRequest {
        id: Uuid::new_v4().to_string(),
    })).await.unwrap();
    assert_eq!(resp.into_inner().guild_id, "g");
}

#[tokio::test]
async fn get_not_found_propagates() {
    let g = grpc(Arc::new(MockCombatsUc::default()));
    let err = g.get(Request::new(proto::GetCombatRequest {
        id: Uuid::new_v4().to_string(),
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::NotFound);
}

// ── get_pending_for_attacker ──

#[tokio::test]
async fn get_pending_for_attacker_some_when_present() {
    let uc = Arc::new(MockCombatsUc::default());
    *uc.pending_attacker.lock().unwrap() = Some(sample_combat());
    let g = grpc(uc);
    let resp = g.get_pending_for_attacker(Request::new(proto::GetPendingForAttackerRequest {
        guild_id: "g".into(), attacker_id: "a".into(),
    })).await.unwrap();
    assert!(resp.into_inner().combat.is_some());
}

#[tokio::test]
async fn get_pending_for_attacker_none_when_absent() {
    let g = grpc(Arc::new(MockCombatsUc::default()));
    let resp = g.get_pending_for_attacker(Request::new(proto::GetPendingForAttackerRequest {
        guild_id: "g".into(), attacker_id: "a".into(),
    })).await.unwrap();
    assert!(resp.into_inner().combat.is_none());
}

// ── get_pending_for_defender ──

#[tokio::test]
async fn get_pending_for_defender_some_when_present() {
    let uc = Arc::new(MockCombatsUc::default());
    *uc.pending_defender.lock().unwrap() = Some(sample_combat());
    let g = grpc(uc);
    let resp = g.get_pending_for_defender(Request::new(proto::GetPendingForDefenderRequest {
        guild_id: "g".into(), defender_id: "d".into(),
    })).await.unwrap();
    assert!(resp.into_inner().combat.is_some());
}

#[tokio::test]
async fn get_pending_for_defender_none_when_absent() {
    let g = grpc(Arc::new(MockCombatsUc::default()));
    let resp = g.get_pending_for_defender(Request::new(proto::GetPendingForDefenderRequest {
        guild_id: "g".into(), defender_id: "d".into(),
    })).await.unwrap();
    assert!(resp.into_inner().combat.is_none());
}

// ── list_expired_pending ──

#[tokio::test]
async fn list_expired_pending_returns_all() {
    let uc = Arc::new(MockCombatsUc::default());
    uc.expired_pending.lock().unwrap().extend(vec![sample_combat(), sample_combat()]);
    let g = grpc(uc);
    let resp = g.list_expired_pending(Request::new(proto::Empty {})).await.unwrap();
    assert_eq!(resp.into_inner().combats.len(), 2);
}

// ── get_betting_for_participant ──

#[tokio::test]
async fn get_betting_for_participant_returns_maybe_combat() {
    let uc = Arc::new(MockCombatsUc::default());
    *uc.betting_participant.lock().unwrap() = Some(sample_combat());
    let g = grpc(uc);
    let resp = g.get_betting_for_participant(Request::new(proto::GetBettingForParticipantRequest {
        guild_id: "g".into(), user_id: "u".into(),
    })).await.unwrap();
    assert!(resp.into_inner().combat.is_some());
}

// ── create ──

#[tokio::test]
async fn create_delegates_all_fields() {
    let uc = Arc::new(MockCombatsUc::default());
    let g = grpc(uc.clone());
    let _ = g.create(Request::new(proto::CreateCombatRequest {
        guild_id: "g".into(),
        channel_id: Some("c".into()),
        attacker_id: "a".into(), attacker_name: "Atk".into(),
        defender_id: "d".into(), defender_name: "Def".into(),
        mise: 250,
        special_attack: Some("surprise".into()),
    })).await.unwrap();
    let calls = uc.create_calls.lock().unwrap();
    assert_eq!(calls[0].mise, 250);
    assert_eq!(calls[0].special_attack.as_deref(), Some("surprise"));
}

// ── cancel ──

#[tokio::test]
async fn cancel_rejects_invalid_uuid() {
    let g = grpc(Arc::new(MockCombatsUc::default()));
    let err = g.cancel(Request::new(proto::CancelCombatRequest {
        id: "bad".into(),
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn cancel_valid_id_delegates() {
    let uc = Arc::new(MockCombatsUc::default());
    let g = grpc(uc.clone());
    let id = Uuid::new_v4();
    let _ = g.cancel(Request::new(proto::CancelCombatRequest {
        id: id.to_string(),
    })).await.unwrap();
    assert_eq!(uc.cancel_calls.lock().unwrap()[0], id);
}

// ── resolve ──

#[tokio::test]
async fn resolve_rejects_invalid_uuid() {
    let g = grpc(Arc::new(MockCombatsUc::default()));
    let err = g.resolve(Request::new(proto::ResolveCombatRequest {
        id: "bad".into(),
        status: "accepted".into(), winner_id: None,
        attacker_roll: None, defender_roll: None,
        chaos_event: None, result_message: None,
        coins_transferred: 0,
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn resolve_with_winner_delegates_resolution() {
    let uc = Arc::new(MockCombatsUc::default());
    let g = grpc(uc.clone());
    let id = Uuid::new_v4();
    let _ = g.resolve(Request::new(proto::ResolveCombatRequest {
        id: id.to_string(),
        status: "accepted".into(),
        winner_id: Some("a".into()),
        attacker_roll: Some(15),
        defender_roll: Some(10),
        chaos_event: Some("eclipse".into()),
        result_message: Some("Victoire".into()),
        coins_transferred: 500,
    })).await.unwrap();
    let calls = uc.resolve_calls.lock().unwrap();
    assert_eq!(calls[0].0, id);
    assert_eq!(calls[0].1.winner_id.as_deref(), Some("a"));
    assert_eq!(calls[0].1.coins_transferred, 500);
}

// ── set_betting ──

#[tokio::test]
async fn set_betting_rejects_invalid_uuid() {
    let g = grpc(Arc::new(MockCombatsUc::default()));
    let err = g.set_betting(Request::new(proto::SetBettingRequest {
        id: "bad".into(), message_id: "m".into(),
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn set_betting_returns_transition_flag() {
    let uc = Arc::new(MockCombatsUc::default());
    *uc.set_betting_return.lock().unwrap() = true;
    let g = grpc(uc.clone());
    let resp = g.set_betting(Request::new(proto::SetBettingRequest {
        id: Uuid::new_v4().to_string(),
        message_id: "msg1".into(),
    })).await.unwrap();
    assert!(resp.into_inner().transitioned);
    assert_eq!(uc.set_betting_calls.lock().unwrap()[0].1, "msg1");
}

// ── expire ──

#[tokio::test]
async fn expire_rejects_invalid_uuid() {
    let g = grpc(Arc::new(MockCombatsUc::default()));
    let err = g.expire(Request::new(proto::ExpireCombatRequest {
        id: "bad".into(),
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn expire_valid_delegates() {
    let uc = Arc::new(MockCombatsUc::default());
    let g = grpc(uc.clone());
    let id = Uuid::new_v4();
    let _ = g.expire(Request::new(proto::ExpireCombatRequest {
        id: id.to_string(),
    })).await.unwrap();
    assert_eq!(uc.expire_calls.lock().unwrap()[0], id);
}

// ── set_defender_special ──

#[tokio::test]
async fn set_defender_special_rejects_invalid_uuid() {
    let g = grpc(Arc::new(MockCombatsUc::default()));
    let err = g.set_defender_special(Request::new(proto::SetDefenderSpecialRequest {
        id: "bad".into(), item_key: "shield".into(),
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn set_defender_special_valid_delegates() {
    let uc = Arc::new(MockCombatsUc::default());
    let g = grpc(uc.clone());
    let id = Uuid::new_v4();
    let _ = g.set_defender_special(Request::new(proto::SetDefenderSpecialRequest {
        id: id.to_string(),
        item_key: "explosion".into(),
    })).await.unwrap();
    assert_eq!(uc.set_defender_calls.lock().unwrap()[0], (id, "explosion".into()));
}

// ── resolve_betting_batch ──

#[tokio::test]
async fn resolve_betting_batch_empty_returns_empty() {
    let uc = Arc::new(MockCombatsUc::default());
    let rb = Arc::new(MockResolveBatch::default());
    let eb = Arc::new(MockExpireBatch::default());
    let rn = Arc::new(MockResolveNow::default());
    let g = grpc_full(uc, rb, eb, rn);
    let resp = g.resolve_betting_batch(Request::new(proto::Empty {})).await.unwrap();
    assert!(resp.into_inner().combats.is_empty());
}

#[tokio::test]
async fn resolve_betting_batch_maps_outputs() {
    let uc = Arc::new(MockCombatsUc::default());
    let rb = Arc::new(MockResolveBatch::default());
    rb.outputs.lock().unwrap().push(ResolvedBettingCombatOutput {
        combat_id: "c1".into(),
        guild_id: "g".into(),
        channel_id: Some("ch".into()),
        message_id: Some("m".into()),
        result_message: "result".into(),
        winner_id: Some("w".into()),
        loser_id: Some("l".into()),
        coins_transferred: 100,
        is_draw: false,
        taunt_events: vec![TauntEvent {
            channel_id: "ch".into(),
            target_user_id: "t".into(),
            message: "mock".into(),
            nickname_suffix: "".into(),
            streak_kind: "win",
            streak_value: 1,
        }],
    });
    let eb = Arc::new(MockExpireBatch::default());
    let rn = Arc::new(MockResolveNow::default());
    let g = grpc_full(uc, rb, eb, rn);
    let resp = g.resolve_betting_batch(Request::new(proto::Empty {})).await.unwrap();
    let inner = resp.into_inner();
    assert_eq!(inner.combats.len(), 1);
    assert_eq!(inner.combats[0].combat_id, "c1");
    assert_eq!(inner.combats[0].taunt_events.len(), 1);
}

// ── expire_combats_batch ──

#[tokio::test]
async fn expire_combats_batch_maps_outputs() {
    let uc = Arc::new(MockCombatsUc::default());
    let rb = Arc::new(MockResolveBatch::default());
    let eb = Arc::new(MockExpireBatch::default());
    eb.outputs.lock().unwrap().push(ExpiredCombatOutput {
        combat_id: "c1".into(),
        guild_id: "g".into(),
        channel_id: "ch".into(),
        defender_id: "d".into(),
        defender_name: "Def".into(),
        penalty: 200,
    });
    let rn = Arc::new(MockResolveNow::default());
    let g = grpc_full(uc, rb, eb, rn);
    let resp = g.expire_combats_batch(Request::new(proto::Empty {})).await.unwrap();
    let inner = resp.into_inner();
    assert_eq!(inner.combats.len(), 1);
    assert_eq!(inner.combats[0].penalty, 200);
    assert_eq!(inner.combats[0].defender_name, "Def");
}

// ── resolve_combat_now ──

#[tokio::test]
async fn resolve_combat_now_rejects_invalid_uuid() {
    let uc = Arc::new(MockCombatsUc::default());
    let rb = Arc::new(MockResolveBatch::default());
    let eb = Arc::new(MockExpireBatch::default());
    let rn = Arc::new(MockResolveNow::default());
    let g = grpc_full(uc, rb, eb, rn);
    let err = g.resolve_combat_now(Request::new(proto::ResolveCombatNowRequest {
        combat_id: "bad".into(),
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn resolve_combat_now_returns_embed_fields_and_taunts() {
    let uc = Arc::new(MockCombatsUc::default());
    let rb = Arc::new(MockResolveBatch::default());
    let eb = Arc::new(MockExpireBatch::default());
    let rn = Arc::new(MockResolveNow::default());
    let id = Uuid::new_v4();
    *rn.output.lock().unwrap() = Some(ResolveCombatNowOutput {
        combat_id: id.to_string(),
        title: "Victoire!".into(),
        description: "Gagnant".into(),
        color: 0x57F287,
        fields: vec![ResolvedCombatEmbedField {
            name: "Combat".into(),
            value: "3 rounds".into(),
            inline: false,
        }],
        taunt_events: vec![],
        vendetta_humiliation: None,
    });
    let g = grpc_full(uc, rb, eb, rn.clone());
    let resp = g.resolve_combat_now(Request::new(proto::ResolveCombatNowRequest {
        combat_id: id.to_string(),
    })).await.unwrap();
    let inner = resp.into_inner();
    assert_eq!(inner.combat_id, id.to_string());
    assert_eq!(inner.title, "Victoire!");
    assert_eq!(inner.color, 0x57F287);
    assert_eq!(inner.fields.len(), 1);
    assert_eq!(inner.fields[0].name, "Combat");
    assert_eq!(rn.calls.lock().unwrap()[0], id);
}
