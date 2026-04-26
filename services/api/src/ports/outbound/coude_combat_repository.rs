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

    /// Phase 2 refacto : reclame atomiquement les combats en phase `betting`
    /// dont le delai de paris est ecoule, en les passant a `resolving`.
    /// Utilise `FOR UPDATE SKIP LOCKED` pour eviter le double traitement si
    /// plusieurs instances du batch runner tournent concurremment.
    ///
    /// Le delai par guild est lu depuis `bot_guild_config` (`bet_delay_secs`)
    /// avec fallback sur `default_delay_secs`.
    async fn claim_due_betting_combats(
        &self,
        default_delay_secs: i64,
    ) -> Result<Vec<CoudeCombat>, DomainError>;

    /// Phase 2 refacto : reclame les combats bloques en `resolving` depuis
    /// plus de `stuck_threshold_secs` (typiquement 120s). Ces combats ont
    /// probablement crashe un tick precedent. On les touche (accepted_at =
    /// NOW()) pour prevenir le double-traitement si le tick reussit apres.
    async fn claim_stuck_resolving_combats(
        &self,
        stuck_threshold_secs: i64,
    ) -> Result<Vec<CoudeCombat>, DomainError>;

    /// Phase 4 : claim atomique des combats pending dont le delai d'expiration
    /// est ecoule. Passe status a 'expired' en une seule requete avec
    /// FOR UPDATE SKIP LOCKED. Le delai est lu depuis bot_guild_config
    /// (coude-worker / combat_expiry_hours) avec fallback sur default.
    async fn claim_expired_pending_combats(
        &self,
        default_expiry_hours: i64,
    ) -> Result<Vec<CoudeCombat>, DomainError>;

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

    /// Purge destructive : vide toutes les tables Coup de Coude listees dans
    /// `COUDE_PURGE_TABLES` (domain) pour une guild, dans une transaction
    /// unique. Retourne les comptes par table. Admin-only.
    ///
    /// Default `unimplemented!()` pour preserver les mocks existants.
    async fn purge_guild_subsystem(
        &self,
        _guild_id: &str,
    ) -> Result<Vec<(String, u64)>, DomainError> {
        unimplemented!("purge_guild_subsystem not implemented")
    }

    /// Compte le nombre de combats perdus aujourd hui par `user_id` (resolved_at
    /// CURRENT_DATE, statut resolved, loser_id = user). Utilise par le bouclier
    /// malchance (lucky_shield) pour detecter la 1ere defaite du jour.
    ///
    /// Default `unimplemented!()` pour preserver les mocks existants.
    async fn count_defeats_today(
        &self,
        _guild_id: &str,
        _user_id: &str,
    ) -> Result<i64, DomainError> {
        unimplemented!("count_defeats_today not implemented")
    }
}
