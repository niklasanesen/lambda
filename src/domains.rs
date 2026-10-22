use crate::{Ctx, error::AppError, layout::Layout};
use askama::Template;
use axum::{Router, response::Html, routing::get};

pub fn mount() -> Router<Ctx> {
    Router::new().route("/", get(domains))
}

#[derive(Template)]
#[template(path = "domains/index.html")]
struct DomainsTemplate {
    layout: Layout,
}

async fn domains(layout: Layout) -> Result<Html<String>, AppError> {
    Ok(Html(DomainsTemplate { layout }.render()?))
}
