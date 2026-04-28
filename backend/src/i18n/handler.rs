//! 多语言接口处理器。
//!
//! 系统资源接口只负责解析查询参数和调用 `I18nService`，实际语言协商结果从 `RequestContext`
//! 获取，保证响应体资源和 `Content-Language` 响应头一致。

use axum::{
    extract::{Query, State},
    response::Response,
};
use http::StatusCode;
use serde::Deserialize;

use crate::{
    AppState, context::request_context::RequestContext, response::api_response::ApiResponse,
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
        Ok(data) => ApiResponse::success(data, ctx.trace_id).with_status(StatusCode::OK),
        Err(err) => err.into_response_with_context(&ctx, &state.i18n),
    }
}
