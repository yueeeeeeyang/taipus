//! 认证模块 HTTP handler。
//!
//! handler 只负责参数提取和统一响应转换，认证业务规则全部下沉到 service。

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
    modules::auth::{dto::*, model::AccountStatus, service::AuthService},
    response::api_response::ApiResponse,
};

pub async fn login(
    State(state): State<AppState>,
    ctx: RequestContext,
    payload: Result<Json<LoginRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        let payload = json_payload(payload)?;
        AuthService::login(require_database(&state)?, &state.config.auth, &ctx, payload).await
    })
    .await
}

pub async fn refresh(
    State(state): State<AppState>,
    ctx: RequestContext,
    payload: Result<Json<RefreshRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        AuthService::refresh(
            require_database(&state)?,
            &state.config.auth,
            &ctx,
            json_payload(payload)?,
        )
        .await
    })
    .await
}

pub async fn logout(
    State(state): State<AppState>,
    ctx: RequestContext,
    payload: Result<Json<LogoutRequest>, JsonRejection>,
) -> Response {
    handle_empty(&state, &ctx, async {
        AuthService::logout(
            require_database(&state)?,
            &state.config.auth,
            &ctx,
            json_payload(payload)?,
        )
        .await
    })
    .await
}

pub async fn logout_all(State(state): State<AppState>, ctx: RequestContext) -> Response {
    handle_empty(&state, &ctx, async {
        AuthService::logout_all(require_database(&state)?, &ctx).await
    })
    .await
}

pub async fn me(State(state): State<AppState>, ctx: RequestContext) -> Response {
    handle(&state, &ctx, async {
        AuthService::me(require_database(&state)?, &ctx).await
    })
    .await
}

pub async fn my_tenants(State(state): State<AppState>, ctx: RequestContext) -> Response {
    handle(&state, &ctx, async {
        AuthService::my_tenants(require_database(&state)?, &ctx).await
    })
    .await
}

pub async fn switch_tenant(
    State(state): State<AppState>,
    ctx: RequestContext,
    payload: Result<Json<SwitchTenantRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        AuthService::switch_tenant(
            require_database(&state)?,
            &state.config.auth,
            &ctx,
            json_payload(payload)?,
        )
        .await
    })
    .await
}

pub async fn create_account(
    State(state): State<AppState>,
    ctx: RequestContext,
    payload: Result<Json<CreateAccountRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        AuthService::create_account(require_database(&state)?, &ctx, json_payload(payload)?).await
    })
    .await
}

pub async fn update_account(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<UpdateAccountRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        AuthService::update_account(require_database(&state)?, &ctx, &id, json_payload(payload)?)
            .await
    })
    .await
}

pub async fn get_account(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Response {
    handle(&state, &ctx, async {
        AuthService::get_account(require_database(&state)?, &id).await
    })
    .await
}

pub async fn page_accounts(
    State(state): State<AppState>,
    ctx: RequestContext,
    query: Result<Query<AccountPageQuery>, QueryRejection>,
) -> Response {
    handle(&state, &ctx, async {
        AuthService::page_accounts(require_database(&state)?, query_payload(query)?).await
    })
    .await
}

pub async fn delete_account(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    query: Result<Query<VersionQuery>, QueryRejection>,
) -> Response {
    handle_empty(&state, &ctx, async {
        AuthService::delete_account(
            require_database(&state)?,
            &ctx,
            &id,
            query_payload(query)?.version,
        )
        .await
    })
    .await
}

pub async fn physical_delete_account(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Response {
    handle_empty(&state, &ctx, async {
        AuthService::physical_delete_account(require_database(&state)?, &id).await
    })
    .await
}

pub async fn enable_account(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<AccountStatusRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        AuthService::set_account_status(
            require_database(&state)?,
            &ctx,
            &id,
            json_payload(payload)?.version,
            AccountStatus::Enabled,
        )
        .await
    })
    .await
}

pub async fn disable_account(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<AccountStatusRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        AuthService::set_account_status(
            require_database(&state)?,
            &ctx,
            &id,
            json_payload(payload)?.version,
            AccountStatus::Disabled,
        )
        .await
    })
    .await
}

pub async fn lock_account(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<AccountStatusRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        AuthService::set_account_status(
            require_database(&state)?,
            &ctx,
            &id,
            json_payload(payload)?.version,
            AccountStatus::Locked,
        )
        .await
    })
    .await
}

pub async fn reset_password(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<ResetPasswordRequest>, JsonRejection>,
) -> Response {
    handle_empty(&state, &ctx, async {
        AuthService::reset_password(require_database(&state)?, &ctx, &id, json_payload(payload)?)
            .await
    })
    .await
}

pub async fn create_account_tenant(
    State(state): State<AppState>,
    ctx: RequestContext,
    payload: Result<Json<CreateAccountTenantRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        AuthService::create_account_tenant(require_database(&state)?, &ctx, json_payload(payload)?)
            .await
    })
    .await
}

pub async fn update_account_tenant(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<UpdateAccountTenantRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        AuthService::update_account_tenant(
            require_database(&state)?,
            &ctx,
            &id,
            json_payload(payload)?,
        )
        .await
    })
    .await
}

pub async fn page_account_tenants(
    State(state): State<AppState>,
    ctx: RequestContext,
    query: Result<Query<AccountTenantPageQuery>, QueryRejection>,
) -> Response {
    handle(&state, &ctx, async {
        AuthService::page_account_tenants(require_database(&state)?, query_payload(query)?).await
    })
    .await
}

pub async fn delete_account_tenant(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    query: Result<Query<VersionQuery>, QueryRejection>,
) -> Response {
    handle_empty(&state, &ctx, async {
        AuthService::delete_account_tenant(
            require_database(&state)?,
            &ctx,
            &id,
            query_payload(query)?.version,
        )
        .await
    })
    .await
}

pub async fn page_sessions(
    State(state): State<AppState>,
    ctx: RequestContext,
    query: Result<Query<SessionPageQuery>, QueryRejection>,
) -> Response {
    handle(&state, &ctx, async {
        AuthService::page_sessions(require_database(&state)?, query_payload(query)?).await
    })
    .await
}

pub async fn revoke_session(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<RevokeSessionRequest>, JsonRejection>,
) -> Response {
    handle_empty(&state, &ctx, async {
        AuthService::revoke_session(require_database(&state)?, &ctx, &id, json_payload(payload)?)
            .await
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

fn json_payload<T: DeserializeOwned>(payload: Result<Json<T>, JsonRejection>) -> AppResult<T> {
    payload
        .map(|Json(value)| value)
        .map_err(|err| AppError::param_invalid(format!("认证请求体不合法: {err}")))
}

fn query_payload<T: DeserializeOwned>(payload: Result<Query<T>, QueryRejection>) -> AppResult<T> {
    payload
        .map(|Query(value)| value)
        .map_err(|err| AppError::param_invalid(format!("认证查询参数不合法: {err}")))
}

fn require_database(state: &AppState) -> AppResult<&DatabasePool> {
    state
        .database
        .as_ref()
        .ok_or_else(|| AppError::system("数据库连接池未初始化"))
}
