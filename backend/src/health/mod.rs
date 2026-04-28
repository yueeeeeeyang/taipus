//! 健康检查模块。
//!
//! 健康检查服务部署探针，允许使用 HTTP 200/503，但响应体仍复用统一结构。

pub mod handler;
pub mod route;
