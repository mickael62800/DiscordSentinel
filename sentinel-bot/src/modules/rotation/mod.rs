//! Administrateur tournant : chaque periode, un moderateur (role configure)
//! devient administrateur a tour de role, apres acceptation (MP) + validation
//! de l'owner (MP). Le precedent admin redevient moderateur.
//!
//! Orchestration cote bot (Discord), etat persiste via l'API (/api/rotation).
//! Les boutons sont cliques en MP : on encode le guild_id dans le custom_id
//! (component.guild_id est None en MP).

pub const MODULE_BOT_NAME: &str = "rotation-bot";

use std::sync::Arc;

use serenity::all::{
    ButtonStyle, ComponentInteraction, Context, CreateActionRow, CreateButton,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, GuildId, RoleId,
    UserId,
};
use tracing::{info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;
use crate::shared::discord_helpers::is_module_enabled;

const PREFIX: &str = "rot:";

// ── Etat (DTO API) ──

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
struct RotationDto {
    guild_id: String,
    state: String,
    current_admin_id: Option<String>,
    current_admin_since: Option<String>,
    period_start: Option<String>,
    next_rotation_at: Option<String>,
    candidate_id: Option<String>,
    candidate_offered_at: Option<String>,
    #[serde(default)]
    asked_this_round: Vec<String>,
}

#[derive(serde::Deserialize)]
struct ServedEntryDto {
    user_id: String,
    served_at: String,
}

struct Cfg {
    mod_role: u64,
    admin_role: u64,
    period_days: i64,
    timeout_hours: i64,
    objective: String,
}

// ── Interface module ──

pub fn handles_component(cid: &str) -> bool {
    cid.starts_with(PREFIX)
}

pub fn spawn_background_tasks(ctx: &Context) {
    let ctx = ctx.clone();
    tokio::spawn(async move {
        loop {
            // Verifie toutes les 10 min : debut de periode / timeout.
            tokio::time::sleep(std::time::Duration::from_secs(600)).await;
            let guilds = ctx.cache.guilds();
            for gid in guilds {
                tick(&ctx, gid).await;
            }
        }
    });
}

// ── Helpers ──

async fn api(ctx: &Context) -> Option<Arc<BaseApiClient>> {
    ctx.data.read().await.get::<ApiClientKey>().cloned()
}

async fn load_cfg(api: &BaseApiClient, guild_id: &str) -> Option<Cfg> {
    let c = api.get_guild_config_for(guild_id, MODULE_BOT_NAME).await.ok()?;
    if !BaseApiClient::config_bool(&c, "enabled", false) {
        return None;
    }
    let mod_role = c.get("mod_role_id").and_then(|v| v.parse::<u64>().ok()).filter(|x| *x > 0)?;
    let admin_role = c.get("admin_role_id").and_then(|v| v.parse::<u64>().ok()).filter(|x| *x > 0)?;
    Some(Cfg {
        mod_role,
        admin_role,
        period_days: BaseApiClient::config_u64(&c, "period_days", 30) as i64,
        timeout_hours: BaseApiClient::config_u64(&c, "response_timeout_hours", 72) as i64,
        objective: c
            .get("objective_message")
            .cloned()
            .unwrap_or_else(|| "Ce mois-ci, c'est ton tour de devenir Administrateur ! Acceptes-tu ?".into()),
    })
}

async fn get_state(api: &BaseApiClient, guild_id: &str) -> Option<RotationDto> {
    api.get_json::<RotationDto>(&format!("/api/rotation/{guild_id}")).await.ok()
}

async fn save_state(api: &BaseApiClient, st: &RotationDto) {
    let _ = api
        .post_json::<_, serde_json::Value>(&format!("/api/rotation/{}/save", st.guild_id), st)
        .await;
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn parse_dt(s: &Option<String>) -> Option<chrono::DateTime<chrono::Utc>> {
    s.as_deref().and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok()).map(|d| d.with_timezone(&chrono::Utc))
}

fn elapsed_hours(since: &Option<String>) -> i64 {
    match parse_dt(since) {
        Some(d) => (chrono::Utc::now() - d).num_hours(),
        None => i64::MAX,
    }
}

/// Choisit le prochain candidat : membre avec le role modo, non bot, pas deja
/// sollicite ce tour, en round-robin (jamais servi d'abord, puis plus ancien).
async fn pick_candidate(ctx: &Context, guild_id: GuildId, cfg: &Cfg, api: &BaseApiClient, asked: &[String]) -> Option<UserId> {
    let mod_role = RoleId::new(cfg.mod_role);
    // Membres ayant le role modo (depuis le cache).
    let mut eligible: Vec<u64> = {
        let g = ctx.cache.guild(guild_id)?;
        g.members
            .values()
            .filter(|m| !m.user.bot && m.roles.contains(&mod_role))
            .map(|m| m.user.id.get())
            .filter(|id| !asked.contains(&id.to_string()))
            .collect()
    };
    if eligible.is_empty() {
        return None;
    }
    // Historique (date de dernier mandat par user).
    let served: Vec<ServedEntryDto> = api
        .get_json(&format!("/api/rotation/{guild_id}/history"))
        .await
        .unwrap_or_default();
    let rank = |uid: u64| -> (u8, String) {
        match served.iter().find(|e| e.user_id == uid.to_string()) {
            Some(e) => (1, e.served_at.clone()), // deja servi : trie par date asc (plus ancien d'abord)
            None => (0, String::new()),          // jamais servi : prioritaire
        }
    };
    eligible.sort_by(|a, b| rank(*a).cmp(&rank(*b)));
    eligible.first().map(|id| UserId::new(*id))
}

async fn dm(ctx: &Context, user_id: UserId, content: &str, components: Vec<CreateActionRow>) {
    if let Ok(ch) = user_id.create_dm_channel(&ctx.http).await {
        let mut msg = CreateMessage::new().content(content);
        if !components.is_empty() {
            msg = msg.components(components);
        }
        if let Err(e) = ch.send_message(&ctx.http, msg).await {
            warn!(error = %e, %user_id, "rotation: echec MP");
        }
    } else {
        warn!(%user_id, "rotation: impossible d'ouvrir le MP (MP fermes ?)");
    }
}

fn candidate_buttons(guild_id: u64, candidate: u64) -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{PREFIX}acc:{guild_id}:{candidate}")).label("Accepter").style(ButtonStyle::Success),
        CreateButton::new(format!("{PREFIX}dec:{guild_id}:{candidate}")).label("Refuser").style(ButtonStyle::Danger),
    ])]
}

fn owner_buttons(guild_id: u64, candidate: u64) -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{PREFIX}val:{guild_id}:{candidate}")).label("Valider").style(ButtonStyle::Success),
        CreateButton::new(format!("{PREFIX}ref:{guild_id}:{candidate}")).label("Refuser").style(ButtonStyle::Danger),
    ])]
}

fn stay_buttons(guild_id: u64, admin: u64) -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{PREFIX}stay:{guild_id}:{admin}")).label("Rester admin").style(ButtonStyle::Success),
        CreateButton::new(format!("{PREFIX}leave:{guild_id}:{admin}")).label("Arreter").style(ButtonStyle::Secondary),
    ])]
}

fn owner_id(ctx: &Context, guild_id: GuildId) -> Option<UserId> {
    ctx.cache.guild(guild_id).map(|g| g.owner_id)
}

/// Applique le changement de roles (best-effort).
async fn swap_roles(ctx: &Context, guild_id: GuildId, user: UserId, add: Option<u64>, remove: Option<u64>) {
    if let Ok(member) = guild_id.member(&ctx.http, user).await {
        if let Some(r) = remove {
            let _ = member.remove_role(&ctx.http, RoleId::new(r)).await;
        }
        if let Some(a) = add {
            let _ = member.add_role(&ctx.http, RoleId::new(a)).await;
        }
    }
}

// ── Tick periodique ──

async fn tick(ctx: &Context, guild_id: GuildId) {
    let gid = guild_id.to_string();
    if !is_module_enabled(ctx, &gid, MODULE_BOT_NAME).await {
        return;
    }
    let api = match api(ctx).await { Some(a) => a, None => return };
    let cfg = match load_cfg(&api, &gid).await { Some(c) => c, None => return };
    let mut st = get_state(&api, &gid).await.unwrap_or_else(|| RotationDto { guild_id: gid.clone(), state: "idle".into(), ..Default::default() });

    match st.state.as_str() {
        "idle" => {
            let due = match parse_dt(&st.next_rotation_at) {
                None => true,
                Some(d) => chrono::Utc::now() >= d,
            };
            if due {
                start_rotation(ctx, guild_id, &cfg, &api, &mut st).await;
            }
        }
        "offering_candidate" | "awaiting_owner" => {
            if elapsed_hours(&st.candidate_offered_at) >= cfg.timeout_hours {
                advance_or_finish(ctx, guild_id, &cfg, &api, &mut st).await;
            }
        }
        "offering_stay" => {
            if elapsed_hours(&st.candidate_offered_at) >= cfg.timeout_hours {
                // Pas de reponse de l'admin actuel : on le garde, fin de cycle.
                st.state = "idle".into();
                save_state(&api, &st).await;
            }
        }
        _ => {}
    }
}

async fn start_rotation(ctx: &Context, guild_id: GuildId, cfg: &Cfg, api: &BaseApiClient, st: &mut RotationDto) {
    st.period_start = Some(now_rfc3339());
    st.next_rotation_at = Some((chrono::Utc::now() + chrono::Duration::days(cfg.period_days.max(1))).to_rfc3339());
    st.asked_this_round = Vec::new();

    match pick_candidate(ctx, guild_id, cfg, api, &st.asked_this_round).await {
        Some(cand) => {
            st.state = "offering_candidate".into();
            st.candidate_id = Some(cand.get().to_string());
            st.candidate_offered_at = Some(now_rfc3339());
            st.asked_this_round.push(cand.get().to_string());
            save_state(api, st).await;
            dm(ctx, cand, &format!("👑 **Administrateur tournant**\n\n{}", cfg.objective), candidate_buttons(guild_id.get(), cand.get())).await;
            info!(guild = %guild_id, candidate = %cand, "rotation: candidat sollicite");
        }
        None => {
            // Aucun modo eligible : rien a faire ce cycle.
            st.state = "idle".into();
            save_state(api, st).await;
        }
    }
}

/// Apres un refus/timeout : propose au suivant, sinon a l'admin actuel de
/// rester, sinon termine sans admin.
async fn advance_or_finish(ctx: &Context, guild_id: GuildId, cfg: &Cfg, api: &BaseApiClient, st: &mut RotationDto) {
    match pick_candidate(ctx, guild_id, cfg, api, &st.asked_this_round).await {
        Some(cand) => {
            st.state = "offering_candidate".into();
            st.candidate_id = Some(cand.get().to_string());
            st.candidate_offered_at = Some(now_rfc3339());
            st.asked_this_round.push(cand.get().to_string());
            save_state(api, st).await;
            dm(ctx, cand, &format!("👑 **Administrateur tournant**\n\n{}", cfg.objective), candidate_buttons(guild_id.get(), cand.get())).await;
        }
        None => {
            // Tout le monde a refuse.
            if let Some(admin) = st.current_admin_id.clone().and_then(|s| s.parse::<u64>().ok()) {
                st.state = "offering_stay".into();
                st.candidate_id = None;
                st.candidate_offered_at = Some(now_rfc3339());
                save_state(api, st).await;
                dm(ctx, UserId::new(admin), "Personne n'a accepte le mandat ce mois-ci. Veux-tu **rester administrateur** ?", stay_buttons(guild_id.get(), admin)).await;
            } else {
                st.state = "idle".into();
                st.candidate_id = None;
                save_state(api, st).await;
                if let Some(owner) = owner_id(ctx, guild_id) {
                    dm(ctx, owner, "Aucun moderateur n'a accepte le mandat d'administrateur ce mois-ci. Il n'y aura pas d'administrateur tournant cette periode.", vec![]).await;
                }
            }
        }
    }
}

// ── Boutons (cliques en MP) ──

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let cid = component.data.custom_id.clone();
    let rest = match cid.strip_prefix(PREFIX) { Some(r) => r, None => return };
    let parts: Vec<&str> = rest.split(':').collect();
    if parts.len() != 3 {
        return;
    }
    let act = parts[0];
    let guild_id = match parts[1].parse::<u64>() { Ok(g) => GuildId::new(g), Err(_) => return };
    let subject = match parts[2].parse::<u64>() { Ok(u) => UserId::new(u), Err(_) => return };

    let api = match api(ctx).await { Some(a) => a, None => return };
    let gid = guild_id.to_string();
    let cfg = match load_cfg(&api, &gid).await { Some(c) => c, None => return };
    let mut st = match get_state(&api, &gid).await { Some(s) => s, None => return };

    let clicker = component.user.id.get();

    match act {
        // Candidat accepte -> MP a l'owner pour validation.
        "acc" => {
            if st.state != "offering_candidate" || st.candidate_id.as_deref() != Some(&subject.get().to_string()) || clicker != subject.get() {
                return ack(ctx, component, "Cette demande n'est plus active.").await;
            }
            st.state = "awaiting_owner".into();
            st.candidate_offered_at = Some(now_rfc3339());
            save_state(&api, &st).await;
            ack(ctx, component, "✅ Reponse transmise au fondateur pour validation.").await;
            if let Some(owner) = owner_id(ctx, guild_id) {
                dm(ctx, owner, &format!("<@{}> a accepte de devenir administrateur ce mois-ci. **Valides-tu ?**", subject.get()), owner_buttons(guild_id.get(), subject.get())).await;
            }
        }
        // Candidat refuse -> suivant.
        "dec" => {
            if st.state != "offering_candidate" || clicker != subject.get() {
                return ack(ctx, component, "Cette demande n'est plus active.").await;
            }
            ack(ctx, component, "Tres bien, ce sera pour une prochaine fois.").await;
            advance_or_finish(ctx, guild_id, &cfg, &api, &mut st).await;
        }
        // Owner valide -> applique les roles.
        "val" => {
            if Some(clicker) != owner_id(ctx, guild_id).map(|o| o.get()) {
                return ack(ctx, component, "Seul le fondateur peut valider.").await;
            }
            if st.state != "awaiting_owner" || st.candidate_id.as_deref() != Some(&subject.get().to_string()) {
                return ack(ctx, component, "Cette validation n'est plus active.").await;
            }
            // Retire le role admin a l'ancien (-> redevient modo).
            if let Some(prev) = st.current_admin_id.clone().and_then(|s| s.parse::<u64>().ok()) {
                swap_roles(ctx, guild_id, UserId::new(prev), Some(cfg.mod_role), Some(cfg.admin_role)).await;
            }
            // Promeut le candidat (modo -> admin).
            swap_roles(ctx, guild_id, subject, Some(cfg.admin_role), Some(cfg.mod_role)).await;
            let _ = api
                .post_json::<_, serde_json::Value>(&format!("/api/rotation/{gid}/served"), &serde_json::json!({"user_id": subject.get().to_string()}))
                .await;
            st.current_admin_id = Some(subject.get().to_string());
            st.current_admin_since = Some(now_rfc3339());
            st.state = "idle".into();
            st.candidate_id = None;
            save_state(&api, &st).await;
            ack(ctx, component, "✅ Valide ! Le role a ete attribue.").await;
            dm(ctx, subject, "🎉 Felicitations, tu es **administrateur** pour ce mandat !", vec![]).await;
        }
        // Owner refuse -> suivant.
        "ref" => {
            if Some(clicker) != owner_id(ctx, guild_id).map(|o| o.get()) {
                return ack(ctx, component, "Seul le fondateur peut refuser.").await;
            }
            if st.state != "awaiting_owner" {
                return ack(ctx, component, "Cette demande n'est plus active.").await;
            }
            ack(ctx, component, "Refuse. Je propose au moderateur suivant.").await;
            advance_or_finish(ctx, guild_id, &cfg, &api, &mut st).await;
        }
        // Admin actuel choisit de rester.
        "stay" => {
            if st.state != "offering_stay" || clicker != subject.get() {
                return ack(ctx, component, "Cette demande n'est plus active.").await;
            }
            st.state = "idle".into();
            save_state(&api, &st).await;
            ack(ctx, component, "👍 Tu restes administrateur pour cette periode.").await;
        }
        // Admin actuel arrete -> retire le role.
        "leave" => {
            if st.state != "offering_stay" || clicker != subject.get() {
                return ack(ctx, component, "Cette demande n'est plus active.").await;
            }
            swap_roles(ctx, guild_id, subject, Some(cfg.mod_role), Some(cfg.admin_role)).await;
            st.current_admin_id = None;
            st.state = "idle".into();
            save_state(&api, &st).await;
            ack(ctx, component, "Tres bien, il n'y aura pas d'administrateur cette periode.").await;
        }
        _ => {}
    }
}

async fn ack(ctx: &Context, component: &ComponentInteraction, text: &str) {
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new().content(text).components(vec![]),
            ),
        )
        .await;
}
