use crate::{auth::SESSION, database::DatabaseConnection, error::AppError};
use axum::{
    Router,
    extract::{Request, State},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use oauth2::{AuthUrl, ClientId, ClientSecret, EndpointNotSet, EndpointSet, RedirectUrl, TokenUrl};
use redis::AsyncCommands;
use tower_http::services::ServeDir;
use uuid::Uuid;

mod auth;
mod dashboard;
mod database;
mod error;

type BasicClient = oauth2::basic::BasicClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
>;

#[derive(Clone)]
struct Ctx {
    google: BasicClient,
    github: BasicClient,
    prod: bool,
    reqwest: reqwest::Client,
    redis: database::ConnectionPool,
    landing_url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let app_url = std::env::var("APP_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let prod =
        std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_owned()) == "production";
    let landing_url = std::env::var("LANDING_URL").unwrap_or_else(|_| "/landing".to_owned());

    let google = oauth_client(
        std::env::var("GOOGLE_CLIENT_ID")?,
        std::env::var("GOOGLE_CLIENT_SECRET")?,
        "https://accounts.google.com/o/oauth2/v2/auth",
        "https://oauth2.googleapis.com/token",
        format!("{app_url}/auth/google/callback"),
    )?;

    let github = oauth_client(
        std::env::var("GITHUB_CLIENT_ID")?,
        std::env::var("GITHUB_CLIENT_SECRET")?,
        "https://github.com/login/oauth/authorize",
        "https://github.com/login/oauth/access_token",
        format!("{app_url}/auth/github/callback"),
    )?;

    let reqwest = reqwest::Client::builder()
        .user_agent("lambda/0.1.0")
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let redis = bb8::Pool::builder()
        .build(redis::Client::open(std::env::var("REDIS_URL")?)?)
        .await?;

    let ctx = Ctx {
        google,
        github,
        prod,
        reqwest,
        redis,
        landing_url,
    };

    let protected = Router::new()
        .merge(dashboard::mount())
        .route_layer(middleware::from_fn_with_state(ctx.clone(), auth_middleware));

    let mut app = Router::new()
        .nest("/auth", auth::mount())
        .merge(protected)
        .with_state(ctx);

    if !prod {
        app = app.nest_service("/landing", ServeDir::new("landing"))
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    println!("listening to http://{}", listener.local_addr()?);
    Ok(axum::serve(listener, app).await?)
}

fn oauth_client(
    client_id: String,
    client_secret: String,
    auth_url: &str,
    token_url: &str,
    redirect_url: String,
) -> anyhow::Result<BasicClient> {
    Ok(oauth2::basic::BasicClient::new(ClientId::new(client_id))
        .set_client_secret(ClientSecret::new(client_secret))
        .set_auth_uri(AuthUrl::new(auth_url.to_owned())?)
        .set_token_uri(TokenUrl::new(token_url.to_owned())?)
        .set_redirect_uri(RedirectUrl::new(redirect_url)?))
}

#[derive(Clone)]
pub struct UserId(Uuid);

async fn auth_middleware(
    jar: CookieJar,
    DatabaseConnection(mut conn): DatabaseConnection,
    State(Ctx { landing_url, .. }): State<Ctx>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    if let Some(session) = jar.get(SESSION)
        && let Some(user_id) = conn.get(format!("session:{}", session.value())).await?
    {
        req.extensions_mut().insert(UserId(user_id));
        return Ok(next.run(req).await);
    }
    let login_url = format!("{landing_url}/login.html");
    Ok(if req.headers().contains_key("hx-request") {
        [("hx-redirect", login_url)].into_response()
    } else {
        Redirect::to(&login_url).into_response()
    })
}
