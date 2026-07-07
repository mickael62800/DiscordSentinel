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
//! Best-effort documente : l'icone du serveur et les emojis sont telecharges
//! depuis les URLs CDN du snapshot puis recrees (echec logge sans interrompre).
//! Les membres ABSENTS ne peuvent pas recevoir leurs roles.

use std::collections::HashMap;

use serenity::all::{
    AfkTimeout, ChannelId, ChannelType, Colour, Context, CreateAttachment, CreateChannel,
    DefaultMessageNotificationLevel, EditGuild, EditRole, ExplicitContentFilter, GuildId,
    PermissionOverwrite, PermissionOverwriteType, Permissions, RoleId, UserId, VerificationLevel,
};
use tracing::{info, warn};

use sentinel_core::domain::entities::guild_backup::snapshot::{GuildSnapshot, SnapshotChannel};

use super::api_client::PendingRoleGrant;

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
    pub emojis_created: usize,
    pub emojis_total: usize,
    /// `Some(true)` = icone restauree, `Some(false)` = echec, `None` = pas d'icone.
    pub icon_restored: Option<bool>,
    pub notes: Vec<String>,
    /// Re-attributions a persister cote API : pour TOUS les membres captures
    /// (presents ET absents), la liste des NOUVEAUX role_id (remappes). Les
    /// membres absents recuperent ainsi leurs roles a leur retour (hook join).
    pub pending_grants: Vec<PendingRoleGrant>,
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

    pub async fn set(&self, text: &str) {
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

    // ── 6. Emojis (best-effort : telecharge l'image CDN puis recree l'emoji) ──
    if !snapshot.emojis.is_empty() {
        report.emojis_total = snapshot.emojis.len();
        let total = snapshot.emojis.len();
        // `full` : une fois la limite du serveur atteinte (erreur Discord), on
        // arrete d'essayer pour ne pas spammer l'API inutilement.
        let mut full = false;
        for (i, emoji) in snapshot.emojis.iter().enumerate() {
            if i % 3 == 0 {
                progress
                    .set(&format!("♻️ Restauration… emojis {}/{}", i, total))
                    .await;
            }
            if full {
                break;
            }
            let Some(bytes) = download_bytes(ctx, &emoji.image_ref).await else {
                warn!(emoji = %emoji.name, url = %emoji.image_ref, "guild_backup: echec download emoji");
                continue;
            };
            // Discord attend une image en data URI base64.
            let data_uri = CreateAttachment::bytes(bytes, "emoji").to_base64();
            match guild_id.create_emoji(&ctx.http, &emoji.name, &data_uri).await {
                Ok(_) => report.emojis_created += 1,
                Err(e) => {
                    let msg = e.to_string();
                    warn!(error = %e, emoji = %emoji.name, "guild_backup: echec creation emoji");
                    // Limite d'emojis atteinte : inutile de continuer.
                    if msg.contains("Maximum number of emojis") || msg.contains("30008") {
                        full = true;
                    }
                }
            }
        }
        if full {
            report
                .notes
                .push("limite d'emojis du serveur atteinte".to_string());
        }
        info!(
            guild = %guild_id,
            created = report.emojis_created,
            total = report.emojis_total,
            "guild_backup: emojis restaures"
        );
    }

    // ── 7. member_roles (TOUS les membres) ──
    //
    // Pour chaque membre capture on traduit ses old_role_id -> nouveaux RoleId.
    // On enregistre TOUJOURS la re-attribution dans `pending_grants` (persistee
    // cote API par l'appelant) afin que les membres ABSENTS recuperent leurs
    // roles a leur retour. Les membres PRESENTS sont en plus re-rolises tout de
    // suite (l'entree pending sera consommee/purgee a leur prochain join, sans
    // effet visible).
    if !snapshot.member_roles.is_empty() {
        progress.set("♻️ Restauration… roles des membres").await;
        let mut absents = 0usize;
        for (user_id, old_roles) in &snapshot.member_roles {
            let Ok(uid) = user_id.parse::<u64>() else {
                continue;
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
            // Persistance de la re-attribution (nouveaux role_id en chaines).
            report.pending_grants.push(PendingRoleGrant {
                user_id: user_id.clone(),
                role_ids: new_roles.iter().map(|r| r.get().to_string()).collect(),
            });
            // Application immediate si le membre est present.
            match guild_id.member(&ctx.http, UserId::new(uid)).await {
                Ok(member) => match member.add_roles(&ctx.http, &new_roles).await {
                    Ok(()) => report.members_updated += 1,
                    Err(e) => {
                        warn!(error = %e, user = %user_id, "guild_backup: echec attribution roles membre")
                    }
                },
                Err(_) => absents += 1,
            }
        }
        if absents > 0 {
            report.notes.push(format!(
                "{absents} membre(s) absent(s) : roles re-attribues a leur retour"
            ));
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

/// Applique les reglages generaux (best-effort), icone du serveur incluse.
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

    // Icone : si presente, telecharge les bytes et applique via EditGuild::icon.
    // On l'ajoute au meme builder pour n'emettre qu'une requete edit.
    let mut icon_attachment: Option<CreateAttachment> = None;
    if let Some(icon_url) = &s.icon {
        match download_bytes(ctx, icon_url).await {
            Some(bytes) => icon_attachment = Some(CreateAttachment::bytes(bytes, "icon.png")),
            None => {
                warn!(guild = %guild_id, url = %icon_url, "guild_backup: echec download icone");
                report.icon_restored = Some(false);
                report.notes.push("icone non restauree (download)".to_string());
            }
        }
    }
    if let Some(att) = &icon_attachment {
        builder = builder.icon(Some(att));
    }

    if let Err(e) = guild_id.edit(&ctx.http, builder).await {
        warn!(error = %e, "guild_backup: echec application des reglages");
        report
            .notes
            .push("reglages du serveur partiellement appliques".to_string());
        if icon_attachment.is_some() {
            report.icon_restored = Some(false);
        }
    } else if icon_attachment.is_some() {
        report.icon_restored = Some(true);
        info!(guild = %guild_id, "guild_backup: icone restauree");
    }
}

/// Telecharge des bytes depuis une URL (CDN Discord) via le client reqwest
/// partage du bot (pooling + timeouts coherents). Best-effort : `None` en cas
/// d'echec reseau, statut non-2xx ou absence de client.
async fn download_bytes(ctx: &Context, url: &str) -> Option<Vec<u8>> {
    let client = {
        let data = ctx.data.read().await;
        let base = data.get::<crate::shared::heartbeat::ApiClientKey>()?;
        base.client().clone()
    };
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        warn!(status = %resp.status(), url, "guild_backup: download non-success");
        return None;
    }
    resp.bytes().await.ok().map(|b| b.to_vec())
}
