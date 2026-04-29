//! 多语言接口处理器。
//!
//! 接口处理器只负责解析 path/query/json 和调用 `I18nService`，实际语言、时区和 traceId 都从
//! `RequestContext` 获取，保证响应体、响应头和日志使用同一请求上下文。

use axum::{
    Json,
    extract::{Path, Query, State, rejection::JsonRejection},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::{
    AppState,
    context::request_context::RequestContext,
    error::app_error::{AppError, AppResult},
    i18n::business_translation::BusinessTranslationWriteCommand,
    response::api_response::ApiResponse,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemResourceQuery {
    /// 请求 locale 已由中间件统一解析，这里保留字段用于接口契约和 query 反序列化。
    pub locale: Option<String>,
    /// 消费端平台，例如 `frontend`、`mobile` 或 `backend`。
    pub platform: Option<String>,
    /// 逗号分隔的 namespace 列表，例如 `common,menu`。
    pub namespaces: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessTranslationQuery {
    /// 逗号分隔的业务字段名列表，例如 `name,description`。
    pub fields: Option<String>,
}

pub async fn system_resources(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(query): Query<SystemResourceQuery>,
) -> Response {
    let result = state.i18n.system_resources(
        &ctx.locale,
        &ctx.time_zone,
        query.platform.as_deref(),
        query.namespaces.as_deref(),
    );

    match result {
        Ok(data) => ApiResponse::success(data, &ctx).into_response(),
        Err(err) => err.into_response_with_context(&ctx, &state.i18n),
    }
}

pub async fn get_business_translations(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path((resource_type, resource_id)): Path<(String, String)>,
    Query(query): Query<BusinessTranslationQuery>,
) -> Response {
    let result = async {
        state
            .i18n
            .validate_business_translation_fields(&resource_type, query.fields.as_deref())?;
        let database = require_database(&state)?;
        state
            .i18n
            .get_business_translations(
                database,
                &resource_type,
                &resource_id,
                query.fields.as_deref(),
                &ctx,
            )
            .await
    }
    .await;

    match result {
        Ok(data) => ApiResponse::success(data, &ctx).into_response(),
        Err(err) => err.into_response_with_context(&ctx, &state.i18n),
    }
}

pub async fn put_business_translations(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path((resource_type, resource_id)): Path<(String, String)>,
    payload: Result<Json<BusinessTranslationWriteCommand>, JsonRejection>,
) -> Response {
    let result = async {
        let Json(command) = payload
            .map_err(|err| AppError::param_invalid(format!("业务翻译请求体不合法: {err}")))?;
        let command = state
            .i18n
            .validate_business_translation_write_command(&resource_type, command)?;
        let database = require_database(&state)?;
        state
            .i18n
            .set_business_translations(database, &resource_type, &resource_id, command, &ctx)
            .await
    }
    .await;

    match result {
        Ok(data) => ApiResponse::success(data, &ctx).into_response(),
        Err(err) => err.into_response_with_context(&ctx, &state.i18n),
    }
}

fn require_database(state: &AppState) -> AppResult<&crate::db::executor::DatabasePool> {
    state
        .database
        .as_ref()
        .ok_or_else(|| AppError::system("数据库连接池未初始化"))
}
