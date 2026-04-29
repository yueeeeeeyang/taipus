//! 应用错误模型。
//!
//! `AppError` 同时保存调用方可见消息和内部日志消息。响应中只暴露安全消息，内部细节交给
//! tracing 记录，避免 SQL、连接串、密钥或堆栈信息泄漏到前端。

use axum::response::{IntoResponse, Response};
use serde_json::Value;
use thiserror::Error;
use tracing::error;

use crate::{
    context::request_context::RequestContext, error::error_code::ErrorCode,
    i18n::service::I18nService, response::api_response::ApiResponse,
};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct AppError {
    /// 对外稳定数字业务码。
    pub code: ErrorCode,
    /// 对调用方可见的安全错误消息。
    pub message: String,
    /// 系统多语言资源 key，用于按请求 locale 渲染错误消息。
    pub message_key: String,
    /// 内部错误详情，只能写入日志，禁止进入接口响应。
    pub internal_message: Option<String>,
    /// 是否需要告警，系统错误默认需要关注。
    pub alert: bool,
}

impl AppError {
    /// 构造应用错误，默认不携带内部详情。
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            message_key: code.message_key().to_string(),
            internal_message: None,
            alert: matches!(code, ErrorCode::SystemError),
        }
    }

    /// 附加内部错误详情。
    ///
    /// 该信息通常来自数据库、迁移或外部依赖，只能用于日志和告警。
    pub fn with_internal_message(mut self, message: impl Into<String>) -> Self {
        self.internal_message = Some(message.into());
        self
    }

    /// 构造参数错误。
    ///
    /// 参数错误应在 handler 或 validation 层显式返回，不能依赖数据库约束兜底。
    pub fn param_invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ParamInvalid, message)
    }

    /// 构造系统错误。
    ///
    /// 对外响应统一使用安全消息，真实原因写入 `internal_message`。
    pub fn system(message: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::SystemError,
            ErrorCode::SystemError.default_message(),
        )
        .with_internal_message(message)
    }

    /// 转换为统一错误响应体。
    ///
    /// 调用方必须传入当前请求的 traceId，保证响应和日志可以关联。
    pub fn to_api_response(
        &self,
        meta: impl crate::response::api_response::IntoApiResponseMeta,
    ) -> ApiResponse<Value> {
        ApiResponse::error(self.code, self.message.clone(), meta)
    }

    /// 转换为本地化错误响应体。
    ///
    /// message 渲染使用请求最终 locale；如果资源缺失，由 `I18nService` 记录缺失并返回 key。
    pub fn to_localized_api_response(
        &self,
        meta: impl crate::response::api_response::IntoApiResponseMeta,
        locale: &str,
        i18n: &I18nService,
    ) -> ApiResponse<Value> {
        let message = i18n.system_text(&self.message_key, locale);
        ApiResponse::error(self.code, message, meta)
    }

    /// 使用当前请求 traceId 转换为 Axum 响应。
    ///
    /// 普通业务 API 错误响应仍使用 HTTP 200，业务状态由负数 `code` 表达。
    pub fn into_response_with_trace(self, trace_id: impl Into<String>) -> Response {
        let trace_id = trace_id.into();
        if let Some(internal_message) = &self.internal_message {
            error!(
                code = self.code.as_i32(),
                %trace_id,
                internal_message,
                alert = self.alert,
                "应用错误"
            );
        }
        self.to_api_response(trace_id).into_response()
    }

    /// 使用请求上下文转换为 Axum 响应。
    ///
    /// handler 应优先使用该入口，避免错误响应脱离中间件生成的 traceId。
    pub fn into_response_with_context(self, ctx: &RequestContext, i18n: &I18nService) -> Response {
        if let Some(internal_message) = &self.internal_message {
            error!(
                code = self.code.as_i32(),
                trace_id = %ctx.trace_id,
                locale = %ctx.locale,
                time_zone = %ctx.time_zone,
                internal_message,
                alert = self.alert,
                "应用错误"
            );
        }
        self.to_localized_api_response(ctx, &ctx.locale, i18n)
            .into_response()
    }
}

impl From<sqlx::Error> for AppError {
    /// SQLx 错误统一转为系统错误，避免数据库细节泄露给调用方。
    fn from(err: sqlx::Error) -> Self {
        AppError::system(format!("数据库执行失败: {err}"))
    }
}

impl From<refinery::Error> for AppError {
    /// Refinery 迁移错误发生在启动期，必须保留内部详情以便定位 migration 问题。
    fn from(err: refinery::Error) -> Self {
        AppError::system(format!("数据库迁移执行失败: {err}"))
    }
}

impl From<tokio::task::JoinError> for AppError {
    /// 阻塞任务或异步任务失败统一转为系统错误。
    fn from(err: tokio::task::JoinError) -> Self {
        AppError::system(format!("异步任务执行失败: {err}"))
    }
}
