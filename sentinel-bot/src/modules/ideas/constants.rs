//! Identifiants des composants et categories du module idees.
//!
//! Les `custom_id` sont des constantes : ils transitent dans les interactions
//! Discord et doivent rester stables (un panneau deja poste continue de vivre).

/// Bouton du panneau public.
pub const PANEL_BUTTON_ID: &str = "idea_panel_open";
/// Menu de choix de la categorie d'idee.
pub const CATEGORY_SELECT_ID: &str = "idea_category_select";
/// Prefixe du modal de proposition : `idea_modal_<categorie>`.
pub const MODAL_ID_PREFIX: &str = "idea_modal_";
/// Prefixe du modal de motif d'une decision : `idea_reason_<statut>`.
pub const REASON_MODAL_PREFIX: &str = "idea_reason_";

/// Boutons de decision, reserves au role staff.
pub const DISCUSS_BUTTON_ID: &str = "idea_status_discuss";
pub const ACCEPT_BUTTON_ID: &str = "idea_status_accept";
pub const REFUSE_BUTTON_ID: &str = "idea_status_refuse";
pub const DONE_BUTTON_ID: &str = "idea_status_done";

/// Champs de la modale de proposition.
pub const FIELD_TITLE: &str = "idea_title";
pub const FIELD_DESCRIPTION: &str = "idea_description";
/// Champ de la modale de motif.
pub const FIELD_REASON: &str = "idea_reason";

/// Categories proposees dans le menu : (valeur, libelle, description).
pub const IDEA_CATEGORIES: &[(&str, &str, &str)] = &[
    (
        "evenement",
        "Evenement",
        "Une animation, un concours, une soiree a organiser",
    ),
    (
        "salon",
        "Salon / categorie",
        "Un nouveau salon ou une reorganisation",
    ),
    ("role", "Role", "Un nouveau role ou un changement de roles"),
    (
        "bot",
        "Bot / fonctionnalite",
        "Une commande ou une automatisation a ajouter",
    ),
    (
        "reglement",
        "Reglement",
        "Une regle a ajouter, changer ou clarifier",
    ),
    ("autre", "Autre", "Tout le reste"),
];

/// Libelle affichable d'une categorie (retombe sur la valeur brute).
pub fn category_label(value: &str) -> &str {
    IDEA_CATEGORIES
        .iter()
        .find(|(v, _, _)| *v == value)
        .map(|(_, l, _)| *l)
        .unwrap_or(value)
}

/// Statut vise par chaque bouton de decision.
pub fn status_for_button(custom_id: &str) -> Option<&'static str> {
    match custom_id {
        DISCUSS_BUTTON_ID => Some("en_discussion"),
        ACCEPT_BUTTON_ID => Some("acceptee"),
        REFUSE_BUTTON_ID => Some("refusee"),
        DONE_BUTTON_ID => Some("realisee"),
        _ => None,
    }
}

/// Libelle affichable d'un statut.
pub fn status_label(status: &str) -> &str {
    match status {
        "nouvelle" => "Nouvelle",
        "en_discussion" => "En discussion",
        "acceptee" => "Acceptee",
        "refusee" => "Refusee",
        "realisee" => "Realisee",
        other => other,
    }
}

/// Cle de config de la couleur associee a un statut, et couleur par defaut.
pub fn status_color_config(status: &str) -> (&'static str, &'static str) {
    match status {
        "acceptee" => ("color_accepted", "2ecc71"),
        "refusee" => ("color_refused", "e74c3c"),
        "realisee" => ("color_done", "9b59b6"),
        // "nouvelle" et "en_discussion" partagent la couleur d'ouverture.
        _ => ("color_new", "3498db"),
    }
}
