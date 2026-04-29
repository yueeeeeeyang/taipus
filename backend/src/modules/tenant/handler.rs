//! 租户 HTTP handler。
//!
//! handler 负责 Axum 参数提取、反序列化错误转换和统一响应构造，租户业务规则下沉到 service。

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
    modules::tenant::{
        dto::{
            CreateTenantRequest, TenantPageQuery, TenantStatusRequest, UpdateTenantRequest,
            VersionQuery,
        },
        service::TenantService,
    },
    response::api_response::ApiResponse,
};

pub async fn create_tenant(
    State(state): State<AppState>,
    ctx: RequestContext,
    payload: Result<Json<CreateTenantRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        let payload = json_payload(payload)?;
        TenantService::create(require_database(&state)?, &ctx, payload).await
    })
    .await
}

pub async fn update_tenant(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<UpdateTenantRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        let payload = json_payload(payload)?;
        TenantService::update(require_database(&state)?, &ctx, &id, payload).await
    })
    .await
}

pub async fn get_tenant(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Response {
    handle(&state, &ctx, async {
        TenantService::get(require_database(&state)?, &id).await
    })
    .await
}

pub async fn page_tenants(
    State(state): State<AppState>,
    ctx: RequestContext,
    query: Result<Query<TenantPageQuery>, QueryRejection>,
) -> Response {
    handle(&state, &ctx, async {
        let query = query_payload(query)?;
        TenantService::page(require_database(&state)?, query).await
    })
    .await
}

pub async fn delete_tenant(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    query: Result<Query<VersionQuery>, QueryRejection>,
) -> Response {
    handle_empty(&state, &ctx, async {
        let query = query_payload(query)?;
        TenantService::logical_delete(require_database(&state)?, &ctx, &id, query.version).await
    })
    .await
}

pub async fn physical_delete_tenant(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Response {
    handle_empty(&state, &ctx, async {
        TenantService::physical_delete(require_database(&state)?, &id).await
    })
    .await
}

pub async fn enable_tenant(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<TenantStatusRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        let payload = json_payload(payload)?;
        TenantService::enable(require_database(&state)?, &ctx, &id, payload.version).await
    })
    .await
}

pub async fn disable_tenant(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<TenantStatusRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        let payload = json_payload(payload)?;
        TenantService::disable(require_database(&state)?, &ctx, &id, payload.version).await
    })
    .await
}

pub async fn suspend_tenant(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<TenantStatusRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        let payload = json_payload(payload)?;
        TenantService::suspend(require_database(&state)?, &ctx, &id, payload.version).await
    })
    .await
}

async fn handle<T, F>(state: &AppState, ctx: &RequestContext, result: F) -> Response
where
    T: serde::Serialize,
    F: std::future::Future<Output = AppResult<T>>,
{
    match result.await {
        Ok(data) => ApiResponse::success(data, ctx).into_response(),
        Err(err) => err.into_response_with_context(ctx, &state.i18n),
    }
}

async fn handle_empty<F>(state: &AppState, ctx: &RequestContext, result: F) -> Response
where
    F: std::future::Future<Output = AppResult<()>>,
{
    match result.await {
        Ok(()) => ApiResponse::empty(ctx).into_response(),
        Err(err) => err.into_response_with_context(ctx, &state.i18n),
    }
}

fn json_payload<T>(payload: Result<Json<T>, JsonRejection>) -> AppResult<T>
where
    T: DeserializeOwned,
{
    payload
        .map(|Json(value)| value)
        .map_err(|err| AppError::param_invalid(format!("租户请求体不合法: {err}")))
}

fn query_payload<T>(payload: Result<Query<T>, QueryRejection>) -> AppResult<T>
where
    T: DeserializeOwned,
{
    payload
        .map(|Query(value)| value)
        .map_err(|err| AppError::param_invalid(format!("租户查询参数不合法: {err}")))
}

fn require_database(state: &AppState) -> AppResult<&DatabasePool> {
    state
        .database
        .as_ref()
        .ok_or_else(|| AppError::system("数据库连接池未初始化"))
}
