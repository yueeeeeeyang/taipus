//! 统一接口响应。
//!
//! 业务 API 的 HTTP 状态码固定为 200，调用方通过 `code` 判断业务状态。健康检查是部署探针
//! 例外，可以使用 200/503，但响应体仍保持同一结构，方便排查。

use axum::{
    Json,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use http::StatusCode;
use serde::Serialize;
use serde_json::Value;

use crate::{
    context::request_context::RequestContext, error::error_code::ErrorCode, utils::time::now_utc,
};

/// 构造统一响应所需的请求元信息。
///
/// handler 传入 `&RequestContext` 时会自动填充 traceId 和后端耗时；测试或后台工具只传 traceId
/// 字符串时，耗时保持 0.0，避免为了无真实请求的场景伪造上下文。
#[derive(Debug, Clone)]
pub struct ApiResponseMeta {
    /// 请求链路标识，必须与响应头中的 `X-Trace-Id` 保持一致。
    pub trace_id: String,
    /// 请求进入后端到响应体构造时的处理耗时，单位毫秒。
    pub elapsed_ms: f64,
}

/// 将请求上下文或裸 traceId 转换为响应元信息。
///
/// 该 trait 只服务 `ApiResponse` 构造函数，目的是让业务 handler 默认传 `&RequestContext`，
/// 不再手动调用 `with_elapsed_ms` 补充耗时。
pub trait IntoApiResponseMeta {
    /// 转换为统一响应元信息。
    fn into_api_response_meta(self) -> ApiResponseMeta;
}

impl IntoApiResponseMeta for &RequestContext {
    /// 使用真实请求上下文填充 traceId 和后端处理耗时。
    fn into_api_response_meta(self) -> ApiResponseMeta {
        ApiResponseMeta {
            trace_id: self.trace_id.clone(),
            elapsed_ms: self.elapsed_ms(),
        }
    }
}

impl IntoApiResponseMeta for String {
    /// 裸 traceId 没有请求开始时间，默认耗时保持 0.0。
    fn into_api_response_meta(self) -> ApiResponseMeta {
        ApiResponseMeta {
            trace_id: self,
            elapsed_ms: 0.0,
        }
    }
}

impl IntoApiResponseMeta for &str {
    /// 字符串字面量主要用于测试，默认耗时保持 0.0。
    fn into_api_response_meta(self) -> ApiResponseMeta {
        self.to_string().into_api_response_meta()
    }
}

impl IntoApiResponseMeta for &String {
    /// 字符串引用主要用于兼容已有调用方，默认耗时保持 0.0。
    fn into_api_response_meta(self) -> ApiResponseMeta {
        self.clone().into_api_response_meta()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    /// 数字业务码。业务成功为正数，错误为负数。
    pub code: i32,
    /// 面向调用方的安全提示信息，不包含 SQL、堆栈或连接串等内部细节。
    pub message: String,
    /// 业务数据。错误响应必须为 `None`，序列化后表现为 JSON `null`。
    pub data: Option<T>,
    /// 请求链路标识，必须与 `X-Trace-Id` 响应头保持一致。
    pub trace_id: String,
    /// 服务端生成响应的 UTC 时间，用于前后端排查时钟和缓存问题。
    pub timestamp: DateTime<Utc>,
    /// 请求进入后端到响应体构造时的处理耗时，单位毫秒，保留三位小数。
    pub elapsed_ms: f64,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    /// 构造业务成功响应。
    ///
    /// 业务 API 的 HTTP 状态码由 `IntoResponse` 固定为 200，调用方通过 `code` 判断成功。
    pub fn success(data: T, meta: impl IntoApiResponseMeta) -> Self {
        let meta = meta.into_api_response_meta();
        Self {
            code: ErrorCode::Success.as_i32(),
            message: ErrorCode::Success.default_message().to_string(),
            data: Some(data),
            trace_id: meta.trace_id,
            timestamp: now_utc(),
            elapsed_ms: meta.elapsed_ms,
        }
    }

    /// 写入请求后端处理耗时。
    ///
    /// handler 应优先把 `RequestContext` 传给构造函数自动填充耗时；该方法只用于少量测试或适配旧调用。
    pub fn with_elapsed_ms(mut self, elapsed_ms: f64) -> Self {
        self.elapsed_ms = elapsed_ms;
        self
    }

    /// 使用指定 HTTP 状态码输出统一响应体。
    ///
    /// 该方法主要服务健康检查等探针接口；普通业务 API 不应随意传入非 200 状态码。
    pub fn with_status(self, status: StatusCode) -> Response {
        (status, Json(self)).into_response()
    }
}

impl ApiResponse<Value> {
    /// 构造无业务数据的成功响应，适合删除、启停等只需要表达执行结果的接口。
    pub fn empty(meta: impl IntoApiResponseMeta) -> Self {
        Self::success(serde_json::json!({}), meta)
    }

    /// 构造错误响应。
    ///
    /// 错误响应的 `data` 固定为 `None`，防止调用方误把错误详情当作业务数据继续处理。
    pub fn error(
        code: ErrorCode,
        message: impl Into<String>,
        meta: impl IntoApiResponseMeta,
    ) -> Self {
        let meta = meta.into_api_response_meta();
        Self {
            code: code.as_i32(),
            message: message.into(),
            data: None,
            trace_id: meta.trace_id,
            timestamp: now_utc(),
            elapsed_ms: meta.elapsed_ms,
        }
    }

    /// 为特殊场景附加诊断数据。
    ///
    /// 当前主要用于健康检查失败时返回 `NOT_READY` 原因；业务错误默认不应附加内部细节。
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}

impl<T> axum::response::IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    /// 普通业务 API 默认使用 HTTP 200，统一由响应体 `code` 表达业务状态。
    fn into_response(self) -> Response {
        self.with_status(StatusCode::OK)
    }
}
