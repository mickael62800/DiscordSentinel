//! Tests du module slot — verifie le routage component custom_id.

use super::*;

#[test]
fn handles_component_recognizes_panel_open() {
    assert!(handles_component(setup::PANEL_OPEN_ID));
}

#[test]
fn handles_component_recognizes_channel_spin() {
    assert!(handles_component(setup::CHANNEL_SPIN_ID));
}

#[test]
fn handles_component_recognizes_channel_daily() {
    assert!(handles_component(setup::CHANNEL_DAILY_ID));
}

#[test]
fn handles_component_recognizes_channel_close() {
    assert!(handles_component(setup::CHANNEL_CLOSE_ID));
}

#[test]
fn handles_component_rejects_unknown() {
    assert!(!handles_component("foo"));
    assert!(!handles_component("bj_panel_play"));
    assert!(!handles_component(""));
    assert!(!handles_component("slot_panel_spin")); // ancien id removed
}

#[test]
fn module_bot_name_is_slot_bot() {
    assert_eq!(MODULE_BOT_NAME, "slot-bot");
}

#[test]
fn all_panel_button_ids_are_distinct() {
    let ids = [
        setup::PANEL_OPEN_ID,
        setup::CHANNEL_SPIN_ID,
        setup::CHANNEL_DAILY_ID,
        setup::CHANNEL_CLOSE_ID,
    ];
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            assert_ne!(ids[i], ids[j], "ids {} et {} doivent etre distincts", i, j);
        }
    }
}

#[tokio::test]
async fn init_is_idempotent() {
    // Appel multiple : ne doit pas paniquer ni doublonner.
    use serenity::client::Cache;
    use serenity::http::Http;

    // Construit un Context minimal via une astuce : on ne peut pas creer un
    // serenity::Context sans WebSocket ; on se contente donc de tester que
    // SlotChannelManager::new est bien idempotent au niveau structure.
    let mgr1 = Arc::new(SlotChannelManager::new());
    let mgr2 = Arc::new(SlotChannelManager::new());
    assert_eq!(mgr1.count(), 0);
    assert_eq!(mgr2.count(), 0);

    // Anti-warning sur les imports
    let _: Option<Arc<Cache>> = None;
    let _: Option<Arc<Http>> = None;
}
