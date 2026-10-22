use crate::{Ctx, error::AppError, layout::Layout};
use askama::Template;
use axum::{Router, response::Html, routing::get};

pub fn mount() -> Router<Ctx> {
    Router::new().route("/", get(dashboard))
}

#[derive(Template)]
#[template(path = "dashboard/index.html")]
struct DashboardTemplate {
    layout: Layout,
}

async fn dashboard(layout: Layout) -> Result<Html<String>, AppError> {
    Ok(Html(DashboardTemplate { layout }.render()?))
}
