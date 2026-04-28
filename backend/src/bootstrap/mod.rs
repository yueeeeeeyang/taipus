//! 启动期基础设施模块。
//!
//! 该模块集中管理数据库连接、数据库迁移和 tracing 初始化，避免启动逻辑散落到业务模块。

pub mod database;
pub mod migration;
pub mod tracing;
