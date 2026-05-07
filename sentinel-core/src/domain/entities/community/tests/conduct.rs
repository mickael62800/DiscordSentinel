use super::*;

fn default_cfg() -> ConductConfig {
    ConductConfig::default_for_guild("g1")
}

#[test]
fn default_for_guild_sets_sensible_values() {
    let c = default_cfg();
    assert_eq!(c.guild_id, "g1");
    assert_eq!(c.max_points, 12);
    assert_eq!(c.regen_amount, 1);
    assert_eq!(c.regen_interval, "weekly");
    assert_eq!(c.penalty_warn, 1);
    assert_eq!(c.penalty_delete, 2);
    assert_eq!(c.penalty_mute, 3);
    assert_eq!(c.penalty_ban, 6);
}

#[test]
fn penalty_for_action_warn() {
    assert_eq!(default_cfg().penalty_for_action("warn"), 1);
}

#[test]
fn penalty_for_action_delete() {
    assert_eq!(default_cfg().penalty_for_action("delete"), 2);
}

#[test]
fn penalty_for_action_mute_variants() {
    let c = default_cfg();
    assert_eq!(c.penalty_for_action("mute"), 3);
    assert_eq!(c.penalty_for_action("mute_temp"), 3);
}

#[test]
fn penalty_for_action_ban_variants() {
    let c = default_cfg();
    assert_eq!(c.penalty_for_action("ban"), 6);
    assert_eq!(c.penalty_for_action("ban_permanent"), 6);
    assert_eq!(c.penalty_for_action("ban_temp"), 6);
}

#[test]
fn penalty_for_action_unknown_returns_zero() {
    let c = default_cfg();
    assert_eq!(c.penalty_for_action(""), 0);
    assert_eq!(c.penalty_for_action("kick"), 0);
    assert_eq!(c.penalty_for_action("WARN"), 0); // case-sensitive
}

#[test]
fn penalty_for_action_unknown_with_subscriber_hits_tracing_branch() {
    // Couvre la branche d'expansion du tracing::warn! (100% coverage : la
    // macro contient un test d'enabled interne non execute sans subscriber).
    use tracing_subscriber::fmt::MakeWriter;
    use std::sync::Arc;
    use std::sync::Mutex;
    // Writer in-memory pour eviter le spam stdout pendant les tests.
    #[derive(Clone, Default)]
    struct SinkWriter(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for SinkWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf); Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
    }
    impl<'a> MakeWriter<'a> for SinkWriter {
        type Writer = SinkWriter;
        fn make_writer(&'a self) -> Self::Writer { self.clone() }
    }

    let writer = SinkWriter::default();
    let sub = tracing_subscriber::fmt()
        .with_writer(writer.clone())
        .with_max_level(tracing::Level::WARN)
        .finish();
    let _guard = tracing::subscriber::set_default(sub);

    let c = default_cfg();
    assert_eq!(c.penalty_for_action("unknown"), 0);
    // Le buffer doit contenir le log de warn (la branche d'enabled a ete hit).
    let buf = writer.0.lock().unwrap();
    let s = String::from_utf8_lossy(&buf);
    assert!(s.contains("Action inconnue") || !buf.is_empty());
}

#[test]
fn penalty_uses_configured_values_not_hardcoded() {
    let mut c = default_cfg();
    c.penalty_warn = 42;
    c.penalty_ban = 99;
    assert_eq!(c.penalty_for_action("warn"), 42);
    assert_eq!(c.penalty_for_action("ban"), 99);
}

#[test]
fn penalty_gradient_escalates() {
    // Invariant metier : warn < delete < mute < ban
    let c = default_cfg();
    assert!(c.penalty_warn < c.penalty_delete);
    assert!(c.penalty_delete < c.penalty_mute);
    assert!(c.penalty_mute < c.penalty_ban);
}


// -- Extractions (MUTE_AT_ZERO_POINTS_DURATION_MINS + apply_conduct_penalty/regen) --

#[test]
fn mute_duration_constant_is_10_minutes() {
    assert_eq!(MUTE_AT_ZERO_POINTS_DURATION_MINS, 10);
}

#[test]
fn apply_penalty_clamps_at_zero() {
    assert_eq!(apply_conduct_penalty(10, 3), 7);
    assert_eq!(apply_conduct_penalty(5, 10), 0); // clamp
    assert_eq!(apply_conduct_penalty(0, 5), 0);
}

#[test]
fn apply_penalty_zero_is_noop() {
    assert_eq!(apply_conduct_penalty(7, 0), 7);
}

#[test]
fn apply_regen_clamps_at_max() {
    assert_eq!(apply_conduct_regen(5, 3, 12), 8);
    assert_eq!(apply_conduct_regen(10, 5, 12), 12); // clamp
    assert_eq!(apply_conduct_regen(12, 1, 12), 12); // deja max
}

#[test]
fn apply_regen_zero_amount_is_noop() {
    assert_eq!(apply_conduct_regen(5, 0, 12), 5);
}
