use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

pub enum AppError {
    Internal(anyhow::Error),
    Unauthorized,
}

#[derive(Template)]
#[template(ext = "html", source = r#"<p class="text-red-500">{{message}}</p>"#)]
struct ErrorTemplate {
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_owned()),
        };
        (
            status,
            Html(
                ErrorTemplate { message }
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
        AppError::Internal(err.into())
    }
}
