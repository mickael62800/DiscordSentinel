//! API client steal_roll (Phase 2 #4 audit).
//!
//! Le bot delegue le tirage RNG (d20 thief/victim + % wallet) au serveur
//! pour rendre la decision auditable. La presentation (templates,
//! embeds, calcul des bonus class/DEF/boost, application du verdict via
//! record_steal) reste cote bot.

use serde::{Deserialize, Serialize};

use super::ApiClient;

#[derive(Debug, Serialize)]
pub struct RollStealBody {
    pub afk: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RollStealResp {
    pub thief_d20: i32,
    pub victim_d20: i32,
    /// Basis points (1bp = 0.01%). Diviser par 10_000 pour obtenir le
    /// ratio applicable au solde de la victime.
    pub steal_pct_bp: u32,
}

/// Corps POST /steal/resolve.
#[derive(Debug, Serialize)]
pub struct ResolveStealBody {
    pub thief_id: String,
    pub target_id: String,
    pub afk: bool,
}

/// TauntEvent tel que serialise par l'API (miroir de `TauntEventDto`).
#[derive(Debug, Deserialize, Clone)]
struct TauntEventHttp {
    channel_id: String,
    target_user_id: String,
    message: String,
    nickname_suffix: String,
    streak_kind: String,
    streak_value: i32,
}

/// Reponse brute de POST /steal/resolve.
#[derive(Debug, Deserialize, Clone)]
struct ResolveStealResp {
    #[allow(dead_code)]
    outcome: String,
    title: String,
    description: String,
    color: u32,
    #[allow(dead_code)]
    stolen: i64,
    #[allow(dead_code)]
    lost: i64,
    #[allow(dead_code)]
    thief_roll: i32,
    #[allow(dead_code)]
    victim_roll: i32,
    taunt_events: Vec<TauntEventHttp>,
}

/// Embed de resolution du vol, cuit cote API. Le bot ne fait que le rendre
/// (title/description/color) + dispatcher les railleries.
#[derive(Debug, Clone)]
pub struct ResolvedSteal {
    pub title: String,
    pub description: String,
    pub color: u32,
    pub taunt_events: Vec<super::TauntEvent>,
}

impl ApiClient {
    pub async fn roll_steal(&self, guild_id: &str, afk: bool) -> Result<RollStealResp, String> {
        let body = RollStealBody { afk };
        self.base
            .post_json(&format!("/api/coude/{guild_id}/steal/roll"), &body)
            .await
    }

    /// Resolution serveur-side complete du vol : l'API decide l'issue,
    /// calcule butin/penalite (clamp serveur), mute les wallets et renvoie
    /// l'embed pret a poster + les railleries.
    pub async fn resolve_steal(
        &self,
        guild_id: &str,
        thief_id: &str,
        target_id: &str,
        afk: bool,
    ) -> Result<ResolvedSteal, String> {
        let body = ResolveStealBody {
            thief_id: thief_id.to_string(),
            target_id: target_id.to_string(),
            afk,
        };
        let resp: ResolveStealResp = self
            .base
            .post_json(&format!("/api/coude/{guild_id}/steal/resolve"), &body)
            .await?;
        Ok(ResolvedSteal {
            title: resp.title,
            description: resp.description,
            color: resp.color,
            taunt_events: resp
                .taunt_events
                .into_iter()
                .map(|e| super::TauntEvent {
                    channel_id: e.channel_id,
                    target_user_id: e.target_user_id,
                    message: e.message,
                    nickname_suffix: e.nickname_suffix,
                    streak_kind: e.streak_kind,
                    streak_value: e.streak_value,
                })
                .collect(),
        })
    }
}
