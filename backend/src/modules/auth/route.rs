//! 认证模块路由。
//!
//! `/api/v1/auth` 服务终端登录态，`/api/v1/system/auth` 服务账号和会话管理。

use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::{AppState, modules::auth::handler};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/login", post(handler::login))
        .route("/api/v1/auth/refresh", post(handler::refresh))
        .route("/api/v1/auth/logout", post(handler::logout))
        .route("/api/v1/auth/logout_all", post(handler::logout_all))
        .route("/api/v1/auth/me", get(handler::me))
        .route("/api/v1/auth/tenants", get(handler::my_tenants))
        .route("/api/v1/auth/switch_tenant", post(handler::switch_tenant))
        .route(
            "/api/v1/system/auth/accounts",
            post(handler::create_account).get(handler::page_accounts),
        )
        .route(
            "/api/v1/system/auth/accounts/{id}",
            get(handler::get_account)
                .put(handler::update_account)
                .delete(handler::delete_account),
        )
        .route(
            "/api/v1/system/auth/accounts/{id}/physical",
            delete(handler::physical_delete_account),
        )
        .route(
            "/api/v1/system/auth/accounts/{id}/enable",
            post(handler::enable_account),
        )
        .route(
            "/api/v1/system/auth/accounts/{id}/disable",
            post(handler::disable_account),
        )
        .route(
            "/api/v1/system/auth/accounts/{id}/lock",
            post(handler::lock_account),
        )
        .route(
            "/api/v1/system/auth/accounts/{id}/reset_password",
            post(handler::reset_password),
        )
        .route(
            "/api/v1/system/auth/account_tenants",
            post(handler::create_account_tenant).get(handler::page_account_tenants),
        )
        .route(
            "/api/v1/system/auth/account_tenants/{id}",
            axum::routing::put(handler::update_account_tenant)
                .delete(handler::delete_account_tenant),
        )
        .route("/api/v1/system/auth/sessions", get(handler::page_sessions))
        .route(
            "/api/v1/system/auth/sessions/{id}/revoke",
            post(handler::revoke_session),
        )
}
