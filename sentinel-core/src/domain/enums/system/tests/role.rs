//! Tests de la hierarchie RBAC (roundtrip + relation d'ordre `satisfies`).

use super::*;

#[test]
fn from_str_roundtrip_all() {
    for r in [Role::Viewer, Role::Moderator, Role::Admin, Role::Owner] {
        assert_eq!(Role::from_str(r.as_str()), Some(r));
    }
}

#[test]
fn from_str_unknown_is_none() {
    assert_eq!(Role::from_str("root"), None);
    assert_eq!(Role::from_str(""), None);
    assert_eq!(Role::from_str("Admin"), None); // sensible a la casse
}

#[test]
fn ordering_matches_hierarchy() {
    assert!(Role::Viewer < Role::Moderator);
    assert!(Role::Moderator < Role::Admin);
    assert!(Role::Admin < Role::Owner);
}

#[test]
fn satisfies_at_or_above_required() {
    // Owner satisfait tout.
    for req in [Role::Viewer, Role::Moderator, Role::Admin, Role::Owner] {
        assert!(Role::Owner.satisfies(req));
    }
    // Egalite satisfait.
    assert!(Role::Moderator.satisfies(Role::Moderator));
    // En dessous ne satisfait pas.
    assert!(!Role::Viewer.satisfies(Role::Moderator));
    assert!(!Role::Moderator.satisfies(Role::Admin));
    assert!(!Role::Admin.satisfies(Role::Owner));
}
