//! 多语言模块集成测试。
//!
//! 这些测试固定语言协商优先级、系统资源接口结构、`Content-Language` 响应头和错误消息本地化。

use axum::{body::Body, http::Request};
use http::StatusCode;
use serde_json::Value;
use taipus_backend::{
    AppConfig, AppState, build_router, tests::fixture::app_state_without_database,
};
use tower::ServiceExt;

async fn read_json(response: axum::response::Response) -> Value {
    // 多语言接口测试只关心响应契约，因此统一解析为 JSON 值。
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("响应体必须可读取");
    serde_json::from_slice(&bytes).expect("响应体必须是 JSON")
}

#[tokio::test]
async fn system_resources_return_requested_locale_resources() {
    // query locale 优先级最高，必须决定响应资源和 Content-Language。
    let app = build_router(app_state_without_database());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/i18n/system_resources?locale=en-US&platform=frontend&namespaces=common,menu")
                .body(Body::empty())
                .expect("测试请求必须可构造"),
        )
        .await
        .expect("系统资源请求必须可执行");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-language")
            .and_then(|value| value.to_str().ok()),
        Some("en-US")
    );

    let body = read_json(response).await;
    assert_eq!(body["data"]["locale"], "en-US");
    assert_eq!(body["data"]["resources"]["common"]["confirm"], "Confirm");
    assert_eq!(
        body["data"]["resources"]["menu"]["systemSettings"],
        "System Settings"
    );
}

#[tokio::test]
async fn locale_header_controls_request_context_locale() {
    // 无 query 参数时，X-Locale 必须参与协商并回写响应头。
    let app = build_router(app_state_without_database());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .header("X-Locale", "en-US")
                .body(Body::empty())
                .expect("测试请求必须可构造"),
        )
        .await
        .expect("健康检查请求必须可执行");

    assert_eq!(
        response
            .headers()
            .get("content-language")
            .and_then(|value| value.to_str().ok()),
        Some("en-US")
    );
}

#[tokio::test]
async fn accept_language_continues_after_unsupported_candidate() {
    // 浏览器语言头中第一个候选不受支持时，后端必须继续协商后续可用语言。
    let app = build_router(app_state_without_database());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .header("Accept-Language", "fr-FR,en-US;q=0.9")
                .body(Body::empty())
                .expect("测试请求必须可构造"),
        )
        .await
        .expect("健康检查请求必须可执行");

    assert_eq!(
        response
            .headers()
            .get("content-language")
            .and_then(|value| value.to_str().ok()),
        Some("en-US")
    );
}

#[tokio::test]
async fn unsupported_locale_falls_back_to_config_default_locale() {
    // 默认语言来自配置；当请求语言不受支持时，必须降级到该配置值。
    let mut config = AppConfig::for_test();
    config.i18n.default_locale = "en-US".to_string();
    let app = build_router(AppState::new(config, None));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/i18n/system_resources?locale=fr-FR&platform=frontend&namespaces=common")
                .body(Body::empty())
                .expect("测试请求必须可构造"),
        )
        .await
        .expect("系统资源请求必须可执行");

    assert_eq!(
        response
            .headers()
            .get("content-language")
            .and_then(|value| value.to_str().ok()),
        Some("en-US")
    );

    let body = read_json(response).await;
    assert_eq!(body["data"]["locale"], "en-US");
    assert_eq!(body["data"]["resources"]["common"]["confirm"], "Confirm");
}

#[tokio::test]
async fn not_found_message_is_localized() {
    // fallback 路由也必须通过 RequestContext locale 渲染错误消息。
    let app = build_router(app_state_without_database());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/not_exists")
                .header("X-Locale", "en-US")
                .body(Body::empty())
                .expect("测试请求必须可构造"),
        )
        .await
        .expect("未匹配路由请求必须可执行");

    let body = read_json(response).await;
    assert_eq!(body["code"], -404);
    assert_eq!(body["message"], "Resource not found");
    assert_eq!(body["traceId"].as_str().is_some(), true);
}
