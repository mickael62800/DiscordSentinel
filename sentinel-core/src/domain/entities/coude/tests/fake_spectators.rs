use super::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

#[test]
fn picks_between_3_and_5() {
    let mut rng = StdRng::seed_from_u64(0);
    for _ in 0..100 {
        let chat = pick_spectator_chat(&mut rng, "A", "B", Some("A"), Some("B"));
        assert!(chat.len() >= SPECTATOR_COUNT_MIN);
        assert!(chat.len() <= SPECTATOR_COUNT_MAX);
    }
}

#[test]
fn pseudos_are_distinct_in_a_single_chat() {
    let mut rng = StdRng::seed_from_u64(7);
    for _ in 0..50 {
        let chat = pick_spectator_chat(&mut rng, "A", "B", Some("A"), Some("B"));
        let mut users: Vec<&str> = chat.iter().map(|(u, _)| u.as_str()).collect();
        users.sort();
        let before = users.len();
        users.dedup();
        assert_eq!(users.len(), before, "spectateurs doublons dans un meme chat");
    }
}

#[test]
fn substitutes_all_placeholders() {
    let mut rng = StdRng::seed_from_u64(42);
    for _ in 0..100 {
        let chat = pick_spectator_chat(&mut rng, "Alice", "Bob", Some("Alice"), Some("Bob"));
        for (_, line) in &chat {
            assert!(!line.contains("{atk}"));
            assert!(!line.contains("{def}"));
            assert!(!line.contains("{winner}"));
            assert!(!line.contains("{loser}"));
        }
    }
}

#[test]
fn handles_draw_with_none_winner_loser() {
    let mut rng = StdRng::seed_from_u64(13);
    let chat = pick_spectator_chat(&mut rng, "A", "B", None, None);
    for (_, line) in &chat {
        assert!(!line.contains("{winner}"));
        assert!(!line.contains("{loser}"));
    }
}

#[test]
fn at_least_100_usernames_in_catalog() {
    assert!(SPECTATOR_USERNAMES.len() >= 100, "got {}", SPECTATOR_USERNAMES.len());
}

#[test]
fn at_least_30_lines_in_catalog() {
    assert!(SPECTATOR_LINES.len() >= 30, "got {}", SPECTATOR_LINES.len());
}

#[test]
fn no_template_has_unbalanced_placeholders() {
    for tmpl in SPECTATOR_LINES {
        let cleaned = tmpl
            .replace("{atk}", "")
            .replace("{def}", "")
            .replace("{winner}", "")
            .replace("{loser}", "");
        assert!(!cleaned.contains('{'), "template avec placeholder inconnu : {tmpl}");
        assert!(!cleaned.contains('}'), "template avec placeholder inconnu : {tmpl}");
    }
}

#[test]
fn format_chat_uses_emoji_and_brackets() {
    let chat = vec![
        ("Kevin".to_string(), "MDR".to_string()),
        ("Mama".to_string(), "GG".to_string()),
    ];
    let s = format_spectator_chat(&chat);
    assert!(s.contains("💬"));
    assert!(s.contains("[Kevin]"));
    assert!(s.contains("MDR"));
    assert!(s.contains("[Mama]"));
}

#[test]
fn distribution_uses_many_different_pseudos() {
    let mut rng = StdRng::seed_from_u64(123);
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for _ in 0..200 {
        for (u, _) in pick_spectator_chat(&mut rng, "A", "B", Some("A"), Some("B")) {
            seen.insert(u);
        }
    }
    assert!(seen.len() >= 50, "doit varier sur 200 chats (got {})", seen.len());
}
