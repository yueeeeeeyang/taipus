//! 后端基础底座库入口。
//!
//! 该 crate 将启动流程、统一响应、错误处理、请求上下文、数据库适配和健康检查拆成清晰模块，
//! 业务模块只能通过这些公共边界接入底座能力，避免直接依赖底层框架或数据库实现细节。

pub mod app;
pub mod bootstrap;
pub mod config;
pub mod context;
pub mod db;
pub mod error;
pub mod health;
pub mod middleware;
pub mod modules;
pub mod response;
pub mod tests;
pub mod utils;
pub mod validation;

pub use app::{AppState, build_router};
pub use config::settings::AppConfig;
pub use error::app_error::{AppError, AppResult};
