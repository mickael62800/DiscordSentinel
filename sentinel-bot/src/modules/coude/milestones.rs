//! Rendu des paliers visibles (cf. COUPE_AMELIORATIONS section 3.2).
//!
//! Le bareme (table niveau -> deblocage) et la regle du cooldown /repos
//! effectif vivent desormais cote API (sentinel-core). Le bot ne fait plus
//! que l'affichage a partir de la `PlayerProgression` renvoyee par l'API.

use crate::modules::coude::api_client::PlayerProgression;

/// Resume compact pour /profil : liste des paliers debloques + prochain a
/// viser. Rendu pur a partir des donnees server-side.
pub fn format_profile_section(progression: &PlayerProgression) -> String {
    let unlocked: Vec<_> = progression
        .milestones
        .iter()
        .filter(|m| m.unlocked)
        .collect();

    let unlocked_line = if unlocked.is_empty() {
        "_Aucun palier debloque pour l instant._".to_string()
    } else {
        unlocked
            .iter()
            .map(|m| format!("{} **{}**", m.emoji, m.label))
            .collect::<Vec<_>>()
            .join(" · ")
    };

    match &progression.next_milestone {
        Some(m) => format!(
            "{}\n\n\u{1f3af} Prochain : niveau **{}** -> {} {} ({})",
            unlocked_line, m.level, m.emoji, m.label, m.description
        ),
        None => format!(
            "{}\n\n\u{1f3c6} Tous les paliers debloques !",
            unlocked_line
        ),
    }
}

#[cfg(test)]
mod tests;
