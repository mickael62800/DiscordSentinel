//! Use case `resolve_steal` : resolution serveur-side complete d'une
//! tentative de vol (`/voler`).
//!
//! Avant : le bot recevait des des bruts (`roll_steal`) puis decidait
//! l'issue (gagne/perdu), calculait le butin et la penalite client-side
//! (`voler.rs::resolve_steal_attempt`). Risque de divergence + le bot
//! etait autorite sur l'issue.
//!
//! Apres (miroir de `ResolveCombatNow`) : toute la logique metier (tirage
//! des des, bonus classe/DEF/boost, malus AFK, decision de l'issue, calcul
//! du butin/penalite avec clamp serveur, mutations wallet atomiques,
//! protections anti-vol, XP, streaks) vit ici. Le bot ne fait que rendre
//! l'embed retourne et dispatcher les railleries.

use async_trait::async_trait;

use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::errors::DomainError;

/// Entree : identifiants + statut AFK de la cible (n'a pas clique sur
/// "Se defendre" → malus defenseur + plage de butin plus faible).
#[derive(Debug, Clone)]
pub struct ResolveStealCommand {
    pub guild_id: String,
    pub thief_id: String,
    pub target_id: String,
    pub afk: bool,
}

/// Issue decidee cote serveur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StealResolutionOutcome {
    /// Le voleur a gagne le combat et le vol a abouti.
    Success,
    /// Le voleur a perdu le combat (penalite appliquee).
    Failed,
    /// Le voleur avait gagne le combat mais une protection a bloque.
    Blocked,
}

/// Sortie complete : tout ce que le bot doit savoir pour construire
/// l'embed de resultat. Aucune logique cote bot.
#[derive(Debug, Clone)]
pub struct ResolveStealOutput {
    pub outcome: StealResolutionOutcome,
    pub title: String,
    pub description: String,
    /// Couleur hex de l'embed.
    pub color: u32,
    /// Montant reellement vole (0 si echec/blocage).
    pub stolen: i64,
    /// Montant reellement perdu par le voleur (0 si succes/blocage).
    pub lost: i64,
    /// d20 tire pour le voleur (affichage/audit).
    pub thief_roll: i32,
    /// d20 tire pour la victime (affichage/audit).
    pub victim_roll: i32,
    /// Railleries a poster apres l'embed (faillite/jackpot/streaks).
    pub taunt_events: Vec<TauntEvent>,
}

#[async_trait]
pub trait ResolveStealUseCase: Send + Sync {
    /// Resout entierement une tentative de vol : tire les des serveur,
    /// applique les bonus/malus (regles de combat cote CORE), decide
    /// l'issue, calcule butin/penalite avec clamp serveur, debite/credite
    /// les wallets atomiquement, enregistre stats/XP/streaks. Renvoie
    /// l'embed pret a poster + les railleries.
    async fn resolve_steal(
        &self,
        cmd: ResolveStealCommand,
    ) -> Result<ResolveStealOutput, DomainError>;
}
