//! 健康检查和 traceId 中间件集成测试。
//!
//! 健康检查需要同时满足部署探针 HTTP 状态码语义和统一响应体契约。

use std::time::Duration;

use axum::{body::Body, http::Request};
use http::StatusCode;
use serde_json::Value;
use sqlx::mysql::MySqlPoolOptions;
use taipus_backend::{
    AppConfig, AppState, build_router, db::executor::DatabasePool,
    tests::fixture::app_state_without_database,
};
use tower::ServiceExt;

async fn read_json(response: axum::response::Response) -> Value {
    // 测试只关心 JSON 契约，因此统一把响应体解析为 `serde_json::Value`。
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("响应体必须可读取");
    serde_json::from_slice(&bytes).expect("响应体必须是 JSON")
}

#[tokio::test]
async fn live_check_returns_unified_success_response() {
    // live 不依赖数据库，进程可响应时必须返回 HTTP 200 和成功业务码。
    let app = build_router(app_state_without_database());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .body(Body::empty())
                .expect("测试请求必须可构造"),
        )
        .await
        .expect("健康检查请求必须可执行");

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("x-trace-id"));

    let body = read_json(response).await;
    assert_eq!(body["code"], 200);
    assert_eq!(body["data"]["status"], "UP");
    assert!(body["traceId"].as_str().is_some());
    assert!(body["elapsedMs"].as_u64().is_some());
}

#[tokio::test]
async fn ready_check_returns_503_when_database_is_missing() {
    // 缺少数据库连接池代表实例不可接流量，ready 必须返回 HTTP 503。
    let app = build_router(app_state_without_database());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/ready")
                .body(Body::empty())
                .expect("测试请求必须可构造"),
        )
        .await
        .expect("就绪检查请求必须可执行");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = read_json(response).await;
    assert_eq!(body["code"], -500);
    assert_eq!(body["data"]["status"], "NOT_READY");
    assert_eq!(body["data"]["reason"], "database_pool_missing");
    assert!(body["traceId"].as_str().is_some());
}

#[tokio::test]
async fn ready_check_hides_raw_database_error() {
    // ping 失败时响应体只能暴露稳定原因码，原始数据库错误必须留在服务端日志。
    let pool = MySqlPoolOptions::new()
        .acquire_timeout(Duration::from_millis(50))
        .connect_lazy("mysql://root:root@127.0.0.1:1/taipus_test")
        .expect("测试懒连接池必须可构造");
    let app = build_router(AppState::new(
        AppConfig::for_test(),
        Some(DatabasePool::MySql(pool)),
    ));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/ready")
                .body(Body::empty())
                .expect("测试请求必须可构造"),
        )
        .await
        .expect("就绪检查请求必须可执行");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = read_json(response).await;
    assert_eq!(body["data"]["reason"], "database_unavailable");
    assert!(
        !body["data"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("root")
    );
    assert!(
        !body["data"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("127.0.0.1")
    );
}

#[tokio::test]
async fn valid_trace_id_is_echoed_to_header_and_body() {
    // 合法 traceId 必须原样透传到响应头和响应体，便于调用方关联日志。
    let app = build_router(app_state_without_database());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .header("X-Trace-Id", "valid-trace-1234")
                .body(Body::empty())
                .expect("测试请求必须可构造"),
        )
        .await
        .expect("健康检查请求必须可执行");

    assert_eq!(
        response
            .headers()
            .get("x-trace-id")
            .and_then(|value| value.to_str().ok()),
        Some("valid-trace-1234")
    );

    let body = read_json(response).await;
    assert_eq!(body["traceId"], "valid-trace-1234");
}

#[tokio::test]
async fn invalid_trace_id_is_replaced_consistently() {
    // 非法 traceId 不能进入日志和响应，服务端必须替换并保持头/body 一致。
    let app = build_router(app_state_without_database());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .header("X-Trace-Id", "bad trace")
                .body(Body::empty())
                .expect("测试请求必须可构造"),
        )
        .await
        .expect("健康检查请求必须可执行");

    let header_trace_id = response
        .headers()
        .get("x-trace-id")
        .and_then(|value| value.to_str().ok())
        .expect("响应头必须回写 traceId")
        .to_string();
    assert_ne!(header_trace_id, "bad trace");

    let body = read_json(response).await;
    assert_eq!(body["traceId"], header_trace_id);
}
