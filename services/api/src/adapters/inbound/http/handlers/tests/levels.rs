use super::*;

#[test]
fn delete_reward_params_no_source_default_none() {
    let p: DeleteRewardParams = serde_json::from_str(r#"{}"#).unwrap();
    assert!(p.source.is_none());
}

#[test]
fn delete_reward_params_with_text_source() {
    let p: DeleteRewardParams = serde_json::from_str(r#"{"source":"text"}"#).unwrap();
    assert_eq!(p.source.as_deref(), Some("text"));
}

#[test]
fn delete_reward_params_with_voice_source() {
    let p: DeleteRewardParams = serde_json::from_str(r#"{"source":"voice"}"#).unwrap();
    assert_eq!(p.source.as_deref(), Some("voice"));
}

#[test]
fn delete_reward_params_null_source() {
    let p: DeleteRewardParams = serde_json::from_str(r#"{"source":null}"#).unwrap();
    assert!(p.source.is_none());
}
