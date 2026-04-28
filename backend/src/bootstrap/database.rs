//! 数据库连接池启动封装。
//!
//! 该模块只负责启动期创建连接池。业务查询统一通过 `db::executor` 暴露的适配类型执行，
//! 保持 MySQL 默认运行和 PostgreSQL 兼容边界清晰。

use crate::{
    config::settings::DatabaseConfig,
    db::executor::{DatabasePool, connect_pool},
    error::app_error::AppResult,
};

pub async fn create_database_pool(config: &DatabaseConfig) -> AppResult<DatabasePool> {
    // 启动入口只依赖该函数创建连接池，后续如需指标、重试或连接预热可集中在这里扩展。
    connect_pool(config).await
}
