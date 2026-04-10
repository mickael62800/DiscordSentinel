use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{CombatResolution, CoudeCombat, NewCoudeCombat};
use crate::domain::errors::DomainError;

/// Repository d'accès aux combats Coup de Coude.
#[async_trait]
pub trait CoudeCombatRepository: Send + Sync {
    // ── Lecture ──

    /// Liste les combats d'un guild, optionnellement filtrés par status.
    /// Le `username` retourné dans `attacker_name`/`defender_name` est rafraîchi
    /// depuis `coude_players` (avec fallback sur l'ID si le joueur n'existe plus).
    async fn list(
        &self,
        guild_id: &str,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<CoudeCombat>, DomainError>;

    async fn get(&self, id: Uuid) -> Result<Option<CoudeCombat>, DomainError>;

    async fn get_pending_for_attacker(
        &self,
        guild_id: &str,
        attacker_id: &str,
    ) -> Result<Option<CoudeCombat>, DomainError>;

    async fn get_pending_for_defender(
        &self,
        guild_id: &str,
        defender_id: &str,
    ) -> Result<Option<CoudeCombat>, DomainError>;

    async fn list_expired_pending(&self) -> Result<Vec<CoudeCombat>, DomainError>;

    /// Récupère le combat actuellement en phase de paris auquel `user_id`
    /// participe (en tant qu'attaquant OU défenseur). Utilisé par le flow
    /// "place_bet" pour récupérer le combat de référence d'un participant.
    async fn get_betting_for_participant(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoudeCombat>, DomainError>;

    // ── Écriture ──

    async fn create(&self, new: NewCoudeCombat) -> Result<CoudeCombat, DomainError>;

    /// Tente de résoudre un combat. Retourne `false` si le combat n'est pas dans
    /// un état actif (`pending`/`accepted`/`betting`) — c'est-à-dire si une autre
    /// race a déjà clôturé.
    async fn resolve(
        &self,
        id: Uuid,
        resolution: CombatResolution,
    ) -> Result<bool, DomainError>;

    /// Passe un combat en phase de paris et stocke le `message_id` Discord.
    /// Retourne `false` si le combat n'est plus en `pending`.
    async fn set_betting(&self, id: Uuid, message_id: &str) -> Result<bool, DomainError>;

    /// Marque un combat comme expiré (peu importe son état actuel).
    /// Utilisé par les workers et la commande `cancel`.
    async fn expire(&self, id: Uuid) -> Result<bool, DomainError>;

    /// Variante stricte de `expire` : ne marque expiré que si encore `pending`.
    /// Retourne `false` sinon. Utilisé pour `cancel_combat`.
    async fn cancel_pending(&self, id: Uuid) -> Result<bool, DomainError>;

    async fn set_defender_special(
        &self,
        id: Uuid,
        item_key: &str,
    ) -> Result<bool, DomainError>;

    /// Marque tous les paris non encore résolus d'un combat comme perdus.
    /// Utilisé après l'annulation/expiration d'un combat avant la résolution
    /// des paris.
    ///
    /// Note : cette méthode touche `coude_bets` mais reste ici pour éviter une
    /// dépendance circulaire avec le futur `CoudeBetRepository`. Sera déplacée
    /// quand le slice "Bets" sera extrait.
    async fn mark_unresolved_bets_lost(&self, combat_id: Uuid) -> Result<(), DomainError>;
}
