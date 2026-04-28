//! traceId 中间件。
//!
//! 统一处理 `X-Trace-Id` 的透传、校验、生成和响应头回写，保证响应体、响应头和日志可以
//! 用同一个 traceId 关联同一次请求。

use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use http::{HeaderName, HeaderValue, header::USER_AGENT};

use crate::{context::request_context::RequestContext, utils::id::normalize_trace_id};

pub const TRACE_ID_HEADER: &str = "x-trace-id";
/// 文档约定的规范响应头名称，用于对外说明；实际插入响应头时使用小写静态名称保证兼容。
pub const TRACE_ID_HEADER_CANONICAL: &str = "X-Trace-Id";

/// 解析、校验并回写 traceId。
///
/// 非法 `X-Trace-Id` 会被丢弃并重新生成，防止日志注入、超长头部和不可见字符污染链路日志。
pub async fn trace_id_middleware(mut request: Request<Body>, next: Next) -> Response {
    let trace_id = normalize_trace_id(
        request
            .headers()
            .get(TRACE_ID_HEADER)
            .and_then(|value| value.to_str().ok()),
    );
    let client_ip = extract_client_ip(&request);
    let user_agent = request
        .headers()
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);

    let context =
        RequestContext::anonymous(trace_id.clone()).with_client_info(client_ip, user_agent);
    request.extensions_mut().insert(context);

    let mut response = next.run(request).await;
    if let Ok(value) = HeaderValue::from_str(&trace_id) {
        // 响应头必须与响应体 traceId 一致，便于调用方在不解析 body 时也能拿到链路标识。
        response
            .headers_mut()
            .insert(HeaderName::from_static(TRACE_ID_HEADER), value);
    }
    response
}

fn extract_client_ip(request: &Request<Body>) -> Option<String> {
    // 优先读取网关标准透传头；多级代理场景只取最左侧原始客户端 IP。
    request
        .headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            request
                .headers()
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .map(ToOwned::to_owned)
        })
}
