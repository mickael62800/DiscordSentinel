//! Couche HTTP axum : router, auth Bearer, handlers.

pub mod dto;
pub mod handlers;

use axum::extract::Request;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{delete, get, patch, post, put};
use axum::Router;

use crate::bootstrap::AppState;

/// Construit le router complet (routes + auth Bearer NEXUS_API_KEY).
pub fn build_router(state: AppState) -> Router {
    let api = Router::new()
        .route(
            "/api/wheel/{guild_id}/{user_id}/spin",
            post(handlers::wheel::spin),
        )
        .route(
            "/api/wheel/{guild_id}/{user_id}/status",
            get(handlers::wheel::status),
        )
        .route(
            "/api/wallet/{guild_id}/transfer",
            post(handlers::wallet::transfer),
        )
        .route(
            "/api/wallet/{guild_id}/leaderboard",
            get(handlers::wallet::leaderboard),
        )
        .route(
            "/api/wallet/{guild_id}/{user_id}",
            get(handlers::wallet::get),
        )
        .route(
            "/api/wallet/{guild_id}/{user_id}/history",
            get(handlers::wallet::history),
        )
        // ── Game Portal : catalogue jeux et panneaux Discord ──
        .route(
            "/api/games/{guild_id}",
            get(handlers::casino::games::list_games),
        )
        .route("/api/games", post(handlers::casino::games::create_game))
        .route(
            "/api/bots/definitions",
            get(handlers::bot_config::get_definitions),
        )
        .route(
            "/api/config/{guild_id}/{bot_name}",
            get(handlers::bot_config::get_config).put(handlers::bot_config::set_config),
        )
        .route("/api/coussin/{guild_id}/{user_id}/profile", get(handlers::coussin::profile))
        .route("/api/coussin/{guild_id}/{user_id}/class", post(handlers::coussin::choose_class))
        .route("/api/coussin/{guild_id}/{user_id}/train", post(handlers::coussin::train))
        .route("/api/coussin/{guild_id}/{user_id}/inventory", get(handlers::coussin::inventory))
        .route("/api/coussin/{guild_id}/{user_id}/shop", post(handlers::coussin::buy_item))
        .route("/api/coussin/{guild_id}/{user_id}/insurance", get(handlers::coussin::insurance).post(handlers::coussin::buy_insurance))
        .route("/api/coussin/{guild_id}/{user_id}/steal", post(handlers::coussin::steal))
        .route("/api/coussin/{guild_id}/{user_id}/prime", post(handlers::coussin::place_prime))
        .route("/api/coussin/{guild_id}/{user_id}/bets", post(handlers::coussin::place_bet))
        .route("/api/coussin/{guild_id}/classement", get(handlers::coussin::ranking))
        .route(
            "/api/coussin/{guild_id}/{user_id}/combats",
            get(handlers::coussin::combat_history),
        )
        .route("/api/coussin/{guild_id}/combats", post(handlers::coussin::challenge))
        .route("/api/coussin/combats/{id}/accept", post(handlers::coussin::accept))
        .route("/api/coussin/combats/{id}/refuse", post(handlers::coussin::refuse))
        .route("/api/coussin/combats/{id}/resolve", post(handlers::coussin::resolve))
        .route(
            "/api/games/{guild_id}/by-category",
            get(handlers::casino::games::list_games_by_category),
        )
        .route(
            "/api/games/{guild_id}/{game_id}",
            put(handlers::casino::games::update_game).delete(handlers::casino::games::delete_game),
        )
        .route(
            "/api/games/{guild_id}/{game_id}/role",
            patch(handlers::casino::games::set_role_id),
        )
        .route(
            "/api/games/{guild_id}/by-name/{game_name}",
            get(handlers::casino::games::get_game_by_name),
        )
        .route(
            "/api/games/{guild_id}/panels",
            get(handlers::casino::games::list_panels).post(handlers::casino::games::save_panel),
        )
        .route(
            "/api/games/{guild_id}/panels/{message_id}",
            get(handlers::casino::games::find_panel_by_message),
        )
        .route(
            "/api/games/{guild_id}/panel/deploy",
            post(handlers::casino::games::deploy_panel),
        )
        .route(
            "/api/games/{guild_id}/upload-emoji",
            post(handlers::casino::games::upload_emoji),
        )
        // ── Game Portal : serveurs, templates et inscriptions ──
        .route(
            "/api/games/{guild_id}/servers",
            post(handlers::game::servers::create_server).get(handlers::game::servers::list_servers),
        )
        .route(
            "/api/games/{guild_id}/templates",
            get(handlers::game::templates::list_templates_for_guild),
        )
        .route(
            "/api/games/{guild_id}/template-settings",
            get(handlers::game::session_events::list_template_settings),
        )
        .route(
            "/api/games/{guild_id}/template-settings/{slug}",
            put(handlers::game::session_events::set_template_role),
        )
        .route(
            "/api/games/templates/{id}",
            get(handlers::game::templates::get_template),
        )
        .route(
            "/api/games/servers/{server_id}",
            get(handlers::game::servers::get_server).delete(handlers::game::servers::delete_server),
        )
        .route(
            "/api/games/servers/{server_id}/start",
            post(handlers::game::servers::start_server),
        )
        .route(
            "/api/games/servers/{server_id}/stop",
            post(handlers::game::servers::stop_server),
        )
        .route(
            "/api/games/servers/{server_id}/restart",
            post(handlers::game::servers::restart_server),
        )
        .route(
            "/api/games/servers/{server_id}/logs",
            get(handlers::game::servers::get_logs),
        )
        .route(
            "/api/games/servers/{server_id}/stats",
            get(handlers::game::servers::get_stats),
        )
        .route(
            "/api/games/servers/{server_id}/config",
            put(handlers::game::servers::update_config),
        )
        .route(
            "/api/games/servers/{server_id}/command",
            post(handlers::game::servers::execute_rcon),
        )
        .route(
            "/api/games/servers/{server_id}/sessions",
            get(handlers::game::sessions::list_sessions),
        )
        .route(
            "/api/games/servers/{server_id}/registrations",
            get(handlers::game::session_events::list_registrations)
                .post(handlers::game::session_events::register_player),
        )
        .route(
            "/api/games/servers/{server_id}/registrations/{user_id}",
            delete(handlers::game::session_events::unregister_player),
        )
        .route(
            "/api/games/servers/{server_id}/session-channels",
            patch(handlers::game::session_events::set_session_channels),
        )
        // Endpoints de travail : uniquement appeles par nexus-worker.
        .route(
            "/api/games/internal/jobs/health-check",
            post(handlers::game::jobs::job_health_check),
        )
        .route(
            "/api/games/internal/jobs/idle-shutdown",
            post(handlers::game::jobs::job_idle_shutdown),
        )
        .route(
            "/api/games/internal/jobs/reconcile",
            post(handlers::game::jobs::job_reconcile),
        )
        .route(
            "/api/games/internal/jobs/image-cleanup",
            post(handlers::game::jobs::job_image_cleanup),
        )
        .route(
            "/api/games/internal/jobs/reveal-ip",
            post(handlers::game::jobs::job_reveal_ip),
        )
        .route(
            "/api/games/internal/jobs/daily-ping",
            post(handlers::game::jobs::job_daily_ping),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_api_key,
        ));

    // Vitrine publique : montee HORS du groupe protege par le Bearer, comme
    // /health. Le DTO est ecrit champ par champ (cf. public_servers.rs).
    let public = Router::new().route(
        "/api/public/games/{guild_id}/servers",
        get(handlers::game::public_servers::public_servers),
    );

    Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(public)
        .merge(api)
        // Verrou mono-serveur applique a TOUT le routeur, public compris.
        // Nexus expose sa propre surface : le verrou de sentinel-api, qui
        // vit dans un autre processus, ne le protege pas.
        .layer(middleware::from_fn_with_state(
            state.clone(),
            single_guild,
        ))
        .with_state(state)
}

/// Refuse toute requete portant un `guild_id` autre que celui configure.
///
/// L'application ne sert qu'un serveur Discord. La colonne `guild_id` reste
/// dans le modele de donnees — la retirer serait un refactor massif pour
/// aucun gain — mais la surface HTTP n'accepte qu'une valeur.
///
/// Les requetes sans identifiant de serveur (sante, routes globales) passent,
/// de meme que TOUT si la variable n'est pas configuree : une installation
/// qui ne l'a pas encore renseignee ne doit pas tomber en panne.
async fn single_guild(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let Some(attendu) = state.guild_id.clone() else {
        return Ok(next.run(req).await);
    };

    // Toutes les routes concernees portent le `guild_id` dans leur chemin.
    // On cherche le premier segment qui ressemble a un identifiant Discord :
    // ici un faux positif provoque un REFUS, d'ou la fenetre stricte de 17 a
    // 20 chiffres, qui ecarte les uuid et les petits entiers.
    let trouve = req
        .uri()
        .path()
        .split('/')
        .find(|seg| {
            (17..=20).contains(&seg.len()) && seg.chars().all(|c| c.is_ascii_digit())
        })
        .map(str::to_string);

    if let Some(gid) = trouve {
        if gid != attendu {
            tracing::warn!(
                guild_id = %gid,
                attendu = %attendu,
                "mono-serveur : requete refusee pour une autre guilde"
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    Ok(next.run(req).await)
}

/// Auth simple : si NEXUS_API_KEY est definie, exige `Authorization: Bearer <key>`
/// sur toutes les routes /api (comme sentinel-api). /health reste ouvert.
async fn require_api_key(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let Some(expected) = &state.api_key else {
        return Ok(next.run(req).await);
    };
    let authorized = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|token| token == expected);
    if !authorized {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(next.run(req).await)
}
