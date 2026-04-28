//! 统一响应和分页契约集成测试。
//!
//! 这些测试固定前后端依赖的 JSON 字段、数字业务码和分页边界，避免后续重构破坏接口契约。

use http::StatusCode;
use serde_json::{Value, json};
use taipus_backend::{
    AppError,
    error::error_code::ErrorCode,
    response::{
        api_response::ApiResponse,
        page::{MAX_PAGE_SIZE, NormalizedPageQuery, PageQuery, PageResult},
    },
};

#[test]
fn success_response_uses_camel_case_and_positive_code() {
    // 成功响应必须使用正数业务码，并且对外 JSON 字段使用 camelCase。
    let response = ApiResponse::success(json!({"name": "taipus"}), "trace-1234");
    let value = serde_json::to_value(response).expect("统一响应必须可以序列化");

    assert_eq!(value["code"], 200);
    assert_eq!(value["message"], "ok");
    assert_eq!(value["data"]["name"], "taipus");
    assert_eq!(value["traceId"], "trace-1234");
    assert!(value.get("trace_id").is_none());
    assert!(value.get("timestamp").is_some());
    assert_eq!(value["elapsedMs"], 0.0);
    assert!(value.get("elapsed_ms").is_none());
}

#[test]
fn error_response_keeps_data_null_and_negative_code() {
    // 错误响应必须使用负数业务码，且 data 固定为 null，避免调用方误读错误数据。
    let response = ApiResponse::error(ErrorCode::ParamInvalid, "请求参数不合法", "trace-1234");
    let value = serde_json::to_value(response).expect("错误响应必须可以序列化");

    assert_eq!(value["code"], -400);
    assert_eq!(value["message"], "请求参数不合法");
    assert!(value["data"].is_null());
    assert_eq!(value["traceId"], "trace-1234");
    assert_eq!(value["elapsedMs"], 0.0);
}

#[tokio::test]
async fn app_error_response_uses_supplied_trace_id() {
    // AppError 必须使用 handler 传入的请求 traceId，不能在兜底转换中重新生成链路标识。
    let response = AppError::system("database connection refused")
        .into_response_with_trace("trace-error-1234");
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("错误响应体必须可读取");
    let value: Value = serde_json::from_slice(&bytes).expect("错误响应体必须是 JSON");

    assert_eq!(value["code"], -500);
    assert_eq!(value["traceId"], "trace-error-1234");
    assert!(value["data"].is_null());
    assert_eq!(value["elapsedMs"], 0.0);
}

#[test]
fn page_query_validates_bounds() {
    // 缺省分页参数用于列表接口的默认行为，必须稳定为第一页和默认分页大小。
    let default_page = PageQuery {
        page_no: None,
        page_size: None,
    }
    .validate_and_normalize()
    .expect("缺省分页参数必须可用");
    assert_eq!(default_page.page_no, 1);
    assert_eq!(default_page.page_size, 20);
    assert_eq!(default_page.offset, 0);

    // 超过服务端分页上限必须拒绝，防止超大列表请求影响数据库和序列化性能。
    let too_large = PageQuery {
        page_no: Some(1),
        page_size: Some(MAX_PAGE_SIZE + 1),
    };
    assert!(too_large.validate_and_normalize().is_err());
}

#[test]
fn page_query_rejects_offset_overflow() {
    // 超大 pageNo 不能让 offset 在 debug 下 panic 或在 release 下回绕。
    let overflow_page = PageQuery {
        page_no: Some(u64::MAX),
        page_size: Some(2),
    };

    let err = overflow_page
        .validate_and_normalize()
        .expect_err("offset 溢出必须返回参数错误");
    assert_eq!(err.code, ErrorCode::ParamInvalid);
}

#[test]
fn page_result_calculates_total_pages_and_has_next() {
    // 第二页、总数 45、每页 20 应得到 3 页，并且仍存在下一页。
    let page = NormalizedPageQuery {
        page_no: 2,
        page_size: 20,
        offset: 20,
    };
    let result: PageResult<Value> = PageResult::new(vec![json!({"id": 1})], page, 45);

    assert_eq!(result.total_pages, 3);
    assert!(result.has_next);
}
