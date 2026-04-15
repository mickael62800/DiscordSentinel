//! Use case dedie a la resolution batch des combats Coup de Coude en phase
//! de paris. Port inbound.
//!
//! Ce use case est appele periodiquement par le coude-worker (via gRPC) et
//! fait tout le travail de resolution cote API :
//!   1. Claim atomique des combats dont le delai de paris est ecoule.
//!   2. Pour chacun : chargement joueurs, events actifs, appel du
//!      `coude_combat_engine` (pur domain), application des effets (wallet,
//!      stats, HP, assurance, paris, vol chaos).
//!   3. Retourne les metadonnees necessaires au worker pour poster le
//!      resultat sur Discord (le worker ne fait plus que du IO Discord).
//!
//! Avant Phase 2, cette logique vivait dans
//! `services/workers/coude-worker/src/jobs/resolve_betting.rs` avec 55
//! requetes SQL directes et le moteur de combat duplique. La refacto la
//! centralise ici pour respecter l'architecture hexagonale : le worker
//! devient thin, l'API possede toute la logique metier + acces DB.

use async_trait::async_trait;

use crate::domain::entities::TauntEvent;
use crate::domain::errors::DomainError;

/// Sortie retournee pour chaque combat resolu.
/// Le worker l'utilise pour poster le resultat sur Discord (edit message ou
/// post nouveau message dans le channel combat).
#[derive(Debug, Clone)]
pub struct ResolvedBettingCombatOutput {
    pub combat_id: String,
    pub guild_id: String,
    pub channel_id: Option<String>,
    pub message_id: Option<String>,
    pub result_message: String,
    pub winner_id: Option<String>,
    pub loser_id: Option<String>,
    pub coins_transferred: i64,
    pub is_draw: bool,
    /// Phase 9 Part D : si un joueur a franchi un palier de streak suite
    /// a ce combat, l'API remplit ce vecteur (0 a 2 events, typiquement
    /// un pour le gagnant ET un pour le perdant si les deux atteignent
    /// un seuil sur le meme combat).
    pub taunt_events: Vec<TauntEvent>,
}

#[async_trait]
pub trait ResolveBettingBatchUseCase: Send + Sync {
    /// Resout en une passe tous les combats en phase `betting` dont le delai
    /// est ecoule + les combats stucks en `resolving` (retry safe).
    /// Retourne la liste des combats resolus avec les metadonnees Discord.
    async fn resolve_batch(&self) -> Result<Vec<ResolvedBettingCombatOutput>, DomainError>;
}
