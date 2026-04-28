//! Refinery 数据库迁移启动封装。
//!
//! 迁移目录按数据库方言分开维护，但版本号必须保持一致。Refinery 暂不直接复用 SQLx 连接池，
//! 因此这里使用数据库连接串构造 Refinery 自身配置，并在阻塞线程中执行同步迁移。

use std::str::FromStr;

use refinery::config::Config;
use tokio::task;

use crate::{
    config::settings::DatabaseConfig,
    db::executor::DatabaseType,
    error::app_error::{AppError, AppResult},
};

mod mysql_migrations {
    use refinery::embed_migrations;
    // MySQL 为首版默认运行目标，migration 文件必须和 PostgreSQL 同版本。
    embed_migrations!("./migrations/mysql");
}

mod postgres_migrations {
    use refinery::embed_migrations;
    // PostgreSQL migration 用于保持兼容能力，文件版本必须与 MySQL 目录一致。
    embed_migrations!("./migrations/postgres");
}

/// 根据数据库类型执行对应方言的 migration。
///
/// Refinery 当前使用同步运行接口，因此放入 `spawn_blocking`，避免阻塞 Tokio 工作线程。
pub async fn run_migrations(config: &DatabaseConfig) -> AppResult<()> {
    let database_type = config.database_type;
    let database_url = config.database_url.clone();

    task::spawn_blocking(move || run_migrations_blocking(database_type, &database_url))
        .await?
        .map_err(AppError::from)
}

fn run_migrations_blocking(
    database_type: DatabaseType,
    database_url: &str,
) -> Result<(), refinery::Error> {
    // 使用连接串构造 Refinery 配置，避免把 SQLx 连接池类型暴露给迁移层。
    let mut config = Config::from_str(database_url)?;

    match database_type {
        DatabaseType::MySql => {
            // grouped=true 确保同一批迁移在单个事务语义下执行；失败时尽量避免半迁移状态。
            mysql_migrations::migrations::runner()
                .set_grouped(true)
                .set_abort_divergent(true)
                .set_abort_missing(true)
                .run(&mut config)?;
        }
        DatabaseType::Postgres => {
            // PostgreSQL 使用独立目录，避免 SQL 方言差异污染默认 MySQL 脚本。
            postgres_migrations::migrations::runner()
                .set_grouped(true)
                .set_abort_divergent(true)
                .set_abort_missing(true)
                .run(&mut config)?;
        }
    }

    Ok(())
}
