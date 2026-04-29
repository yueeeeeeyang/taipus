//! 认证中间件和接口契约集成测试。
//!
//! 这些测试不依赖真实数据库，固定公开接口、受保护接口和统一错误响应的基础行为。

use axum::{body::Body, http::Request};
use serde_json::Value;
use taipus_backend::{build_router, tests::fixture::app_state_without_database};
use tower::ServiceExt;

async fn read_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("响应体必须可读取");
    serde_json::from_slice(&bytes).expect("响应体必须是 JSON")
}

#[tokio::test]
async fn auth_login_reports_body_parse_error_as_api_response() {
    // 登录属于公开接口；即使无数据库环境，JSON 解析错误也应先返回统一参数错误。
    let app = build_router(app_state_without_database());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":1}"#))
                .expect("测试请求必须可构造"),
        )
        .await
        .expect("登录请求必须可执行");

    let body = read_json(response).await;
    assert_eq!(body["code"], -400);
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("认证请求体不合法")
    );
}

#[tokio::test]
async fn auth_middleware_is_skipped_without_database_for_existing_contract_tests() {
    // 无数据库测试状态用于验证路由和响应契约，认证中间件不能破坏既有无数据库测试。
    let app = build_router(app_state_without_database());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .body(Body::empty())
                .expect("测试请求必须可构造"),
        )
        .await
        .expect("当前账号请求必须可执行");

    let body = read_json(response).await;
    assert_eq!(body["code"], -500);
}
