//! 健康检查处理器。
//!
//! 健康检查是业务 API 的例外：探针需要用 HTTP 200/503 判断实例是否可接流量，
//! 但响应体仍保持统一结构，便于排查失败原因并关联 traceId。

use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use http::StatusCode;
use serde_json::json;
use tracing::error;

use crate::{
    AppState, context::request_context::RequestContext, error::error_code::ErrorCode,
    response::api_response::ApiResponse,
};

pub async fn live(ctx: RequestContext) -> Response {
    // live 只证明进程可响应，不触碰数据库，避免依赖抖动导致容器被错误重启。
    ApiResponse::success(
        json!({
            "status": "UP"
        }),
        &ctx,
    )
    .into_response()
}

pub async fn ready(State(state): State<AppState>, ctx: RequestContext) -> Response {
    match &state.database {
        Some(database) => match database.ping().await {
            // ready 成功说明实例可以接收业务流量。
            Ok(()) => ApiResponse::success(
                json!({
                    "status": "READY",
                    "databaseType": database.database_type().as_str()
                }),
                &ctx,
            )
            .into_response(),
            Err(err) => {
                let message = state
                    .i18n
                    .system_text("health.database_unavailable", &ctx.locale);
                error!(
                    code = ErrorCode::SystemError.as_i32(),
                    trace_id = %ctx.trace_id,
                    locale = %ctx.locale,
                    time_zone = %ctx.time_zone,
                    database_type = %database.database_type(),
                    error = %err,
                    "就绪检查数据库连接失败"
                );
                // ready 失败必须返回 HTTP 503，满足 Kubernetes、网关和负载均衡探针语义。
                ApiResponse::error(ErrorCode::SystemError, message, &ctx)
                    .with_data(json!({
                        "status": "NOT_READY",
                        "reason": "database_unavailable"
                    }))
                    .with_status(StatusCode::SERVICE_UNAVAILABLE)
            }
        },
        // 测试或异常启动状态下数据库连接池可能缺失，此时实例必须明确不可接流量。
        None => {
            let message = state
                .i18n
                .system_text("health.database_pool_missing", &ctx.locale);
            ApiResponse::error(ErrorCode::SystemError, message, &ctx)
                .with_data(json!({
                    "status": "NOT_READY",
                    "reason": "database_pool_missing"
                }))
                .with_status(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}
