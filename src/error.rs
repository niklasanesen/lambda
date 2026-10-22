use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

pub struct AppError(anyhow::Error);

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(
                ErrorTemplate {
                    message: self.0.to_string(),
                }
                .render()
                .unwrap_or_else(|_| "something went wrong".to_owned()),
            ),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
