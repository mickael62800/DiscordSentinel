//! # `global_rbac_gate` — Gate RBAC GLOBAL fail-closed (feature-flaggue).
//!
//! ## Probleme resolu
//!
//! L'enforcement RBAC est aujourd'hui *par handler* : chaque handler mutant
//! doit penser a appeler `check_role` / `check_role_for_guild` /
//! `check_component_role`. ~70 endpoints mutants n'ont aucun check, donc un
//! utilisateur web *Moderator* peut appeler des routes qui devraient etre
//! *Admin*/*Owner*. Ce gate global ferme ce trou : meme si un handler oublie
//! son check, le gate refuse l'appel.
//!
//! ## Modele
//!
//! Le gate s'execute APRES `rbac_middleware` (RoleContext disponible) et APRES
//! `whitelist_middleware`. Pour CHAQUE requete *mutante* (POST/PUT/PATCH/
//! DELETE) provenant d'un *utilisateur web* :
//!
//!   1. Methode safe (GET/HEAD/OPTIONS) → pass.
//!   2. `AuthKind::Internal` (bot/workers, Bearer api_key) → pass (confiance).
//!   3. Dev mode (`state.api_key` vide) → pass (ne casse pas le local dev).
//!   4. Route dans l'allowlist PUBLIC/self-service → pass.
//!   5. Superadmin (`SUPERADMIN_USER_IDS`) → pass.
//!   6. Route mappee dans `ROUTE_ROLES` → exige le role : `role >= required`.
//!      Aucun role resolvable pour une route mappee → **DENY 403** (fail-closed).
//!   7. Route mutante *non mappee* → **DENY 403** (fail-closed) + log du
//!      pattern refuse, pour qu'on puisse l'ajouter a la table.
//!
//! ## Feature flag (SECURITE)
//!
//! Pilote par `RBAC_GLOBAL_GATE` (default **off**), tri-etat :
//!   - `off` (absent / autre)     → no-op total (zero changement).
//!   - `audit` / `dryrun`         → log-only : execute la decision, journalise
//!     ce qui SERAIT refuse, mais laisse TOUJOURS passer. Sert a reperer en
//!     prod les routes legitimes non mappees (403 potentiels) sans rien casser.
//!   - `true` / `1`               → enforce : refuse reellement (fail-closed).
//!
//! Sequence de deploiement recommandee : `audit` en prod → surveiller les logs
//! `global_rbac_gate` (mode=AUDIT) → completer `ROUTE_ROLES` → basculer enforce.
//!
//! ## ⚠️ A VALIDER EN STAGING avant activation en prod
//!
//! L'auth ne peut pas etre testee dans l'environnement de dev (pas de vrai
//! token Discord / Redis / DB RBAC). Activer le flag uniquement apres :
//!   1. `RBAC_GLOBAL_GATE=true` sur le staging ;
//!   2. login en tant que *Moderator* → verifier 403 sur les routes Admin/Owner
//!      (ex: `PUT /api/welcome/{guild_id}`, `POST /api/invitations`) ;
//!   3. login en tant que *Admin*/*Owner* → verifier 200 sur ces memes routes ;
//!   4. surveiller les logs `global_rbac_gate: route mutante NON MAPPEE` pour
//!      completer la table avant generalisation.
//!
//! Les checks par handler existants restent en place (defense en profondeur).

use std::collections::HashMap;
use std::sync::OnceLock;

use axum::body::Body;
use axum::extract::{MatchedPath, State};
use axum::http::{Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::adapters::inbound::http::middleware::auth::AuthKind;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::enums::system::role::Role;

/// Routes publiques / self-service : un utilisateur web (meme non whitelist)
/// doit pouvoir les appeler. Match exact sur le `MatchedPath`.
const PUBLIC_PATHS: &[&str] = &[
    // Self-service auth (le user s'auto-autorise via un code d'invitation).
    "/api/auth/redeem-invitation",
    "/api/auth/check-access",
    // OAuth web + session (routes publiques cote router, listees par securite).
    "/auth/discord/authorize",
    "/auth/discord/callback",
    "/auth/refresh",
    "/auth/logout",
    // Health / metrics.
    "/health",
    "/metrics",
];

/// Table (methode, pattern de route) → role minimum requis pour un user web.
///
/// Convention de roles (cf. prompt + patch RBAC cible) :
///   - **Owner** : tres destructif (delete guild, purges, reset, RBAC, docker,
///     invitations, ban IP, RCON, suppression serveur de jeu).
///   - **Admin** : config / CRUD destructif (welcome, levels config, panels,
///     annonces, salons themes, roles Discord, bots config, exports...).
///   - **Moderator** : contenu de moderation (tickets, confessions, watched
///     users, reviews automod, strikes, notes, reminders, actions de mod).
///
/// Les routes purement bot/worker (wallet credit/debit, snapshots,
/// bot_persistence, audit create_log, casino gameplay, stats record...)
/// ne sont **volontairement pas** listees : `AuthKind::Internal` les laisse
/// deja passer. Si un user web les atteint un jour, le fail-closed (point 7)
/// les refuse — ce qui est le comportement correct.
fn route_roles() -> &'static HashMap<(&'static str, &'static str), Role> {
    static TABLE: OnceLock<HashMap<(&'static str, &'static str), Role>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut m: HashMap<(&'static str, &'static str), Role> = HashMap::new();

        // ── Dashboard / guilds / config ──────────────────────────────────
        m.insert(("POST", "/api/guilds/register"), Role::Admin);
        m.insert(("POST", "/api/guilds/reconcile"), Role::Owner);
        m.insert(("DELETE", "/api/guilds/{guild_id}"), Role::Owner);
        m.insert(("DELETE", "/api/logs/{category}"), Role::Admin);
        m.insert(("DELETE", "/api/infractions/{id}"), Role::Moderator);
        m.insert(("PATCH", "/api/rules/{id}"), Role::Admin);
        m.insert(("POST", "/api/bots/config"), Role::Admin);
        m.insert(("DELETE", "/api/bots/config"), Role::Admin);
        m.insert(("DELETE", "/api/purge/infractions"), Role::Owner);
        m.insert(("DELETE", "/api/purge/audit-logs"), Role::Owner);
        m.insert(("DELETE", "/api/purge/logs"), Role::Owner);

        // ── Rules / infractions (bot.rs, surface web) ────────────────────
        m.insert(("POST", "/rules"), Role::Admin);
        m.insert(("DELETE", "/rules/{guild_id}/{rule_id}"), Role::Admin);
        m.insert(("DELETE", "/infractions/delete/{id}"), Role::Moderator);

        // ── Audit logs / watched users / discord roles ───────────────────
        m.insert(("DELETE", "/api/audit-logs/{guild_id}"), Role::Owner);
        m.insert(("POST", "/api/watched-users"), Role::Moderator);
        m.insert(
            ("DELETE", "/api/watched-users/{guild_id}/{user_id}"),
            Role::Moderator,
        );
        m.insert(
            ("POST", "/api/discord-roles/{guild_id}/create"),
            Role::Admin,
        );
        m.insert(
            ("PATCH", "/api/discord-roles/{guild_id}/{role_id}"),
            Role::Admin,
        );
        m.insert(
            ("DELETE", "/api/discord-roles/{guild_id}/{role_id}"),
            Role::Admin,
        );

        // ── Automod reviews (contenu de moderation) ──────────────────────
        m.insert(("POST", "/api/automod/reviews"), Role::Moderator);
        // Finalisation = application d'une sanction de membre : reserve aux
        // Admins, comme la finalisation Discord (regle can_finalize_review).
        // Le vote/ignore/discussion restent ouverts aux Moderateurs.
        m.insert(
            ("POST", "/api/automod/reviews/{review_id}/resolve"),
            Role::Admin,
        );
        m.insert(
            ("POST", "/api/automod/reviews/{review_id}/ignore"),
            Role::Moderator,
        );
        m.insert(
            ("POST", "/api/automod/reviews/{review_id}/reopen"),
            Role::Moderator,
        );
        m.insert(
            ("POST", "/api/automod/reviews/{review_id}/vote"),
            Role::Moderator,
        );
        m.insert(
            ("POST", "/api/automod/reviews/{review_id}/decide"),
            Role::Moderator,
        );
        m.insert(
            ("POST", "/api/automod/reviews/{review_id}/discussion"),
            Role::Moderator,
        );
        m.insert(
            ("DELETE", "/api/automod/reviews/{review_id}/discussion"),
            Role::Moderator,
        );
        m.insert(
            (
                "POST",
                "/api/automod/reviews/{review_id}/discussion/messages",
            ),
            Role::Moderator,
        );

        // ── Tickets (contenu de moderation) ──────────────────────────────
        m.insert(("POST", "/api/tickets/"), Role::Moderator);
        m.insert(("DELETE", "/api/tickets/bulk"), Role::Admin);
        m.insert(("POST", "/api/tickets/{id}/messages"), Role::Moderator);
        m.insert(("PATCH", "/api/tickets/{id}/close"), Role::Moderator);
        m.insert(("PATCH", "/api/tickets/{id}/assign"), Role::Moderator);
        m.insert(("PATCH", "/api/tickets/{id}/status"), Role::Moderator);
        m.insert(("PATCH", "/api/tickets/{id}/channels"), Role::Moderator);

        // ── Security (purges = Owner ; quarantine/slowmode = Moderator ;
        //    lockdown = Admin) ──────────────────────────────────────────
        m.insert(("DELETE", "/api/security/events/{guild_id}"), Role::Owner);
        m.insert(("POST", "/api/security/quarantine"), Role::Moderator);
        m.insert(
            ("DELETE", "/api/security/quarantine/{guild_id}/{user_id}"),
            Role::Moderator,
        );
        m.insert(("POST", "/api/security/lockdown"), Role::Admin);
        m.insert(("DELETE", "/api/security/lockdown/{guild_id}"), Role::Admin);
        m.insert(("POST", "/api/security/slowmode"), Role::Moderator);
        m.insert(
            ("DELETE", "/api/security/slowmode/{guild_id}"),
            Role::Moderator,
        );
        // Security monitoring host (system.rs) : superadmin-level → Owner.
        m.insert(("DELETE", "/api/security/cleanup"), Role::Owner);
        m.insert(("POST", "/api/security/ban-ip"), Role::Owner);
        m.insert(("POST", "/api/security/unban-ip"), Role::Owner);

        // ── Moderation actions ───────────────────────────────────────────
        m.insert(("DELETE", "/api/moderation/actions/{id}"), Role::Moderator);
        m.insert(("POST", "/api/moderation/execute-ban"), Role::Moderator);
        m.insert(("POST", "/api/moderation/execute-unban"), Role::Moderator);
        m.insert(("POST", "/api/moderation/execute-mute"), Role::Moderator);
        m.insert(("POST", "/api/moderation/evidence"), Role::Moderator);
        m.insert(("POST", "/api/moderation/review"), Role::Moderator);
        m.insert(
            ("PATCH", "/api/moderation/review/{id}/resolve"),
            Role::Moderator,
        );

        // ── Strikes / notes / reminders ──────────────────────────────────
        m.insert(("PUT", "/api/strikes/config/{guild_id}"), Role::Admin);
        m.insert(
            ("DELETE", "/api/strikes/{guild_id}/{user_id}"),
            Role::Moderator,
        );
        m.insert(("POST", "/api/strikes/"), Role::Moderator);
        m.insert(("POST", "/api/notes/"), Role::Moderator);
        m.insert(("DELETE", "/api/notes/{id}"), Role::Moderator);
        m.insert(("POST", "/api/reminders/"), Role::Moderator);

        // ── Levels / progression (admin XP overrides) ────────────────────
        m.insert(("POST", "/api/levels/admin/set-xp"), Role::Admin);
        m.insert(("POST", "/api/levels/admin/reset-xp"), Role::Admin);

        // ── Role panels / auto-roles ─────────────────────────────────────
        m.insert(("POST", "/api/role-panels/"), Role::Admin);
        m.insert(
            ("DELETE", "/api/role-panels/detail/{panel_id}"),
            Role::Admin,
        );
        m.insert(("PATCH", "/api/role-panels/set-message"), Role::Admin);
        m.insert(("POST", "/api/auto-roles/"), Role::Admin);
        m.insert(
            ("DELETE", "/api/auto-roles/{guild_id}/{role_id}"),
            Role::Admin,
        );

        // ── Announcements (config) ───────────────────────────────────────
        m.insert(("POST", "/api/announcements/"), Role::Admin);
        m.insert(("PATCH", "/api/announcements/by-id/{id}"), Role::Admin);
        m.insert(("DELETE", "/api/announcements/by-id/{id}"), Role::Admin);
        m.insert(("POST", "/api/announcements/{id}/toggle"), Role::Admin);

        // ── Confessions (contenu de moderation + config) ─────────────────
        m.insert(("PATCH", "/api/confessions/by-id/{id}"), Role::Moderator);
        m.insert(("DELETE", "/api/confessions/by-id/{id}"), Role::Moderator);
        m.insert(("DELETE", "/api/confessions/replies/{id}"), Role::Moderator);
        m.insert(
            ("POST", "/api/confessions/reports/{id}/resolve"),
            Role::Moderator,
        );
        m.insert(("POST", "/api/confessions/config"), Role::Admin);

        // ── Voice channels (purges + themes config) ──────────────────────
        m.insert(
            ("DELETE", "/api/voice-channels/{guild_id}/history"),
            Role::Admin,
        );
        m.insert(
            (
                "DELETE",
                "/api/voice-channels/by-channel/{channel_id}/purge",
            ),
            Role::Moderator,
        );
        m.insert(
            ("POST", "/api/voice-channels/themes/{guild_id}"),
            Role::Admin,
        );
        m.insert(
            ("PATCH", "/api/voice-channels/themes/{guild_id}/{theme_id}"),
            Role::Admin,
        );
        m.insert(
            ("DELETE", "/api/voice-channels/themes/{guild_id}/{theme_id}"),
            Role::Admin,
        );

        // ── Rotation admin tournant ──────────────────────────────────────
        m.insert(("POST", "/api/rotation/{guild_id}/save"), Role::Admin);

        // ── Systeme : modeles IA / reset / welcome / jobs / exports ──────
        m.insert(("POST", "/api/models/reload"), Role::Owner);
        m.insert(("POST", "/api/system/guild-reset/{guild_id}"), Role::Owner);
        m.insert(("PUT", "/api/welcome/{guild_id}"), Role::Admin);
        m.insert(
            ("POST", "/api/welcome/{guild_id}/rules/publish"),
            Role::Admin,
        );
        m.insert(("POST", "/api/ai/jobs"), Role::Moderator);
        m.insert(("POST", "/api/exports/jobs"), Role::Admin);
        m.insert(
            ("DELETE", "/api/ai-dataset/messages/{guild_id}"),
            Role::Admin,
        );

        // ── RBAC management (Owner only) ─────────────────────────────────
        m.insert(
            ("POST", "/api/rbac/guilds/{guild_id}/users/{user_id}"),
            Role::Owner,
        );
        m.insert(
            ("PATCH", "/api/rbac/guilds/{guild_id}/users/{user_id}"),
            Role::Owner,
        );
        m.insert(
            ("DELETE", "/api/rbac/guilds/{guild_id}/users/{user_id}"),
            Role::Owner,
        );
        m.insert(
            ("PUT", "/api/rbac/component-visibility/{guild_id}"),
            Role::Admin,
        );
        m.insert(
            ("PUT", "/api/rbac/component-min-role/{guild_id}"),
            Role::Owner,
        );
        m.insert(
            (
                "DELETE",
                "/api/rbac/component-min-role/{guild_id}/{component_key}",
            ),
            Role::Owner,
        );

        // ── Invitations (gestion = Owner ; redeem = PUBLIC) ──────────────
        m.insert(("POST", "/api/invitations"), Role::Owner);
        m.insert(("DELETE", "/api/invitations/code/{code}"), Role::Owner);

        // ── Docker (host admin = Owner ; egalement superadmin-gated) ──────
        m.insert(("DELETE", "/api/docker/containers/{id}"), Role::Owner);
        m.insert(("POST", "/api/docker/containers/{id}/start"), Role::Owner);
        m.insert(("POST", "/api/docker/containers/{id}/stop"), Role::Owner);
        m.insert(("POST", "/api/docker/containers/{id}/restart"), Role::Owner);
        m.insert(("DELETE", "/api/docker/images/{id}"), Role::Owner);
        m.insert(("DELETE", "/api/docker/volumes/{name}"), Role::Owner);
        m.insert(("POST", "/api/docker/prune/containers"), Role::Owner);
        m.insert(("POST", "/api/docker/prune/images"), Role::Owner);
        m.insert(("POST", "/api/docker/prune/volumes"), Role::Owner);
        m.insert(("POST", "/api/docker/prune/networks"), Role::Owner);
        m.insert(("POST", "/api/docker/prune/system"), Role::Owner);

        m
    })
}

/// `true` si la methode ne mute pas l'etat (lecture seule).
fn is_safe_method(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// Resout le role requis en tolerant la variation de slash final entre le
/// `MatchedPath` axum et nos cles statiques.
fn required_role(method: &str, path: &str) -> Option<Role> {
    let table = route_roles();
    if let Some(r) = table.get(&(method, path)) {
        return Some(*r);
    }
    // Tolerance slash final (ex: "/api/tickets/" vs "/api/tickets").
    let alt = path.strip_suffix('/').unwrap_or(path);
    if alt != path {
        if let Some(r) = table.get(&(method, alt)) {
            return Some(*r);
        }
    }
    None
}

fn forbidden(msg: &str) -> Response {
    (StatusCode::FORBIDDEN, msg.to_string()).into_response()
}

pub async fn global_rbac_gate(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Interrupteur de securite : ni enforce ni audit → no-op total.
    if !state.rbac_global_gate && !state.rbac_global_gate_audit {
        return next.run(request).await;
    }
    // En mode audit (log-only), on execute toute la logique de decision mais on
    // ne bloque jamais : on journalise ce qui SERAIT refuse et on laisse passer.
    let audit_only = !state.rbac_global_gate && state.rbac_global_gate_audit;

    // Methodes safe : jamais gatees.
    if is_safe_method(request.method()) {
        return next.run(request).await;
    }

    // Service interne de confiance (bot/workers) : acces complet.
    if request.extensions().get::<AuthKind>() == Some(&AuthKind::Internal) {
        return next.run(request).await;
    }

    // Dev mode (pas d'API_KEY) : ne casse pas le local dev.
    if state.api_key.is_empty() {
        return next.run(request).await;
    }

    // Pattern de route (ex: "/api/community/{guild_id}/announcements").
    // Fallback sur l'URI brute si MatchedPath absent (ne devrait pas arriver
    // sur une route matchee, mais on reste fail-closed cote mapping).
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string());

    // Routes publiques / self-service.
    if PUBLIC_PATHS.contains(&path.as_str()) {
        return next.run(request).await;
    }

    let method = request.method().as_str().to_string();
    let ctx = request.extensions().get::<RoleContext>().cloned();

    // Superadmin : bypass global (coherent avec les autres middlewares).
    if let Some(ref c) = ctx {
        if state
            .superadmin_user_ids
            .iter()
            .any(|id| id == &c.discord_user_id)
        {
            return next.run(request).await;
        }
    }

    // Prefixe de log distinguant audit (laisse passer) et enforce (bloque).
    let mode = if audit_only { "AUDIT" } else { "ENFORCE" };

    match required_role(&method, &path) {
        Some(required) => match ctx.as_ref().and_then(|c| c.role) {
            Some(role) if role.satisfies(required) => next.run(request).await,
            other => {
                tracing::warn!(
                    mode = %mode,
                    method = %method,
                    route = %path,
                    required = %required.as_str(),
                    actual = ?other.map(|r| r.as_str()),
                    user_id = ?ctx.as_ref().map(|c| c.discord_user_id.as_str()),
                    "global_rbac_gate: acces refuse (role insuffisant / non resolu)"
                );
                if audit_only {
                    next.run(request).await
                } else {
                    forbidden("Forbidden: role insuffisant pour cette operation")
                }
            }
        },
        None => {
            // Fail-closed : toute route mutante non mappee est refusee pour un
            // user web. Le log permet d'identifier la route a ajouter a la table.
            tracing::error!(
                mode = %mode,
                method = %method,
                route = %path,
                user_id = ?ctx.as_ref().map(|c| c.discord_user_id.as_str()),
                "global_rbac_gate: route mutante NON MAPPEE -> DENY (fail-closed). \
                 Ajouter (methode, pattern) a ROUTE_ROLES si elle doit etre web-accessible."
            );
            if audit_only {
                next.run(request).await
            } else {
                forbidden("Forbidden: route non autorisee pour les utilisateurs web")
            }
        }
    }
}
