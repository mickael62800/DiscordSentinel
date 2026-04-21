use super::*;
use crate::ports::outbound::ReviewEntry;
use chrono::{TimeZone, Utc};
use uuid::Uuid;

fn sample_entry(resolved: Option<chrono::DateTime<Utc>>) -> ReviewEntry {
    ReviewEntry {
        id: Uuid::new_v4(),
        action_id: Uuid::new_v4(),
        guild_id: "g".into(),
        added_by: "u1".into(),
        added_by_name: "Alice".into(),
        reason: Some("appel".into()),
        status: "pending".into(),
        reviewer_id: None,
        reviewer_name: None,
        reviewer_notes: None,
        added_at: Utc.with_ymd_and_hms(2024, 5, 1, 10, 0, 0).unwrap(),
        resolved_at: resolved,
        action_type: Some("ban".into()),
        target_name: Some("bob".into()),
        action_reason: Some("spam".into()),
    }
}

#[test]
fn review_entry_to_dto_maps_all_ids_to_strings() {
    let e = sample_entry(None);
    let id = e.id.to_string();
    let aid = e.action_id.to_string();
    let dto = review_entry_to_dto(e);
    assert_eq!(dto.id, id);
    assert_eq!(dto.action_id, aid);
}

#[test]
fn review_entry_to_dto_formats_added_at_rfc3339() {
    let dto = review_entry_to_dto(sample_entry(None));
    assert!(dto.added_at.starts_with("2024-05-01T"));
}

#[test]
fn review_entry_to_dto_resolved_none_preserved() {
    let dto = review_entry_to_dto(sample_entry(None));
    assert!(dto.resolved_at.is_none());
}

#[test]
fn review_entry_to_dto_resolved_formatted_when_some() {
    let resolved = Utc.with_ymd_and_hms(2024, 6, 15, 12, 30, 0).unwrap();
    let dto = review_entry_to_dto(sample_entry(Some(resolved)));
    assert_eq!(dto.resolved_at.as_deref(), Some("2024-06-15T12:30:00+00:00"));
}

#[test]
fn review_entry_to_dto_preserves_enrichment_fields() {
    let dto = review_entry_to_dto(sample_entry(None));
    assert_eq!(dto.action_type.as_deref(), Some("ban"));
    assert_eq!(dto.target_name.as_deref(), Some("bob"));
    assert_eq!(dto.action_reason.as_deref(), Some("spam"));
}

#[test]
fn review_entry_to_dto_preserves_pending_status() {
    let dto = review_entry_to_dto(sample_entry(None));
    assert_eq!(dto.status, "pending");
    assert!(dto.reviewer_id.is_none());
}
