use crate::{Ctx, error::AppError, layout::Layout};
use askama::Template;
use axum::{Router, response::Html, routing::get};

pub fn mount() -> Router<Ctx> {
    Router::new().route("/", get(projects))
}

#[derive(Template)]
#[template(path = "projects/index.html")]
struct ProjectsTemplate {
    layout: Layout,
}

async fn projects(layout: Layout) -> Result<Html<String>, AppError> {
    Ok(Html(ProjectsTemplate { layout }.render()?))
}
