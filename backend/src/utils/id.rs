//! ID 生成与 traceId 校验工具。
//!
//! `traceId` 既会进入响应体，也会写入响应头和日志。这里统一校验规则，避免不同中间件
//! 对透传值的接受范围不一致，导致日志链路无法稳定关联。

use uuid::Uuid;

const TRACE_ID_MIN_LEN: usize = 8;
const TRACE_ID_MAX_LEN: usize = 128;

/// 生成新的请求链路标识。
///
/// 使用 UUID v4 可以避免依赖中心化发号器，适合中间件在高并发请求中快速生成 traceId。
pub fn generate_trace_id() -> String {
    Uuid::new_v4().to_string()
}

/// 生成通用业务 ID。
///
/// 首版使用 UUID v4，后续如果需要有序 ID 或雪花算法，应只替换该统一入口。
pub fn generate_business_id() -> String {
    Uuid::new_v4().to_string()
}

/// 归一化外部传入的 traceId。
///
/// 合法值直接透传，非法或缺失时重新生成，确保响应头、响应体和日志始终有可用链路标识。
pub fn normalize_trace_id(input: Option<&str>) -> String {
    input
        .map(str::trim)
        .filter(|value| is_valid_trace_id(value))
        .map(ToOwned::to_owned)
        .unwrap_or_else(generate_trace_id)
}

/// 校验 traceId 是否满足安全透传要求。
///
/// 只允许常见可打印 ASCII 字符，避免换行、空格或非 ASCII 字符污染日志和响应头。
pub fn is_valid_trace_id(value: &str) -> bool {
    let len = value.len();
    if !(TRACE_ID_MIN_LEN..=TRACE_ID_MAX_LEN).contains(&len) {
        return false;
    }

    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

#[cfg(test)]
mod tests {
    use super::{is_valid_trace_id, normalize_trace_id};

    #[test]
    fn invalid_trace_id_is_replaced() {
        // 非 ASCII 和空格属于非法 traceId，必须被替换为服务端生成值。
        let normalized = normalize_trace_id(Some("中文 trace"));
        assert!(is_valid_trace_id(&normalized));
        assert_ne!(normalized, "中文 trace");
    }
}
