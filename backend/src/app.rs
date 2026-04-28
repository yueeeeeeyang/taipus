//! Axum 应用装配模块。
//!
//! 这里集中挂载全局中间件和基础路由。业务模块后续只需要提供 `Router<AppState>`，
//! 不应在各模块重复实现 traceId、访问日志或统一错误响应等横切能力。

use std::sync::Arc;

use axum::{Router, middleware as axum_middleware, response::IntoResponse};
use http::StatusCode;

use crate::{
    config::settings::AppConfig,
    context::request_context::RequestContext,
    db::executor::DatabasePool,
    error::error_code::ErrorCode,
    health, i18n,
    i18n::service::I18nService,
    middleware::{
        access_log::access_log_middleware, locale::locale_middleware, trace_id::trace_id_middleware,
    },
    response::api_response::ApiResponse,
};

/// 应用共享状态。
///
/// `database` 使用 `Option` 是为了让集成测试可以构造无数据库实例并验证 `/health/ready`
/// 的失败语义；生产启动路径必须在创建状态前完成数据库连接和迁移。
#[derive(Clone)]
pub struct AppState {
    /// 应用配置使用 `Arc` 共享，避免每个请求复制配置内容。
    pub config: Arc<AppConfig>,
    /// 数据库连接池。生产必须存在，测试允许为空以验证 ready 失败语义。
    pub database: Option<DatabasePool>,
    /// 多语言服务。系统文本、错误消息和系统资源接口都通过该服务渲染。
    pub i18n: Arc<I18nService>,
}

impl AppState {
    /// 创建应用共享状态。
    ///
    /// 该函数不执行任何 I/O，启动流程必须在调用前完成数据库连接和 migration。
    pub fn new(config: AppConfig, database: Option<DatabasePool>) -> Self {
        let i18n = Arc::new(I18nService::new(&config.i18n));
        Self {
            config: Arc::new(config),
            database,
            i18n,
        }
    }
}

pub fn build_router(state: AppState) -> Router {
    let locale_state = state.clone();
    // traceId 中间件必须位于外层，确保访问日志、handler 和 fallback 都能读取同一个上下文。
    Router::new()
        .merge(health::route::routes())
        .merge(i18n::route::routes())
        .fallback(not_found)
        .with_state(state.clone())
        .layer(axum_middleware::from_fn(access_log_middleware))
        .layer(axum_middleware::from_fn_with_state(
            locale_state,
            locale_middleware,
        ))
        .layer(axum_middleware::from_fn(trace_id_middleware))
}

async fn not_found(
    axum::extract::State(state): axum::extract::State<AppState>,
    ctx: RequestContext,
) -> impl IntoResponse {
    // 未匹配路由属于业务 API 标准响应链路，因此 HTTP 仍返回 200，业务码返回 -404。
    let message = state
        .i18n
        .system_text(ErrorCode::ResourceNotFound.message_key(), &ctx.locale);
    ApiResponse::error(ErrorCode::ResourceNotFound, message, ctx.trace_id)
        .with_status(StatusCode::OK)
}
