//! Catalogue + helpers purs pour la commande /travaux (Phase 2 #2 audit).
//!
//! Ces taches communautaires ne sont disponibles que pendant la prison.
//! Le RNG est cote API : choix de tache + succes/echec + montant + flavor.

/// Cooldown entre deux tentatives (2h en secondes).
pub const TRAVAUX_COOLDOWN_SECS: i64 = 2 * 3600;
/// Cle de cooldown stockee dans `coude_cooldowns`.
pub const TRAVAUX_COOLDOWN_KEY: &str = "travaux_prison";
/// Probabilite de succes (50/50).
pub const TRAVAUX_SUCCESS_PCT: f64 = 0.5;
/// Borne basse du gain en cas de succes.
pub const TRAVAUX_COINS_MIN: i64 = 50;
/// Borne haute du gain en cas de succes.
pub const TRAVAUX_COINS_MAX: i64 = 100;
/// XP attribue a chaque tache (succes ou echec).
pub const TRAVAUX_XP_PER_TASK: i64 = 5;

/// Une tache de prison : cle stable + libelle UI + description narrative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TravauxTask {
    pub key: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

pub const TRAVAUX_TASKS: &[TravauxTask] = &[
    TravauxTask {
        key: "clean",
        label: "\u{1f9f9} Nettoyer les cellules",
        description: "Tu prends une vadrouille et tu nettoies les vomissures des dernieres bagarres. Pas glorieux, mais ca paie.",
    },
    TravauxTask {
        key: "cook",
        label: "\u{1f373} Cuisiner pour les gardes",
        description: "Tu rejoins la cuisine, prepares des œufs au plat trop cuits, sers les gardes en silence.",
    },
    TravauxTask {
        key: "inform",
        label: "\u{1f5e3}\u{fe0f} Informer la police",
        description: "Tu balances quelques rumeurs douteuses sur tes copegars. La police te paie en sourires gras.",
    },
];

pub const TRAVAUX_SUCCESS_FLAVORS: &[&str] = &[
    "Les gardes te tapent sur l epaule. \"T es un peu moins nul que prevu.\"",
    "Personne ne t a vu glander. Bravo.",
    "Le systeme penitentiaire te remercie pour ta contribution citoyenne.",
    "Tu as evite de te faire poignarder. C est deja une victoire.",
];

pub const TRAVAUX_FAIL_FLAVORS: &[&str] = &[
    "Tu glisses sur une serpilliere. Les gardes rient. Tu n es pas paye.",
    "Tu rates ta tache. Personne n est etonne.",
    "Un detenu t a vu et te chambre depuis. Pas de coins aujourd hui.",
    "Tu t es endormi a moitie. Reveille-toi quand tu veux travailler vraiment.",
];

/// Selectionne une tache deterministically a partir d'un index.
/// Utilise par le service avec `rng.gen_range(0..TRAVAUX_TASKS.len())`.
pub fn task_at(index: usize) -> TravauxTask {
    TRAVAUX_TASKS[index % TRAVAUX_TASKS.len()]
}

/// Selectionne un flavor `success` deterministically par index.
pub fn success_flavor_at(index: usize) -> &'static str {
    TRAVAUX_SUCCESS_FLAVORS[index % TRAVAUX_SUCCESS_FLAVORS.len()]
}

/// Selectionne un flavor `fail` deterministically par index.
pub fn fail_flavor_at(index: usize) -> &'static str {
    TRAVAUX_FAIL_FLAVORS[index % TRAVAUX_FAIL_FLAVORS.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_tasks_defined() {
        assert_eq!(TRAVAUX_TASKS.len(), 3);
    }

    #[test]
    fn task_at_wraps_modulo() {
        assert_eq!(task_at(0), TRAVAUX_TASKS[0]);
        assert_eq!(task_at(3), TRAVAUX_TASKS[0]);
        assert_eq!(task_at(7), TRAVAUX_TASKS[1]);
    }

    #[test]
    fn flavors_non_empty() {
        assert!(!TRAVAUX_SUCCESS_FLAVORS.is_empty());
        assert!(!TRAVAUX_FAIL_FLAVORS.is_empty());
        assert!(!success_flavor_at(0).is_empty());
        assert!(!fail_flavor_at(0).is_empty());
    }

    #[test]
    fn task_keys_are_unique() {
        let mut keys: Vec<_> = TRAVAUX_TASKS.iter().map(|t| t.key).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), TRAVAUX_TASKS.len());
    }

    #[test]
    fn coins_range_valid() {
        assert!(TRAVAUX_COINS_MIN > 0);
        assert!(TRAVAUX_COINS_MAX >= TRAVAUX_COINS_MIN);
    }
}
