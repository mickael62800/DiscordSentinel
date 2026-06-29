//! Petites correspondances action <-> caractere <-> libelle francais.

use super::super::api_client::Action;

pub(super) fn action_char(action: &Action) -> char {
    match action {
        Action::Warn => 'w',
        Action::Delete => 'd',
        Action::Mute => 'm',
        Action::Ban => 'b',
        Action::None => 'i',
    }
}

pub(super) fn char_to_str(c: char) -> &'static str {
    match c {
        'p' => "prevention",
        'w' => "warn",
        'd' => "delete",
        'm' => "mute",
        'b' => "ban",
        _ => "ignore",
    }
}

pub(super) fn action_label(s: &str) -> &'static str {
    match s {
        "prevention" => "Prevention",
        "warn" => "Avertissement",
        "delete" => "Suppression",
        "mute" => "Mute",
        "ban" => "Bannissement",
        _ => "Ignorer",
    }
}
