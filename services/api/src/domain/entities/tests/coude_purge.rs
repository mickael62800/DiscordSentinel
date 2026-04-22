use super::*;

#[test]
fn purge_tables_contains_seven_entries() {
    assert_eq!(COUDE_PURGE_TABLES.len(), 7);
}

#[test]
fn purge_tables_cover_core_coude_data() {
    let t: Vec<&str> = COUDE_PURGE_TABLES.to_vec();
    assert!(t.contains(&"coude_players"));
    assert!(t.contains(&"coude_combats"));
    assert!(t.contains(&"coude_bets"));
    assert!(t.contains(&"coude_insurances"));
    assert!(t.contains(&"coude_inventory"));
    assert!(t.contains(&"coude_primes"));
    assert!(t.contains(&"coude_events"));
}

#[test]
fn purge_order_bets_before_combats() {
    // Integrite referentielle : on supprime les paris avant le combat qu'ils referencent.
    let bets = COUDE_PURGE_TABLES.iter().position(|&t| t == "coude_bets").unwrap();
    let combats = COUDE_PURGE_TABLES.iter().position(|&t| t == "coude_combats").unwrap();
    assert!(bets < combats, "bets doivent etre purges avant combats");
}

#[test]
fn purge_order_players_last() {
    // Les joueurs sont referenced par tout le reste -> en dernier.
    let players = COUDE_PURGE_TABLES.iter().position(|&t| t == "coude_players").unwrap();
    assert_eq!(players, COUDE_PURGE_TABLES.len() - 1);
}
