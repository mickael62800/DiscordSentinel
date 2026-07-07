use serde::Deserialize;

/// Seuils de detection d'anomalie, resolus per-guild depuis bot_guild_config
/// et transmis a l'API qui decide (cf. `anomaly_thresholds_for`).
#[derive(Debug, Clone, Copy)]
pub struct AnomalyThresholds {
    pub mass_ban: usize,
    pub mass_delete: usize,
    pub mass_role_change: usize,
}

impl Default for AnomalyThresholds {
    fn default() -> Self {
        Self {
            mass_ban: 5,
            mass_delete: 20,
            mass_role_change: 10,
        }
    }
}

/// Alerte d'anomalie decidee par l'API et renvoyee au bot pour affichage.
///
/// La DECISION (comptage fenetre + seuil + reset) est desormais server-side
/// (`DetectModerationAnomaly` cote sentinel-api). Le bot ne fait qu'afficher
/// l'embed URGENT a partir de cette alerte.
#[derive(Debug, Clone, Deserialize)]
pub struct AnomalyAlert {
    pub anomaly_type: String,
    pub count: usize,
    pub window_secs: u64,
}
