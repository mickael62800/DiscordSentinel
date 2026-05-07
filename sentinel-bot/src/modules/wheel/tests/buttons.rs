use super::*;

#[test]
fn humanizes_already_claimed() {
    let raw = r#"Erreur API 400: {"error":"Tu as deja tire la Roue du Destin aujourd hui."}"#;
    let msg = humanize_api_error(raw);
    assert!(msg.contains("deja tire"));
    assert!(msg.contains("demain"));
}

#[test]
fn humanizes_module_disabled() {
    let raw = r#"Erreur API 400: {"error":"Module desactive"}"#;
    let msg = humanize_api_error(raw);
    assert!(msg.contains("desactive"));
}

#[test]
fn humanizes_unknown_to_generic() {
    let raw = "Erreur API 500: timeout";
    let msg = humanize_api_error(raw);
    assert!(msg.contains("Erreur") && msg.contains("Reessaie"));
}

#[test]
fn format_payout_positive() {
    assert_eq!(format_payout(5000), "+5000c");
}

#[test]
fn format_payout_negative() {
    assert_eq!(format_payout(-500), "-500c");
}

#[test]
fn format_payout_zero() {
    assert_eq!(format_payout(0), "0c");
}

#[test]
fn animation_duration_is_4_seconds() {
    assert_eq!(SPIN_ANIMATION_MS, 4000);
}
