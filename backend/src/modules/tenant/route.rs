//! 租户路由。
//!
//! 租户管理接口挂载在 `/api/v1/system/tenants`，与普通租户内业务接口保持清晰边界。

use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::{AppState, modules::tenant::handler};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/system/tenants",
            post(handler::create_tenant).get(handler::page_tenants),
        )
        .route(
            "/api/v1/system/tenants/{id}",
            get(handler::get_tenant)
                .put(handler::update_tenant)
                .delete(handler::delete_tenant),
        )
        .route(
            "/api/v1/system/tenants/{id}/physical",
            delete(handler::physical_delete_tenant),
        )
        .route(
            "/api/v1/system/tenants/{id}/enable",
            post(handler::enable_tenant),
        )
        .route(
            "/api/v1/system/tenants/{id}/disable",
            post(handler::disable_tenant),
        )
        .route(
            "/api/v1/system/tenants/{id}/suspend",
            post(handler::suspend_tenant),
        )
}
