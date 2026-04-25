//! Tests du module slot — verifie le routage component custom_id.

use super::*;

#[test]
fn handles_component_recognizes_panel_spin() {
    assert!(handles_component(setup::PANEL_SPIN_ID));
}

#[test]
fn handles_component_recognizes_panel_daily() {
    assert!(handles_component(setup::PANEL_DAILY_ID));
}

#[test]
fn handles_component_rejects_unknown() {
    assert!(!handles_component("foo"));
    assert!(!handles_component("bj_panel_play"));
    assert!(!handles_component(""));
}

#[test]
fn module_bot_name_is_slot_bot() {
    assert_eq!(MODULE_BOT_NAME, "slot-bot");
}

#[test]
fn panel_button_ids_are_distinct() {
    assert_ne!(setup::PANEL_SPIN_ID, setup::PANEL_DAILY_ID);
}
