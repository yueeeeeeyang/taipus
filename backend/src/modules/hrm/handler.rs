//! HRM HTTP handler。
//!
//! handler 只负责 Axum 参数提取、JSON 反序列化错误转换和统一响应构造。业务规则全部下沉到
//! `HrmService`，避免接口层和服务层出现重复校验。

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
    modules::hrm::{
        dto::{
            CreateOrgRequest, CreatePostRequest, CreateUserOrgPostRequest, CreateUserRequest,
            OrgPageQuery, PostPageQuery, UpdateOrgRequest, UpdatePostRequest,
            UpdateUserOrgPostRequest, UpdateUserRequest, UserOrgPostPageQuery, UserPageQuery,
            VersionQuery,
        },
        service::HrmService,
    },
    response::api_response::ApiResponse,
};

pub async fn create_user(
    State(state): State<AppState>,
    ctx: RequestContext,
    payload: Result<Json<CreateUserRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        HrmService::create_user(require_database(&state)?, &ctx, json_payload(payload)?).await
    })
    .await
}

pub async fn update_user(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<UpdateUserRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        HrmService::update_user(require_database(&state)?, &ctx, &id, json_payload(payload)?).await
    })
    .await
}

pub async fn get_user(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Response {
    handle(&state, &ctx, async {
        HrmService::get_user(require_database(&state)?, &ctx, &id).await
    })
    .await
}

pub async fn page_users(
    State(state): State<AppState>,
    ctx: RequestContext,
    query: Result<Query<UserPageQuery>, QueryRejection>,
) -> Response {
    handle(&state, &ctx, async {
        let query = query_payload(query)?;
        HrmService::page_users(require_database(&state)?, &ctx, query).await
    })
    .await
}

pub async fn delete_user(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    query: Result<Query<VersionQuery>, QueryRejection>,
) -> Response {
    handle_empty(&state, &ctx, async {
        let query = query_payload(query)?;
        HrmService::logical_delete_user(require_database(&state)?, &ctx, &id, query.version).await
    })
    .await
}

pub async fn physical_delete_user(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Response {
    handle_empty(&state, &ctx, async {
        HrmService::physical_delete_user(require_database(&state)?, &ctx, &id).await
    })
    .await
}

pub async fn create_org(
    State(state): State<AppState>,
    ctx: RequestContext,
    payload: Result<Json<CreateOrgRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        HrmService::create_org(require_database(&state)?, &ctx, json_payload(payload)?).await
    })
    .await
}

pub async fn update_org(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<UpdateOrgRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        HrmService::update_org(require_database(&state)?, &ctx, &id, json_payload(payload)?).await
    })
    .await
}

pub async fn get_org(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Response {
    handle(&state, &ctx, async {
        HrmService::get_org(require_database(&state)?, &ctx, &id).await
    })
    .await
}

pub async fn page_orgs(
    State(state): State<AppState>,
    ctx: RequestContext,
    query: Result<Query<OrgPageQuery>, QueryRejection>,
) -> Response {
    handle(&state, &ctx, async {
        let query = query_payload(query)?;
        HrmService::page_orgs(require_database(&state)?, &ctx, query).await
    })
    .await
}

pub async fn org_tree(State(state): State<AppState>, ctx: RequestContext) -> Response {
    handle(&state, &ctx, async {
        HrmService::org_tree(require_database(&state)?, &ctx).await
    })
    .await
}

pub async fn delete_org(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    query: Result<Query<VersionQuery>, QueryRejection>,
) -> Response {
    handle_empty(&state, &ctx, async {
        let query = query_payload(query)?;
        HrmService::logical_delete_org(require_database(&state)?, &ctx, &id, query.version).await
    })
    .await
}

pub async fn physical_delete_org(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Response {
    handle_empty(&state, &ctx, async {
        HrmService::physical_delete_org(require_database(&state)?, &ctx, &id).await
    })
    .await
}

pub async fn create_post(
    State(state): State<AppState>,
    ctx: RequestContext,
    payload: Result<Json<CreatePostRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        HrmService::create_post(require_database(&state)?, &ctx, json_payload(payload)?).await
    })
    .await
}

pub async fn update_post(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<UpdatePostRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        HrmService::update_post(require_database(&state)?, &ctx, &id, json_payload(payload)?).await
    })
    .await
}

pub async fn get_post(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Response {
    handle(&state, &ctx, async {
        HrmService::get_post(require_database(&state)?, &ctx, &id).await
    })
    .await
}

pub async fn page_posts(
    State(state): State<AppState>,
    ctx: RequestContext,
    query: Result<Query<PostPageQuery>, QueryRejection>,
) -> Response {
    handle(&state, &ctx, async {
        let query = query_payload(query)?;
        HrmService::page_posts(require_database(&state)?, &ctx, query).await
    })
    .await
}

pub async fn delete_post(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    query: Result<Query<VersionQuery>, QueryRejection>,
) -> Response {
    handle_empty(&state, &ctx, async {
        let query = query_payload(query)?;
        HrmService::logical_delete_post(require_database(&state)?, &ctx, &id, query.version).await
    })
    .await
}

pub async fn physical_delete_post(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Response {
    handle_empty(&state, &ctx, async {
        HrmService::physical_delete_post(require_database(&state)?, &ctx, &id).await
    })
    .await
}

pub async fn create_relation(
    State(state): State<AppState>,
    ctx: RequestContext,
    payload: Result<Json<CreateUserOrgPostRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        HrmService::create_relation(require_database(&state)?, &ctx, json_payload(payload)?).await
    })
    .await
}

pub async fn update_relation(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    payload: Result<Json<UpdateUserOrgPostRequest>, JsonRejection>,
) -> Response {
    handle(&state, &ctx, async {
        HrmService::update_relation(require_database(&state)?, &ctx, &id, json_payload(payload)?)
            .await
    })
    .await
}

pub async fn get_relation(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Response {
    handle(&state, &ctx, async {
        HrmService::get_relation(require_database(&state)?, &ctx, &id).await
    })
    .await
}

pub async fn page_relations(
    State(state): State<AppState>,
    ctx: RequestContext,
    query: Result<Query<UserOrgPostPageQuery>, QueryRejection>,
) -> Response {
    handle(&state, &ctx, async {
        let query = query_payload(query)?;
        HrmService::page_relations(require_database(&state)?, &ctx, query).await
    })
    .await
}

pub async fn delete_relation(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    query: Result<Query<VersionQuery>, QueryRejection>,
) -> Response {
    handle_empty(&state, &ctx, async {
        let query = query_payload(query)?;
        HrmService::logical_delete_relation(require_database(&state)?, &ctx, &id, query.version)
            .await
    })
    .await
}

pub async fn physical_delete_relation(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Response {
    handle_empty(&state, &ctx, async {
        HrmService::physical_delete_relation(require_database(&state)?, &ctx, &id).await
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
        .map_err(|err| AppError::param_invalid(format!("HRM 请求体不合法: {err}")))
}

fn query_payload<T>(payload: Result<Query<T>, QueryRejection>) -> AppResult<T>
where
    T: DeserializeOwned,
{
    payload
        .map(|Query(value)| value)
        .map_err(|err| AppError::param_invalid(format!("HRM 查询参数不合法: {err}")))
}

fn require_database(state: &AppState) -> AppResult<&DatabasePool> {
    state
        .database
        .as_ref()
        .ok_or_else(|| AppError::system("数据库连接池未初始化"))
}
