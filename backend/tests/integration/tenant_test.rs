//! 租户中间件和接口契约集成测试。
//!
//! 这些测试不依赖真实数据库，固定首版 `X-Tenant-Id` 和默认租户写入 RequestContext 的行为。

use axum::{
    Json, Router, body::Body, http::Request, middleware as axum_middleware, response::IntoResponse,
    routing::get,
};
use serde_json::{Value, json};
use taipus_backend::{
    AppState,
    context::request_context::RequestContext,
    middleware::{tenant::tenant_middleware, trace_id::trace_id_middleware},
    tests::fixture::app_state_without_database,
};
use tower::ServiceExt;

async fn tenant_probe(ctx: RequestContext) -> impl IntoResponse {
    Json(json!({
        "tenantId": ctx.tenant_id,
        "tenantSource": ctx.tenant_source
    }))
}

async fn read_json(response: axum::response::Response) -> Value {
    // 租户中间件测试只关心响应体中的上下文字段。
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("响应体必须可读取");
    serde_json::from_slice(&bytes).expect("响应体必须是 JSON")
}

fn tenant_probe_app(state: AppState) -> Router {
    let tenant_state = state.clone();
    Router::new()
        .route("/probe", get(tenant_probe))
        .with_state(state)
        .layer(axum_middleware::from_fn_with_state(
            tenant_state,
            tenant_middleware,
        ))
        .layer(axum_middleware::from_fn(trace_id_middleware))
}

#[tokio::test]
async fn tenant_middleware_uses_default_tenant_without_header() {
    // 无租户请求头时，首版应写入配置默认租户，保证单租户和开发环境可用。
    let app = tenant_probe_app(app_state_without_database());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/probe")
                .body(Body::empty())
                .expect("测试请求必须可构造"),
        )
        .await
        .expect("租户探针请求必须可执行");

    let body = read_json(response).await;
    assert_eq!(body["tenantId"], "default");
    assert_eq!(body["tenantSource"], "default");
}

#[tokio::test]
async fn tenant_middleware_uses_header_tenant_when_allowed() {
    // 允许 header 覆盖时，X-Tenant-Id 必须成为最终租户上下文。
    let app = tenant_probe_app(app_state_without_database());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/probe")
                .header("X-Tenant-Id", "tenant_a")
                .body(Body::empty())
                .expect("测试请求必须可构造"),
        )
        .await
        .expect("租户探针请求必须可执行");

    let body = read_json(response).await;
    assert_eq!(body["tenantId"], "tenant_a");
    assert_eq!(body["tenantSource"], "header");
}

#[tokio::test]
async fn tenant_middleware_rejects_header_when_disabled_by_config() {
    // 当部署配置禁止 header 覆盖时，即使请求传入 X-Tenant-Id 也必须返回权限错误。
    let mut state = app_state_without_database();
    let config = std::sync::Arc::make_mut(&mut state.config);
    config.tenant.allow_header_override = false;

    let app = tenant_probe_app(state);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/probe")
                .header("X-Tenant-Id", "tenant_a")
                .body(Body::empty())
                .expect("测试请求必须可构造"),
        )
        .await
        .expect("租户探针请求必须可执行");

    let body = read_json(response).await;
    assert_eq!(body["code"], -403);
}
