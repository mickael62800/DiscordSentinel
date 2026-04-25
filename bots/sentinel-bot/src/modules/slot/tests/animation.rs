use super::*;

fn final_3() -> [String; 3] {
    ["🍒".into(), "🍋".into(), "🍊".into()]
}

#[test]
fn frame_zero_all_spinning() {
    let f = frame_symbols(&final_3(), 0);
    assert_eq!(f[0], SPINNING_PLACEHOLDER);
    assert_eq!(f[1], SPINNING_PLACEHOLDER);
    assert_eq!(f[2], SPINNING_PLACEHOLDER);
}

#[test]
fn frame_one_reveals_first_only() {
    let f = frame_symbols(&final_3(), 1);
    assert_eq!(f[0], "🍒");
    assert_eq!(f[1], SPINNING_PLACEHOLDER);
    assert_eq!(f[2], SPINNING_PLACEHOLDER);
}

#[test]
fn frame_two_reveals_first_two() {
    let f = frame_symbols(&final_3(), 2);
    assert_eq!(f[0], "🍒");
    assert_eq!(f[1], "🍋");
    assert_eq!(f[2], SPINNING_PLACEHOLDER);
}

#[test]
fn frame_three_reveals_all() {
    let f = frame_symbols(&final_3(), 3);
    assert_eq!(f[0], "🍒");
    assert_eq!(f[1], "🍋");
    assert_eq!(f[2], "🍊");
}

#[test]
fn frame_above_three_clamps_to_three() {
    let f = frame_symbols(&final_3(), 99);
    assert_eq!(f[0], "🍒");
    assert_eq!(f[1], "🍋");
    assert_eq!(f[2], "🍊");
}

#[test]
fn total_reveal_frames_is_three() {
    assert_eq!(TOTAL_REVEAL_FRAMES, 3);
}

#[test]
fn frame_delay_is_two_seconds() {
    assert_eq!(FRAME_DELAY_MS, 2000);
}

#[test]
fn animation_total_duration_is_six_seconds() {
    let total_ms = TOTAL_REVEAL_FRAMES as u64 * FRAME_DELAY_MS;
    assert_eq!(total_ms, 6000);
}

#[test]
fn frame_zero_returns_three_placeholders_only() {
    let f = frame_symbols(&final_3(), 0);
    assert_eq!(f.len(), 3);
    assert!(f.iter().all(|s| s == SPINNING_PLACEHOLDER));
}

#[test]
fn frames_form_progressive_reveal_sequence() {
    // Verifie que la sequence frame 0 -> 1 -> 2 -> 3 est strictement croissante
    // en nb de symboles reveles.
    let final_syms = final_3();
    let count_revealed = |f: &[String; 3]| {
        f.iter().filter(|s| s.as_str() != SPINNING_PLACEHOLDER).count()
    };
    let f0 = frame_symbols(&final_syms, 0);
    let f1 = frame_symbols(&final_syms, 1);
    let f2 = frame_symbols(&final_syms, 2);
    let f3 = frame_symbols(&final_syms, 3);
    assert_eq!(count_revealed(&f0), 0);
    assert_eq!(count_revealed(&f1), 1);
    assert_eq!(count_revealed(&f2), 2);
    assert_eq!(count_revealed(&f3), 3);
}
