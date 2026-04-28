//! 测试辅助。
//!
//! 集成测试通过这里构造无数据库应用状态，避免单元测试依赖外部 MySQL 或 PostgreSQL 实例。

use crate::{AppConfig, AppState};

pub fn app_state_without_database() -> AppState {
    AppState::new(AppConfig::for_test(), None)
}
