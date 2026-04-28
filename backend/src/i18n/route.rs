//! 多语言路由。
//!
//! 系统资源和业务数据翻译接口统一挂在 `/api/v1/i18n` 下，便于前端和移动端复用语言上下文。

use axum::{Router, routing::get};

use crate::{AppState, i18n::handler};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/i18n/system_resources",
            get(handler::system_resources),
        )
        .route(
            "/api/v1/i18n/business_translations/{resource_type}/{resource_id}",
            get(handler::get_business_translations).put(handler::put_business_translations),
        )
}
