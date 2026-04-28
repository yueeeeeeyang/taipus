//! 时间工具。
//!
//! 数据库和接口统一使用 UTC 时间，避免服务端部署时区不同导致审计字段和响应时间出现偏差。

use chrono::{DateTime, Utc};

/// 获取当前 UTC 时间。
///
/// 所有审计字段和响应时间都必须基于该入口，避免不同模块自行处理时区。
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// 获取当前 UTC 时间的 RFC3339 字符串。
///
/// 该函数主要服务日志或外部文本输出；接口响应优先直接序列化 `DateTime<Utc>`。
pub fn now_utc_rfc3339() -> String {
    now_utc().to_rfc3339()
}
