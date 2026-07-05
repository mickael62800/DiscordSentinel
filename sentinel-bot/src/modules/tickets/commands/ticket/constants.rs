// ── Constantes pour les custom_id des boutons/menus ──
pub const PANEL_BUTTON_ID: &str = "sentinel_ticket_create";
pub const TYPE_SELECT_ID: &str = "sentinel_ticket_type";
pub const MODAL_ID_PREFIX: &str = "sentinel_ticket_modal:";
pub const CLOSE_BUTTON_ID: &str = "sentinel_ticket_close";
pub const INVITE_BUTTON_ID: &str = "sentinel_ticket_invite";
pub const INVITE_SELECT_ID: &str = "sentinel_ticket_invite_select";
pub const VOCAL_BUTTON_ID: &str = "sentinel_ticket_vocal";
pub const VOCAL_USER_ACCEPT_ID: &str = "sentinel_ticket_vocal_user_accept";
pub const VOCAL_USER_DECLINE_ID: &str = "sentinel_ticket_vocal_user_decline";
pub const CLOSE_CONFIRM_ID: &str = "sentinel_ticket_close_confirm";
pub const CLOSE_CANCEL_ID: &str = "sentinel_ticket_close_cancel";

/// Types de tickets qui restreignent la visibilite aux admins uniquement (pas les modos)
pub const ADMIN_ONLY_TYPES: &[&str] = &["probleme_moderateur"];

/// Types de tickets a priorite urgente automatique
pub const URGENT_TYPES: &[&str] = &["urgence_detresse"];

pub const TICKET_TYPES: &[(&str, &str, &str)] = &[
    (
        "probleme_serveur",
        "Probleme serveur",
        "Un souci technique ou de configuration du serveur",
    ),
    (
        "probleme_membre",
        "Probleme avec un membre",
        "Signaler le comportement d'un membre",
    ),
    (
        "probleme_moderateur",
        "Probleme avec un moderateur",
        "Signaler un abus ou probleme avec un moderateur (confidentiel, remonte aux proprietaires du serveur)",
    ),
    (
        "appel_sanction",
        "Appel de sanction",
        "Contester une sanction recue",
    ),
    (
        "urgence_detresse",
        "Situation urgente / detresse",
        "Vous traversez une situation grave et avez besoin d'aide rapidement",
    ),
    ("question", "Question", "Poser une question au staff"),
    (
        "autre",
        "Autre",
        "Demande qui ne rentre pas dans les autres categories",
    ),
];
