//! Phase de WIPE (optionnelle) précédant une restauration.
//!
//! Vide le serveur pour repartir d'un état vierge : supprime TOUS les salons +
//! catégories, les rôles supprimables, et tous les emojis. Best-effort, purement
//! séquentiel (respect des rate limits) : chaque suppression qui échoue est
//! loggée puis ignorée — jamais bloquante pour la restauration qui suit.
//!
//! ⚠️ DESTRUCTIF ET IRRÉVERSIBLE. N'efface PAS les bannissements (la restauration
//! les ré-applique ; les wiper n'est pas le but).
//!
//! Règles de skip des rôles (sinon 403 garanti côté Discord) :
//! - `@everyone` (id == guild_id) — non supprimable.
//! - rôles `managed` (gérés par une intégration / bot) — non supprimables.
//! - propre rôle du bot.
//! - rôles dont la `position` est >= au plus haut rôle du BOT (hiérarchie) —
//!   le bot ne peut pas agir sur un rôle au-dessus ou égal au sien.

use serenity::all::{Context, GuildId};
use tracing::{info, warn};

use super::restore::Progress;

/// Compteurs de la phase de wipe (inclus au rapport final de restauration).
#[derive(Debug, Default, Clone, Copy)]
pub struct WipeReport {
    pub channels_deleted: usize,
    pub roles_deleted: usize,
    pub emojis_deleted: usize,
}

/// Vide le serveur (salons -> rôles -> emojis) avant recréation. Best-effort.
///
/// Ordre volontaire : salons d'abord (leurs overwrites référençant des rôles
/// disparaissent avec eux), puis rôles, puis emojis.
pub async fn wipe(ctx: &Context, guild_id: GuildId, progress: &Progress<'_>) -> WipeReport {
    let mut report = WipeReport::default();

    // ── 1. Salons + catégories ──
    //
    // Le feedback passe par les FOLLOWUPS d'interaction (token-based) : il
    // survit à la suppression du salon courant, contrairement à un envoi direct
    // dans un salon. Un serveur peut rester transitoirement à 0 salon.
    match guild_id.channels(&ctx.http).await {
        Ok(channels) => {
            let total = channels.len();
            for (i, (_, channel)) in channels.into_iter().enumerate() {
                if i % 5 == 0 {
                    progress
                        .set(&format!("🧨 Suppression… salons {}/{}", i, total))
                        .await;
                }
                if let Err(e) = channel.delete(&ctx.http).await {
                    warn!(error = %e, channel = %channel.name, "guild_backup(wipe): échec suppression salon");
                } else {
                    report.channels_deleted += 1;
                }
            }
        }
        Err(e) => warn!(error = %e, "guild_backup(wipe): lecture des salons impossible"),
    }

    // ── 2. Rôles ──
    let bot_id = ctx.cache.current_user().id;
    let bot_role_id = serenity::all::RoleId::new(bot_id.get());
    match guild_id.roles(&ctx.http).await {
        Ok(roles) => {
            // Plus haute position d'un rôle porté par le bot : borne de hiérarchie.
            // Si introuvable (bot non trouvé / sans rôle), on ne supprime aucun
            // rôle par prudence (0 == position de @everyone, tout sera skippé).
            let bot_top = match guild_id.member(&ctx.http, bot_id).await {
                Ok(member) => member
                    .roles
                    .iter()
                    .filter_map(|rid| roles.get(rid))
                    .map(|r| r.position)
                    .max()
                    .unwrap_or(0),
                Err(e) => {
                    warn!(error = %e, "guild_backup(wipe): membre bot introuvable, rôles préservés");
                    0
                }
            };

            let everyone = guild_id.everyone_role();
            let total = roles.len();
            let mut seen = 0usize;
            for (rid, role) in roles.iter() {
                seen += 1;
                if seen.is_multiple_of(5) {
                    progress
                        .set(&format!("🧨 Suppression… rôles {}/{}", seen, total))
                        .await;
                }
                // Skips obligatoires : @everyone, propre rôle du bot.
                if *rid == everyone || *rid == bot_role_id {
                    continue;
                }
                if role.managed {
                    info!(role = %role.name, "guild_backup(wipe): rôle managed préservé");
                    continue;
                }
                if role.position >= bot_top {
                    info!(
                        role = %role.name,
                        position = role.position,
                        bot_top,
                        "guild_backup(wipe): rôle >= hiérarchie du bot préservé"
                    );
                    continue;
                }
                if let Err(e) = guild_id.delete_role(&ctx.http, *rid).await {
                    warn!(error = %e, role = %role.name, "guild_backup(wipe): échec suppression rôle");
                } else {
                    report.roles_deleted += 1;
                }
            }
        }
        Err(e) => warn!(error = %e, "guild_backup(wipe): lecture des rôles impossible"),
    }

    // ── 3. Emojis ──
    match guild_id.emojis(&ctx.http).await {
        Ok(emojis) => {
            let total = emojis.len();
            for (i, emoji) in emojis.into_iter().enumerate() {
                if i % 3 == 0 {
                    progress
                        .set(&format!("🧨 Suppression… emojis {}/{}", i, total))
                        .await;
                }
                if let Err(e) = guild_id.delete_emoji(&ctx.http, emoji.id).await {
                    warn!(error = %e, emoji = %emoji.name, "guild_backup(wipe): échec suppression emoji");
                } else {
                    report.emojis_deleted += 1;
                }
            }
        }
        Err(e) => warn!(error = %e, "guild_backup(wipe): lecture des emojis impossible"),
    }

    info!(
        guild = %guild_id,
        channels = report.channels_deleted,
        roles = report.roles_deleted,
        emojis = report.emojis_deleted,
        "guild_backup(wipe): serveur vidé"
    );

    report
}
