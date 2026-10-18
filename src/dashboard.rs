use crate::{Ctx, UserId};
use axum::{Extension, Router, response::Html, routing::get};

pub fn mount() -> Router<Ctx> {
    Router::new().route("/", get(root))
}

async fn root(Extension(UserId(user_id)): Extension<UserId>) -> Html<String> {
    Html(format!("<h1>hello {user_id}</h1>"))
}
