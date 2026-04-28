//! 访问日志中间件。
//!
//! 访问日志记录 HTTP 方法、路径、状态码、耗时和 traceId。响应业务码位于响应体中，
//! 当前中间件不读取 body，避免为了日志破坏流式响应或引入额外序列化成本。

use std::time::Instant;

use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use tracing::info;

use crate::context::request_context::RequestContext;

pub async fn access_log_middleware(request: Request<Body>, next: Next) -> Response {
    let started_at = Instant::now();
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let trace_id = request
        .extensions()
        .get::<RequestContext>()
        .map(|ctx| ctx.trace_id.clone())
        .unwrap_or_else(|| "-".to_string());
    let locale = request
        .extensions()
        .get::<RequestContext>()
        .map(|ctx| ctx.locale.clone())
        .unwrap_or_else(|| "und".to_string());

    let response = next.run(request).await;
    let status = response.status().as_u16();
    let elapsed_ms = started_at.elapsed().as_millis();

    // 访问日志不读取响应体，避免为了拿业务码而破坏 body 消费语义或增加序列化成本。
    info!(
        %method,
        %path,
        status,
        elapsed_ms,
        %trace_id,
        %locale,
        "HTTP 请求完成"
    );

    response
}
