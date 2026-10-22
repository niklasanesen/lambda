use crate::{
    Ctx, UserId,
    auth::User,
    database::{ConnectionPool, DatabaseConnection},
    error::AppError,
};
use anyhow::Context;
use axum::{
    RequestPartsExt,
    extract::{Extension, FromRef, FromRequestParts, OriginalUri},
    http::request::Parts,
};
use uuid::Uuid;

pub struct Layout {
    pub path: String,
    pub landing_url: String,
    pub user: User,
    pub team_id: Uuid,
}

impl<S> FromRequestParts<S> for Layout
where
    Ctx: FromRef<S>,
    ConnectionPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let OriginalUri(original_uri) = parts.extract::<OriginalUri>().await?;
        let ctx = Ctx::from_ref(state);
        let DatabaseConnection(mut conn) = parts
            .extract_with_state::<DatabaseConnection, _>(state)
            .await?;
        let Extension(UserId(user_id)) = parts.extract::<Extension<UserId>>().await?;
        let (user_json, teams): (String, Vec<Uuid>) = redis::pipe()
            .get(format!("user:{user_id}"))
            .smembers(format!("user:{user_id}:teams"))
            .query_async(&mut *conn)
            .await?;
        Ok(Layout {
            path: original_uri.path().to_owned(),
            landing_url: ctx.landing_url,
            user: serde_json::from_str(&user_json)?,
            team_id: teams.into_iter().next().context("user has no team")?,
        })
    }
}

impl Layout {
    pub fn title(&self) -> String {
        self.path
            .rsplit('/')
            .find(|s| !s.is_empty())
            .unwrap_or("dashboard")
            .replace('-', " ")
    }
    pub fn initial(&self) -> char {
        self.user.email.chars().next().unwrap_or('u')
    }
}
