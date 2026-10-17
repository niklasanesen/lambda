use crate::Ctx;
use axum::{
    Router,
    extract::State,
    response::{IntoResponse, Redirect},
    routing::get,
};
use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite},
};
use oauth2::{CsrfToken, Scope};

const CSRF_TOKEN: &str = "csrf_token";

pub fn mount() -> Router<Ctx> {
    Router::new()
        .route("/google", get(google))
        .route("/github", get(github))
}

async fn google(State(Ctx { google, prod, .. }): State<Ctx>, jar: CookieJar) -> impl IntoResponse {
    let (auth_url, csrf_token) = google
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".to_owned()))
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
