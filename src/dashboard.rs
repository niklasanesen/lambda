use crate::{Ctx, UserId, database::DatabaseConnection, error::AppError};
use anyhow::Context;
use askama::Template;
use axum::{Extension, Router, response::Html, routing::get};
use redis::AsyncCommands;

pub fn mount() -> Router<Ctx> {
    Router::new().route("/avatar", get(avatar))
}

#[derive(Template)]
#[template(
    ext = "html",
    source = r#"
<span class="size-8 rounded-full bg-neutral-800 flex items-center justify-center shrink-0 select-none">
    {{initial}}
</span>
"#
)]
struct AvatarTemplate {
    initial: String,
}

async fn avatar(
    DatabaseConnection(mut conn): DatabaseConnection,
    Extension(UserId(user_id)): Extension<UserId>,
) -> Result<Html<String>, AppError> {
    let exists: Option<String> = conn.get(format!("user:{user_id}")).await?;
    let email = exists.context("failed to get user email from session")?;
    let initial = email.chars().next().context("empty email")?.to_string();
    Ok(Html(AvatarTemplate { initial }.render()?))
}
