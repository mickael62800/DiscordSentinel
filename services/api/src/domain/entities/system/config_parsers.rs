//! Helpers purs pour parser les valeurs de `bot_guild_config`.
//!
//! Les configs sont stockees en `TEXT` (cle/valeur stringifiees). Ces
//! parsers gardent un defaut si la cle est absente ou si la valeur ne
//! parse pas — defensif. Utilise par les application services taunts
//! (seuils jackpot/donor, flag bankruptcy) et potentiellement d'autres.

use std::collections::HashMap;

/// Parse un flag booleen depuis un map de config. Accepte (insensible a
/// la casse) : `"true"`, `"1"`, `"yes"`. Tout le reste = false.
pub fn parse_bool_config(map: &HashMap<String, String>, key: &str, default: bool) -> bool {
    map.get(key)
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
        .unwrap_or(default)
}

/// Parse un entier i64 depuis un map de config. Si la cle est absente
/// ou si la valeur ne parse pas, retourne `default`.
pub fn parse_i64_config(map: &HashMap<String, String>, key: &str, default: i64) -> i64 {
    map.get(key).and_then(|v| v.parse::<i64>().ok()).unwrap_or(default)
}

/// Convention de nommage : les services de type "worker" (jobs batch
/// planifies) ont `worker` dans leur nom. Les autres sont des bots
/// Discord. Utilise par le dashboard pour afficher les compteurs
/// bots_online / workers_online.
pub fn is_worker_service(name: &str) -> bool {
    name.contains("worker")
}

#[cfg(test)]
#[path = "tests/config_parsers.rs"]
mod tests;
