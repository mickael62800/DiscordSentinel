//! Use case pour la resolution instantanee d'un combat (attaque surprise,
//! bloodbath event, defense via item). Port inbound.
//!
//! Phase 7 refacto : appele par coude-bot via gRPC. Le bot ne fait plus
//! que poster le resultat sur Discord — toute la logique metier (combat
//! engine, wallet, stats, XP, primes, paris, assurance) vit ici.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::errors::DomainError;

/// Champ d'un embed Discord construit cote API, pret a etre affiche par le bot.
/// Le bot se contente de faire `embed.field(name, value, inline)`.
#[derive(Debug, Clone)]
pub struct ResolvedCombatEmbedField {
    pub name: String,
    pub value: String,
    pub inline: bool,
}

/// Sortie complete : tout ce que le bot doit savoir pour construire l'embed
/// de resultat. Aucune logique cote bot.
#[derive(Debug, Clone)]
pub struct ResolveCombatNowOutput {
    pub combat_id: String,
    pub title: String,
    pub description: String,
    /// Couleur hex de l'embed (0x57F287 = vert, 0x9B59B6 = violet chaos/draw).
    pub color: u32,
    pub fields: Vec<ResolvedCombatEmbedField>,
    /// Phase 9 Part D : events de raillerie si un joueur a franchi un
    /// palier de streak sur ce combat. Le bot les poste tels quels.
    pub taunt_events: Vec<TauntEvent>,
    /// Cf. COUPE_AMELIORATIONS 5.3 — humiliation suite a une vendetta
    /// perdue. `Some` si le perdant avait declare une vendetta contre
    /// le gagnant (qui vient de l ecraser). Le bot doit alors renommer
    /// le gagnant "@gagnant le Bourreau de @perdant" pendant 7 jours.
    pub vendetta_humiliation: Option<VendettaHumiliation>,
}

/// Donnees pour appliquer le rename "Bourreau" cote bot apres une
/// vendetta perdue (cf. COUPE_AMELIORATIONS 5.3).
#[derive(Debug, Clone)]
pub struct VendettaHumiliation {
    /// Le joueur a renommer (= le gagnant du combat).
    pub target_user_id: String,
    /// Le challenger qui vient de perdre sa vendetta. Le bot resout son
    /// pseudo Discord pour construire le suffixe " le Bourreau de @X".
    pub challenger_user_id: String,
}

#[async_trait]
pub trait ResolveCombatNowUseCase: Send + Sync {
    /// Resout instantanement un combat deja cree (pending) : combat engine,
    /// wallet, stats, XP, primes, paris, assurance. Utilise pour les attaques
    /// surprise, bloodbath, et defense via item (tout ce qui court-circuite
    /// la phase de paris).
    async fn resolve_now(&self, combat_id: Uuid) -> Result<ResolveCombatNowOutput, DomainError>;
}
