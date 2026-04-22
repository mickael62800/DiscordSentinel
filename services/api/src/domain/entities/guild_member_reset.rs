//! Regles metier pour la gestion des membres Discord cote API :
//! - limites de l'API Discord (max 1000 membres par appel list_members)
//! - TTL du cache Redis `guild:members:*`
//! - liste des tables a purger lors d'un reset_member (invariant metier :
//!   quelles donnees de moderation d'un membre sont effacees).

/// Limite de l'API Discord GET /guilds/{guild_id}/members : max 1000 membres
/// par appel. Source : https://discord.com/developers/docs/resources/guild
pub const DISCORD_LIST_MEMBERS_CAP: u32 = 1000;

/// TTL du cache Redis pour `guild:members:{guild_id}` (10 minutes).
pub const MEMBERS_CACHE_TTL_SECS: u64 = 600;

/// Table impactee par un reset de membre : nom SQL + colonne de clef
/// user + cle de sortie dans le JSON de reponse.
#[derive(Debug, Clone, Copy)]
pub struct MemberResetTable {
    pub sql_table: &'static str,
    pub user_column: &'static str,
    pub response_key: &'static str,
}

/// Tables purgees par `POST /api/members/{guild_id}/{user_id}/reset`, dans
/// l'ordre d'execution. Regle metier : quelles donnees de moderation on
/// efface quand on "reset" un membre (operation irreversible).
pub const MEMBER_RESET_TABLES: &[MemberResetTable] = &[
    MemberResetTable {
        sql_table: "infractions",
        user_column: "user_id",
        response_key: "infractions",
    },
    MemberResetTable {
        sql_table: "moderation_actions",
        user_column: "target_id",
        response_key: "moderation_actions",
    },
    MemberResetTable {
        sql_table: "user_conduct_points",
        user_column: "user_id",
        response_key: "conduct_points",
    },
    MemberResetTable {
        sql_table: "conduct_points_log",
        user_column: "user_id",
        response_key: "conduct_log",
    },
    MemberResetTable {
        sql_table: "user_strikes",
        user_column: "user_id",
        response_key: "strikes",
    },
    MemberResetTable {
        sql_table: "user_notes",
        user_column: "user_id",
        response_key: "notes",
    },
    MemberResetTable {
        sql_table: "manual_watched_users",
        user_column: "user_id",
        response_key: "manual_watched",
    },
    MemberResetTable {
        sql_table: "sanction_reminders",
        user_column: "target_id",
        response_key: "sanction_reminders",
    },
];

#[cfg(test)]
#[path = "tests/guild_member_reset.rs"]
mod tests;
