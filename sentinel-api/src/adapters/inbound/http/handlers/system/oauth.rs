//! OAuth2 Discord — flux web (panneau d'admin Vue.js).
//!
//! Contrairement au desktop (Tauri) qui fait l'echange code↔token en local
//! avec son propre client_secret, le web passe par le backend pour ne jamais
//! exposer le secret dans le navigateur.
//!
//! Flux :
//!   1. `GET /auth/discord/authorize` → genere un state CSRF (Redis, TTL 10min),
//!      redirige l'utilisateur vers Discord.
//!   2. Discord renvoie l'utilisateur sur `GET /auth/discord/callback?code=&state=`.
//!   3. Le backend verifie le state, echange le code contre un access_token,
//!      appelle `/users/@me` pour recuperer l'identite, puis redirige le
//!      navigateur vers `${WEB_FRONT_URL}/auth/callback#token=…&id=…&username=…`.
//!      Le fragment `#…` (pas la query string) evite que le token n'apparaisse
//!      dans les logs serveur / referer.

use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use redis::AsyncCommands;
use serde::Deserialize;

use crate::adapters::inbound::http::state::AppState;

const STATE_TTL_SECS: u64 = 600;
const STATE_PREFIX: &str = "oauth:web:state:";
const DISCORD_AUTHORIZE_URL: &str = "https://discord.com/api/oauth2/authorize";
const DISCORD_TOKEN_URL: &str = "https://discord.com/api/v10/oauth2/token";
const DISCORD_USER_URL: &str = "https://discord.com/api/v10/users/@me";
const OAUTH_SCOPES: &str = "identify guilds";

fn percent_encode(s: &str) -> String {
    // Encode strict selon RFC 3986 (unreserved = alphanumeric + - . _ ~).
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn redirect_to(location: &str) -> Response {
    let mut headers = HeaderMap::new();
    match header::HeaderValue::from_str(location) {
        Ok(v) => {
            headers.insert(header::LOCATION, v);
            (StatusCode::FOUND, headers).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid redirect location",
        )
            .into_response(),
    }
}

/// Variante de `redirect_to` qui pose en plus un cookie (Set-Cookie).
fn redirect_to_with_cookie(location: &str, cookie: &str) -> Response {
    let mut headers = HeaderMap::new();
    let loc = match header::HeaderValue::from_str(location) {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid redirect location",
            )
                .into_response()
        }
    };
    headers.insert(header::LOCATION, loc);
    if let Ok(c) = header::HeaderValue::from_str(cookie) {
        headers.insert(header::SET_COOKIE, c);
    }
    (StatusCode::FOUND, headers).into_response()
}

const SESSION_COOKIE: &str = "ds_session";
const SESSION_MAX_AGE_SECS: i64 = 30 * 24 * 3600;

/// Cookie de session opaque : httpOnly (invisible au JS), Secure, SameSite=Lax
/// (first-party : front et API derriere le meme reverse proxy en prod).
fn build_session_cookie(id: &str, max_age: i64) -> String {
    format!("{SESSION_COOKIE}={id}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age={max_age}")
}

fn clear_session_cookie() -> String {
    format!("{SESSION_COOKIE}=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0")
}

/// Extrait la valeur d'un cookie depuis l'en-tete `Cookie`.
fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';').find_map(|kv| {
        let kv = kv.trim();
        let (k, v) = kv.split_once('=')?;
        if k.trim() == name {
            Some(v.trim().to_string())
        } else {
            None
        }
    })
}

fn front_error_redirect(front_url: &str, reason: &str) -> Response {
    let target = format!(
        "{}/login?error={}",
        front_url.trim_end_matches('/'),
        percent_encode(reason)
    );
    redirect_to(&target)
}

/// `GET /auth/discord/authorize` — point d'entree du flux OAuth web.
pub async fn authorize(State(state): State<AppState>) -> Response {
    if state.discord_oauth_client_id.is_empty()
        || state.discord_oauth_client_secret.is_empty()
        || state.discord_oauth_redirect_uri.is_empty()
    {
        tracing::error!(
            "OAuth Discord non configure (DISCORD_CLIENT_ID/SECRET/REDIRECT_URI manquants)"
        );
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "OAuth Discord non configure cote serveur",
        )
            .into_response();
    }

    let csrf_state = uuid::Uuid::new_v4().to_string();

    // Stocker le state en Redis avec TTL de 10 min (anti-CSRF + one-shot).
    match state.redis_client.get_multiplexed_async_connection().await {
        Ok(mut conn) => {
            let key = format!("{}{}", STATE_PREFIX, csrf_state);
            if let Err(e) = conn.set_ex::<_, _, ()>(&key, "1", STATE_TTL_SECS).await {
                tracing::error!(error = %e, "Impossible de stocker le state OAuth en Redis");
                return (StatusCode::INTERNAL_SERVER_ERROR, "Redis unavailable").into_response();
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "Connexion Redis impossible pour OAuth");
            return (StatusCode::SERVICE_UNAVAILABLE, "Redis unavailable").into_response();
        }
    }

    // PAS de `prompt=none` ici : Discord refuse silencieusement avec
    // ?error=login_required quand la session navigateur a expire ou
    // que l'app a ete revoquee, ce qui pieges les users dans une boucle
    // /login -> Discord -> /login?error=login_required. Le default
    // Discord (pas de prompt explicite) est exactement ce qu'on veut :
    // re-auth silencieuse si app deja autorisee + session active,
    // consent screen sinon.
    let url = format!(
        "{}?response_type=code&client_id={}&scope={}&redirect_uri={}&state={}",
        DISCORD_AUTHORIZE_URL,
        percent_encode(&state.discord_oauth_client_id),
        percent_encode(OAUTH_SCOPES),
        percent_encode(&state.discord_oauth_redirect_uri),
        percent_encode(&csrf_state),
    );

    redirect_to(&url)
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[allow(dead_code)]
    token_type: Option<String>,
    #[allow(dead_code)]
    expires_in: Option<i64>,
    #[allow(dead_code)]
    refresh_token: Option<String>,
    #[allow(dead_code)]
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DiscordMe {
    id: String,
    username: String,
    #[serde(default)]
    global_name: Option<String>,
    #[serde(default)]
    avatar: Option<String>,
}

/// `GET /auth/discord/callback` — Discord nous renvoie l'utilisateur ici.
pub async fn callback(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(q): Query<CallbackQuery>,
) -> Response {
    let front = if state.web_front_url.is_empty() {
        "/".to_string()
    } else {
        state.web_front_url.clone()
    };

    // Cas d'erreur renvoye par Discord (refus user, scope invalide, etc.)
    if let Some(err) = q.error {
        let reason = q.error_description.unwrap_or(err);
        tracing::warn!(reason = %reason, "Discord a renvoye une erreur OAuth");
        return front_error_redirect(&front, &reason);
    }

    let code = match q.code {
        Some(c) if !c.is_empty() => c,
        _ => return front_error_redirect(&front, "code_manquant"),
    };
    let csrf_state = match q.state {
        Some(s) if !s.is_empty() => s,
        _ => return front_error_redirect(&front, "state_manquant"),
    };

    // 1. Verifier + consommer le state CSRF (one-shot, pop atomique).
    let mut redis_conn = match state.redis_client.get_multiplexed_async_connection().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "Redis down pendant callback OAuth");
            return front_error_redirect(&front, "redis_unavailable");
        }
    };
    let state_key = format!("{}{}", STATE_PREFIX, csrf_state);
    let existed: Option<String> = redis_conn.get(&state_key).await.unwrap_or(None);
    if existed.is_none() {
        tracing::warn!("state OAuth inconnu ou expire");
        return front_error_redirect(&front, "state_invalide");
    }
    let _: Result<(), _> = redis_conn.del::<_, ()>(&state_key).await;

    // 2. Echanger le code contre un access_token via Discord.
    let client = reqwest::Client::new();
    let form = [
        ("client_id", state.discord_oauth_client_id.as_str()),
        ("client_secret", state.discord_oauth_client_secret.as_str()),
        ("grant_type", "authorization_code"),
        ("code", code.as_str()),
        ("redirect_uri", state.discord_oauth_redirect_uri.as_str()),
    ];

    let token_resp = match client
        .post(DISCORD_TOKEN_URL)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .form(&form)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "Echec appel /oauth2/token");
            return front_error_redirect(&front, "discord_token_error");
        }
    };

    if !token_resp.status().is_success() {
        let status = token_resp.status();
        let body = token_resp.text().await.unwrap_or_default();
        tracing::error!(%status, %body, "Discord /oauth2/token a renvoye une erreur");
        return front_error_redirect(&front, "code_invalide");
    }

    let token: TokenResponse = match token_resp.json().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "Parse TokenResponse impossible");
            return front_error_redirect(&front, "discord_token_parse");
        }
    };

    // 3. Recuperer l'identite du user via /users/@me.
    let user_resp = match client
        .get(DISCORD_USER_URL)
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", token.access_token),
        )
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "Echec appel /users/@me");
            return front_error_redirect(&front, "discord_me_error");
        }
    };

    if !user_resp.status().is_success() {
        let status = user_resp.status();
        tracing::error!(%status, "Discord /users/@me a renvoye une erreur");
        return front_error_redirect(&front, "discord_me_status");
    }

    let me: DiscordMe = match user_resp.json().await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!(error = %e, "Parse /users/@me impossible");
            return front_error_redirect(&front, "discord_me_parse");
        }
    };

    // 3.5. Trace le login reussi pour la page Securite serveur (best-effort).
    let client_ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "-".to_string());
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.chars().take(500).collect::<String>())
        .unwrap_or_default();
    let pool = state.pg_pool.clone();
    let user_id = me.id.clone();
    let username = me.username.clone();
    tokio::spawn(async move {
        let res = sqlx::query(
            "INSERT INTO successful_logins (discord_user_id, username, client_ip, user_agent) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(&user_id)
        .bind(&username)
        .bind(&client_ip)
        .bind(&user_agent)
        .execute(&pool)
        .await;
        if let Err(e) = res {
            tracing::warn!(error = %e, "Echec insert successful_logins");
        }
    });

    // 3.7. Cree une session web persistante (refresh token cote serveur) pour
    //      le "rester connecte". Best-effort : si pas de refresh_token ou echec
    //      DB, on degrade vers le comportement actuel (re-login a l'expiration).
    let session_cookie: Option<String> = if let Some(refresh) = token.refresh_token.as_deref() {
        let session_id = uuid::Uuid::new_v4();
        let expires_in = token.expires_in.unwrap_or(604800).max(0);
        let access_exp = chrono::Utc::now() + chrono::Duration::seconds(expires_in);
        let res = sqlx::query(
            "INSERT INTO web_oauth_sessions \
                (id, discord_user_id, username, global_name, avatar, access_token, refresh_token, access_expires_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(session_id)
        .bind(&me.id)
        .bind(&me.username)
        .bind(&me.global_name)
        .bind(&me.avatar)
        .bind(&token.access_token)
        .bind(refresh)
        .bind(access_exp)
        .execute(&state.pg_pool)
        .await;
        match res {
            Ok(_) => Some(build_session_cookie(
                &session_id.to_string(),
                SESSION_MAX_AGE_SECS,
            )),
            Err(e) => {
                tracing::warn!(error = %e, "Echec creation session web (refresh) -- login sans persistance");
                None
            }
        }
    } else {
        None
    };

    // 4. Rediriger le navigateur vers le front avec les infos dans le FRAGMENT
    //    (apres `#`) pour eviter que le token n'apparaisse dans les logs serveur,
    //    le referer ou l'historique intermediaire. Le front lit `location.hash`
    //    puis nettoie l'URL.
    let fragment = format!(
        "token={}&id={}&username={}&global_name={}&avatar={}",
        percent_encode(&token.access_token),
        percent_encode(&me.id),
        percent_encode(&me.username),
        percent_encode(me.global_name.as_deref().unwrap_or("")),
        percent_encode(me.avatar.as_deref().unwrap_or("")),
    );
    let target = format!("{}/auth/callback#{}", front.trim_end_matches('/'), fragment);

    match session_cookie {
        Some(c) => redirect_to_with_cookie(&target, &c),
        None => redirect_to(&target),
    }
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    discord_user_id: String,
    username: String,
    global_name: Option<String>,
    avatar: Option<String>,
    access_token: String,
    refresh_token: String,
    access_expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize)]
struct RefreshResponse {
    token: String,
    id: String,
    username: String,
    global_name: Option<String>,
    avatar: Option<String>,
}

fn unauthorized_clear_cookie() -> Response {
    let mut headers = HeaderMap::new();
    if let Ok(c) = header::HeaderValue::from_str(&clear_session_cookie()) {
        headers.insert(header::SET_COOKIE, c);
    }
    (StatusCode::UNAUTHORIZED, headers, "no session").into_response()
}

/// `POST /auth/refresh` — ré-émet un token d'accès Discord à partir du cookie
/// de session (refresh token côté serveur). Permet de rester connecté après
/// fermeture du navigateur sans re-validation interactive.
pub async fn refresh(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let sid = match cookie_value(&headers, SESSION_COOKIE) {
        Some(s) if !s.is_empty() => s,
        _ => return unauthorized_clear_cookie(),
    };
    let session_id = match uuid::Uuid::parse_str(&sid) {
        Ok(u) => u,
        Err(_) => return unauthorized_clear_cookie(),
    };

    let row: Option<SessionRow> = sqlx::query_as(
        "SELECT discord_user_id, username, global_name, avatar, access_token, refresh_token, access_expires_at \
         FROM web_oauth_sessions WHERE id = $1",
    )
    .bind(session_id)
    .fetch_optional(&state.pg_pool)
    .await
    .unwrap_or(None);

    let Some(s) = row else {
        return unauthorized_clear_cookie();
    };

    // Token encore valide (marge 60s) -> on le renvoie tel quel.
    let now = chrono::Utc::now();
    if s.access_expires_at > now + chrono::Duration::seconds(60) {
        let _ = sqlx::query("UPDATE web_oauth_sessions SET last_used_at = NOW() WHERE id = $1")
            .bind(session_id)
            .execute(&state.pg_pool)
            .await;
        return axum::Json(RefreshResponse {
            token: s.access_token,
            id: s.discord_user_id,
            username: s.username,
            global_name: s.global_name,
            avatar: s.avatar,
        })
        .into_response();
    }

    // Sinon : on rafraichit aupres de Discord (grant_type=refresh_token).
    let client = reqwest::Client::new();
    let form = [
        ("client_id", state.discord_oauth_client_id.as_str()),
        ("client_secret", state.discord_oauth_client_secret.as_str()),
        ("grant_type", "refresh_token"),
        ("refresh_token", s.refresh_token.as_str()),
    ];
    let resp = match client
        .post(DISCORD_TOKEN_URL)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .form(&form)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            tracing::warn!(status = %r.status(), "Refresh Discord refuse -> session invalidee");
            let _ = sqlx::query("DELETE FROM web_oauth_sessions WHERE id = $1")
                .bind(session_id)
                .execute(&state.pg_pool)
                .await;
            return unauthorized_clear_cookie();
        }
        Err(e) => {
            tracing::error!(error = %e, "Echec appel refresh Discord");
            return (StatusCode::SERVICE_UNAVAILABLE, "discord unavailable").into_response();
        }
    };
    let token: TokenResponse = match resp.json().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "Parse refresh TokenResponse impossible");
            return (StatusCode::BAD_GATEWAY, "discord parse").into_response();
        }
    };

    let expires_in = token.expires_in.unwrap_or(604800).max(0);
    let new_exp = now + chrono::Duration::seconds(expires_in);
    // Discord peut faire tourner le refresh_token : on garde le nouveau s'il existe.
    let new_refresh = token.refresh_token.clone().unwrap_or(s.refresh_token);
    let _ = sqlx::query(
        "UPDATE web_oauth_sessions SET access_token = $2, refresh_token = $3, \
            access_expires_at = $4, last_used_at = NOW() WHERE id = $1",
    )
    .bind(session_id)
    .bind(&token.access_token)
    .bind(&new_refresh)
    .bind(new_exp)
    .execute(&state.pg_pool)
    .await;

    axum::Json(RefreshResponse {
        token: token.access_token,
        id: s.discord_user_id,
        username: s.username,
        global_name: s.global_name,
        avatar: s.avatar,
    })
    .into_response()
}

/// `POST /auth/logout` — supprime la session serveur + efface le cookie.
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(sid) = cookie_value(&headers, SESSION_COOKIE) {
        if let Ok(session_id) = uuid::Uuid::parse_str(&sid) {
            let _ = sqlx::query("DELETE FROM web_oauth_sessions WHERE id = $1")
                .bind(session_id)
                .execute(&state.pg_pool)
                .await;
        }
    }
    let mut resp_headers = HeaderMap::new();
    if let Ok(c) = header::HeaderValue::from_str(&clear_session_cookie()) {
        resp_headers.insert(header::SET_COOKIE, c);
    }
    (StatusCode::NO_CONTENT, resp_headers).into_response()
}

#[cfg(test)]
#[path = "tests/oauth.rs"]
mod tests;
