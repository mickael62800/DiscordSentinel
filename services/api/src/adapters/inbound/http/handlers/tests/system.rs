use super::*;

#[test]
fn parse_redis_info_handles_basic_fields() {
    let raw = "# Memory\nused_memory:1048576\nconnected_clients:42\nuptime_in_seconds:300\n";
    let m = parse_redis_info(raw);
    assert_eq!(m.used_memory_mb, 1);
    assert_eq!(m.connected_clients, 42);
    assert_eq!(m.uptime_seconds, 300);
}

#[test]
fn parse_redis_info_sums_keys_across_dbs() {
    let raw = "db0:keys=100,expires=10,avg_ttl=0\ndb1:keys=50,expires=5,avg_ttl=0\n";
    let m = parse_redis_info(raw);
    assert_eq!(m.total_keys, 150);
}

#[test]
fn parse_redis_info_ignores_comments_and_blank_lines() {
    let raw = "\n# Server\n\nused_memory:2097152\n# more comments\n";
    let m = parse_redis_info(raw);
    assert_eq!(m.used_memory_mb, 2);
}

#[test]
fn parse_redis_info_ignores_unknown_fields() {
    let raw = "some_other_field:xyz\nused_memory:1048576\n";
    let m = parse_redis_info(raw);
    assert_eq!(m.used_memory_mb, 1);
    assert_eq!(m.connected_clients, 0);
}

#[test]
fn parse_redis_info_handles_malformed_values_gracefully() {
    let raw = "connected_clients:not_a_number\nused_memory:also_bad\n";
    let m = parse_redis_info(raw);
    assert_eq!(m.connected_clients, 0);
    assert_eq!(m.used_memory_mb, 0);
}

#[test]
fn parse_redis_info_empty_input() {
    let m = parse_redis_info("");
    assert_eq!(m.used_memory_mb, 0);
    assert_eq!(m.connected_clients, 0);
    assert_eq!(m.uptime_seconds, 0);
    assert_eq!(m.total_keys, 0);
}

#[test]
fn parse_redis_info_db_without_keys_prefix_ignored() {
    let raw = "db0:expires=10,avg_ttl=0\n";
    let m = parse_redis_info(raw);
    assert_eq!(m.total_keys, 0);
}

#[test]
fn parse_redis_info_used_memory_rounds_down() {
    // 1.5 Mo = 1 572 864 bytes → 1 Mo (division entiere)
    let raw = "used_memory:1572864\n";
    let m = parse_redis_info(raw);
    assert_eq!(m.used_memory_mb, 1);
}

#[test]
fn uptime_seconds_initializes_and_returns_value() {
    // Le premier appel initialise STARTED_AT ; les suivants doivent reutiliser.
    let a = uptime_seconds();
    let b = uptime_seconds();
    assert!(b >= a);
}

#[test]
fn record_startup_is_idempotent() {
    // OnceLock → le 2e set est ignore silencieusement.
    record_startup();
    record_startup();
    // Pas de panique attendue.
}
