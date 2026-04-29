//! HRM 模块接口契约集成测试。
//!
//! 这些测试优先覆盖不依赖数据库的参数解析和统一响应行为，确保前端传参错误时能拿到可定位的
//! 字段级原因，而不是只能看到通用 400 文案。

use axum::{body::Body, http::Request};
use serde_json::{Value, json};
use taipus_backend::{
    AppError, build_router, context::request_context::RequestContext,
    tests::fixture::app_state_without_database,
};
use tower::ServiceExt;

async fn read_json(response: axum::response::Response) -> Value {
    // HRM 接口统一返回 JSON 响应，测试只关心业务码和错误消息契约。
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("响应体必须可读取");
    serde_json::from_slice(&bytes).expect("响应体必须是 JSON")
}

#[tokio::test]
async fn create_user_reports_detailed_json_field_error() {
    // sortNo 是数字字段；前端误传字符串时，错误消息必须保留 serde/axum 给出的字段路径和类型原因。
    let app = build_router(app_state_without_database());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/hrm/users")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "employeeNo": "1",
                        "name": "曹越洋",
                        "sortNo": "1",
                        "status": "enabled"
                    })
                    .to_string(),
                ))
                .expect("测试请求必须可构造"),
        )
        .await
        .expect("HRM 新增用户请求必须可执行");

    let body = read_json(response).await;
    let message = body["message"].as_str().expect("错误消息必须是字符串");
    assert_eq!(body["code"], -400);
    assert!(message.contains("sortNo"));
    assert!(message.contains("invalid type"));
    assert!(message.contains("expected i64"));
}

#[tokio::test]
async fn get_user_not_found_still_uses_locale_message() {
    // 非参数错误不能因为业务侧传入中文详情而绕过系统多语言文案。
    let state = app_state_without_database();
    let mut ctx = RequestContext::anonymous("trace-hrm-not-found");
    ctx.locale = "en-US".to_string();
    let response = AppError::resource_not_found("用户不存在或已删除")
        .into_response_with_context(&ctx, &state.i18n);

    let body = read_json(response).await;
    assert_eq!(body["code"], -404);
    assert_eq!(body["message"], "Resource not found");
}
