//! 权限模块路由。
//!
//! 管理接口挂载在 `/api/v1/permission` 下，路径使用 snake_case，资源管理按资源类型拆分。

use axum::{
    Router,
    routing::{get, post},
};

use crate::{AppState, modules::permission::handler};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/permission/applications",
            post(handler::create_application).get(handler::page_applications),
        )
        .route(
            "/api/v1/permission/applications/{id}",
            get(handler::get_application),
        )
        .route(
            "/api/v1/permission/menus",
            post(handler::create_menu).get(handler::menu_tree),
        )
        .route("/api/v1/permission/menu_tree", get(handler::menu_tree))
        .route(
            "/api/v1/permission/buttons",
            post(handler::create_button).get(handler::page_buttons),
        )
        .route(
            "/api/v1/permission/apis",
            post(handler::create_api).get(handler::page_apis),
        )
        .route("/api/v1/permission/apis/import", post(handler::import_apis))
        .route(
            "/api/v1/permission/roles",
            post(handler::create_role).get(handler::page_roles),
        )
        .route("/api/v1/permission/role_tree", get(handler::role_tree))
        .route("/api/v1/permission/roles/{id}", get(handler::get_role))
        .route(
            "/api/v1/permission/roles/{id}/parents",
            post(handler::set_role_parents),
        )
        .route(
            "/api/v1/permission/roles/{id}/permissions",
            get(handler::role_permissions).put(handler::set_role_permissions),
        )
        .route(
            "/api/v1/permission/roles/{id}/inherited_permissions",
            get(handler::inherited_role_permissions),
        )
        .route(
            "/api/v1/permission/resource_grants",
            get(handler::resource_grants).put(handler::set_resource_grants),
        )
        .route(
            "/api/v1/permission/accounts/{id}/roles",
            get(handler::account_roles).put(handler::set_account_roles),
        )
        .route(
            "/api/v1/permission/me/resources",
            get(handler::me_permissions),
        )
        .route("/api/v1/permission/me/menus", get(handler::me_menus))
        .route("/api/v1/permission/me/buttons", get(handler::me_buttons))
        .route(
            "/api/v1/permission/me/permissions",
            get(handler::me_permissions),
        )
        .route(
            "/api/v1/permission/version",
            get(handler::permission_version),
        )
}
