//! 中间件模块。
//!
//! 横切能力统一在这里实现，包括 traceId、访问日志和后续鉴权扩展。

pub mod access_log;
pub mod auth;
pub mod trace_id;
