use super::*;
use crate::domain::entities::UserNote;
use chrono::Utc;
use uuid::Uuid;

#[test]
fn default_category_is_general() {
    assert_eq!(default_category(), "general");
}

#[test]
fn add_note_dto_to_command_preserves_fields() {
    let dto = AddNoteDto {
        guild_id: "g".into(),
        user_id: "u".into(),
        author_id: "mod".into(),
        author_name: "Mod".into(),
        content: "some note".into(),
        category: "security".into(),
    };
    let cmd: AddNoteCommand = dto.into();
    assert_eq!(cmd.guild_id, "g");
    assert_eq!(cmd.user_id, "u");
    assert_eq!(cmd.content, "some note");
    assert_eq!(cmd.category, "security");
}

#[test]
fn user_note_to_dto_formats_dates_rfc3339() {
    let note = UserNote {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        user_id: "u".into(),
        author_id: "mod".into(),
        author_name: "Mod".into(),
        content: "hi".into(),
        category: "general".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let dto: UserNoteDto = note.into();
    assert!(dto.created_at.contains('T'));
    assert!(dto.updated_at.contains('T'));
    assert_eq!(dto.content, "hi");
    assert_eq!(dto.category, "general");
}
