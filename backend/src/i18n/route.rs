//! 多语言路由。
//!
//! 当前只开放系统资源分发接口，业务数据多语言读写接口待具体业务模块落地时按资源权限接入。

use axum::{Router, routing::get};

use crate::{AppState, i18n::handler};

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/api/v1/i18n/system_resources",
        get(handler::system_resources),
    )
}
