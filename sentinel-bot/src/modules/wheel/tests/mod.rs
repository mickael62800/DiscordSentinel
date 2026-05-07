use super::*;

#[test]
fn module_bot_name_is_wheel_bot() {
    assert_eq!(MODULE_BOT_NAME, "wheel-bot");
}

#[test]
fn handles_panel_spin() {
    assert!(handles_component(setup::PANEL_SPIN_ID));
}

#[test]
fn rejects_unknown_id() {
    assert!(!handles_component("foo"));
    assert!(!handles_component(""));
    assert!(!handles_component("slot_panel_open"));
}
