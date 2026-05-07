//! Tests des helpers du module buttons (humanize errors + action_buttons_row).

use super::*;

#[test]
fn humanizes_cooldown_error() {
    let raw = r#"Erreur API 400 POST /api/slot/g/spin: {"error":"Cooldown actif : encore 3 secondes"}"#;
    let msg = humanize_api_error(raw);
    assert!(msg.contains("Cooldown"));
    assert!(msg.contains("3 secondes"));
}

#[test]
fn humanizes_mise_out_of_range() {
    let raw = r#"Erreur API 400: {"error":"Mise hors borne (autorise : 10 - 1000)"}"#;
    let msg = humanize_api_error(raw);
    assert!(msg.contains("Mise hors borne"));
    assert!(msg.contains("10"));
    assert!(msg.contains("1000"));
}

#[test]
fn humanizes_insufficient_balance() {
    let raw = r#"Erreur API 400: {"error":"ValidationError(\"Solde insuffisant: tu as 50 coins\")"}"#;
    let msg = humanize_api_error(raw);
    assert!(msg.contains("Solde insuffisant"));
}

#[test]
fn humanizes_daily_disabled() {
    let raw = r#"Erreur API 400: {"error":"Daily bonus desactive sur ce serveur"}"#;
    let msg = humanize_api_error(raw);
    assert!(msg.contains("Daily bonus") && msg.contains("desactive"));
}

#[test]
fn humanizes_daily_already_claimed() {
    let raw = r#"Erreur API 400: {"error":"Daily bonus deja reclame aujourd hui"}"#;
    let msg = humanize_api_error(raw);
    assert!(msg.contains("deja reclame"));
}

#[test]
fn humanizes_invalid_config() {
    let raw = r#"Erreur API 400: {"error":"Config slot-bot invalide : poids zero"}"#;
    let msg = humanize_api_error(raw);
    assert!(msg.contains("Configuration slot invalide"));
}

#[test]
fn humanizes_unknown_error_to_generic() {
    let raw = "Erreur API 500: timeout";
    let msg = humanize_api_error(raw);
    assert!(msg.contains("Erreur") && msg.contains("Reessaie"));
}

#[test]
fn action_buttons_row_has_three_buttons() {
    let rows = action_buttons_row();
    assert_eq!(rows.len(), 1, "1 ActionRow attendue");
    // Verifie qu on a bien 3 boutons via la serialisation
    let json = serde_json::to_string(&rows[0]).unwrap();
    // Compte le nombre de "custom_id"
    let count = json.matches("custom_id").count();
    assert_eq!(count, 3, "3 boutons attendus dans la row");
}

#[test]
fn action_buttons_row_contains_all_three_ids() {
    let rows = action_buttons_row();
    let json = serde_json::to_string(&rows[0]).unwrap();
    assert!(json.contains(super::setup::CHANNEL_SPIN_ID));
    assert!(json.contains(super::setup::CHANNEL_DAILY_ID));
    assert!(json.contains(super::setup::CHANNEL_CLOSE_ID));
}
