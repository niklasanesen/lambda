use crate::{
    auth::SESSION,
    database::{ConnectionPool, DatabaseConnection},
    error::AppError,
};
use axum::{
    Router,
    extract::Request,
    http::{HeaderValue, Method},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use axum_extra::extract::CookieJar;
use oauth2::{AuthUrl, ClientId, ClientSecret, EndpointNotSet, EndpointSet, RedirectUrl, TokenUrl};
use redis::AsyncCommands;
use tower_http::{
    cors::{AllowHeaders, CorsLayer},
    services::ServeDir,
};
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
    reqwest: reqwest::Client,
    prod: bool,
    redis: ConnectionPool,
    client_url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let api_url = std::env::var("API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let prod =
        std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_owned()) == "production";
    let client_url =
        std::env::var("CLIENT_URL").unwrap_or_else(|_| "http://127.0.0.1:8080/web".to_owned());

    let cors = CorsLayer::new()
        .allow_origin(client_url.parse::<HeaderValue>()?)
        .allow_methods([Method::GET])
        .allow_headers(AllowHeaders::mirror_request())
        .allow_credentials(true)
        .max_age(std::time::Duration::from_secs(600));

    let google = oauth_client(
        std::env::var("GOOGLE_CLIENT_ID")?,
        std::env::var("GOOGLE_CLIENT_SECRET")?,
        "https://accounts.google.com/o/oauth2/v2/auth".to_owned(),
        "https://oauth2.googleapis.com/token".to_owned(),
        format!("{api_url}/auth/google/callback"),
    )?;

    let github = oauth_client(
        std::env::var("GITHUB_CLIENT_ID")?,
        std::env::var("GITHUB_CLIENT_SECRET")?,
        "https://github.com/login/oauth/authorize".to_owned(),
        "https://github.com/login/oauth/access_token".to_owned(),
        format!("{api_url}/auth/github/callback"),
    )?;

    let reqwest = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("lambda")
        .build()?;

    let redis = bb8::Pool::builder()
        .build(redis::Client::open(std::env::var("REDIS_URL")?)?)
        .await?;

    let ctx = Ctx {
        google,
        github,
        reqwest,
        prod,
        redis,
        client_url,
    };

    let protected = Router::new()
        .nest("/dashboard", dashboard::mount())
        .route_layer(middleware::from_fn_with_state(ctx.clone(), auth_middleware));

    let mut app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .nest("/auth", auth::mount())
        .merge(protected)
        .layer(cors)
        .with_state(ctx);

    if !prod {
        app = app.nest_service("/web", ServeDir::new("web"))
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    println!("listening to http://{}", listener.local_addr()?);
    Ok(axum::serve(listener, app).await?)
}

fn oauth_client(
    client_id: String,
    client_secret: String,
    auth_url: String,
    token_url: String,
    redirect_url: String,
) -> anyhow::Result<BasicClient> {
    Ok(oauth2::basic::BasicClient::new(ClientId::new(client_id))
        .set_client_secret(ClientSecret::new(client_secret))
        .set_auth_uri(AuthUrl::new(auth_url)?)
        .set_token_uri(TokenUrl::new(token_url)?)
        .set_redirect_uri(RedirectUrl::new(redirect_url)?))
}

#[derive(Clone)]
pub struct UserId(Uuid);

async fn auth_middleware(
    jar: CookieJar,
    DatabaseConnection(mut conn): DatabaseConnection,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let session_id = jar.get(SESSION).ok_or(AppError::Unauthorized)?.value();
    let exists: Option<Uuid> = conn.get(format!("session:{session_id}")).await?;
    let user_id = exists.ok_or(AppError::Unauthorized)?;
    req.extensions_mut().insert(UserId(user_id));
    Ok(next.run(req).await)
}
