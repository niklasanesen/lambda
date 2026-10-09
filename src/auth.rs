use crate::{
    Ctx,
    database::{Connection, DatabaseConnection},
    error::AppError,
};
use anyhow::Context;
use axum::{
    Router,
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    routing::get,
};
use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite},
};
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use redis::AsyncCommands;
use serde::Deserialize;
use uuid::Uuid;

const CSRF_TOKEN: &str = "csrf_token";
const SESSION_MAX_AGE: u64 = 60 * 60 * 24 * 7;
pub const SESSION: &str = "session";

pub fn mount() -> Router<Ctx> {
    Router::new()
        .route("/google", get(google))
        .route("/github", get(github))
        .route("/google/callback", get(google_callback))
        .route("/github/callback", get(github_callback))
}

async fn google(State(Ctx { google, prod, .. }): State<Ctx>, jar: CookieJar) -> impl IntoResponse {
    let (auth_url, csrf_token) = google
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("email".to_owned()))
        .url();
    (
        set_csrf_token(csrf_token, "google", prod, jar),
        Redirect::to(auth_url.as_ref()),
    )
}

async fn github(State(Ctx { github, prod, .. }): State<Ctx>, jar: CookieJar) -> impl IntoResponse {
    let (auth_url, csrf_token) = github
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("user:email".to_owned()))
        .url();
    (
        set_csrf_token(csrf_token, "github", prod, jar),
        Redirect::to(auth_url.as_ref()),
    )
}

fn set_csrf_token(csrf_token: CsrfToken, provider: &str, prod: bool, jar: CookieJar) -> CookieJar {
    let cookie = Cookie::build((CSRF_TOKEN, csrf_token.into_secret()))
        .path(format!("/auth/{provider}"))
        .http_only(true)
        .secure(prod)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(600));
    jar.add(cookie)
}

#[derive(Deserialize)]
struct CallbackParams {
    state: String,
    code: String,
}

#[derive(Deserialize)]
struct GoogleUser {
    sub: String,
    email: String,
    email_verified: bool,
}

async fn google_callback(
    jar: CookieJar,
    Query(CallbackParams { state, code }): Query<CallbackParams>,
    State(Ctx {
        google,
        reqwest,
        prod,
        client_url,
        ..
    }): State<Ctx>,
    DatabaseConnection(mut conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    check_csrf_token(&jar, &state)?;

    let token = google
        .exchange_code(AuthorizationCode::new(code))
        .request_async(&reqwest)
        .await
        .context("failed to exchange code for google access token")?;

    let user: GoogleUser = reqwest
        .get("https://openidconnect.googleapis.com/v1/userinfo")
        .bearer_auth(token.access_token().secret())
        .send()
        .await
        .context("failed to get google user")?
        .json()
        .await
        .context("failed to deserialize google user")?;

    if !user.email_verified {
        return Err(anyhow::anyhow!("unverified google email").into());
    }

    let user_id = upsert_user("google", &user.sub, &mut conn, &user.email).await?;

    Ok((
        set_session(&mut conn, user_id, prod, jar).await?,
        Redirect::to(&format!("{client_url}/dashboard/index.html")),
    ))
}

#[derive(Deserialize)]
struct GithubUser {
    id: u64,
}

#[derive(Deserialize)]
struct GithubEmail {
    email: String,
    verified: bool,
    primary: bool,
}

async fn github_callback(
    jar: CookieJar,
    Query(CallbackParams { state, code }): Query<CallbackParams>,
    State(Ctx {
        github,
        reqwest,
        prod,
        client_url,
        ..
    }): State<Ctx>,
    DatabaseConnection(mut conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    check_csrf_token(&jar, &state)?;

    let token = github
        .exchange_code(AuthorizationCode::new(code))
        .request_async(&reqwest)
        .await
        .context("failed to exchange code for github access token")?;
    let access_token = token.access_token().secret();

    let user: GithubUser = reqwest
        .get("https://api.github.com/user")
        .bearer_auth(access_token)
        .send()
        .await
        .context("failed to get github user")?
        .json()
        .await
        .context("failed to deserialize github user")?;

    let email = reqwest
        .get("https://api.github.com/user/emails")
        .bearer_auth(access_token)
        .send()
        .await
        .context("failed to get github emails")?
        .json::<Vec<GithubEmail>>()
        .await
        .context("failed to deserialize github emails")?
        .into_iter()
        .find(|e| e.primary && e.verified)
        .map(|e| e.email)
        .context("unverified primary github email")?;

    let user_id = upsert_user("github", user.id, &mut conn, &email).await?;

    Ok((
        set_session(&mut conn, user_id, prod, jar).await?,
        Redirect::to(&format!("{client_url}/dashboard/index.html")),
    ))
}

fn check_csrf_token(jar: &CookieJar, state: &str) -> Result<(), AppError> {
    if !jar.get(CSRF_TOKEN).is_some_and(|c| c.value() == state) {
        return Err(AppError::Unauthorized);
    }
    Ok(())
}

async fn upsert_user(
    provider: &str,
    provider_id: impl std::fmt::Display,
    conn: &mut Connection,
    email: &str,
) -> anyhow::Result<Uuid> {
    let provider_key = format!("{provider}:{provider_id}");
    let exists: Option<Uuid> = conn.get(&provider_key).await?;
    let id = exists.unwrap_or_else(Uuid::now_v7);
    if exists.is_none() {
        conn.set::<_, _, ()>(provider_key, id).await?;
    }
    conn.set::<_, _, ()>(format!("user:{id}"), email).await?;
    Ok(id)
}

async fn set_session(
    conn: &mut Connection,
    user_id: Uuid,
    prod: bool,
    jar: CookieJar,
) -> anyhow::Result<CookieJar> {
    let id = Uuid::new_v4();
    conn.set_ex::<_, _, ()>(format!("session:{id}"), user_id, SESSION_MAX_AGE)
        .await?;
    let cookie = Cookie::build((SESSION, id.to_string()))
        .path("/")
        .http_only(true)
        .secure(prod)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(SESSION_MAX_AGE as i64));
    Ok(jar.add(cookie))
}
