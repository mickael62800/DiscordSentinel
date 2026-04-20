use super::*;
use crate::domain::entities::IaConfig;

#[test]
fn ia_config_to_dto_preserves_all_fields() {
    let c = IaConfig {
        guild_id: "g".into(),
        text_enabled: true,
        text_threshold: 0.8,
        vision_enabled: false,
        vision_threshold: 0.6,
        context_dampening: 0.3,
        context_format: "natural".into(),
        context_max_messages: 5,
        context_max_chars: 500,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let dto: IaConfigDto = c.into();
    assert_eq!(dto.guild_id, "g");
    assert!(dto.text_enabled);
    assert!(!dto.vision_enabled);
    assert_eq!(dto.text_threshold, 0.8);
    assert_eq!(dto.vision_threshold, 0.6);
    assert_eq!(dto.context_dampening, 0.3);
    assert_eq!(dto.context_format, "natural");
    assert_eq!(dto.context_max_messages, 5);
    assert_eq!(dto.context_max_chars, 500);
}

#[test]
fn ia_config_dto_preserves_edge_thresholds() {
    let c = IaConfig {
        guild_id: "g".into(),
        text_enabled: true,
        text_threshold: 0.0,
        vision_enabled: true,
        vision_threshold: 1.0,
        context_dampening: 0.0,
        context_format: "tagged".into(),
        context_max_messages: 0,
        context_max_chars: 0,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let dto: IaConfigDto = c.into();
    assert_eq!(dto.text_threshold, 0.0);
    assert_eq!(dto.vision_threshold, 1.0);
    assert_eq!(dto.context_max_messages, 0);
}
