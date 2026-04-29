//! HRM 路由。
//!
//! 所有 HRM 管理接口统一挂载在 `/api/v1/hrm` 下，路径使用 snake_case，避免把内部实现细节暴露
//! 给调用方。权限检查入口保留在 service 层，后续接入权限模块时无需调整路由结构。

use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::{AppState, modules::hrm::handler};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/hrm/users",
            post(handler::create_user).get(handler::page_users),
        )
        .route(
            "/api/v1/hrm/users/{id}",
            get(handler::get_user)
                .put(handler::update_user)
                .delete(handler::delete_user),
        )
        .route(
            "/api/v1/hrm/users/{id}/physical",
            delete(handler::physical_delete_user),
        )
        .route(
            "/api/v1/hrm/orgs",
            post(handler::create_org).get(handler::page_orgs),
        )
        .route(
            "/api/v1/hrm/orgs/{id}",
            get(handler::get_org)
                .put(handler::update_org)
                .delete(handler::delete_org),
        )
        .route(
            "/api/v1/hrm/orgs/{id}/physical",
            delete(handler::physical_delete_org),
        )
        .route("/api/v1/hrm/org_tree", get(handler::org_tree))
        .route(
            "/api/v1/hrm/posts",
            post(handler::create_post).get(handler::page_posts),
        )
        .route(
            "/api/v1/hrm/posts/{id}",
            get(handler::get_post)
                .put(handler::update_post)
                .delete(handler::delete_post),
        )
        .route(
            "/api/v1/hrm/posts/{id}/physical",
            delete(handler::physical_delete_post),
        )
        .route(
            "/api/v1/hrm/user_org_posts",
            post(handler::create_relation).get(handler::page_relations),
        )
        .route(
            "/api/v1/hrm/user_org_posts/{id}",
            get(handler::get_relation)
                .put(handler::update_relation)
                .delete(handler::delete_relation),
        )
        .route(
            "/api/v1/hrm/user_org_posts/{id}/physical",
            delete(handler::physical_delete_relation),
        )
}
