use crate::{Ctx, error::AppError, layout::Layout};
use askama::Template;
use axum::{Router, response::Html, routing::get};

pub fn mount() -> Router<Ctx> {
    Router::new().route("/", get(settings))
}

#[derive(Template)]
#[template(path = "settings/index.html")]
struct SettingsTemplate {
    layout: Layout,
}

async fn settings(layout: Layout) -> Result<Html<String>, AppError> {
    Ok(Html(SettingsTemplate { layout }.render()?))
}
