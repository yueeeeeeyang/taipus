use axum::{Router, routing::get};

use crate::{AppState, health::handler};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health/live", get(handler::live))
        .route("/health/ready", get(handler::ready))
}
