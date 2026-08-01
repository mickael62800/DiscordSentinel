use super::*;

const GUILDE: &str = "1509157282636890265";

#[test]
fn extrait_un_identifiant_de_chemin() {
    assert_eq!(
        guild_id_from_path(&format!("/api/public/news/{GUILDE}")),
        Some(GUILDE.to_string())
    );
}

#[test]
fn extrait_meme_au_milieu_du_chemin() {
    assert_eq!(
        guild_id_from_path(&format!("/api/levels/{GUILDE}/leaderboard")),
        Some(GUILDE.to_string())
    );
}

#[test]
fn chemin_sans_identifiant_ne_donne_rien() {
    assert_eq!(guild_id_from_path("/api/guilds"), None);
    assert_eq!(guild_id_from_path("/health"), None);
    assert_eq!(guild_id_from_path("/"), None);
}

/// Un uuid ne doit pas etre pris pour un identifiant de serveur : le
/// confondre ferait refuser une route de detail parfaitement legitime.
#[test]
fn un_uuid_n_est_pas_pris_pour_une_guilde() {
    assert_eq!(
        guild_id_from_path("/api/news/detail/3f2504e0-4f89-11d3-9a0c-0305e82c3301"),
        None
    );
}

/// Ici un faux positif provoque un REFUS : on prefere ignorer un nombre
/// trop court plutot que bloquer une route valide.
#[test]
fn un_nombre_trop_court_est_ignore() {
    assert_eq!(guild_id_from_path("/api/events/detail/42"), None);
    assert_eq!(guild_id_from_path("/api/levels/12345/leaderboard"), None);
}

#[test]
fn un_nombre_trop_long_est_ignore() {
    let trop_long = "1".repeat(21);
    assert_eq!(guild_id_from_path(&format!("/api/x/{trop_long}")), None);
}

#[test]
fn un_segment_alphanumerique_est_ignore() {
    assert_eq!(
        guild_id_from_path("/api/x/1509157282636890a65"),
        None,
        "un segment contenant une lettre n'est pas un snowflake"
    );
}

/// Le premier segment plausible gagne : les routes reelles portent le
/// `guild_id` avant tout autre identifiant numerique.
#[test]
fn le_premier_segment_plausible_est_retenu() {
    let autre = "9999999999999999999";
    assert_eq!(
        guild_id_from_path(&format!("/api/x/{GUILDE}/y/{autre}")),
        Some(GUILDE.to_string())
    );
}
