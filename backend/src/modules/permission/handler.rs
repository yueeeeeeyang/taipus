//! 权限模块 HTTP handler。
//!
//! handler 只负责参数提取、调用 service 和统一响应转换，不承载权限业务规则。

use axum::{
    Json,
    extract::{
        Path, Query, State,
        rejection::{JsonRejection, QueryRejection},
    },
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;

use crate::{
    AppState,
    context::request_context::RequestContext,
    db::executor::DatabasePool,
    error::app_error::{AppError, AppResult},
    modules::permission::{dto::*, service::PermissionService},
    response::api_response::ApiResponse,
};

pub async fn create_application(
    State(state): State<AppState>,
    ctx: RequestContext,
    payload: Result<Json<CreateApplicationRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::create_application(
            require_database(&state)?,
            &ctx,
            json_payload(payload)?,
        )
        .await
    })
    .await
}

pub async fn page_applications(
    State(state): State<AppState>,
    ctx: RequestContext,
    query: Result<Query<ResourcePageQuery>, QueryRejection>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::page_applications(require_database(&state)?, &ctx, query_payload(query)?)
            .await
    })
    .await
}

pub async fn get_application(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::get_application(require_database(&state)?, &id).await
    })
    .await
}

pub async fn create_menu(
    State(state): State<AppState>,
    ctx: RequestContext,
    payload: Result<Json<CreateMenuRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::create_menu(require_database(&state)?, &ctx, json_payload(payload)?)
            .await
    })
    .await
}

pub async fn menu_tree(State(state): State<AppState>, ctx: RequestContext) -> Response {
    handle(&state, &ctx, async {
        PermissionService::menu_tree(require_database(&state)?, &ctx).await
    })
    .await
}

pub async fn create_button(
    State(state): State<AppState>,
    ctx: RequestContext,
    payload: Result<Json<CreateButtonRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::create_button(require_database(&state)?, &ctx, json_payload(payload)?)
            .await
    })
    .await
}

pub async fn page_buttons(
    State(state): State<AppState>,
    ctx: RequestContext,
    query: Result<Query<MeResourceQuery>, QueryRejection>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::page_buttons(require_database(&state)?, &ctx, query_payload(query)?)
            .await
    })
    .await
}

pub async fn me_menus(
    State(state): State<AppState>,
    ctx: RequestContext,
    query: Result<Query<MeResourceQuery>, QueryRejection>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::me_menus(require_database(&state)?, &ctx, query_payload(query)?).await
    })
    .await
}

pub async fn me_buttons(
    State(state): State<AppState>,
    ctx: RequestContext,
    query: Result<Query<MeResourceQuery>, QueryRejection>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::me_buttons(require_database(&state)?, &ctx, query_payload(query)?).await
    })
    .await
}

pub async fn create_api(
    State(state): State<AppState>,
    ctx: RequestContext,
    payload: Result<Json<CreateApiRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::create_api(require_database(&state)?, &ctx, json_payload(payload)?).await
    })
    .await
}

pub async fn page_apis(
    State(state): State<AppState>,
    ctx: RequestContext,
    query: Result<Query<ResourcePageQuery>, QueryRejection>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::page_apis(require_database(&state)?, &ctx, query_payload(query)?).await
    })
    .await
}

pub async fn import_apis(State(state): State<AppState>, ctx: RequestContext) -> Response {
    handle_empty(&state, &ctx, async {
        PermissionService::import_apis(require_database(&state)?, &ctx).await
    })
    .await
}

pub async fn create_role(
    State(state): State<AppState>,
    ctx: RequestContext,
    payload: Result<Json<CreateRoleRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::create_role(require_database(&state)?, &ctx, json_payload(payload)?)
            .await
    })
    .await
}

pub async fn page_roles(
    State(state): State<AppState>,
    ctx: RequestContext,
    query: Result<Query<RolePageQuery>, QueryRejection>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::page_roles(require_database(&state)?, &ctx, query_payload(query)?).await
    })
    .await
}

pub async fn role_tree(State(state): State<AppState>, ctx: RequestContext) -> Response {
    handle(&state, &ctx, async {
        PermissionService::role_tree(require_database(&state)?, &ctx).await
    })
    .await
}

pub async fn get_role(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::get_role(require_database(&state)?, &id).await
    })
    .await
}

pub async fn role_permissions(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::role_permissions(require_database(&state)?, &ctx, &id).await
    })
    .await
}

pub async fn inherited_role_permissions(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::inherited_role_permissions(require_database(&state)?, &ctx, &id).await
    })
    .await
}

pub async fn set_role_permissions(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<SetRolePermissionsRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::set_role_permissions(
            require_database(&state)?,
            &ctx,
            &id,
            json_payload(payload)?,
        )
        .await
    })
    .await
}

pub async fn set_role_parents(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<SetRoleParentsRequest>, JsonRejection>,
) -> Response {
    handle_empty(&state, &ctx, async {
        PermissionService::set_role_parents(
            require_database(&state)?,
            &ctx,
            &id,
            json_payload(payload)?,
        )
        .await
    })
    .await
}

pub async fn resource_grants(
    State(state): State<AppState>,
    ctx: RequestContext,
    query: Result<Query<ResourceGrantsQuery>, QueryRejection>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::resource_grants(require_database(&state)?, &ctx, query_payload(query)?)
            .await
    })
    .await
}

pub async fn set_resource_grants(
    State(state): State<AppState>,
    ctx: RequestContext,
    payload: Result<Json<SetResourceGrantsRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::set_resource_grants(
            require_database(&state)?,
            &ctx,
            json_payload(payload)?,
        )
        .await
    })
    .await
}

pub async fn account_roles(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::account_roles(require_database(&state)?, &ctx, &id).await
    })
    .await
}

pub async fn set_account_roles(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<SetAccountRolesRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        PermissionService::set_account_roles(
            require_database(&state)?,
            &ctx,
            &id,
            json_payload(payload)?,
        )
        .await
    })
    .await
}

pub async fn me_permissions(State(state): State<AppState>, ctx: RequestContext) -> Response {
    handle(&state, &ctx, async {
        PermissionService::me_permissions(require_database(&state)?, &ctx).await
    })
    .await
}

pub async fn permission_version(State(state): State<AppState>, ctx: RequestContext) -> Response {
    handle(&state, &ctx, async {
        PermissionService::permission_version(require_database(&state)?, &ctx).await
    })
    .await
}

async fn handle<T, Fut>(state: &AppState, ctx: &RequestContext, future: Fut) -> Response
where
    T: serde::Serialize,
    Fut: std::future::Future<Output = AppResult<T>>,
{
    match future.await {
        Ok(data) => ApiResponse::success(data, ctx).into_response(),
        Err(err) => err
            .into_response_with_context(ctx, &state.i18n)
            .into_response(),
    }
}

async fn handle_empty<Fut>(state: &AppState, ctx: &RequestContext, future: Fut) -> Response
where
    Fut: std::future::Future<Output = AppResult<()>>,
{
    match future.await {
        Ok(()) => ApiResponse::success(serde_json::json!({}), ctx).into_response(),
        Err(err) => err
            .into_response_with_context(ctx, &state.i18n)
            .into_response(),
    }
}

fn require_database(state: &AppState) -> AppResult<&DatabasePool> {
    state
        .database
        .as_ref()
        .ok_or_else(|| AppError::system("数据库未初始化"))
}

fn json_payload<T>(payload: Result<Json<T>, JsonRejection>) -> AppResult<T>
where
    T: DeserializeOwned,
{
    payload
        .map(|Json(value)| value)
        .map_err(|err| AppError::param_invalid(format!("权限请求体不合法: {err}")))
}

fn query_payload<T>(payload: Result<Query<T>, QueryRejection>) -> AppResult<T>
where
    T: DeserializeOwned,
{
    payload
        .map(|Query(value)| value)
        .map_err(|err| AppError::param_invalid(format!("权限查询参数不合法: {err}")))
}
