//! Restauration d'un `GuildSnapshot` dans un serveur Discord, avec REMAPPING
//! d'IDs.
//!
//! Recree la structure dans l'ordre roles -> categories -> salons (+ overwrites)
//! -> settings -> bans -> emojis -> member_roles. Chaque etape construit une
//! table `old_id -> new_id` consommee par les etapes suivantes.
//!
//! Robustesse : sequentiel, gestion d'erreur PAR ELEMENT (une creation qui
//! echoue est loggee et n'interrompt pas la restauration). Ne parallelise pas
//! (serenity gere les rate limits sur des appels sequentiels).
//!
//! Best-effort documente : l'icone du serveur et les emojis ne sont PAS
//! restaures (images) — seulement logges. Les membres ABSENTS ne peuvent pas
//! recevoir leurs roles.

use std::collections::HashMap;

use serenity::all::{
    AfkTimeout, ChannelId, ChannelType, Colour, Context, CreateChannel,
    DefaultMessageNotificationLevel, EditGuild, EditRole, ExplicitContentFilter, GuildId,
    PermissionOverwrite, PermissionOverwriteType, Permissions, RoleId, UserId, VerificationLevel,
};
use tracing::{info, warn};

use sentinel_core::domain::entities::guild_backup::snapshot::{GuildSnapshot, SnapshotChannel};

/// Rapport de restauration (compteurs pour le feedback final).
#[derive(Debug, Default)]
pub struct RestoreReport {
    pub roles_created: usize,
    pub roles_failed: usize,
    pub categories_created: usize,
    pub channels_created: usize,
    pub channels_failed: usize,
    pub bans_applied: usize,
    pub members_updated: usize,
    pub notes: Vec<String>,
}

/// Rapporteur de progression : edite le message deferre de l'interaction pour
/// afficher l'avancement ("Restauration… X/Y salons").
pub struct Progress<'a> {
    ctx: &'a Context,
    component: &'a serenity::all::ComponentInteraction,
}

impl<'a> Progress<'a> {
    pub fn new(ctx: &'a Context, component: &'a serenity::all::ComponentInteraction) -> Self {
        Self { ctx, component }
    }

    async fn set(&self, text: &str) {
        let _ = self
            .component
            .edit_response(
                &self.ctx.http,
                serenity::all::EditInteractionResponse::new().content(text),
            )
            .await;
    }
}

/// Parse un bitfield de permissions (chaine) vers [`Permissions`].
fn parse_permissions(bits: &str) -> Permissions {
    let raw = bits.parse::<u64>().unwrap_or(0);
    Permissions::from_bits_truncate(raw)
}

/// Traduit le `kind` textuel du snapshot vers un [`ChannelType`] serenity.
fn channel_type(kind: &str) -> ChannelType {
    match kind {
        "voice" => ChannelType::Voice,
        "forum" => ChannelType::Forum,
        "announcement" => ChannelType::News,
        "stage" => ChannelType::Stage,
        _ => ChannelType::Text,
    }
}

/// Restaure le snapshot dans `guild_id`. Renvoie un rapport de synthese.
pub async fn restore(
    ctx: &Context,
    guild_id: GuildId,
    snapshot: &GuildSnapshot,
    progress: &Progress<'_>,
) -> RestoreReport {
    let mut report = RestoreReport::default();

    // Tables de remapping old_id -> new_id.
    let mut role_map: HashMap<String, RoleId> = HashMap::new();
    let mut channel_map: HashMap<String, ChannelId> = HashMap::new();

    // @everyone : ne pas recreer, mapper l'ancien @everyone (== ancien guild_id)
    // vers le @everyone du serveur cible.
    role_map.insert(snapshot.guild_id.clone(), guild_id.everyone_role());

    // ── 1. Roles ──
    progress.set("♻️ Restauration… roles").await;
    for role in &snapshot.roles {
        let builder = EditRole::new()
            .name(&role.name)
            .colour(Colour::new(role.color))
            .hoist(role.hoist)
            .mentionable(role.mentionable)
            .permissions(parse_permissions(&role.permissions));
        match guild_id.create_role(&ctx.http, builder).await {
            Ok(new_role) => {
                role_map.insert(role.old_id.clone(), new_role.id);
                report.roles_created += 1;
            }
            Err(e) => {
                warn!(error = %e, role = %role.name, "guild_backup: echec creation role");
                report.roles_failed += 1;
            }
        }
    }

    // ── 2. Categories ──
    progress.set("♻️ Restauration… categories").await;
    for cat in &snapshot.categories {
        let builder = CreateChannel::new(&cat.name).kind(ChannelType::Category);
        match guild_id.create_channel(&ctx.http, builder).await {
            Ok(ch) => {
                channel_map.insert(cat.old_id.clone(), ch.id);
                report.categories_created += 1;
            }
            Err(e) => {
                warn!(error = %e, category = %cat.name, "guild_backup: echec creation categorie");
            }
        }
    }

    // ── 3. Salons (+ overwrites) ──
    let total = snapshot.channels.len();
    for (i, chan) in snapshot.channels.iter().enumerate() {
        if i % 5 == 0 {
            progress
                .set(&format!("♻️ Restauration… salons {}/{}", i, total))
                .await;
        }
        match create_channel(ctx, guild_id, chan, &channel_map, &role_map).await {
            Some(id) => {
                channel_map.insert(chan.old_id.clone(), id);
                report.channels_created += 1;
            }
            None => report.channels_failed += 1,
        }
    }

    // ── 4. Settings ──
    progress.set("♻️ Restauration… reglages").await;
    apply_settings(ctx, guild_id, snapshot, &channel_map, &mut report).await;

    // ── 5. Bans ──
    if !snapshot.bans.is_empty() {
        progress.set("♻️ Restauration… bannissements").await;
        for ban in &snapshot.bans {
            let Ok(uid) = ban.user_id.parse::<u64>() else {
                continue;
            };
            let reason = ban.reason.clone().unwrap_or_default();
            let res = if reason.is_empty() {
                guild_id.ban(&ctx.http, UserId::new(uid), 0).await
            } else {
                guild_id
                    .ban_with_reason(&ctx.http, UserId::new(uid), 0, &reason)
                    .await
            };
            match res {
                Ok(()) => report.bans_applied += 1,
                Err(e) => warn!(error = %e, user = %ban.user_id, "guild_backup: echec ban"),
            }
        }
    }

    // ── 6. Emojis (best-effort : NON restaures, images) ──
    if !snapshot.emojis.is_empty() {
        let note = format!(
            "{} emoji(s) non restaures (images non recreees dans cette version)",
            snapshot.emojis.len()
        );
        info!(guild = %guild_id, "guild_backup: {note}");
        report.notes.push(note);
    }

    // ── 7. member_roles (membres PRESENTS uniquement) ──
    if !snapshot.member_roles.is_empty() {
        progress.set("♻️ Restauration… roles des membres").await;
        let mut absents = 0usize;
        for (user_id, old_roles) in &snapshot.member_roles {
            let Ok(uid) = user_id.parse::<u64>() else {
                continue;
            };
            let member = match guild_id.member(&ctx.http, UserId::new(uid)).await {
                Ok(m) => m,
                Err(_) => {
                    absents += 1;
                    continue;
                }
            };
            let new_roles: Vec<RoleId> = old_roles
                .iter()
                .filter_map(|old| role_map.get(old).copied())
                // Ne pas re-ajouter @everyone (implicite).
                .filter(|r| *r != guild_id.everyone_role())
                .collect();
            if new_roles.is_empty() {
                continue;
            }
            match member.add_roles(&ctx.http, &new_roles).await {
                Ok(()) => report.members_updated += 1,
                Err(e) => {
                    warn!(error = %e, user = %user_id, "guild_backup: echec attribution roles membre")
                }
            }
        }
        if absents > 0 {
            report
                .notes
                .push(format!("{absents} membre(s) absent(s) non re-rolises"));
        }
    }

    info!(
        guild = %guild_id,
        roles = report.roles_created,
        categories = report.categories_created,
        channels = report.channels_created,
        bans = report.bans_applied,
        members = report.members_updated,
        "guild_backup: restauration terminee"
    );

    report
}

/// Cree un salon avec son type, son parent, ses attributs et ses overwrites
/// (traduits via les tables de remapping). Renvoie l'ID cree ou `None`.
async fn create_channel(
    ctx: &Context,
    guild_id: GuildId,
    chan: &SnapshotChannel,
    channel_map: &HashMap<String, ChannelId>,
    role_map: &HashMap<String, RoleId>,
) -> Option<ChannelId> {
    let kind = channel_type(&chan.kind);
    let mut builder = CreateChannel::new(&chan.name).kind(kind).nsfw(chan.nsfw);

    if let Some(parent_old) = &chan.parent_old_id {
        if let Some(new_parent) = channel_map.get(parent_old) {
            builder = builder.category(*new_parent);
        }
    }
    if let Some(topic) = &chan.topic {
        if !topic.is_empty() {
            builder = builder.topic(topic);
        }
    }
    // Slowmode : salons textuels / forum uniquement.
    if matches!(kind, ChannelType::Text | ChannelType::Forum) && chan.slowmode > 0 {
        builder = builder.rate_limit_per_user(chan.slowmode.min(u16::MAX as u32) as u16);
    }
    // Bitrate / user_limit : salons vocaux / stage uniquement.
    if matches!(kind, ChannelType::Voice | ChannelType::Stage) {
        if let Some(bitrate) = chan.bitrate {
            builder = builder.bitrate(bitrate);
        }
        if let Some(limit) = chan.user_limit {
            builder = builder.user_limit(limit);
        }
    }

    // Overwrites : traduit la cible via les tables de remapping.
    let mut overwrites: Vec<PermissionOverwrite> = Vec::new();
    for ow in &chan.overwrites {
        let kind = match ow.target_type.as_str() {
            "role" => match role_map.get(&ow.target_old_id) {
                Some(rid) => PermissionOverwriteType::Role(*rid),
                None => continue, // role non remappe (managed / disparu)
            },
            "member" => match ow.target_old_id.parse::<u64>() {
                Ok(uid) => PermissionOverwriteType::Member(UserId::new(uid)),
                Err(_) => continue,
            },
            _ => continue,
        };
        overwrites.push(PermissionOverwrite {
            allow: parse_permissions(&ow.allow),
            deny: parse_permissions(&ow.deny),
            kind,
        });
    }
    if !overwrites.is_empty() {
        builder = builder.permissions(overwrites);
    }

    match guild_id.create_channel(&ctx.http, builder).await {
        Ok(ch) => Some(ch.id),
        Err(e) => {
            warn!(error = %e, channel = %chan.name, "guild_backup: echec creation salon");
            None
        }
    }
}

/// Applique les reglages generaux (best-effort). L'icone n'est PAS restauree.
async fn apply_settings(
    ctx: &Context,
    guild_id: GuildId,
    snapshot: &GuildSnapshot,
    channel_map: &HashMap<String, ChannelId>,
    report: &mut RestoreReport,
) {
    let s = &snapshot.settings;
    let mut builder = EditGuild::new()
        .name(&s.name)
        .verification_level(VerificationLevel::from(s.verification_level as u8))
        .default_message_notifications(Some(DefaultMessageNotificationLevel::from(
            s.default_notifications as u8,
        )))
        .explicit_content_filter(Some(ExplicitContentFilter::from(
            s.explicit_content_filter as u8,
        )))
        .afk_timeout(AfkTimeout::from(s.afk_timeout as u16));

    if let Some(old) = &s.afk_channel_old_id {
        builder = builder.afk_channel(channel_map.get(old).copied());
    }
    if let Some(old) = &s.system_channel_old_id {
        builder = builder.system_channel_id(channel_map.get(old).copied());
    }

    if let Err(e) = guild_id.edit(&ctx.http, builder).await {
        warn!(error = %e, "guild_backup: echec application des reglages");
        report
            .notes
            .push("reglages du serveur partiellement appliques".to_string());
    }

    if s.icon.is_some() {
        info!(guild = %guild_id, "guild_backup: icone non restauree (image non recreee)");
        report.notes.push("icone non restauree".to_string());
    }
}
