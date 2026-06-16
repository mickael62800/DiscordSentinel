//! Carte de review automod persistee (cf. migration 176).
//!
//! Le bot poste un embed avec boutons (Apply / Warn / Mute / Ban / Ignore)
//! dans le salon de logs ; en parallele il INSERT une `AutomodReview` dans
//! cette table et register l'`action_id` dans `discord_action_messages`.
//! Du coup la web peut lister les reviews pending et resoudre depuis l UI ;
//! le bot edite la carte Discord en reaction (sync bilateral).

use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;
use crate::domain::entities::system::discord_ids::MessageId;
use crate::domain::entities::system::discord_ids::ChannelId;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::entities::system::discord_ids::GuildId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuggestedAction {
    Warn,
    Delete,
    Mute,
    Ban,
}

impl SuggestedAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Warn => "warn",
            Self::Delete => "delete",
            Self::Mute => "mute",
            Self::Ban => "ban",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "warn" => Some(Self::Warn),
            "delete" => Some(Self::Delete),
            "mute" => Some(Self::Mute),
            "ban" => Some(Self::Ban),
            _ => None,
        }
    }
    /// Rang de severite (warn < delete < mute < ban).
    pub fn severity(&self) -> u8 {
        match self {
            Self::Warn => 1,
            Self::Delete => 2,
            Self::Mute => 3,
            Self::Ban => 4,
        }
    }
}

/// Retourne la plus severe des deux actions suggerees (strings). Sert a
/// l'agregation : l'action d'une carte regroupee escalade vers le pire vu.
/// En cas de valeur inconnue, on retombe sur l'autre (ou "warn").
pub fn more_severe_suggested(a: &str, b: &str) -> String {
    let rank = |s: &str| SuggestedAction::from_str(s).map(|x| x.severity()).unwrap_or(0);
    if rank(a) >= rank(b) {
        if rank(a) == 0 { "warn".to_string() } else { a.to_string() }
    } else {
        b.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppliedAction {
    Warn,
    Delete,
    Mute,
    Ban,
    Ignore,
}

impl AppliedAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Warn => "warn",
            Self::Delete => "delete",
            Self::Mute => "mute",
            Self::Ban => "ban",
            Self::Ignore => "ignore",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "warn" => Some(Self::Warn),
            "delete" => Some(Self::Delete),
            "mute" => Some(Self::Mute),
            "ban" => Some(Self::Ban),
            "ignore" => Some(Self::Ignore),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AutomodReview {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub user_id: UserId,
    pub user_name: String,
    pub content_preview: String,
    pub suggested_action: String,
    pub score: f64,
    pub reason: String,
    pub flags: serde_json::Value,
    pub status: String,
    pub applied_action: Option<String>,
    pub resolved_by_id: Option<String>,
    pub resolved_by_name: Option<String>,
    pub resolved_source: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    // ── Systeme de vote (cf. migration 251) ──
    /// Echeance du vote (statut 'voting'). None si review hors mode vote.
    pub voting_deadline: Option<DateTime<Utc>>,
    /// Sanction retenue apres depouillement (statut 'decided'+).
    pub decided_action: Option<String>,
    /// Le quorum minimum de votes a-t-il ete atteint ?
    pub quorum_met: bool,
    /// Horodatage du depouillement.
    pub decided_at: Option<DateTime<Utc>>,
    // ── Agregation par utilisateur (cf. migration 264) ──
    /// Nombre d'incidents agreges dans cette carte (1 si pas de regroupement).
    pub incident_count: i32,
    /// Somme des scores des incidents agreges (le champ `score` reste le max).
    pub cumulative_score: f64,
    /// Liste JSON des incidents agreges
    /// (`[{message_id, channel_id, content_preview, score, reason, suggested_action, at}]`).
    pub incidents: serde_json::Value,
}

// ── Vote des moderateurs ──────────────────────────────────────────────

/// Sanction qu'un moderateur peut voter (identique a AppliedAction, mais
/// nommee distinctement pour exprimer l'intention "vote").
pub type VoteAction = AppliedAction;

impl AppliedAction {
    /// Rang de severite : sert au tie-break (plus clemente / plus severe).
    /// ignore (0) < warn (1) < delete (2) < mute (3) < ban (4).
    pub fn severity(&self) -> u8 {
        match self {
            Self::Ignore => 0,
            Self::Warn => 1,
            Self::Delete => 2,
            Self::Mute => 3,
            Self::Ban => 4,
        }
    }
}

/// Un vote individuel persiste (table automod_review_votes).
#[derive(Debug, Clone)]
pub struct ReviewVote {
    pub id: Uuid,
    pub review_id: Uuid,
    pub voter_id: String,
    pub voter_name: String,
    pub vote_action: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Strategie de departage en cas d'egalite de voix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TieAction {
    /// Aucune sanction.
    Ignore,
    /// La sanction la plus clemente parmi les ex-aequo.
    Clemente,
    /// La sanction la plus severe parmi les ex-aequo.
    Severe,
}

impl TieAction {
    pub fn from_str(s: &str) -> Self {
        match s {
            "clemente" => Self::Clemente,
            "severe" => Self::Severe,
            _ => Self::Ignore,
        }
    }
}

/// Resultat d'un depouillement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TallyResult {
    /// Sanction retenue. `Ignore` = aucune sanction (refus, quorum non
    /// atteint, ou egalite resolue en ignore).
    pub decided: AppliedAction,
    /// Le quorum a-t-il ete atteint ?
    pub quorum_met: bool,
    /// Nombre total de votes exprimes.
    pub total_votes: usize,
}

/// Depouille les votes : majorite des voix, quorum minimum, tie-break.
///
/// - Si `total < quorum` -> Ignore, quorum_met=false (alerte ignoree).
/// - Sinon la sanction avec le plus de voix gagne.
/// - En cas d'egalite entre plusieurs sanctions, applique `tie`.
pub fn tally_votes(votes: &[VoteAction], quorum: usize, tie: TieAction) -> TallyResult {
    let total = votes.len();
    if total == 0 || total < quorum.max(1) {
        return TallyResult { decided: AppliedAction::Ignore, quorum_met: false, total_votes: total };
    }

    // Comptage par action.
    let mut counts: std::collections::HashMap<u8, (AppliedAction, usize)> = std::collections::HashMap::new();
    for v in votes {
        let entry = counts.entry(v.severity()).or_insert((v.clone(), 0));
        entry.1 += 1;
    }

    let max_count = counts.values().map(|(_, c)| *c).max().unwrap_or(0);
    let mut leaders: Vec<AppliedAction> =
        counts.values().filter(|(_, c)| *c == max_count).map(|(a, _)| a.clone()).collect();

    let decided = if leaders.len() == 1 {
        leaders.remove(0)
    } else {
        // Egalite : departage.
        match tie {
            TieAction::Ignore => AppliedAction::Ignore,
            TieAction::Clemente => leaders.into_iter().min_by_key(|a| a.severity()).unwrap(),
            TieAction::Severe => leaders.into_iter().max_by_key(|a| a.severity()).unwrap(),
        }
    };

    TallyResult { decided, quorum_met: true, total_votes: total }
}

// ── Salon de discussion lie a une review ─────────────────────────────

/// Salon textuel ouvert pour discuter d'une review (membre + modos).
/// Persiste pour l'audit et l'idempotence (un seul salon par review).
#[derive(Debug, Clone)]
pub struct DiscussionChannel {
    pub id: Uuid,
    pub review_id: Uuid,
    pub guild_id: String,
    pub channel_id: String,
    pub opened_by_id: String,
    pub opened_by_name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewDiscussionChannel {
    pub review_id: Uuid,
    pub guild_id: String,
    pub channel_id: String,
    pub opened_by_id: String,
    pub opened_by_name: String,
}

/// Faits Discord du demandeur, fournis par l'adapter bot. La DECISION
/// d'autorisation (les regles ci-dessous) est prise par le domaine, pas par
/// le bot — utilise pour le vote, la finalisation et l'ouverture de discussion.
#[derive(Debug, Clone, Default)]
pub struct ModeratorFacts {
    pub is_admin: bool,
    pub has_moderate_members: bool,
    pub has_manage_messages: bool,
    /// Porte le role moderateur configure (`vote_mod_role_id`).
    pub has_mod_role: bool,
    /// Porte le role admin configure (`vote_admin_role_id`).
    pub has_admin_role: bool,
}

/// Regle metier : qui est "moderateur" (peut voter, ouvrir une discussion).
/// Admin, "Moderer les membres", "Gerer les messages", ou role modo configure.
pub fn is_moderator(f: &ModeratorFacts) -> bool {
    f.is_admin || f.has_moderate_members || f.has_manage_messages || f.has_mod_role
}

/// Regle metier : qui peut FINALISER un vote (appliquer la sanction).
/// Reserve aux administrateurs (permission ADMINISTRATOR ou role admin configure).
pub fn can_finalize_review(f: &ModeratorFacts) -> bool {
    f.is_admin || f.has_admin_role
}

/// Regle metier : qui peut ouvrir un salon de discussion (= moderateur).
pub fn can_open_discussion(f: &ModeratorFacts) -> bool {
    is_moderator(f)
}

#[derive(Debug, Clone)]
pub struct NewAutomodReview {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub user_id: UserId,
    pub user_name: String,
    pub content_preview: String,
    pub suggested_action: SuggestedAction,
    pub score: f64,
    pub reason: String,
    pub flags: serde_json::Value,
    /// Si Some, la review naît en mode VOTE (statut 'voting') avec cette
    /// echeance. Si None, comportement historique (statut 'pending').
    pub voting_deadline: Option<DateTime<Utc>>,
}
